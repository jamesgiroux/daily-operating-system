#![cfg(feature = "load-test")]

use dailyos_lib::db::ActionDb;
use dailyos_lib::migration_test_api::run_migrations;
use dailyos_lib::services::context::{ExternalClients, ServiceContext, SystemClock, SystemRng};
use dailyos_lib::services::invalidation_jobs::{
    enqueue_signal_claim_recompute_in_tx, enqueue_signal_claim_recompute_with_config_in_tx,
    InvalidationJobQueueConfig,
};
use rusqlite::{params, Connection};
use serde_json::json;

fn load_test_db() -> ActionDb {
    let conn = Connection::open_in_memory().expect("open in-memory db");
    run_migrations(&conn).expect("run migrations");
    conn.execute_batch("PRAGMA foreign_keys = OFF;")
        .expect("disable FK for synthetic load test");
    ActionDb::from_connection_for_tests(conn)
}

fn seed_account(db: &ActionDb, account_id: &str) {
    db.conn_ref()
        .execute(
            "INSERT INTO accounts (id, name, updated_at, claim_version)
             VALUES (?1, ?2, '2026-05-09T00:00:00Z', 0)",
            params![account_id, format!("Load Test {account_id}")],
        )
        .expect("seed account");
}

fn count(db: &ActionDb, sql: &str) -> i64 {
    db.conn_ref()
        .query_row(sql, [], |row| row.get(0))
        .expect("count query")
}

#[test]
fn dos237_synthetic_signal_load_stays_coalesced_and_bounded() {
    let db = load_test_db();
    let account_id = "dos237-load-account";
    seed_account(&db, account_id);

    let clock = SystemClock;
    let rng = SystemRng;
    let ext = ExternalClients::default();
    let ctx = ServiceContext::new_live(&clock, &rng, &ext);

    for idx in 0..1_000 {
        db.with_transaction(|tx| {
            tx.conn_ref()
                .execute(
                    "UPDATE accounts
                     SET claim_version = claim_version + 1
                     WHERE id = ?1",
                    params![account_id],
                )
                .map_err(|e| e.to_string())?;
            let signal_id = dailyos_lib::services::signals::emit_in_transaction(
                &ctx,
                tx,
                "account",
                account_id,
                "ClaimTrustChanged",
                "dos237_load_test",
                json!({ "idx": idx }),
            )?;
            enqueue_signal_claim_recompute_in_tx(tx, &signal_id, "account", account_id)?;
            Ok(())
        })
        .expect("claim trust update transaction");
    }

    for idx in 0..100 {
        dailyos_lib::signals::bus::emit_signal(
            &db,
            "account",
            account_id,
            "AbilityOutputChanged",
            "dos237_load_test",
            Some(&json!({ "idx": idx, "ability": "load-test" }).to_string()),
            1.0,
        )
        .expect("ability output signal");
    }

    let claim_signal_rows = count(
        &db,
        "SELECT count(*) FROM signal_events WHERE signal_type = 'ClaimTrustChanged'",
    );
    let ability_signal_rows = count(
        &db,
        "SELECT count(*) FROM signal_events WHERE signal_type = 'AbilityOutputChanged'",
    );
    let pending_jobs = count(
        &db,
        "SELECT count(*) FROM invalidation_jobs WHERE status = 'pending'",
    );
    let raw_signal_count = count(
        &db,
        "SELECT COALESCE(max(raw_signal_count), 0) FROM invalidation_jobs",
    );

    assert!(
        claim_signal_rows <= 100,
        "claim trust coalescing ineffective: {claim_signal_rows} rows"
    );
    assert!(
        ability_signal_rows <= 10,
        "ability output coalescing ineffective: {ability_signal_rows} rows"
    );
    assert!(pending_jobs < 10_000, "queue exceeded pending bound");
    assert_eq!(raw_signal_count, 1_000);

    let mut terminalized = 0;
    while let Some(job) = db
        .claim_next_claim_recompute_job("dos237-load-worker", 60)
        .expect("claim next recompute")
    {
        db.terminalize_claim_recompute_job(&job.id)
            .expect("terminalize recompute");
        terminalized += 1;
    }

    assert!(terminalized > 0, "expected at least one affected output");
    let dead_lettered = count(
        &db,
        "SELECT count(*) FROM invalidation_jobs WHERE status = 'dead_lettered'",
    );
    let total_jobs = count(&db, "SELECT count(*) FROM invalidation_jobs").max(1);
    let dead_letter_rate = dead_lettered as f64 / total_jobs as f64;
    assert!(
        dead_letter_rate < 0.001,
        "dead-letter rate must stay below 0.1%, got {dead_letter_rate}"
    );

    let unresolved_outputs = count(
        &db,
        "SELECT count(*)
         FROM invalidation_jobs
         WHERE status NOT IN ('completed', 'dead_lettered', 'cycle_detected')",
    );
    let completed_or_stale = count(
        &db,
        "SELECT count(*)
         FROM invalidation_jobs
         WHERE status = 'completed' OR stale_marker_json IS NOT NULL",
    );
    let completed = count(
        &db,
        "SELECT count(*) FROM invalidation_jobs WHERE status = 'completed'",
    );

    assert_eq!(unresolved_outputs, 0);
    assert_eq!(completed_or_stale, total_jobs);
    assert_eq!(dead_lettered, 0);
    assert_eq!(completed, terminalized);
}

