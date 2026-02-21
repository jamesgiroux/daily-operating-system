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

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use super::test_utils::test_db;

    fn sample_action(id: &str, title: &str) -> DbAction {
        let now = Utc::now().to_rfc3339();
        DbAction {
            id: id.to_string(),
            title: title.to_string(),
            priority: "P2".to_string(),
            status: "pending".to_string(),
            created_at: now.clone(),
            due_date: None,
            completed_at: None,
            account_id: None,
            project_id: None,
            source_type: None,
            source_id: None,
            source_label: None,
            context: None,
            waiting_on: None,
            updated_at: now,
            person_id: None,
            account_name: None,
            next_meeting_title: None,
            next_meeting_start: None,
        }
    }

    #[test]
    fn test_open_creates_tables() {
        let db = test_db();
        // Verify tables exist by querying them (should not error)
        let count: i32 = db
            .conn
            .query_row("SELECT COUNT(*) FROM actions", [], |row| row.get(0))
            .expect("actions table should exist");
        assert_eq!(count, 0);

        let count: i32 = db
            .conn
            .query_row("SELECT COUNT(*) FROM accounts", [], |row| row.get(0))
            .expect("accounts table should exist");
        assert_eq!(count, 0);

        let count: i32 = db
            .conn
            .query_row("SELECT COUNT(*) FROM meetings_history", [], |row| {
                row.get(0)
            })
            .expect("meetings_history table should exist");
        assert_eq!(count, 0);
    }

    #[test]
    fn test_upsert_and_query_action() {
        let db = test_db();

        let action = sample_action("act-001", "Follow up with Acme");
        db.upsert_action(&action).expect("upsert should succeed");

        let results = db.get_due_actions(7).expect("query should succeed");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "act-001");
        assert_eq!(results[0].title, "Follow up with Acme");
    }

    #[test]
    fn test_upsert_updates_existing() {
        let db = test_db();

        let mut action = sample_action("act-002", "Original title");
        db.upsert_action(&action).expect("first upsert");

        action.title = "Updated title".to_string();
        action.priority = "P1".to_string();
        db.upsert_action(&action).expect("second upsert");

        let results = db.get_due_actions(7).expect("query");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Updated title");
        assert_eq!(results[0].priority, "P1");
    }

    #[test]
    fn test_complete_action() {
        let db = test_db();

        let action = sample_action("act-003", "Task to complete");
        db.upsert_action(&action).expect("upsert");

        db.complete_action("act-003").expect("complete");

        // Should no longer appear in pending results
        let results = db.get_due_actions(7).expect("query");
        assert_eq!(results.len(), 0);

        // Verify directly that status changed
        let status: String = db
            .conn
            .query_row(
                "SELECT status FROM actions WHERE id = 'act-003'",
                [],
                |row| row.get(0),
            )
            .expect("direct query");
        assert_eq!(status, "completed");

        // Verify completed_at was set
        let completed_at: Option<String> = db
            .conn
            .query_row(
                "SELECT completed_at FROM actions WHERE id = 'act-003'",
                [],
                |row| row.get(0),
            )
            .expect("direct query");
        assert!(completed_at.is_some());
    }

    #[test]
    fn test_get_account_actions() {
        let db = test_db();

        let mut action1 = sample_action("act-010", "Acme task");
        action1.account_id = Some("acme-corp".to_string());
        db.upsert_action(&action1).expect("upsert 1");

        let mut action2 = sample_action("act-011", "Beta task");
        action2.account_id = Some("beta-inc".to_string());
        db.upsert_action(&action2).expect("upsert 2");

        let mut action3 = sample_action("act-012", "Acme waiting");
        action3.account_id = Some("acme-corp".to_string());
        action3.status = "waiting".to_string();
        action3.waiting_on = Some("John".to_string());
        db.upsert_action(&action3).expect("upsert 3");

        let results = db.get_account_actions("acme-corp").expect("account query");
        assert_eq!(results.len(), 2);
        // Both pending and waiting should appear
        let statuses: Vec<&str> = results.iter().map(|a| a.status.as_str()).collect();
        assert!(statuses.contains(&"pending"));
        assert!(statuses.contains(&"waiting"));
    }

    #[test]
    fn test_upsert_and_query_account() {
        let db = test_db();

        let now = Utc::now().to_rfc3339();
        let account = DbAccount {
            id: "acme-corp".to_string(),
            name: "Acme Corp".to_string(),
            lifecycle: Some("steady-state".to_string()),
            arr: Some(120_000.0),
            health: Some("green".to_string()),
            contract_start: Some("2025-01-01".to_string()),
            contract_end: Some("2026-01-01".to_string()),
            nps: None,
            tracker_path: Some("Accounts/acme-corp".to_string()),
            parent_id: None,
            is_internal: false,
            updated_at: now,
            archived: false,
            keywords: None,
            keywords_extracted_at: None,
            metadata: None,
        };

        db.upsert_account(&account).expect("upsert account");

        let result = db.get_account("acme-corp").expect("get account");
        assert!(result.is_some());
        let acct = result.unwrap();
        assert_eq!(acct.name, "Acme Corp");
        assert_eq!(acct.lifecycle, Some("steady-state".to_string()));
        assert_eq!(acct.arr, Some(120_000.0));
    }

    #[test]
    fn test_get_account_not_found() {
        let db = test_db();
        let result = db.get_account("nonexistent").expect("get account");
        assert!(result.is_none());
    }

    #[test]
    fn test_get_all_accounts_excludes_archived() {
        let db = test_db();
        let now = Utc::now().to_rfc3339();

        let active = DbAccount {
            id: "active-corp".to_string(),
            name: "Active Corp".to_string(),
            lifecycle: None,
            arr: None,
            health: None,
            contract_start: None,
            contract_end: None,
            nps: None,
            tracker_path: None,
            parent_id: None,
            is_internal: false,
            updated_at: now.clone(),
            archived: false,
            keywords: None,
            keywords_extracted_at: None,
            metadata: None,
        };

        let archived = DbAccount {
            id: "archived-corp".to_string(),
            name: "Archived Corp".to_string(),
            lifecycle: None,
            arr: None,
            health: None,
            contract_start: None,
            contract_end: None,
            nps: None,
            tracker_path: None,
            parent_id: None,
            is_internal: false,
            updated_at: now,
            archived: true,
            keywords: None,
            keywords_extracted_at: None,
            metadata: None,
        };

        db.upsert_account(&active).expect("upsert active");
        db.upsert_account(&archived).expect("upsert archived");

        let results = db.get_all_accounts().expect("get all");
        assert_eq!(results.len(), 1, "should only return active account");
        assert_eq!(results[0].id, "active-corp");
        assert!(!results[0].archived);
    }

    #[test]
    fn test_upsert_and_query_meeting() {
        let db = test_db();

        let now = Utc::now().to_rfc3339();
        let meeting = DbMeeting {
            id: "mtg-001".to_string(),
            title: "Acme QBR".to_string(),
            meeting_type: "customer".to_string(),
            start_time: now.clone(),
            end_time: None,
            attendees: Some(r#"["alice@acme.com","bob@us.com"]"#.to_string()),
            notes_path: None,
            summary: Some("Discussed renewal".to_string()),
            created_at: now,
            calendar_event_id: Some("gcal-evt-001".to_string()),
            description: None,
            prep_context_json: None,
            user_agenda_json: None,
            user_notes: None,
            prep_frozen_json: None,
            prep_frozen_at: None,
            prep_snapshot_path: None,
            prep_snapshot_hash: None,
            transcript_path: None,
            transcript_processed_at: None,
            intelligence_state: None,
            intelligence_quality: None,
            last_enriched_at: None,
            signal_count: None,
            has_new_signals: None,
            last_viewed_at: None,
        };

        db.upsert_meeting(&meeting).expect("upsert meeting");
        db.link_meeting_entity("mtg-001", "acme-corp", "account")
            .expect("link meeting entity");

        let results = db
            .get_meeting_history("acme-corp", 30, 10)
            .expect("meeting history");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Acme QBR");
        assert_eq!(results[0].summary, Some("Discussed renewal".to_string()));
    }

    #[test]
    fn test_meeting_history_respects_limit() {
        let db = test_db();
        let now = Utc::now().to_rfc3339();

        for i in 0..5 {
            let mid = format!("mtg-{i:03}");
            let meeting = DbMeeting {
                id: mid.clone(),
                title: format!("Meeting {i}"),
                meeting_type: "customer".to_string(),
                start_time: now.clone(),
                end_time: None,
                attendees: None,
                notes_path: None,
                summary: None,
                created_at: now.clone(),
                calendar_event_id: None,
                description: None,
                prep_context_json: None,
                user_agenda_json: None,
                user_notes: None,
                prep_frozen_json: None,
                prep_frozen_at: None,
                prep_snapshot_path: None,
                prep_snapshot_hash: None,
                transcript_path: None,
                transcript_processed_at: None,
                intelligence_state: None,
                intelligence_quality: None,
                last_enriched_at: None,
                signal_count: None,
                has_new_signals: None,
                last_viewed_at: None,
            };
            db.upsert_meeting(&meeting).expect("upsert");
            db.link_meeting_entity(&mid, "acme-corp", "account")
                .expect("link");
        }

        let results = db.get_meeting_history("acme-corp", 30, 3).expect("history");
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_due_actions_ordering() {
        let db = test_db();

        // P2 with no due date (should appear because due_date IS NULL)
        let action_no_date = sample_action("act-a", "No date task");
        db.upsert_action(&action_no_date).expect("upsert");

        // P1 with future due date
        let mut action_p1 = sample_action("act-b", "P1 future task");
        action_p1.priority = "P1".to_string();
        action_p1.due_date = Some("2099-12-31".to_string());
        db.upsert_action(&action_p1).expect("upsert");

        // P1 overdue
        let mut action_overdue = sample_action("act-c", "Overdue task");
        action_overdue.priority = "P1".to_string();
        action_overdue.due_date = Some("2020-01-01".to_string());
        db.upsert_action(&action_overdue).expect("upsert");

        let results = db.get_due_actions(365_000).expect("query");
        assert_eq!(results.len(), 3);
        // Overdue should be first
        assert_eq!(results[0].id, "act-c");
    }

    #[test]
    fn test_mark_prep_reviewed() {
        let db = test_db();

        db.mark_prep_reviewed("gcal-evt-1", Some("gcal-evt-1"), "Acme Sync")
            .expect("mark reviewed");

        let reviewed = db.get_reviewed_preps().expect("get reviewed");
        assert_eq!(reviewed.len(), 1);
        assert!(reviewed.contains_key("gcal-evt-1"));
    }

    #[test]
    fn test_mark_prep_reviewed_upsert() {
        let db = test_db();

        db.mark_prep_reviewed("0900-acme", None, "Acme")
            .expect("first mark");
        db.mark_prep_reviewed("0900-acme", Some("evt-1"), "Acme")
            .expect("second mark (upsert)");

        let reviewed = db.get_reviewed_preps().expect("get reviewed");
        assert_eq!(reviewed.len(), 1);
    }

    #[test]
    fn test_freeze_meeting_prep_snapshot_is_idempotent() {
        let db = test_db();
        let meeting = DbMeeting {
            id: "evt-1".to_string(),
            title: "Acme Sync".to_string(),
            meeting_type: "customer".to_string(),
            start_time: Utc::now().to_rfc3339(),
            end_time: None,
            attendees: None,
            notes_path: None,
            summary: None,
            created_at: Utc::now().to_rfc3339(),
            calendar_event_id: Some("evt-1".to_string()),
            description: None,
            prep_context_json: None,
            user_agenda_json: None,
            user_notes: None,
            prep_frozen_json: None,
            prep_frozen_at: None,
            prep_snapshot_path: None,
            prep_snapshot_hash: None,
            transcript_path: None,
            transcript_processed_at: None,
            intelligence_state: None,
            intelligence_quality: None,
            last_enriched_at: None,
            signal_count: None,
            has_new_signals: None,
            last_viewed_at: None,
        };
        db.upsert_meeting(&meeting).expect("upsert meeting");

        let first = db
            .freeze_meeting_prep_snapshot(
                "evt-1",
                "{\"k\":\"v\"}",
                "2026-02-12T10:00:00Z",
                "/tmp/snapshot.json",
                "hash-1",
            )
            .expect("first freeze");
        let second = db
            .freeze_meeting_prep_snapshot(
                "evt-1",
                "{\"k\":\"override\"}",
                "2026-02-12T11:00:00Z",
                "/tmp/snapshot-2.json",
                "hash-2",
            )
            .expect("second freeze");
        assert!(first);
        assert!(!second);

        let persisted = db
            .get_meeting_by_id("evt-1")
            .expect("query")
            .expect("row exists");
        assert_eq!(persisted.prep_snapshot_hash.as_deref(), Some("hash-1"));
    }

    #[test]
    fn test_upsert_action_title_dedup() {
        let db = test_db();

        // Insert and complete an action under one ID
        let mut action = sample_action("briefing-001", "Follow up with Acme");
        action.account_id = Some("acme".to_string());
        db.upsert_action(&action).expect("insert");
        db.complete_action("briefing-001").expect("complete");

        // Try to insert the same action under a different ID (cross-source)
        let action2 = DbAction {
            id: "postmeet-999".to_string(),
            title: "Follow up with Acme".to_string(),
            account_id: Some("acme".to_string()),
            ..sample_action("postmeet-999", "Follow up with Acme")
        };
        db.upsert_action_if_not_completed(&action2)
            .expect("dedup upsert");

        // The new action should NOT have been inserted
        let result = db.get_action_by_id("postmeet-999").expect("query");
        assert!(result.is_none(), "Title-based dedup should prevent insert");
    }

    #[test]
    fn test_upsert_action_title_dedup_pending() {
        let db = test_db();

        // Insert a PENDING action
        let action = sample_action("inbox-001", "Review contract");
        db.upsert_action_if_not_completed(&action).expect("insert");

        // Try to insert the same title under a different ID (re-processing same file)
        let action2 = DbAction {
            id: "inbox-002".to_string(),
            title: "Review contract".to_string(),
            ..sample_action("inbox-002", "Review contract")
        };
        db.upsert_action_if_not_completed(&action2)
            .expect("dedup upsert");

        // The duplicate should NOT have been inserted
        let result = db.get_action_by_id("inbox-002").expect("query");
        assert!(
            result.is_none(),
            "Title-based dedup should prevent duplicate pending actions"
        );
    }

    #[test]
    fn test_get_non_briefing_pending_actions() {
        let db = test_db();

        // Insert a briefing-sourced action (should NOT appear)
        let mut briefing_action = sample_action("brief-001", "Briefing task");
        briefing_action.source_type = Some("briefing".to_string());
        db.upsert_action(&briefing_action).expect("insert");

        // Insert a post-meeting action (should appear)
        let mut pm_action = sample_action("pm-001", "Post-meeting task");
        pm_action.source_type = Some("post_meeting".to_string());
        db.upsert_action(&pm_action).expect("insert");

        // Insert an inbox action (should appear)
        let mut inbox_action = sample_action("inbox-001", "Inbox task");
        inbox_action.source_type = Some("inbox".to_string());
        db.upsert_action(&inbox_action).expect("insert");

        // Insert a completed post-meeting action (should NOT appear)
        let mut completed = sample_action("pm-002", "Done task");
        completed.source_type = Some("post_meeting".to_string());
        db.upsert_action(&completed).expect("insert");
        db.complete_action("pm-002").expect("complete");

        // Insert a waiting inbox action (SHOULD appear)
        let mut waiting_action = sample_action("inbox-wait", "Waiting on legal");
        waiting_action.source_type = Some("inbox".to_string());
        waiting_action.status = "waiting".to_string();
        waiting_action.waiting_on = Some("true".to_string());
        db.upsert_action(&waiting_action).expect("insert");

        let results = db.get_non_briefing_pending_actions().expect("query");
        assert_eq!(results.len(), 3);
        let ids: Vec<&str> = results.iter().map(|a| a.id.as_str()).collect();
        assert!(ids.contains(&"pm-001"));
        assert!(ids.contains(&"inbox-001"));
        assert!(ids.contains(&"inbox-wait"));
    }

    #[test]
    fn test_get_captures_for_account() {
        let db = test_db();

        // Insert captures for two accounts
        db.insert_capture(
            "mtg-1",
            "Acme QBR",
            Some("acme"),
            "win",
            "Expanded deployment",
        )
        .expect("insert capture 1");
        db.insert_capture("mtg-1", "Acme QBR", Some("acme"), "risk", "Budget freeze")
            .expect("insert capture 2");
        db.insert_capture(
            "mtg-2",
            "Beta Sync",
            Some("beta"),
            "win",
            "New champion identified",
        )
        .expect("insert capture 3");

        // Query for acme — should get 2
        let results = db
            .get_captures_for_account("acme", 30)
            .expect("query captures");
        assert_eq!(results.len(), 2);

        // Verify capture types are correct
        let types: Vec<&str> = results.iter().map(|c| c.capture_type.as_str()).collect();
        assert!(types.contains(&"win"));
        assert!(types.contains(&"risk"));

        // Query for beta — should get 1
        let results = db
            .get_captures_for_account("beta", 30)
            .expect("query captures");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "New champion identified");

        // Query for nonexistent account — should get 0
        let results = db
            .get_captures_for_account("nonexistent", 30)
            .expect("query captures");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_get_captures_for_date() {
        let db = test_db();

        // Insert captures with explicit timestamps for today and yesterday
        let today = Utc::now().format("%Y-%m-%d").to_string();
        let today_ts = format!("{}T10:00:00+00:00", today);
        let yesterday = (Utc::now() - chrono::Duration::days(1))
            .format("%Y-%m-%d")
            .to_string();
        let yesterday_ts = format!("{}T10:00:00+00:00", yesterday);

        // Today's captures
        db.conn
            .execute(
                "INSERT INTO captures (id, meeting_id, meeting_title, account_id, capture_type, content, captured_at)
                 VALUES ('c1', 'mtg-1', 'Acme QBR', 'acme', 'win', 'Expanded deployment', ?1)",
                params![today_ts],
            )
            .expect("insert c1");
        db.conn
            .execute(
                "INSERT INTO captures (id, meeting_id, meeting_title, account_id, capture_type, content, captured_at)
                 VALUES ('c2', 'mtg-1', 'Acme QBR', 'acme', 'risk', 'Budget freeze', ?1)",
                params![today_ts],
            )
            .expect("insert c2");

        // Yesterday's capture (should NOT appear)
        db.conn
            .execute(
                "INSERT INTO captures (id, meeting_id, meeting_title, account_id, capture_type, content, captured_at)
                 VALUES ('c3', 'mtg-2', 'Beta Sync', 'beta', 'win', 'Old win', ?1)",
                params![yesterday_ts],
            )
            .expect("insert c3");

        let results = db.get_captures_for_date(&today).expect("query");
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].capture_type, "win");
        assert_eq!(results[1].capture_type, "risk");

        // Yesterday should have exactly 1
        let yesterday_results = db.get_captures_for_date(&yesterday).expect("query");
        assert_eq!(yesterday_results.len(), 1);

        // Nonexistent date returns empty
        let empty = db.get_captures_for_date("2020-01-01").expect("query");
        assert!(empty.is_empty());
    }

    #[test]
    fn test_touch_account_last_contact_by_name() {
        let db = test_db();
        let account = DbAccount {
            id: "acme-corp".to_string(),
            name: "Acme Corp".to_string(),
            lifecycle: Some("steady-state".to_string()),
            arr: None,
            health: None,
            contract_start: None,
            contract_end: None,
            nps: None,
            tracker_path: None,
            parent_id: None,
            is_internal: false,
            updated_at: "2020-01-01T00:00:00Z".to_string(),
            archived: false,
            keywords: None,
            keywords_extracted_at: None,
            metadata: None,
        };
        db.upsert_account(&account).expect("upsert");

        // Touch by name (case-insensitive)
        let matched = db.touch_account_last_contact("acme corp").expect("touch");
        assert!(matched, "Should match by case-insensitive name");

        // Verify updated_at changed
        let acct = db.get_account("acme-corp").expect("get").unwrap();
        assert_ne!(acct.updated_at, "2020-01-01T00:00:00Z");
    }

    #[test]
    fn test_touch_account_last_contact_by_id() {
        let db = test_db();
        let account = DbAccount {
            id: "acme-corp".to_string(),
            name: "Acme Corp".to_string(),
            lifecycle: None,
            arr: None,
            health: None,
            contract_start: None,
            contract_end: None,
            nps: None,
            archived: false,
            keywords: None,
            keywords_extracted_at: None,
            metadata: None,
            tracker_path: None,
            parent_id: None,
            is_internal: false,
            updated_at: "2020-01-01T00:00:00Z".to_string(),
        };
        db.upsert_account(&account).expect("upsert");

        let matched = db
            .touch_account_last_contact("acme-corp")
            .expect("touch by id");
        assert!(matched, "Should match by id");
    }

    #[test]
    fn test_touch_account_last_contact_no_match() {
        let db = test_db();
        let matched = db.touch_account_last_contact("nonexistent").expect("touch");
        assert!(!matched, "Should return false when no account matches");
    }

    // =========================================================================
    // Entity tests (ADR-0045)
    // =========================================================================

    #[test]
    fn test_upsert_and_get_entity() {
        let db = test_db();

        let entity = DbEntity {
            id: "proj-alpha".to_string(),
            name: "Project Alpha".to_string(),
            entity_type: EntityType::Project,
            tracker_path: Some("Projects/alpha".to_string()),
            updated_at: "2025-01-01T00:00:00Z".to_string(),
        };
        db.upsert_entity(&entity).expect("upsert entity");

        let result = db.get_entity("proj-alpha").expect("get entity");
        assert!(result.is_some());
        let e = result.unwrap();
        assert_eq!(e.name, "Project Alpha");
        assert_eq!(e.entity_type, EntityType::Project);
        assert_eq!(e.tracker_path, Some("Projects/alpha".to_string()));

        // Not found
        let missing = db.get_entity("nonexistent").expect("get entity");
        assert!(missing.is_none());
    }

    #[test]
    fn test_touch_entity_last_contact() {
        let db = test_db();

        let entity = DbEntity {
            id: "acme".to_string(),
            name: "Acme Corp".to_string(),
            entity_type: EntityType::Account,
            tracker_path: None,
            updated_at: "2020-01-01T00:00:00Z".to_string(),
        };
        db.upsert_entity(&entity).expect("upsert");

        // Touch by name (case-insensitive)
        let matched = db
            .touch_entity_last_contact("acme corp")
            .expect("touch by name");
        assert!(matched);

        let e = db.get_entity("acme").expect("get").unwrap();
        assert_ne!(e.updated_at, "2020-01-01T00:00:00Z");

        // Touch by ID
        let matched_id = db.touch_entity_last_contact("acme").expect("touch by id");
        assert!(matched_id);

        // No match
        let no_match = db.touch_entity_last_contact("nonexistent").expect("touch");
        assert!(!no_match);
    }

    #[test]
    fn test_ensure_entity_for_account() {
        let db = test_db();

        let account = DbAccount {
            id: "beta-inc".to_string(),
            name: "Beta Inc".to_string(),
            lifecycle: Some("ramping".to_string()),
            arr: Some(50_000.0),
            health: Some("yellow".to_string()),
            contract_start: None,
            contract_end: None,
            nps: None,
            tracker_path: Some("Accounts/beta-inc".to_string()),
            parent_id: None,
            is_internal: false,
            updated_at: "2025-06-01T00:00:00Z".to_string(),
            archived: false,
            keywords: None,
            keywords_extracted_at: None,
            metadata: None,
        };

        // upsert_account now calls ensure_entity_for_account automatically
        db.upsert_account(&account).expect("upsert account");

        // Entity should exist with matching fields
        let entity = db.get_entity("beta-inc").expect("get entity").unwrap();
        assert_eq!(entity.name, "Beta Inc");
        assert_eq!(entity.entity_type, EntityType::Account);
        assert_eq!(entity.tracker_path, Some("Accounts/beta-inc".to_string()));
        assert_eq!(entity.updated_at, "2025-06-01T00:00:00Z");
    }

    #[test]
    fn test_get_entities_by_type() {
        let db = test_db();

        let e1 = DbEntity {
            id: "acme".to_string(),
            name: "Acme".to_string(),
            entity_type: EntityType::Account,
            tracker_path: None,
            updated_at: Utc::now().to_rfc3339(),
        };
        let e2 = DbEntity {
            id: "beta".to_string(),
            name: "Beta".to_string(),
            entity_type: EntityType::Account,
            tracker_path: None,
            updated_at: Utc::now().to_rfc3339(),
        };
        let e3 = DbEntity {
            id: "proj-x".to_string(),
            name: "Project X".to_string(),
            entity_type: EntityType::Project,
            tracker_path: None,
            updated_at: Utc::now().to_rfc3339(),
        };

        db.upsert_entity(&e1).expect("upsert");
        db.upsert_entity(&e2).expect("upsert");
        db.upsert_entity(&e3).expect("upsert");

        let accounts = db.get_entities_by_type("account").expect("query");
        assert_eq!(accounts.len(), 2);

        let projects = db.get_entities_by_type("project").expect("query");
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].name, "Project X");
    }

    #[test]
    fn test_idempotent_schema_application() {
        // Opening the same DB twice should not error (IF NOT EXISTS)
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("idempotent.db");

        let _db1 = ActionDb::open_at(path.clone()).expect("first open");
        let _db2 = ActionDb::open_at(path).expect("second open should not fail");
    }

    // =========================================================================
    // Intelligence query tests (I42)
    // =========================================================================

    #[test]
    fn test_get_stale_delegations() {
        let db = test_db();

        // Insert a waiting action created 10 days ago (should be stale at 3-day threshold)
        let mut stale = sample_action("wait-001", "Waiting on legal review");
        stale.status = "waiting".to_string();
        stale.waiting_on = Some("Legal".to_string());
        stale.created_at = "2020-01-01T00:00:00Z".to_string(); // very old
        db.upsert_action(&stale).expect("insert stale");

        // Insert a waiting action created now (should NOT be stale)
        let mut fresh = sample_action("wait-002", "Fresh delegation");
        fresh.status = "waiting".to_string();
        fresh.waiting_on = Some("Bob".to_string());
        db.upsert_action(&fresh).expect("insert fresh");

        // Insert a pending action (not waiting — should NOT appear)
        let pending = sample_action("pend-001", "Pending task");
        db.upsert_action(&pending).expect("insert pending");

        let results = db.get_stale_delegations(3).expect("query");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "wait-001");
        assert_eq!(results[0].waiting_on, Some("Legal".to_string()));
    }

    #[test]
    fn test_get_stale_delegations_empty() {
        let db = test_db();
        let results = db.get_stale_delegations(3).expect("query");
        assert!(results.is_empty());
    }

    #[test]
    fn test_flag_and_get_decisions() {
        let db = test_db();

        // Insert actions
        let mut act1 = sample_action("dec-001", "Decide on vendor");
        act1.due_date = Some("2099-12-31".to_string()); // future, within range
        db.upsert_action(&act1).expect("insert");

        let mut act2 = sample_action("dec-002", "Choose architecture");
        act2.due_date = Some("2099-12-31".to_string());
        db.upsert_action(&act2).expect("insert");

        let act3 = sample_action("dec-003", "Not flagged");
        db.upsert_action(&act3).expect("insert");

        // Flag only the first two
        assert!(db.flag_action_as_decision("dec-001").expect("flag"));
        assert!(db.flag_action_as_decision("dec-002").expect("flag"));

        // Non-existent action returns false
        assert!(!db.flag_action_as_decision("nonexistent").expect("flag"));

        // Query with large lookahead — should get both flagged actions
        let results = db.get_flagged_decisions(365_000).expect("query");
        assert_eq!(results.len(), 2);
        let ids: Vec<&str> = results.iter().map(|a| a.id.as_str()).collect();
        assert!(ids.contains(&"dec-001"));
        assert!(ids.contains(&"dec-002"));
    }

    #[test]
    fn test_flagged_decisions_excludes_completed() {
        let db = test_db();

        let mut act = sample_action("dec-010", "Completed decision");
        act.due_date = Some("2099-12-31".to_string());
        db.upsert_action(&act).expect("insert");
        db.flag_action_as_decision("dec-010").expect("flag");
        db.complete_action("dec-010").expect("complete");

        let results = db.get_flagged_decisions(365_000).expect("query");
        assert!(results.is_empty(), "Completed actions should not appear");
    }

    #[test]
    fn test_flagged_decisions_includes_no_due_date() {
        let db = test_db();

        // Action with no due date but flagged
        let act = sample_action("dec-020", "Open-ended decision");
        db.upsert_action(&act).expect("insert");
        db.flag_action_as_decision("dec-020").expect("flag");

        let results = db.get_flagged_decisions(3).expect("query");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "dec-020");
    }

    #[test]
    fn test_clear_decision_flags() {
        let db = test_db();

        let act = sample_action("dec-030", "Will be unflagged");
        db.upsert_action(&act).expect("insert");
        db.flag_action_as_decision("dec-030").expect("flag");

        // Verify flagged
        let before = db.get_flagged_decisions(365_000).expect("query");
        assert_eq!(before.len(), 1);

        // Clear
        db.clear_decision_flags().expect("clear");

        let after = db.get_flagged_decisions(365_000).expect("query");
        assert!(after.is_empty(), "All flags should be cleared");
    }

    #[test]
    fn test_get_renewal_alerts() {
        let db = test_db();

        // Account renewing in 30 days (should appear at 60-day threshold)
        let soon = DbAccount {
            id: "renew-soon".to_string(),
            name: "Renewing Soon Corp".to_string(),
            lifecycle: Some("steady-state".to_string()),
            arr: Some(100_000.0),
            health: Some("green".to_string()),
            contract_start: Some("2025-01-01".to_string()),
            contract_end: Some(
                (Utc::now() + chrono::Duration::days(30))
                    .format("%Y-%m-%d")
                    .to_string(),
            ),
            nps: None,
            tracker_path: None,
            parent_id: None,
            is_internal: false,
            updated_at: Utc::now().to_rfc3339(),
            archived: false,
            keywords: None,
            keywords_extracted_at: None,
            metadata: None,
        };
        db.upsert_account(&soon).expect("insert");

        // Account with no contract_end (should NOT appear)
        let no_end = DbAccount {
            id: "no-end".to_string(),
            name: "No End Corp".to_string(),
            lifecycle: Some("ramping".to_string()),
            arr: None,
            health: None,
            contract_start: None,
            contract_end: None,
            nps: None,
            tracker_path: None,
            parent_id: None,
            is_internal: false,
            updated_at: Utc::now().to_rfc3339(),
            archived: false,
            keywords: None,
            keywords_extracted_at: None,
            metadata: None,
        };
        db.upsert_account(&no_end).expect("insert");

        // Account already expired (should NOT appear — contract_end < now)
        let expired = DbAccount {
            id: "expired".to_string(),
            name: "Expired Corp".to_string(),
            lifecycle: Some("onboarding".to_string()),
            arr: None,
            health: None,
            contract_start: None,
            contract_end: Some("2020-01-01".to_string()),
            nps: None,
            tracker_path: None,
            parent_id: None,
            is_internal: false,
            updated_at: Utc::now().to_rfc3339(),
            archived: false,
            keywords: None,
            keywords_extracted_at: None,
            metadata: None,
        };
        db.upsert_account(&expired).expect("insert");

        let results = db.get_renewal_alerts(60).expect("query");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "renew-soon");
    }

    #[test]
    fn test_get_stale_accounts() {
        let db = test_db();

        // Account updated 60 days ago (should be stale at 30-day threshold)
        let stale = DbAccount {
            id: "stale-acct".to_string(),
            name: "Stale Corp".to_string(),
            lifecycle: Some("ramping".to_string()),
            arr: None,
            health: None,
            contract_start: None,
            contract_end: None,
            nps: None,
            tracker_path: None,
            parent_id: None,
            is_internal: false,
            updated_at: "2020-01-01T00:00:00Z".to_string(),
            archived: false,
            keywords: None,
            keywords_extracted_at: None,
            metadata: None,
        };
        db.upsert_account(&stale).expect("insert");

        // Account updated just now (should NOT be stale)
        let fresh = DbAccount {
            id: "fresh-acct".to_string(),
            name: "Fresh Corp".to_string(),
            lifecycle: Some("steady-state".to_string()),
            arr: None,
            health: None,
            contract_start: None,
            contract_end: None,
            nps: None,
            tracker_path: None,
            parent_id: None,
            is_internal: false,
            updated_at: Utc::now().to_rfc3339(),
            archived: false,
            keywords: None,
            keywords_extracted_at: None,
            metadata: None,
        };
        db.upsert_account(&fresh).expect("insert");

        let results = db.get_stale_accounts(30).expect("query");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "stale-acct");
    }

    #[test]
    fn test_needs_decision_migration() {
        // Verify the needs_decision column exists after opening a fresh DB
        let db = test_db();
        let act = sample_action("mig-001", "Test migration");
        db.upsert_action(&act).expect("insert");

        // Should be able to flag it without error
        db.flag_action_as_decision("mig-001").expect("flag");

        // Verify directly
        let flagged: i32 = db
            .conn
            .query_row(
                "SELECT needs_decision FROM actions WHERE id = 'mig-001'",
                [],
                |row| row.get(0),
            )
            .expect("direct query");
        assert_eq!(flagged, 1);
    }

    // =========================================================================
    // Stakeholder Signals tests (I43)
    // =========================================================================

    #[test]
    fn test_stakeholder_signals_empty() {
        let db = test_db();
        let signals = db
            .get_stakeholder_signals("nonexistent-corp")
            .expect("should not error for missing account");
        assert_eq!(signals.meeting_frequency_30d, 0);
        assert_eq!(signals.meeting_frequency_90d, 0);
        assert!(signals.last_meeting.is_none());
        assert!(signals.last_contact.is_none());
        assert_eq!(signals.temperature, "cold");
        assert_eq!(signals.trend, "stable");
    }

    #[test]
    fn test_stakeholder_signals_with_meetings() {
        let db = test_db();
        let now = Utc::now();

        // Insert recent meetings
        for i in 0..5 {
            let mid = format!("mtg-{}", i);
            let meeting = DbMeeting {
                id: mid.clone(),
                title: format!("Sync #{}", i),
                meeting_type: "customer".to_string(),
                start_time: (now - chrono::Duration::days(i * 5)).to_rfc3339(),
                end_time: None,
                attendees: None,
                notes_path: None,
                summary: None,
                created_at: now.to_rfc3339(),
                calendar_event_id: None,
                description: None,
                prep_context_json: None,
                user_agenda_json: None,
                user_notes: None,
                prep_frozen_json: None,
                prep_frozen_at: None,
                prep_snapshot_path: None,
                prep_snapshot_hash: None,
                transcript_path: None,
                transcript_processed_at: None,
                intelligence_state: None,
                intelligence_quality: None,
                last_enriched_at: None,
                signal_count: None,
                has_new_signals: None,
                last_viewed_at: None,
            };
            db.upsert_meeting(&meeting).expect("insert meeting");
            db.link_meeting_entity(&mid, "acme-corp", "account")
                .expect("link");
        }

        let signals = db.get_stakeholder_signals("acme-corp").expect("signals");
        assert_eq!(signals.meeting_frequency_30d, 5);
        assert_eq!(signals.meeting_frequency_90d, 5);
        assert!(signals.last_meeting.is_some());
        assert_eq!(signals.temperature, "hot"); // most recent < 7 days ago
    }

    #[test]
    fn test_stakeholder_signals_with_account_contact() {
        let db = test_db();

        let account = DbAccount {
            id: "acme-corp".to_string(),
            name: "Acme Corp".to_string(),
            lifecycle: None,
            arr: None,
            health: None,
            contract_start: None,
            contract_end: None,
            nps: None,
            tracker_path: None,
            parent_id: None,
            is_internal: false,
            updated_at: Utc::now().to_rfc3339(),
            archived: false,
            keywords: None,
            keywords_extracted_at: None,
            metadata: None,
        };
        db.upsert_account(&account).expect("insert account");

        let signals = db.get_stakeholder_signals("acme-corp").expect("signals");
        assert!(signals.last_contact.is_some());
    }

    #[test]
    fn test_compute_temperature() {
        assert_eq!(super::compute_temperature(&Utc::now().to_rfc3339()), "hot");

        let days_ago_10 = (Utc::now() - chrono::Duration::days(10)).to_rfc3339();
        assert_eq!(super::compute_temperature(&days_ago_10), "warm");

        let days_ago_45 = (Utc::now() - chrono::Duration::days(45)).to_rfc3339();
        assert_eq!(super::compute_temperature(&days_ago_45), "cool");

        let days_ago_90 = (Utc::now() - chrono::Duration::days(90)).to_rfc3339();
        assert_eq!(super::compute_temperature(&days_ago_90), "cold");
    }

    #[test]
    fn test_compute_trend() {
        // Even distribution: 3 in 30d out of 9 in 90d → stable
        assert_eq!(super::compute_trend(3, 9), "stable");

        // Increasing: 5 in 30d out of 6 in 90d (way above 1/3)
        assert_eq!(super::compute_trend(5, 6), "increasing");

        // Decreasing: 0 in 30d out of 9 in 90d (way below 1/3)
        assert_eq!(super::compute_trend(0, 9), "decreasing");

        // No data: should be stable
        assert_eq!(super::compute_trend(0, 0), "stable");
    }

    // =========================================================================
    // People Tests (I51)
    // =========================================================================

    fn sample_person(email: &str) -> DbPerson {
        let now = Utc::now().to_rfc3339();
        DbPerson {
            id: crate::util::person_id_from_email(email),
            email: email.to_lowercase(),
            name: crate::util::name_from_email(email),
            organization: Some(crate::util::org_from_email(email)),
            role: None,
            relationship: "unknown".to_string(),
            notes: None,
            tracker_path: None,
            last_seen: None,
            first_seen: Some(now.clone()),
            meeting_count: 0,
            updated_at: now,
            archived: false,
            linkedin_url: None,
            twitter_handle: None,
            phone: None,
            photo_url: None,
            bio: None,
            title_history: None,
            company_industry: None,
            company_size: None,
            company_hq: None,
            last_enriched_at: None,
            enrichment_sources: None,
        }
    }

    #[test]
    fn test_upsert_and_get_person() {
        let db = test_db();
        let person = sample_person("sarah.chen@acme.com");
        db.upsert_person(&person).expect("upsert person");

        let result = db.get_person(&person.id).expect("get person");
        assert!(result.is_some());
        let p = result.unwrap();
        assert_eq!(p.name, "Sarah Chen");
        assert_eq!(p.email, "sarah.chen@acme.com");
        assert_eq!(p.organization, Some("Acme".to_string()));
    }

    #[test]
    fn test_get_person_by_email() {
        let db = test_db();
        let person = sample_person("bob@example.com");
        db.upsert_person(&person).expect("upsert");

        let result = db
            .get_person_by_email("BOB@EXAMPLE.COM")
            .expect("get by email");
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, person.id);
    }

    #[test]
    fn test_get_person_by_email_or_alias() {
        let db = test_db();
        let person = sample_person("alice@acme.com");
        db.upsert_person(&person).expect("upsert");

        // Exact match still works
        let result = db
            .get_person_by_email_or_alias("alice@acme.com")
            .expect("exact match");
        assert!(result.is_some());
        assert_eq!(result.as_ref().unwrap().id, person.id);

        // Add an alias
        db.add_person_email(&person.id, "alice@acmecorp.com", false)
            .expect("add alias");

        // Alias lookup works
        let result = db
            .get_person_by_email_or_alias("alice@acmecorp.com")
            .expect("alias match");
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, person.id);

        // Unknown email returns None
        let result = db
            .get_person_by_email_or_alias("unknown@nowhere.com")
            .expect("no match");
        assert!(result.is_none());
    }

    #[test]
    fn test_find_person_by_domain_alias() {
        let db = test_db();
        let person = sample_person("renan@a8c.com");
        db.upsert_person(&person).expect("upsert");

        // Search for the same local part at a sibling domain
        let result = db
            .find_person_by_domain_alias("renan@wpvip.com", &["a8c.com".to_string()])
            .expect("domain alias search");
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, person.id);

        // No match when sibling domains don't contain the person
        let result = db
            .find_person_by_domain_alias("renan@wpvip.com", &["unknown.com".to_string()])
            .expect("no match");
        assert!(result.is_none());
    }

    #[test]
    fn test_get_sibling_domains_for_email() {
        let db = test_db();

        // Set up an account with multiple domains
        setup_account(&db, "acc1", "Automattic");
        db.set_account_domains("acc1", &["a8c.com".to_string(), "wpvip.com".to_string()])
            .expect("set domains");

        // Email at a8c.com should return wpvip.com as sibling
        let siblings = db
            .get_sibling_domains_for_email("renan@a8c.com", &[])
            .expect("siblings");
        assert!(siblings.contains(&"wpvip.com".to_string()));
        assert!(!siblings.contains(&"a8c.com".to_string())); // self excluded

        // Personal email domains should return no siblings
        let siblings = db
            .get_sibling_domains_for_email("alice@gmail.com", &[])
            .expect("personal");
        assert!(siblings.is_empty());

        // user_domains path
        let user_domains = vec!["myco.com".to_string(), "myco.io".to_string()];
        let siblings = db
            .get_sibling_domains_for_email("alice@myco.com", &user_domains)
            .expect("user domains");
        assert!(siblings.contains(&"myco.io".to_string()));
    }

    #[test]
    fn test_person_emails_crud() {
        let db = test_db();
        let person = sample_person("alice@acme.com");
        db.upsert_person(&person).expect("upsert");

        // upsert_person should auto-seed person_emails
        let emails = db.get_person_emails(&person.id).expect("list emails");
        assert_eq!(emails.len(), 1);
        assert_eq!(emails[0], "alice@acme.com");

        // Add an alias
        db.add_person_email(&person.id, "alice@acmecorp.com", false)
            .expect("add alias");
        let emails = db.get_person_emails(&person.id).expect("list emails");
        assert_eq!(emails.len(), 2);

        // Duplicate insert is idempotent
        db.add_person_email(&person.id, "alice@acmecorp.com", false)
            .expect("duplicate add");
        let emails = db.get_person_emails(&person.id).expect("list emails");
        assert_eq!(emails.len(), 2);
    }

    #[test]
    fn test_merge_people_transfers_aliases() {
        let db = test_db();
        let p1 = sample_person("alice@acme.com");
        let p2 = sample_person("alice@acmecorp.com");
        db.upsert_person(&p1).expect("upsert p1");
        db.upsert_person(&p2).expect("upsert p2");

        // Both have their primary emails
        assert_eq!(db.get_person_emails(&p1.id).unwrap().len(), 1);
        assert_eq!(db.get_person_emails(&p2.id).unwrap().len(), 1);

        // Merge p2 into p1
        db.merge_people(&p1.id, &p2.id).expect("merge");

        // p1 should now have both emails
        let emails = db.get_person_emails(&p1.id).unwrap();
        assert!(emails.contains(&"alice@acme.com".to_string()));
        assert!(emails.contains(&"alice@acmecorp.com".to_string()));

        // p2 should be gone
        assert!(db.get_person(&p2.id).unwrap().is_none());
        assert!(db.get_person_emails(&p2.id).unwrap().is_empty());
    }

    #[test]
    fn test_alias_aware_person_resolution_integration() {
        let db = test_db();

        // Set up account with two domains
        setup_account(&db, "acc1", "Automattic");
        db.set_account_domains("acc1", &["a8c.com".to_string(), "wpvip.com".to_string()])
            .expect("set domains");

        // Create person from domain A
        let person = sample_person("renan@a8c.com");
        db.upsert_person(&person).expect("upsert");

        // Simulate: calendar event arrives with renan@wpvip.com
        let email = "renan@wpvip.com";
        let found = db.get_person_by_email_or_alias(email).ok().flatten();
        assert!(found.is_none(), "no direct match yet");

        // Get siblings and try domain alias
        let siblings = db
            .get_sibling_domains_for_email(email, &[])
            .expect("siblings");
        assert!(!siblings.is_empty());

        let found = db
            .find_person_by_domain_alias(email, &siblings)
            .expect("domain alias");
        assert!(found.is_some());
        assert_eq!(found.as_ref().unwrap().id, person.id);

        // Record the alias
        db.add_person_email(&person.id, email, false)
            .expect("record alias");

        // Now direct alias lookup should work
        let found = db
            .get_person_by_email_or_alias(email)
            .expect("alias lookup");
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, person.id);

        // person_emails should have both
        let emails = db.get_person_emails(&person.id).unwrap();
        assert_eq!(emails.len(), 2);
    }

    #[test]
    fn test_get_people_with_filter() {
        let db = test_db();
        let mut p1 = sample_person("alice@myco.com");
        p1.relationship = "internal".to_string();
        let mut p2 = sample_person("bob@other.com");
        p2.relationship = "external".to_string();

        db.upsert_person(&p1).expect("upsert p1");
        db.upsert_person(&p2).expect("upsert p2");

        let all = db.get_people(None).expect("get all");
        assert_eq!(all.len(), 2);

        let internal = db.get_people(Some("internal")).expect("get internal");
        assert_eq!(internal.len(), 1);
        assert_eq!(internal[0].name, "Alice");

        let external = db.get_people(Some("external")).expect("get external");
        assert_eq!(external.len(), 1);
        assert_eq!(external[0].name, "Bob");
    }

    #[test]
    fn test_get_people_excludes_archived() {
        let db = test_db();

        let mut active = sample_person("active@test.com");
        active.relationship = "external".to_string();

        let mut archived = sample_person("archived@test.com");
        archived.relationship = "external".to_string();
        archived.archived = true;

        db.upsert_person(&active).expect("upsert active");
        db.upsert_person(&archived).expect("upsert archived");

        // No filter — should exclude archived
        let all = db.get_people(None).expect("get all");
        assert_eq!(all.len(), 1, "should only return active person");
        assert_eq!(all[0].email, "active@test.com");

        // With relationship filter — should also exclude archived
        let filtered = db.get_people(Some("external")).expect("get external");
        assert_eq!(
            filtered.len(),
            1,
            "should only return active external person"
        );
        assert_eq!(filtered[0].email, "active@test.com");
    }

    #[test]
    fn test_person_entity_linking() {
        let db = test_db();
        let person = sample_person("jane@acme.com");
        db.upsert_person(&person).expect("upsert person");

        let account = DbAccount {
            id: "acme-corp".to_string(),
            name: "Acme Corp".to_string(),
            lifecycle: None,
            arr: None,
            health: None,
            contract_start: None,
            contract_end: None,
            nps: None,
            tracker_path: None,
            parent_id: None,
            is_internal: false,
            updated_at: Utc::now().to_rfc3339(),
            archived: false,
            keywords: None,
            keywords_extracted_at: None,
            metadata: None,
        };
        db.upsert_account(&account).expect("upsert account");

        db.link_person_to_entity(&person.id, "acme-corp", "associated")
            .expect("link");

        let people = db
            .get_people_for_entity("acme-corp")
            .expect("people for entity");
        assert_eq!(people.len(), 1);
        assert_eq!(people[0].id, person.id);

        let entities = db
            .get_entities_for_person(&person.id)
            .expect("entities for person");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].id, "acme-corp");

        // Unlink
        db.unlink_person_from_entity(&person.id, "acme-corp")
            .expect("unlink");
        let people_after = db.get_people_for_entity("acme-corp").expect("after unlink");
        assert_eq!(people_after.len(), 0);
    }

    #[test]
    fn test_meeting_attendance() {
        let db = test_db();
        let person = sample_person("attendee@test.com");
        db.upsert_person(&person).expect("upsert person");

        let now = Utc::now().to_rfc3339();
        let meeting = DbMeeting {
            id: "mtg-attend-001".to_string(),
            title: "Test Meeting".to_string(),
            meeting_type: "internal".to_string(),
            start_time: now.clone(),
            end_time: None,
            attendees: None,
            notes_path: None,
            summary: None,
            created_at: now,
            calendar_event_id: None,
            description: None,
            prep_context_json: None,
            user_agenda_json: None,
            user_notes: None,
            prep_frozen_json: None,
            prep_frozen_at: None,
            prep_snapshot_path: None,
            prep_snapshot_hash: None,
            transcript_path: None,
            transcript_processed_at: None,
            intelligence_state: None,
            intelligence_quality: None,
            last_enriched_at: None,
            signal_count: None,
            has_new_signals: None,
            last_viewed_at: None,
        };
        db.upsert_meeting(&meeting).expect("upsert meeting");
        db.record_meeting_attendance("mtg-attend-001", &person.id)
            .expect("record attendance");

        // Check attendees for meeting
        let attendees = db
            .get_meeting_attendees("mtg-attend-001")
            .expect("get attendees");
        assert_eq!(attendees.len(), 1);
        assert_eq!(attendees[0].id, person.id);

        // Check meetings for person
        let meetings = db
            .get_person_meetings(&person.id, 10)
            .expect("person meetings");
        assert_eq!(meetings.len(), 1);
        assert_eq!(meetings[0].id, "mtg-attend-001");

        // Check meeting_count was incremented
        let updated = db.get_person(&person.id).expect("get updated").unwrap();
        assert_eq!(updated.meeting_count, 1);

        // Idempotent: recording again should not increment
        db.record_meeting_attendance("mtg-attend-001", &person.id)
            .expect("re-record");
        let same = db.get_person(&person.id).expect("get same").unwrap();
        assert_eq!(same.meeting_count, 1);
    }

    #[test]
    fn test_search_people() {
        let db = test_db();
        db.upsert_person(&sample_person("alice@acme.com"))
            .expect("upsert");
        db.upsert_person(&sample_person("bob@bigcorp.io"))
            .expect("upsert");

        let results = db.search_people("acme", 10).expect("search");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Alice");

        let results = db.search_people("bob", 10).expect("search");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_update_person_field() {
        let db = test_db();
        let person = sample_person("field@test.com");
        db.upsert_person(&person).expect("upsert");

        db.update_person_field(&person.id, "role", "VP Engineering")
            .expect("update role");
        let updated = db.get_person(&person.id).expect("get").unwrap();
        assert_eq!(updated.role, Some("VP Engineering".to_string()));

        // Invalid field should error
        let err = db.update_person_field(&person.id, "invalid_field", "val");
        assert!(err.is_err());
    }

    #[test]
    fn test_person_signals_empty() {
        let db = test_db();
        let person = sample_person("nobody@test.com");
        db.upsert_person(&person).expect("upsert");

        let signals = db.get_person_signals(&person.id).expect("signals");
        assert_eq!(signals.meeting_frequency_30d, 0);
        assert_eq!(signals.temperature, "cold");
        assert_eq!(signals.trend, "stable");
    }

    #[test]
    fn test_people_table_created() {
        let db = test_db();
        let count: i32 = db
            .conn
            .query_row("SELECT COUNT(*) FROM people", [], |row| row.get(0))
            .expect("people table should exist");
        assert_eq!(count, 0);

        let count: i32 = db
            .conn
            .query_row("SELECT COUNT(*) FROM meeting_attendees", [], |row| {
                row.get(0)
            })
            .expect("meeting_attendees table should exist");
        assert_eq!(count, 0);

        let count: i32 = db
            .conn
            .query_row("SELECT COUNT(*) FROM entity_people", [], |row| row.get(0))
            .expect("entity_people table should exist");
        assert_eq!(count, 0);

        let count: i32 = db
            .conn
            .query_row("SELECT COUNT(*) FROM meeting_entities", [], |row| {
                row.get(0)
            })
            .expect("meeting_entities table should exist");
        assert_eq!(count, 0);
    }

    // =========================================================================
    // Merge + Delete People
    // =========================================================================

    fn make_meeting(db: &ActionDb, id: &str) {
        let now = Utc::now().to_rfc3339();
        let meeting = DbMeeting {
            id: id.to_string(),
            title: format!("Meeting {}", id),
            meeting_type: "internal".to_string(),
            start_time: now.clone(),
            end_time: None,
            attendees: None,
            notes_path: None,
            summary: None,
            created_at: now,
            calendar_event_id: None,
            description: None,
            prep_context_json: None,
            user_agenda_json: None,
            user_notes: None,
            prep_frozen_json: None,
            prep_frozen_at: None,
            prep_snapshot_path: None,
            prep_snapshot_hash: None,
            transcript_path: None,
            transcript_processed_at: None,
            intelligence_state: None,
            intelligence_quality: None,
            last_enriched_at: None,
            signal_count: None,
            has_new_signals: None,
            last_viewed_at: None,
        };
        db.upsert_meeting(&meeting).expect("upsert meeting");
    }

    #[test]
    fn test_merge_transfers_attendees() {
        let db = test_db();
        let keep = sample_person("keep@acme.com");
        let remove = sample_person("remove@other.com");
        db.upsert_person(&keep).expect("upsert keep");
        db.upsert_person(&remove).expect("upsert remove");

        make_meeting(&db, "mtg-a");
        make_meeting(&db, "mtg-b");
        db.record_meeting_attendance("mtg-a", &keep.id)
            .expect("attend");
        db.record_meeting_attendance("mtg-b", &remove.id)
            .expect("attend");

        db.merge_people(&keep.id, &remove.id).expect("merge");

        let meetings = db.get_person_meetings(&keep.id, 50).expect("meetings");
        assert_eq!(meetings.len(), 2, "kept person should attend both meetings");
    }

    #[test]
    fn test_merge_transfers_entity_links() {
        let db = test_db();
        let keep = sample_person("keep@acme.com");
        let remove = sample_person("remove@other.com");
        db.upsert_person(&keep).expect("upsert");
        db.upsert_person(&remove).expect("upsert");

        let account = DbAccount {
            id: "acme".to_string(),
            name: "Acme".to_string(),
            lifecycle: None,
            arr: None,
            health: None,
            contract_start: None,
            contract_end: None,
            nps: None,
            tracker_path: None,
            parent_id: None,
            is_internal: false,
            updated_at: Utc::now().to_rfc3339(),
            archived: false,
            keywords: None,
            keywords_extracted_at: None,
            metadata: None,
        };
        db.upsert_account(&account).expect("upsert account");
        db.link_person_to_entity(&remove.id, "acme", "associated")
            .expect("link");

        db.merge_people(&keep.id, &remove.id).expect("merge");

        let entities = db.get_entities_for_person(&keep.id).expect("entities");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].id, "acme");
    }

    #[test]
    fn test_merge_transfers_actions() {
        let db = test_db();
        let keep = sample_person("keep@acme.com");
        let remove = sample_person("remove@other.com");
        db.upsert_person(&keep).expect("upsert");
        db.upsert_person(&remove).expect("upsert");

        let mut action = sample_action("act-1", "Follow up");
        action.person_id = Some(remove.id.clone());
        db.upsert_action(&action).expect("upsert action");

        db.merge_people(&keep.id, &remove.id).expect("merge");

        let fetched = db.get_action_by_id("act-1").expect("get action").unwrap();
        assert_eq!(fetched.person_id, Some(keep.id));
    }

    #[test]
    fn test_merge_handles_shared_meetings() {
        let db = test_db();
        let keep = sample_person("keep@acme.com");
        let remove = sample_person("remove@other.com");
        db.upsert_person(&keep).expect("upsert");
        db.upsert_person(&remove).expect("upsert");

        make_meeting(&db, "mtg-shared");
        db.record_meeting_attendance("mtg-shared", &keep.id)
            .expect("attend");
        db.record_meeting_attendance("mtg-shared", &remove.id)
            .expect("attend");

        // Should not fail despite both attending the same meeting
        db.merge_people(&keep.id, &remove.id)
            .expect("merge should succeed with shared meetings");

        let attendees = db.get_meeting_attendees("mtg-shared").expect("attendees");
        assert_eq!(attendees.len(), 1, "only kept person remains");
        assert_eq!(attendees[0].id, keep.id);
    }

    #[test]
    fn test_merge_deletes_removed() {
        let db = test_db();
        let keep = sample_person("keep@acme.com");
        let remove = sample_person("remove@other.com");
        db.upsert_person(&keep).expect("upsert");
        db.upsert_person(&remove).expect("upsert");

        db.merge_people(&keep.id, &remove.id).expect("merge");

        assert!(
            db.get_person(&remove.id).expect("get").is_none(),
            "removed person should be gone"
        );
        assert!(
            db.get_person(&keep.id).expect("get").is_some(),
            "kept person should still exist"
        );
    }

    #[test]
    fn test_merge_recomputes_count() {
        let db = test_db();
        let keep = sample_person("keep@acme.com");
        let remove = sample_person("remove@other.com");
        db.upsert_person(&keep).expect("upsert");
        db.upsert_person(&remove).expect("upsert");

        make_meeting(&db, "mtg-1");
        make_meeting(&db, "mtg-2");
        make_meeting(&db, "mtg-3");
        db.record_meeting_attendance("mtg-1", &keep.id)
            .expect("attend");
        db.record_meeting_attendance("mtg-2", &remove.id)
            .expect("attend");
        db.record_meeting_attendance("mtg-3", &remove.id)
            .expect("attend");

        db.merge_people(&keep.id, &remove.id).expect("merge");

        let person = db.get_person(&keep.id).expect("get").unwrap();
        assert_eq!(person.meeting_count, 3, "should have all 3 meetings");
    }

    #[test]
    fn test_merge_nonexistent_fails() {
        let db = test_db();
        let keep = sample_person("keep@acme.com");
        db.upsert_person(&keep).expect("upsert");

        let err = db.merge_people(&keep.id, "nonexistent-id");
        assert!(
            err.is_err(),
            "merge should fail when remove_id doesn't exist"
        );

        let err = db.merge_people("nonexistent-id", &keep.id);
        assert!(err.is_err(), "merge should fail when keep_id doesn't exist");
    }

    #[test]
    fn test_delete_person_cascades() {
        let db = test_db();
        let person = sample_person("doomed@test.com");
        db.upsert_person(&person).expect("upsert");

        make_meeting(&db, "mtg-doom");
        db.record_meeting_attendance("mtg-doom", &person.id)
            .expect("attend");

        let account = DbAccount {
            id: "doom-corp".to_string(),
            name: "Doom Corp".to_string(),
            lifecycle: None,
            arr: None,
            health: None,
            contract_start: None,
            contract_end: None,
            nps: None,
            tracker_path: None,
            parent_id: None,
            is_internal: false,
            updated_at: Utc::now().to_rfc3339(),
            archived: false,
            keywords: None,
            keywords_extracted_at: None,
            metadata: None,
        };
        db.upsert_account(&account).expect("upsert account");
        db.link_person_to_entity(&person.id, "doom-corp", "associated")
            .expect("link");

        let mut action = sample_action("act-doom", "Doomed action");
        action.person_id = Some(person.id.clone());
        db.upsert_action(&action).expect("upsert action");

        db.delete_person(&person.id).expect("delete");

        // Person gone
        assert!(db.get_person(&person.id).expect("get").is_none());

        // Attendance gone
        let attendees = db.get_meeting_attendees("mtg-doom").expect("attendees");
        assert_eq!(attendees.len(), 0);

        // Entity link gone
        let people = db.get_people_for_entity("doom-corp").expect("people");
        assert_eq!(people.len(), 0);

        // Action person_id nulled
        let action = db
            .get_action_by_id("act-doom")
            .expect("get action")
            .unwrap();
        assert!(
            action.person_id.is_none(),
            "person_id should be nulled, not left dangling"
        );
    }

    // =========================================================================
    // Projects (I50)
    // =========================================================================

    #[test]
    fn test_upsert_and_get_project() {
        let db = test_db();
        let now = Utc::now().to_rfc3339();

        let project = DbProject {
            id: "widget-v2".to_string(),
            name: "Widget v2".to_string(),
            status: "active".to_string(),
            milestone: Some("Beta Launch".to_string()),
            owner: Some("Alice".to_string()),
            target_date: Some("2026-06-01".to_string()),
            tracker_path: Some("Projects/Widget v2".to_string()),
            updated_at: now,
            archived: false,
            keywords: None,
            keywords_extracted_at: None,
            metadata: None,
        };

        db.upsert_project(&project).expect("upsert");

        let fetched = db.get_project("widget-v2").expect("get").unwrap();
        assert_eq!(fetched.name, "Widget v2");
        assert_eq!(fetched.status, "active");
        assert_eq!(fetched.milestone, Some("Beta Launch".to_string()));
    }

    #[test]
    fn test_get_project_by_name() {
        let db = test_db();
        let now = Utc::now().to_rfc3339();

        let project = DbProject {
            id: "gadget".to_string(),
            name: "Gadget".to_string(),
            status: "active".to_string(),
            milestone: None,
            owner: None,
            target_date: None,
            tracker_path: None,
            updated_at: now,
            archived: false,
            keywords: None,
            keywords_extracted_at: None,
            metadata: None,
        };

        db.upsert_project(&project).expect("upsert");

        let fetched = db.get_project_by_name("gadget").expect("get").unwrap();
        assert_eq!(fetched.id, "gadget");

        // Case-insensitive
        let fetched = db.get_project_by_name("GADGET").expect("get").unwrap();
        assert_eq!(fetched.id, "gadget");
    }

    #[test]
    fn test_get_all_projects() {
        let db = test_db();
        let now = Utc::now().to_rfc3339();

        for name in &["Alpha", "Beta", "Gamma"] {
            let project = DbProject {
                id: name.to_lowercase(),
                name: name.to_string(),
                status: "active".to_string(),
                milestone: None,
                owner: None,
                target_date: None,
                tracker_path: None,
                updated_at: now.clone(),
                archived: false,
                keywords: None,
                keywords_extracted_at: None,
                metadata: None,
            };
            db.upsert_project(&project).expect("upsert");
        }

        let all = db.get_all_projects().expect("get all");
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].name, "Alpha"); // Sorted by name
    }

    #[test]
    fn test_update_project_field() {
        let db = test_db();
        let now = Utc::now().to_rfc3339();

        let project = DbProject {
            id: "proj-1".to_string(),
            name: "Proj 1".to_string(),
            status: "active".to_string(),
            milestone: None,
            owner: None,
            target_date: None,
            tracker_path: None,
            updated_at: now,
            archived: false,
            keywords: None,
            keywords_extracted_at: None,
            metadata: None,
        };
        db.upsert_project(&project).expect("upsert");

        db.update_project_field("proj-1", "status", "on_hold")
            .expect("update");

        let fetched = db.get_project("proj-1").expect("get").unwrap();
        assert_eq!(fetched.status, "on_hold");
    }

    #[test]
    fn test_update_project_field_rejects_invalid() {
        let db = test_db();
        let now = Utc::now().to_rfc3339();

        let project = DbProject {
            id: "proj-1".to_string(),
            name: "Proj 1".to_string(),
            status: "active".to_string(),
            milestone: None,
            owner: None,
            target_date: None,
            tracker_path: None,
            updated_at: now,
            archived: false,
            keywords: None,
            keywords_extracted_at: None,
            metadata: None,
        };
        db.upsert_project(&project).expect("upsert");

        let result = db.update_project_field("proj-1", "id", "Hacked");
        assert!(result.is_err());
    }

    #[test]
    fn test_ensure_entity_for_project() {
        let db = test_db();
        let now = Utc::now().to_rfc3339();

        let project = DbProject {
            id: "widget-v2".to_string(),
            name: "Widget v2".to_string(),
            status: "active".to_string(),
            milestone: None,
            owner: None,
            target_date: None,
            tracker_path: Some("Projects/Widget v2".to_string()),
            updated_at: now,
            archived: false,
            keywords: None,
            keywords_extracted_at: None,
            metadata: None,
        };

        db.upsert_project(&project).expect("upsert");

        let entity = db.get_entity("widget-v2").expect("get").unwrap();
        assert_eq!(entity.name, "Widget v2");
        assert_eq!(entity.entity_type, EntityType::Project);
    }

    #[test]
    fn test_get_project_actions() {
        let db = test_db();
        let now = Utc::now().to_rfc3339();

        let project = DbProject {
            id: "proj-actions".to_string(),
            name: "Action Test".to_string(),
            status: "active".to_string(),
            milestone: None,
            owner: None,
            target_date: None,
            tracker_path: None,
            updated_at: now.clone(),
            archived: false,
            keywords: None,
            keywords_extracted_at: None,
            metadata: None,
        };
        db.upsert_project(&project).expect("upsert");

        // Insert an action linked to project
        let action = DbAction {
            id: "act-proj-1".to_string(),
            title: "Fix the widget".to_string(),
            priority: "P1".to_string(),
            status: "pending".to_string(),
            created_at: now.clone(),
            due_date: None,
            completed_at: None,
            account_id: None,
            project_id: Some("proj-actions".to_string()),
            source_type: None,
            source_id: None,
            source_label: None,
            context: None,
            waiting_on: None,
            updated_at: now,
            person_id: None,
            account_name: None,
            next_meeting_title: None,
            next_meeting_start: None,
        };
        db.upsert_action(&action).expect("upsert action");

        let actions = db.get_project_actions("proj-actions").expect("get");
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].title, "Fix the widget");
    }

    #[test]
    fn test_project_signals_empty() {
        let db = test_db();
        let signals = db.get_project_signals("nonexistent").expect("signals");
        assert_eq!(signals.meeting_frequency_30d, 0);
        assert_eq!(signals.meeting_frequency_90d, 0);
        assert_eq!(signals.open_action_count, 0);
        assert_eq!(signals.temperature, "cold");
    }

    #[test]
    fn test_link_meeting_to_project_and_query() {
        let db = test_db();
        let now = Utc::now().to_rfc3339();

        let project = DbProject {
            id: "proj-mtg".to_string(),
            name: "Meeting Project".to_string(),
            status: "active".to_string(),
            milestone: None,
            owner: None,
            target_date: None,
            tracker_path: None,
            updated_at: now.clone(),
            archived: false,
            keywords: None,
            keywords_extracted_at: None,
            metadata: None,
        };
        db.upsert_project(&project).expect("upsert project");

        // Insert a meeting directly
        db.conn
            .execute(
                "INSERT INTO meetings_history (id, title, meeting_type, start_time, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params!["mtg-proj-001", "Sprint Review", "internal", &now, &now],
            )
            .expect("insert meeting");

        // Link it
        db.link_meeting_to_project("mtg-proj-001", "proj-mtg")
            .expect("link");

        // Query via project
        let meetings = db
            .get_meetings_for_project("proj-mtg", 10)
            .expect("get meetings");
        assert_eq!(meetings.len(), 1);
        assert_eq!(meetings[0].title, "Sprint Review");

        // Idempotent
        db.link_meeting_to_project("mtg-proj-001", "proj-mtg")
            .expect("re-link should not fail");
        let meetings = db
            .get_meetings_for_project("proj-mtg", 10)
            .expect("still 1");
        assert_eq!(meetings.len(), 1);
    }

    // =========================================================================
    // I52: Meeting-entity M2M junction tests
    // =========================================================================

    #[test]
    fn test_generic_link_unlink_meeting_entity() {
        let db = test_db();
        let now = Utc::now().to_rfc3339();

        // Create an entity (account)
        db.conn
            .execute(
                "INSERT INTO entities (id, name, entity_type, updated_at) VALUES (?1, ?2, ?3, ?4)",
                params!["acme-ent", "Acme", "account", &now],
            )
            .expect("insert entity");

        db.conn
            .execute(
                "INSERT INTO meetings_history (id, title, meeting_type, start_time, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params!["mtg-j1", "Acme QBR", "customer", &now, &now],
            )
            .expect("insert meeting");

        // Link
        db.link_meeting_entity("mtg-j1", "acme-ent", "account")
            .expect("link");
        let entities = db.get_meeting_entities("mtg-j1").expect("get entities");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "Acme");

        // Unlink
        db.unlink_meeting_entity("mtg-j1", "acme-ent")
            .expect("unlink");
        let entities = db.get_meeting_entities("mtg-j1").expect("empty now");
        assert_eq!(entities.len(), 0);
    }

    #[test]
    fn test_meeting_multi_entity_link() {
        let db = test_db();
        let now = Utc::now().to_rfc3339();

        // Create an account entity and a project entity
        db.conn
            .execute(
                "INSERT INTO entities (id, name, entity_type, updated_at) VALUES (?1, ?2, ?3, ?4)",
                params!["acme-m2m", "Acme", "account", &now],
            )
            .expect("insert account entity");

        let project = DbProject {
            id: "proj-m2m".to_string(),
            name: "Migration".to_string(),
            status: "active".to_string(),
            milestone: None,
            owner: None,
            target_date: None,
            tracker_path: None,
            updated_at: now.clone(),
            archived: false,
            keywords: None,
            keywords_extracted_at: None,
            metadata: None,
        };
        db.upsert_project(&project).expect("upsert project");

        db.conn
            .execute(
                "INSERT INTO meetings_history (id, title, meeting_type, start_time, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params!["mtg-m2m", "Migration Review", "customer", &now, &now],
            )
            .expect("insert meeting");

        // Link to both account and project
        db.link_meeting_entity("mtg-m2m", "acme-m2m", "account")
            .expect("link account");
        db.link_meeting_entity("mtg-m2m", "proj-m2m", "project")
            .expect("link project");

        let entities = db.get_meeting_entities("mtg-m2m").expect("get entities");
        assert_eq!(entities.len(), 2);

        // Generic get_meetings_for_entity works for both
        let acme_meetings = db
            .get_meetings_for_entity("acme-m2m", 10)
            .expect("acme meetings");
        assert_eq!(acme_meetings.len(), 1);
        let proj_meetings = db
            .get_meetings_for_entity("proj-m2m", 10)
            .expect("proj meetings");
        assert_eq!(proj_meetings.len(), 1);
    }

    #[test]
    fn test_link_meeting_entity_manual() {
        let db = test_db();
        let now = Utc::now().to_rfc3339();

        let meeting = DbMeeting {
            id: "mtg-link".to_string(),
            title: "Link Test".to_string(),
            meeting_type: "customer".to_string(),
            start_time: now.clone(),
            end_time: None,
            attendees: None,
            notes_path: None,
            summary: None,
            created_at: now.clone(),
            calendar_event_id: None,
            description: None,
            prep_context_json: None,
            user_agenda_json: None,
            user_notes: None,
            prep_frozen_json: None,
            prep_frozen_at: None,
            prep_snapshot_path: None,
            prep_snapshot_hash: None,
            transcript_path: None,
            transcript_processed_at: None,
            intelligence_state: None,
            intelligence_quality: None,
            last_enriched_at: None,
            signal_count: None,
            has_new_signals: None,
            last_viewed_at: None,
        };
        db.upsert_meeting(&meeting).expect("upsert");
        db.link_meeting_entity("mtg-link", "acme-auto", "account")
            .expect("link");

        // Junction should contain the link
        let count: i32 = db
            .conn
            .query_row(
                "SELECT COUNT(*) FROM meeting_entities WHERE meeting_id = ?1 AND entity_id = ?2",
                params!["mtg-link", "acme-auto"],
                |row| row.get(0),
            )
            .expect("count");
        assert_eq!(count, 1);
    }

    #[test]
    fn test_captures_with_project_id() {
        let db = test_db();

        db.insert_capture_with_project(
            "mtg-p1",
            "Sprint Review",
            None,
            Some("proj-cap"),
            "win",
            "Feature shipped",
        )
        .expect("insert");

        let captures = db.get_captures_for_project("proj-cap", 30).expect("query");
        assert_eq!(captures.len(), 1);
        assert_eq!(captures[0].project_id.as_deref(), Some("proj-cap"));
        assert_eq!(captures[0].content, "Feature shipped");

        // Regular insert_capture still works (project_id = None)
        db.insert_capture("mtg-p2", "Acme QBR", Some("acme"), "risk", "Budget freeze")
            .expect("insert without project");
        let proj_captures = db.get_captures_for_project("acme", 30).expect("query");
        assert_eq!(proj_captures.len(), 0); // acme is account_id, not project_id
    }

    // =========================================================================
    // I124: Content Index tests
    // =========================================================================

    #[test]
    fn test_upsert_and_get_content_files() {
        let db = test_db();
        let now = Utc::now().to_rfc3339();

        let file = DbContentFile {
            id: "acme/notes-md".to_string(),
            entity_id: "acme".to_string(),
            entity_type: "account".to_string(),
            filename: "notes.md".to_string(),
            relative_path: "Accounts/Acme/notes.md".to_string(),
            absolute_path: "/tmp/workspace/Accounts/Acme/notes.md".to_string(),
            format: "Markdown".to_string(),
            file_size: 1234,
            modified_at: now.clone(),
            indexed_at: now.clone(),
            extracted_at: None,
            summary: None,
            embeddings_generated_at: None,
            content_type: "notes".to_string(),
            priority: 7,
        };

        db.upsert_content_file(&file).unwrap();

        let files = db.get_entity_files("acme").unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].filename, "notes.md");
        assert_eq!(files[0].file_size, 1234);
        assert_eq!(files[0].format, "Markdown");
    }

    #[test]
    fn test_delete_content_file() {
        let db = test_db();
        let now = Utc::now().to_rfc3339();

        let file = DbContentFile {
            id: "beta/report-pdf".to_string(),
            entity_id: "beta".to_string(),
            entity_type: "account".to_string(),
            filename: "report.pdf".to_string(),
            relative_path: "Accounts/Beta/report.pdf".to_string(),
            absolute_path: "/tmp/workspace/Accounts/Beta/report.pdf".to_string(),
            format: "Pdf".to_string(),
            file_size: 50000,
            modified_at: now.clone(),
            indexed_at: now.clone(),
            extracted_at: None,
            summary: None,
            embeddings_generated_at: None,
            content_type: "general".to_string(),
            priority: 5,
        };

        db.upsert_content_file(&file).unwrap();
        assert_eq!(db.get_entity_files("beta").unwrap().len(), 1);

        db.delete_content_file("beta/report-pdf").unwrap();
        assert_eq!(db.get_entity_files("beta").unwrap().len(), 0);
    }

    #[test]
    fn test_coalesce_preserves_extraction() {
        let db = test_db();
        let now = Utc::now().to_rfc3339();

        // Insert with extraction data
        let file = DbContentFile {
            id: "gamma/doc-md".to_string(),
            entity_id: "gamma".to_string(),
            entity_type: "account".to_string(),
            filename: "doc.md".to_string(),
            relative_path: "Accounts/Gamma/doc.md".to_string(),
            absolute_path: "/tmp/workspace/Accounts/Gamma/doc.md".to_string(),
            format: "Markdown".to_string(),
            file_size: 500,
            modified_at: now.clone(),
            indexed_at: now.clone(),
            extracted_at: Some(now.clone()),
            summary: Some("Important document about things.".to_string()),
            embeddings_generated_at: Some(now.clone()),
            content_type: "general".to_string(),
            priority: 5,
        };
        db.upsert_content_file(&file).unwrap();

        // Upsert again without extraction data (simulating a re-scan)
        let file_rescan = DbContentFile {
            id: "gamma/doc-md".to_string(),
            entity_id: "gamma".to_string(),
            entity_type: "account".to_string(),
            filename: "doc.md".to_string(),
            relative_path: "Accounts/Gamma/doc.md".to_string(),
            absolute_path: "/tmp/workspace/Accounts/Gamma/doc.md".to_string(),
            format: "Markdown".to_string(),
            file_size: 600, // size changed
            modified_at: now.clone(),
            indexed_at: now.clone(),
            extracted_at: None, // Not re-extracted
            summary: None,      // Not re-extracted
            embeddings_generated_at: None,
            content_type: "general".to_string(),
            priority: 5,
        };
        db.upsert_content_file(&file_rescan).unwrap();

        // Extraction data should be preserved via COALESCE
        let files = db.get_entity_files("gamma").unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].file_size, 600); // Size updated
        assert!(files[0].extracted_at.is_some()); // Preserved
        assert_eq!(
            files[0].summary.as_deref(),
            Some("Important document about things.")
        ); // Preserved
    }

    #[test]
    fn test_get_files_needing_embeddings() {
        let db = test_db();
        let now = Utc::now().to_rfc3339();

        let mut ready_file = DbContentFile {
            id: "emb/ready".to_string(),
            entity_id: "emb".to_string(),
            entity_type: "account".to_string(),
            filename: "ready.md".to_string(),
            relative_path: "Accounts/Emb/ready.md".to_string(),
            absolute_path: "/tmp/workspace/Accounts/Emb/ready.md".to_string(),
            format: "Markdown".to_string(),
            file_size: 100,
            modified_at: now.clone(),
            indexed_at: now.clone(),
            extracted_at: None,
            summary: None,
            embeddings_generated_at: Some(now.clone()),
            content_type: "general".to_string(),
            priority: 5,
        };
        db.upsert_content_file(&ready_file).unwrap();

        ready_file.id = "emb/stale".to_string();
        ready_file.filename = "stale.md".to_string();
        ready_file.relative_path = "Accounts/Emb/stale.md".to_string();
        ready_file.absolute_path = "/tmp/workspace/Accounts/Emb/stale.md".to_string();
        ready_file.embeddings_generated_at = None;
        db.upsert_content_file(&ready_file).unwrap();

        let needing = db.get_files_needing_embeddings(10).unwrap();
        assert_eq!(needing.len(), 1);
        assert_eq!(needing[0].id, "emb/stale");
    }

    #[test]
    fn test_replace_content_embeddings_for_file() {
        let db = test_db();
        let now = Utc::now().to_rfc3339();

        let file = DbContentFile {
            id: "emb/file".to_string(),
            entity_id: "emb".to_string(),
            entity_type: "account".to_string(),
            filename: "file.md".to_string(),
            relative_path: "Accounts/Emb/file.md".to_string(),
            absolute_path: "/tmp/workspace/Accounts/Emb/file.md".to_string(),
            format: "Markdown".to_string(),
            file_size: 100,
            modified_at: now.clone(),
            indexed_at: now.clone(),
            extracted_at: None,
            summary: None,
            embeddings_generated_at: None,
            content_type: "general".to_string(),
            priority: 5,
        };
        db.upsert_content_file(&file).unwrap();

        let chunk = DbContentEmbedding {
            id: "chunk-1".to_string(),
            content_file_id: file.id.clone(),
            chunk_index: 0,
            chunk_text: "hello world".to_string(),
            embedding: vec![0, 1, 2, 3],
            created_at: now.clone(),
        };
        db.replace_content_embeddings_for_file(&file.id, &[chunk])
            .unwrap();
        db.set_embeddings_generated_at(&file.id, Some(&now))
            .unwrap();

        let chunks = db.get_entity_embedding_chunks("emb").unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].chunk_text, "hello world");
    }

    #[test]
    fn test_chat_session_roundtrip() {
        let db = test_db();
        let now = Utc::now().to_rfc3339();

        let session = db
            .create_chat_session("sess-1", Some("acme"), Some("account"), &now, &now)
            .unwrap();
        assert_eq!(session.id, "sess-1");

        let open = db
            .get_open_chat_session(Some("acme"), Some("account"))
            .unwrap()
            .unwrap();
        assert_eq!(open.id, "sess-1");

        let idx = db.get_next_chat_turn_index("sess-1").unwrap();
        assert_eq!(idx, 0);

        db.append_chat_turn("turn-1", "sess-1", idx, "user", "hi", &now)
            .unwrap();
        db.bump_chat_session_stats("sess-1", 1, Some("hi")).unwrap();

        let turns = db.get_chat_session_turns("sess-1", 10).unwrap();
        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].content, "hi");
    }

    // === I127/I128: Manual action creation & editing tests ===

    #[test]
    fn test_create_action_all_fields() {
        let db = test_db();
        let now = Utc::now().to_rfc3339();

        let action = DbAction {
            id: "manual-001".to_string(),
            title: "Call Jane about renewal".to_string(),
            priority: "P1".to_string(),
            status: "pending".to_string(),
            created_at: now.clone(),
            due_date: Some("2026-02-15".to_string()),
            completed_at: None,
            account_id: Some("acme-corp".to_string()),
            project_id: Some("proj-q1".to_string()),
            source_type: Some("manual".to_string()),
            source_id: None,
            source_label: Some("Slack #cs-team".to_string()),
            context: Some("Jane mentioned churn risk in standup".to_string()),
            waiting_on: None,
            updated_at: now.clone(),
            person_id: Some("person-jane".to_string()),
            account_name: None,
            next_meeting_title: None,
            next_meeting_start: None,
        };
        db.upsert_action(&action).unwrap();

        let fetched = db.get_action_by_id("manual-001").unwrap().unwrap();
        assert_eq!(fetched.title, "Call Jane about renewal");
        assert_eq!(fetched.priority, "P1");
        assert_eq!(fetched.status, "pending");
        assert_eq!(fetched.due_date.as_deref(), Some("2026-02-15"));
        assert_eq!(fetched.account_id.as_deref(), Some("acme-corp"));
        assert_eq!(fetched.project_id.as_deref(), Some("proj-q1"));
        assert_eq!(fetched.source_type.as_deref(), Some("manual"));
        assert_eq!(fetched.source_label.as_deref(), Some("Slack #cs-team"));
        assert_eq!(
            fetched.context.as_deref(),
            Some("Jane mentioned churn risk in standup")
        );
        assert_eq!(fetched.person_id.as_deref(), Some("person-jane"));
    }

    #[test]
    fn test_create_action_defaults() {
        let db = test_db();

        // Simulate creating with title only — mirroring the create_action command defaults
        let now = Utc::now().to_rfc3339();
        let action = DbAction {
            id: "manual-002".to_string(),
            title: "Quick follow-up".to_string(),
            priority: "P2".to_string(),
            status: "pending".to_string(),
            created_at: now.clone(),
            due_date: None,
            completed_at: None,
            account_id: None,
            project_id: None,
            source_type: Some("manual".to_string()),
            source_id: None,
            source_label: None,
            context: None,
            waiting_on: None,
            updated_at: now,
            person_id: None,
            account_name: None,
            next_meeting_title: None,
            next_meeting_start: None,
        };
        db.upsert_action(&action).unwrap();

        let fetched = db.get_action_by_id("manual-002").unwrap().unwrap();
        assert_eq!(fetched.priority, "P2");
        assert_eq!(fetched.status, "pending");
        assert_eq!(fetched.source_type.as_deref(), Some("manual"));
        assert!(fetched.due_date.is_none());
        assert!(fetched.account_id.is_none());
        assert!(fetched.person_id.is_none());
        assert!(fetched.context.is_none());
    }

    #[test]
    fn test_update_action_fields() {
        let db = test_db();

        // Create initial action
        let action = sample_action("update-001", "Original title");
        db.upsert_action(&action).unwrap();

        // Update specific fields (mirroring update_action command logic)
        let mut updated = db.get_action_by_id("update-001").unwrap().unwrap();
        updated.title = "Updated title".to_string();
        updated.due_date = Some("2026-03-01".to_string());
        updated.context = Some("New context added".to_string());
        updated.account_id = Some("acme".to_string());
        updated.person_id = Some("person-bob".to_string());
        updated.updated_at = Utc::now().to_rfc3339();
        db.upsert_action(&updated).unwrap();

        // Verify updates applied and other fields preserved
        let fetched = db.get_action_by_id("update-001").unwrap().unwrap();
        assert_eq!(fetched.title, "Updated title");
        assert_eq!(fetched.due_date.as_deref(), Some("2026-03-01"));
        assert_eq!(fetched.context.as_deref(), Some("New context added"));
        assert_eq!(fetched.account_id.as_deref(), Some("acme"));
        assert_eq!(fetched.person_id.as_deref(), Some("person-bob"));
        // Unchanged fields preserved
        assert_eq!(fetched.priority, "P2");
        assert_eq!(fetched.status, "pending");
    }

    #[test]
    fn test_update_action_clear_fields() {
        let db = test_db();

        // Create action with fields populated
        let now = Utc::now().to_rfc3339();
        let action = DbAction {
            id: "clear-001".to_string(),
            title: "Action with fields".to_string(),
            priority: "P1".to_string(),
            status: "pending".to_string(),
            created_at: now.clone(),
            due_date: Some("2026-02-20".to_string()),
            completed_at: None,
            account_id: Some("acme".to_string()),
            project_id: Some("proj-1".to_string()),
            source_type: Some("manual".to_string()),
            source_id: None,
            source_label: Some("Call".to_string()),
            context: Some("Some context".to_string()),
            waiting_on: None,
            updated_at: now,
            person_id: Some("person-alice".to_string()),
            account_name: None,
            next_meeting_title: None,
            next_meeting_start: None,
        };
        db.upsert_action(&action).unwrap();

        // Clear specific fields (mirroring clear_* flags in update_action command)
        let mut cleared = db.get_action_by_id("clear-001").unwrap().unwrap();
        cleared.due_date = None;
        cleared.account_id = None;
        cleared.person_id = None;
        cleared.updated_at = Utc::now().to_rfc3339();
        db.upsert_action(&cleared).unwrap();

        let fetched = db.get_action_by_id("clear-001").unwrap().unwrap();
        assert!(fetched.due_date.is_none(), "due_date should be cleared");
        assert!(fetched.account_id.is_none(), "account_id should be cleared");
        assert!(fetched.person_id.is_none(), "person_id should be cleared");
        // Non-cleared fields preserved
        assert_eq!(fetched.title, "Action with fields");
        assert_eq!(fetched.priority, "P1");
        assert_eq!(fetched.context.as_deref(), Some("Some context"));
        assert_eq!(fetched.source_label.as_deref(), Some("Call"));
        assert_eq!(fetched.project_id.as_deref(), Some("proj-1"));
    }

    #[test]
    fn test_person_id_column() {
        let db = test_db();

        // Insert without person_id
        let action = sample_action("pid-001", "No person");
        db.upsert_action(&action).unwrap();

        let fetched = db.get_action_by_id("pid-001").unwrap().unwrap();
        assert!(fetched.person_id.is_none());

        // Update to add person_id
        let mut with_person = fetched;
        with_person.person_id = Some("person-charlie".to_string());
        with_person.updated_at = Utc::now().to_rfc3339();
        db.upsert_action(&with_person).unwrap();

        let fetched2 = db.get_action_by_id("pid-001").unwrap().unwrap();
        assert_eq!(fetched2.person_id.as_deref(), Some("person-charlie"));

        // Verify person_id appears in get_due_actions results too
        let due = db.get_due_actions(90).unwrap();
        let found = due.iter().find(|a| a.id == "pid-001").unwrap();
        assert_eq!(found.person_id.as_deref(), Some("person-charlie"));

        // Clear person_id
        let mut cleared = fetched2;
        cleared.person_id = None;
        cleared.updated_at = Utc::now().to_rfc3339();
        db.upsert_action(&cleared).unwrap();

        let fetched3 = db.get_action_by_id("pid-001").unwrap().unwrap();
        assert!(fetched3.person_id.is_none());
    }

    #[test]
    fn test_manual_actions_in_non_briefing_query() {
        let db = test_db();

        // Manual action should appear in get_non_briefing_pending_actions
        let now = Utc::now().to_rfc3339();
        let action = DbAction {
            id: "manual-nbp".to_string(),
            title: "Manual task".to_string(),
            priority: "P2".to_string(),
            status: "pending".to_string(),
            created_at: now.clone(),
            due_date: None,
            completed_at: None,
            account_id: None,
            project_id: None,
            source_type: Some("manual".to_string()),
            source_id: None,
            source_label: None,
            context: None,
            waiting_on: None,
            updated_at: now,
            person_id: None,
            account_name: None,
            next_meeting_title: None,
            next_meeting_start: None,
        };
        db.upsert_action(&action).unwrap();

        let non_briefing = db.get_non_briefing_pending_actions().unwrap();
        let found = non_briefing.iter().find(|a| a.id == "manual-nbp");
        assert!(
            found.is_some(),
            "Manual actions should appear in non-briefing pending query"
        );
    }

    #[test]
    fn test_get_latest_processing_status() {
        let db = test_db();

        // Insert two entries for the same file — only latest should be returned
        let entry1 = DbProcessingLog {
            id: "log-1".to_string(),
            filename: "report.pdf".to_string(),
            source_path: "/inbox/report.pdf".to_string(),
            destination_path: None,
            classification: "document".to_string(),
            status: "error".to_string(),
            processed_at: None,
            error_message: Some("parse failed".to_string()),
            created_at: "2025-01-01T00:00:00Z".to_string(),
        };
        db.insert_processing_log(&entry1).unwrap();

        let entry2 = DbProcessingLog {
            id: "log-2".to_string(),
            filename: "report.pdf".to_string(),
            source_path: "/inbox/report.pdf".to_string(),
            destination_path: Some("/accounts/acme/report.pdf".to_string()),
            classification: "document".to_string(),
            status: "completed".to_string(),
            processed_at: Some("2025-01-02T00:00:00Z".to_string()),
            error_message: None,
            created_at: "2025-01-02T00:00:00Z".to_string(),
        };
        db.insert_processing_log(&entry2).unwrap();

        // Insert a separate file with error status
        let entry3 = DbProcessingLog {
            id: "log-3".to_string(),
            filename: "notes.md".to_string(),
            source_path: "/inbox/notes.md".to_string(),
            destination_path: None,
            classification: "meeting".to_string(),
            status: "error".to_string(),
            processed_at: None,
            error_message: Some("AI enrichment timed out".to_string()),
            created_at: "2025-01-03T00:00:00Z".to_string(),
        };
        db.insert_processing_log(&entry3).unwrap();

        let map = db.get_latest_processing_status().unwrap();

        // Should have exactly 2 filenames
        assert_eq!(map.len(), 2);

        // report.pdf should show the LATEST entry (completed, no error)
        let (status, error) = map.get("report.pdf").expect("report.pdf should be in map");
        assert_eq!(status, "completed");
        assert!(error.is_none());

        // notes.md should show error with message
        let (status, error) = map.get("notes.md").expect("notes.md should be in map");
        assert_eq!(status, "error");
        assert_eq!(error.as_deref(), Some("AI enrichment timed out"));
    }

    // =========================================================================
    // cascade_meeting_entity_to_people (I184)
    // =========================================================================

    /// Helper: create an account and ensure its entity row exists.
    fn setup_account(db: &ActionDb, id: &str, name: &str) {
        let now = Utc::now().to_rfc3339();
        let account = DbAccount {
            id: id.to_string(),
            name: name.to_string(),
            lifecycle: None,
            arr: None,
            health: None,
            contract_start: None,
            contract_end: None,
            nps: None,
            tracker_path: None,
            parent_id: None,
            is_internal: false,
            updated_at: now,
            archived: false,
            keywords: None,
            keywords_extracted_at: None,
            metadata: None,
        };
        db.upsert_account(&account).expect("upsert account");
        db.ensure_entity_for_account(&account)
            .expect("ensure entity");
    }

    /// Helper: create a meeting.
    fn setup_meeting(db: &ActionDb, id: &str, title: &str) {
        let now = Utc::now().to_rfc3339();
        let meeting = DbMeeting {
            id: id.to_string(),
            title: title.to_string(),
            meeting_type: "external".to_string(),
            start_time: now.clone(),
            end_time: None,
            attendees: None,
            notes_path: None,
            summary: None,
            created_at: now,
            calendar_event_id: None,
            description: None,
            prep_context_json: None,
            user_agenda_json: None,
            user_notes: None,
            prep_frozen_json: None,
            prep_frozen_at: None,
            prep_snapshot_path: None,
            prep_snapshot_hash: None,
            transcript_path: None,
            transcript_processed_at: None,
            intelligence_state: None,
            intelligence_quality: None,
            last_enriched_at: None,
            signal_count: None,
            has_new_signals: None,
            last_viewed_at: None,
        };
        db.upsert_meeting(&meeting).expect("upsert meeting");
    }

    #[test]
    fn test_cascade_meeting_entity_to_people_external_only() {
        let db = test_db();
        setup_account(&db, "acc1", "Acme Corp");
        setup_meeting(&db, "m1", "Acme QBR");

        // External person → should be linked
        let mut external = sample_person("jane@acme.com");
        external.relationship = "external".to_string();
        db.upsert_person(&external).expect("upsert external");
        db.record_meeting_attendance("m1", &external.id)
            .expect("attend");

        // Internal person → should NOT be linked
        let mut internal = sample_person("john@mycompany.com");
        internal.relationship = "internal".to_string();
        db.upsert_person(&internal).expect("upsert internal");
        db.record_meeting_attendance("m1", &internal.id)
            .expect("attend");

        let linked = db
            .cascade_meeting_entity_to_people("m1", Some("acc1"), None)
            .expect("cascade");
        assert_eq!(linked, 1);

        // External person linked to account
        let entities = db
            .get_entities_for_person(&external.id)
            .expect("entities for external");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].id, "acc1");

        // Internal person NOT linked
        let entities = db
            .get_entities_for_person(&internal.id)
            .expect("entities for internal");
        assert_eq!(entities.len(), 0);
    }

    #[test]
    fn test_cascade_meeting_entity_to_people_idempotent() {
        let db = test_db();
        setup_account(&db, "acc1", "Acme Corp");
        setup_meeting(&db, "m1", "Acme QBR");

        let mut person = sample_person("jane@acme.com");
        person.relationship = "external".to_string();
        db.upsert_person(&person).expect("upsert");
        db.record_meeting_attendance("m1", &person.id)
            .expect("attend");

        // Manually link person first
        db.link_person_to_entity(&person.id, "acc1", "associated")
            .expect("manual link");

        // Cascade should detect existing link, return 0 new
        let linked = db
            .cascade_meeting_entity_to_people("m1", Some("acc1"), None)
            .expect("cascade");
        assert_eq!(linked, 0);

        // Still only one link
        let entities = db.get_entities_for_person(&person.id).expect("entities");
        assert_eq!(entities.len(), 1);
    }

    #[test]
    fn test_cascade_meeting_entity_to_people_no_entity() {
        let db = test_db();
        setup_meeting(&db, "m1", "Internal Sync");

        let mut person = sample_person("someone@test.com");
        person.relationship = "external".to_string();
        db.upsert_person(&person).expect("upsert");
        db.record_meeting_attendance("m1", &person.id)
            .expect("attend");

        // Cascade with no entity → 0 links
        let linked = db
            .cascade_meeting_entity_to_people("m1", None, None)
            .expect("cascade");
        assert_eq!(linked, 0);
    }

    // =========================================================================
    // Domain reclassification tests (I184)
    // =========================================================================

    #[test]
    fn test_reclassify_people_for_domains() {
        let db = test_db();

        // Create two people: one external, one unknown
        let mut p1 = sample_person("alice@subsidiary.com");
        p1.relationship = "external".to_string();
        db.upsert_person(&p1).expect("upsert");

        let mut p2 = sample_person("bob@vendor.com");
        p2.relationship = "external".to_string();
        db.upsert_person(&p2).expect("upsert");

        // Add subsidiary.com as internal domain
        let domains = vec!["myco.com".to_string(), "subsidiary.com".to_string()];
        let changed = db
            .reclassify_people_for_domains(&domains)
            .expect("reclassify");

        // alice should flip to internal, bob stays external
        assert_eq!(changed, 1);

        let alice = db.get_person(&p1.id).expect("get").unwrap();
        assert_eq!(alice.relationship, "internal");

        let bob = db.get_person(&p2.id).expect("get").unwrap();
        assert_eq!(bob.relationship, "external");
    }

    #[test]
    fn test_reclassify_meeting_types_from_attendees() {
        let db = test_db();
        setup_meeting(&db, "m1", "Subsidiary Sync");

        // Create person who is currently external
        let mut p1 = sample_person("alice@subsidiary.com");
        p1.relationship = "external".to_string();
        db.upsert_person(&p1).expect("upsert");
        db.record_meeting_attendance("m1", &p1.id).expect("attend");

        // Meeting was classified as 'customer' because alice was external
        db.conn
            .execute(
                "UPDATE meetings_history SET meeting_type = 'customer' WHERE id = 'm1'",
                [],
            )
            .expect("set type");

        // Now reclassify alice as internal
        let domains = vec!["myco.com".to_string(), "subsidiary.com".to_string()];
        db.reclassify_people_for_domains(&domains)
            .expect("reclassify people");

        // Reclassify meetings
        let changed = db
            .reclassify_meeting_types_from_attendees()
            .expect("reclassify meetings");
        assert_eq!(changed, 1);

        let meeting: String = db
            .conn
            .query_row(
                "SELECT meeting_type FROM meetings_history WHERE id = 'm1'",
                [],
                |row| row.get(0),
            )
            .expect("query");
        assert_eq!(meeting, "internal");
    }

    #[test]
    fn test_reclassify_preserves_title_based_types() {
        let db = test_db();
        setup_meeting(&db, "m1", "All Hands");

        // Even with attendee changes, all_hands should not be touched
        db.conn
            .execute(
                "UPDATE meetings_history SET meeting_type = 'all_hands' WHERE id = 'm1'",
                [],
            )
            .expect("set type");

        let changed = db
            .reclassify_meeting_types_from_attendees()
            .expect("reclassify");
        assert_eq!(changed, 0);

        let meeting_type: String = db
            .conn
            .query_row(
                "SELECT meeting_type FROM meetings_history WHERE id = 'm1'",
                [],
                |row| row.get(0),
            )
            .expect("query");
        assert_eq!(meeting_type, "all_hands");
    }

    fn sample_account(id: &str, name: &str) -> DbAccount {
        DbAccount {
            id: id.to_string(),
            name: name.to_string(),
            lifecycle: None,
            arr: None,
            health: None,
            contract_start: None,
            contract_end: None,
            nps: None,
            tracker_path: None,
            parent_id: None,
            is_internal: false,
            updated_at: Utc::now().to_rfc3339(),
            archived: false,
            keywords: None,
            keywords_extracted_at: None,
            metadata: None,
        }
    }

    #[test]
    fn test_get_all_accounts_with_domains_single_query() {
        let db = test_db();

        let acct = sample_account("acme", "Acme Corp");
        db.upsert_account(&acct).unwrap();
        db.set_account_domains("acme", &["acme.com".to_string(), "acme.io".to_string()])
            .unwrap();

        let acct2 = sample_account("globex", "Globex Inc");
        db.upsert_account(&acct2).unwrap();
        db.set_account_domains("globex", &["globex.com".to_string()])
            .unwrap();

        let results = db.get_all_accounts_with_domains(false).unwrap();
        assert_eq!(results.len(), 2);

        // Find acme
        let acme = results.iter().find(|(a, _)| a.id == "acme").unwrap();
        assert_eq!(acme.0.name, "Acme Corp");
        assert_eq!(acme.1.len(), 2);
        assert!(acme.1.contains(&"acme.com".to_string()));
        assert!(acme.1.contains(&"acme.io".to_string()));

        // Find globex
        let globex = results.iter().find(|(a, _)| a.id == "globex").unwrap();
        assert_eq!(globex.1.len(), 1);
        assert_eq!(globex.1[0], "globex.com");
    }

    #[test]
    fn test_get_all_accounts_with_domains_no_domains() {
        let db = test_db();

        let acct = sample_account("solo", "Solo Corp");
        db.upsert_account(&acct).unwrap();

        let results = db.get_all_accounts_with_domains(false).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0.id, "solo");
        assert!(results[0].1.is_empty());
    }

    #[test]
    fn test_get_all_accounts_with_domains_filters_archived() {
        let db = test_db();

        let active = sample_account("active", "Active Corp");
        db.upsert_account(&active).unwrap();
        db.set_account_domains("active", &["active.com".to_string()])
            .unwrap();

        let mut archived = sample_account("old", "Old Corp");
        archived.archived = true;
        db.upsert_account(&archived).unwrap();
        db.set_account_domains("old", &["old.com".to_string()])
            .unwrap();

        // Exclude archived
        let results = db.get_all_accounts_with_domains(false).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0.id, "active");

        // Include archived
        let results = db.get_all_accounts_with_domains(true).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_insert_and_query_email_signals() {
        let db = test_db();
        setup_account(&db, "acc1", "Acme Corp");

        db.upsert_email_signal(
            "email-1",
            Some("owner@acme.com"),
            None,
            "acc1",
            "account",
            "timeline",
            "Customer asked to move launch date by two weeks",
            Some(0.86),
            Some("neutral"),
            Some("high"),
            Some("2026-02-12T09:00:00Z"),
        )
        .expect("insert signal");

        // Duplicate should be ignored by dedupe unique index.
        db.upsert_email_signal(
            "email-1",
            Some("owner@acme.com"),
            None,
            "acc1",
            "account",
            "timeline",
            "Customer asked to move launch date by two weeks",
            Some(0.86),
            Some("neutral"),
            Some("high"),
            Some("2026-02-12T09:00:00Z"),
        )
        .expect("insert duplicate signal");

        let signals = db
            .list_recent_email_signals_for_entity("acc1", 10)
            .expect("list signals");
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].signal_type, "timeline");
        assert!(signals[0].signal_text.contains("launch date"));
    }

    #[test]
    fn test_domain_based_account_lookup() {
        let db = test_db();
        setup_account(&db, "acc1", "Acme Corp");
        db.set_account_domains("acc1", &["acme.com".to_string()])
            .expect("set domains");

        let candidates = db
            .lookup_account_candidates_by_domain("acme.com")
            .expect("lookup domain");
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].id, "acc1");
    }

    // =========================================================================
    // Email signal pipeline integration tests (S3)
    //
    // These test the same person-resolution → domain-fallback → signal-upsert
    // pipeline used by Executor::sync_email_signals_from_payload, exercised
    // at the DB layer to avoid needing a Tauri AppHandle.
    // =========================================================================

    #[test]
    fn test_email_signal_pipeline_person_direct_match() {
        let db = test_db();
        setup_account(&db, "acc1", "Acme Corp");

        // Create person linked to account
        let person = sample_person("alice@acme.com");
        db.upsert_person(&person).expect("upsert person");
        db.link_person_to_entity(&person.id, "acc1", "contact")
            .expect("link person");

        // Simulate: person lookup → entity resolution → signal insert
        let sender = "alice@acme.com";
        let found = db
            .get_person_by_email(sender)
            .expect("lookup")
            .expect("person should exist");
        let entities = db.get_entities_for_person(&found.id).expect("get entities");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].id, "acc1");

        let inserted = db
            .upsert_email_signal(
                "email-1",
                Some(sender),
                Some(&found.id),
                &entities[0].id,
                entities[0].entity_type.as_str(),
                "expansion",
                "Wants to add 50 seats in Q2",
                Some(0.85),
                Some("positive"),
                Some("medium"),
                Some("2026-02-13T10:00:00Z"),
            )
            .expect("insert signal");
        assert!(inserted);

        let signals = db
            .list_recent_email_signals_for_entity("acc1", 10)
            .expect("list signals");
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].signal_type, "expansion");
        assert_eq!(signals[0].entity_id, "acc1");
        assert_eq!(signals[0].person_id, Some(found.id));
    }

    #[test]
    fn test_email_signal_pipeline_domain_fallback() {
        let db = test_db();
        setup_account(&db, "acc1", "Acme Corp");
        db.set_account_domains("acc1", &["acme.com".to_string()])
            .expect("set domains");

        // No person record — simulate domain fallback
        let sender = "unknown@acme.com";
        let person = db.get_person_by_email(sender).expect("lookup");
        assert!(person.is_none(), "no person should match");

        // Domain fallback
        let domain = sender.split('@').nth(1).unwrap();
        let candidates = db
            .lookup_account_candidates_by_domain(domain)
            .expect("lookup domain");
        assert_eq!(candidates.len(), 1);

        let inserted = db
            .upsert_email_signal(
                "email-2",
                Some(sender),
                None, // no person_id
                &candidates[0].id,
                "account",
                "question",
                "Asking about enterprise pricing",
                Some(0.75),
                Some("neutral"),
                Some("low"),
                None,
            )
            .expect("insert signal");
        assert!(inserted);

        let signals = db
            .list_recent_email_signals_for_entity("acc1", 10)
            .expect("list signals");
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].signal_type, "question");
        assert!(signals[0].person_id.is_none());
    }

    #[test]
    fn test_email_signal_pipeline_deduplication() {
        let db = test_db();
        setup_account(&db, "acc1", "Acme Corp");

        // Insert same signal twice (same email_id + entity)
        let first = db
            .upsert_email_signal(
                "email-dup",
                Some("alice@acme.com"),
                None,
                "acc1",
                "account",
                "expansion",
                "Wants to expand",
                Some(0.85),
                Some("positive"),
                Some("high"),
                Some("2026-02-13T10:00:00Z"),
            )
            .expect("first insert");
        assert!(first);

        let second = db
            .upsert_email_signal(
                "email-dup",
                Some("alice@acme.com"),
                None,
                "acc1",
                "account",
                "expansion",
                "Wants to expand",
                Some(0.85),
                Some("positive"),
                Some("high"),
                Some("2026-02-13T10:00:00Z"),
            )
            .expect("second insert");
        assert!(!second, "duplicate should return false");

        let signals = db
            .list_recent_email_signals_for_entity("acc1", 10)
            .expect("list");
        assert_eq!(signals.len(), 1, "only one signal despite two inserts");
    }

    #[test]
    fn test_email_signal_pipeline_multi_entity_targets() {
        let db = test_db();
        setup_account(&db, "acc1", "Acme Corp");
        setup_account(&db, "acc2", "Acme Sub");

        // Person linked to two accounts
        let person = sample_person("alice@acme.com");
        db.upsert_person(&person).expect("upsert person");
        db.link_person_to_entity(&person.id, "acc1", "contact")
            .expect("link 1");
        db.link_person_to_entity(&person.id, "acc2", "contact")
            .expect("link 2");

        let entities = db
            .get_entities_for_person(&person.id)
            .expect("get entities");
        assert_eq!(entities.len(), 2);

        // Insert signal for each entity (mirrors executor loop)
        for entity in &entities {
            db.upsert_email_signal(
                "email-multi",
                Some("alice@acme.com"),
                Some(&person.id),
                &entity.id,
                entity.entity_type.as_str(),
                "feedback",
                "Great experience with the new feature",
                Some(0.9),
                Some("positive"),
                None,
                Some("2026-02-13T11:00:00Z"),
            )
            .expect("insert");
        }

        let signals_acc1 = db
            .list_recent_email_signals_for_entity("acc1", 10)
            .expect("list acc1");
        let signals_acc2 = db
            .list_recent_email_signals_for_entity("acc2", 10)
            .expect("list acc2");
        assert_eq!(signals_acc1.len(), 1);
        assert_eq!(signals_acc2.len(), 1);
    }
}
