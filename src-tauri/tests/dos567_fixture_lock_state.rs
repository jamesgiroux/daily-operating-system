//! W4-B ac §30 — MutationGuard `Drop` finalizes aborted attempts.
//!
//! The MutationGuard wraps the `mutation_attempts` reservation; if the
//! mutation Tx panics or returns Err without `mark_completed`, the `Drop`
//! impl runs Tx 3 (post-rollback) and finalizes the row to `aborted` while
//! emitting a terminal `mutation_aborted` event at the reserved cursor. No
//! permanent in-flight rows are left behind.
//!
//! We exercise Drop by dropping a guard without marking it completed.

use chrono::{TimeZone, Utc};
use dailyos_lib::db::ActionDb;
use dailyos_lib::migration_test_api::run_migrations;
use dailyos_lib::services::versioning::MutationGuard;
use rusqlite::{params, Connection};

fn fresh_full_db() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory db");
    run_migrations(&conn).expect("apply production migrations");
    conn
}

#[test]
fn dos567_mutation_guard_drop_finalizes_aborted_attempt_and_emits_terminal_event() {
    let conn = fresh_full_db();
    let db = ActionDb::from_conn(&conn);
    let now = Utc.with_ymd_and_hms(2026, 5, 13, 12, 0, 0).unwrap();

    // Reserve cursor through Tx 1, then drop the guard mid-flight (no
    // mark_completed) — simulates panic-unwind in the mutation Tx 2.
    let (cursor, mutation_id) = {
        let guard = MutationGuard::reserve(&db, "claim-drop-test", now).expect("reserve mutation");
        let cursor = guard.cursor().as_str().to_string();
        let mutation_id = guard.attempt().mutation_id.clone();

        // Sanity: in_flight row exists before drop.
        let status: String = conn
            .query_row(
                "SELECT status FROM mutation_attempts WHERE mutation_id = ?1",
                params![mutation_id],
                |row| row.get(0),
            )
            .expect("in-flight row visible");
        assert_eq!(status, "in_flight");

        (cursor, mutation_id)
        // guard drops here -> finalize_mutation_attempt_aborted runs.
    };

    // Post-drop: mutation_attempts row marked aborted, finalized_at populated.
    let (status, finalized_at): (String, Option<String>) = conn
        .query_row(
            "SELECT status, finalized_at FROM mutation_attempts WHERE mutation_id = ?1",
            params![mutation_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("attempt row");
    assert_eq!(status, "aborted");
    assert!(
        finalized_at.is_some(),
        "Drop populates finalized_at consistent with status='aborted'"
    );

    // version_events terminal row at the reserved cursor.
    let (event_kind, event_cursor, reason): (String, String, Option<String>) = conn
        .query_row(
            "SELECT event_kind, cursor, reason FROM version_events
             WHERE mutation_id = ?1 AND event_kind = 'mutation_aborted'",
            params![mutation_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("terminal mutation_aborted event");
    assert_eq!(event_kind, "mutation_aborted");
    assert_eq!(event_cursor, cursor);
    assert_eq!(reason.as_deref(), Some("mutation_aborted"));

    // No zombie in_flight rows.
    let in_flight: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM mutation_attempts WHERE status = 'in_flight'",
            [],
            |row| row.get(0),
        )
        .expect("count");
    assert_eq!(in_flight, 0);
}