/// Multi-subject phase: 200 distinct accounts × 5 ClaimTrustChanged each = 1000
/// emissions that cannot all coalesce into a single job. Exercises the queue
/// behavior under non-coalescible load — verifies the cap is respected, every
/// emission either lands a job or aggressive-coalesces, and dead-letter rate
/// stays clean. Without this phase, the single-subject test above can never
/// reach the queue cap because everything folds to one pending job.
#[test]
fn dos237_multi_subject_load_respects_queue_cap_and_coalesces_per_subject() {
    let db = load_test_db();

    let clock = SystemClock;
    let rng = SystemRng;
    let ext = ExternalClients::default();
    let ctx = ServiceContext::new_live(&clock, &rng, &ext);

    let subject_count = 200usize;
    let events_per_subject = 5usize;
    let total_emissions = subject_count * events_per_subject;

    for s in 0..subject_count {
        let account_id = format!("dos237-multi-{s}");
        seed_account(&db, &account_id);
    }

    for round in 0..events_per_subject {
        for s in 0..subject_count {
            let account_id = format!("dos237-multi-{s}");
            db.with_transaction(|tx| {
                tx.conn_ref()
                    .execute(
                        "UPDATE accounts
                         SET claim_version = claim_version + 1
                         WHERE id = ?1",
                        params![account_id],
                    )
                    .map_err(|e| e.to_string())?;
                let signal_id = dailyos_lib::services::signals::emit_in_transaction(
                    &ctx,
                    tx,
                    "account",
                    &account_id,
                    "ClaimTrustChanged",
                    "dos237_multi_load_test",
                    json!({ "round": round, "subject": s }),
                )?;
                enqueue_signal_claim_recompute_in_tx(tx, &signal_id, "account", &account_id)?;
                Ok(())
            })
            .expect("multi-subject claim trust transaction");
        }
    }

    let pending_jobs = count(
        &db,
        "SELECT count(*) FROM invalidation_jobs WHERE status = 'pending'",
    );
    let dead_lettered = count(
        &db,
        "SELECT count(*) FROM invalidation_jobs WHERE status = 'dead_lettered'",
    );
    let total_jobs = count(&db, "SELECT count(*) FROM invalidation_jobs").max(1);

    // Each of the N subjects coalesces into 1 pending job (5 events/subject
    // collapse) — pending count should equal subject_count, well under the
    // 10000 default cap and proves per-subject coalescing is per-subject keyed.
    assert!(
        pending_jobs <= subject_count as i64,
        "expected per-subject coalescing: pending_jobs={pending_jobs} > subjects={subject_count}"
    );
    assert!(
        pending_jobs < 10_000,
        "queue exceeded pending bound under multi-subject load"
    );
    assert!(
        (dead_lettered as f64 / total_jobs as f64) < 0.001,
        "dead-letter rate must stay below 0.1% under multi-subject load"
    );

    // All raw signal emissions should be aggregated into the per-subject jobs;
    // total raw_signal_count across pending jobs ≥ total_emissions proves no
    // emission silently dropped (some may aggressive-coalesce into running
    // jobs which is also fine).
    let aggregated: i64 = db
        .conn_ref()
        .query_row(
            "SELECT COALESCE(SUM(raw_signal_count), 0) FROM invalidation_jobs",
            [],
            |row| row.get(0),
        )
        .expect("sum raw signal counts");
    assert!(
        aggregated >= total_emissions as i64,
        "raw_signal_count aggregated={aggregated} < total emissions={total_emissions} — silent drop"
    );
}

