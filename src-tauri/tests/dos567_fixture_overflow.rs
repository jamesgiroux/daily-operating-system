//! W4-B ac §25 — claim_version overflow defense.
//!
//! Seeds an `intelligence_claims` row at `claim_version = i64::MAX`; attempts a
//! `commit_claim` Mutate proposal with `expected_claim_version = i64::MAX`. The
//! substrate must reject with `ClaimError::ClaimVersionOverflow { claim_id }`
//! (per `services::versioning::checked_next_version` overflow path), leaving
//! the row at i64::MAX without state poisoning.

use chrono::{TimeZone, Utc};
use dailyos_lib::db::claims::{ClaimSensitivity, TemporalScope};
use dailyos_lib::db::ActionDb;
use dailyos_lib::migration_test_api::run_migrations;
use dailyos_lib::services::claims::{commit_claim, ClaimError, ClaimProposal};
use dailyos_lib::services::context::{
    ExternalClients, FixedClock, SeedableRng, ServiceContext,
};
use rusqlite::{params, Connection};

fn fresh_full_db() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory db");
    run_migrations(&conn).expect("apply production migrations");
    conn
}

fn ctx_parts() -> (FixedClock, SeedableRng, ExternalClients) {
    (
        FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 13, 12, 0, 0).unwrap()),
        SeedableRng::new(101),
        ExternalClients::default(),
    )
}

fn live_ctx<'a>(
    clock: &'a FixedClock,
    rng: &'a SeedableRng,
    external: &'a ExternalClients,
) -> ServiceContext<'a> {
    ServiceContext::new_live(clock, rng, external).with_actor("agent:test_overflow")
}

fn subject_ref(account_id: &str) -> String {
    serde_json::json!({ "kind": "account", "id": account_id }).to_string()
}

fn seed_account(conn: &Connection, account_id: &str) {
    conn.execute(
        "INSERT INTO accounts (id, name, updated_at) VALUES (?1, ?2, ?3)",
        params![account_id, "Overflow Example", "2026-05-13T12:00:00Z"],
    )
    .expect("seed account");
}

fn insert_claim_at_version(conn: &Connection, claim_id: &str, account_id: &str, version: i64) {
    // Minimal seed: only the columns the overflow CAS read touches matter.
    // intelligence_claims requires NOT NULL on the structural columns; we
    // backfill everything to keep the row legal under v172's CHECK constraints.
    conn.execute(
        "INSERT INTO intelligence_claims (
            id, subject_ref, claim_type, field_path, topic_key, text,
            dedup_key, item_hash, actor, data_source, source_ref, source_asof,
            observed_at, created_at, provenance_json, metadata_json,
            claim_state, surfacing_state, demotion_reason, reactivated_at,
            retraction_reason, expires_at, superseded_by, trust_score,
            trust_computed_at, trust_version, thread_id, temporal_scope,
            sensitivity, verification_state, verification_reason,
            needs_user_decision_at, claim_version
         ) VALUES (
            ?1, ?2, 'risk', 'risks', NULL, 'overflow seed',
            ?3, ?4, 'agent:test', 'test', NULL, NULL,
            ?5, ?5, '{}', NULL,
            'active', 'active', NULL, NULL,
            NULL, NULL, NULL, NULL,
            NULL, NULL, NULL, 'state',
            'internal', 'active', NULL,
            NULL, ?6
         )",
        params![
            claim_id,
            subject_ref(account_id),
            format!("dedup-{claim_id}"),
            format!("hash-{claim_id}"),
            "2026-05-13T12:00:00Z",
            version,
        ],
    )
    .expect("seed claim at version");
}

#[test]
fn dos567_overflow_rejects_mutation_at_i64_max() {
    let conn = fresh_full_db();
    seed_account(&conn, "acct-overflow");
    let claim_id = "claim-overflow-test";
    insert_claim_at_version(&conn, claim_id, "acct-overflow", i64::MAX);

    let (clock, rng, external) = ctx_parts();
    let ctx = live_ctx(&clock, &rng, &external);

    let observed_at = ctx.clock.now().to_rfc3339();
    let proposal = ClaimProposal {
        id: Some(claim_id.to_string()),
        expected_claim_version: Some(i64::MAX as u64),
        subject_ref: subject_ref("acct-overflow"),
        claim_type: "risk".to_string(),
        field_path: Some("risks".to_string()),
        topic_key: None,
        text: "overflow attempt".to_string(),
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
    };

    let error = commit_claim(&ctx, ActionDb::from_conn(&conn), proposal)
        .expect_err("overflow at i64::MAX must reject");

    match error {
        ClaimError::ClaimVersionOverflow { claim_id: id } => {
            assert_eq!(id, claim_id);
        }
        other => panic!("expected ClaimVersionOverflow, got {other:?}"),
    }

    // No state poisoning: the claim row remains at i64::MAX, no version_events
    // outbox row was written for a non-existent next version.
    let stored: i64 = conn
        .query_row(
            "SELECT claim_version FROM intelligence_claims WHERE id = ?1",
            params![claim_id],
            |row| row.get(0),
        )
        .expect("claim row survives overflow reject");
    assert_eq!(stored, i64::MAX);

    let version_event_rows: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM version_events
             WHERE claim_id = ?1 AND event_kind = 'claim.updated'",
            params![claim_id],
            |row| row.get(0),
        )
        .expect("query version_events");
    assert_eq!(
        version_event_rows, 0,
        "no claim.updated event emitted on overflow reject"
    );
}
