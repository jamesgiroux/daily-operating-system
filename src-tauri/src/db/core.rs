//! SQLite-based local state management for actions, accounts, and meeting history.
//!
//! The database lives at `~/.dailyos/dailyos.db` and serves as the working store
//! for operational data (ADR-0048). The filesystem (markdown + JSON) is the durable
//! layer; SQLite enables fast queries, state tracking, and cross-entity intelligence.
//! SQLite is not disposable — important state lives here and is written back to the
//! filesystem at natural synchronization points (archive, dashboard regeneration).

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use super::types::*;
use crate::db::encryption;
use rusqlite::{params, Connection, OpenFlags};

// ---------------------------------------------------------------------------
// Dev DB isolation (I298)
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

impl ActionDb {
    /// Borrow the underlying connection for ad-hoc queries.
    pub fn conn_ref(&self) -> &Connection {
        &self.conn
    }

    /// Create an `&ActionDb` from a borrowed `Connection`. Used inside
    /// `tokio_rusqlite::Connection::call()` closures to access all existing
    /// `ActionDb` methods without converting them to free functions.
    ///
    /// SAFETY: `ActionDb` is `#[repr(transparent)]` over `rusqlite::Connection`,
    /// so `&Connection` and `&ActionDb` have identical memory layouts.
    pub fn from_conn(conn: &Connection) -> &Self {
        // SAFETY: repr(transparent) guarantees layout equivalence.
        unsafe { &*(conn as *const Connection as *const Self) }
    }

