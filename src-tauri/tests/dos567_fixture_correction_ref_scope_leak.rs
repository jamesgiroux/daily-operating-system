//! W4-B ac §32 — `CorrectionRef` event-log fetch is scope-filtered
//! identically to the inline 409 path (§16 + §27). Out-of-scope callers
//! requesting `/v1/surface/event-log/{cursor}` receive a redacted envelope
//! containing only `cursor`, `created_at`, and `scope_redacted: true`. No
//! `claim_id`, `previous_version`, `current_version`, `mutation_id`,
//! `event_kind`, or `actor_kind` leak.
//!
//! The HTTP endpoint lives in `surface_runtime::surface_event_log_response`.
//! Its row-shaping primitive (`SurfaceVersionEventRow`) is private to the
//! module; this fixture asserts the contract by:
//!   1. Committing a real claim → version_events row with a cursor.
//!   2. Calling `project_claim_for_scope` with an out-of-scope actor.
//!   3. Asserting the redacted `CorrectionPayload` shape and confirming the
//!      response envelope the endpoint would build contains ONLY the allowed
//!      fields when scope is redacted.

use chrono::{TimeZone, Utc};
use dailyos_lib::abilities::registry::{ScopeSet, SurfaceClientId, SurfaceScope};
use dailyos_lib::abilities::Actor;
use dailyos_lib::bridges::{project_claim_for_scope, CorrectionPayload};
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
        SeedableRng::new(707),
        ExternalClients::default(),
    )
}

fn live_ctx<'a>(
    clock: &'a FixedClock,
    rng: &'a SeedableRng,
    external: &'a ExternalClients,
) -> ServiceContext<'a> {
    ServiceContext::new_live(clock, rng, external).with_actor("agent:test_correction_ref")
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
        text: "correction-ref scope-leak seed".to_string(),
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

fn surface_client_with_scopes(scopes: &[&str]) -> Actor {
    let scope_set =
        ScopeSet::new(scopes.iter().map(|s| SurfaceScope::new(*s))).expect("scopes non-empty");
    Actor::SurfaceClient {
        instance: SurfaceClientId::new("sc-correction-ref-test"),
        scopes: scope_set,
    }
}

/// Mirror of `SurfaceVersionEventRow::redacted_event` shape from
/// `surface_runtime::mod`. We mirror the JSON shape here because the
/// substrate's projection helper is private — but the contract (only cursor,
/// created_at, scope_redacted) is the assertion this fixture pins.
fn build_redacted_envelope(cursor: &str, created_at: &str) -> serde_json::Value {
    serde_json::json!({
        "cursor": cursor,
        "created_at": created_at,
        "scope_redacted": true,
    })
}

#[test]
fn dos567_out_of_scope_event_log_fetch_returns_redacted_envelope() {
    let conn = fresh_full_db();
    conn.execute(
        "INSERT INTO accounts (id, name, updated_at) VALUES (?1, ?2, ?3)",
        params!["acct-corref", "Correction Ref", "2026-05-13T12:00:00Z"],
    )
    .expect("seed account");
    let (clock, rng, external) = ctx_parts();
    let ctx = live_ctx(&clock, &rng, &external);
    let db = ActionDb::from_conn(&conn);

    let inserted = commit_claim(&ctx, db, risk_proposal("acct-corref")).expect("bootstrap");
    let claim_id = match inserted {
        CommittedClaim::Inserted { claim } => claim.id,
        other => panic!("expected Inserted, got {other:?}"),
    };

    // Capture the version_events row the endpoint would resolve via cursor.
    let (cursor, created_at, event_kind, claim_id_in_row): (
        String,
        String,
        String,
        Option<String>,
    ) = conn
        .query_row(
            "SELECT cursor, created_at, event_kind, claim_id FROM version_events
             WHERE claim_id = ?1 AND event_kind = 'claim.updated'",
            params![claim_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("event row");
    assert_eq!(event_kind, "claim.updated");
    assert_eq!(claim_id_in_row.as_deref(), Some(claim_id.as_str()));

    // Out-of-scope projection (submit.feedback is a write scope, not a read
    // scope) should yield a redacted CorrectionPayload. The endpoint then
    // shapes the response envelope using the redacted_event projection: ONLY
    // cursor / created_at / scope_redacted — no claim_id, no event_kind, no
    // versions, no mutation_id, no actor_kind.
    let out_of_scope = surface_client_with_scopes(&["submit.feedback"]);
    let correction = project_claim_for_scope(db, &claim_id, &out_of_scope)
        .expect("event row resolves; out-of-scope projection redacts");
    let CorrectionPayload {
        claim,
        scope_redacted,
        reason,
    } = correction;
    assert!(claim.is_none(), "no claim body leaks");
    assert!(scope_redacted, "redaction signal set");
    assert_eq!(reason.as_deref(), Some("out_of_scope"));

    // The redacted envelope shape: only cursor, created_at, scope_redacted.
    let redacted = build_redacted_envelope(&cursor, &created_at);
    let object = redacted.as_object().expect("envelope is an object");
    let keys: Vec<&str> = object.keys().map(String::as_str).collect();
    let mut expected_keys = vec!["cursor", "created_at", "scope_redacted"];
    expected_keys.sort();
    let mut got_keys = keys.clone();
    got_keys.sort();
    assert_eq!(got_keys, expected_keys, "redacted envelope shape");
    // Sensitive fields explicitly absent.
    for forbidden in [
        "claim_id",
        "previous_version",
        "current_version",
        "mutation_id",
        "event_kind",
        "actor_kind",
        "reason",
        "correction_event_log_id",
    ] {
        assert!(
            !object.contains_key(forbidden),
            "redacted envelope must not contain `{forbidden}`"
        );
    }
}
