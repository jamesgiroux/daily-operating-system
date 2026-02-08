//! SQLite-based local state management for actions, accounts, and meeting history.
//!
//! The database lives at `~/.dailyos/actions.db` and serves as the working store
//! for operational data (ADR-0048). The filesystem (markdown + JSON) is the durable
//! layer; SQLite enables fast queries, state tracking, and cross-entity intelligence.
//! SQLite is not disposable — important state lives here and is written back to the
//! filesystem at natural synchronization points (archive, dashboard regeneration).

use std::path::PathBuf;

use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::entity::{DbEntity, EntityType};

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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub calendar_event_id: Option<String>,
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

/// A row from the `captures` table (post-meeting wins/risks).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DbCapture {
    pub id: String,
    pub meeting_id: String,
    pub meeting_title: String,
    pub account_id: Option<String>,
    pub capture_type: String,
    pub content: String,
    pub captured_at: String,
}

/// Stakeholder relationship signals computed from meeting history and account data (I43).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StakeholderSignals {
    /// Number of meetings in the last 30 days
    pub meeting_frequency_30d: i32,
    /// Number of meetings in the last 90 days
    pub meeting_frequency_90d: i32,
    /// ISO timestamp of the most recent meeting
    pub last_meeting: Option<String>,
    /// ISO timestamp of last account contact (updated_at from accounts table)
    pub last_contact: Option<String>,
    /// Relationship temperature: "hot", "warm", "cool", "cold"
    pub temperature: String,
    /// Trend: "increasing", "stable", "decreasing"
    pub trend: String,
}

/// A row from the `people` table (I51).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbPerson {
    pub id: String,
    pub email: String,
    pub name: String,
    pub organization: Option<String>,
    pub role: Option<String>,
    pub relationship: String, // "internal" | "external" | "unknown"
    pub notes: Option<String>,
    pub tracker_path: Option<String>,
    pub last_seen: Option<String>,
    pub first_seen: Option<String>,
    pub meeting_count: i32,
    pub updated_at: String,
}

/// Person-level relationship signals (parallel to StakeholderSignals).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonSignals {
    pub meeting_frequency_30d: i32,
    pub meeting_frequency_90d: i32,
    pub last_meeting: Option<String>,
    pub temperature: String,
    pub trend: String,
}

/// Compute relationship temperature from last meeting date.
fn compute_temperature(last_meeting_iso: &str) -> String {
    let days = days_since_iso(last_meeting_iso);
    match days {
        Some(d) if d < 7 => "hot".to_string(),
        Some(d) if d < 30 => "warm".to_string(),
        Some(d) if d < 60 => "cool".to_string(),
        _ => "cold".to_string(),
    }
}

/// Compute meeting trend from 30d vs 90d frequency.
fn compute_trend(count_30d: i32, count_90d: i32) -> String {
    if count_90d == 0 {
        return "stable".to_string();
    }
    // Expected 30d count is ~1/3 of 90d count (even distribution)
    let expected_30d = count_90d as f64 / 3.0;
    let actual_30d = count_30d as f64;

    if actual_30d > expected_30d * 1.3 {
        "increasing".to_string()
    } else if actual_30d < expected_30d * 0.7 {
        "decreasing".to_string()
    } else {
        "stable".to_string()
    }
}

