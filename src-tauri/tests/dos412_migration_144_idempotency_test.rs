use rusqlite::Connection;

const MIGRATION_144_SQL: &str =
    include_str!("../src/migrations/144_sensitivity_reveal_audit_action_token.sql");

#[derive(Clone, Copy)]
enum StartingState {
    FreshNoV143,
    LegacyRevealSession,
    AuditBucket,
}

impl StartingState {
    fn label(self) -> &'static str {
        match self {
            Self::FreshNoV143 => "fresh_no_v143",
            Self::LegacyRevealSession => "legacy_reveal_session",
            Self::AuditBucket => "audit_bucket",
        }
    }
}

#[test]
fn migration_144_rebuilds_prior_audit_schemas_to_canonical_shape() {
    for state in [
        StartingState::FreshNoV143,
        StartingState::LegacyRevealSession,
        StartingState::AuditBucket,
    ] {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        setup_starting_state(&conn, state);

        conn.execute_batch(MIGRATION_144_SQL)
            .unwrap_or_else(|error| {
                panic!("{}: first migration apply failed: {error}", state.label())
            });
        assert_canonical_reveal_audit_schema(&conn, state.label());

        conn.execute_batch(MIGRATION_144_SQL)
            .unwrap_or_else(|error| {
                panic!("{}: second migration apply failed: {error}", state.label())
            });
        assert_canonical_reveal_audit_schema(&conn, state.label());
    }
}

#[test]
fn migration_144_repairs_partial_prior_action_column_without_index() {
    let conn = Connection::open_in_memory().expect("open in-memory database");
    setup_base_reveal_audit_table(&conn);
    conn.execute_batch(
        "ALTER TABLE sensitivity_reveal_audit
            ADD COLUMN reveal_action_id TEXT NOT NULL DEFAULT '';",
    )
    .expect("create partial prior action column state");

    conn.execute_batch(MIGRATION_144_SQL)
        .expect("migration repairs partial prior action column state");

    assert_canonical_reveal_audit_schema(&conn, "partial_action_column");
}

fn setup_starting_state(conn: &Connection, state: StartingState) {
    setup_base_reveal_audit_table(conn);

    match state {
        StartingState::FreshNoV143 => {}
        StartingState::LegacyRevealSession => conn
            .execute_batch(
                "ALTER TABLE sensitivity_reveal_audit
                    ADD COLUMN reveal_session_id TEXT NOT NULL DEFAULT '';
                 CREATE UNIQUE INDEX idx_sensitivity_reveal_audit_reveal_session
                    ON sensitivity_reveal_audit(claim_id, user_id, reveal_session_id)
                    WHERE reveal_session_id != '';",
            )
            .expect("create legacy reveal_session_id state"),
        StartingState::AuditBucket => conn
            .execute_batch(
                "ALTER TABLE sensitivity_reveal_audit
                    ADD COLUMN audit_bucket TEXT NOT NULL DEFAULT '';
                 CREATE UNIQUE INDEX idx_sensitivity_reveal_audit_audit_bucket
                    ON sensitivity_reveal_audit(claim_id, user_id, audit_bucket)
                    WHERE audit_bucket != '';",
            )
            .expect("create audit_bucket state"),
    }
}

fn setup_base_reveal_audit_table(conn: &Connection) {
    conn.execute_batch(
        "CREATE TABLE sensitivity_reveal_audit (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            claim_id TEXT NOT NULL,
            user_id TEXT NOT NULL,
            revealed_at TEXT NOT NULL
        );
        CREATE INDEX idx_sensitivity_reveal_audit_claim
            ON sensitivity_reveal_audit(claim_id, revealed_at);
        CREATE INDEX idx_sensitivity_reveal_audit_user
            ON sensitivity_reveal_audit(user_id, revealed_at);
        INSERT INTO sensitivity_reveal_audit (id, claim_id, user_id, revealed_at)
            VALUES (7, 'claim-144', 'user-144', '2026-05-07T12:00:00Z');",
    )
    .expect("create base reveal audit table");
}

