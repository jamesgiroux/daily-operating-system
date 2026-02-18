//! Schema migration framework (ADR-0071).
//!
//! Numbered SQL migrations are embedded at compile time via `include_str!`.
//! Each migration runs exactly once, tracked by the `schema_version` table.
//!
//! For existing databases (pre-migration-framework), the bootstrap function
//! detects the presence of known tables and marks migration 001 as applied
//! so the baseline SQL never runs against an already-populated database.

use rusqlite::Connection;

struct Migration {
    version: i32,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    sql: include_str!("migrations/001_baseline.sql"),
}, Migration {
    version: 2,
    sql: include_str!("migrations/002_internal_teams.sql"),
}, Migration {
    version: 3,
    sql: include_str!("migrations/003_account_team.sql"),
}, Migration {
    version: 4,
    sql: include_str!("migrations/004_account_team_role_index.sql"),
}, Migration {
    version: 5,
    sql: include_str!("migrations/005_email_signals.sql"),
}, Migration {
    version: 6,
    sql: include_str!("migrations/006_content_embeddings.sql"),
}, Migration {
    version: 7,
    sql: include_str!("migrations/007_chat_interface.sql"),
}, Migration {
    version: 8,
    sql: include_str!("migrations/008_missing_indexes.sql"),
}, Migration {
    version: 9,
    sql: include_str!("migrations/009_fix_embeddings_column.sql"),
}, Migration {
    version: 10,
    sql: include_str!("migrations/010_foreign_keys.sql"),
}, Migration {
    version: 11,
    sql: include_str!("migrations/011_proposed_actions.sql"),
}, Migration {
    version: 12,
    sql: include_str!("migrations/012_person_emails.sql"),
}, Migration {
    version: 13,
    sql: include_str!("migrations/013_quill_sync.sql"),
}, Migration {
    version: 14,
    sql: include_str!("migrations/014_gravatar_cache.sql"),
}];

/// Create the `schema_version` table if it doesn't exist.
fn ensure_schema_version_table(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )
    .map_err(|e| format!("Failed to create schema_version table: {}", e))
}

/// Return the highest applied migration version, or 0 if none.
fn current_version(conn: &Connection) -> Result<i32, String> {
    conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_version",
        [],
        |row| row.get(0),
    )
    .map_err(|e| format!("Failed to read schema version: {}", e))
}

/// Detect a pre-framework database and mark the baseline as applied.
///
/// If the `actions` table exists but `schema_version` does not, this is a
/// database created before the migration framework was introduced. We mark
/// migration 001 (the baseline) as applied so its CREATE TABLE statements
/// never run against an already-populated database.
fn bootstrap_existing_db(conn: &Connection) -> Result<bool, String> {
    // Check if schema_version already has rows (framework already in use)
    let version = current_version(conn)?;
    if version > 0 {
        return Ok(false);
    }

    // Check if this is an existing database (has the actions table with data)
    let has_actions: bool = conn
        .prepare("SELECT 1 FROM actions LIMIT 1")
        .and_then(|mut stmt| stmt.exists([]))
        .unwrap_or(false);

    if has_actions {
        // Existing database — mark baseline as applied
        conn.execute(
            "INSERT OR IGNORE INTO schema_version (version) VALUES (?1)",
            [1],
        )
        .map_err(|e| format!("Failed to bootstrap schema version: {}", e))?;
        log::info!("Migration bootstrap: marked v1 (baseline) as applied for existing database");
        return Ok(true);
    }

    Ok(false)
}

/// Back up the database before applying migrations.
///
/// Uses SQLite's online backup API to create a hot copy at
/// `<db_path>.pre-migration.bak`. Only called when there are pending migrations.
fn backup_before_migration(conn: &Connection) -> Result<(), String> {
    let db_path: String = conn
        .query_row("PRAGMA database_list", [], |row| row.get(2))
        .map_err(|e| format!("Failed to get database path: {}", e))?;

    if db_path.is_empty() || db_path == ":memory:" {
        // In-memory or temp database — skip backup
        return Ok(());
    }

    let backup_path = format!("{}.pre-migration.bak", db_path);
    let mut backup_conn = rusqlite::Connection::open(&backup_path)
        .map_err(|e| format!("Failed to open backup file: {}", e))?;

    let backup = rusqlite::backup::Backup::new(conn, &mut backup_conn)
        .map_err(|e| format!("Failed to initialize pre-migration backup: {}", e))?;

    backup
        .step(-1)
        .map_err(|e| format!("Pre-migration backup failed: {}", e))?;

    log::info!("Pre-migration backup created at {}", backup_path);
    Ok(())
}

