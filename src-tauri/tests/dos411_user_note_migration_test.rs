#![cfg(feature = "test-harness")]

use rusqlite::Connection;

fn seed_legacy_entry(conn: &Connection, id: &str, title: &str, content: &str) {
    conn.execute_batch("PRAGMA ignore_check_constraints = ON;")
        .expect("disable check constraints for legacy seed");
    conn.execute(
        "INSERT INTO entity_context_entries
         (id, entity_type, entity_id, title, content, created_at, updated_at)
         VALUES (?1, 'account', 'acct-dos411-migration', ?2, ?3, '2026-05-06 12:00:00', '2026-05-06 12:00:00')",
        rusqlite::params![id, title, content],
    )
    .expect("seed legacy entity_context_entries row");
    conn.execute_batch("PRAGMA ignore_check_constraints = OFF;")
        .expect("restore check constraints");
}

fn make_migration_141_pending(conn: &Connection) {
    conn.execute("DELETE FROM schema_version WHERE version >= 141", [])
        .expect("make migration 141 pending");
}

#[test]
fn user_note_backfill_is_idempotent_and_freezes_legacy_writes() {
    let conn = Connection::open_in_memory().expect("open db");
    dailyos_lib::migration_test_api::run_migrations(&conn).expect("apply baseline migrations");

    conn.execute("DELETE FROM legacy_user_note_migration_audit", [])
        .expect("clear audit");
    conn.execute(
        "DELETE FROM intelligence_claims WHERE claim_type = 'user_note'",
        [],
    )
    .expect("clear user_note claims");

    seed_legacy_entry(
        &conn,
        "legacy-note-a",
        "Migration note A",
        "First legacy note",
    );
    seed_legacy_entry(
        &conn,
        "legacy-note-b",
        "Migration note B",
        "Second legacy note",
    );

    make_migration_141_pending(&conn);
    let applied =
        dailyos_lib::migration_test_api::run_migrations(&conn).expect("apply migration 141");
    assert!(
        applied >= 1,
        "expected migration 141 window to apply at least one migration"
    );

    let claim_count: i64 = conn
        .query_row(
            "SELECT count(*) FROM intelligence_claims
             WHERE claim_type = 'user_note'
               AND claim_state = 'active'
               AND surfacing_state = 'active'",
            [],
            |row| row.get(0),
        )
        .expect("count user_note claims");
    assert_eq!(claim_count, 2);

    let audit_count: i64 = conn
        .query_row(
            "SELECT count(*) FROM legacy_user_note_migration_audit",
            [],
            |row| row.get(0),
        )
        .expect("count audit rows");
    assert_eq!(audit_count, 2);

    let migrated: (String, String, String, String, f64) = conn
        .query_row(
            "SELECT ic.text, ic.actor, ic.data_source, ic.sensitivity, ic.trust_score
             FROM intelligence_claims ic
             JOIN legacy_user_note_migration_audit audit ON audit.claim_id = ic.id
             WHERE audit.legacy_entry_id = 'legacy-note-a'",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .expect("read migrated user_note");
    assert_eq!(
        migrated,
        (
            "First legacy note".to_string(),
            "user".to_string(),
            "manual".to_string(),
            "internal".to_string(),
            0.85,
        )
    );

    make_migration_141_pending(&conn);
    dailyos_lib::migration_test_api::run_migrations(&conn)
        .expect("re-apply migration 141 idempotently");

    let claim_count_after_rerun: i64 = conn
        .query_row(
            "SELECT count(*) FROM intelligence_claims WHERE claim_type = 'user_note'",
            [],
            |row| row.get(0),
        )
        .expect("count user_note claims after rerun");
    assert_eq!(claim_count_after_rerun, 2);

    let blocked = conn.execute(
        "INSERT INTO entity_context_entries
         (id, entity_type, entity_id, title, content)
         VALUES ('blocked-note', 'account', 'acct-dos411-migration', 'Blocked', 'Blocked')",
        [],
    );
    assert!(
        blocked.is_err(),
        "legacy entity_context_entries inserts must be CHECK-blocked"
    );
}
