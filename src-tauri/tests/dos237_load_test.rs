#![cfg(feature = "load-test")]

use dailyos_lib::db::ActionDb;
use dailyos_lib::migration_test_api::run_migrations;
use dailyos_lib::services::context::{ExternalClients, ServiceContext, SystemClock, SystemRng};
use dailyos_lib::services::invalidation_jobs::enqueue_signal_claim_recompute_in_tx;
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
