//! SQLite-based local state management for actions, accounts, and meeting history.
//!
//! The database lives at `~/.dailyos/dailyos.db` and serves as the working store
//! for operational data (ADR-0048). The filesystem (markdown + JSON) is the durable
//! layer; SQLite enables fast queries, state tracking, and cross-entity intelligence.
//! SQLite is not disposable — important state lives here and is written back to the
//! filesystem at natural synchronization points (archive, dashboard regeneration).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use super::types::*;
use crate::db::encryption;
use crate::db::key_provider::{DbKeyProvider, EncryptionKey, UserIdentity};
use rusqlite::{params, Connection, OpenFlags};

// ---------------------------------------------------------------------------
// Dev DB isolation
// ---------------------------------------------------------------------------

/// Process-wide flag steering `ActionDb::db_path()` between live and dev files.
/// Background threads (executor, intel_queue, watcher, hygiene) all call
/// `ActionDb::open()` independently — the static flag means they automatically
/// pick up the right path without plumbing config through every thread.
static DEV_DB_MODE: AtomicBool = AtomicBool::new(false);

/// Activate dev-mode DB isolation. All subsequent `ActionDb::open()` calls
/// will target `~/.dailyos/dailyos-dev.db` instead of `dailyos.db`.
pub fn set_dev_db_mode(enabled: bool) {
    DEV_DB_MODE.store(enabled, Ordering::Relaxed);
}

/// Check whether dev-mode DB isolation is active.
pub fn is_dev_db_mode() -> bool {
    DEV_DB_MODE.load(Ordering::Relaxed)
}

#[repr(transparent)]
pub struct ActionDb {
    pub(crate) conn: Connection,
}

#[cfg(test)]
struct FixtureDbKeyProvider {
    key: EncryptionKey,
}

#[cfg(test)]
impl FixtureDbKeyProvider {
    fn new() -> Self {
        Self {
            key: EncryptionKey::from_hex(
                "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
            ),
        }
    }
}

#[cfg(test)]
impl DbKeyProvider for FixtureDbKeyProvider {
    fn get_or_create_key(
        &self,
        _user: &UserIdentity,
    ) -> crate::db::key_provider::Result<EncryptionKey> {
        Ok(self.key.clone())
    }

    fn rotate_key(&self, _user: &UserIdentity) -> crate::db::key_provider::Result<EncryptionKey> {
        Ok(self.key.clone())
    }
}

impl ActionDb {
    /// Borrow the underlying connection for ad-hoc queries.
    pub fn conn_ref(&self) -> &Connection {
        &self.conn
    }

    /// Borrow a `Connection` owned elsewhere as an `ActionDb` view.
    ///
    /// `ActionDb` is `repr(transparent)` over `rusqlite::Connection`, so this
    /// view has the same layout and a lifetime tied to the input borrow. The
    /// borrowed view cannot outlive `conn` or be moved into a `'static`
    /// closure, which keeps pooled `.call()` usage type-system bounded.
    pub fn from_conn(conn: &Connection) -> &Self {
        // SAFETY: `ActionDb` is `repr(transparent)` and its only field is
        // `Connection`, so `&Connection` and `&ActionDb` have identical layout.
        unsafe { &*(conn as *const Connection as *const Self) }
    }