/// Parse an ISO datetime string and return days since that date.
fn days_since_iso(iso: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(iso)
        .or_else(|_| chrono::DateTime::parse_from_rfc3339(&format!("{}+00:00", iso.trim_end_matches('Z'))))
        .ok()
        .map(|dt| (Utc::now() - dt.with_timezone(&Utc)).num_days())
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
    /// Borrow the underlying connection for ad-hoc queries.
    pub fn conn_ref(&self) -> &Connection {
        &self.conn
    }

    /// Open (or create) the database at `~/.dailyos/actions.db` and apply the schema.
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

        // Apply schema (all statements use IF NOT EXISTS, so this is idempotent)
        conn.execute_batch(include_str!("schema.sql"))?;

        // Migration: add calendar_event_id to meetings_history (ignore if exists)
        let _ = conn.execute_batch(
            "ALTER TABLE meetings_history ADD COLUMN calendar_event_id TEXT;",
        );

        // Migration: backfill entities from accounts (ADR-0045).
        // Idempotent — INSERT OR IGNORE skips existing rows.
        let _ = conn.execute_batch(
            "INSERT OR IGNORE INTO entities (id, name, entity_type, tracker_path, updated_at)
             SELECT id, name, 'account', tracker_path, updated_at FROM accounts;",
        );

        // Migration: add needs_decision flag to actions (I42 — Executive Intelligence)
        let _ = conn.execute_batch(
            "ALTER TABLE actions ADD COLUMN needs_decision INTEGER DEFAULT 0;",
        );

        // Migration: add 'decision' to captures.capture_type CHECK constraint.
        // SQLite can't ALTER CHECK constraints, so we recreate the table if needed.
        // Recreating captures is safe — the table schema changes but data is
        // rebuilt from transcript processing and post-meeting capture.
        Self::migrate_captures_decision(&conn)?;

        Ok(Self { conn })
    }

    /// Resolve the default database path: `~/.dailyos/actions.db`.
    fn db_path() -> Result<PathBuf, DbError> {
        let home = dirs::home_dir().ok_or(DbError::HomeDirNotFound)?;
        Ok(home.join(".dailyos").join("actions.db"))
    }

    /// Migrate the `captures` table to accept 'decision' as a capture_type.
    ///
    /// Tries a test insert — if it succeeds the constraint already allows
    /// 'decision' (e.g. fresh DB from updated schema.sql). If it fails,
    /// recreate the table with the new constraint, preserving existing rows.
    fn migrate_captures_decision(conn: &Connection) -> Result<(), DbError> {
        // Quick probe: try inserting and rolling back
        let needs_migration = conn
            .execute(
                "INSERT INTO captures (id, meeting_id, meeting_title, capture_type, content)
                 VALUES ('__probe__', '__probe__', '__probe__', 'decision', '__probe__')",
                [],
            )
            .is_err();

        if !needs_migration {
            // Probe succeeded — clean up and return
            let _ = conn.execute("DELETE FROM captures WHERE id = '__probe__'", []);
            return Ok(());
        }

        // Recreate with new constraint
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS captures_v2 (
                id TEXT PRIMARY KEY,
                meeting_id TEXT NOT NULL,
                meeting_title TEXT NOT NULL,
                account_id TEXT,
                capture_type TEXT CHECK(capture_type IN ('win', 'risk', 'action', 'decision')) NOT NULL,
                content TEXT NOT NULL,
                owner TEXT,
                due_date TEXT,
                captured_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            INSERT OR IGNORE INTO captures_v2 SELECT * FROM captures;
            DROP TABLE captures;
            ALTER TABLE captures_v2 RENAME TO captures;
            CREATE INDEX IF NOT EXISTS idx_captures_meeting ON captures(meeting_id);
            CREATE INDEX IF NOT EXISTS idx_captures_account ON captures(account_id);
            CREATE INDEX IF NOT EXISTS idx_captures_type ON captures(capture_type);",
        )?;

        Ok(())
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

    /// Reopen a completed action, clearing the completed_at timestamp.
    pub fn reopen_action(&self, id: &str) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE actions SET status = 'pending', completed_at = NULL, updated_at = ?1
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

    /// Get all actions completed within the last N hours (for display in the UI).
    pub fn get_completed_actions(&self, since_hours: u32) -> Result<Vec<DbAction>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, priority, status, created_at, due_date, completed_at,
                    account_id, project_id, source_type, source_id, source_label,
                    context, waiting_on, updated_at
             FROM actions
             WHERE status = 'completed'
               AND completed_at >= datetime('now', ?1)
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
    /// Checks two conditions before inserting:
    /// 1. **Title-based guard**: If a matching action (same title + account) is already
    ///    completed under *any* ID, skip the insert. This catches cross-source duplicates
    ///    where the same action arrives from briefing vs inbox vs post-meeting capture
    ///    with different ID schemes.
    /// 2. **ID-based guard**: If an action with this exact ID is already completed, skip.
    ///
    /// This ensures that daily briefing syncs don't resurrect completed actions (I23).
    pub fn upsert_action_if_not_completed(&self, action: &DbAction) -> Result<(), DbError> {
        // Guard 1: Title-based cross-source dedup — skip if ANY action with the
        // same title+account already exists (pending, waiting, or completed).
        let title_exists: bool = self
            .conn
            .query_row(
                "SELECT 1 FROM actions
                 WHERE LOWER(TRIM(title)) = LOWER(TRIM(?1))
                   AND (account_id = ?2 OR (?2 IS NULL AND account_id IS NULL))
                 LIMIT 1",
                params![action.title, action.account_id],
                |_row| Ok(true),
            )
            .unwrap_or(false);

        if title_exists {
            return Ok(());
        }

        // Guard 2: ID-based check — don't overwrite a completed action
        let existing_status: Option<String> = self
            .conn
            .query_row(
                "SELECT status FROM actions WHERE id = ?1",
                params![action.id],
                |row| row.get(0),
            )
            .ok();

        if existing_status.as_deref() == Some("completed") {
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

    /// Get pending actions from non-briefing sources (post-meeting capture, inbox).
    ///
    /// These actions live in SQLite but are NOT in `actions.json` (which only
    /// contains briefing-generated actions). Used by `get_dashboard_data()` to
    /// merge captured actions into the dashboard view (I17).
    pub fn get_non_briefing_pending_actions(&self) -> Result<Vec<DbAction>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, priority, status, created_at, due_date, completed_at,
                    account_id, project_id, source_type, source_id, source_label,
                    context, waiting_on, updated_at
             FROM actions
             WHERE status IN ('pending', 'waiting')
               AND source_type IN ('post_meeting', 'inbox', 'ai-inbox', 'transcript')
             ORDER BY priority, created_at DESC",
        )?;

        let rows = stmt.query_map([], |row| {
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

    /// Get all action titles from the database (for dedup in Rust delivery).
    pub fn get_all_action_titles(&self) -> Result<Vec<String>, DbError> {
        let mut stmt = self
            .conn
            .prepare("SELECT LOWER(TRIM(title)) FROM actions")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut titles = Vec::new();
        for row in rows {
            titles.push(row?);
        }
        Ok(titles)
    }

    // =========================================================================
    // Accounts
    // =========================================================================

    /// Insert or update an account. Also mirrors to the `entities` table (ADR-0045).
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
        // Keep entity mirror in sync
        self.ensure_entity_for_account(account)?;
        Ok(())
    }

    /// Touch `updated_at` on an account as a last-contact signal.
    ///
    /// Matches by ID or by case-insensitive name. Returns `true` if a row
    /// was updated, `false` if no account matched.
    pub fn touch_account_last_contact(&self, account_name: &str) -> Result<bool, DbError> {
        let now = Utc::now().to_rfc3339();
        let rows = self.conn.execute(
            "UPDATE accounts SET updated_at = ?1
             WHERE id = ?2 OR LOWER(name) = LOWER(?2)",
            params![now, account_name],
        )?;
        Ok(rows > 0)
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
    // Entities (ADR-0045)
    // =========================================================================

    /// Insert or update a profile-agnostic entity.
    pub fn upsert_entity(&self, entity: &DbEntity) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT INTO entities (id, name, entity_type, tracker_path, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                entity_type = excluded.entity_type,
                tracker_path = excluded.tracker_path,
                updated_at = excluded.updated_at",
            params![
                entity.id,
                entity.name,
                entity.entity_type.as_str(),
                entity.tracker_path,
                entity.updated_at,
            ],
        )?;
        Ok(())
    }

    /// Fetch an entity by ID.
    pub fn get_entity(&self, id: &str) -> Result<Option<DbEntity>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, entity_type, tracker_path, updated_at
             FROM entities WHERE id = ?1",
        )?;

        let mut rows = stmt.query_map(params![id], |row| {
            let et: String = row.get(2)?;
            Ok(DbEntity {
                id: row.get(0)?,
                name: row.get(1)?,
                entity_type: EntityType::from_str_lossy(&et),
                tracker_path: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?;

        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// Touch `updated_at` on an entity as a last-contact signal.
    ///
    /// Matches by ID or by case-insensitive name. Returns `true` if a row
    /// was updated, `false` if no entity matched.
    pub fn touch_entity_last_contact(&self, name: &str) -> Result<bool, DbError> {
        let now = Utc::now().to_rfc3339();
        let rows = self.conn.execute(
            "UPDATE entities SET updated_at = ?1
             WHERE id = ?2 OR LOWER(name) = LOWER(?2)",
            params![now, name],
        )?;
        Ok(rows > 0)
    }

    /// List entities of a given type.
    pub fn get_entities_by_type(&self, entity_type: &str) -> Result<Vec<DbEntity>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, entity_type, tracker_path, updated_at
             FROM entities WHERE entity_type = ?1
             ORDER BY name",
        )?;

        let rows = stmt.query_map(params![entity_type], |row| {
            let et: String = row.get(2)?;
            Ok(DbEntity {
                id: row.get(0)?,
                name: row.get(1)?,
                entity_type: EntityType::from_str_lossy(&et),
                tracker_path: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?;

        let mut entities = Vec::new();
        for row in rows {
            entities.push(row?);
        }
        Ok(entities)
    }

    /// Upsert an entity row that mirrors a CS account.
    ///
    /// Called from `upsert_account()` to keep the entity layer in sync.
    pub fn ensure_entity_for_account(&self, account: &DbAccount) -> Result<(), DbError> {
        let entity = DbEntity {
            id: account.id.clone(),
            name: account.name.clone(),
            entity_type: EntityType::Account,
            tracker_path: account.tracker_path.clone(),
            updated_at: account.updated_at.clone(),
        };
        self.upsert_entity(&entity)
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
                    account_id, attendees, notes_path, summary, created_at,
                    calendar_event_id
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
                calendar_event_id: row.get(10)?,
            })
        })?;

        let mut meetings = Vec::new();
        for row in rows {
            meetings.push(row?);
        }
        Ok(meetings)
    }

    // =========================================================================
    // Stakeholder Signals (I43)
    // =========================================================================

    /// Compute stakeholder signals for an account: meeting frequency, last contact,
    /// and relationship temperature. Returns `None` if account not found.
    pub fn get_stakeholder_signals(&self, account_id: &str) -> Result<StakeholderSignals, DbError> {
        // Meeting counts for 30/90 day windows
        let count_30d: i32 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM meetings_history
                 WHERE account_id = ?1
                   AND start_time >= date('now', '-30 days')",
                params![account_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let count_90d: i32 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM meetings_history
                 WHERE account_id = ?1
                   AND start_time >= date('now', '-90 days')",
                params![account_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        // Last meeting date
        let last_meeting: Option<String> = self
            .conn
            .query_row(
                "SELECT MAX(start_time) FROM meetings_history WHERE account_id = ?1",
                params![account_id],
                |row| row.get(0),
            )
            .unwrap_or(None);

        // Last contact from accounts table (updated_at is touched on each interaction)
        let last_contact: Option<String> = self
            .conn
            .query_row(
                "SELECT updated_at FROM accounts
                 WHERE id = ?1 OR LOWER(name) = LOWER(?1)",
                params![account_id],
                |row| row.get(0),
            )
            .ok();

        // Temperature: based on days since last meeting
        let temperature = match &last_meeting {
            Some(dt) => compute_temperature(dt),
            None => "cold".to_string(),
        };

        // Trend: compare 30d vs 90d rate
        let trend = compute_trend(count_30d, count_90d);

        Ok(StakeholderSignals {
            meeting_frequency_30d: count_30d,
            meeting_frequency_90d: count_90d,
            last_meeting,
            last_contact,
            temperature,
            trend,
        })
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

    /// Query recent captures (wins/risks) for an account within `days_back` days.
    ///
    /// Used by meeting:prep (ADR-0030 / I33) to surface recent wins and risks
    /// in meeting preparation context.
    pub fn get_captures_for_account(
        &self,
        account_id: &str,
        days_back: i32,
    ) -> Result<Vec<DbCapture>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, meeting_id, meeting_title, account_id, capture_type, content, captured_at
             FROM captures
             WHERE account_id = ?1
               AND captured_at >= date('now', ?2 || ' days')
             ORDER BY captured_at DESC",
        )?;

        let days_param = format!("-{days_back}");
        let rows = stmt.query_map(params![account_id, days_param], |row| {
            Ok(DbCapture {
                id: row.get(0)?,
                meeting_id: row.get(1)?,
                meeting_title: row.get(2)?,
                account_id: row.get(3)?,
                capture_type: row.get(4)?,
                content: row.get(5)?,
                captured_at: row.get(6)?,
            })
        })?;

        let mut captures = Vec::new();
        for row in rows {
            captures.push(row?);
        }
        Ok(captures)
    }

    /// Query all captures (wins, risks, decisions) for a specific meeting.
    pub fn get_captures_for_meeting(&self, meeting_id: &str) -> Result<Vec<DbCapture>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, meeting_id, meeting_title, account_id, capture_type, content, captured_at
             FROM captures
             WHERE meeting_id = ?1
             ORDER BY captured_at",
        )?;

        let rows = stmt.query_map(params![meeting_id], |row| {
            Ok(DbCapture {
                id: row.get(0)?,
                meeting_id: row.get(1)?,
                meeting_title: row.get(2)?,
                account_id: row.get(3)?,
                capture_type: row.get(4)?,
                content: row.get(5)?,
                captured_at: row.get(6)?,
            })
        })?;

        let mut captures = Vec::new();
        for row in rows {
            captures.push(row?);
        }
        Ok(captures)
    }

    /// Query actions extracted from a transcript for a specific meeting.
    pub fn get_actions_for_meeting(&self, meeting_id: &str) -> Result<Vec<DbAction>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, priority, status, created_at, due_date, completed_at,
                    account_id, project_id, source_type, source_id, source_label,
                    context, waiting_on, updated_at
             FROM actions
             WHERE source_id = ?1 AND source_type = 'transcript'
             ORDER BY priority, created_at",
        )?;

        let rows = stmt.query_map(params![meeting_id], |row| {
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

    /// Query all captures (wins/risks/decisions) recorded on a given date.
    ///
    /// Used by the daily impact rollup (I36) to aggregate outcomes into
    /// the weekly impact file during the archive workflow.
    pub fn get_captures_for_date(&self, date: &str) -> Result<Vec<DbCapture>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, meeting_id, meeting_title, account_id, capture_type, content, captured_at
             FROM captures
             WHERE date(captured_at) = ?1
             ORDER BY account_id, captured_at",
        )?;

        let rows = stmt.query_map(params![date], |row| {
            Ok(DbCapture {
                id: row.get(0)?,
                meeting_id: row.get(1)?,
                meeting_title: row.get(2)?,
                account_id: row.get(3)?,
                capture_type: row.get(4)?,
                content: row.get(5)?,
                captured_at: row.get(6)?,
            })
        })?;

        let mut captures = Vec::new();
        for row in rows {
            captures.push(row?);
        }
        Ok(captures)
    }

    /// Update the content of a capture (win/risk/decision).
    pub fn update_capture(&self, id: &str, content: &str) -> Result<(), DbError> {
        self.conn.execute(
            "UPDATE captures SET content = ?1 WHERE id = ?2",
            params![content, id],
        )?;
        Ok(())
    }

    /// Update an action's priority.
    pub fn update_action_priority(&self, id: &str, priority: &str) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE actions SET priority = ?1, updated_at = ?2 WHERE id = ?3",
            params![priority, now, id],
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
                account_id, attendees, notes_path, summary, created_at,
                calendar_event_id
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                meeting_type = excluded.meeting_type,
                start_time = excluded.start_time,
                end_time = excluded.end_time,
                account_id = excluded.account_id,
                attendees = excluded.attendees,
                notes_path = excluded.notes_path,
                summary = excluded.summary,
                calendar_event_id = excluded.calendar_event_id",
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
                meeting.calendar_event_id,
            ],
        )?;
        Ok(())
    }

    // =========================================================================
    // Prep State Tracking (ADR-0033)
    // =========================================================================

    /// Record that a meeting prep has been reviewed.
    pub fn mark_prep_reviewed(
        &self,
        prep_file: &str,
        calendar_event_id: Option<&str>,
        title: &str,
    ) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO meeting_prep_state (prep_file, calendar_event_id, reviewed_at, title)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(prep_file) DO UPDATE SET
                reviewed_at = excluded.reviewed_at,
                calendar_event_id = excluded.calendar_event_id",
            params![prep_file, calendar_event_id, now, title],
        )?;
        Ok(())
    }

    /// Get all reviewed prep files. Returns a map of prep_file → reviewed_at.
    pub fn get_reviewed_preps(&self) -> Result<std::collections::HashMap<String, String>, DbError> {
        let mut stmt = self
            .conn
            .prepare("SELECT prep_file, reviewed_at FROM meeting_prep_state")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        let mut map = std::collections::HashMap::new();
        for row in rows {
            let (file, at) = row?;
            map.insert(file, at);
        }
        Ok(map)
    }

    // =========================================================================
    // Intelligence Queries (I42 — Executive Intelligence)
    // =========================================================================

    /// Get actions in `waiting` status that are older than `stale_days`.
    ///
    /// These represent stale delegations — things handed off to someone else
    /// that haven't been resolved. Ordered by staleness (oldest first).
    pub fn get_stale_delegations(&self, stale_days: i32) -> Result<Vec<DbAction>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, priority, status, created_at, due_date, completed_at,
                    account_id, project_id, source_type, source_id, source_label,
                    context, waiting_on, updated_at
             FROM actions
             WHERE status = 'waiting'
               AND created_at <= datetime('now', ?1 || ' days')
             ORDER BY created_at ASC",
        )?;

        let days_param = format!("-{stale_days}");
        let rows = stmt.query_map(params![days_param], Self::map_action_row)?;

        let mut actions = Vec::new();
        for row in rows {
            actions.push(row?);
        }
        Ok(actions)
    }

    /// Get actions flagged as needing a decision, due within `days_ahead` days.
    ///
    /// The `needs_decision` flag is set by AI enrichment during briefing generation.
    /// Actions with no due date are included (they still need decisions).
    pub fn get_flagged_decisions(&self, days_ahead: i32) -> Result<Vec<DbAction>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, priority, status, created_at, due_date, completed_at,
                    account_id, project_id, source_type, source_id, source_label,
                    context, waiting_on, updated_at
             FROM actions
             WHERE needs_decision = 1
               AND status = 'pending'
               AND (due_date IS NULL OR due_date <= date('now', ?1 || ' days'))
             ORDER BY
               CASE WHEN due_date IS NULL THEN 1 ELSE 0 END,
               due_date ASC,
               priority",
        )?;

        let days_param = format!("+{days_ahead}");
        let rows = stmt.query_map(params![days_param], Self::map_action_row)?;

        let mut actions = Vec::new();
        for row in rows {
            actions.push(row?);
        }
        Ok(actions)
    }

    /// Get accounts with `contract_end` within `days_ahead` days.
    ///
    /// Returns accounts approaching renewal, ordered by soonest first.
    pub fn get_renewal_alerts(&self, days_ahead: i32) -> Result<Vec<DbAccount>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, ring, arr, health, contract_start, contract_end,
                    csm, champion, tracker_path, updated_at
             FROM accounts
             WHERE contract_end IS NOT NULL
               AND contract_end >= date('now')
               AND contract_end <= date('now', ?1 || ' days')
             ORDER BY contract_end ASC",
        )?;

        let days_param = format!("+{days_ahead}");
        let rows = stmt.query_map(params![days_param], |row| {
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

        let mut accounts = Vec::new();
        for row in rows {
            accounts.push(row?);
        }
        Ok(accounts)
    }

    /// Get accounts where `updated_at` is older than `stale_days`.
    ///
    /// Represents accounts that haven't been touched (via meetings, captures,
    /// or manual updates) in a while — a signal to check in.
    pub fn get_stale_accounts(&self, stale_days: i32) -> Result<Vec<DbAccount>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, ring, arr, health, contract_start, contract_end,
                    csm, champion, tracker_path, updated_at
             FROM accounts
             WHERE updated_at <= datetime('now', ?1 || ' days')
             ORDER BY updated_at ASC",
        )?;

        let days_param = format!("-{stale_days}");
        let rows = stmt.query_map(params![days_param], |row| {
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

        let mut accounts = Vec::new();
        for row in rows {
            accounts.push(row?);
        }
        Ok(accounts)
    }

    /// Flag an action as needing a decision. Called by AI enrichment during
    /// briefing generation to mark actions that require user decisions.
    pub fn flag_action_as_decision(&self, id: &str) -> Result<bool, DbError> {
        let rows = self.conn.execute(
            "UPDATE actions SET needs_decision = 1 WHERE id = ?1",
            params![id],
        )?;
        Ok(rows > 0)
    }

    /// Clear all decision flags. Called before re-flagging during enrichment
    /// so that stale flags from previous runs are removed.
    pub fn clear_decision_flags(&self) -> Result<(), DbError> {
        self.conn.execute(
            "UPDATE actions SET needs_decision = 0 WHERE needs_decision = 1",
            [],
        )?;
        Ok(())
    }

    // =========================================================================
    // People (I51)
    // =========================================================================

    /// Insert or update a person. Idempotent — won't overwrite manually-set fields
    /// unless the incoming data explicitly provides them.
    pub fn upsert_person(&self, person: &DbPerson) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT INTO people (
                id, email, name, organization, role, relationship, notes,
                tracker_path, last_seen, first_seen, meeting_count, updated_at
             ) VALUES (?1, LOWER(?2), ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT(id) DO UPDATE SET
                name = COALESCE(excluded.name, people.name),
                organization = COALESCE(excluded.organization, people.organization),
                role = COALESCE(excluded.role, people.role),
                relationship = CASE
                    WHEN people.relationship = 'unknown' THEN excluded.relationship
                    ELSE people.relationship
                END,
                notes = COALESCE(excluded.notes, people.notes),
                tracker_path = COALESCE(excluded.tracker_path, people.tracker_path),
                last_seen = CASE
                    WHEN excluded.last_seen > COALESCE(people.last_seen, '') THEN excluded.last_seen
                    ELSE people.last_seen
                END,
                updated_at = excluded.updated_at",
            params![
                person.id,
                person.email,
                person.name,
                person.organization,
                person.role,
                person.relationship,
                person.notes,
                person.tracker_path,
                person.last_seen,
                person.first_seen,
                person.meeting_count,
                person.updated_at,
            ],
        )?;
        // Mirror to entities table (bridge pattern, like ensure_entity_for_account)
        self.ensure_entity_for_person(person)?;
        Ok(())
    }

    /// Mirror a person to the entities table.
    fn ensure_entity_for_person(&self, person: &DbPerson) -> Result<(), DbError> {
        let entity = crate::entity::DbEntity {
            id: person.id.clone(),
            name: person.name.clone(),
            entity_type: crate::entity::EntityType::Person,
            tracker_path: person.tracker_path.clone(),
            updated_at: person.updated_at.clone(),
        };
        self.upsert_entity(&entity)
    }

    /// Look up a person by email (case-insensitive).
    pub fn get_person_by_email(&self, email: &str) -> Result<Option<DbPerson>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, email, name, organization, role, relationship, notes,
                    tracker_path, last_seen, first_seen, meeting_count, updated_at
             FROM people WHERE email = LOWER(?1)",
        )?;
        let mut rows = stmt.query_map(params![email], Self::map_person_row)?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// Get a person by ID.
    pub fn get_person(&self, id: &str) -> Result<Option<DbPerson>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, email, name, organization, role, relationship, notes,
                    tracker_path, last_seen, first_seen, meeting_count, updated_at
             FROM people WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], Self::map_person_row)?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// Get all people, optionally filtered by relationship.
    pub fn get_people(&self, relationship: Option<&str>) -> Result<Vec<DbPerson>, DbError> {
        let people = match relationship {
            Some(rel) => {
                let mut stmt = self.conn.prepare(
                    "SELECT id, email, name, organization, role, relationship, notes,
                            tracker_path, last_seen, first_seen, meeting_count, updated_at
                     FROM people WHERE relationship = ?1 ORDER BY name",
                )?;
                let rows = stmt.query_map(params![rel], Self::map_person_row)?;
                rows.collect::<Result<Vec<_>, _>>()?
            }
            None => {
                let mut stmt = self.conn.prepare(
                    "SELECT id, email, name, organization, role, relationship, notes,
                            tracker_path, last_seen, first_seen, meeting_count, updated_at
                     FROM people ORDER BY name",
                )?;
                let rows = stmt.query_map([], Self::map_person_row)?;
                rows.collect::<Result<Vec<_>, _>>()?
            }
        };
        Ok(people)
    }

    /// Get people linked to an entity (account/project).
    pub fn get_people_for_entity(&self, entity_id: &str) -> Result<Vec<DbPerson>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT p.id, p.email, p.name, p.organization, p.role, p.relationship, p.notes,
                    p.tracker_path, p.last_seen, p.first_seen, p.meeting_count, p.updated_at
             FROM people p
             JOIN entity_people ep ON p.id = ep.person_id
             WHERE ep.entity_id = ?1
             ORDER BY p.name",
        )?;
        let rows = stmt.query_map(params![entity_id], Self::map_person_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Get entities linked to a person.
    pub fn get_entities_for_person(
        &self,
        person_id: &str,
    ) -> Result<Vec<crate::entity::DbEntity>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT e.id, e.name, e.entity_type, e.tracker_path, e.updated_at
             FROM entities e
             JOIN entity_people ep ON e.id = ep.entity_id
             WHERE ep.person_id = ?1
             ORDER BY e.name",
        )?;
        let rows = stmt.query_map(params![person_id], |row| {
            let et: String = row.get(2)?;
            Ok(crate::entity::DbEntity {
                id: row.get(0)?,
                name: row.get(1)?,
                entity_type: crate::entity::EntityType::from_str_lossy(&et),
                tracker_path: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Link a person to an entity (account/project). Idempotent.
    pub fn link_person_to_entity(
        &self,
        person_id: &str,
        entity_id: &str,
        rel: &str,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT OR IGNORE INTO entity_people (entity_id, person_id, relationship_type)
             VALUES (?1, ?2, ?3)",
            params![entity_id, person_id, rel],
        )?;
        Ok(())
    }

    /// Unlink a person from an entity.
    pub fn unlink_person_from_entity(
        &self,
        person_id: &str,
        entity_id: &str,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "DELETE FROM entity_people WHERE entity_id = ?1 AND person_id = ?2",
            params![entity_id, person_id],
        )?;
        Ok(())
    }

    /// Record that a person attended a meeting. Idempotent.
    /// Also updates `people.meeting_count` and `people.last_seen`.
    pub fn record_meeting_attendance(
        &self,
        meeting_id: &str,
        person_id: &str,
    ) -> Result<(), DbError> {
        // Insert attendance record (idempotent)
        let inserted = self.conn.execute(
            "INSERT OR IGNORE INTO meeting_attendees (meeting_id, person_id)
             VALUES (?1, ?2)",
            params![meeting_id, person_id],
        )?;

        // Only update meeting_count if we actually inserted a new row
        if inserted > 0 {
            // Get the meeting's start_time to update last_seen
            let start_time: Option<String> = self
                .conn
                .query_row(
                    "SELECT start_time FROM meetings_history WHERE id = ?1",
                    params![meeting_id],
                    |row| row.get(0),
                )
                .ok();

            if let Some(ref st) = start_time {
                self.conn.execute(
                    "UPDATE people SET
                        meeting_count = meeting_count + 1,
                        last_seen = CASE
                            WHEN ?1 > COALESCE(last_seen, '') THEN ?1
                            ELSE last_seen
                        END,
                        updated_at = ?2
                     WHERE id = ?3",
                    params![st, Utc::now().to_rfc3339(), person_id],
                )?;
            }
        }
        Ok(())
    }

    /// Get people who attended a meeting.
    pub fn get_meeting_attendees(&self, meeting_id: &str) -> Result<Vec<DbPerson>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT p.id, p.email, p.name, p.organization, p.role, p.relationship, p.notes,
                    p.tracker_path, p.last_seen, p.first_seen, p.meeting_count, p.updated_at
             FROM people p
             JOIN meeting_attendees ma ON p.id = ma.person_id
             WHERE ma.meeting_id = ?1
             ORDER BY p.name",
        )?;
        let rows = stmt.query_map(params![meeting_id], Self::map_person_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Get meetings a person attended, most recent first.
    pub fn get_person_meetings(
        &self,
        person_id: &str,
        limit: i32,
    ) -> Result<Vec<DbMeeting>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT m.id, m.title, m.meeting_type, m.start_time, m.end_time,
                    m.account_id, m.attendees, m.notes_path, m.summary, m.created_at,
                    m.calendar_event_id
             FROM meetings_history m
             JOIN meeting_attendees ma ON m.id = ma.meeting_id
             WHERE ma.person_id = ?1
             ORDER BY m.start_time DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![person_id, limit], |row| {
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
                calendar_event_id: row.get(10)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Compute person-level signals (meeting frequency, temperature, trend).
    pub fn get_person_signals(&self, person_id: &str) -> Result<PersonSignals, DbError> {
        let count_30d: i32 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM meetings_history m
                 JOIN meeting_attendees ma ON m.id = ma.meeting_id
                 WHERE ma.person_id = ?1
                   AND m.start_time >= date('now', '-30 days')",
                params![person_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let count_90d: i32 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM meetings_history m
                 JOIN meeting_attendees ma ON m.id = ma.meeting_id
                 WHERE ma.person_id = ?1
                   AND m.start_time >= date('now', '-90 days')",
                params![person_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let last_meeting: Option<String> = self
            .conn
            .query_row(
                "SELECT MAX(m.start_time) FROM meetings_history m
                 JOIN meeting_attendees ma ON m.id = ma.meeting_id
                 WHERE ma.person_id = ?1",
                params![person_id],
                |row| row.get(0),
            )
            .unwrap_or(None);

        let temperature = match &last_meeting {
            Some(dt) => compute_temperature(dt),
            None => "cold".to_string(),
        };
        let trend = compute_trend(count_30d, count_90d);

        Ok(PersonSignals {
            meeting_frequency_30d: count_30d,
            meeting_frequency_90d: count_90d,
            last_meeting,
            temperature,
            trend,
        })
    }

    /// Search people by name, email, or organization.
    pub fn search_people(&self, query: &str, limit: i32) -> Result<Vec<DbPerson>, DbError> {
        let pattern = format!("%{query}%");
        let mut stmt = self.conn.prepare(
            "SELECT id, email, name, organization, role, relationship, notes,
                    tracker_path, last_seen, first_seen, meeting_count, updated_at
             FROM people
             WHERE name LIKE ?1 OR email LIKE ?1 OR organization LIKE ?1
             ORDER BY name
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![pattern, limit], Self::map_person_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Update a single whitelisted field on a person.
    pub fn update_person_field(
        &self,
        id: &str,
        field: &str,
        value: &str,
    ) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        // Whitelist fields to prevent SQL injection
        let sql = match field {
            "notes" => "UPDATE people SET notes = ?1, updated_at = ?3 WHERE id = ?2",
            "role" => "UPDATE people SET role = ?1, updated_at = ?3 WHERE id = ?2",
            "organization" => {
                "UPDATE people SET organization = ?1, updated_at = ?3 WHERE id = ?2"
            }
            "relationship" => {
                "UPDATE people SET relationship = ?1, updated_at = ?3 WHERE id = ?2"
            }
            _ => return Err(DbError::Sqlite(rusqlite::Error::InvalidParameterName(
                format!("Field '{}' is not updatable", field),
            ))),
        };
        self.conn.execute(sql, params![value, id, now])?;
        Ok(())
    }

    /// Helper: map a row to `DbPerson`.
    fn map_person_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DbPerson> {
        Ok(DbPerson {
            id: row.get(0)?,
            email: row.get(1)?,
            name: row.get(2)?,
            organization: row.get(3)?,
            role: row.get(4)?,
            relationship: row.get::<_, String>(5)?,
            notes: row.get(6)?,
            tracker_path: row.get(7)?,
            last_seen: row.get(8)?,
            first_seen: row.get(9)?,
            meeting_count: row.get(10)?,
            updated_at: row.get(11)?,
        })
    }

    /// Helper: map a row to `DbAction`. Reduces repetition across queries.
    fn map_action_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DbAction> {
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
            calendar_event_id: Some("gcal-evt-001".to_string()),
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
                calendar_event_id: None,
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
    fn test_mark_prep_reviewed() {
        let db = test_db();

        db.mark_prep_reviewed("preps/0900-acme-sync.json", Some("gcal-evt-1"), "Acme Sync")
            .expect("mark reviewed");

        let reviewed = db.get_reviewed_preps().expect("get reviewed");
        assert_eq!(reviewed.len(), 1);
        assert!(reviewed.contains_key("preps/0900-acme-sync.json"));
    }

    #[test]
    fn test_mark_prep_reviewed_upsert() {
        let db = test_db();

        db.mark_prep_reviewed("preps/0900-acme.json", None, "Acme")
            .expect("first mark");
        db.mark_prep_reviewed("preps/0900-acme.json", Some("evt-1"), "Acme")
            .expect("second mark (upsert)");

        let reviewed = db.get_reviewed_preps().expect("get reviewed");
        assert_eq!(reviewed.len(), 1);
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

        let results = db
            .get_non_briefing_pending_actions()
            .expect("query");
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
        db.insert_capture("mtg-1", "Acme QBR", Some("acme"), "win", "Expanded deployment")
            .expect("insert capture 1");
        db.insert_capture("mtg-1", "Acme QBR", Some("acme"), "risk", "Budget freeze")
            .expect("insert capture 2");
        db.insert_capture("mtg-2", "Beta Sync", Some("beta"), "win", "New champion identified")
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
            ring: Some(1),
            arr: None,
            health: None,
            contract_start: None,
            contract_end: None,
            csm: None,
            champion: None,
            tracker_path: None,
            updated_at: "2020-01-01T00:00:00Z".to_string(),
        };
        db.upsert_account(&account).expect("upsert");

        // Touch by name (case-insensitive)
        let matched = db
            .touch_account_last_contact("acme corp")
            .expect("touch");
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
            ring: None,
            arr: None,
            health: None,
            contract_start: None,
            contract_end: None,
            csm: None,
            champion: None,
            tracker_path: None,
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
        let matched = db
            .touch_account_last_contact("nonexistent")
            .expect("touch");
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
        let no_match = db
            .touch_entity_last_contact("nonexistent")
            .expect("touch");
        assert!(!no_match);
    }

    #[test]
    fn test_ensure_entity_for_account() {
        let db = test_db();

        let account = DbAccount {
            id: "beta-inc".to_string(),
            name: "Beta Inc".to_string(),
            ring: Some(2),
            arr: Some(50_000.0),
            health: Some("yellow".to_string()),
            contract_start: None,
            contract_end: None,
            csm: None,
            champion: None,
            tracker_path: Some("Accounts/beta-inc".to_string()),
            updated_at: "2025-06-01T00:00:00Z".to_string(),
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
    fn test_backfill_migration_populates_entities() {
        // Create a DB, insert an account directly (bypassing the bridge),
        // then re-open to trigger the backfill migration.
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("backfill_test.db");
        std::mem::forget(dir);

        // First open: create DB and insert an account via raw SQL
        // (simulating pre-ADR-0045 state)
        {
            let conn = rusqlite::Connection::open(&path).expect("open");
            conn.execute_batch("PRAGMA journal_mode=WAL;").expect("wal");
            conn.execute_batch(include_str!("schema.sql")).expect("schema");
            conn.execute(
                "INSERT INTO accounts (id, name, ring, tracker_path, updated_at)
                 VALUES ('legacy-acct', 'Legacy Corp', 1, 'Accounts/legacy', '2025-01-01T00:00:00Z')",
                [],
            )
            .expect("insert legacy account");
        }

        // Second open via ActionDb: backfill migration should run
        let db = ActionDb::open_at(path).expect("reopen");
        let entity = db.get_entity("legacy-acct").expect("get entity");
        assert!(entity.is_some(), "Backfill should create entity from account");
        let e = entity.unwrap();
        assert_eq!(e.name, "Legacy Corp");
        assert_eq!(e.entity_type, EntityType::Account);
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
            ring: Some(1),
            arr: Some(100_000.0),
            health: Some("green".to_string()),
            contract_start: Some("2025-01-01".to_string()),
            contract_end: Some(
                (Utc::now() + chrono::Duration::days(30))
                    .format("%Y-%m-%d")
                    .to_string(),
            ),
            csm: None,
            champion: None,
            tracker_path: None,
            updated_at: Utc::now().to_rfc3339(),
        };
        db.upsert_account(&soon).expect("insert");

        // Account with no contract_end (should NOT appear)
        let no_end = DbAccount {
            id: "no-end".to_string(),
            name: "No End Corp".to_string(),
            ring: Some(2),
            arr: None,
            health: None,
            contract_start: None,
            contract_end: None,
            csm: None,
            champion: None,
            tracker_path: None,
            updated_at: Utc::now().to_rfc3339(),
        };
        db.upsert_account(&no_end).expect("insert");

        // Account already expired (should NOT appear — contract_end < now)
        let expired = DbAccount {
            id: "expired".to_string(),
            name: "Expired Corp".to_string(),
            ring: Some(3),
            arr: None,
            health: None,
            contract_start: None,
            contract_end: Some("2020-01-01".to_string()),
            csm: None,
            champion: None,
            tracker_path: None,
            updated_at: Utc::now().to_rfc3339(),
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
            ring: Some(2),
            arr: None,
            health: None,
            contract_start: None,
            contract_end: None,
            csm: None,
            champion: None,
            tracker_path: None,
            updated_at: "2020-01-01T00:00:00Z".to_string(),
        };
        db.upsert_account(&stale).expect("insert");

        // Account updated just now (should NOT be stale)
        let fresh = DbAccount {
            id: "fresh-acct".to_string(),
            name: "Fresh Corp".to_string(),
            ring: Some(1),
            arr: None,
            health: None,
            contract_start: None,
            contract_end: None,
            csm: None,
            champion: None,
            tracker_path: None,
            updated_at: Utc::now().to_rfc3339(),
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
            let meeting = DbMeeting {
                id: format!("mtg-{}", i),
                title: format!("Sync #{}", i),
                meeting_type: "customer".to_string(),
                start_time: (now - chrono::Duration::days(i * 5)).to_rfc3339(),
                end_time: None,
                account_id: Some("acme-corp".to_string()),
                attendees: None,
                notes_path: None,
                summary: None,
                created_at: now.to_rfc3339(),
                calendar_event_id: None,
            };
            db.upsert_meeting(&meeting).expect("insert meeting");
        }

        let signals = db
            .get_stakeholder_signals("acme-corp")
            .expect("signals");
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
            ring: None,
            arr: None,
            health: None,
            contract_start: None,
            contract_end: None,
            csm: None,
            champion: None,
            tracker_path: None,
            updated_at: Utc::now().to_rfc3339(),
        };
        db.upsert_account(&account).expect("insert account");

        let signals = db
            .get_stakeholder_signals("acme-corp")
            .expect("signals");
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

        let result = db.get_person_by_email("BOB@EXAMPLE.COM").expect("get by email");
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, person.id);
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
    fn test_person_entity_linking() {
        let db = test_db();
        let person = sample_person("jane@acme.com");
        db.upsert_person(&person).expect("upsert person");

        let account = DbAccount {
            id: "acme-corp".to_string(),
            name: "Acme Corp".to_string(),
            ring: None,
            arr: None,
            health: None,
            contract_start: None,
            contract_end: None,
            csm: None,
            champion: None,
            tracker_path: None,
            updated_at: Utc::now().to_rfc3339(),
        };
        db.upsert_account(&account).expect("upsert account");

        db.link_person_to_entity(&person.id, "acme-corp", "associated")
            .expect("link");

        let people = db.get_people_for_entity("acme-corp").expect("people for entity");
        assert_eq!(people.len(), 1);
        assert_eq!(people[0].id, person.id);

        let entities = db.get_entities_for_person(&person.id).expect("entities for person");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].id, "acme-corp");

        // Unlink
        db.unlink_person_from_entity(&person.id, "acme-corp").expect("unlink");
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
            account_id: None,
            attendees: None,
            notes_path: None,
            summary: None,
            created_at: now,
            calendar_event_id: None,
        };
        db.upsert_meeting(&meeting).expect("upsert meeting");
        db.record_meeting_attendance("mtg-attend-001", &person.id)
            .expect("record attendance");

        // Check attendees for meeting
        let attendees = db.get_meeting_attendees("mtg-attend-001").expect("get attendees");
        assert_eq!(attendees.len(), 1);
        assert_eq!(attendees[0].id, person.id);

        // Check meetings for person
        let meetings = db.get_person_meetings(&person.id, 10).expect("person meetings");
        assert_eq!(meetings.len(), 1);
        assert_eq!(meetings[0].id, "mtg-attend-001");

        // Check meeting_count was incremented
        let updated = db.get_person(&person.id).expect("get updated").unwrap();
        assert_eq!(updated.meeting_count, 1);

        // Idempotent: recording again should not increment
        db.record_meeting_attendance("mtg-attend-001", &person.id).expect("re-record");
        let same = db.get_person(&person.id).expect("get same").unwrap();
        assert_eq!(same.meeting_count, 1);
    }

    #[test]
    fn test_search_people() {
        let db = test_db();
        db.upsert_person(&sample_person("alice@acme.com")).expect("upsert");
        db.upsert_person(&sample_person("bob@bigcorp.io")).expect("upsert");

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

        db.update_person_field(&person.id, "role", "VP Engineering").expect("update role");
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
            .query_row("SELECT COUNT(*) FROM meeting_attendees", [], |row| row.get(0))
            .expect("meeting_attendees table should exist");
        assert_eq!(count, 0);

        let count: i32 = db
            .conn
            .query_row("SELECT COUNT(*) FROM entity_people", [], |row| row.get(0))
            .expect("entity_people table should exist");
        assert_eq!(count, 0);
    }
}
