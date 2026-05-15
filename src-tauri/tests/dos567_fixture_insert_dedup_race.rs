//! W4-B ac §32c — concurrent Inserts with the same dedup-equivalent
//! key collapse to one Insert + one Reinforced inside the canonical
//! commit/dedup-lock path (`claims.rs:5286-5314`).
//!
//! Two ClaimProposals with identical `(subject_ref, claim_type, field_path,
//! text)` produce the same `dedup_key` via `compute_dedup_key`. The
//! per-process `commit_lock_for` serializes both calls; the first commits
//! cleanly as `Inserted`; the second hits the dedup path and returns
//! `Reinforced` with a matching canonical claim_id. Both emit `version_events`
//! rows with distinct `event_seq` and `cursor`.
//!
//! Note: rusqlite::Connection is `!Sync`, so this fixture invokes both commits
//! sequentially on a single thread. The substrate's `commit_lock_for` would
//! serialize multi-thread races to the same outcome — the observable contract
//! (one Inserted, one Reinforced, no duplicate rows) is identical.

use chrono::{TimeZone, Utc};
use dailyos_lib::db::claims::{ClaimSensitivity, TemporalScope};
use dailyos_lib::db::ActionDb;
use dailyos_lib::migration_test_api::run_migrations;
use dailyos_lib::services::claims::{commit_claim, ClaimProposal, CommittedClaim};
use dailyos_lib::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};
use rusqlite::{params, Connection};

fn fresh_full_db() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory db");
    run_migrations(&conn).expect("apply production migrations");
    conn
}

fn ctx_parts() -> (FixedClock, SeedableRng, ExternalClients) {
    (
        FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 13, 12, 0, 0).unwrap()),
        SeedableRng::new(808),
        ExternalClients::default(),
    )
}

fn live_ctx<'a>(
    clock: &'a FixedClock,
    rng: &'a SeedableRng,
    external: &'a ExternalClients,
) -> ServiceContext<'a> {
    ServiceContext::new_live(clock, rng, external).with_actor("agent:test_insert_dedup")
}

fn subject_ref(account_id: &str) -> String {
    serde_json::json!({ "kind": "account", "id": account_id }).to_string()
}

fn risk_proposal(account_id: &str) -> ClaimProposal {
    let observed_at = "2026-05-13T12:00:00+00:00".to_string();
    ClaimProposal {
        id: None,
        expected_claim_version: None,
        subject_ref: subject_ref(account_id),
        claim_type: "risk".to_string(),
        field_path: Some("risks".to_string()),
        topic_key: None,
        text: "identical content for dedup race".to_string(),
        actor: "agent:test".to_string(),
        data_source: "test".to_string(),
        source_ref: None,
        source_asof: Some(observed_at.clone()),
        observed_at,
        provenance_json: "{}".to_string(),
        metadata_json: None,
        thread_id: None,
        temporal_scope: Some(TemporalScope::State),
        sensitivity: Some(ClaimSensitivity::Internal),
        supersedes: None,
        tombstone: None,
    }
}

#[test]
fn dos567_two_inserts_same_dedup_key_collapse_to_insert_plus_reinforced() {
    let conn = fresh_full_db();
    conn.execute(
        "INSERT INTO accounts (id, name, updated_at) VALUES (?1, ?2, ?3)",
        params!["acct-dedup", "Dedup Race", "2026-05-13T12:00:00Z"],
    )
    .expect("seed account");
    let (clock, rng, external) = ctx_parts();
    let ctx = live_ctx(&clock, &rng, &external);
    let db = ActionDb::from_conn(&conn);

    let first = commit_claim(&ctx, db, risk_proposal("acct-dedup")).expect("first commit");
    let canonical_id = match first {
        CommittedClaim::Inserted { claim } => claim.id,
        other => panic!("first must be Inserted, got {other:?}"),
    };

    let second = commit_claim(&ctx, db, risk_proposal("acct-dedup")).expect("second commit");
    match second {
        CommittedClaim::Reinforced { claim, .. } => {
            assert_eq!(
                claim.id, canonical_id,
                "Reinforced returns the canonical claim_id"
            );
        }
        other => panic!("second must be Reinforced, got {other:?}"),
    }

    // Exactly one intelligence_claims row for the dedup-equivalent content.
    let row_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM intelligence_claims
             WHERE subject_ref = ?1 AND claim_type = ?2 AND text = ?3",
            params![
                subject_ref("acct-dedup"),
                "risk",
                "identical content for dedup race",
            ],
            |row| row.get(0),
        )
        .expect("row count");
    assert_eq!(row_count, 1, "no duplicate rows");

    // Two version_events rows on this claim, distinct cursor + event_seq.
    let events: Vec<(i64, String, String)> = {
        let mut stmt = conn
            .prepare(
                "SELECT event_seq, cursor, event_kind FROM version_events
                 WHERE claim_id = ?1
                 ORDER BY event_seq",
            )
            .expect("prepare");
        let rows = stmt
            .query_map(params![canonical_id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .expect("query")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect");
        rows
    };
    assert_eq!(events.len(), 2, "two version_events rows for two commits");
    let (seq_first, cursor_first, _) = &events[0];
    let (seq_second, cursor_second, _) = &events[1];
    assert!(seq_second > seq_first, "event_seq strictly monotonic");
    assert_ne!(cursor_first, cursor_second, "distinct cursors");
}
