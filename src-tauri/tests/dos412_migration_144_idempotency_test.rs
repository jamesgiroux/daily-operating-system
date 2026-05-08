use dailyos_lib::migration_test_api::run_migrations;
use rusqlite::Connection;

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

        apply_pending_from_v143(&conn, state.label());
        assert_canonical_reveal_audit_schema(&conn, state.label());
        assert_legacy_data_preserved(&conn, state.label());

        let second = run_migrations(&conn).unwrap_or_else(|error| {
            panic!("{}: second migration run failed: {error}", state.label())
        });
        assert_eq!(
            second,
            0,
            "{}: second migration run should be a no-op",
            state.label()
        );
        assert_canonical_reveal_audit_schema(&conn, state.label());
        assert_legacy_data_preserved(&conn, state.label());
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
    setup_migration_runner_state(&conn);

    apply_pending_from_v143(&conn, "partial_action_column");

    assert_canonical_reveal_audit_schema(&conn, "partial_action_column");
    assert_legacy_data_preserved(&conn, "partial_action_column");
}

#[test]
fn retry_after_partial_prior_state_preserves_tokens() {
    let conn = Connection::open_in_memory().expect("open in-memory database");
    setup_base_reveal_audit_table(&conn);
    conn.execute_batch(
        "ALTER TABLE sensitivity_reveal_audit
            ADD COLUMN reveal_action_id TEXT NOT NULL DEFAULT '';
         UPDATE sensitivity_reveal_audit
            SET reveal_action_id = 'abc-123'
            WHERE id = 7;",
    )
    .expect("create partial prior action token state");
    setup_migration_runner_state(&conn);

    apply_pending_from_v143(&conn, "partial_prior_token");

    assert_canonical_reveal_audit_schema(&conn, "partial_prior_token");
    assert_eq!(reveal_audit_row_count(&conn), 1);
    assert_eq!(single_reveal_action_id(&conn), "abc-123");
}

#[test]
fn idempotency_when_already_canonical() {
    let conn = Connection::open_in_memory().expect("open in-memory database");
    setup_base_reveal_audit_table(&conn);
    conn.execute_batch(
        "ALTER TABLE sensitivity_reveal_audit
            ADD COLUMN reveal_action_id TEXT NOT NULL DEFAULT '';
         UPDATE sensitivity_reveal_audit
            SET reveal_action_id = 'abc-123'
            WHERE id = 7;
         CREATE UNIQUE INDEX idx_sensitivity_reveal_audit_action_token
            ON sensitivity_reveal_audit(claim_id, user_id, reveal_action_id)
            WHERE reveal_action_id != '';",
    )
    .expect("create canonical action token state");
    setup_migration_runner_state(&conn);
    let before_columns = reveal_audit_columns(&conn);
    let before_index_sql = reveal_action_index_sql(&conn);

    apply_pending_from_v143(&conn, "already_canonical");

    assert_eq!(reveal_audit_columns(&conn), before_columns);
    assert_eq!(reveal_action_index_sql(&conn), before_index_sql);
    assert_eq!(reveal_audit_row_count(&conn), 1);
    assert_eq!(single_reveal_action_id(&conn), "abc-123");
    assert_eq!(schema_version(&conn), 145);
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

    setup_migration_runner_state(conn);
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

fn setup_migration_runner_state(conn: &Connection) {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        INSERT INTO schema_version (version) VALUES (143);

        CREATE TABLE IF NOT EXISTS meetings (id TEXT PRIMARY KEY);
        CREATE TABLE IF NOT EXISTS meeting_prep (id TEXT PRIMARY KEY);
        CREATE TABLE IF NOT EXISTS meeting_transcripts (id TEXT PRIMARY KEY);
        CREATE TABLE IF NOT EXISTS account_stakeholders (
            id TEXT PRIMARY KEY,
            data_source TEXT
        );
        CREATE TABLE IF NOT EXISTS entity_assessment (
            id TEXT PRIMARY KEY,
            health_json TEXT,
            org_health_json TEXT,
            dimensions_json TEXT,
            success_plan_signals_json TEXT
        );
        CREATE TABLE IF NOT EXISTS entity_quality (
            id TEXT PRIMARY KEY,
            health_score REAL,
            health_trend TEXT,
            coherence_score REAL,
            coherence_flagged INTEGER
        );
        CREATE TABLE IF NOT EXISTS person_relationships (
            id TEXT PRIMARY KEY,
            rationale TEXT
        );
        CREATE TABLE IF NOT EXISTS email_signals (
            id TEXT PRIMARY KEY,
            source TEXT
        );
        CREATE TABLE IF NOT EXISTS entities (
            id TEXT PRIMARY KEY,
            entity_type TEXT NOT NULL DEFAULT 'project'
        );
        CREATE TABLE IF NOT EXISTS entity_members (
            entity_id TEXT NOT NULL,
            person_id TEXT NOT NULL,
            relationship_type TEXT DEFAULT 'associated',
            PRIMARY KEY (entity_id, person_id)
        );",
    )
    .expect("create migration runner fixture state");
}

fn apply_pending_from_v143(conn: &Connection, label: &str) {
    let applied = run_migrations(conn)
        .unwrap_or_else(|error| panic!("{label}: migration runner failed: {error}"));
    assert_eq!(
        applied, 2,
        "{label}: v144 and v145 should be the pending migrations"
    );
    assert_eq!(
        schema_version(conn),
        145,
        "{label}: latest schema version should be recorded"
    );
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

    let index_sql = reveal_action_index_sql(conn);
    assert!(
        index_sql.contains("WHERE reveal_action_id != ''"),
        "{label}: action token unique index must stay partial"
    );
}

fn reveal_action_index_sql(conn: &Connection) -> String {
    conn.query_row(
        "SELECT sql
         FROM sqlite_master
         WHERE type = 'index'
           AND name = 'idx_sensitivity_reveal_audit_action_token'",
        [],
        |row| row.get(0),
    )
    .expect("read reveal audit index SQL")
}

fn reveal_audit_row_count(conn: &Connection) -> i64 {
    conn.query_row("SELECT COUNT(*) FROM sensitivity_reveal_audit", [], |row| {
        row.get(0)
    })
    .expect("read reveal audit row count")
}

fn single_reveal_action_id(conn: &Connection) -> String {
    conn.query_row(
        "SELECT reveal_action_id FROM sensitivity_reveal_audit WHERE id = 7",
        [],
        |row| row.get(0),
    )
    .expect("read reveal action id")
}

fn schema_version(conn: &Connection) -> i64 {
    conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_version",
        [],
        |row| row.get(0),
    )
    .expect("read schema version")
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
