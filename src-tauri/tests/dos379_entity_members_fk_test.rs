use dailyos_lib::migration_test_api::run_migrations;
use rusqlite::{params, Connection};

#[test]
fn migration_145_enforces_entity_members_entity_id_fk() {
    let conn = Connection::open_in_memory().expect("open in-memory database");
    run_migrations(&conn).expect("apply migrations");

    let fk_count: i64 = conn
        .query_row(
            "SELECT COUNT(*)
             FROM pragma_foreign_key_list('entity_members')
             WHERE \"table\" = 'entities'
               AND \"from\" = 'entity_id'
               AND \"to\" = 'id'
               AND on_delete = 'CASCADE'",
            [],
            |row| row.get(0),
        )
        .expect("inspect entity_members foreign keys");
    assert_eq!(fk_count, 1);

    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .expect("enable FK enforcement");
    conn.execute(
        "INSERT INTO entity_members (entity_id, person_id, relationship_type)
         VALUES (?1, ?2, 'member')",
        params!["missing-entity", "person-1"],
    )
    .expect_err("orphan entity_members.entity_id should be rejected");
}
