//! SQLite-based local state management for actions, accounts, and meeting history.
//!
//! The database lives at `~/.dailyos/dailyos.db` and serves as the working store
//! for operational data (ADR-0048). The filesystem (markdown + JSON) is the durable
//! layer; SQLite enables fast queries, state tracking, and cross-entity intelligence.
//! SQLite is not disposable — important state lives here and is written back to the
//! filesystem at natural synchronization points (archive, dashboard regeneration).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::Utc;
use rusqlite::{params, Connection, OpenFlags};
use crate::entity::{DbEntity, EntityType};
use crate::types::LinkedEntity;

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

pub mod types;
pub use types::*;

pub struct ActionDb {
    conn: Connection,
}

impl ActionDb {
    /// Borrow the underlying connection for ad-hoc queries.
    pub fn conn_ref(&self) -> &Connection {
        &self.conn
    }

    /// Execute a closure within a SQLite transaction.
    /// Commits on Ok, rolls back on Err.
    pub fn with_transaction<F, T>(&self, f: F) -> Result<T, String>
    where
        F: FnOnce(&Self) -> Result<T, String>,
    {
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

        let conn = Connection::open(&path)?;

        // Enable WAL mode for better concurrent read performance
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;

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

        Ok(Self { conn })
    }

    /// Open the database in read-only mode. Used by the MCP binary for safe
    /// concurrent reads while the Tauri app owns writes.
    pub fn open_readonly() -> Result<Self, DbError> {
        let path = Self::db_path()?;
        Self::open_readonly_at(&path)
    }

    /// Open a database at an explicit path in read-only mode.
    pub fn open_readonly_at(path: &std::path::Path) -> Result<Self, DbError> {
        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        Ok(Self { conn })
    }

    /// Resolve the default database path: `~/.dailyos/dailyos.db`.
    ///
    /// When dev-mode DB isolation is active (`set_dev_db_mode(true)`), returns
    /// `~/.dailyos/dailyos-dev.db` instead. Migration logic only applies to the
    /// live path — the dev DB is always created fresh.
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
                 FROM meetings_history
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
                "SELECT COUNT(*) FROM meetings_history WHERE id = ?1",
                params![canonical_id],
                |r| r.get(0),
            )?;

            if canonical_exists > 0 {
                // Merge sparse fields from old row into canonical row.
                conn.execute(
                    "UPDATE meetings_history
                     SET title = COALESCE(title, (SELECT title FROM meetings_history WHERE id = ?1)),
                         meeting_type = COALESCE(meeting_type, (SELECT meeting_type FROM meetings_history WHERE id = ?1)),
                         start_time = COALESCE(start_time, (SELECT start_time FROM meetings_history WHERE id = ?1)),
                         end_time = COALESCE(end_time, (SELECT end_time FROM meetings_history WHERE id = ?1)),
                         attendees = COALESCE(attendees, (SELECT attendees FROM meetings_history WHERE id = ?1)),
                         notes_path = COALESCE(notes_path, (SELECT notes_path FROM meetings_history WHERE id = ?1)),
                         summary = COALESCE(summary, (SELECT summary FROM meetings_history WHERE id = ?1)),
                         prep_context_json = COALESCE(prep_context_json, (SELECT prep_context_json FROM meetings_history WHERE id = ?1)),
                         description = COALESCE(description, (SELECT description FROM meetings_history WHERE id = ?1)),
                         user_agenda_json = COALESCE(user_agenda_json, (SELECT user_agenda_json FROM meetings_history WHERE id = ?1)),
                         user_notes = COALESCE(user_notes, (SELECT user_notes FROM meetings_history WHERE id = ?1)),
                         prep_frozen_json = COALESCE(prep_frozen_json, (SELECT prep_frozen_json FROM meetings_history WHERE id = ?1)),
                         prep_frozen_at = COALESCE(prep_frozen_at, (SELECT prep_frozen_at FROM meetings_history WHERE id = ?1)),
                         prep_snapshot_path = COALESCE(prep_snapshot_path, (SELECT prep_snapshot_path FROM meetings_history WHERE id = ?1)),
                         prep_snapshot_hash = COALESCE(prep_snapshot_hash, (SELECT prep_snapshot_hash FROM meetings_history WHERE id = ?1)),
                         transcript_path = COALESCE(transcript_path, (SELECT transcript_path FROM meetings_history WHERE id = ?1)),
                         transcript_processed_at = COALESCE(transcript_processed_at, (SELECT transcript_processed_at FROM meetings_history WHERE id = ?1))
                     WHERE id = ?2",
                    params![old_id, canonical_id],
                )?;
            } else {
                conn.execute(
                    "UPDATE meetings_history
                     SET id = ?1
                     WHERE id = ?2",
                    params![canonical_id, old_id],
                )?;
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
                 WHERE source_type = 'transcript' AND source_id = ?2",
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
                conn.execute(
                    "DELETE FROM meetings_history WHERE id = ?1",
                    params![old_id],
                )?;
            }
        }
        Ok(())
    }

    fn backfill_meeting_user_layer(conn: &Connection) -> Result<(), DbError> {
        let rows: Vec<(String, String, Option<String>, Option<String>)> = {
            let mut stmt = conn.prepare(
                "SELECT id, prep_context_json, user_agenda_json, user_notes
                 FROM meetings_history
                 WHERE prep_context_json IS NOT NULL
                   AND trim(prep_context_json) != ''",
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
                "UPDATE meetings_history
                 SET user_agenda_json = COALESCE(user_agenda_json, ?1),
                     user_notes = COALESCE(user_notes, ?2)
                 WHERE id = ?3",
                params![agenda_target, notes_target, meeting_id],
            )?;
        }
        Ok(())
    }

}

pub mod actions;
pub mod accounts;
pub mod people;
pub mod meetings;
pub mod projects;
pub mod entities;
pub mod signals;
pub mod emails;
pub mod content;

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
        let db = ActionDb::open_at(path).expect("Failed to open test database");
        db.conn_ref()
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .expect("disable FK for tests");
        db
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
