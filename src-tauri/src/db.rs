//! SQLite-based local state management for actions, accounts, and meeting history.
//!
//! The database lives at `~/.dailyos/actions.db` and serves as a disposable cache.
//! Markdown files remain the source of truth; this DB enables fast queries and
//! state tracking (e.g., action completion) that markdown cannot provide.

use std::path::PathBuf;

use chrono::Utc;
use rusqlite::{params, Connection};
use serde::Serialize;
use thiserror::Error;

/// Errors specific to database operations.
#[derive(Debug, Error)]
pub enum DbError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("Home directory not found")]
    HomeDirNotFound,

    #[error("Failed to create database directory: {0}")]
    CreateDir(std::io::Error),
}

/// A row from the `actions` table.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DbAction {
    pub id: String,
    pub title: String,
    pub priority: String,
    pub status: String,
    pub created_at: String,
    pub due_date: Option<String>,
    pub completed_at: Option<String>,
    pub account_id: Option<String>,
    pub project_id: Option<String>,
    pub source_type: Option<String>,
    pub source_id: Option<String>,
    pub source_label: Option<String>,
    pub context: Option<String>,
    pub waiting_on: Option<String>,
    pub updated_at: String,
}

/// A row from the `accounts` table.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DbAccount {
    pub id: String,
    pub name: String,
    pub ring: Option<i32>,
    pub arr: Option<f64>,
    pub health: Option<String>,
    pub contract_start: Option<String>,
    pub contract_end: Option<String>,
    pub csm: Option<String>,
    pub champion: Option<String>,
    pub tracker_path: Option<String>,
    pub updated_at: String,
}

/// A row from the `meetings_history` table.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DbMeeting {
    pub id: String,
    pub title: String,
    pub meeting_type: String,
    pub start_time: String,
    pub end_time: Option<String>,
    pub account_id: Option<String>,
    pub attendees: Option<String>,
    pub notes_path: Option<String>,
    pub summary: Option<String>,
    pub created_at: String,
}

/// A row from the `processing_log` table.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DbProcessingLog {
    pub id: String,
    pub filename: String,
    pub source_path: String,
    pub destination_path: Option<String>,
    pub classification: String,
    pub status: String,
    pub processed_at: Option<String>,
    pub error_message: Option<String>,
    pub created_at: String,
}

/// SQLite connection wrapper for action/account/meeting state.
///
/// This is intentionally NOT `Clone` or `Sync`. It is held behind a
/// `std::sync::Mutex` in `AppState` so that Tauri sync commands can
/// access it safely.
pub struct ActionDb {
    conn: Connection,
}

impl ActionDb {
    /// Open (or create) the database at `~/.dailyos/actions.db` and apply the schema.
    pub fn open() -> Result<Self, DbError> {
        let path = Self::db_path()?;
        Self::open_at(path)
    }

