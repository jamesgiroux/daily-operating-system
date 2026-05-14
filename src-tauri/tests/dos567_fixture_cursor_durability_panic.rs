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