/// Cap-path phase: drive the queue past a low explicit cap to actually
/// exercise the rejection/aggressive-coalesce code path. The default cap
/// (10_000) is too high to hit with reasonable test sizes — this test uses
/// a 5-job cap and 50 distinct subjects so the cap-rejection branch fires
/// deterministically. Asserts that every emission is accounted for: either
/// represented by a pending/coalesced job, or surfaced as a producer-side
/// rejection (no silent drop).
#[test]
fn dos237_low_cap_load_surfaces_rejections_without_silent_drop() {
    let db = load_test_db();

    let clock = SystemClock;
    let rng = SystemRng;
    let ext = ExternalClients::default();
    let ctx = ServiceContext::new_live(&clock, &rng, &ext);

    let cap_config = InvalidationJobQueueConfig { pending_cap: 5 };
    let subject_count = 50usize;

    for s in 0..subject_count {
        let account_id = format!("dos237-cap-{s}");
        seed_account(&db, &account_id);
    }

    let mut emit_attempts = 0usize;
    let mut enqueue_rejections = 0usize;
    let mut enqueue_successes = 0usize;
    let mut other_errors: Vec<String> = Vec::new();

    for s in 0..subject_count {
        let account_id = format!("dos237-cap-{s}");
        emit_attempts += 1;

        let outcome = db.with_transaction(|tx| {
            tx.conn_ref()
                .execute(
                    "UPDATE accounts
                     SET claim_version = claim_version + 1
                     WHERE id = ?1",
                    params![account_id],
                )
                .map_err(|e| e.to_string())?;
            let signal_id = dailyos_lib::services::signals::emit_in_transaction(
                &ctx,
                tx,
                "account",
                &account_id,
                "ClaimTrustChanged",
                "dos237_cap_load_test",
                json!({ "subject": s }),
            )?;
            enqueue_signal_claim_recompute_with_config_in_tx(
                tx,
                &signal_id,
                "account",
                &account_id,
                cap_config,
            )?;
            Ok(())
        });

        match outcome {
            Ok(_) => enqueue_successes += 1,
            Err(e) => {
                // The cap rejection emits the exact message "invalidation
                // queue pending cap N reached; enqueue rejected" from
                // db/invalidation_jobs.rs. Match BOTH halves of that
                // signature to rule out other DbError::InvalidArgument
                // sources (schema validation, FK violations stringified
                // through the same variant, etc.) that would also contain
                // "InvalidArgument" but are NOT the cap-rejection branch.
                if e.contains("invalidation queue pending cap") && e.contains("enqueue rejected") {
                    enqueue_rejections += 1;
                } else {
                    other_errors.push(e);
                }
            }
        }
    }

    assert!(
        other_errors.is_empty(),
        "unexpected non-cap errors in low-cap test: {other_errors:?}"
    );

    let pending_jobs = count(
        &db,
        "SELECT count(*) FROM invalidation_jobs WHERE status = 'pending'",
    );
    let total_jobs = count(&db, "SELECT count(*) FROM invalidation_jobs");
    let signal_rows = count(&db, "SELECT count(*) FROM signal_events");

    // Cap actually saturated. With 50 distinct subjects and cap=5, the queue
    // must reach exactly the cap (not just stay under it).
    assert_eq!(
        pending_jobs, cap_config.pending_cap,
        "pending_jobs={pending_jobs} != cap={} — cap path not exercised",
        cap_config.pending_cap
    );

    // Cap path actually fires (50 subjects, cap 5 → at least 45 rejections).
    assert!(
        enqueue_rejections >= (subject_count - cap_config.pending_cap as usize),
        "expected >= {} cap-driven rejections, got {enqueue_rejections}",
        subject_count - cap_config.pending_cap as usize
    );
    assert!(
        enqueue_successes >= cap_config.pending_cap as usize,
        "expected >= {} successful enqueues, got {enqueue_successes} — proof cap-path was reachable, not just every subject erroring",
        cap_config.pending_cap
    );

    // No silent drop: every emission either succeeded (signal+job both
    // committed via with_transaction rollback semantics) or rejected
    // (transaction rolled back, no signal row, no job — caller observes the
    // error). Sum of accounted-for paths == total attempts.
    assert_eq!(
        emit_attempts,
        enqueue_successes + enqueue_rejections,
        "{emit_attempts} attempts != {enqueue_successes} success + {enqueue_rejections} rejections"
    );

    // No orphan signals: rejected transactions roll back the signal_events
    // INSERT too. signal_events count must match successful enqueues.
    assert_eq!(
        signal_rows as usize, enqueue_successes,
        "signal_events count {signal_rows} != enqueue_successes {enqueue_successes} — orphan signals from rolled-back txs"
    );
    assert!(
        total_jobs >= enqueue_successes as i64,
        "total_jobs={total_jobs} < enqueue_successes={enqueue_successes}"
    );
}
