//! W4-B ac §31 — outbox atomicity: claim row + version_events row are
//! tied by the mutation transaction. If the outbox insert fails after the
//! version row was updated within the same Tx, the entire Tx rolls back —
//! neither survives.
//!
//! We exercise the atomicity primitive by running an explicit transaction that
//! advances `intelligence_claims.claim_version` and then attempts a
//! constraint-violating `version_events` insert (bad `event_kind` against the
//! enum CHECK). Post-rollback assertions: claim row unchanged, no event row,
//! doctor `claims_missing_outbox == 0`.

use chrono::{TimeZone, Utc};
use dailyos_lib::db::ActionDb;
use dailyos_lib::doctor::inspect_watermarks;
use dailyos_lib::migration_test_api::run_migrations;
use dailyos_lib::services::claims::{commit_claim, ClaimProposal, CommittedClaim};
use dailyos_lib::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};
use dailyos_lib::db::claims::{ClaimSensitivity, TemporalScope};
use rusqlite::{params, Connection};

fn fresh_full_db() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory db");
    run_migrations(&conn).expect("apply production migrations");
    conn
}

fn ctx_parts() -> (FixedClock, SeedableRng, ExternalClients) {
    (
        FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 13, 12, 0, 0).unwrap()),
        SeedableRng::new(505),
        ExternalClients::default(),
    )
}

fn live_ctx<'a>(
    clock: &'a FixedClock,
    rng: &'a SeedableRng,
    external: &'a ExternalClients,
) -> ServiceContext<'a> {
    ServiceContext::new_live(clock, rng, external).with_actor("agent:test_outbox_atomicity")
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
        text: "outbox atomicity seed".to_string(),
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
fn dos567_outbox_atomic_with_claim_version_advance() {
    let conn = fresh_full_db();
    conn.execute(
        "INSERT INTO accounts (id, name, updated_at) VALUES (?1, ?2, ?3)",
        params!["acct-atomic", "Atomic Example", "2026-05-13T12:00:00Z"],
    )
    .expect("seed account");
    let (clock, rng, external) = ctx_parts();
    let ctx = live_ctx(&clock, &rng, &external);
    let db = ActionDb::from_conn(&conn);

    // Bootstrap a real claim at v=1 via commit_claim — both rows present.
    let committed = commit_claim(&ctx, db, risk_proposal("acct-atomic")).expect("bootstrap");
    let claim_id = match committed {
        CommittedClaim::Inserted { claim } => claim.id,
        other => panic!("expected Inserted, got {other:?}"),
    };

    let version_pre: i64 = conn
        .query_row(
            "SELECT claim_version FROM intelligence_claims WHERE id = ?1",
            params![claim_id],
            |row| row.get(0),
        )
        .expect("pre-version");
    let event_count_pre: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM version_events
             WHERE claim_id = ?1 AND event_kind = 'claim.updated'",
            params![claim_id],
            |row| row.get(0),
        )
        .expect("pre-event-count");

    // Drive a failing Tx that advances claim_version AND attempts to insert
    // a version_events row violating the CHECK constraint on `event_kind`.
    // SQLite must roll back the entire Tx; claim_version stays at its old
    // value AND no extra event row persists.
    let tx_outcome: Result<(), String> = db.with_transaction(|tx| {
        tx.conn_ref()
            .execute(
                "UPDATE intelligence_claims SET claim_version = claim_version + 1 WHERE id = ?1",
                params![claim_id],
            )
            .map_err(|e| e.to_string())?;
        // Attempt a CHECK-violating insert; this MUST fail and abort the Tx.
        let result = tx.conn_ref().execute(
            "INSERT INTO version_events (
                cursor, event_kind, claim_id, current_version,
                scope_redacted, created_at, actor_kind
             ) VALUES (?1, ?2, ?3, ?4, 0, ?5, 'system')",
            params![
                "deadbeef-1234-4234-8234-1234567890ab",
                "not_a_valid_event_kind", // CHECK violation
                claim_id,
                2_i64,
                "2026-05-13T12:00:01Z",
            ],
        );
        result.map(|_| ()).map_err(|e| e.to_string())?;
        Ok(())
    });
    assert!(
        tx_outcome.is_err(),
        "Tx with CHECK-violating event_kind must abort"
    );

    // Post-rollback: neither the claim row advance NOR the event row persists.
    let version_post: i64 = conn
        .query_row(
            "SELECT claim_version FROM intelligence_claims WHERE id = ?1",
            params![claim_id],
            |row| row.get(0),
        )
        .expect("post-version");
    assert_eq!(
        version_post, version_pre,
        "claim_version must roll back on outbox failure"
    );
    let event_count_post: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM version_events WHERE claim_id = ?1",
            params![claim_id],
            |row| row.get(0),
        )
        .expect("post-event-count");
    assert_eq!(
        event_count_post,
        event_count_pre,
        "no event row persists after rollback (orphans forbidden)"
    );

    // Doctor invariant: no claim_version row without a matching outbox event.
    let report = inspect_watermarks(db).expect("doctor");
    assert_eq!(report.claims_missing_outbox, 0);
}
