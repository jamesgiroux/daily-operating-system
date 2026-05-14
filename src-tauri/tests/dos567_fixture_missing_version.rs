//! DOS-567 W4-B ac §26 — MissingExpectedClaimVersion foot-gun fix.
//!
//! A `ClaimProposal { id: Some("existing-claim-id"), expected_claim_version: None }`
//! submitted against an existing claim must be rejected with
//! `ClaimError::MissingExpectedClaimVersion { claim_id }`. This is the foot-gun
//! fix that landed in commit 322ca5b2: never silently route to Insert when the
//! caller named an existing claim id. The trait now routes such proposals to
//! `ClaimMutationTarget::Mutate { expected_claim_version: 0 }`, which the
//! transaction rejects (0 is reserved for backfill).

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
        SeedableRng::new(202),
        ExternalClients::default(),
    )
}

fn live_ctx<'a>(
    clock: &'a FixedClock,
    rng: &'a SeedableRng,
    external: &'a ExternalClients,
) -> ServiceContext<'a> {
    ServiceContext::new_live(clock, rng, external).with_actor("agent:test_missing_version")
}

fn subject_ref(account_id: &str) -> String {
    serde_json::json!({ "kind": "account", "id": account_id }).to_string()
}

fn risk_claim(account_id: &str, id: Option<String>, expected: Option<u64>) -> ClaimProposal {
    let observed_at = "2026-05-13T12:00:00+00:00".to_string();
    ClaimProposal {
        id,
        expected_claim_version: expected,
        subject_ref: subject_ref(account_id),
        claim_type: "risk".to_string(),
        field_path: Some("risks".to_string()),
        topic_key: None,
        text: "renewal timing requires attention".to_string(),
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
fn dos567_existing_claim_id_without_expected_version_rejected_not_silent_insert() {
    let conn = fresh_full_db();
    conn.execute(
        "INSERT INTO accounts (id, name, updated_at) VALUES (?1, ?2, ?3)",
        params!["acct-missing", "Missing Example", "2026-05-13T12:00:00Z"],
    )
    .expect("seed account");

    let (clock, rng, external) = ctx_parts();
    let ctx = live_ctx(&clock, &rng, &external);
    let db = ActionDb::from_conn(&conn);

    // Bootstrap: commit a fresh claim so we have an existing claim_id to target.
    let inserted = commit_claim(&ctx, db, risk_claim("acct-missing", None, None))
        .expect("bootstrap insert");
    let existing_claim_id = match inserted {
        CommittedClaim::Inserted { claim } => claim.id,
        other => panic!("expected inserted bootstrap, got {other:?}"),
    };

    // Foot-gun case: caller names the existing id but forgets expected_claim_version.
    // Trait routes to Mutate { expected_claim_version: 0 }; commit rejects.
    let foot_gun = risk_claim("acct-missing", Some(existing_claim_id.clone()), None);
    // Trait contract: `target()` routes a `Some(claim_id) + None` proposal to
    // `Mutate { expected_claim_version: 0 }` (not Insert) — that's the fix.
    match foot_gun.target() {
        ClaimMutationTarget::Mutate {
            claim_id,
            expected_claim_version,
        } => {
            assert_eq!(claim_id, existing_claim_id);
            assert_eq!(expected_claim_version, 0);
        }
        other => panic!("foot-gun proposal must route to Mutate, not {other:?}"),
    }

    let error =
        commit_claim(&ctx, db, foot_gun).expect_err("foot-gun mutate w/o version must reject");
    match error {
        ClaimError::MissingExpectedClaimVersion { claim_id } => {
            assert_eq!(claim_id, existing_claim_id);
        }
        other => panic!("expected MissingExpectedClaimVersion, got {other:?}"),
    }

    // No second version_events row was emitted: the bootstrap insert produced
    // exactly one `claim.updated`, and the rejected mutation produced none.
    let claim_updated_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM version_events
             WHERE claim_id = ?1 AND event_kind = 'claim.updated'",
            params![existing_claim_id],
            |row| row.get(0),
        )
        .expect("count claim.updated events");
    assert_eq!(claim_updated_count, 1, "no silent Insert leaked through");
}
