//! W4-B ac §32b — fresh insert routes via `ClaimMutationTarget::Insert`.
//!
//! Positive: `ClaimProposal { id: None, expected_claim_version: None }` flows
//! through `commit_claim` as `Insert`, skipping version CAS and producing a
//! row with `claim_version = 1`.
//! Negative companion: the same caller, after seeing the new claim_id, must NOT
//! be able to submit `id: Some(new_id), expected_claim_version: None` —
//! that's the foot-gun shape from §26. Trait routes it to
//! `Mutate { expected_claim_version: 0 }`, which the transaction rejects as
//! `MissingExpectedClaimVersion`.

use chrono::{TimeZone, Utc};
use dailyos_lib::db::claims::{ClaimSensitivity, TemporalScope};
use dailyos_lib::db::ActionDb;
use dailyos_lib::migration_test_api::run_migrations;
use dailyos_lib::services::claims::{
    commit_claim, ClaimError, ClaimMutationTarget, ClaimProposal, CommittedClaim, MutatingProposal,
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
        SeedableRng::new(303),
        ExternalClients::default(),
    )
}

fn live_ctx<'a>(
    clock: &'a FixedClock,
    rng: &'a SeedableRng,
    external: &'a ExternalClients,
) -> ServiceContext<'a> {
    ServiceContext::new_live(clock, rng, external).with_actor("agent:test_fresh_insert")
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
        text: "fresh insert via target".to_string(),
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
fn dos567_fresh_insert_routes_through_insert_variant_with_version_one() {
    let conn = fresh_full_db();
    conn.execute(
        "INSERT INTO accounts (id, name, updated_at) VALUES (?1, ?2, ?3)",
        params!["acct-fresh", "Fresh Example", "2026-05-13T12:00:00Z"],
    )
    .expect("seed account");
    let (clock, rng, external) = ctx_parts();
    let ctx = live_ctx(&clock, &rng, &external);
    let db = ActionDb::from_conn(&conn);

    let fresh = risk_proposal("acct-fresh", None, None);
    // Trait must route a fresh proposal to `Insert`.
    assert!(matches!(fresh.target(), ClaimMutationTarget::Insert { .. }));

    let committed = commit_claim(&ctx, db, fresh).expect("fresh insert commits");
    let new_claim = match committed {
        CommittedClaim::Inserted { claim } => claim,
        other => panic!("expected Inserted, got {other:?}"),
    };
    assert_eq!(new_claim.claim_version, 1);

    // version_events row at claim_version=1 with previous_version NULL.
    let (current, previous): (i64, Option<i64>) = conn
        .query_row(
            "SELECT current_version, previous_version FROM version_events
             WHERE claim_id = ?1 AND event_kind = 'claim.updated'
             ORDER BY event_seq DESC LIMIT 1",
            params![new_claim.id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("event row");
    assert_eq!(current, 1);
    assert_eq!(previous, None);
}

#[test]
fn dos567_named_existing_id_without_version_rejects_missing_expected_claim_version() {
    let conn = fresh_full_db();
    conn.execute(
        "INSERT INTO accounts (id, name, updated_at) VALUES (?1, ?2, ?3)",
        params!["acct-foot-gun", "Foot Gun", "2026-05-13T12:00:00Z"],
    )
    .expect("seed account");
    let (clock, rng, external) = ctx_parts();
    let ctx = live_ctx(&clock, &rng, &external);
    let db = ActionDb::from_conn(&conn);

    let bootstrap = commit_claim(&ctx, db, risk_proposal("acct-foot-gun", None, None))
        .expect("bootstrap");
    let new_id = match bootstrap {
        CommittedClaim::Inserted { claim } => claim.id,
        other => panic!("bootstrap expected Inserted, got {other:?}"),
    };

    let foot_gun = risk_proposal("acct-foot-gun", Some(new_id.clone()), None);
    // Trait must NOT route this to Insert; must be Mutate with
    // expected_claim_version: 0 (reserved → rejected by tx).
    assert!(matches!(
        foot_gun.target(),
        ClaimMutationTarget::Mutate { expected_claim_version: 0, .. }
    ));

    let error = commit_claim(&ctx, db, foot_gun).expect_err("must reject");
    match error {
        ClaimError::MissingExpectedClaimVersion { claim_id } => assert_eq!(claim_id, new_id),
        other => panic!("expected MissingExpectedClaimVersion, got {other:?}"),
    }
}