    /// Execute a closure within a SQLite transaction.
    /// Commits on Ok, rolls back on Err.
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
                let _ = self.conn.execute_batch("ROLLBACK");
                Err(e)
            }
        }
    }

    /// Open (or create) the database at `~/.dailyos/dailyos.db` and apply the schema.
    pub fn open() -> Result<Self, DbError> {
        let path = Self::db_path()?;
        Self::open_at(path)
    }

    /// Open a database at an explicit path. Useful for testing.
    pub(crate) fn open_at(path: PathBuf) -> Result<Self, DbError> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(DbError::CreateDir)?;
            }
        }

        // Get or create encryption key from Keychain
        let hex_key = encryption::get_or_create_db_key(&path).map_err(|e| {
            if e.starts_with("KEY_MISSING:") {
                DbError::KeyMissing {
                    db_path: e.trim_start_matches("KEY_MISSING:").to_string(),
                }
            } else {
                DbError::Encryption(e)
            }
        })?;

        // Migrate plaintext DB if it exists (ADR-0092)
        if path.exists() && encryption::is_database_plaintext(&path) {
            log::info!("Detected plaintext database, migrating to encrypted...");
            encryption::migrate_to_encrypted(&path, &hex_key).map_err(DbError::Encryption)?;
        }

        let conn = Connection::open(&path)?;

        // PRAGMA key MUST be first — before any other PRAGMA (ADR-0092)
        conn.execute_batch(&encryption::key_to_pragma(&hex_key))?;

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
        crate::migrations::run_migrations(&conn).map_err(DbError::Migration)?;

        // Enable FK constraint enforcement (I285). Set after migrations since
        // migration 010 uses PRAGMA foreign_keys = OFF for table recreation.
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;

        // Legacy data repairs — idempotent Rust code, safe to run every startup.
        // Will be removed once all alpha users are past v0.7.3.
        let _ = Self::normalize_reviewed_prep_keys(&conn);
        let _ = Self::backfill_meeting_identity(&conn);
        let _ = Self::backfill_meeting_user_layer(&conn);
        let _ = Self::backfill_stakeholder_columns(&conn);

        let db = Self { conn };

        // One-time initialization tasks (guarded by init_tasks table).
        // These run exactly once per database and are safe to call on every startup.
        let _ = db.run_guarded_init_backfill_account_domains();

        Ok(db)
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
        let _ = db.run_guarded_init_backfill_account_domains();
        Ok(db)
    }

    /// Open the database in read-only mode. Used by the MCP binary for safe
    /// concurrent reads while the Tauri app owns writes.
    pub fn open_readonly() -> Result<Self, DbError> {
        let path = Self::db_path()?;
        Self::open_readonly_at(&path)
    }

    /// Open a database at an explicit path in read-only mode.
    pub fn open_readonly_at(path: &std::path::Path) -> Result<Self, DbError> {
        let hex_key = encryption::get_or_create_db_key(path).map_err(|e| {
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
        conn.execute_batch(&encryption::key_to_pragma(&hex_key))?;

        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        conn.execute_batch("PRAGMA busy_timeout = 5000;")?;
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
            let _ = std::fs::remove_file(dailyos_dir.join("actions.db-wal"));
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

    fn sanitize_calendar_event_id(calendar_event_id: &str) -> String {
        calendar_event_id.replace('@', "_at_")
    }

    /// Re-key meeting IDs to canonical event IDs and update dependent references.
    fn backfill_meeting_identity(conn: &Connection) -> Result<(), DbError> {
        let rows: Vec<(String, String)> = {
            let mut stmt = conn.prepare(
                "SELECT id, calendar_event_id
                 FROM meetings
                 WHERE calendar_event_id IS NOT NULL
                   AND trim(calendar_event_id) != ''",
            )?;
            let mapped = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            let mut items = Vec::new();
            for row in mapped {
                items.push(row?);
            }
            items
        };

        for (old_id, calendar_event_id) in rows {
            let canonical_id = Self::sanitize_calendar_event_id(&calendar_event_id);
            if canonical_id.is_empty() || canonical_id == old_id {
                continue;
            }

            let canonical_exists: i64 = conn.query_row(
                "SELECT COUNT(*) FROM meetings WHERE id = ?1",
                params![canonical_id],
                |r| r.get(0),
            )?;

            if canonical_exists > 0 {
                // Merge sparse fields from old row into canonical row (meetings table).
                conn.execute(
                    "UPDATE meetings
                     SET title = COALESCE(title, (SELECT title FROM meetings WHERE id = ?1)),
                         meeting_type = COALESCE(meeting_type, (SELECT meeting_type FROM meetings WHERE id = ?1)),
                         start_time = COALESCE(start_time, (SELECT start_time FROM meetings WHERE id = ?1)),
                         end_time = COALESCE(end_time, (SELECT end_time FROM meetings WHERE id = ?1)),
                         attendees = COALESCE(attendees, (SELECT attendees FROM meetings WHERE id = ?1)),
                         notes_path = COALESCE(notes_path, (SELECT notes_path FROM meetings WHERE id = ?1)),
                         description = COALESCE(description, (SELECT description FROM meetings WHERE id = ?1))
                     WHERE id = ?2",
                    params![old_id, canonical_id],
                )?;
                // Merge meeting_prep fields
                conn.execute(
                    "UPDATE meeting_prep
                     SET prep_context_json = COALESCE(prep_context_json, (SELECT prep_context_json FROM meeting_prep WHERE meeting_id = ?1)),
                         user_agenda_json = COALESCE(user_agenda_json, (SELECT user_agenda_json FROM meeting_prep WHERE meeting_id = ?1)),
                         user_notes = COALESCE(user_notes, (SELECT user_notes FROM meeting_prep WHERE meeting_id = ?1)),
                         prep_frozen_json = COALESCE(prep_frozen_json, (SELECT prep_frozen_json FROM meeting_prep WHERE meeting_id = ?1)),
                         prep_frozen_at = COALESCE(prep_frozen_at, (SELECT prep_frozen_at FROM meeting_prep WHERE meeting_id = ?1)),
                         prep_snapshot_path = COALESCE(prep_snapshot_path, (SELECT prep_snapshot_path FROM meeting_prep WHERE meeting_id = ?1)),
                         prep_snapshot_hash = COALESCE(prep_snapshot_hash, (SELECT prep_snapshot_hash FROM meeting_prep WHERE meeting_id = ?1))
                     WHERE meeting_id = ?2",
                    params![old_id, canonical_id],
                )?;
                // Merge meeting_transcripts fields
                conn.execute(
                    "UPDATE meeting_transcripts
                     SET summary = COALESCE(summary, (SELECT summary FROM meeting_transcripts WHERE meeting_id = ?1)),
                         transcript_path = COALESCE(transcript_path, (SELECT transcript_path FROM meeting_transcripts WHERE meeting_id = ?1)),
                         transcript_processed_at = COALESCE(transcript_processed_at, (SELECT transcript_processed_at FROM meeting_transcripts WHERE meeting_id = ?1))
                     WHERE meeting_id = ?2",
                    params![old_id, canonical_id],
                )?;
            } else {
                conn.execute(
                    "UPDATE meetings SET id = ?1 WHERE id = ?2",
                    params![canonical_id, old_id],
                )?;
                // Child tables updated via CASCADE on meetings.id
            }

            // Update foreign references.
            conn.execute(
                "UPDATE captures SET meeting_id = ?1 WHERE meeting_id = ?2",
                params![canonical_id, old_id],
            )?;
            conn.execute(
                "UPDATE meeting_entities SET meeting_id = ?1 WHERE meeting_id = ?2",
                params![canonical_id, old_id],
            )?;
            conn.execute(
                "UPDATE meeting_attendees SET meeting_id = ?1 WHERE meeting_id = ?2",
                params![canonical_id, old_id],
            )?;
            conn.execute(
                "UPDATE actions
                 SET source_id = ?1
                 WHERE source_type IN ('transcript', 'post_meeting') AND source_id = ?2",
                params![canonical_id, old_id],
            )?;

            // Update reviewed state keys.
            conn.execute(
                "UPDATE meeting_prep_state
                 SET prep_file = ?1
                 WHERE prep_file = ?2 OR prep_file = ?3",
                params![canonical_id, old_id, format!("preps/{}.json", old_id)],
            )?;

            if canonical_exists > 0 {
                // CASCADE will clean up meeting_prep and meeting_transcripts
                conn.execute("DELETE FROM meetings WHERE id = ?1", params![old_id])?;
            }
        }
        Ok(())
    }

    fn backfill_meeting_user_layer(conn: &Connection) -> Result<(), DbError> {
        let rows: Vec<(String, String, Option<String>, Option<String>)> = {
            let mut stmt = conn.prepare(
                "SELECT mp.meeting_id, mp.prep_context_json, mp.user_agenda_json, mp.user_notes
                 FROM meeting_prep mp
                 WHERE mp.prep_context_json IS NOT NULL
                   AND trim(mp.prep_context_json) != ''",
            )?;
            let mapped = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })?;
            let mut items = Vec::new();
            for row in mapped {
                items.push(row?);
            }
            items
        };

        for (meeting_id, prep_json, agenda_existing, notes_existing) in rows {
            let Ok(value) = serde_json::from_str::<serde_json::Value>(&prep_json) else {
                continue;
            };
            let agenda_from_prep = value
                .get("userAgenda")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.trim().to_string()))
                        .filter(|s| !s.is_empty())
                        .collect::<Vec<String>>()
                })
                .filter(|v| !v.is_empty())
                .and_then(|v| serde_json::to_string(&v).ok());
            let notes_from_prep = value
                .get("userNotes")
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());

            let agenda_target = agenda_existing.or(agenda_from_prep);
            let notes_target = notes_existing.or(notes_from_prep);
            if agenda_target.is_none() && notes_target.is_none() {
                continue;
            }

            conn.execute(
                "UPDATE meeting_prep
                 SET user_agenda_json = COALESCE(user_agenda_json, ?1),
                     user_notes = COALESCE(user_notes, ?2)
                 WHERE meeting_id = ?3",
                params![agenda_target, notes_target, meeting_id],
            )?;
        }
        Ok(())
    }

    /// I652: Backfill engagement/assessment columns on `account_stakeholders` from
    /// the legacy `entity_assessment.stakeholder_insights_json` blob.
    /// Runs once at startup — only touches rows where `engagement IS NULL`.
    fn backfill_stakeholder_columns(conn: &Connection) -> Result<(), DbError> {
        // Step 1: Find all account_stakeholders rows missing engagement.
        let rows: Vec<(String, String)> = {
            let mut stmt = conn.prepare(
                "SELECT account_id, person_id FROM account_stakeholders WHERE engagement IS NULL",
            )?;
            let mapped = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            let mut items = Vec::new();
            for row in mapped {
                items.push(row?);
            }
            items
        };

        if rows.is_empty() {
            return Ok(());
        }

        // Step 2: Group by account_id to avoid repeated JSON parses.
        let mut by_account: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for (account_id, person_id) in &rows {
            by_account
                .entry(account_id.clone())
                .or_default()
                .push(person_id.clone());
        }

        let mut updated = 0u32;
        for (account_id, person_ids) in &by_account {
            // Step 3: Read the stakeholder_insights_json for this entity.
            let json_opt: Option<String> = conn
                .query_row(
                    "SELECT stakeholder_insights_json FROM entity_assessment WHERE entity_id = ?1",
                    params![account_id],
                    |row| row.get(0),
                )
                .ok();

            let json_str = match json_opt {
                Some(ref s) if !s.is_empty() => s.as_str(),
                _ => continue,
            };

            let entries: Vec<serde_json::Value> = match serde_json::from_str(json_str) {
                Ok(v) => v,
                Err(err) => {
                    log::warn!(
                        "I652 backfill: failed to parse stakeholder_insights_json for {}: {}",
                        account_id,
                        err
                    );
                    continue;
                }
            };

            // Step 4: For each person_id, find a matching entry and update.
            for person_id in person_ids {
                let matching = entries.iter().find(|e| {
                    e.get("person_id")
                        .and_then(|v| v.as_str())
                        .map(|pid| pid == person_id)
                        .unwrap_or(false)
                });

                if let Some(entry) = matching {
                    let engagement = entry.get("engagement").and_then(|v| v.as_str());
                    let assessment = entry.get("assessment").and_then(|v| v.as_str());

                    if engagement.is_some() || assessment.is_some() {
                        conn.execute(
                            "UPDATE account_stakeholders
                             SET engagement = COALESCE(engagement, ?1),
                                 assessment = COALESCE(assessment, ?2),
                                 data_source_engagement = COALESCE(data_source_engagement, 'ai'),
                                 data_source_assessment = COALESCE(data_source_assessment, 'ai')
                             WHERE account_id = ?3 AND person_id = ?4
                               AND engagement IS NULL",
                            params![engagement, assessment, account_id, person_id],
                        )?;
                        updated += 1;
                    }
                }
            }
        }

        if updated > 0 {
            log::info!(
                "I652 backfill: populated engagement/assessment for {} stakeholder rows",
                updated
            );
        }
        Ok(())
    }

    /// Check if a one-time init task has been completed.
    ///
    /// Returns true if the task has already run and been marked in init_tasks.
    fn is_init_task_completed(conn: &Connection, task_name: &str) -> Result<bool, DbError> {
        let completed = conn
            .query_row(
                "SELECT 1 FROM init_tasks WHERE task_name = ?1",
                params![task_name],
                |_| Ok(true),
            )
            .unwrap_or(false);
        Ok(completed)
    }

    /// Mark a one-time init task as completed.
    fn mark_init_task_completed(conn: &Connection, task_name: &str) -> Result<(), DbError> {
        conn.execute(
            "INSERT OR IGNORE INTO init_tasks (task_name) VALUES (?1)",
            params![task_name],
        )?;
        Ok(())
    }

    /// Guarded backfill: Account domains from meeting attendees (Path 2b entity resolution).
    ///
    /// Runs exactly once. Subsequent calls are guarded by init_tasks table.
    fn run_guarded_init_backfill_account_domains(&self) -> Result<(), DbError> {
        const TASK_NAME: &str = "backfill_account_domains_v1";

        if Self::is_init_task_completed(&self.conn, TASK_NAME)? {
            return Ok(());
        }

        // Run the backfill
        let inserted = self.backfill_account_domains_from_meetings()?;

        // Mark task as complete
        Self::mark_init_task_completed(&self.conn, TASK_NAME)?;

        if inserted > 0 {
            log::info!(
                "Entity resolution: backfilled {} account→domain mappings from meeting attendees",
                inserted
            );
        }

        Ok(())
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