    /// Execute a closure within a SQLite transaction.
    /// Commits on Ok, rolls back on Err.
    #[must_use = "the closure may write to the DB; dropping this Result silently swallows transaction failure or rollback"]
    pub fn with_transaction<F, T>(&self, f: F) -> Result<T, String>
    where
        F: FnOnce(&Self) -> Result<T, String>,
    {
        // Nested transaction support: if we're already inside a transaction on this
        // connection, execute the closure directly so all writes stay in the
        // caller's transaction boundary.
        if !self.conn.is_autocommit() {
            return f(self);
        }

        self.conn
            .execute_batch("BEGIN IMMEDIATE")
            .map_err(|e| format!("Failed to begin transaction: {e}"))?;
        match f(self) {
            Ok(val) => {
                self.conn
                    .execute_batch("COMMIT")
                    .map_err(|e| format!("Failed to commit transaction: {e}"))?;
                Ok(val)
            }
            Err(e) => {
                #[allow(
                    clippy::let_underscore_must_use,
                    reason = "intentional best-effort discard; preserves existing non-blocking behavior"
                )]
                // best-effort: preserve the original transaction error if rollback itself fails.
                let _ = self.conn.execute_batch("ROLLBACK");
                Err(e)
            }
        }
    }

    fn recover_stuck_version_mutations_logged(db: &ActionDb) {
        match crate::services::versioning::recover_stuck_mutation_attempts(db, chrono::Utc::now()) {
            Ok(0) => {}
            Ok(count) => log::warn!(
                "recovered {count} stale in-flight version mutation attempt(s) at startup"
            ),
            Err(error) => log::warn!("version mutation startup recovery scan failed: {error}"),
        }
    }

    fn map_key_error(error: String) -> DbError {
        if error.starts_with("KEY_MISSING:") {
            DbError::KeyMissing {
                db_path: error.trim_start_matches("KEY_MISSING:").to_string(),
            }
        } else {
            DbError::Encryption(error)
        }
    }

    fn prepare_encrypted_connection(
        path: &Path,
        key_provider: Arc<dyn DbKeyProvider>,
    ) -> Result<(Connection, EncryptionKey), DbError> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(DbError::CreateDir)?;
            }
        }

        // Get or create encryption key from Keychain
        let user = UserIdentity::local(path.to_path_buf());
        let encryption_key = key_provider
            .get_or_create_key(&user)
            .map_err(Self::map_key_error)?;

        // Migrate plaintext DB if it exists (ADR-0092)
        if path.exists() && encryption::is_database_plaintext(path) {
            log::info!("Detected plaintext database, migrating to encrypted...");
            encryption::migrate_to_encrypted(path, encryption_key.as_hex())
                .map_err(DbError::Encryption)?;
        }

        let conn = Connection::open(path)?;

        // PRAGMA key MUST be first — before any other PRAGMA (ADR-0092)
        conn.execute_batch(&encryption_key.to_pragma())?;

        // Validate that the key can read the database by touching schema metadata.
        // This avoids engine-specific SQLCipher functions (e.g. sqlcipher_version)
        // that may not exist in all bundled builds.
        conn.query_row("SELECT count(*) FROM sqlite_master LIMIT 1", [], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(|e| {
            DbError::Encryption(format!(
                "SQLCipher key verification failed (database unreadable): {e}"
            ))
        })?;

        // Enable WAL mode for better concurrent read performance
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;

        // Retry for up to 5s on SQLITE_BUSY instead of failing immediately.
        // Without this, background tasks opening their own connections cause
        // immediate failures when the main connection holds a write lock.
        conn.execute_batch("PRAGMA busy_timeout = 5000;")?;

        // NORMAL sync is safe with WAL — only fsyncs on checkpoint, not every commit.
        // ~3x write throughput improvement over the default FULL.
        conn.execute_batch("PRAGMA synchronous = NORMAL;")?;

        // Run schema migrations (ADR-0071)
        crate::migrations::run_migrations_with_key(&conn, Some(&encryption_key))
            .map_err(DbError::Migration)?;

        Self::recover_stuck_version_mutations_logged(Self::from_conn(&conn));

        // Enable FK constraint enforcement. Set after migrations since
        // migration 010 uses PRAGMA foreign_keys = OFF for table recreation.
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;

        // Legacy data repairs — idempotent Rust code, safe to run every startup.
        // Will be removed once all alpha users are past v0.7.3.
        #[allow(
            clippy::let_underscore_must_use,
            reason = "intentional best-effort discard; preserves existing non-blocking behavior"
        )]
        let _ = Self::normalize_reviewed_prep_keys(&conn);
        #[allow(
            clippy::let_underscore_must_use,
            reason = "intentional best-effort discard; preserves existing non-blocking behavior"
        )]
        let _ = Self::backfill_meeting_identity(&conn);
        #[allow(
            clippy::let_underscore_must_use,
            reason = "intentional best-effort discard; preserves existing non-blocking behavior"
        )]
        let _ = Self::backfill_meeting_user_layer(&conn);
        #[allow(
            clippy::let_underscore_must_use,
            reason = "intentional best-effort discard; preserves existing non-blocking behavior"
        )]
        let _ = Self::backfill_stakeholder_columns(&conn);
        #[allow(
            clippy::let_underscore_must_use,
            reason = "intentional best-effort discard; preserves existing non-blocking behavior"
        )]
        let _ = Self::dismiss_internal_stakeholder_suggestions(&conn);

        Ok((conn, encryption_key))
    }

    pub(crate) fn open_encrypted_connection(
        path: PathBuf,
        key_provider: Arc<dyn DbKeyProvider>,
    ) -> Result<(Connection, EncryptionKey), DbError> {
        let (conn, encryption_key) = Self::prepare_encrypted_connection(&path, key_provider)?;
        let db = Self { conn };

        // One-time initialization tasks (guarded by init_tasks table).
        // These run exactly once per database and are safe to call on every startup.
        #[allow(
            clippy::let_underscore_must_use,
            reason = "intentional best-effort discard; preserves existing non-blocking behavior"
        )]
        let _ = db.run_guarded_init_backfill_account_domains();

        Ok((db.conn, encryption_key))
    }

    /// Open (or create) the database at `~/.dailyos/dailyos.db` and apply the schema.
    ///
    /// Every call creates a fresh `rusqlite::Connection` via direct open. When
    /// a global `DbService` is installed, the fresh-open path is executed on
    /// the writer's dedicated thread to avoid SQLCipher WAL key-verification races
    /// (SQLITE_NOTADB) while preserving a non-shared ownership contract.
    pub fn open(key_provider: Arc<dyn DbKeyProvider>) -> Result<Self, DbError> {
        let path = Self::db_path()?;
        Self::open_resolved_path(path, key_provider)
    }

    /// Open the database for diagnostic inspection WITHOUT running startup
    /// recovery for stuck mutation attempts.
    ///
    /// `ActionDb::open` calls `recover_stuck_mutation_attempts` during
    /// connection bring-up, which aborts any `mutation_attempts` row whose
    /// `in_flight` lease exceeds 30 seconds. `dailyos doctor watermarks`
    /// needs to inventory those zombie rows BEFORE recovery wipes them;
    /// otherwise the doctor's own action mutates the state it's reporting.
    /// Per packet ac §36 + L2 cycle-2 P2 (codex): the doctor must read,
    /// not heal.
    pub fn open_for_inspection(key_provider: Arc<dyn DbKeyProvider>) -> Result<Self, DbError> {
        let path = Self::db_path()?;
        let (conn, _key) = Self::prepare_encrypted_connection_no_recovery(&path, key_provider)?;
        Ok(Self { conn })
    }

    /// Variant of `prepare_encrypted_connection` that runs migrations but
    /// skips startup recovery for in-flight mutation attempts. Used by
    /// `open_for_inspection`. The two should diverge in EXACTLY that line
    /// of behaviour; centralised so the encryption + migration setup
    /// cannot drift between paths.
    fn prepare_encrypted_connection_no_recovery(
        path: &Path,
        key_provider: Arc<dyn DbKeyProvider>,
    ) -> Result<(Connection, EncryptionKey), DbError> {
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(DbError::CreateDir)?;
            }
        }

        let user = UserIdentity::local(path.to_path_buf());
        let encryption_key = key_provider
            .get_or_create_key(&user)
            .map_err(Self::map_key_error)?;

        if path.exists() && encryption::is_database_plaintext(path) {
            log::info!("Detected plaintext database, migrating to encrypted...");
            encryption::migrate_to_encrypted(path, encryption_key.as_hex())
                .map_err(DbError::Encryption)?;
        }

        let conn = Connection::open(path)?;
        conn.execute_batch(&encryption_key.to_pragma())?;
        conn.query_row("SELECT count(*) FROM sqlite_master LIMIT 1", [], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(|e| {
            DbError::Encryption(format!(
                "SQLCipher key verification failed (database unreadable): {e}"
            ))
        })?;
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        conn.execute_batch("PRAGMA busy_timeout = 5000;")?;
        conn.execute_batch("PRAGMA synchronous = NORMAL;")?;

        crate::migrations::run_migrations_with_key(&conn, Some(&encryption_key))
            .map_err(DbError::Migration)?;

        // Intentionally skip recover_stuck_version_mutations_logged so the
        // doctor inspection can count zombie attempts. No legacy backfill
        // either — those are healing operations.
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;

        Ok((conn, encryption_key))
    }

    fn open_resolved_path(
        path: PathBuf,
        key_provider: Arc<dyn DbKeyProvider>,
    ) -> Result<Self, DbError> {
        let rotation_lock = crate::db::key_provider::rotation_lock_read();
        if let Some(svc) = crate::db_service::try_global() {
            let user = UserIdentity::local(path.clone());
            let encryption_key = key_provider
                .get_or_create_key(&user)
                .map_err(Self::map_key_error)?;
            let conn = svc.open_fresh_serialized(path.clone(), encryption_key)?;
            drop(rotation_lock);
            let db = Self { conn };
            Self::recover_stuck_version_mutations_logged(&db);
            #[allow(
                clippy::let_underscore_must_use,
                reason = "intentional best-effort discard; preserves existing non-blocking behavior"
            )]
            let _ = db.run_guarded_init_backfill_account_domains();
            return Ok(db);
        }

        Self::open_at(path, key_provider)
    }

    #[cfg(test)]
    pub(crate) fn open_resolved_path_for_tests(
        path: PathBuf,
        key_provider: Arc<dyn DbKeyProvider>,
    ) -> Result<Self, DbError> {
        Self::open_resolved_path(path, key_provider)
    }

    #[cfg(test)]
    pub(crate) fn open_resolved_path_with_fixture_provider_for_tests(
        path: PathBuf,
    ) -> Result<Self, DbError> {
        Self::open_resolved_path(path, Arc::new(FixtureDbKeyProvider::new()))
    }

    /// Open a database at an explicit path. Useful for testing.
    pub(crate) fn open_at(
        path: PathBuf,
        key_provider: Arc<dyn DbKeyProvider>,
    ) -> Result<Self, DbError> {
        let (conn, _) = Self::open_encrypted_connection(path, key_provider)?;
        Ok(Self { conn })
    }

    /// Open without encryption. Used for tests only.
    #[cfg(test)]
    pub(crate) fn open_at_unencrypted(path: PathBuf) -> Result<Self, DbError> {
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(DbError::CreateDir)?;
            }
        }
        let conn = Connection::open(&path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        conn.execute_batch("PRAGMA busy_timeout = 5000;")?;
        conn.execute_batch("PRAGMA synchronous = NORMAL;")?;
        crate::migrations::run_migrations(&conn).map_err(DbError::Migration)?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        let _ = Self::normalize_reviewed_prep_keys(&conn);
        let _ = Self::backfill_meeting_identity(&conn);
        let _ = Self::backfill_meeting_user_layer(&conn);
        let _ = Self::backfill_stakeholder_columns(&conn);

        let db = Self { conn };
        Self::recover_stuck_version_mutations_logged(&db);
        let _ = db.run_guarded_init_backfill_account_domains();
        Ok(db)
    }

    /// Open the database in read-only mode. Used by the MCP binary for safe
    /// concurrent reads while the Tauri app owns writes.
    pub fn open_readonly(key_provider: Arc<dyn DbKeyProvider>) -> Result<Self, DbError> {
        let path = Self::db_path()?;
        Self::open_readonly_at(&path, key_provider)
    }

    /// Open a database at an explicit path in read-only mode.
    pub fn open_readonly_at(
        path: &std::path::Path,
        key_provider: Arc<dyn DbKeyProvider>,
    ) -> Result<Self, DbError> {
        let user = UserIdentity::local(path.to_path_buf());
        let encryption_key = key_provider.get_or_create_key(&user).map_err(|e| {
            if e.starts_with("KEY_MISSING:") {
                DbError::KeyMissing {
                    db_path: e.trim_start_matches("KEY_MISSING:").to_string(),
                }
            } else {
                DbError::Encryption(e)
            }
        })?;

        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;

        // PRAGMA key MUST be first
        conn.execute_batch(&encryption_key.to_pragma())?;

        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        conn.execute_batch("PRAGMA busy_timeout = 5000;")?;
        conn.execute_batch("PRAGMA query_only = ON;")?;
        Ok(Self { conn })
    }

    #[cfg(any(test, feature = "test-harness", feature = "bench-harness"))]
    #[doc(hidden)]
    pub fn from_connection_for_tests(conn: Connection) -> Self {
        Self { conn }
    }

    #[cfg(any(test, feature = "test-harness"))]
    #[doc(hidden)]
    pub fn open_unencrypted_readonly_at_for_tests(path: &std::path::Path) -> Result<Self, DbError> {
        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        conn.execute_batch("PRAGMA busy_timeout = 5000;")?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        conn.execute_batch("PRAGMA query_only = ON;")?;
        Ok(Self { conn })
    }

    /// Resolve the default database path: `~/.dailyos/dailyos.db`.
    ///
    /// When dev-mode DB isolation is active (`set_dev_db_mode(true)`), returns
    /// `~/.dailyos/dailyos-dev.db` instead. Migration logic only applies to the
    /// live path — the dev DB is always created fresh.
    /// Public accessor for the resolved DB path. Used by `DbService` to open
    /// connections at the same path as `ActionDb::open()`.
    pub fn db_path_public() -> Result<PathBuf, DbError> {
        Self::db_path()
    }

    fn db_path() -> Result<PathBuf, DbError> {
        let home = dirs::home_dir().ok_or(DbError::HomeDirNotFound)?;
        let dailyos_dir = home.join(".dailyos");

        // Dev-mode: isolated DB, no migration needed
        if is_dev_db_mode() {
            return Ok(dailyos_dir.join("dailyos-dev.db"));
        }

        let new_path = dailyos_dir.join("dailyos.db");
        let legacy_path = dailyos_dir.join("actions.db");

        // One-time migration: rename actions.db → dailyos.db
        if !new_path.exists() && legacy_path.exists() {
            // Checkpoint WAL into the main file before renaming, otherwise
            // data written to the WAL but not yet flushed would be lost.
            if let Ok(conn) = Connection::open(&legacy_path) {
                #[allow(
                    clippy::let_underscore_must_use,
                    reason = "intentional best-effort discard; preserves existing non-blocking behavior"
                )]
                // best-effort: rename migration can still proceed if no WAL frames need flushing.
                let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
                drop(conn);
            }

            if let Err(e) = std::fs::rename(&legacy_path, &new_path) {
                log::warn!(
                    "Failed to rename actions.db → dailyos.db: {}. Will open at legacy path.",
                    e
                );
                return Ok(legacy_path);
            }
            // Clean up WAL/SHM files (SQLite recreates them under the new name)
            #[allow(
                clippy::let_underscore_must_use,
                reason = "intentional best-effort discard; preserves existing non-blocking behavior"
            )]
            let _ = std::fs::remove_file(dailyos_dir.join("actions.db-wal"));
            #[allow(
                clippy::let_underscore_must_use,
                reason = "intentional best-effort discard; preserves existing non-blocking behavior"
            )]
            let _ = std::fs::remove_file(dailyos_dir.join("actions.db-shm"));
            log::info!("Migrated database: actions.db → dailyos.db");
        }

        Ok(new_path)
    }

    /// Convert reviewed-prep keys from legacy prep file paths to meeting IDs.
    fn normalize_reviewed_prep_keys(conn: &Connection) -> Result<(), DbError> {
        let rows: Vec<(String, Option<String>, String, Option<String>)> = {
            let mut stmt = conn.prepare(
                "SELECT prep_file, calendar_event_id, reviewed_at, title
                 FROM meeting_prep_state",
            )?;
            let mapped = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })?;
            let mut items = Vec::new();
            for row in mapped {
                items.push(row?);
            }
            items
        };

        for (legacy_key, calendar_event_id, reviewed_at, title) in rows {
            let canonical = if let Some(ref cal_id) = calendar_event_id {
                if !cal_id.trim().is_empty() {
                    Self::sanitize_calendar_event_id(cal_id)
                } else {
                    Self::extract_meeting_id_from_review_key(&legacy_key)
                }
            } else {
                Self::extract_meeting_id_from_review_key(&legacy_key)
            };
            if canonical.is_empty() || canonical == legacy_key {
                continue;
            }
            conn.execute(
                "INSERT INTO meeting_prep_state (prep_file, calendar_event_id, reviewed_at, title)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(prep_file) DO UPDATE SET
                    reviewed_at = CASE
                        WHEN excluded.reviewed_at > meeting_prep_state.reviewed_at
                        THEN excluded.reviewed_at
                        ELSE meeting_prep_state.reviewed_at
                    END,
                    calendar_event_id = COALESCE(excluded.calendar_event_id, meeting_prep_state.calendar_event_id),
                    title = COALESCE(excluded.title, meeting_prep_state.title)",
                params![canonical, calendar_event_id, reviewed_at, title],
            )?;
            conn.execute(
                "DELETE FROM meeting_prep_state WHERE prep_file = ?1",
                params![legacy_key],
            )?;
        }
        Ok(())
    }

    fn extract_meeting_id_from_review_key(key: &str) -> String {
        let trimmed = key.trim();
        let without_prefix = trimmed.strip_prefix("preps/").unwrap_or(trimmed);
        without_prefix
            .trim_end_matches(".json")
            .trim_end_matches(".md")
            .to_string()
    }

    pub(super) fn sanitize_calendar_event_id(calendar_event_id: &str) -> String {
        calendar_event_id.replace('@', "_at_")
    }

    /// One-time backfill of dashboard.json narrative fields into DB columns.
    ///
    /// Iterates accounts with `tracker_path IS NOT NULL AND company_overview IS NULL`,
    /// reads their dashboard.json, and writes the fields to DB.
    /// Same for projects.
    #[must_use = "check how many dashboard fields were backfilled before trusting DB narrative columns"]
    pub fn backfill_dashboard_json_to_db(&self, workspace: &Path) -> Result<usize, DbError> {
        const TASK_NAME: &str = "backfill_dashboard_json_to_db_v1";

        if Self::is_init_task_completed(&self.conn, TASK_NAME)? {
            return Ok(0);
        }

        let mut count = 0usize;

        // Backfill accounts
        let accounts: Vec<(String, String)> = {
            let mut stmt = self.conn.prepare(
                "SELECT id, name FROM accounts \
                 WHERE tracker_path IS NOT NULL AND company_overview IS NULL",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
        };

        for (account_id, _account_name) in &accounts {
            if let Ok(Some(account)) = self.get_account(account_id) {
                let account_dir = crate::accounts::resolve_account_dir(workspace, &account);
                let json_path = account_dir.join("dashboard.json");
                if json_path.exists() {
                    match crate::accounts::read_account_json(&json_path) {
                        Ok(result) => {
                            let ov_json = result
                                .json
                                .company_overview
                                .and_then(|ov| serde_json::to_string(&ov).ok());
                            let prg_json = if result.json.strategic_programs.is_empty() {
                                None
                            } else {
                                serde_json::to_string(&result.json.strategic_programs).ok()
                            };
                            let notes = result.json.notes;
                            let now = chrono::Utc::now().to_rfc3339();
                            if let Err(e) =
                                crate::services::derived_state::update_account_ai_columns_projection(
                                    self,
                                    account_id,
                                    ov_json.as_deref(),
                                    prg_json.as_deref(),
                                    notes.as_deref(),
                                    &now,
                                )
                            {
                                log::warn!(
                                    "I644 backfill: failed to update account {}: {}",
                                    account_id,
                                    e
                                );
                            } else {
                                count += 1;
                            }
                        }
                        Err(e) => {
                            log::warn!(
                                "I644 backfill: failed to read dashboard.json for account {}: {}",
                                account_id,
                                e
                            );
                        }
                    }
                }
            }
        }

        // Backfill projects
        let projects: Vec<(String, String)> = {
            let mut stmt = self.conn.prepare(
                "SELECT id, name FROM projects \
                 WHERE tracker_path IS NOT NULL AND description IS NULL",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
        };

        for (project_id, project_name) in &projects {
            let project_dir = crate::projects::project_dir(workspace, project_name);
            let json_path = project_dir.join("dashboard.json");
            if json_path.exists() {
                match crate::projects::read_project_json(&json_path) {
                    Ok(result) => {
                        let ms_json = if result.json.milestones.is_empty() {
                            None
                        } else {
                            serde_json::to_string(&result.json.milestones).ok()
                        };
                        let now = chrono::Utc::now().to_rfc3339();
                        if let Err(e) = self.conn.execute(
                            "UPDATE projects SET description = ?1, milestones = ?2, \
                             notes = ?3, updated_at = ?4 WHERE id = ?5",
                            rusqlite::params![
                                result.json.description,
                                ms_json,
                                result.json.notes,
                                now,
                                project_id
                            ],
                        ) {
                            log::warn!(
                                "I644 backfill: failed to update project {}: {}",
                                project_id,
                                e
                            );
                        } else {
                            count += 1;
                        }
                    }
                    Err(e) => {
                        log::warn!(
                            "I644 backfill: failed to read dashboard.json for project {}: {}",
                            project_id,
                            e
                        );
                    }
                }
            }
        }

        Self::mark_init_task_completed(&self.conn, TASK_NAME)?;

        Ok(count)
    }
}

// =============================================================================
// Shared test utilities
// =============================================================================

#[cfg(test)]
pub mod test_utils {
    use super::ActionDb;

    /// Create a temporary database for testing.
    ///
    /// We leak the `TempDir` so the directory persists for the duration of the test.
    /// Test temp dirs are cleaned up by the OS. FK enforcement is disabled so that
    /// unit tests can insert rows without satisfying every foreign key constraint.
    pub fn test_db() -> ActionDb {
        let dir = tempfile::tempdir().expect("Failed to create temp dir");
        let path = dir.path().join("test.db");
        std::mem::forget(dir);
        let db = ActionDb::open_at_unencrypted(path).expect("Failed to open test database");
        db.conn_ref()
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .expect("disable FK for tests");
        db
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
