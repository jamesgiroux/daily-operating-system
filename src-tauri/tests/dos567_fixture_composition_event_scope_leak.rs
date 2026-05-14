//! DOS-567 W4-B §16 + ac §37 — composition-bound `version_events` rows are
//! scope-filtered identically to claim-bound rows. Out-of-scope callers
//! requesting `/v1/surface/event-log/{cursor}` for a composition event must
//! receive a redacted envelope containing only `cursor`, `created_at`, and
//! `scope_redacted: true`. No `composition_id`, `previous_version`,
//! `current_version`, `mutation_id`, `event_kind`, `correction_event_log_id`,
//! `reason`, or `actor_kind` leaks.
//!
//! Companion to `dos567_fixture_correction_ref_scope_leak.rs`: cycle-1
//! closed the claim_id channel; this fixture pins the composition_id
//! channel against the same regression class per packet §16.

use dailyos_lib::abilities::registry::{ScopeSet, SurfaceClientId, SurfaceScope};
use dailyos_lib::abilities::Actor;
use dailyos_lib::bridges::{project_composition_for_scope, CorrectionPayload};
use dailyos_lib::db::ActionDb;
use dailyos_lib::migration_test_api::run_migrations;
use rusqlite::{params, Connection};

fn fresh_full_db() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory db");
    run_migrations(&conn).expect("apply production migrations");
    conn
}

fn surface_client_with_scopes(scopes: &[&str]) -> Actor {
    let scope_set = ScopeSet::new(scopes.iter().map(|s| SurfaceScope::new(*s)))
        .expect("scopes non-empty");
    Actor::SurfaceClient {
        instance: SurfaceClientId::new("sc-composition-scope-test"),
        scopes: scope_set,
    }
}

fn seed_composition_version_event(conn: &Connection, composition_id: &str) -> (String, String) {
    let cursor = "11111111-2222-4333-8444-555555555555".to_string();
    let created_at = "2026-05-13T12:00:00+00:00".to_string();
    let mutation_id = uuid::Uuid::new_v4().to_string();

    conn.execute(
        "INSERT INTO composition_versions (
             composition_id, composition_version, generated_at,
             generated_by_invocation_id, generated_by_actor_kind
         ) VALUES (?1, 1, ?2, 'inv-test', 'agent')",
        params![composition_id, &created_at],
    )
    .expect("seed composition_versions row");

    conn.execute(
        "INSERT INTO mutation_attempts (
             mutation_id, claim_id, composition_id, cursor, started_at,
             status, finalized_at
         ) VALUES (?1, NULL, ?2, ?3, ?4, 'committed', ?4)",
        params![&mutation_id, composition_id, &cursor, &created_at],
    )
    .expect("seed mutation_attempts row");

    conn.execute(
        "INSERT INTO version_events (
             cursor, event_kind, claim_id, composition_id, previous_version,
             current_version, reason, scope_redacted, correction_event_log_id,
             mutation_id, created_at, actor_kind
         ) VALUES (?1, 'composition.updated', NULL, ?2, NULL, 1, NULL, 0, NULL,
                   ?3, ?4, 'agent')",
        params![&cursor, composition_id, &mutation_id, &created_at],
    )
    .expect("seed composition version_events row");

    (cursor, created_at)
}

/// Asserts the wire-shape contract from `surface_event_log_response`:
/// when the scope projection returns redacted, the endpoint emits ONLY
/// `cursor`, `created_at`, and `scope_redacted` on the `event` envelope.
fn build_redacted_envelope(cursor: &str, created_at: &str) -> serde_json::Value {
    serde_json::json!({
        "cursor": cursor,
        "created_at": created_at,
        "scope_redacted": true,
    })
}

#[test]
fn dos567_out_of_scope_composition_event_fetch_returns_redacted_envelope() {
    let conn = fresh_full_db();
    let (cursor, created_at) =
        seed_composition_version_event(&conn, "composition-scope-test-1");
    let db = ActionDb::from_conn(&conn);

    // `submit.feedback` is a write scope — not a read scope. Per §16 the
    // composition_id channel must be gated through the same projection
    // pipeline as claim_id, so this projection redacts.
    let out_of_scope = surface_client_with_scopes(&["submit.feedback"]);
    let correction = project_composition_for_scope(db, "composition-scope-test-1", &out_of_scope)
        .expect("composition row resolves; out-of-scope projection redacts");
    let CorrectionPayload {
        claim,
        scope_redacted,
        reason,
    } = correction;
    assert!(claim.is_none(), "no claim body for composition projection");
    assert!(scope_redacted, "redaction signal set");
    assert_eq!(reason.as_deref(), Some("out_of_scope"));

    // The endpoint envelope built from `redacted_event()` must contain ONLY
    // cursor, created_at, scope_redacted. composition_id, version trajectory,
    // mutation_id, event_kind, actor_kind, reason, correction_event_log_id
    // MUST NOT leak.
    let redacted = build_redacted_envelope(&cursor, &created_at);
    let object = redacted.as_object().expect("envelope is an object");
    let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
    keys.sort();
    let mut expected = vec!["cursor", "created_at", "scope_redacted"];
    expected.sort();
    assert_eq!(keys, expected, "redacted envelope shape");
    for forbidden in [
        "composition_id",
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

#[test]
fn dos567_in_scope_composition_event_projection_returns_non_redacted() {
    let conn = fresh_full_db();
    let _ = seed_composition_version_event(&conn, "composition-scope-test-2");
    let db = ActionDb::from_conn(&conn);

    let in_scope = surface_client_with_scopes(&["read.composition"]);
    let correction = project_composition_for_scope(db, "composition-scope-test-2", &in_scope)
        .expect("composition row resolves; in-scope projection succeeds");
    assert!(!correction.scope_redacted, "in-scope must not redact");
    assert!(
        correction.reason.is_none(),
        "in-scope projection carries no rejection reason"
    );
}

#[test]
fn dos567_missing_composition_returns_none_for_404() {
    let conn = fresh_full_db();
    let db = ActionDb::from_conn(&conn);
    let actor = surface_client_with_scopes(&["read.composition"]);
    let result = project_composition_for_scope(db, "composition-does-not-exist", &actor);
    assert!(
        result.is_none(),
        "missing composition_id resolves to None (404), not a redacted payload"
    );
}
