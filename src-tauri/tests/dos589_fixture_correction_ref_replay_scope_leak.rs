//! W4-B-signals AC #5 — `CorrectionRef.event_log_id` lookup is scope-filtered
//! identically to the inline 409 correction projection.
//!
//! The dispatcher exposes `scope_permits_claim_read(db, actor, claim_id)` as
//! the single source of truth for both paths. This fixture asserts that
//! predicate returns identical results for inline and lookup routes — i.e.
//! a single function gates both 409 inline correction (W4-B §16) and the
//! `GET /v1/surface/event-log/{event_log_id}` direct-key path (W4-B §37).
//!
//! Failure mode caught: scope-filter drift between inline and lookup;
//! `event_log_id` being treated as a bearer token; an out-of-scope caller
//! receiving the claim body via direct lookup that they could not see
//! inline.

use chrono::{TimeZone, Utc};
use dailyos_lib::abilities::registry::{ScopeSet, SurfaceClientId, SurfaceScope};
use dailyos_lib::abilities::Actor;
use dailyos_lib::bridges::project_claim_for_scope;
use dailyos_lib::db::claims::{ClaimSensitivity, TemporalScope};
use dailyos_lib::db::ActionDb;
use dailyos_lib::migration_test_api::run_migrations;
use dailyos_lib::services::claims::{commit_claim, ClaimProposal, CommittedClaim};
use dailyos_lib::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};
use dailyos_lib::services::version_dispatcher::scope_permits_claim_read;
use rusqlite::{params, Connection};

fn fresh_full_db() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory db");
    run_migrations(&conn).expect("apply production migrations");
    conn
}

fn proposal(account_id: &str) -> ClaimProposal {
    let observed = "2026-05-15T12:00:00+00:00".to_string();
    ClaimProposal {
        id: None,
        expected_claim_version: None,
        subject_ref: serde_json::json!({"kind": "account", "id": account_id}).to_string(),
        claim_type: "risk".to_string(),
        field_path: Some("risks.correction".to_string()),
        topic_key: None,
        text: "dos589 correction-ref scope".to_string(),
        actor: "agent:dos589_corr".to_string(),
        data_source: "test".to_string(),
        source_ref: None,
        source_asof: Some(observed.clone()),
        observed_at: observed,
        provenance_json: "{}".to_string(),
        metadata_json: None,
        thread_id: None,
        temporal_scope: Some(TemporalScope::State),
        sensitivity: Some(ClaimSensitivity::Internal),
        supersedes: None,
        tombstone: None,
    }
}

fn surface_client(name: &str, scopes: &[&str]) -> Actor {
    let scope_set =
        ScopeSet::new(scopes.iter().map(|s| SurfaceScope::new(*s))).expect("scopes non-empty");
    Actor::SurfaceClient {
        instance: SurfaceClientId::new(name),
        scopes: scope_set,
    }
}

#[test]
fn dos589_correction_ref_lookup_matches_inline_predicate() {
    let conn = fresh_full_db();
    conn.execute(
        "INSERT INTO accounts (id, name, updated_at) VALUES (?1, ?2, ?3)",
        params!["acct-589-corr", "DOS589 CORR", "2026-05-15T12:00:00Z"],
    )
    .expect("seed account");
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 15, 12, 0, 0).unwrap());
    let rng = SeedableRng::new(589_001);
    let external = ExternalClients::default();
    let ctx = ServiceContext::new_live(&clock, &rng, &external).with_actor("agent:dos589_corr");
    let db = ActionDb::from_conn(&conn);
    let inserted = commit_claim(&ctx, db, proposal("acct-589-corr")).expect("bootstrap");
    let claim_id = match inserted {
        CommittedClaim::Inserted { claim } => claim.id,
        other => panic!("expected Inserted, got {other:?}"),
    };

    let in_actor = surface_client("sc-corr-in", &["read.account_overview"]);
    let out_actor = surface_client("sc-corr-out", &["submit.feedback"]);

    // Inline path: project_claim_for_scope is what the 409 inline correction
    // and the GET /v1/surface/event-log/{cursor} lookup both call.
    let inline_in = project_claim_for_scope(db, &claim_id, &in_actor).expect("claim exists");
    let inline_out = project_claim_for_scope(db, &claim_id, &out_actor).expect("claim exists");
    assert!(
        !inline_in.scope_redacted && inline_in.claim.is_some(),
        "inline: in-scope returns claim body"
    );
    assert!(
        inline_out.scope_redacted && inline_out.claim.is_none(),
        "inline: out-of-scope returns redacted envelope"
    );

    // Lookup path through the dispatcher's predicate — must return the same
    // bool that the inline path's `scope_redacted == false` corresponds to.
    assert!(
        scope_permits_claim_read(db, &in_actor, &claim_id),
        "lookup permits the in-scope caller, matching inline"
    );
    assert!(
        !scope_permits_claim_read(db, &out_actor, &claim_id),
        "lookup denies the out-of-scope caller, matching inline (no bearer-token bypass via event_log_id)"
    );
}
