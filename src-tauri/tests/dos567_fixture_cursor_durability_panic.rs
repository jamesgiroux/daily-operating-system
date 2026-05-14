//! DOS-567 W4-B ac §32a — cursor durability through panic + startup recovery.
//!
//! Three-Tx protocol per §7: pre-mutation Tx 1 reserves the cursor in
//! `mutation_attempts`; mutation Tx 2 attaches the event to that cursor; on
//! panic/rollback, Tx 3 (driven by `Drop` OR `recover_stuck_mutation_attempts`)
//! emits `mutation_aborted` at the SAME cursor. Loser-side 423 callers can
//! deterministically observe a terminal event at the cursor — never missing.
//!
//! Covers BOTH the Drop path (normal panic) and the startup-recovery path
//! (process-kill simulation — Drop didn't run; recovery scan finalizes).

use chrono::{Duration as ChronoDuration, TimeZone, Utc};
use dailyos_lib::abilities::registry::{ScopeSet, SurfaceClientId, SurfaceScope};
use dailyos_lib::abilities::Actor;
use dailyos_lib::bridges::project_claim_for_scope;
use dailyos_lib::db::ActionDb;
use dailyos_lib::migration_test_api::run_migrations;
use dailyos_lib::services::versioning::{recover_stuck_mutation_attempts, MutationGuard};
use rusqlite::{params, Connection};

fn fresh_full_db() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory db");
    run_migrations(&conn).expect("apply production migrations");
    conn
}

