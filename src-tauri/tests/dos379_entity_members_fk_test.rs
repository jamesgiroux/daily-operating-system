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

#[test]
#[ignore = "v144 fixture omits actions schema and ai_commitment_bridge columns that the post-v155 commitment-identity migration chain ALTERs; see Codebase Maintenance project for the migration target-version refactor"]
fn migration_145_preserves_project_memberships_and_surfaces_unrecoverable_orphans() {
    let conn = Connection::open_in_memory().expect("open in-memory database");
    setup_v144_migration_state(&conn);

    conn.execute(
        "INSERT INTO projects (id, name, tracker_path, updated_at)
         VALUES (?1, ?2, ?3, ?4)",
        params![
            "project-without-entity",
            "Legacy Project",
            "/projects/legacy",
            "2026-05-08T12:00:00Z"
        ],
    )
    .expect("seed project row without entity mirror");
    conn.execute(
        "INSERT INTO entity_members (entity_id, person_id, relationship_type)
         VALUES (?1, ?2, ?3)",
        params!["project-without-entity", "person-project", "member"],
    )
    .expect("seed recoverable project membership");
    conn.execute(
        "INSERT INTO entity_members (entity_id, person_id, relationship_type)
         VALUES (?1, ?2, ?3)",
        params!["missing-entity", "person-orphan", "reviewer"],
    )
    .expect("seed unrecoverable membership");

    let applied = run_migrations(&conn).expect("apply migration 145");
    assert!(
        applied >= 1,
        "expected at least migration 145, got {applied}"
    );

    let recovered_relationship: String = conn
        .query_row(
            "SELECT relationship_type
             FROM entity_members
             WHERE entity_id = ?1 AND person_id = ?2",
            params!["project-without-entity", "person-project"],
            |row| row.get(0),
        )
        .expect("project membership should survive migration");
    assert_eq!(recovered_relationship, "member");

    let recovered_entity: (String, String, Option<String>) = conn
        .query_row(
            "SELECT name, entity_type, tracker_path
             FROM entities
             WHERE id = ?1",
            params!["project-without-entity"],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("missing project entity mirror should be backfilled");
    assert_eq!(
        recovered_entity,
        (
            "Legacy Project".to_string(),
            "project".to_string(),
            Some("/projects/legacy".to_string())
        )
    );

    let active_orphan_count: i64 = conn
        .query_row(
            "SELECT COUNT(*)
             FROM entity_members
             WHERE entity_id = ?1 AND person_id = ?2",
            params!["missing-entity", "person-orphan"],
            |row| row.get(0),
        )
        .expect("count active orphan rows");
    assert_eq!(active_orphan_count, 0);

    let surfaced_orphan: (String, String) = conn
        .query_row(
            "SELECT relationship_type, reason
             FROM entity_members_migration_145_orphans
             WHERE entity_id = ?1 AND person_id = ?2",
            params!["missing-entity", "person-orphan"],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("unrecoverable membership should be surfaced for triage");
    assert_eq!(
        surfaced_orphan,
        (
            "reviewer".to_string(),
            "missing_entity_after_project_mirror_backfill".to_string()
        )
    );
}

#[test]
#[ignore = "v144 fixture omits actions schema and ai_commitment_bridge columns that the post-v155 commitment-identity migration chain ALTERs; see Codebase Maintenance project for the migration target-version refactor"]
fn migration_145_mirrors_zero_member_legacy_projects() {
    let conn = Connection::open_in_memory().expect("open in-memory database");
    setup_v144_migration_state(&conn);

    conn.execute(
        "INSERT INTO projects (id, name, tracker_path, updated_at)
         VALUES (?1, ?2, ?3, ?4)",
        params![
            "project-zero-members",
            "Legacy Zero Member Project",
            "/projects/zero-members",
            "2026-05-08T13:00:00Z"
        ],
    )
    .expect("seed zero-member project row without entity mirror");

    let applied = run_migrations(&conn).expect("apply migration 145");
    assert!(
        applied >= 1,
        "expected at least migration 145, got {applied}"
    );

    let mirrored_entity: (String, String, Option<String>) = conn
        .query_row(
            "SELECT name, entity_type, tracker_path
             FROM entities
             WHERE id = ?1",
            params!["project-zero-members"],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("zero-member legacy project should be mirrored into entities");
    assert_eq!(
        mirrored_entity,
        (
            "Legacy Zero Member Project".to_string(),
            "project".to_string(),
            Some("/projects/zero-members".to_string())
        )
    );

    let member_count: i64 = conn
        .query_row(
            "SELECT COUNT(*)
             FROM entity_members
             WHERE entity_id = ?1",
            params!["project-zero-members"],
            |row| row.get(0),
        )
        .expect("count zero-member project memberships");
    assert_eq!(member_count, 0);
}

fn setup_v144_migration_state(conn: &Connection) {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        INSERT INTO schema_version (version) VALUES (144);

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
        CREATE TABLE IF NOT EXISTS projects (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            tracker_path TEXT,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS entities (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            entity_type TEXT NOT NULL DEFAULT 'account',
            tracker_path TEXT,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS entity_members (
            entity_id TEXT NOT NULL,
            person_id TEXT NOT NULL,
            relationship_type TEXT DEFAULT 'associated',
            PRIMARY KEY (entity_id, person_id)
        );
        CREATE TABLE IF NOT EXISTS signal_events (
            id TEXT PRIMARY KEY,
            entity_type TEXT NOT NULL,
            entity_id TEXT NOT NULL,
            signal_type TEXT NOT NULL,
            source TEXT NOT NULL,
            value TEXT,
            confidence REAL DEFAULT 1.0,
            decay_half_life_days INTEGER DEFAULT 90,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            superseded_by TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_signal_events_source
            ON signal_events(source, signal_type);",
    )
    .expect("create v144 migration fixture state");
}
