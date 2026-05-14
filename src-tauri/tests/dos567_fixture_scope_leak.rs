//! DOS-567 W4-B ac §27 — out-of-scope SurfaceClient receives a redacted
//! correction envelope. The 409 stale-watermark response includes
//! `correction.claim = null` and `scope_redacted: true, reason: "out_of_scope"`
//! when the caller's scopes don't permit reading the corrected claim.
//!
//! Asserted directly against `project_claim_for_scope`, the substrate primitive
//! that the bridge calls into for 409 correction projection.

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
        SeedableRng::new(606),
        ExternalClients::default(),
    )
}

fn live_ctx<'a>(
    clock: &'a FixedClock,
    rng: &'a SeedableRng,
    external: &'a ExternalClients,
) -> ServiceContext<'a> {
    ServiceContext::new_live(clock, rng, external).with_actor("agent:test_scope_leak")
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
        text: "scope-leak seed".to_string(),
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
    let scope_set = ScopeSet::new(scopes.iter().map(|s| SurfaceScope::new(*s)))
        .expect("scopes non-empty");
    Actor::SurfaceClient {
        instance: SurfaceClientId::new("sc-scope-leak-test"),
        scopes: scope_set,
    }
}

#[test]
fn dos567_out_of_scope_surface_client_receives_redacted_correction() {
    let conn = fresh_full_db();
    conn.execute(
        "INSERT INTO accounts (id, name, updated_at) VALUES (?1, ?2, ?3)",
        params!["acct-scoped", "Scoped Example", "2026-05-13T12:00:00Z"],
    )
    .expect("seed account");
    let (clock, rng, external) = ctx_parts();
    let ctx = live_ctx(&clock, &rng, &external);
    let db = ActionDb::from_conn(&conn);

    let inserted = commit_claim(&ctx, db, risk_proposal("acct-scoped")).expect("bootstrap");
    let claim_id = match inserted {
        CommittedClaim::Inserted { claim } => claim.id,
        other => panic!("expected Inserted, got {other:?}"),
    };

    // Caller carries a scope outside the claim-read allowlist
    // (submit.feedback is a write scope, not a read scope).
    let out_of_scope_caller = surface_client_with_scopes(&["submit.feedback"]);
    let projection = project_claim_for_scope(db, &claim_id, &out_of_scope_caller)
        .expect("claim exists, projection must return Some");

    match projection {
        CorrectionPayload {
            claim,
            scope_redacted,
            reason,
        } => {
            assert!(claim.is_none(), "no claim body leaks to out-of-scope caller");
            assert!(scope_redacted, "envelope reports redaction");
            assert_eq!(reason.as_deref(), Some("out_of_scope"));
        }
    }

    // Sanity contrast: an in-scope caller receives the claim body.
    let in_scope_caller = surface_client_with_scopes(&["read.account_overview"]);
    let allowed = project_claim_for_scope(db, &claim_id, &in_scope_caller)
        .expect("in-scope projection");
    assert!(!allowed.scope_redacted);
    assert!(allowed.claim.is_some());
}