    /// Open a database at an explicit path. Useful for testing.
    fn open_at(path: PathBuf) -> Result<Self, DbError> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(DbError::CreateDir)?;
            }
        }

        let conn = Connection::open(&path)?;

        // Enable WAL mode for better concurrent read performance
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;

        // Apply schema (all statements use IF NOT EXISTS, so this is idempotent)
        conn.execute_batch(include_str!("schema.sql"))?;

        Ok(Self { conn })
    }

    /// Resolve the default database path: `~/.dailyos/actions.db`.
    fn db_path() -> Result<PathBuf, DbError> {
        let home = dirs::home_dir().ok_or(DbError::HomeDirNotFound)?;
        Ok(home.join(".dailyos").join("actions.db"))
    }

    // =========================================================================
    // Actions
    // =========================================================================

    /// Query pending actions where `due_date` is within `days_ahead` days or is NULL.
    ///
    /// Results are ordered: overdue first, then by priority, then by due date.
    pub fn get_due_actions(&self, days_ahead: i32) -> Result<Vec<DbAction>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, priority, status, created_at, due_date, completed_at,
                    account_id, project_id, source_type, source_id, source_label,
                    context, waiting_on, updated_at
             FROM actions
             WHERE status = 'pending'
               AND (due_date IS NULL OR due_date <= date('now', ?1 || ' days'))
             ORDER BY
               CASE WHEN due_date < date('now') THEN 0 ELSE 1 END,
               priority,
               due_date",
        )?;

        let days_param = format!("+{days_ahead}");
        let rows = stmt.query_map(params![days_param], |row| {
            Ok(DbAction {
                id: row.get(0)?,
                title: row.get(1)?,
                priority: row.get(2)?,
                status: row.get(3)?,
                created_at: row.get(4)?,
                due_date: row.get(5)?,
                completed_at: row.get(6)?,
                account_id: row.get(7)?,
                project_id: row.get(8)?,
                source_type: row.get(9)?,
                source_id: row.get(10)?,
                source_label: row.get(11)?,
                context: row.get(12)?,
                waiting_on: row.get(13)?,
                updated_at: row.get(14)?,
            })
        })?;

        let mut actions = Vec::new();
        for row in rows {
            actions.push(row?);
        }
        Ok(actions)
    }

    /// Query pending and waiting actions for a specific account.
    pub fn get_account_actions(&self, account_id: &str) -> Result<Vec<DbAction>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, priority, status, created_at, due_date, completed_at,
                    account_id, project_id, source_type, source_id, source_label,
                    context, waiting_on, updated_at
             FROM actions
             WHERE account_id = ?1
               AND status IN ('pending', 'waiting')
             ORDER BY priority, due_date",
        )?;

        let rows = stmt.query_map(params![account_id], |row| {
            Ok(DbAction {
                id: row.get(0)?,
                title: row.get(1)?,
                priority: row.get(2)?,
                status: row.get(3)?,
                created_at: row.get(4)?,
                due_date: row.get(5)?,
                completed_at: row.get(6)?,
                account_id: row.get(7)?,
                project_id: row.get(8)?,
                source_type: row.get(9)?,
                source_id: row.get(10)?,
                source_label: row.get(11)?,
                context: row.get(12)?,
                waiting_on: row.get(13)?,
                updated_at: row.get(14)?,
            })
        })?;

        let mut actions = Vec::new();
        for row in rows {
            actions.push(row?);
        }
        Ok(actions)
    }

    /// Mark an action as completed with the current timestamp.
    pub fn complete_action(&self, id: &str) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE actions SET status = 'completed', completed_at = ?1, updated_at = ?1
             WHERE id = ?2",
            params![now, id],
        )?;
        Ok(())
    }

    /// Get a single action by its ID.
    pub fn get_action_by_id(&self, id: &str) -> Result<Option<DbAction>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, priority, status, created_at, due_date, completed_at,
                    account_id, project_id, source_type, source_id, source_label,
                    context, waiting_on, updated_at
             FROM actions
             WHERE id = ?1",
        )?;

        let mut rows = stmt.query_map(params![id], |row| {
            Ok(DbAction {
                id: row.get(0)?,
                title: row.get(1)?,
                priority: row.get(2)?,
                status: row.get(3)?,
                created_at: row.get(4)?,
                due_date: row.get(5)?,
                completed_at: row.get(6)?,
                account_id: row.get(7)?,
                project_id: row.get(8)?,
                source_type: row.get(9)?,
                source_id: row.get(10)?,
                source_label: row.get(11)?,
                context: row.get(12)?,
                waiting_on: row.get(13)?,
                updated_at: row.get(14)?,
            })
        })?;

        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// Get actions recently marked as completed (within the last N hours)
    /// that have a source_label set (so we know which file to update).
    pub fn get_recently_completed(&self, since_hours: u32) -> Result<Vec<DbAction>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, priority, status, created_at, due_date, completed_at,
                    account_id, project_id, source_type, source_id, source_label,
                    context, waiting_on, updated_at
             FROM actions
             WHERE status = 'completed'
               AND completed_at >= datetime('now', ?1)
               AND source_label IS NOT NULL
             ORDER BY completed_at DESC",
        )?;

        let hours_param = format!("-{} hours", since_hours);
        let rows = stmt.query_map(params![hours_param], |row| {
            Ok(DbAction {
                id: row.get(0)?,
                title: row.get(1)?,
                priority: row.get(2)?,
                status: row.get(3)?,
                created_at: row.get(4)?,
                due_date: row.get(5)?,
                completed_at: row.get(6)?,
                account_id: row.get(7)?,
                project_id: row.get(8)?,
                source_type: row.get(9)?,
                source_id: row.get(10)?,
                source_label: row.get(11)?,
                context: row.get(12)?,
                waiting_on: row.get(13)?,
                updated_at: row.get(14)?,
            })
        })?;

        let mut actions = Vec::new();
        for row in rows {
            actions.push(row?);
        }
        Ok(actions)
    }

    /// Insert or update an action, but never overwrite a user-set `completed` status.
    ///
    /// If the action already exists and is `completed`, skip the update entirely.
    /// This ensures that daily briefing syncs don't reset user-completed actions.
    pub fn upsert_action_if_not_completed(&self, action: &DbAction) -> Result<(), DbError> {
        // Check if this action already exists and is completed
        let existing_status: Option<String> = self
            .conn
            .query_row(
                "SELECT status FROM actions WHERE id = ?1",
                params![action.id],
                |row| row.get(0),
            )
            .ok();

        if existing_status.as_deref() == Some("completed") {
            // Don't overwrite user-completed actions
            return Ok(());
        }

        self.upsert_action(action)
    }

    /// Insert or update an action. Uses SQLite `ON CONFLICT` (upsert).
    pub fn upsert_action(&self, action: &DbAction) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT INTO actions (
                id, title, priority, status, created_at, due_date, completed_at,
                account_id, project_id, source_type, source_id, source_label,
                context, waiting_on, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
             ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                priority = excluded.priority,
                status = excluded.status,
                due_date = excluded.due_date,
                completed_at = excluded.completed_at,
                account_id = excluded.account_id,
                project_id = excluded.project_id,
                source_type = excluded.source_type,
                source_id = excluded.source_id,
                source_label = excluded.source_label,
                context = excluded.context,
                waiting_on = excluded.waiting_on,
                updated_at = excluded.updated_at",
            params![
                action.id,
                action.title,
                action.priority,
                action.status,
                action.created_at,
                action.due_date,
                action.completed_at,
                action.account_id,
                action.project_id,
                action.source_type,
                action.source_id,
                action.source_label,
                action.context,
                action.waiting_on,
                action.updated_at,
            ],
        )?;
        Ok(())
    }

    // =========================================================================
    // Accounts
    // =========================================================================

    /// Insert or update an account.
    pub fn upsert_account(&self, account: &DbAccount) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT INTO accounts (
                id, name, ring, arr, health, contract_start, contract_end,
                csm, champion, tracker_path, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                ring = excluded.ring,
                arr = excluded.arr,
                health = excluded.health,
                contract_start = excluded.contract_start,
                contract_end = excluded.contract_end,
                csm = excluded.csm,
                champion = excluded.champion,
                tracker_path = excluded.tracker_path,
                updated_at = excluded.updated_at",
            params![
                account.id,
                account.name,
                account.ring,
                account.arr,
                account.health,
                account.contract_start,
                account.contract_end,
                account.csm,
                account.champion,
                account.tracker_path,
                account.updated_at,
            ],
        )?;
        Ok(())
    }

    /// Get an account by ID.
    pub fn get_account(&self, id: &str) -> Result<Option<DbAccount>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, ring, arr, health, contract_start, contract_end,
                    csm, champion, tracker_path, updated_at
             FROM accounts
             WHERE id = ?1",
        )?;

        let mut rows = stmt.query_map(params![id], |row| {
            Ok(DbAccount {
                id: row.get(0)?,
                name: row.get(1)?,
                ring: row.get(2)?,
                arr: row.get(3)?,
                health: row.get(4)?,
                contract_start: row.get(5)?,
                contract_end: row.get(6)?,
                csm: row.get(7)?,
                champion: row.get(8)?,
                tracker_path: row.get(9)?,
                updated_at: row.get(10)?,
            })
        })?;

        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    // =========================================================================
    // Meetings
    // =========================================================================

    /// Query recent meetings for an account within `lookback_days`, limited to `limit` results.
    pub fn get_meeting_history(
        &self,
        account_id: &str,
        lookback_days: i32,
        limit: i32,
    ) -> Result<Vec<DbMeeting>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, meeting_type, start_time, end_time,
                    account_id, attendees, notes_path, summary, created_at
             FROM meetings_history
             WHERE account_id = ?1
               AND start_time >= date('now', ?2 || ' days')
             ORDER BY start_time DESC
             LIMIT ?3",
        )?;

        let days_param = format!("-{lookback_days}");
        let rows = stmt.query_map(params![account_id, days_param, limit], |row| {
            Ok(DbMeeting {
                id: row.get(0)?,
                title: row.get(1)?,
                meeting_type: row.get(2)?,
                start_time: row.get(3)?,
                end_time: row.get(4)?,
                account_id: row.get(5)?,
                attendees: row.get(6)?,
                notes_path: row.get(7)?,
                summary: row.get(8)?,
                created_at: row.get(9)?,
            })
        })?;

        let mut meetings = Vec::new();
        for row in rows {
            meetings.push(row?);
        }
        Ok(meetings)
    }

    // =========================================================================
    // Processing Log
    // =========================================================================

    /// Insert a processing log entry.
    pub fn insert_processing_log(&self, entry: &DbProcessingLog) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT INTO processing_log (id, filename, source_path, destination_path, classification, status, processed_at, error_message, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                entry.id,
                entry.filename,
                entry.source_path,
                entry.destination_path,
                entry.classification,
                entry.status,
                entry.processed_at,
                entry.error_message,
                entry.created_at,
            ],
        )?;
        Ok(())
    }

    /// Get recent processing log entries.
    pub fn get_processing_log(&self, limit: i32) -> Result<Vec<DbProcessingLog>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, filename, source_path, destination_path, classification, status, processed_at, error_message, created_at
             FROM processing_log
             ORDER BY created_at DESC
             LIMIT ?1",
        )?;

        let rows = stmt.query_map(params![limit], |row| {
            Ok(DbProcessingLog {
                id: row.get(0)?,
                filename: row.get(1)?,
                source_path: row.get(2)?,
                destination_path: row.get(3)?,
                classification: row.get(4)?,
                status: row.get(5)?,
                processed_at: row.get(6)?,
                error_message: row.get(7)?,
                created_at: row.get(8)?,
            })
        })?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(row?);
        }
        Ok(entries)
    }

    // =========================================================================
    // Captures (post-meeting wins/risks)
    // =========================================================================

    /// Insert a capture (win, risk, or action) from a post-meeting prompt.
    pub fn insert_capture(
        &self,
        meeting_id: &str,
        meeting_title: &str,
        account_id: Option<&str>,
        capture_type: &str,
        content: &str,
    ) -> Result<(), DbError> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO captures (id, meeting_id, meeting_title, account_id, capture_type, content, captured_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, meeting_id, meeting_title, account_id, capture_type, content, now],
        )?;
        Ok(())
    }

    // =========================================================================
    // Meetings
    // =========================================================================

    /// Insert or update a meeting history record.
    pub fn upsert_meeting(&self, meeting: &DbMeeting) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT INTO meetings_history (
                id, title, meeting_type, start_time, end_time,
                account_id, attendees, notes_path, summary, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                meeting_type = excluded.meeting_type,
                start_time = excluded.start_time,
                end_time = excluded.end_time,
                account_id = excluded.account_id,
                attendees = excluded.attendees,
                notes_path = excluded.notes_path,
                summary = excluded.summary",
            params![
                meeting.id,
                meeting.title,
                meeting.meeting_type,
                meeting.start_time,
                meeting.end_time,
                meeting.account_id,
                meeting.attendees,
                meeting.notes_path,
                meeting.summary,
                meeting.created_at,
            ],
        )?;
        Ok(())
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a temporary database for testing.
    ///
    /// We leak the `TempDir` so the directory persists for the duration of the test.
    /// Test temp dirs are cleaned up by the OS.
    fn test_db() -> ActionDb {
        let dir = tempfile::tempdir().expect("Failed to create temp dir");
        let path = dir.path().join("test_actions.db");
        // Leak the TempDir so it is not deleted while the DB connection is open.
        std::mem::forget(dir);
        ActionDb::open_at(path).expect("Failed to open test database")
    }

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
            .query_row("SELECT COUNT(*) FROM meetings_history", [], |row| row.get(0))
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

        let results = db
            .get_account_actions("acme-corp")
            .expect("account query");
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
            ring: Some(1),
            arr: Some(120_000.0),
            health: Some("green".to_string()),
            contract_start: Some("2025-01-01".to_string()),
            contract_end: Some("2026-01-01".to_string()),
            csm: Some("Alice".to_string()),
            champion: Some("Bob".to_string()),
            tracker_path: Some("Accounts/acme-corp".to_string()),
            updated_at: now,
        };

        db.upsert_account(&account).expect("upsert account");

        let result = db.get_account("acme-corp").expect("get account");
        assert!(result.is_some());
        let acct = result.unwrap();
        assert_eq!(acct.name, "Acme Corp");
        assert_eq!(acct.ring, Some(1));
        assert_eq!(acct.arr, Some(120_000.0));
    }

    #[test]
    fn test_get_account_not_found() {
        let db = test_db();
        let result = db.get_account("nonexistent").expect("get account");
        assert!(result.is_none());
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
            account_id: Some("acme-corp".to_string()),
            attendees: Some(r#"["alice@acme.com","bob@us.com"]"#.to_string()),
            notes_path: None,
            summary: Some("Discussed renewal".to_string()),
            created_at: now,
        };

        db.upsert_meeting(&meeting).expect("upsert meeting");

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
            let meeting = DbMeeting {
                id: format!("mtg-{i:03}"),
                title: format!("Meeting {i}"),
                meeting_type: "customer".to_string(),
                start_time: now.clone(),
                end_time: None,
                account_id: Some("acme-corp".to_string()),
                attendees: None,
                notes_path: None,
                summary: None,
                created_at: now.clone(),
            };
            db.upsert_meeting(&meeting).expect("upsert");
        }

        let results = db
            .get_meeting_history("acme-corp", 30, 3)
            .expect("history");
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
    fn test_idempotent_schema_application() {
        // Opening the same DB twice should not error (IF NOT EXISTS)
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("idempotent.db");

        let _db1 = ActionDb::open_at(path.clone()).expect("first open");
        let _db2 = ActionDb::open_at(path).expect("second open should not fail");
    }
}