fn assert_canonical_reveal_audit_schema(conn: &Connection, label: &str) {
    assert_eq!(
        reveal_audit_columns(conn),
        vec![
            "id".to_string(),
            "claim_id".to_string(),
            "user_id".to_string(),
            "revealed_at".to_string(),
            "reveal_action_id".to_string(),
        ],
        "{label}: reveal audit columns should be canonical"
    );
    assert_index_missing(conn, "idx_sensitivity_reveal_audit_reveal_session", label);
    assert_index_missing(conn, "idx_sensitivity_reveal_audit_audit_bucket", label);
    assert_index_exists(conn, "idx_sensitivity_reveal_audit_claim", false, label);
    assert_index_exists(conn, "idx_sensitivity_reveal_audit_user", false, label);
    assert_reveal_action_id_unique_index(conn, label);
    assert_legacy_data_preserved(conn, label);
}

fn reveal_audit_columns(conn: &Connection) -> Vec<String> {
    let mut stmt = conn
        .prepare("PRAGMA table_info(sensitivity_reveal_audit)")
        .expect("query audit schema");
    stmt.query_map([], |row| row.get::<_, String>(1))
        .expect("read columns")
        .map(|row| row.expect("column row"))
        .collect()
}

fn assert_index_exists(conn: &Connection, index_name: &str, unique: bool, label: &str) {
    let expected_unique = i64::from(unique);
    let index_count: i64 = conn
        .query_row(
            "SELECT COUNT(*)
             FROM pragma_index_list('sensitivity_reveal_audit')
             WHERE name = ?1
               AND [unique] = ?2",
            rusqlite::params![index_name, expected_unique],
            |row| row.get(0),
        )
        .expect("read reveal audit indexes");
    assert_eq!(index_count, 1, "{label}: expected index {index_name}");
}

fn assert_index_missing(conn: &Connection, index_name: &str, label: &str) {
    let index_count: i64 = conn
        .query_row(
            "SELECT COUNT(*)
             FROM pragma_index_list('sensitivity_reveal_audit')
             WHERE name = ?1",
            [index_name],
            |row| row.get(0),
        )
        .expect("read reveal audit indexes");
    assert_eq!(index_count, 0, "{label}: legacy index should be absent");
}

fn assert_reveal_action_id_unique_index(conn: &Connection, label: &str) {
    assert_index_exists(
        conn,
        "idx_sensitivity_reveal_audit_action_token",
        true,
        label,
    );

    let indexed_columns = index_columns(conn, "idx_sensitivity_reveal_audit_action_token");
    assert_eq!(
        indexed_columns,
        vec![
            "claim_id".to_string(),
            "user_id".to_string(),
            "reveal_action_id".to_string(),
        ],
        "{label}: action token unique index columns"
    );

    let index_sql: String = conn
        .query_row(
            "SELECT sql
             FROM sqlite_master
             WHERE type = 'index'
               AND name = 'idx_sensitivity_reveal_audit_action_token'",
            [],
            |row| row.get(0),
        )
        .expect("read reveal audit index SQL");
    assert!(
        index_sql.contains("WHERE reveal_action_id != ''"),
        "{label}: action token unique index must stay partial"
    );
}

fn index_columns(conn: &Connection, index_name: &str) -> Vec<String> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA index_info('{index_name}')"))
        .expect("query index columns");
    stmt.query_map([], |row| row.get::<_, String>(2))
        .expect("read index columns")
        .map(|row| row.expect("index column row"))
        .collect()
}

fn assert_legacy_data_preserved(conn: &Connection, label: &str) {
    let row = conn
        .query_row(
            "SELECT id, claim_id, user_id, revealed_at, reveal_action_id
             FROM sensitivity_reveal_audit",
            [],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )
        .expect("read migrated legacy row");

    assert_eq!(
        row,
        (
            7,
            "claim-144".to_string(),
            "user-144".to_string(),
            "2026-05-07T12:00:00Z".to_string(),
            String::new(),
        ),
        "{label}: legacy row should be preserved with empty action token"
    );
}