/// Run all pending migrations.
///
/// Returns the number of migrations applied (0 if already up-to-date).
///
/// Forward-compat guard: if the database has a higher version than the highest
/// known migration, returns an error telling the user to update DailyOS.
pub fn run_migrations(conn: &Connection) -> Result<usize, String> {
    ensure_schema_version_table(conn)?;
    bootstrap_existing_db(conn)?;

    let current = current_version(conn)?;
    let max_known = MIGRATIONS.last().map(|m| m.version).unwrap_or(0);

    // Forward-compat guard
    if current > max_known {
        return Err(format!(
            "Database schema version ({}) is newer than this version of DailyOS supports ({}). \
             Please update DailyOS to the latest version.",
            current, max_known
        ));
    }

    // Collect pending migrations
    let pending: Vec<&Migration> = MIGRATIONS.iter().filter(|m| m.version > current).collect();

    if pending.is_empty() {
        return Ok(0);
    }

    // Backup before applying any migrations
    backup_before_migration(conn)?;

    // Apply each pending migration in order
    for migration in &pending {
        if let Err(e) = conn.execute_batch(migration.sql) {
            let msg = e.to_string();
            // Tolerate "duplicate column name" errors — the ALTER TABLE may have
            // already been applied out-of-band (e.g. baseline schema included the
            // column but migration 006 wasn't tracked). All other errors are fatal.
            if msg.contains("duplicate column name") {
                log::info!(
                    "Migration v{}: duplicate column (already exists), continuing",
                    migration.version
                );
            } else {
                return Err(format!("Migration v{} failed: {}", migration.version, e));
            }
        }

        conn.execute(
            "INSERT INTO schema_version (version) VALUES (?1)",
            [migration.version],
        )
        .map_err(|e| format!("Failed to record migration v{}: {}", migration.version, e))?;

        log::info!("Applied migration v{}", migration.version);
    }

    Ok(pending.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    /// Helper: open an in-memory database with WAL-like settings.
    fn mem_db() -> Connection {
        Connection::open_in_memory().expect("in-memory db")
    }

    #[test]
    fn test_fresh_db_applies_baseline() {
        let conn = mem_db();
        let applied = run_migrations(&conn).expect("migrations should succeed");
        assert_eq!(
            applied, 14,
            "should apply all migrations including gravatar_cache"
        );

        // Verify schema_version
        let version = current_version(&conn).expect("version query");
        assert_eq!(version, 14);

        // Verify key tables exist with correct columns
        let action_count: i32 = conn
            .query_row("SELECT COUNT(*) FROM actions", [], |row| row.get(0))
            .expect("actions table should exist");
        assert_eq!(action_count, 0);

        // Verify needs_decision column exists (was an ALTER TABLE migration)
        conn.execute(
            "INSERT INTO actions (id, title, created_at, updated_at, needs_decision)
             VALUES ('test', 'test', '2025-01-01', '2025-01-01', 1)",
            [],
        )
        .expect("needs_decision column should exist");

        // Verify meetings_history has all migrated columns
        conn.execute(
            "INSERT INTO meetings_history (id, title, meeting_type, start_time, created_at,
             calendar_event_id, prep_context_json, description, user_agenda_json, user_notes,
             prep_frozen_json, prep_frozen_at, prep_snapshot_path, prep_snapshot_hash,
             transcript_path, transcript_processed_at)
             VALUES ('m1', 'Test', 'customer', '2025-01-01', '2025-01-01',
             'cal1', '{}', 'desc', '[]', 'notes', '{}', '2025-01-01',
             '/path', 'abc123', '/transcript', '2025-01-01')",
            [],
        )
        .expect("meetings_history should have all columns");

        // Verify captures has project_id and decision type
        conn.execute(
            "INSERT INTO captures (id, meeting_id, meeting_title, project_id, capture_type, content)
             VALUES ('c1', 'm1', 'Test', 'p1', 'decision', 'content')",
            [],
        )
        .expect("captures should accept project_id and decision type");

        // Verify content_index has content_type and priority
        conn.execute(
            "INSERT INTO content_index (id, entity_id, filename, relative_path, absolute_path,
             format, modified_at, indexed_at, content_type, priority)
             VALUES ('ci1', 'e1', 'f.md', 'f.md', '/f.md', 'markdown', '2025-01-01',
             '2025-01-01', 'transcript', 1)",
            [],
        )
        .expect("content_index should have content_type and priority");

        // Verify accounts has all migrated columns
        conn.execute(
            "INSERT INTO accounts (id, name, updated_at, lifecycle, nps, parent_id, is_internal, archived)
             VALUES ('a1', 'Acme', '2025-01-01', 'onboarding', 85, NULL, 0, 0)",
            [],
        )
        .expect("accounts should include is_internal");

        conn.execute(
            "INSERT INTO account_team (account_id, person_id, role) VALUES ('a1', 'p1', 'tam')",
            [],
        )
        .expect("account_team table should exist");

        conn.execute(
            "INSERT INTO account_team_import_notes (account_id, legacy_field, legacy_value, note)
             VALUES ('a1', 'csm', 'Legacy Name', 'note')",
            [],
        )
        .expect("account_team_import_notes table should exist");

        // Verify account_domains exists and accepts inserts
        conn.execute(
            "INSERT INTO account_domains (account_id, domain) VALUES ('a1', 'acme.com')",
            [],
        )
        .expect("account_domains table should exist");

        // Verify account_events table
        conn.execute(
            "INSERT INTO account_events (account_id, event_type, event_date)
             VALUES ('a1', 'renewal', '2025-06-01')",
            [],
        )
        .expect("account_events table should exist");

        // Verify email_signals exists and accepts inserts
        conn.execute(
            "INSERT INTO email_signals (
                email_id, sender_email, entity_id, entity_type, signal_type, signal_text
             ) VALUES ('em-1', 'owner@acme.com', 'a1', 'account', 'timeline', 'Customer asked for revised launch date')",
            [],
        )
        .expect("email_signals table should exist");

        // Verify content_embeddings exists and accepts inserts
        conn.execute(
            "INSERT INTO content_embeddings (
                id, content_file_id, chunk_index, chunk_text, embedding, created_at
             ) VALUES ('emb-1', 'ci1', 0, 'test chunk', X'', '2025-01-01')",
            [],
        )
        .expect("content_embeddings table should exist");

        // Verify chat_sessions exists and accepts inserts
        conn.execute(
            "INSERT INTO chat_sessions (
                id, entity_id, entity_type, session_start, turn_count, created_at
             ) VALUES ('cs-1', 'a1', 'account', '2025-01-01', 0, '2025-01-01')",
            [],
        )
        .expect("chat_sessions table should exist");

        // Verify chat_turns exists and accepts inserts
        conn.execute(
            "INSERT INTO chat_turns (
                id, session_id, turn_index, role, content, timestamp
             ) VALUES ('ct-1', 'cs-1', 0, 'user', 'Hello', '2025-01-01')",
            [],
        )
        .expect("chat_turns table should exist");

        // Verify proposed/archived action statuses work (migration 011)
        conn.execute(
            "INSERT INTO actions (id, title, status, created_at, updated_at)
             VALUES ('proposed-1', 'Proposed action', 'proposed', '2025-01-01', '2025-01-01')",
            [],
        )
        .expect("proposed status should be accepted");

        conn.execute(
            "INSERT INTO actions (id, title, status, created_at, updated_at)
             VALUES ('archived-1', 'Archived action', 'archived', '2025-01-01', '2025-01-01')",
            [],
        )
        .expect("archived status should be accepted");

        // Verify person_emails table exists and accepts inserts (migration 012)
        conn.execute(
            "INSERT INTO person_emails (person_id, email, is_primary, added_at)
             VALUES ('p1', 'alice@acme.com', 1, '2025-01-01')",
            [],
        )
        .expect("person_emails table should exist");

        // Verify quill_sync_state table accepts inserts (migration 013)
        conn.execute(
            "INSERT INTO quill_sync_state (id, meeting_id, state)
             VALUES ('qs-1', 'm1', 'pending')",
            [],
        )
        .expect("quill_sync_state table should exist and accept inserts");

        // Verify gravatar_cache table accepts inserts (migration 014)
        conn.execute(
            "INSERT INTO gravatar_cache (email, has_gravatar, fetched_at, person_id)
             VALUES ('alice@acme.com', 1, '2025-01-01T00:00:00Z', 'p1')",
            [],
        )
        .expect("gravatar_cache table should exist and accept inserts");
    }

    #[test]
    fn test_bootstrap_existing_db() {
        let conn = mem_db();

        // Simulate a pre-framework database: create actions table with all baseline columns.
        // A real pre-framework DB would have all columns from inline CREATE TABLE + ALTER TABLE
        // statements that existed in db.rs before the migration framework.
        conn.execute_batch(
            "CREATE TABLE actions (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                priority TEXT CHECK(priority IN ('P1', 'P2', 'P3')) DEFAULT 'P2',
                status TEXT CHECK(status IN ('pending', 'completed', 'waiting', 'cancelled')) DEFAULT 'pending',
                created_at TEXT NOT NULL,
                due_date TEXT,
                completed_at TEXT,
                account_id TEXT,
                project_id TEXT,
                source_type TEXT,
                source_id TEXT,
                source_label TEXT,
                context TEXT,
                waiting_on TEXT,
                updated_at TEXT NOT NULL,
                person_id TEXT,
                needs_decision INTEGER DEFAULT 0
            );
            INSERT INTO actions (id, title, created_at, updated_at)
            VALUES ('existing', 'Existing Action', '2025-01-01', '2025-01-01');",
        )
        .expect("seed existing db");

        // Create other tables that a pre-framework DB would have
        conn.execute_batch(
            "CREATE TABLE accounts (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                lifecycle TEXT,
                arr REAL,
                health TEXT,
                csm TEXT,
                champion TEXT,
                contract_start TEXT,
                contract_end TEXT,
                nps INTEGER,
                tracker_path TEXT,
                parent_id TEXT,
                updated_at TEXT NOT NULL,
                archived INTEGER DEFAULT 0
            );
             CREATE TABLE meetings_history (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                meeting_type TEXT NOT NULL,
                start_time TEXT NOT NULL,
                created_at TEXT NOT NULL,
                calendar_event_id TEXT
             );
             CREATE TABLE people (
                id TEXT PRIMARY KEY,
                email TEXT NOT NULL,
                name TEXT NOT NULL,
                relationship TEXT NOT NULL DEFAULT 'unknown'
             );
             CREATE TABLE entity_people (
                entity_id TEXT NOT NULL,
                person_id TEXT NOT NULL,
                relationship_type TEXT DEFAULT 'associated',
                PRIMARY KEY (entity_id, person_id)
             );
             CREATE TABLE meeting_entities (
                meeting_id TEXT NOT NULL,
                entity_id TEXT NOT NULL,
                entity_type TEXT NOT NULL,
                PRIMARY KEY (meeting_id, entity_id)
             );
             CREATE TABLE content_index (
                id TEXT PRIMARY KEY,
                entity_id TEXT NOT NULL,
                entity_type TEXT NOT NULL DEFAULT 'account',
                filename TEXT NOT NULL,
                relative_path TEXT NOT NULL,
                absolute_path TEXT NOT NULL,
                format TEXT NOT NULL,
                file_size INTEGER NOT NULL DEFAULT 0,
                modified_at TEXT NOT NULL,
                indexed_at TEXT NOT NULL,
                extracted_at TEXT,
                summary TEXT,
                content_type TEXT NOT NULL DEFAULT 'general',
                priority INTEGER NOT NULL DEFAULT 5
             );",
        )
        .expect("seed existing tables");

        // Run migrations — should bootstrap v1 and apply v2 through v9
        let applied = run_migrations(&conn).expect("migrations should succeed");
        assert_eq!(applied, 13, "bootstrap should mark v1, then apply v2 through v14");

        // Verify schema version
        let version = current_version(&conn).expect("version query");
        assert_eq!(version, 14);

        // Verify existing data is untouched
        let title: String = conn
            .query_row(
                "SELECT title FROM actions WHERE id = 'existing'",
                [],
                |row| row.get(0),
            )
            .expect("existing data should be preserved");
        assert_eq!(title, "Existing Action");
    }

    #[test]
    fn test_forward_compat_guard() {
        let conn = mem_db();

        // Set up schema_version with a future version
        ensure_schema_version_table(&conn).unwrap();
        conn.execute("INSERT INTO schema_version (version) VALUES (999)", [])
            .unwrap();

        // run_migrations should fail with a clear error
        let result = run_migrations(&conn);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("newer than this version"),
            "error should mention version mismatch: {}",
            err
        );
    }

    #[test]
    fn test_idempotency() {
        let conn = mem_db();
        let total = MIGRATIONS.len();

        // Run migrations twice
        let first = run_migrations(&conn).expect("first run");
        assert_eq!(first, total);

        let second = run_migrations(&conn).expect("second run");
        assert_eq!(second, 0, "second run should apply no migrations");

        // Version should match the highest migration
        let version = current_version(&conn).expect("version query");
        assert_eq!(version, MIGRATIONS.last().unwrap().version);
    }

    #[test]
    fn test_pre_migration_backup_created() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("test_backup.db");

        let conn = Connection::open(&db_path).expect("open db");
        conn.execute_batch("PRAGMA journal_mode=WAL;").unwrap();

        let applied = run_migrations(&conn).expect("migrations should succeed");
        assert_eq!(applied, MIGRATIONS.len());

        // Verify backup file was created
        let backup_path = dir.path().join("test_backup.db.pre-migration.bak");
        assert!(
            backup_path.exists(),
            "pre-migration backup should be created at {}",
            backup_path.display()
        );
    }
}
