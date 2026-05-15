//! W4-B ac §28 — inflated `expected_claim_version` rejection.
//!
//! Distinct from stale (`expected < current`): when the caller presents
//! `expected_claim_version > current`, the substrate rejects with
//! `ClaimError::InflatedVersion` and emits a `version_events` row with
//! `event_kind = 'claim.write_rejected'` and `reason = 'inflated_version_rejected'`
//! (W4-B audit substrate uses the `version_events` row as the durable audit
//! record per §15; the file-based audit chain is separate and not asserted
//! here).

use chrono::{TimeZone, Utc};
use dailyos_lib::db::claims::{ClaimSensitivity, TemporalScope};
use dailyos_lib::db::ActionDb;
use dailyos_lib::migration_test_api::run_migrations;
use dailyos_lib::services::claims::{
    commit_claim, ClaimError, ClaimProposal, CommittedClaim,
};
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
        SeedableRng::new(404),
        ExternalClients::default(),
    )
}

fn live_ctx<'a>(
    clock: &'a FixedClock,
    rng: &'a SeedableRng,
    external: &'a ExternalClients,
) -> ServiceContext<'a> {
    ServiceContext::new_live(clock, rng, external).with_actor("agent:test_inflated")
}

fn subject_ref(account_id: &str) -> String {
    serde_json::json!({ "kind": "account", "id": account_id }).to_string()
}

fn risk_proposal(account_id: &str, id: Option<String>, expected: Option<u64>) -> ClaimProposal {
    let observed_at = "2026-05-13T12:00:00+00:00".to_string();
    ClaimProposal {
        id,
        expected_claim_version: expected,
        subject_ref: subject_ref(account_id),
        claim_type: "risk".to_string(),
        field_path: Some("risks".to_string()),
        topic_key: None,
        text: "inflated version attempt".to_string(),
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
fn dos567_inflated_expected_version_rejected_with_distinct_audit_reason() {
    let conn = fresh_full_db();
    conn.execute(
        "INSERT INTO accounts (id, name, updated_at) VALUES (?1, ?2, ?3)",
        params!["acct-inflated", "Inflated Example", "2026-05-13T12:00:00Z"],
    )
    .expect("seed account");

    let (clock, rng, external) = ctx_parts();
    let ctx = live_ctx(&clock, &rng, &external);
    let db = ActionDb::from_conn(&conn);

    // Bootstrap a claim at v=1. The mutation we care about presents a
    // fabricated future version against this current=1 row.
    let inserted = commit_claim(&ctx, db, risk_proposal("acct-inflated", None, None))
        .expect("bootstrap insert");
    let claim_id = match inserted {
        CommittedClaim::Inserted { claim } => claim.id,
        other => panic!("expected Inserted, got {other:?}"),
    };

    let current: i64 = conn
        .query_row(
            "SELECT claim_version FROM intelligence_claims WHERE id = ?1",
            params![claim_id],
            |row| row.get(0),
        )
        .expect("current version");
    assert_eq!(current, 1, "bootstrap claim sits at v=1");

    // Caller presents fabricated future version 999 (current is 1).
    let inflated = risk_proposal("acct-inflated", Some(claim_id.clone()), Some(999));
    let error = commit_claim(&ctx, db, inflated).expect_err("inflated must reject");
    match error {
        ClaimError::InflatedVersion {
            claim_id: id,
            expected,
            current,
        } => {
            assert_eq!(id, claim_id);
            assert_eq!(expected, 999);
            assert_eq!(current, 1);
        }
        other => panic!("expected InflatedVersion, got {other:?}"),
    }

    // `version_events` row records the rejection with the distinct reason.
    let (event_kind, reason): (String, Option<String>) = conn
        .query_row(
            "SELECT event_kind, reason FROM version_events
             WHERE claim_id = ?1 AND event_kind = 'claim.write_rejected'
             ORDER BY event_seq DESC LIMIT 1",
            params![claim_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("rejection event present");
    assert_eq!(event_kind, "claim.write_rejected");
    assert_eq!(reason.as_deref(), Some("inflated_version_rejected"));

    // Substrate state unchanged after rejection.
    let still: i64 = conn
        .query_row(
            "SELECT claim_version FROM intelligence_claims WHERE id = ?1",
            params![claim_id],
            |row| row.get(0),
        )
        .expect("post-reject version");
    assert_eq!(still, 1);
}