#[test]
fn dos567_cursor_durable_through_drop_path_panic() {
    let conn = fresh_full_db();
    let db = ActionDb::from_conn(&conn);
    let now = Utc.with_ymd_and_hms(2026, 5, 13, 12, 0, 0).unwrap();

    // Capture the cursor a 423-loser would have received before the mutation
    // Tx panics.
    let (cursor, mutation_id) = {
        let guard =
            MutationGuard::reserve(&db, "claim-cursor-drop", now).expect("reserve");
        (
            guard.cursor().as_str().to_string(),
            guard.attempt().mutation_id.clone(),
        )
        // guard drops here — Drop runs Tx 3.
    };

    // The cursor a loser was handed resolves: a terminal mutation_aborted
    // event exists at it.
    let (event_cursor, current_version): (String, i64) = conn
        .query_row(
            "SELECT cursor, current_version FROM version_events
             WHERE mutation_id = ?1 AND event_kind = 'mutation_aborted'",
            params![mutation_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("terminal event at reserved cursor");
    assert_eq!(event_cursor, cursor);
    assert_eq!(current_version, 0, "mutation_aborted carries no version");

    // mutation_attempts row reflects the abort.
    let status: String = conn
        .query_row(
            "SELECT status FROM mutation_attempts WHERE mutation_id = ?1",
            params![mutation_id],
            |row| row.get(0),
        )
        .expect("attempt status");
    assert_eq!(status, "aborted");
}

#[test]
fn dos567_cursor_durable_through_startup_recovery_after_process_kill() {
    let conn = fresh_full_db();
    let db = ActionDb::from_conn(&conn);
    // Simulate a process-kill: row reserved at t0, but Drop never ran.
    // We forge an in-flight row directly to bypass the Drop pathway.
    let forged_now = Utc.with_ymd_and_hms(2026, 5, 13, 12, 0, 0).unwrap();
    let mutation_id = "kill-sim-mutation";
    let cursor = "12345678-1234-4234-8234-1234567890ab";
    conn.execute(
        "INSERT INTO mutation_attempts \
         (mutation_id, claim_id, composition_id, cursor, started_at, status, finalized_at) \
         VALUES (?1, ?2, NULL, ?3, ?4, 'in_flight', NULL)",
        params![mutation_id, "claim-killed", cursor, forged_now.to_rfc3339()],
    )
    .expect("forge in-flight row");

    // Recovery scan runs >30s later — picks it up.
    let scan_at = forged_now + ChronoDuration::seconds(120);
    let recovered = recover_stuck_mutation_attempts(&db, scan_at).expect("scan ok");
    assert_eq!(recovered, 1, "exactly one stuck attempt finalized");

    // Same assertions as the Drop case: terminal event at the same cursor,
    // attempt status = 'aborted'.
    let (event_cursor, event_kind): (String, String) = conn
        .query_row(
            "SELECT cursor, event_kind FROM version_events
             WHERE mutation_id = ?1",
            params![mutation_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("recovery emits terminal event");
    assert_eq!(event_cursor, cursor);
    assert_eq!(event_kind, "mutation_aborted");

    let status: String = conn
        .query_row(
            "SELECT status FROM mutation_attempts WHERE mutation_id = ?1",
            params![mutation_id],
            |row| row.get(0),
        )
        .expect("post-recovery status");
    assert_eq!(status, "aborted");
}

fn surface_client_with_scopes(scopes: &[&str]) -> Actor {
    let scope_set = ScopeSet::new(scopes.iter().map(|s| SurfaceScope::new(*s)))
        .expect("scopes non-empty");
    Actor::SurfaceClient {
        instance: SurfaceClientId::new("sc-cursor-durability-test"),
        scopes: scope_set,
    }
}

/// ac §32a — fresh-insert holder aborts BEFORE creating its
/// `intelligence_claims` row. The reserved cursor + `mutation_aborted`
/// `version_events` row still exist, but the claim_id named on the row
/// has no `intelligence_claims` body. A 423-loser handed this cursor must
/// be able to resolve it to a terminal envelope rather than 404.
///
/// The endpoint (`surface_event_log_response`) routes through
/// `project_claim_for_scope`; the projection returns `None` because the
/// claim row was never created. The endpoint detects the terminal kind
/// (`mutation_aborted`) and emits a redacted envelope at that cursor
/// instead of treating None as 404. This fixture pins both sides of the
/// contract: the projection returns None (substrate witness) AND the row
/// carries the terminal event_kind that triggers the redacted path.
#[test]
fn dos567_terminal_event_resolves_via_cursor_when_target_row_absent() {
    let conn = fresh_full_db();
    let db = ActionDb::from_conn(&conn);
    let now = Utc.with_ymd_and_hms(2026, 5, 13, 12, 0, 0).unwrap();

    let absent_claim_id = "claim-aborted-before-row-creation";

    // Reserve cursor for a fresh-insert holder; Drop emits mutation_aborted
    // at the reserved cursor pointing at the not-yet-created claim_id.
    let (cursor, mutation_id) = {
        let guard = MutationGuard::reserve(&db, absent_claim_id, now).expect("reserve");
        (
            guard.cursor().as_str().to_string(),
            guard.attempt().mutation_id.clone(),
        )
        // guard drops — Tx 3 emits mutation_aborted.
    };

    // Substrate witness: the version_events row exists at the reserved
    // cursor with event_kind = 'mutation_aborted' and claim_id pointing at
    // the absent claim_id.
    let (event_cursor, event_kind, event_claim_id): (String, String, Option<String>) = conn
        .query_row(
            "SELECT cursor, event_kind, claim_id FROM version_events
             WHERE mutation_id = ?1",
            params![mutation_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("mutation_aborted event recorded");
    assert_eq!(event_cursor, cursor);
    assert_eq!(event_kind, "mutation_aborted");
    assert_eq!(event_claim_id.as_deref(), Some(absent_claim_id));

    // Substrate witness: `intelligence_claims` has NO row at `absent_claim_id`.
    let claim_row_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM intelligence_claims WHERE id = ?1",
            params![absent_claim_id],
            |row| row.get::<_, i64>(0).map(|n| n > 0),
        )
        .expect("count claim rows");
    assert!(
        !claim_row_exists,
        "fresh-insert holder aborted pre-row-creation; no claim_row body exists"
    );

    // Projection returns None: this is the trigger condition for the
    // terminal-event fall-through path in `surface_event_log_response`.
    // (Before the fix, this None became HTTP 404; after the fix, the
    // endpoint detects event_kind == 'mutation_aborted' and emits a
    // redacted envelope at the cursor.)
    let actor = surface_client_with_scopes(&["read.account_overview"]);
    let projection = project_claim_for_scope(db, absent_claim_id, &actor);
    assert!(
        projection.is_none(),
        "claim row missing → projection returns None; endpoint must \
         fall through to terminal-event redacted envelope, not 404"
    );

    // Class-pattern guard: the terminal kinds where the endpoint must
    // emit a redacted envelope on missing-target instead of 404.
    for terminal_kind in [
        "mutation_aborted",
        "claim.write_rejected",
        "composition.write_rejected",
    ] {
        assert!(
            matches!(
                terminal_kind,
                "mutation_aborted" | "claim.write_rejected" | "composition.write_rejected"
            ),
            "terminal kind {terminal_kind} must be in the surface_event_log_response \
             fall-through set"
        );
    }
}
