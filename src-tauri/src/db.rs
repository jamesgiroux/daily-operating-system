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
use serde::{Deserialize, Serialize};
use thiserror::Error;

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

/// Errors specific to database operations.
#[derive(Debug, Error)]
pub enum DbError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("Home directory not found")]
    HomeDirNotFound,

    #[error("Failed to create database directory: {0}")]
    CreateDir(std::io::Error),

    #[error("Schema migration failed: {0}")]
    Migration(String),
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
    pub person_id: Option<String>,
}

/// A row from the `accounts` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbAccount {
    pub id: String,
    pub name: String,
    pub lifecycle: Option<String>,
    pub arr: Option<f64>,
    pub health: Option<String>,
    pub contract_start: Option<String>,
    pub contract_end: Option<String>,
    pub nps: Option<i32>,
    pub tracker_path: Option<String>,
    pub parent_id: Option<String>,
    pub is_internal: bool,
    pub updated_at: String,
    pub archived: bool,
    /// JSON array of auto-extracted keywords for entity resolution (I305).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<String>,
    /// UTC timestamp when keywords were last extracted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords_extracted_at: Option<String>,
}

/// A row from `account_team`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbAccountTeamMember {
    pub account_id: String,
    pub person_id: String,
    pub person_name: String,
    pub person_email: String,
    pub role: String,
    pub created_at: String,
}

/// A row from `account_team_import_notes`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbAccountTeamImportNote {
    pub id: i64,
    pub account_id: String,
    pub legacy_field: String,
    pub legacy_value: String,
    pub note: String,
    pub created_at: String,
}

/// Aggregated signals for a parent account's children (I114).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParentAggregate {
    pub bu_count: usize,
    pub total_arr: Option<f64>,
    pub worst_health: Option<String>,
    pub nearest_renewal: Option<String>,
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
    /// Calendar event description (I185). Plumbed from Google Calendar API.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Enriched prep context JSON (I181). Only populated by get_meeting_by_id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prep_context_json: Option<String>,
    /// User-authored agenda items (JSON array).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agenda_json: Option<String>,
    /// User-authored notes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_notes: Option<String>,
    /// Frozen prep JSON captured during archive.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prep_frozen_json: Option<String>,
    /// UTC timestamp when prep was frozen.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prep_frozen_at: Option<String>,
    /// Absolute path to immutable prep snapshot JSON.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prep_snapshot_path: Option<String>,
    /// SHA-256 hash of snapshot payload.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prep_snapshot_hash: Option<String>,
    /// Absolute path to transcript destination.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transcript_path: Option<String>,
    /// UTC timestamp when transcript was processed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transcript_processed_at: Option<String>,
}

pub struct EnsureMeetingHistoryInput<'a> {
    pub id: &'a str,
    pub title: &'a str,
    pub meeting_type: &'a str,
    pub start_time: &'a str,
    pub end_time: Option<&'a str>,
    pub account_id: Option<&'a str>,
    pub calendar_event_id: Option<&'a str>,
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
    pub project_id: Option<String>,
    pub capture_type: String,
    pub content: String,
    pub captured_at: String,
}

/// Email-derived intelligence signal linked to an entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbEmailSignal {
    pub id: i64,
    pub email_id: String,
    pub sender_email: Option<String>,
    pub person_id: Option<String>,
    pub entity_id: String,
    pub entity_type: String,
    pub signal_type: String,
    pub signal_text: String,
    pub confidence: Option<f64>,
    pub sentiment: Option<String>,
    pub urgency: Option<String>,
    pub detected_at: String,
}

/// Stakeholder relationship signals computed from meeting history and account data (I43).
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub archived: bool,
    // Clay enrichment fields (I228)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linkedin_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub twitter_handle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub photo_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_history: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company_industry: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company_size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company_hq: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_enriched_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enrichment_sources: Option<String>,
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

/// Person with pre-computed signals for list pages (I106).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonListItem {
    pub id: String,
    pub email: String,
    pub name: String,
    pub organization: Option<String>,
    pub role: Option<String>,
    pub relationship: String,
    pub notes: Option<String>,
    pub tracker_path: Option<String>,
    pub last_seen: Option<String>,
    pub first_seen: Option<String>,
    pub meeting_count: i32,
    pub updated_at: String,
    pub archived: bool,
    pub temperature: String,
    pub trend: String,
    /// Comma-separated names of linked account entities (from entity_people).
    pub account_names: Option<String>,
}

/// A row from the `projects` table (I50).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbProject {
    pub id: String,
    pub name: String,
    pub status: String,
    pub milestone: Option<String>,
    pub owner: Option<String>,
    pub target_date: Option<String>,
    pub tracker_path: Option<String>,
    pub updated_at: String,
    pub archived: bool,
    /// JSON array of auto-extracted keywords for entity resolution (I305).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<String>,
    /// UTC timestamp when keywords were last extracted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords_extracted_at: Option<String>,
}

/// Activity signals for a project (parallel to StakeholderSignals).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSignals {
    pub meeting_frequency_30d: i32,
    pub meeting_frequency_90d: i32,
    pub last_meeting: Option<String>,
    pub days_until_target: Option<i64>,
    pub open_action_count: i32,
    pub temperature: String,
    pub trend: String,
}

/// A row from the `content_index` table (I124).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DbContentFile {
    pub id: String,
    pub entity_id: String,
    pub entity_type: String,
    pub filename: String,
    pub relative_path: String,
    pub absolute_path: String,
    pub format: String,
    pub file_size: i64,
    pub modified_at: String,
    pub indexed_at: String,
    pub extracted_at: Option<String>,
    pub summary: Option<String>,
    pub embeddings_generated_at: Option<String>,
    pub content_type: String,
    pub priority: i32,
}

/// A row from `content_embeddings`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbContentEmbedding {
    pub id: String,
    pub content_file_id: String,
    pub chunk_index: i32,
    pub chunk_text: String,
    pub embedding: Vec<u8>,
    pub created_at: String,
}

/// A row from `chat_sessions`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbChatSession {
    pub id: String,
    pub entity_id: Option<String>,
    pub entity_type: Option<String>,
    pub session_start: String,
    pub session_end: Option<String>,
    pub turn_count: i32,
    pub last_message: Option<String>,
    pub created_at: String,
}

/// A row from `chat_turns`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbChatTurn {
    pub id: String,
    pub session_id: String,
    pub turn_index: i32,
    pub role: String,
    pub content: String,
    pub timestamp: String,
}

/// A lifecycle event for an account (I143 — renewal tracking).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbAccountEvent {
    pub id: i64,
    pub account_id: String,
    pub event_type: String,
    pub event_date: String,
    pub arr_impact: Option<f64>,
    pub notes: Option<String>,
    pub created_at: String,
}

/// A row from the `quill_sync_state` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbQuillSyncState {
    pub id: String,
    pub meeting_id: String,
    pub quill_meeting_id: Option<String>,
    pub state: String,
    pub attempts: i32,
    pub max_attempts: i32,
    pub next_attempt_at: Option<String>,
    pub last_attempt_at: Option<String>,
    pub completed_at: Option<String>,
    pub error_message: Option<String>,
    pub match_confidence: Option<f64>,
    pub transcript_path: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default = "default_quill_source")]
    pub source: String,
}

fn default_quill_source() -> String {
    "quill".to_string()
}

/// Row mapper for quill_sync_state SELECT queries (15 columns including source).
fn map_sync_row(row: &rusqlite::Row) -> rusqlite::Result<DbQuillSyncState> {
    Ok(DbQuillSyncState {
        id: row.get(0)?,
        meeting_id: row.get(1)?,
        quill_meeting_id: row.get(2)?,
        state: row.get(3)?,
        attempts: row.get(4)?,
        max_attempts: row.get(5)?,
        next_attempt_at: row.get(6)?,
        last_attempt_at: row.get(7)?,
        completed_at: row.get(8)?,
        error_message: row.get(9)?,
        match_confidence: row.get(10)?,
        transcript_path: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
        source: row.get::<_, String>(14).unwrap_or_else(|_| "quill".to_string()),
    })
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
pub fn days_since_iso(iso: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(iso)
        .or_else(|_| {
            chrono::DateTime::parse_from_rfc3339(&format!("{}+00:00", iso.trim_end_matches('Z')))
        })
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
                         account_id = COALESCE(account_id, (SELECT account_id FROM meetings_history WHERE id = ?1)),
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
                    context, waiting_on, updated_at, person_id
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
                person_id: row.get(15)?,
            })
        })?;

        let mut actions = Vec::new();
        for row in rows {
            actions.push(row?);
        }
        Ok(actions)
    }

    /// Query pending + waiting actions for focus prioritization.
    ///
    /// Includes actions with no due date so the ranker can decide feasibility.
    /// Ordered by urgency first, then priority/due date.
    pub fn get_focus_candidate_actions(&self, days_ahead: i32) -> Result<Vec<DbAction>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, priority, status, created_at, due_date, completed_at,
                    account_id, project_id, source_type, source_id, source_label,
                    context, waiting_on, updated_at, person_id
             FROM actions
             WHERE status IN ('pending', 'waiting')
               AND (due_date IS NULL OR due_date <= date('now', ?1 || ' days'))
             ORDER BY
               CASE
                 WHEN due_date < date('now') THEN 0
                 WHEN due_date = date('now') THEN 1
                 WHEN due_date IS NULL THEN 3
                 ELSE 2
               END,
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
                person_id: row.get(15)?,
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
                    context, waiting_on, updated_at, person_id
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
                person_id: row.get(15)?,
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
                    context, waiting_on, updated_at, person_id
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
                person_id: row.get(15)?,
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
                    context, waiting_on, updated_at, person_id
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
                person_id: row.get(15)?,
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
                    context, waiting_on, updated_at, person_id
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
                person_id: row.get(15)?,
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
                context, waiting_on, updated_at, person_id
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
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
                updated_at = excluded.updated_at,
                person_id = excluded.person_id",
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
                action.person_id,
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
                    context, waiting_on, updated_at, person_id
             FROM actions
             WHERE status IN ('pending', 'waiting')
               AND source_type IN ('post_meeting', 'inbox', 'ai-inbox', 'transcript', 'import', 'manual')
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
                person_id: row.get(15)?,
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
    // Proposed Actions (I256)
    // =========================================================================

    /// Get all proposed actions.
    pub fn get_proposed_actions(&self) -> Result<Vec<DbAction>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, priority, status, created_at, due_date, completed_at,
                    account_id, project_id, source_type, source_id, source_label,
                    context, waiting_on, updated_at, person_id
             FROM actions
             WHERE status = 'proposed'
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
                person_id: row.get(15)?,
            })
        })?;

        let mut actions = Vec::new();
        for row in rows {
            actions.push(row?);
        }
        Ok(actions)
    }

    /// Accept a proposed action, moving it to pending status.
    pub fn accept_proposed_action(&self, id: &str) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        let changed = self.conn.execute(
            "UPDATE actions SET status = 'pending', updated_at = ?1
             WHERE id = ?2 AND status = 'proposed'",
            params![now, id],
        )?;
        if changed == 0 {
            return Err(DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows));
        }
        Ok(())
    }

    /// Reject a proposed action by archiving it and recording the rejection signal.
    pub fn reject_proposed_action(&self, id: &str) -> Result<(), DbError> {
        self.reject_proposed_action_with_source(id, "unknown")
    }

    /// Reject a proposed action, recording the source surface for correction learning.
    pub fn reject_proposed_action_with_source(
        &self,
        id: &str,
        source: &str,
    ) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        let changed = self.conn.execute(
            "UPDATE actions SET status = 'archived', updated_at = ?1,
             rejected_at = ?1, rejection_source = ?3
             WHERE id = ?2 AND status = 'proposed'",
            params![now, id, source],
        )?;
        if changed == 0 {
            return Err(DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows));
        }
        Ok(())
    }

    /// Archive an action (any status -> archived).
    pub fn archive_action(&self, id: &str) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE actions SET status = 'archived', updated_at = ?1
             WHERE id = ?2",
            params![now, id],
        )?;
        Ok(())
    }

    /// Auto-archive proposed actions older than N days.
    /// Returns the number of actions archived.
    pub fn auto_archive_old_proposed(&self, days: i64) -> Result<usize, DbError> {
        let now = Utc::now().to_rfc3339();
        let cutoff_param = format!("-{} days", days);
        let changed = self.conn.execute(
            "UPDATE actions SET status = 'archived', updated_at = ?1
             WHERE status = 'proposed'
               AND created_at < datetime('now', ?2)",
            params![now, cutoff_param],
        )?;
        Ok(changed)
    }

    // =========================================================================
    // Accounts
    // =========================================================================

    /// Insert or update an account. Also mirrors to the `entities` table (ADR-0045).
    pub fn upsert_account(&self, account: &DbAccount) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT INTO accounts (
                id, name, lifecycle, arr, health, contract_start, contract_end,
                nps, tracker_path, parent_id, is_internal, updated_at, archived,
                keywords, keywords_extracted_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                lifecycle = excluded.lifecycle,
                arr = excluded.arr,
                health = excluded.health,
                contract_start = excluded.contract_start,
                contract_end = excluded.contract_end,
                nps = excluded.nps,
                tracker_path = excluded.tracker_path,
                parent_id = excluded.parent_id,
                is_internal = excluded.is_internal,
                updated_at = excluded.updated_at",
            params![
                account.id,
                account.name,
                account.lifecycle,
                account.arr,
                account.health,
                account.contract_start,
                account.contract_end,
                account.nps,
                account.tracker_path,
                account.parent_id,
                account.is_internal as i32,
                account.updated_at,
                account.archived as i32,
                account.keywords,
                account.keywords_extracted_at,
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
            "SELECT id, name, lifecycle, arr, health, contract_start, contract_end,
                    nps, tracker_path, parent_id, is_internal, updated_at, archived,
                    keywords, keywords_extracted_at
             FROM accounts
             WHERE id = ?1",
        )?;

        let mut rows = stmt.query_map(params![id], Self::map_account_row)?;

        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// Get an account by name (case-insensitive).
    pub fn get_account_by_name(&self, name: &str) -> Result<Option<DbAccount>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, lifecycle, arr, health, contract_start, contract_end,
                    nps, tracker_path, parent_id, is_internal, updated_at, archived,
                    keywords, keywords_extracted_at
             FROM accounts
             WHERE LOWER(name) = LOWER(?1)",
        )?;

        let mut rows = stmt.query_map(params![name], Self::map_account_row)?;

        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// Get all accounts, ordered by name.
    pub fn get_all_accounts(&self) -> Result<Vec<DbAccount>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, lifecycle, arr, health, contract_start, contract_end,
                    nps, tracker_path, parent_id, is_internal, updated_at, archived,
                    keywords, keywords_extracted_at
             FROM accounts WHERE archived = 0 ORDER BY name",
        )?;
        let rows = stmt.query_map([], Self::map_account_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Get top-level accounts (no parent), ordered by name.
    pub fn get_top_level_accounts(&self) -> Result<Vec<DbAccount>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, lifecycle, arr, health, contract_start, contract_end,
                    nps, tracker_path, parent_id, is_internal, updated_at, archived,
                    keywords, keywords_extracted_at
             FROM accounts WHERE parent_id IS NULL AND archived = 0 ORDER BY name",
        )?;
        let rows = stmt.query_map([], Self::map_account_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Get child accounts for a parent, ordered by name.
    pub fn get_child_accounts(&self, parent_id: &str) -> Result<Vec<DbAccount>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, lifecycle, arr, health, contract_start, contract_end,
                    nps, tracker_path, parent_id, is_internal, updated_at, archived,
                    keywords, keywords_extracted_at
             FROM accounts WHERE parent_id = ?1 AND archived = 0 ORDER BY name",
        )?;
        let rows = stmt.query_map(params![parent_id], Self::map_account_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Set domains for an account (replace-all).
    pub fn set_account_domains(&self, account_id: &str, domains: &[String]) -> Result<(), DbError> {
        let normalized = crate::util::normalize_domains(domains);
        self.conn.execute(
            "DELETE FROM account_domains WHERE account_id = ?1",
            params![account_id],
        )?;
        for domain in normalized {
            self.conn.execute(
                "INSERT OR IGNORE INTO account_domains (account_id, domain) VALUES (?1, ?2)",
                params![account_id, &domain],
            )?;
        }
        Ok(())
    }

    /// Get account domains for an account.
    pub fn get_account_domains(&self, account_id: &str) -> Result<Vec<String>, DbError> {
        let mut stmt = self
            .conn
            .prepare("SELECT domain FROM account_domains WHERE account_id = ?1 ORDER BY domain")?;
        let rows = stmt.query_map(params![account_id], |row| row.get::<_, String>(0))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Get all accounts with their domains in a single JOIN query.
    ///
    /// Eliminates N+1 queries when callers need domains for many accounts.
    /// Returns `Vec<(DbAccount, Vec<String>)>` — each tuple is an account + its domains.
    pub fn get_all_accounts_with_domains(
        &self,
        include_archived: bool,
    ) -> Result<Vec<(DbAccount, Vec<String>)>, DbError> {
        let query = if include_archived {
            "SELECT a.id, a.name, a.lifecycle, a.arr, a.health, a.contract_start,
                    a.contract_end, a.nps, a.tracker_path, a.parent_id, a.is_internal,
                    a.updated_at, a.archived, a.keywords, a.keywords_extracted_at,
                    ad.domain
             FROM accounts a
             LEFT JOIN account_domains ad ON a.id = ad.account_id
             ORDER BY a.id, ad.domain"
        } else {
            "SELECT a.id, a.name, a.lifecycle, a.arr, a.health, a.contract_start,
                    a.contract_end, a.nps, a.tracker_path, a.parent_id, a.is_internal,
                    a.updated_at, a.archived, a.keywords, a.keywords_extracted_at,
                    ad.domain
             FROM accounts a
             LEFT JOIN account_domains ad ON a.id = ad.account_id
             WHERE a.archived = 0
             ORDER BY a.id, ad.domain"
        };

        let mut stmt = self.conn.prepare(query)?;
        let mut rows = stmt.query([])?;

        let mut result: Vec<(DbAccount, Vec<String>)> = Vec::new();
        let mut current_id: Option<String> = None;

        while let Some(row) = rows.next()? {
            let account_id: String = row.get(0)?;
            let domain: Option<String> = row.get(15)?;

            if current_id.as_deref() != Some(&account_id) {
                // New account — push a new entry
                let account = DbAccount {
                    id: account_id.clone(),
                    name: row.get(1)?,
                    lifecycle: row.get(2)?,
                    arr: row.get(3)?,
                    health: row.get(4)?,
                    contract_start: row.get(5)?,
                    contract_end: row.get(6)?,
                    nps: row.get(7)?,
                    tracker_path: row.get(8)?,
                    parent_id: row.get(9)?,
                    is_internal: row.get::<_, i32>(10).unwrap_or(0) != 0,
                    updated_at: row.get(11)?,
                    archived: row.get::<_, i32>(12).unwrap_or(0) != 0,
                    keywords: row.get(13).unwrap_or(None),
                    keywords_extracted_at: row.get(14).unwrap_or(None),
                };
                let domains = domain.into_iter().collect();
                result.push((account, domains));
                current_id = Some(account_id);
            } else if let Some(d) = domain {
                // Same account — append domain
                if let Some(last) = result.last_mut() {
                    last.1.push(d);
                }
            }
        }

        Ok(result)
    }

    /// Lookup non-archived account candidates by email domain.
    pub fn lookup_account_candidates_by_domain(
        &self,
        domain: &str,
    ) -> Result<Vec<DbAccount>, DbError> {
        let normalized = domain.trim().to_lowercase();
        if normalized.is_empty() {
            return Ok(Vec::new());
        }

        let mut stmt = self.conn.prepare(
            "SELECT a.id, a.name, a.lifecycle, a.arr, a.health, a.contract_start, a.contract_end,
                    a.nps, a.tracker_path, a.parent_id, a.is_internal,
                    a.updated_at, a.archived
             FROM accounts a
             INNER JOIN account_domains d ON d.account_id = a.id
             WHERE d.domain = ?1
               AND a.archived = 0
             ORDER BY a.is_internal ASC, a.name ASC",
        )?;
        let rows = stmt.query_map(params![normalized], Self::map_account_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Copy domains from parent to child (idempotent).
    pub fn copy_account_domains(&self, parent_id: &str, child_id: &str) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT OR IGNORE INTO account_domains (account_id, domain)
             SELECT ?1, domain FROM account_domains WHERE account_id = ?2",
            params![child_id, parent_id],
        )?;
        Ok(())
    }

    /// Root internal organization account (top-level internal account).
    pub fn get_internal_root_account(&self) -> Result<Option<DbAccount>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, lifecycle, arr, health, contract_start, contract_end,
                    nps, tracker_path, parent_id, is_internal, updated_at, archived,
                    keywords, keywords_extracted_at
             FROM accounts
             WHERE is_internal = 1 AND parent_id IS NULL AND archived = 0
             ORDER BY updated_at DESC
             LIMIT 1",
        )?;
        let mut rows = stmt.query_map([], Self::map_account_row)?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// All active internal accounts.
    pub fn get_internal_accounts(&self) -> Result<Vec<DbAccount>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, lifecycle, arr, health, contract_start, contract_end,
                    nps, tracker_path, parent_id, is_internal, updated_at, archived,
                    keywords, keywords_extracted_at
             FROM accounts
             WHERE is_internal = 1 AND archived = 0
             ORDER BY name",
        )?;
        let rows = stmt.query_map([], Self::map_account_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Get account team members with person details.
    pub fn get_account_team(&self, account_id: &str) -> Result<Vec<DbAccountTeamMember>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT at.account_id, at.person_id, p.name, p.email, at.role, at.created_at
             FROM account_team at
             JOIN people p ON p.id = at.person_id
             WHERE at.account_id = ?1
             ORDER BY at.role, p.name",
        )?;
        let rows = stmt.query_map(params![account_id], |row| {
            Ok(DbAccountTeamMember {
                account_id: row.get(0)?,
                person_id: row.get(1)?,
                person_name: row.get(2)?,
                person_email: row.get(3)?,
                role: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Add an account team member role link (idempotent).
    pub fn add_account_team_member(
        &self,
        account_id: &str,
        person_id: &str,
        role: &str,
    ) -> Result<(), DbError> {
        let role = role.trim().to_lowercase();
        self.conn.execute(
            "INSERT OR IGNORE INTO account_team (account_id, person_id, role, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![account_id, person_id, role, Utc::now().to_rfc3339()],
        )?;
        self.conn.execute(
            "INSERT OR IGNORE INTO entity_people (entity_id, person_id, relationship_type)
             VALUES (?1, ?2, 'associated')",
            params![account_id, person_id],
        )?;
        Ok(())
    }

    /// Remove an account team role link.
    /// If no roles remain for this person on this account, also removes the entity_people link.
    pub fn remove_account_team_member(
        &self,
        account_id: &str,
        person_id: &str,
        role: &str,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "DELETE FROM account_team
             WHERE account_id = ?1 AND person_id = ?2 AND LOWER(role) = LOWER(?3)",
            params![account_id, person_id, role.trim()],
        )?;

        // Clean up entity_people link if no roles remain
        let remaining_roles: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM account_team
             WHERE account_id = ?1 AND person_id = ?2",
            params![account_id, person_id],
            |row| row.get(0),
        )?;

        if remaining_roles == 0 {
            self.conn.execute(
                "DELETE FROM entity_people
                 WHERE entity_id = ?1 AND person_id = ?2",
                params![account_id, person_id],
            )?;
        }

        Ok(())
    }

    /// Import notes from migration for unmatched legacy account-team fields.
    pub fn get_account_team_import_notes(
        &self,
        account_id: &str,
    ) -> Result<Vec<DbAccountTeamImportNote>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, account_id, legacy_field, legacy_value, note, created_at
             FROM account_team_import_notes
             WHERE account_id = ?1
             ORDER BY id",
        )?;
        let rows = stmt.query_map(params![account_id], |row| {
            Ok(DbAccountTeamImportNote {
                id: row.get(0)?,
                account_id: row.get(1)?,
                legacy_field: row.get(2)?,
                legacy_value: row.get(3)?,
                note: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Aggregate child account signals for a parent account (I114).
    ///
    /// Returns total ARR, worst health, nearest renewal, and BU count.
    pub fn get_parent_aggregate(&self, parent_id: &str) -> Result<ParentAggregate, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT COUNT(*), COALESCE(SUM(arr), 0),
                    MIN(CASE health WHEN 'red' THEN 0 WHEN 'yellow' THEN 1 WHEN 'green' THEN 2 ELSE 3 END),
                    MIN(contract_end)
             FROM accounts WHERE parent_id = ?1",
        )?;
        let row = stmt.query_row(params![parent_id], |row| {
            let bu_count: usize = row.get(0)?;
            let total_arr: f64 = row.get(1)?;
            let worst_health_int: i32 = row.get(2)?;
            let nearest_renewal: Option<String> = row.get(3)?;
            Ok(ParentAggregate {
                bu_count,
                total_arr: if total_arr > 0.0 {
                    Some(total_arr)
                } else {
                    None
                },
                worst_health: match worst_health_int {
                    0 => Some("red".to_string()),
                    1 => Some("yellow".to_string()),
                    2 => Some("green".to_string()),
                    _ => None,
                },
                nearest_renewal,
            })
        })?;
        Ok(row)
    }

    /// Get meetings for an account, most recent first.
    pub fn get_meetings_for_account(
        &self,
        account_id: &str,
        limit: i32,
    ) -> Result<Vec<DbMeeting>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, meeting_type, start_time, end_time,
                    account_id, attendees, notes_path, summary, created_at,
                    calendar_event_id
             FROM meetings_history
             WHERE account_id = ?1
             ORDER BY start_time DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![account_id, limit], |row| {
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
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Get past meetings for an account with prep context (ADR-0063).
    /// Used only on account detail page where prep preview cards are needed.
    pub fn get_meetings_for_account_with_prep(
        &self,
        account_id: &str,
        limit: i32,
    ) -> Result<Vec<DbMeeting>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT m.id, m.title, m.meeting_type, m.start_time, m.end_time,
                    m.account_id, m.attendees, m.notes_path, m.summary, m.created_at,
                    m.calendar_event_id, m.prep_context_json
             FROM meetings_history m
             INNER JOIN meeting_entities me ON m.id = me.meeting_id
             WHERE me.entity_id = ?1 AND me.entity_type = 'account'
             ORDER BY m.start_time DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![account_id, limit], |row| {
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
                description: None,
                prep_context_json: row.get(11)?,
                user_agenda_json: None,
                user_notes: None,
                prep_frozen_json: None,
                prep_frozen_at: None,
                prep_snapshot_path: None,
                prep_snapshot_hash: None,
                transcript_path: None,
                transcript_processed_at: None,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Get upcoming (future) meetings for an account, soonest first.
    pub fn get_upcoming_meetings_for_account(
        &self,
        account_id: &str,
        limit: i32,
    ) -> Result<Vec<DbMeeting>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT m.id, m.title, m.meeting_type, m.start_time, m.end_time,
                    m.account_id, m.attendees, m.notes_path, m.summary, m.created_at,
                    m.calendar_event_id
             FROM meetings_history m
             INNER JOIN meeting_entities me ON m.id = me.meeting_id
             WHERE me.entity_id = ?1 AND me.entity_type = 'account'
               AND m.start_time >= datetime('now')
             ORDER BY m.start_time ASC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![account_id, limit], |row| {
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
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Update a single whitelisted field on an account.
    pub fn update_account_field(&self, id: &str, field: &str, value: &str) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        let sql = match field {
            "name" => "UPDATE accounts SET name = ?1, updated_at = ?3 WHERE id = ?2",
            "health" => "UPDATE accounts SET health = ?1, updated_at = ?3 WHERE id = ?2",
            "lifecycle" => "UPDATE accounts SET lifecycle = ?1, updated_at = ?3 WHERE id = ?2",
            "arr" => "UPDATE accounts SET arr = CAST(?1 AS REAL), updated_at = ?3 WHERE id = ?2",
            "nps" => "UPDATE accounts SET nps = CAST(?1 AS INTEGER), updated_at = ?3 WHERE id = ?2",
            "contract_start" => {
                "UPDATE accounts SET contract_start = ?1, updated_at = ?3 WHERE id = ?2"
            }
            "contract_end" => {
                "UPDATE accounts SET contract_end = ?1, updated_at = ?3 WHERE id = ?2"
            }
            _ => {
                return Err(DbError::Sqlite(rusqlite::Error::InvalidParameterName(
                    format!("Field '{}' is not updatable", field),
                )))
            }
        };
        self.conn.execute(sql, params![value, id, now])?;
        Ok(())
    }

    // =========================================================================
    // Content Index (I124)
    // =========================================================================

    /// Upsert a content file record. Preserves existing `extracted_at` / `summary`
    /// when the incoming record has `None` for those fields (COALESCE pattern).
    pub fn upsert_content_file(&self, file: &DbContentFile) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT INTO content_index (
                id, entity_id, entity_type, filename, relative_path, absolute_path,
                format, file_size, modified_at, indexed_at, extracted_at, summary,
                embeddings_generated_at, content_type, priority
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
             ON CONFLICT(id) DO UPDATE SET
                filename = excluded.filename,
                relative_path = excluded.relative_path,
                absolute_path = excluded.absolute_path,
                format = excluded.format,
                file_size = excluded.file_size,
                modified_at = excluded.modified_at,
                indexed_at = excluded.indexed_at,
                extracted_at = COALESCE(excluded.extracted_at, content_index.extracted_at),
                summary = COALESCE(excluded.summary, content_index.summary),
                embeddings_generated_at = excluded.embeddings_generated_at,
                content_type = excluded.content_type,
                priority = excluded.priority",
            params![
                file.id,
                file.entity_id,
                file.entity_type,
                file.filename,
                file.relative_path,
                file.absolute_path,
                file.format,
                file.file_size,
                file.modified_at,
                file.indexed_at,
                file.extracted_at,
                file.summary,
                file.embeddings_generated_at,
                file.content_type,
                file.priority,
            ],
        )?;
        Ok(())
    }

    /// Get all indexed files for an entity, highest priority first, then most recently modified.
    pub fn get_entity_files(&self, entity_id: &str) -> Result<Vec<DbContentFile>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, entity_id, entity_type, filename, relative_path, absolute_path,
                    format, file_size, modified_at, indexed_at, extracted_at, summary,
                    embeddings_generated_at, content_type, priority
             FROM content_index WHERE entity_id = ?1
             ORDER BY priority DESC, modified_at DESC",
        )?;
        let rows = stmt.query_map(params![entity_id], |row| {
            Ok(DbContentFile {
                id: row.get(0)?,
                entity_id: row.get(1)?,
                entity_type: row.get(2)?,
                filename: row.get(3)?,
                relative_path: row.get(4)?,
                absolute_path: row.get(5)?,
                format: row.get(6)?,
                file_size: row.get(7)?,
                modified_at: row.get(8)?,
                indexed_at: row.get(9)?,
                extracted_at: row.get(10)?,
                summary: row.get(11)?,
                embeddings_generated_at: row.get(12)?,
                content_type: row.get(13)?,
                priority: row.get(14)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Delete a single content file record by ID.
    pub fn delete_content_file(&self, id: &str) -> Result<(), DbError> {
        self.conn
            .execute("DELETE FROM content_index WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Delete all content file records for an entity.
    pub fn delete_entity_files(&self, entity_id: &str) -> Result<(), DbError> {
        self.conn.execute(
            "DELETE FROM content_index WHERE entity_id = ?1",
            params![entity_id],
        )?;
        Ok(())
    }

    /// Update extraction results for a content file: summary, content_type, and priority.
    pub fn update_content_extraction(
        &self,
        id: &str,
        extracted_at: &str,
        summary: Option<&str>,
        content_type: Option<&str>,
        priority: Option<i32>,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "UPDATE content_index SET extracted_at = ?1, summary = ?2,
                    content_type = COALESCE(?3, content_type),
                    priority = COALESCE(?4, priority)
             WHERE id = ?5",
            params![extracted_at, summary, content_type, priority, id],
        )?;
        Ok(())
    }

    /// Update the embeddings watermark for a content file.
    pub fn set_embeddings_generated_at(
        &self,
        id: &str,
        embeddings_generated_at: Option<&str>,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "UPDATE content_index
             SET embeddings_generated_at = ?1
             WHERE id = ?2",
            params![embeddings_generated_at, id],
        )?;
        Ok(())
    }

    /// Replace all embedding chunks for a file atomically.
    pub fn replace_content_embeddings_for_file(
        &self,
        content_file_id: &str,
        chunks: &[DbContentEmbedding],
    ) -> Result<(), DbError> {
        self.with_transaction(|tx| {
            tx.conn
                .execute(
                    "DELETE FROM content_embeddings WHERE content_file_id = ?1",
                    params![content_file_id],
                )
                .map_err(|e| format!("failed deleting prior embeddings: {e}"))?;

            for chunk in chunks {
                tx.conn
                    .execute(
                        "INSERT INTO content_embeddings (
                            id, content_file_id, chunk_index, chunk_text, embedding, created_at
                         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        params![
                            chunk.id,
                            chunk.content_file_id,
                            chunk.chunk_index,
                            chunk.chunk_text,
                            chunk.embedding,
                            chunk.created_at,
                        ],
                    )
                    .map_err(|e| format!("failed inserting content embedding: {e}"))?;
            }

            Ok(())
        })
        .map_err(DbError::Migration)?;

        Ok(())
    }

    /// Files requiring embedding generation.
    pub fn get_files_needing_embeddings(
        &self,
        limit: usize,
    ) -> Result<Vec<DbContentFile>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, entity_id, entity_type, filename, relative_path, absolute_path,
                    format, file_size, modified_at, indexed_at, extracted_at, summary,
                    embeddings_generated_at, content_type, priority
             FROM content_index
             WHERE embeddings_generated_at IS NULL OR embeddings_generated_at < modified_at
             ORDER BY modified_at DESC
             LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            Ok(DbContentFile {
                id: row.get(0)?,
                entity_id: row.get(1)?,
                entity_type: row.get(2)?,
                filename: row.get(3)?,
                relative_path: row.get(4)?,
                absolute_path: row.get(5)?,
                format: row.get(6)?,
                file_size: row.get(7)?,
                modified_at: row.get(8)?,
                indexed_at: row.get(9)?,
                extracted_at: row.get(10)?,
                summary: row.get(11)?,
                embeddings_generated_at: row.get(12)?,
                content_type: row.get(13)?,
                priority: row.get(14)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Returns all embedding chunks for an entity.
    pub fn get_entity_embedding_chunks(
        &self,
        entity_id: &str,
    ) -> Result<Vec<DbContentEmbedding>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT ce.id, ce.content_file_id, ce.chunk_index, ce.chunk_text, ce.embedding, ce.created_at
             FROM content_embeddings ce
             JOIN content_index ci ON ci.id = ce.content_file_id
             WHERE ci.entity_id = ?1
             ORDER BY ce.chunk_index ASC",
        )?;
        let rows = stmt.query_map(params![entity_id], |row| {
            Ok(DbContentEmbedding {
                id: row.get(0)?,
                content_file_id: row.get(1)?,
                chunk_index: row.get(2)?,
                chunk_text: row.get(3)?,
                embedding: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Returns all entities that have at least one indexed content file.
    pub fn get_entities_with_content(&self) -> Result<Vec<(String, String)>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT entity_id, entity_type
             FROM content_index
             ORDER BY entity_id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    // =========================================================================
    // Projects (I50)
    // =========================================================================

    /// Helper: map a row to `DbProject`.
    fn map_project_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DbProject> {
        Ok(DbProject {
            id: row.get(0)?,
            name: row.get(1)?,
            status: row
                .get::<_, Option<String>>(2)?
                .unwrap_or_else(|| "active".to_string()),
            milestone: row.get(3)?,
            owner: row.get(4)?,
            target_date: row.get(5)?,
            tracker_path: row.get(6)?,
            updated_at: row.get(7)?,
            archived: row.get::<_, i32>(8).unwrap_or(0) != 0,
            keywords: row.get(9).unwrap_or(None),
            keywords_extracted_at: row.get(10).unwrap_or(None),
        })
    }

    /// Insert or update a project. Also mirrors to the `entities` table.
    pub fn upsert_project(&self, project: &DbProject) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT INTO projects (
                id, name, status, milestone, owner, target_date,
                tracker_path, updated_at, archived, keywords, keywords_extracted_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                status = excluded.status,
                milestone = excluded.milestone,
                owner = excluded.owner,
                target_date = excluded.target_date,
                tracker_path = excluded.tracker_path,
                updated_at = excluded.updated_at",
            params![
                project.id,
                project.name,
                project.status,
                project.milestone,
                project.owner,
                project.target_date,
                project.tracker_path,
                project.updated_at,
                project.archived as i32,
                project.keywords,
                project.keywords_extracted_at,
            ],
        )?;
        self.ensure_entity_for_project(project)?;
        Ok(())
    }

    /// Mirror a project to the entities table (bridge pattern).
    pub fn ensure_entity_for_project(&self, project: &DbProject) -> Result<(), DbError> {
        let entity = DbEntity {
            id: project.id.clone(),
            name: project.name.clone(),
            entity_type: EntityType::Project,
            tracker_path: project.tracker_path.clone(),
            updated_at: project.updated_at.clone(),
        };
        self.upsert_entity(&entity)
    }

    /// Get a project by ID.
    pub fn get_project(&self, id: &str) -> Result<Option<DbProject>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, status, milestone, owner, target_date,
                    tracker_path, updated_at, archived,
                    keywords, keywords_extracted_at
             FROM projects WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], Self::map_project_row)?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// Get a project by name (case-insensitive).
    pub fn get_project_by_name(&self, name: &str) -> Result<Option<DbProject>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, status, milestone, owner, target_date,
                    tracker_path, updated_at, archived,
                    keywords, keywords_extracted_at
             FROM projects WHERE LOWER(name) = LOWER(?1)",
        )?;
        let mut rows = stmt.query_map(params![name], Self::map_project_row)?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// Get all projects, ordered by name.
    pub fn get_all_projects(&self) -> Result<Vec<DbProject>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, status, milestone, owner, target_date,
                    tracker_path, updated_at, archived,
                    keywords, keywords_extracted_at
             FROM projects WHERE archived = 0 ORDER BY name",
        )?;
        let rows = stmt.query_map([], Self::map_project_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Update a single whitelisted field on a project.
    pub fn update_project_field(&self, id: &str, field: &str, value: &str) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        let sql = match field {
            "name" => "UPDATE projects SET name = ?1, updated_at = ?3 WHERE id = ?2",
            "status" => "UPDATE projects SET status = ?1, updated_at = ?3 WHERE id = ?2",
            "milestone" => "UPDATE projects SET milestone = ?1, updated_at = ?3 WHERE id = ?2",
            "owner" => "UPDATE projects SET owner = ?1, updated_at = ?3 WHERE id = ?2",
            "target_date" => "UPDATE projects SET target_date = ?1, updated_at = ?3 WHERE id = ?2",
            _ => {
                return Err(DbError::Sqlite(rusqlite::Error::InvalidParameterName(
                    format!("Field '{}' is not updatable", field),
                )))
            }
        };
        self.conn.execute(sql, params![value, id, now])?;
        Ok(())
    }

    /// Update keywords for a project (I305).
    pub fn update_project_keywords(
        &self,
        project_id: &str,
        keywords_json: &str,
    ) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE projects SET keywords = ?1, keywords_extracted_at = ?2, updated_at = ?2
             WHERE id = ?3",
            params![keywords_json, now, project_id],
        )?;
        Ok(())
    }

    /// Update keywords for an account (I305).
    pub fn update_account_keywords(
        &self,
        account_id: &str,
        keywords_json: &str,
    ) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE accounts SET keywords = ?1, keywords_extracted_at = ?2, updated_at = ?2
             WHERE id = ?3",
            params![keywords_json, now, account_id],
        )?;
        Ok(())
    }

    /// Remove a keyword from a project's keyword list (I305 — user curation).
    pub fn remove_project_keyword(
        &self,
        project_id: &str,
        keyword: &str,
    ) -> Result<(), DbError> {
        let current: Option<String> = self
            .conn
            .query_row(
                "SELECT keywords FROM projects WHERE id = ?1",
                params![project_id],
                |row| row.get(0),
            )
            .unwrap_or(None);

        if let Some(json_str) = current {
            if let Ok(mut keywords) = serde_json::from_str::<Vec<String>>(&json_str) {
                keywords.retain(|k| k != keyword);
                let updated = serde_json::to_string(&keywords).map_err(|e| {
                    DbError::Sqlite(rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
                })?;
                self.conn.execute(
                    "UPDATE projects SET keywords = ?1 WHERE id = ?2",
                    params![updated, project_id],
                )?;
            }
        }
        Ok(())
    }

    /// Remove a keyword from an account's keyword list (I305 — user curation).
    pub fn remove_account_keyword(
        &self,
        account_id: &str,
        keyword: &str,
    ) -> Result<(), DbError> {
        let current: Option<String> = self
            .conn
            .query_row(
                "SELECT keywords FROM accounts WHERE id = ?1",
                params![account_id],
                |row| row.get(0),
            )
            .unwrap_or(None);

        if let Some(json_str) = current {
            if let Ok(mut keywords) = serde_json::from_str::<Vec<String>>(&json_str) {
                keywords.retain(|k| k != keyword);
                let updated = serde_json::to_string(&keywords).map_err(|e| {
                    DbError::Sqlite(rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
                })?;
                self.conn.execute(
                    "UPDATE accounts SET keywords = ?1 WHERE id = ?2",
                    params![updated, account_id],
                )?;
            }
        }
        Ok(())
    }

    /// Invalidate meeting prep data (I305 — prep invalidation on entity correction).
    /// NULLs prep columns and returns the old prep_snapshot_path for disk cleanup.
    pub fn invalidate_meeting_prep(&self, meeting_id: &str) -> Result<Option<String>, DbError> {
        let old_path: Option<String> = self
            .conn
            .query_row(
                "SELECT prep_snapshot_path FROM meetings_history WHERE id = ?1",
                params![meeting_id],
                |row| row.get(0),
            )
            .unwrap_or(None);

        self.conn.execute(
            "UPDATE meetings_history SET
                prep_context_json = NULL,
                prep_frozen_json = NULL,
                prep_frozen_at = NULL,
                prep_snapshot_path = NULL
             WHERE id = ?1",
            params![meeting_id],
        )?;

        Ok(old_path)
    }

    /// Get meetings from last N days with no entity links (I305 — hygiene detection).
    /// Returns (id, title, calendar_event_id, start_time) tuples.
    pub fn get_unlinked_meetings(
        &self,
        since: &str,
        limit: usize,
    ) -> Result<Vec<(String, String, Option<String>, String)>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT m.id, m.title, m.calendar_event_id, m.start_time
             FROM meetings_history m
             LEFT JOIN meeting_entities me ON me.meeting_id = m.id
             WHERE m.start_time >= ?1 AND me.meeting_id IS NULL
             ORDER BY m.start_time DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![since, limit as i64], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Get pending/waiting actions for a project.
    pub fn get_project_actions(&self, project_id: &str) -> Result<Vec<DbAction>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, priority, status, created_at, due_date, completed_at,
                    account_id, project_id, source_type, source_id, source_label,
                    context, waiting_on, updated_at, person_id
             FROM actions
             WHERE project_id = ?1
               AND status IN ('pending', 'waiting')
             ORDER BY priority, due_date",
        )?;
        let rows = stmt.query_map(params![project_id], Self::map_action_row)?;
        let mut actions = Vec::new();
        for row in rows {
            actions.push(row?);
        }
        Ok(actions)
    }

    /// Get meetings linked to a project via the meeting_entities junction table.
    pub fn get_meetings_for_project(
        &self,
        project_id: &str,
        limit: i32,
    ) -> Result<Vec<DbMeeting>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT m.id, m.title, m.meeting_type, m.start_time, m.end_time,
                    m.account_id, m.attendees, m.notes_path, m.summary, m.created_at,
                    m.calendar_event_id
             FROM meetings_history m
             JOIN meeting_entities me ON me.meeting_id = m.id
             WHERE me.entity_id = ?1 AND me.entity_type = 'project'
             ORDER BY m.start_time DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![project_id, limit], |row| {
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
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Link a meeting to a project in the meeting_entities junction table.
    pub fn link_meeting_to_project(
        &self,
        meeting_id: &str,
        project_id: &str,
    ) -> Result<(), DbError> {
        self.link_meeting_entity(meeting_id, project_id, "project")
    }

    /// Link a meeting to any entity in the junction table (I52 generic).
    pub fn link_meeting_entity(
        &self,
        meeting_id: &str,
        entity_id: &str,
        entity_type: &str,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT OR IGNORE INTO meeting_entities (meeting_id, entity_id, entity_type)
             VALUES (?1, ?2, ?3)",
            params![meeting_id, entity_id, entity_type],
        )?;
        Ok(())
    }

    /// Remove a meeting-entity link from the junction table.
    pub fn unlink_meeting_entity(&self, meeting_id: &str, entity_id: &str) -> Result<(), DbError> {
        self.conn.execute(
            "DELETE FROM meeting_entities WHERE meeting_id = ?1 AND entity_id = ?2",
            params![meeting_id, entity_id],
        )?;
        Ok(())
    }

    /// Get all entities linked to a meeting via the junction table.
    pub fn get_meeting_entities(&self, meeting_id: &str) -> Result<Vec<DbEntity>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT e.id, e.name, e.entity_type, e.tracker_path, e.updated_at
             FROM entities e
             JOIN meeting_entities me ON me.entity_id = e.id
             WHERE me.meeting_id = ?1",
        )?;
        let rows = stmt.query_map(params![meeting_id], |row| {
            let et: String = row.get(2)?;
            Ok(DbEntity {
                id: row.get(0)?,
                name: row.get(1)?,
                entity_type: EntityType::from_str_lossy(&et),
                tracker_path: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Batch query: get linked entities for multiple meetings at once.
    /// Returns a map from meeting_id → Vec<LinkedEntity>.
    pub fn get_meeting_entity_map(
        &self,
        meeting_ids: &[String],
    ) -> Result<HashMap<String, Vec<LinkedEntity>>, DbError> {
        if meeting_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let placeholders: Vec<String> = (0..meeting_ids.len())
            .map(|i| format!("?{}", i + 1))
            .collect();
        let sql = format!(
            "SELECT me.meeting_id, e.id, e.name, me.entity_type
             FROM meeting_entities me
             JOIN entities e ON e.id = me.entity_id
             WHERE me.meeting_id IN ({})",
            placeholders.join(", ")
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let params: Vec<&dyn rusqlite::types::ToSql> = meeting_ids
            .iter()
            .map(|id| id as &dyn rusqlite::types::ToSql)
            .collect();
        let rows = stmt.query_map(params.as_slice(), |row| {
            Ok((
                row.get::<_, String>(0)?,
                LinkedEntity {
                    id: row.get(1)?,
                    name: row.get(2)?,
                    entity_type: row.get(3)?,
                },
            ))
        })?;
        let mut map: HashMap<String, Vec<LinkedEntity>> = HashMap::new();
        for row in rows {
            let (meeting_id, entity) = row?;
            map.entry(meeting_id).or_default().push(entity);
        }
        Ok(map)
    }

    /// Clear all entity links for a given meeting.
    pub fn clear_meeting_entities(&self, meeting_id: &str) -> Result<(), DbError> {
        self.conn.execute(
            "DELETE FROM meeting_entities WHERE meeting_id = ?1",
            params![meeting_id],
        )?;
        Ok(())
    }

    /// Update the legacy `account_id` column on `meetings_history`.
    pub fn update_meeting_account(
        &self,
        meeting_id: &str,
        account_id: Option<&str>,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "UPDATE meetings_history SET account_id = ?1 WHERE id = ?2",
            params![account_id, meeting_id],
        )?;
        Ok(())
    }

    /// Cascade entity reassignment to actions linked to this meeting.
    pub fn cascade_meeting_entity_to_actions(
        &self,
        meeting_id: &str,
        account_id: Option<&str>,
        project_id: Option<&str>,
    ) -> Result<usize, DbError> {
        let mut total = 0;
        total += self.conn.execute(
            "UPDATE actions SET account_id = ?1 WHERE source_id = ?2",
            params![account_id, meeting_id],
        )?;
        total += self.conn.execute(
            "UPDATE actions SET project_id = ?1 WHERE source_id = ?2",
            params![project_id, meeting_id],
        )?;
        Ok(total)
    }

    /// Cascade entity reassignment to captures linked to this meeting.
    pub fn cascade_meeting_entity_to_captures(
        &self,
        meeting_id: &str,
        account_id: Option<&str>,
        project_id: Option<&str>,
    ) -> Result<usize, DbError> {
        let mut total = 0;
        total += self.conn.execute(
            "UPDATE captures SET account_id = ?1 WHERE meeting_id = ?2",
            params![account_id, meeting_id],
        )?;
        total += self.conn.execute(
            "UPDATE captures SET project_id = ?1 WHERE meeting_id = ?2",
            params![project_id, meeting_id],
        )?;
        Ok(total)
    }

    /// Cascade meeting entity links to all non-internal attendees.
    /// When a meeting is linked to an account/project, automatically link all
    /// external attendees to that entity via the `entity_people` junction table.
    ///
    /// Returns the number of new person-entity links created (excludes existing links).
    pub fn cascade_meeting_entity_to_people(
        &self,
        meeting_id: &str,
        account_id: Option<&str>,
        project_id: Option<&str>,
    ) -> Result<usize, DbError> {
        let entity_id = account_id.or(project_id);
        let entity_id = match entity_id {
            Some(eid) => eid,
            None => return Ok(0),
        };

        // Link all external attendees of this meeting to the entity (idempotent).
        let count = self.conn.execute(
            "INSERT OR IGNORE INTO entity_people (entity_id, person_id, relationship_type)
             SELECT ?1, ma.person_id, 'attendee'
             FROM meeting_attendees ma
             JOIN people p ON ma.person_id = p.id
             WHERE ma.meeting_id = ?2
               AND p.relationship = 'external'",
            params![entity_id, meeting_id],
        )?;

        Ok(count)
    }

    // =========================================================================
    // Domain reclassification (I184 — reclassify on domain change)
    // =========================================================================

    /// Reclassify all people's relationship based on current user domains.
    /// People whose email domain matches ANY domain → "internal", otherwise → "external".
    /// Returns the number of people whose relationship changed.
    pub fn reclassify_people_for_domains(&self, user_domains: &[String]) -> Result<usize, DbError> {
        if user_domains.is_empty() {
            return Ok(0);
        }

        let mut stmt = self
            .conn
            .prepare("SELECT id, email, relationship FROM people")?;
        let people: Vec<(String, String, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
            .collect::<Result<Vec<_>, _>>()?;

        let mut update_stmt = self
            .conn
            .prepare("UPDATE people SET relationship = ?1 WHERE id = ?2")?;

        let mut total = 0;
        for (id, email, current_rel) in &people {
            let domain = email.split('@').nth(1).unwrap_or("");
            if domain.is_empty() {
                continue;
            }
            let new_rel = if user_domains.iter().any(|d| d.eq_ignore_ascii_case(domain)) {
                "internal"
            } else {
                "external"
            };
            if new_rel != current_rel {
                update_stmt.execute(params![new_rel, id])?;
                total += 1;
            }
        }

        Ok(total)
    }

    /// Reclassify meeting types based on current attendee relationships.
    /// Call after `reclassify_people_for_domains` to fix meetings whose type
    /// was stale due to domain changes. Returns the number updated.
    ///
    /// Only touches domain-dependent types (customer, external, one_on_one, internal).
    /// Title-derived types (qbr, training, all_hands, team_sync, personal) are left alone
    /// since they don't depend on domain classification.
    pub fn reclassify_meeting_types_from_attendees(&self) -> Result<usize, DbError> {
        let mut total = 0;

        // Meetings classified as customer/external/one_on_one but ALL attendees are now internal → internal
        total += self.conn.execute(
            "UPDATE meetings_history SET meeting_type = 'internal'
             WHERE meeting_type IN ('customer', 'external', 'one_on_one')
               AND id IN (
                   SELECT ma.meeting_id
                   FROM meeting_attendees ma
                   JOIN people p ON ma.person_id = p.id
                   GROUP BY ma.meeting_id
                   HAVING COUNT(CASE WHEN p.relationship != 'internal' THEN 1 END) = 0
               )",
            [],
        )?;

        // Meetings classified as internal but ANY attendee is now external → customer
        total += self.conn.execute(
            "UPDATE meetings_history SET meeting_type = 'customer'
             WHERE meeting_type = 'internal'
               AND id IN (
                   SELECT DISTINCT ma.meeting_id
                   FROM meeting_attendees ma
                   JOIN people p ON ma.person_id = p.id
                   WHERE p.relationship = 'external'
               )",
            [],
        )?;

        Ok(total)
    }

    /// Get meetings for any entity (generic, via junction table).
    pub fn get_meetings_for_entity(
        &self,
        entity_id: &str,
        limit: i32,
    ) -> Result<Vec<DbMeeting>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT m.id, m.title, m.meeting_type, m.start_time, m.end_time,
                    m.account_id, m.attendees, m.notes_path, m.summary, m.created_at,
                    m.calendar_event_id
             FROM meetings_history m
             JOIN meeting_entities me ON me.meeting_id = m.id
             WHERE me.entity_id = ?1
             ORDER BY m.start_time DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![entity_id, limit], |row| {
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
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Compute activity signals for a project.
    pub fn get_project_signals(&self, project_id: &str) -> Result<ProjectSignals, DbError> {
        // Meeting counts via junction table
        let count_30d: i32 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM meetings_history m
                 JOIN meeting_entities me ON me.meeting_id = m.id
                 WHERE me.entity_id = ?1 AND me.entity_type = 'project'
                   AND m.start_time >= date('now', '-30 days')",
                params![project_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let count_90d: i32 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM meetings_history m
                 JOIN meeting_entities me ON me.meeting_id = m.id
                 WHERE me.entity_id = ?1 AND me.entity_type = 'project'
                   AND m.start_time >= date('now', '-90 days')",
                params![project_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let last_meeting: Option<String> = self
            .conn
            .query_row(
                "SELECT MAX(m.start_time) FROM meetings_history m
                 JOIN meeting_entities me ON me.meeting_id = m.id
                 WHERE me.entity_id = ?1 AND me.entity_type = 'project'",
                params![project_id],
                |row| row.get(0),
            )
            .unwrap_or(None);

        // Days until target date
        let target_date: Option<String> = self
            .conn
            .query_row(
                "SELECT target_date FROM projects WHERE id = ?1",
                params![project_id],
                |row| row.get(0),
            )
            .unwrap_or(None);

        let days_until_target = target_date.as_ref().and_then(|td| {
            chrono::NaiveDate::parse_from_str(td, "%Y-%m-%d")
                .ok()
                .map(|date| {
                    let today = Utc::now().date_naive();
                    (date - today).num_days()
                })
        });

        // Open action count
        let open_action_count: i32 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM actions
                 WHERE project_id = ?1 AND status IN ('pending', 'waiting')",
                params![project_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let temperature = match &last_meeting {
            Some(dt) => compute_temperature(dt),
            None => "cold".to_string(),
        };
        let trend = compute_trend(count_30d, count_90d);

        Ok(ProjectSignals {
            meeting_frequency_30d: count_30d,
            meeting_frequency_90d: count_90d,
            last_meeting,
            days_until_target,
            open_action_count,
            temperature,
            trend,
        })
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
            })
        })?;

        let mut meetings = Vec::new();
        for row in rows {
            meetings.push(row?);
        }
        Ok(meetings)
    }

    /// Look up a single meeting by its ID (includes prep_context_json).
    pub fn get_meeting_by_id(&self, id: &str) -> Result<Option<DbMeeting>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, meeting_type, start_time, end_time,
                    account_id, attendees, notes_path, summary, created_at,
                    calendar_event_id, description, prep_context_json,
                    user_agenda_json, user_notes, prep_frozen_json, prep_frozen_at,
                    prep_snapshot_path, prep_snapshot_hash, transcript_path, transcript_processed_at
             FROM meetings_history
             WHERE id = ?1",
        )?;

        let mut rows = stmt.query_map(params![id], |row| {
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
                description: row.get(11)?,
                prep_context_json: row.get(12)?,
                user_agenda_json: row.get(13)?,
                user_notes: row.get(14)?,
                prep_frozen_json: row.get(15)?,
                prep_frozen_at: row.get(16)?,
                prep_snapshot_path: row.get(17)?,
                prep_snapshot_hash: row.get(18)?,
                transcript_path: row.get(19)?,
                transcript_processed_at: row.get(20)?,
            })
        })?;

        match rows.next() {
            Some(Ok(meeting)) => Ok(Some(meeting)),
            Some(Err(e)) => Err(DbError::Sqlite(e)),
            None => Ok(None),
        }
    }

    /// Look up a single meeting row with all permanence/transcript columns.
    pub fn get_meeting_intelligence_row(
        &self,
        meeting_id: &str,
    ) -> Result<Option<DbMeeting>, DbError> {
        self.get_meeting_by_id(meeting_id)
    }

    /// Return all meetings that have persisted prep context JSON.
    pub fn list_meeting_prep_contexts(&self) -> Result<Vec<(String, String)>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, prep_context_json
             FROM meetings_history
             WHERE prep_context_json IS NOT NULL
               AND trim(prep_context_json) != ''",
        )?;
        let rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let prep_context_json: String = row.get(1)?;
            Ok((id, prep_context_json))
        })?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    /// Update prep context JSON for a single meeting.
    pub fn update_meeting_prep_context(
        &self,
        meeting_id: &str,
        prep_context_json: &str,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "UPDATE meetings_history
             SET prep_context_json = ?1
             WHERE id = ?2",
            params![prep_context_json, meeting_id],
        )?;
        Ok(())
    }

    /// Persist user-authored agenda/notes in the meeting row.
    pub fn update_meeting_user_layer(
        &self,
        meeting_id: &str,
        user_agenda_json: Option<&str>,
        user_notes: Option<&str>,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "UPDATE meetings_history
             SET user_agenda_json = ?1,
                 user_notes = ?2
             WHERE id = ?3",
            params![user_agenda_json, user_notes, meeting_id],
        )?;
        Ok(())
    }

    /// Freeze immutable prep snapshot metadata once. No-op when already frozen.
    pub fn freeze_meeting_prep_snapshot(
        &self,
        meeting_id: &str,
        frozen_json: &str,
        frozen_at: &str,
        snapshot_path: &str,
        snapshot_hash: &str,
    ) -> Result<bool, DbError> {
        let affected = self.conn.execute(
            "UPDATE meetings_history
             SET prep_frozen_json = ?1,
                 prep_frozen_at = ?2,
                 prep_snapshot_path = ?3,
                 prep_snapshot_hash = ?4
             WHERE id = ?5
               AND prep_frozen_at IS NULL",
            params![
                frozen_json,
                frozen_at,
                snapshot_path,
                snapshot_hash,
                meeting_id
            ],
        )?;
        Ok(affected > 0)
    }

    /// Persist transcript metadata directly on the meeting row.
    pub fn update_meeting_transcript_metadata(
        &self,
        meeting_id: &str,
        transcript_path: &str,
        processed_at: &str,
        summary_opt: Option<&str>,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "UPDATE meetings_history
             SET transcript_path = ?1,
                 transcript_processed_at = ?2,
                 summary = COALESCE(?3, summary)
             WHERE id = ?4",
            params![transcript_path, processed_at, summary_opt, meeting_id],
        )?;
        Ok(())
    }

    // =========================================================================
    // Quill Sync State
    // =========================================================================

    /// Insert a new sync state row for a meeting with a specific source.
    pub fn insert_quill_sync_state_with_source(
        &self,
        meeting_id: &str,
        source: &str,
    ) -> Result<String, DbError> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let next_attempt = (Utc::now() + chrono::Duration::minutes(2)).to_rfc3339();
        self.conn.execute(
            "INSERT OR IGNORE INTO quill_sync_state (id, meeting_id, source, next_attempt_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, meeting_id, source, next_attempt, now, now],
        )?;
        Ok(id)
    }

    /// Insert a new Quill sync state row for a meeting (state=pending).
    pub fn insert_quill_sync_state(&self, meeting_id: &str) -> Result<String, DbError> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let next_attempt = (Utc::now() + chrono::Duration::minutes(2)).to_rfc3339();
        self.conn.execute(
            "INSERT OR IGNORE INTO quill_sync_state (id, meeting_id, next_attempt_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, meeting_id, next_attempt, now, now],
        )?;
        Ok(id)
    }

    /// Get Quill sync state for a specific meeting.
    pub fn get_quill_sync_state(
        &self,
        meeting_id: &str,
    ) -> Result<Option<DbQuillSyncState>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, meeting_id, quill_meeting_id, state, attempts, max_attempts,
                    next_attempt_at, last_attempt_at, completed_at, error_message,
                    match_confidence, transcript_path, created_at, updated_at, source
             FROM quill_sync_state WHERE meeting_id = ?1",
        )?;
        let mut rows = stmt.query_map(params![meeting_id], map_sync_row)?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// Get sync state for a specific meeting and source.
    pub fn get_quill_sync_state_by_source(
        &self,
        meeting_id: &str,
        source: &str,
    ) -> Result<Option<DbQuillSyncState>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, meeting_id, quill_meeting_id, state, attempts, max_attempts,
                    next_attempt_at, last_attempt_at, completed_at, error_message,
                    match_confidence, transcript_path, created_at, updated_at, source
             FROM quill_sync_state WHERE meeting_id = ?1 AND source = ?2",
        )?;
        let mut rows = stmt.query_map(params![meeting_id, source], map_sync_row)?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// Get all pending Quill syncs ready for processing (source='quill' only).
    pub fn get_pending_quill_syncs(&self) -> Result<Vec<DbQuillSyncState>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, meeting_id, quill_meeting_id, state, attempts, max_attempts,
                    next_attempt_at, last_attempt_at, completed_at, error_message,
                    match_confidence, transcript_path, created_at, updated_at, source
             FROM quill_sync_state
             WHERE state IN ('pending', 'polling') AND next_attempt_at <= datetime('now')
               AND source = 'quill'",
        )?;
        let rows = stmt.query_map([], map_sync_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    /// Update Quill sync state fields.
    pub fn update_quill_sync_state(
        &self,
        id: &str,
        state: &str,
        quill_meeting_id: Option<&str>,
        match_confidence: Option<f64>,
        error_message: Option<&str>,
        transcript_path: Option<&str>,
    ) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        let completed_at: Option<String> = if state == "completed" {
            Some(now.clone())
        } else {
            None
        };
        self.conn.execute(
            "UPDATE quill_sync_state
             SET state = ?1,
                 quill_meeting_id = COALESCE(?2, quill_meeting_id),
                 match_confidence = COALESCE(?3, match_confidence),
                 error_message = ?4,
                 transcript_path = COALESCE(?5, transcript_path),
                 completed_at = COALESCE(?6, completed_at),
                 updated_at = ?7
             WHERE id = ?8",
            params![
                state,
                quill_meeting_id,
                match_confidence,
                error_message,
                transcript_path,
                completed_at,
                now,
                id
            ],
        )?;
        Ok(())
    }

    /// Advance attempt counter with exponential backoff (10, 20, 40, 80, 160 min).
    /// Returns true if still has attempts remaining, false if abandoned.
    pub fn advance_quill_sync_attempt(&self, id: &str) -> Result<bool, DbError> {
        let (attempts, max_attempts): (i32, i32) = self.conn.query_row(
            "SELECT attempts, max_attempts FROM quill_sync_state WHERE id = ?1",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        let new_attempts = attempts + 1;
        let now = Utc::now().to_rfc3339();

        if new_attempts >= max_attempts {
            self.conn.execute(
                "UPDATE quill_sync_state
                 SET attempts = ?1, state = 'abandoned', last_attempt_at = ?2, updated_at = ?2
                 WHERE id = ?3",
                params![new_attempts, now, id],
            )?;
            return Ok(false);
        }

        // Exponential backoff: 5 * 2^attempts minutes (5, 10, 20, 40, 80 min)
        let delay_minutes = 5i64 * (1i64 << new_attempts);
        let next_attempt = (Utc::now() + chrono::Duration::minutes(delay_minutes)).to_rfc3339();

        self.conn.execute(
            "UPDATE quill_sync_state
             SET attempts = ?1, last_attempt_at = ?2, next_attempt_at = ?3, updated_at = ?2
             WHERE id = ?4",
            params![new_attempts, now, next_attempt, id],
        )?;
        Ok(true)
    }

    /// Count sync rows in a given state (all sources).
    pub fn count_quill_syncs_by_state(&self, state: &str) -> Result<usize, DbError> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM quill_sync_state WHERE state = ?1",
            params![state],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    /// Count sync rows in a given state for a specific source.
    pub fn count_syncs_by_state_and_source(
        &self,
        state: &str,
        source: &str,
    ) -> Result<usize, DbError> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM quill_sync_state WHERE state = ?1 AND source = ?2",
            params![state, source],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    /// Get abandoned Quill syncs eligible for retry (between min_days and max_days old).
    pub fn get_retryable_abandoned_quill_syncs(
        &self,
        min_days: i32,
        max_days: i32,
    ) -> Result<Vec<DbQuillSyncState>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, meeting_id, quill_meeting_id, state, attempts, max_attempts,
                    next_attempt_at, last_attempt_at, completed_at, error_message,
                    match_confidence, transcript_path, created_at, updated_at, source
             FROM quill_sync_state
             WHERE state = 'abandoned'
               AND created_at >= datetime('now', ?1)
               AND created_at <= datetime('now', ?2)",
        )?;
        let min_offset = format!("-{} days", max_days);
        let max_offset = format!("-{} days", min_days);
        let rows = stmt.query_map(params![min_offset, max_offset], map_sync_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    /// Reset an abandoned Quill sync for retry: set state to pending, clear attempts.
    pub fn reset_quill_sync_for_retry(&self, sync_id: &str) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE quill_sync_state
             SET state = 'pending', attempts = 0, error_message = NULL,
                 next_attempt_at = ?1, updated_at = ?1
             WHERE id = ?2",
            params![now, sync_id],
        )?;
        Ok(())
    }

    /// Get recent meetings as (id, title, start_time) tuples for transcript matching.
    pub fn get_meetings_for_transcript_matching(
        &self,
        days_back: i32,
    ) -> Result<Vec<(String, String, String)>, DbError> {
        let offset = format!("-{} days", days_back);
        let mut stmt = self.conn.prepare(
            "SELECT id, title, start_time FROM meetings_history
             WHERE start_time >= datetime('now', ?1)
             ORDER BY start_time DESC",
        )?;
        let rows = stmt.query_map(params![offset], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    /// Get meeting IDs eligible for Quill backfill: past meetings within `days_back`
    /// that have no transcript and no existing quill_sync_state row.
    ///
    /// Only includes account-linked meetings with relationship-relevant types
    /// (customer, qbr, partnership). Excludes internal, one_on_one, and external
    /// meetings which are too broad and would pull in personal or tangential calls.
    pub fn get_backfill_eligible_meeting_ids(&self, days_back: i32) -> Result<Vec<String>, DbError> {
        let offset = format!("-{} days", days_back);
        let mut stmt = self.conn.prepare(
            "SELECT id FROM meetings_history
             WHERE transcript_path IS NULL AND transcript_processed_at IS NULL
               AND start_time >= datetime('now', ?1)
               AND end_time < datetime('now')
               AND account_id IS NOT NULL
               AND meeting_type IN ('customer','qbr','partnership')
               AND id NOT IN (SELECT meeting_id FROM quill_sync_state)
             ORDER BY start_time DESC",
        )?;
        let rows = stmt.query_map(params![offset], |row| row.get(0))?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
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

    /// Get the latest processing status for each filename in the processing_log.
    ///
    /// Returns a map of `filename -> (status, error_message)` using the most recent
    /// log entry per filename. Uses the existing `idx_processing_created` index.
    pub fn get_latest_processing_status(
        &self,
    ) -> Result<HashMap<String, (String, Option<String>)>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT p.filename, p.status, p.error_message
             FROM processing_log p
             INNER JOIN (
                 SELECT filename, MAX(created_at) AS max_created
                 FROM processing_log
                 GROUP BY filename
             ) latest ON p.filename = latest.filename AND p.created_at = latest.max_created",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })?;

        let mut map = HashMap::new();
        for row in rows {
            let (filename, status, error_message) = row?;
            map.insert(filename, (status, error_message));
        }
        Ok(map)
    }

    // =========================================================================
    // Captures (post-meeting wins/risks)
    // =========================================================================

    /// Map a row to DbCapture. Expects columns:
    /// id, meeting_id, meeting_title, account_id, project_id, capture_type, content, captured_at
    fn map_capture_row(row: &rusqlite::Row) -> rusqlite::Result<DbCapture> {
        Ok(DbCapture {
            id: row.get(0)?,
            meeting_id: row.get(1)?,
            meeting_title: row.get(2)?,
            account_id: row.get(3)?,
            project_id: row.get(4)?,
            capture_type: row.get(5)?,
            content: row.get(6)?,
            captured_at: row.get(7)?,
        })
    }

    /// Insert a capture (win, risk, or action) from a post-meeting prompt.
    pub fn insert_capture(
        &self,
        meeting_id: &str,
        meeting_title: &str,
        account_id: Option<&str>,
        capture_type: &str,
        content: &str,
    ) -> Result<(), DbError> {
        self.insert_capture_with_project(
            meeting_id,
            meeting_title,
            account_id,
            None,
            capture_type,
            content,
        )
    }

    /// Insert a capture with optional project_id (I52).
    pub fn insert_capture_with_project(
        &self,
        meeting_id: &str,
        meeting_title: &str,
        account_id: Option<&str>,
        project_id: Option<&str>,
        capture_type: &str,
        content: &str,
    ) -> Result<(), DbError> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO captures (id, meeting_id, meeting_title, account_id, project_id, capture_type, content, captured_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![id, meeting_id, meeting_title, account_id, project_id, capture_type, content, now],
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
            "SELECT id, meeting_id, meeting_title, account_id, project_id, capture_type, content, captured_at
             FROM captures
             WHERE account_id = ?1
               AND captured_at >= date('now', ?2 || ' days')
             ORDER BY captured_at DESC",
        )?;

        let days_param = format!("-{days_back}");
        let rows = stmt.query_map(params![account_id, days_param], Self::map_capture_row)?;

        let mut captures = Vec::new();
        for row in rows {
            captures.push(row?);
        }
        Ok(captures)
    }

    /// Query recent captures for a project within `days_back` days (I52).
    pub fn get_captures_for_project(
        &self,
        project_id: &str,
        days_back: i32,
    ) -> Result<Vec<DbCapture>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, meeting_id, meeting_title, account_id, project_id, capture_type, content, captured_at
             FROM captures
             WHERE project_id = ?1
               AND captured_at >= date('now', ?2 || ' days')
             ORDER BY captured_at DESC",
        )?;

        let days_param = format!("-{days_back}");
        let rows = stmt.query_map(params![project_id, days_param], Self::map_capture_row)?;

        let mut captures = Vec::new();
        for row in rows {
            captures.push(row?);
        }
        Ok(captures)
    }

    /// Query all captures (wins, risks, decisions) for a specific meeting.
    pub fn get_captures_for_meeting(&self, meeting_id: &str) -> Result<Vec<DbCapture>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, meeting_id, meeting_title, account_id, project_id, capture_type, content, captured_at
             FROM captures
             WHERE meeting_id = ?1
             ORDER BY captured_at",
        )?;

        let rows = stmt.query_map(params![meeting_id], Self::map_capture_row)?;

        let mut captures = Vec::new();
        for row in rows {
            captures.push(row?);
        }
        Ok(captures)
    }

    /// Get recent captures from meetings a person attended within `days_back` days.
    pub fn get_captures_for_person(
        &self,
        person_id: &str,
        days_back: i32,
    ) -> Result<Vec<DbCapture>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT c.id, c.meeting_id, c.meeting_title, c.account_id, c.project_id, c.capture_type, c.content, c.captured_at
             FROM captures c
             JOIN meeting_attendees ma ON ma.meeting_id = c.meeting_id
             WHERE ma.person_id = ?1
               AND c.captured_at >= date('now', ?2 || ' days')
             ORDER BY c.captured_at DESC
             LIMIT 20",
        )?;

        let days_param = format!("-{days_back}");
        let rows = stmt.query_map(params![person_id, days_param], Self::map_capture_row)?;

        let mut captures = Vec::new();
        for row in rows {
            captures.push(row?);
        }
        Ok(captures)
    }

    /// Insert an email intelligence signal, deduped by `(email_id, entity_id, signal_type, signal_text)`.
    /// Known signal types from AI enrichment. Unknown types are rejected to prevent
    /// hallucinated categories from polluting the database.
    const VALID_SIGNAL_TYPES: &'static [&'static str] = &[
        "expansion",
        "question",
        "timeline",
        "sentiment",
        "feedback",
        "relationship",
    ];

    const VALID_ENTITY_TYPES: &'static [&'static str] = &["account", "project"];

    /// Insert an email signal, returning `true` if a new row was inserted.
    #[allow(clippy::too_many_arguments)]
    pub fn upsert_email_signal(
        &self,
        email_id: &str,
        sender_email: Option<&str>,
        person_id: Option<&str>,
        entity_id: &str,
        entity_type: &str,
        signal_type: &str,
        signal_text: &str,
        confidence: Option<f64>,
        sentiment: Option<&str>,
        urgency: Option<&str>,
        detected_at: Option<&str>,
    ) -> Result<bool, DbError> {
        if !Self::VALID_SIGNAL_TYPES.contains(&signal_type) {
            log::warn!(
                "Ignoring unknown email signal type '{}' for entity {}",
                signal_type,
                entity_id
            );
            return Ok(false);
        }
        if !Self::VALID_ENTITY_TYPES.contains(&entity_type) {
            log::warn!(
                "Ignoring unknown entity type '{}' for email signal",
                entity_type
            );
            return Ok(false);
        }

        self.conn.execute(
            "INSERT OR IGNORE INTO email_signals (
                email_id, sender_email, person_id, entity_id, entity_type,
                signal_type, signal_text, confidence, sentiment, urgency, detected_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, COALESCE(?11, datetime('now')))",
            params![
                email_id,
                sender_email,
                person_id,
                entity_id,
                entity_type,
                signal_type,
                signal_text,
                confidence,
                sentiment,
                urgency,
                detected_at,
            ],
        )?;
        Ok(self.conn.changes() > 0)
    }

    /// List recent email signals for an entity.
    pub fn list_recent_email_signals_for_entity(
        &self,
        entity_id: &str,
        limit: usize,
    ) -> Result<Vec<DbEmailSignal>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, email_id, sender_email, person_id, entity_id, entity_type,
                    signal_type, signal_text, confidence, sentiment, urgency, detected_at
             FROM email_signals
             WHERE entity_id = ?1
             ORDER BY detected_at DESC, id DESC
             LIMIT ?2",
        )?;

        let rows = stmt.query_map(params![entity_id, limit as i64], |row| {
            Ok(DbEmailSignal {
                id: row.get(0)?,
                email_id: row.get(1)?,
                sender_email: row.get(2)?,
                person_id: row.get(3)?,
                entity_id: row.get(4)?,
                entity_type: row.get(5)?,
                signal_type: row.get(6)?,
                signal_text: row.get(7)?,
                confidence: row.get(8)?,
                sentiment: row.get(9)?,
                urgency: row.get(10)?,
                detected_at: row.get(11)?,
            })
        })?;

        let mut signals = Vec::new();
        for row in rows {
            signals.push(row?);
        }
        Ok(signals)
    }

    /// Batch-query email signals for multiple email IDs.
    /// Returns all signals whose email_id is in the provided list.
    pub fn list_email_signals_by_email_ids(
        &self,
        email_ids: &[String],
    ) -> Result<Vec<DbEmailSignal>, DbError> {
        if email_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders: Vec<String> = (1..=email_ids.len()).map(|i| format!("?{}", i)).collect();
        let sql = format!(
            "SELECT id, email_id, sender_email, person_id, entity_id, entity_type,
                    signal_type, signal_text, confidence, sentiment, urgency, detected_at
             FROM email_signals
             WHERE email_id IN ({})
             ORDER BY detected_at DESC, id DESC",
            placeholders.join(", ")
        );

        let mut stmt = self.conn.prepare(&sql)?;
        let params: Vec<&dyn rusqlite::types::ToSql> = email_ids
            .iter()
            .map(|id| id as &dyn rusqlite::types::ToSql)
            .collect();

        let rows = stmt.query_map(&*params, |row| {
            Ok(DbEmailSignal {
                id: row.get(0)?,
                email_id: row.get(1)?,
                sender_email: row.get(2)?,
                person_id: row.get(3)?,
                entity_id: row.get(4)?,
                entity_type: row.get(5)?,
                signal_type: row.get(6)?,
                signal_text: row.get(7)?,
                confidence: row.get(8)?,
                sentiment: row.get(9)?,
                urgency: row.get(10)?,
                detected_at: row.get(11)?,
            })
        })?;

        let mut signals = Vec::new();
        for row in rows {
            signals.push(row?);
        }
        Ok(signals)
    }

    /// Query actions extracted from a transcript for a specific meeting.
    pub fn get_actions_for_meeting(&self, meeting_id: &str) -> Result<Vec<DbAction>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, priority, status, created_at, due_date, completed_at,
                    account_id, project_id, source_type, source_id, source_label,
                    context, waiting_on, updated_at, person_id
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
                person_id: row.get(15)?,
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
            "SELECT id, meeting_id, meeting_title, account_id, project_id, capture_type, content, captured_at
             FROM captures
             WHERE date(captured_at) = ?1
             ORDER BY account_id, captured_at",
        )?;

        let rows = stmt.query_map(params![date], Self::map_capture_row)?;

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
                calendar_event_id, description, prep_context_json,
                user_agenda_json, user_notes, prep_frozen_json, prep_frozen_at,
                prep_snapshot_path, prep_snapshot_hash, transcript_path, transcript_processed_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)
             ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                meeting_type = excluded.meeting_type,
                start_time = excluded.start_time,
                end_time = excluded.end_time,
                account_id = excluded.account_id,
                attendees = excluded.attendees,
                notes_path = excluded.notes_path,
                summary = excluded.summary,
                calendar_event_id = excluded.calendar_event_id,
                description = excluded.description,
                prep_context_json = COALESCE(excluded.prep_context_json, meetings_history.prep_context_json),
                user_agenda_json = COALESCE(excluded.user_agenda_json, meetings_history.user_agenda_json),
                user_notes = COALESCE(excluded.user_notes, meetings_history.user_notes),
                prep_frozen_json = COALESCE(meetings_history.prep_frozen_json, excluded.prep_frozen_json),
                prep_frozen_at = COALESCE(meetings_history.prep_frozen_at, excluded.prep_frozen_at),
                prep_snapshot_path = COALESCE(meetings_history.prep_snapshot_path, excluded.prep_snapshot_path),
                prep_snapshot_hash = COALESCE(meetings_history.prep_snapshot_hash, excluded.prep_snapshot_hash),
                transcript_path = COALESCE(excluded.transcript_path, meetings_history.transcript_path),
                transcript_processed_at = COALESCE(excluded.transcript_processed_at, meetings_history.transcript_processed_at)",
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
                meeting.description,
                meeting.prep_context_json,
                meeting.user_agenda_json,
                meeting.user_notes,
                meeting.prep_frozen_json,
                meeting.prep_frozen_at,
                meeting.prep_snapshot_path,
                meeting.prep_snapshot_hash,
                meeting.transcript_path,
                meeting.transcript_processed_at,
            ],
        )?;

        // Auto-link junction when account_id is present (I52)
        // Resolve account name → slugified entity ID via accounts table
        if let Some(ref account_name) = meeting.account_id {
            if !account_name.is_empty() {
                if let Ok(Some(account)) = self.get_account_by_name(account_name) {
                    let _ = self.link_meeting_entity(&meeting.id, &account.id, "account");
                }
            }
        }

        Ok(())
    }

    /// Look up a meeting by its Google Calendar event ID (I168).
    pub fn get_meeting_by_calendar_event_id(
        &self,
        calendar_event_id: &str,
    ) -> Result<Option<DbMeeting>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, meeting_type, start_time, end_time,
                    account_id, attendees, notes_path, summary, created_at,
                    calendar_event_id, description, prep_context_json,
                    user_agenda_json, user_notes, prep_frozen_json, prep_frozen_at,
                    prep_snapshot_path, prep_snapshot_hash, transcript_path, transcript_processed_at
             FROM meetings_history
             WHERE calendar_event_id = ?1
             LIMIT 1",
        )?;
        let mut rows = stmt.query_map(params![calendar_event_id], |row| {
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
                description: row.get(11)?,
                prep_context_json: row.get(12)?,
                user_agenda_json: row.get(13)?,
                user_notes: row.get(14)?,
                prep_frozen_json: row.get(15)?,
                prep_frozen_at: row.get(16)?,
                prep_snapshot_path: row.get(17)?,
                prep_snapshot_hash: row.get(18)?,
                transcript_path: row.get(19)?,
                transcript_processed_at: row.get(20)?,
            })
        })?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// Ensure a meeting exists in meetings_history (INSERT OR IGNORE).
    /// Used by calendar polling to create lightweight records so
    /// record_meeting_attendance() can query start_time.
    /// Does NOT overwrite existing rows — reconcile.rs owns updates.
    pub fn ensure_meeting_in_history(
        &self,
        input: EnsureMeetingHistoryInput<'_>,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT OR IGNORE INTO meetings_history
                (id, title, meeting_type, start_time, end_time, account_id, created_at, calendar_event_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                input.id,
                input.title,
                input.meeting_type,
                input.start_time,
                input.end_time,
                input.account_id,
                Utc::now().to_rfc3339(),
                input.calendar_event_id,
            ],
        )?;
        Ok(())
    }

    // =========================================================================
    // Prep State Tracking (ADR-0033)
    // =========================================================================

    /// Record that a meeting prep has been reviewed.
    ///
    /// `meeting_id` is the canonical meeting identity (event-id primary).
    pub fn mark_prep_reviewed(
        &self,
        meeting_id: &str,
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
            params![meeting_id, calendar_event_id, now, title],
        )?;
        Ok(())
    }

    /// Get all reviewed meeting IDs. Returns a map of meeting_id → reviewed_at.
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
                    context, waiting_on, updated_at, person_id
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
                    context, waiting_on, updated_at, person_id
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
            "SELECT id, name, lifecycle, arr, health, contract_start, contract_end,
                    nps, tracker_path, parent_id, is_internal, updated_at, archived,
                    keywords, keywords_extracted_at
             FROM accounts
             WHERE contract_end IS NOT NULL
               AND contract_end >= date('now')
               AND contract_end <= date('now', ?1 || ' days')
             ORDER BY contract_end ASC",
        )?;

        let days_param = format!("+{days_ahead}");
        let rows = stmt.query_map(params![days_param], Self::map_account_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Get accounts where `updated_at` is older than `stale_days`.
    ///
    /// Represents accounts that haven't been touched (via meetings, captures,
    /// or manual updates) in a while — a signal to check in.
    pub fn get_stale_accounts(&self, stale_days: i32) -> Result<Vec<DbAccount>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, lifecycle, arr, health, contract_start, contract_end,
                    nps, tracker_path, parent_id, is_internal, updated_at, archived,
                    keywords, keywords_extracted_at
             FROM accounts
             WHERE updated_at <= datetime('now', ?1 || ' days')
             ORDER BY updated_at ASC",
        )?;

        let days_param = format!("-{stale_days}");
        let rows = stmt.query_map(params![days_param], Self::map_account_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
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
                tracker_path, last_seen, first_seen, meeting_count, updated_at, archived
             ) VALUES (?1, LOWER(?2), ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
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
                person.archived as i32,
            ],
        )?;
        // Mirror to entities table (bridge pattern, like ensure_entity_for_account)
        self.ensure_entity_for_person(person)?;
        // Seed person_emails with the primary email
        self.add_person_email(&person.id, &person.email, true)?;
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
                    tracker_path, last_seen, first_seen, meeting_count, updated_at, archived
             FROM people WHERE email = LOWER(?1)",
        )?;
        let mut rows = stmt.query_map(params![email], Self::map_person_row)?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// Look up a person by email, falling back to the `person_emails` alias table.
    ///
    /// 1. Exact match on `people.email`
    /// 2. Exact match on `person_emails.email` → join back to `people`
    pub fn get_person_by_email_or_alias(&self, email: &str) -> Result<Option<DbPerson>, DbError> {
        // Fast path: exact match on primary email
        if let Some(person) = self.get_person_by_email(email)? {
            return Ok(Some(person));
        }
        // Fallback: check person_emails alias table
        let mut stmt = self.conn.prepare(
            "SELECT p.id, p.email, p.name, p.organization, p.role, p.relationship, p.notes,
                    p.tracker_path, p.last_seen, p.first_seen, p.meeting_count, p.updated_at, p.archived
             FROM person_emails pe
             JOIN people p ON p.id = pe.person_id
             WHERE pe.email = LOWER(?1)
             LIMIT 1",
        )?;
        let mut rows = stmt.query_map(params![email], Self::map_person_row)?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// Search for a person by constructing `local_part@sibling` for each sibling domain.
    ///
    /// Returns the first match found (checks both `people.email` and `person_emails`).
    pub fn find_person_by_domain_alias(
        &self,
        email: &str,
        sibling_domains: &[String],
    ) -> Result<Option<DbPerson>, DbError> {
        let local_part = match email.rfind('@') {
            Some(pos) => &email[..pos],
            None => return Ok(None),
        };
        for domain in sibling_domains {
            let candidate = format!("{}@{}", local_part, domain);
            if let Some(person) = self.get_person_by_email_or_alias(&candidate)? {
                return Ok(Some(person));
            }
        }
        Ok(None)
    }

    /// Collect sibling domains for an email address.
    ///
    /// Uses `account_domains` to find accounts that own this email's domain,
    /// then collects all domains from those accounts. Also includes `user_domains`
    /// if this email's domain is among them. Skips personal email domains.
    pub fn get_sibling_domains_for_email(
        &self,
        email: &str,
        user_domains: &[String],
    ) -> Result<Vec<String>, DbError> {
        let domain = crate::prepare::email_classify::extract_domain(email);
        if domain.is_empty() {
            return Ok(Vec::new());
        }
        // Never alias personal email domains
        if crate::google_api::classify::PERSONAL_EMAIL_DOMAINS.contains(&domain.as_str()) {
            return Ok(Vec::new());
        }

        let mut siblings = std::collections::HashSet::new();

        // Path A: account_domains — find accounts owning this domain, collect all their domains
        let accounts = self.lookup_account_candidates_by_domain(&domain)?;
        for account in &accounts {
            let domains = self.get_account_domains(&account.id)?;
            for d in domains {
                if d != domain {
                    siblings.insert(d);
                }
            }
        }

        // Path B: user_domains — if this domain is among user's configured domains
        let user_domains_lower: Vec<String> = user_domains.iter().map(|d| d.to_lowercase()).collect();
        if user_domains_lower.contains(&domain) {
            for d in &user_domains_lower {
                if *d != domain {
                    siblings.insert(d.clone());
                }
            }
        }

        Ok(siblings.into_iter().collect())
    }

    /// Record an email alias for a person (INSERT OR IGNORE).
    pub fn add_person_email(
        &self,
        person_id: &str,
        email: &str,
        is_primary: bool,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT OR IGNORE INTO person_emails (person_id, email, is_primary, added_at)
             VALUES (?1, LOWER(?2), ?3, ?4)",
            params![person_id, email, is_primary as i32, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    /// List all known email addresses for a person.
    pub fn get_person_emails(&self, person_id: &str) -> Result<Vec<String>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT email FROM person_emails WHERE person_id = ?1 ORDER BY is_primary DESC, email",
        )?;
        let rows = stmt.query_map(params![person_id], |row| row.get::<_, String>(0))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Get a person by ID.
    pub fn get_person(&self, id: &str) -> Result<Option<DbPerson>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, email, name, organization, role, relationship, notes,
                    tracker_path, last_seen, first_seen, meeting_count, updated_at, archived
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
                            tracker_path, last_seen, first_seen, meeting_count, updated_at, archived
                     FROM people WHERE relationship = ?1 AND archived = 0 ORDER BY name",
                )?;
                let rows = stmt.query_map(params![rel], Self::map_person_row)?;
                rows.collect::<Result<Vec<_>, _>>()?
            }
            None => {
                let mut stmt = self.conn.prepare(
                    "SELECT id, email, name, organization, role, relationship, notes,
                            tracker_path, last_seen, first_seen, meeting_count, updated_at, archived
                     FROM people WHERE archived = 0 ORDER BY name",
                )?;
                let rows = stmt.query_map([], Self::map_person_row)?;
                rows.collect::<Result<Vec<_>, _>>()?
            }
        };
        Ok(people)
    }

    /// Get all people with pre-computed temperature/trend signals (I106).
    /// Uses a single batch query with LEFT JOIN subqueries instead of 3N individual queries.
    pub fn get_people_with_signals(
        &self,
        relationship: Option<&str>,
    ) -> Result<Vec<PersonListItem>, DbError> {
        let sql = "SELECT p.id, p.email, p.name, p.organization, p.role, p.relationship, p.notes,
                          p.tracker_path, p.last_seen, p.first_seen, p.meeting_count, p.updated_at,
                          p.archived,
                          COALESCE(cnt30.c, 0) AS count_30d,
                          COALESCE(cnt90.c, 0) AS count_90d,
                          last_m.max_start,
                          acct_names.names AS account_names
                   FROM people p
                   LEFT JOIN (
                       SELECT ma.person_id, COUNT(*) AS c FROM meeting_attendees ma
                       JOIN meetings_history m ON m.id = ma.meeting_id
                       WHERE m.start_time >= date('now', '-30 days') GROUP BY ma.person_id
                   ) cnt30 ON cnt30.person_id = p.id
                   LEFT JOIN (
                       SELECT ma.person_id, COUNT(*) AS c FROM meeting_attendees ma
                       JOIN meetings_history m ON m.id = ma.meeting_id
                       WHERE m.start_time >= date('now', '-90 days') GROUP BY ma.person_id
                   ) cnt90 ON cnt90.person_id = p.id
                   LEFT JOIN (
                       SELECT ma.person_id, MAX(m.start_time) AS max_start FROM meeting_attendees ma
                       JOIN meetings_history m ON m.id = ma.meeting_id GROUP BY ma.person_id
                   ) last_m ON last_m.person_id = p.id
                   LEFT JOIN (
                       SELECT ep.person_id, GROUP_CONCAT(e.name, ', ') AS names
                       FROM entity_people ep
                       JOIN entities e ON e.id = ep.entity_id AND e.entity_type = 'account'
                       GROUP BY ep.person_id
                   ) acct_names ON acct_names.person_id = p.id
                   WHERE p.archived = 0 AND (?1 IS NULL OR p.relationship = ?1)
                   ORDER BY p.name";

        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map(params![relationship], |row| {
            let count_30d: i32 = row.get(13)?;
            let count_90d: i32 = row.get(14)?;
            let last_meeting: Option<String> = row.get(15)?;

            let temperature = match &last_meeting {
                Some(dt) => compute_temperature(dt),
                None => "cold".to_string(),
            };
            let trend = compute_trend(count_30d, count_90d);

            Ok(PersonListItem {
                id: row.get(0)?,
                email: row.get(1)?,
                name: row.get(2)?,
                organization: row.get(3)?,
                role: row.get(4)?,
                relationship: row.get(5)?,
                notes: row.get(6)?,
                tracker_path: row.get(7)?,
                last_seen: row.get(8)?,
                first_seen: row.get(9)?,
                meeting_count: row.get(10)?,
                updated_at: row.get(11)?,
                archived: row.get::<_, i32>(12).unwrap_or(0) != 0,
                temperature,
                trend,
                account_names: row.get(16)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Get people linked to an entity (account/project).
    pub fn get_people_for_entity(&self, entity_id: &str) -> Result<Vec<DbPerson>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT p.id, p.email, p.name, p.organization, p.role, p.relationship, p.notes,
                    p.tracker_path, p.last_seen, p.first_seen, p.meeting_count, p.updated_at, p.archived
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
                    p.tracker_path, p.last_seen, p.first_seen, p.meeting_count, p.updated_at, p.archived
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
                    tracker_path, last_seen, first_seen, meeting_count, updated_at, archived
             FROM people
             WHERE name LIKE ?1 OR email LIKE ?1 OR organization LIKE ?1
             ORDER BY name
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![pattern, limit], Self::map_person_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Update a single whitelisted field on a person.
    pub fn update_person_field(&self, id: &str, field: &str, value: &str) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        // Whitelist fields to prevent SQL injection
        let sql = match field {
            "name" => "UPDATE people SET name = ?1, updated_at = ?3 WHERE id = ?2",
            "notes" => "UPDATE people SET notes = ?1, updated_at = ?3 WHERE id = ?2",
            "role" => "UPDATE people SET role = ?1, updated_at = ?3 WHERE id = ?2",
            "organization" => "UPDATE people SET organization = ?1, updated_at = ?3 WHERE id = ?2",
            "relationship" => "UPDATE people SET relationship = ?1, updated_at = ?3 WHERE id = ?2",
            _ => {
                return Err(DbError::Sqlite(rusqlite::Error::InvalidParameterName(
                    format!("Field '{}' is not updatable", field),
                )))
            }
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
            archived: row.get::<_, i32>(12).unwrap_or(0) != 0,
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
        })
    }

    /// Helper: map a row to `DbAccount`.
    fn map_account_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DbAccount> {
        Ok(DbAccount {
            id: row.get(0)?,
            name: row.get(1)?,
            lifecycle: row.get(2)?,
            arr: row.get(3)?,
            health: row.get(4)?,
            contract_start: row.get(5)?,
            contract_end: row.get(6)?,
            nps: row.get(7)?,
            tracker_path: row.get(8)?,
            parent_id: row.get(9)?,
            is_internal: row.get::<_, i32>(10).unwrap_or(0) != 0,
            updated_at: row.get(11)?,
            archived: row.get::<_, i32>(12).unwrap_or(0) != 0,
            keywords: row.get(13).unwrap_or(None),
            keywords_extracted_at: row.get(14).unwrap_or(None),
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
            person_id: row.get(15)?,
        })
    }

    // =========================================================================
    // Hygiene Gap Detection (I145 — ADR-0058)
    // =========================================================================

    /// People with email-derived names: no spaces, contains @, or single word.
    /// These likely need real names resolved from email headers or manual input.
    pub fn get_unnamed_people(&self) -> Result<Vec<DbPerson>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, email, name, organization, role, relationship, notes,
                    tracker_path, last_seen, first_seen, meeting_count, updated_at, archived
             FROM people
             WHERE name NOT LIKE '% %' OR name LIKE '%@%'
             ORDER BY meeting_count DESC",
        )?;
        let rows = stmt.query_map([], Self::map_person_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// People never classified because user_domain wasn't set at creation time.
    pub fn get_unknown_relationship_people(&self) -> Result<Vec<DbPerson>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, email, name, organization, role, relationship, notes,
                    tracker_path, last_seen, first_seen, meeting_count, updated_at, archived
             FROM people
             WHERE relationship = 'unknown'
             ORDER BY meeting_count DESC",
        )?;
        let rows = stmt.query_map([], Self::map_person_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Entities with content files in content_index but no intelligence cache row
    /// or NULL enriched_at. Returns (entity_id, entity_type) pairs.
    pub fn get_entities_without_intelligence(&self) -> Result<Vec<(String, String)>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT ci.entity_id, ci.entity_type
             FROM content_index ci
             LEFT JOIN entity_intelligence ei ON ei.entity_id = ci.entity_id
             WHERE ei.enriched_at IS NULL OR ei.entity_id IS NULL
             ORDER BY ci.entity_id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Entities where enriched_at is older than threshold AND new content exists
    /// since last enrichment. Returns (entity_id, entity_type, enriched_at) tuples.
    pub fn get_stale_entity_intelligence(
        &self,
        stale_days: i32,
    ) -> Result<Vec<(String, String, String)>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT ei.entity_id, ei.entity_type, ei.enriched_at
             FROM entity_intelligence ei
             WHERE ei.enriched_at < datetime('now', ?1 || ' days')
               AND EXISTS (
                   SELECT 1 FROM content_index ci
                   WHERE ci.entity_id = ei.entity_id
                     AND ci.modified_at > ei.enriched_at
               )
             ORDER BY ei.enriched_at ASC",
        )?;
        let days_param = format!("-{stale_days}");
        let rows = stmt.query_map(params![days_param], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Content files with no extracted summary. These can be backfilled mechanically.
    pub fn get_unsummarized_content_files(&self) -> Result<Vec<DbContentFile>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, entity_id, entity_type, filename, relative_path, absolute_path,
                    format, file_size, modified_at, indexed_at, extracted_at, summary,
                    embeddings_generated_at, content_type, priority
             FROM content_index
             WHERE summary IS NULL
               AND format IN ('Markdown', 'PlainText', 'Pdf', 'Docx', 'Xlsx', 'Pptx', 'Html', 'Rtf')
             ORDER BY priority DESC, modified_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(DbContentFile {
                id: row.get(0)?,
                entity_id: row.get(1)?,
                entity_type: row.get(2)?,
                filename: row.get(3)?,
                relative_path: row.get(4)?,
                absolute_path: row.get(5)?,
                format: row.get(6)?,
                file_size: row.get(7)?,
                modified_at: row.get(8)?,
                indexed_at: row.get(9)?,
                extracted_at: row.get(10)?,
                summary: row.get(11)?,
                embeddings_generated_at: row.get(12)?,
                content_type: row.get(13)?,
                priority: row.get(14)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Meetings with an account_id (legacy column) but no meeting_entities junction row.
    /// These are orphaned from the Sprint 9 M2M refactor.
    pub fn get_orphaned_meetings(&self, days_back: i32) -> Result<Vec<DbMeeting>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT mh.id, mh.title, mh.meeting_type, mh.start_time, mh.end_time,
                    mh.account_id, mh.attendees, mh.notes_path, mh.summary,
                    mh.created_at, mh.calendar_event_id
             FROM meetings_history mh
             LEFT JOIN meeting_entities me ON me.meeting_id = mh.id
             WHERE mh.account_id IS NOT NULL AND mh.account_id != ''
               AND me.meeting_id IS NULL
               AND mh.start_time >= datetime('now', ?1 || ' days')
             ORDER BY mh.start_time DESC",
        )?;
        let days_param = format!("-{days_back}");
        let rows = stmt.query_map(params![days_param], |row| {
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
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Update a person's relationship classification.
    pub fn update_person_relationship(
        &self,
        person_id: &str,
        relationship: &str,
    ) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE people SET relationship = ?1, updated_at = ?3 WHERE id = ?2",
            params![relationship, person_id, now],
        )?;
        Ok(())
    }

    /// Recompute a person's meeting count from the meeting_attendees junction table.
    pub fn recompute_person_meeting_count(&self, person_id: &str) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE people SET meeting_count = (
                SELECT COUNT(*) FROM meeting_attendees WHERE person_id = ?1
             ), updated_at = ?2
             WHERE id = ?1",
            params![person_id, now],
        )?;
        Ok(())
    }

    /// Update a person's name (for email display name resolution).
    pub fn update_person_name(&self, person_id: &str, name: &str) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE people SET name = ?1, updated_at = ?3 WHERE id = ?2",
            params![name, person_id, now],
        )?;
        Ok(())
    }

    /// Merge two people: transfer all references from `remove_id` to `keep_id`, then delete `remove_id`.
    ///
    /// Transfers meeting attendees, entity links, and action associations.
    /// Uses INSERT OR IGNORE to handle overlapping meeting/entity links gracefully.
    pub fn merge_people(&self, keep_id: &str, remove_id: &str) -> Result<(), DbError> {
        // Verify both exist
        let keep = self
            .get_person(keep_id)?
            .ok_or_else(|| DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows))?;
        let _remove = self
            .get_person(remove_id)?
            .ok_or_else(|| DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows))?;

        // 1. Transfer meeting_attendees (INSERT OR IGNORE handles shared meetings)
        self.conn.execute(
            "INSERT OR IGNORE INTO meeting_attendees (meeting_id, person_id)
             SELECT meeting_id, ?1 FROM meeting_attendees WHERE person_id = ?2",
            params![keep_id, remove_id],
        )?;
        self.conn.execute(
            "DELETE FROM meeting_attendees WHERE person_id = ?1",
            params![remove_id],
        )?;

        // 2. Transfer entity_people links
        self.conn.execute(
            "INSERT OR IGNORE INTO entity_people (entity_id, person_id, relationship_type)
             SELECT entity_id, ?1, relationship_type FROM entity_people WHERE person_id = ?2",
            params![keep_id, remove_id],
        )?;
        self.conn.execute(
            "DELETE FROM entity_people WHERE person_id = ?1",
            params![remove_id],
        )?;

        // 3. Transfer actions
        self.conn.execute(
            "UPDATE actions SET person_id = ?1 WHERE person_id = ?2",
            params![keep_id, remove_id],
        )?;

        // 4. Delete removed person's intelligence cache
        self.conn.execute(
            "DELETE FROM entity_intelligence WHERE entity_id = ?1",
            params![remove_id],
        )?;

        // 5. Delete removed person's entity row
        self.conn.execute(
            "DELETE FROM entities WHERE id = ?1 AND entity_type = 'person'",
            params![remove_id],
        )?;

        // 6. Delete removed person's content_index rows
        self.conn.execute(
            "DELETE FROM content_index WHERE entity_id = ?1",
            params![remove_id],
        )?;

        // 6b. Transfer email aliases from removed person to kept person
        self.conn.execute(
            "UPDATE OR IGNORE person_emails SET person_id = ?1 WHERE person_id = ?2",
            params![keep_id, remove_id],
        )?;
        // Clean up any that couldn't be transferred (duplicate email for same person)
        self.conn.execute(
            "DELETE FROM person_emails WHERE person_id = ?1",
            params![remove_id],
        )?;
        // Ensure the removed person's primary email is recorded as an alias of the kept person
        self.add_person_email(keep_id, &_remove.email, false)?;

        // 7. Delete removed person
        self.conn
            .execute("DELETE FROM people WHERE id = ?1", params![remove_id])?;

        // 8. Recompute kept person's meeting count
        self.recompute_person_meeting_count(keep_id)?;

        // Merge notes if the removed person had any
        if let Some(ref remove_notes) = _remove.notes {
            if !remove_notes.is_empty() {
                let merged_notes = match &keep.notes {
                    Some(existing) if !existing.is_empty() => {
                        format!(
                            "{}\n\n--- Merged from {} ---\n{}",
                            existing, _remove.name, remove_notes
                        )
                    }
                    _ => format!("--- Merged from {} ---\n{}", _remove.name, remove_notes),
                };
                let now = Utc::now().to_rfc3339();
                self.conn.execute(
                    "UPDATE people SET notes = ?1, updated_at = ?2 WHERE id = ?3",
                    params![merged_notes, now, keep_id],
                )?;
            }
        }

        Ok(())
    }

    /// Delete a person and all their references (attendance, entity links, actions, intelligence).
    pub fn delete_person(&self, person_id: &str) -> Result<(), DbError> {
        // Verify exists
        let _person = self
            .get_person(person_id)?
            .ok_or_else(|| DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows))?;

        // Cascade deletes
        self.conn.execute(
            "DELETE FROM meeting_attendees WHERE person_id = ?1",
            params![person_id],
        )?;
        self.conn.execute(
            "DELETE FROM entity_people WHERE person_id = ?1",
            params![person_id],
        )?;
        self.conn.execute(
            "UPDATE actions SET person_id = NULL WHERE person_id = ?1",
            params![person_id],
        )?;
        self.conn.execute(
            "DELETE FROM entity_intelligence WHERE entity_id = ?1",
            params![person_id],
        )?;
        self.conn.execute(
            "DELETE FROM entities WHERE id = ?1 AND entity_type = 'person'",
            params![person_id],
        )?;
        self.conn.execute(
            "DELETE FROM content_index WHERE entity_id = ?1",
            params![person_id],
        )?;
        self.conn.execute(
            "DELETE FROM person_emails WHERE person_id = ?1",
            params![person_id],
        )?;
        self.conn
            .execute("DELETE FROM people WHERE id = ?1", params![person_id])?;

        Ok(())
    }

    // =========================================================================
    // Archive (Sprint 12)
    // =========================================================================

    /// Archive or unarchive an account. Cascade: archiving a parent archives all children.
    pub fn archive_account(&self, id: &str, archived: bool) -> Result<usize, DbError> {
        let val = if archived { 1 } else { 0 };
        let now = Utc::now().to_rfc3339();

        // Archive/unarchive the account itself
        let changed = self.conn.execute(
            "UPDATE accounts SET archived = ?1, updated_at = ?2 WHERE id = ?3",
            params![val, now, id],
        )?;

        // If archiving a parent, cascade to children
        if archived {
            self.conn.execute(
                "UPDATE accounts SET archived = 1, updated_at = ?1 WHERE parent_id = ?2",
                params![now, id],
            )?;
        }

        Ok(changed)
    }

    /// Archive or unarchive a project.
    pub fn archive_project(&self, id: &str, archived: bool) -> Result<usize, DbError> {
        let val = if archived { 1 } else { 0 };
        let now = Utc::now().to_rfc3339();
        Ok(self.conn.execute(
            "UPDATE projects SET archived = ?1, updated_at = ?2 WHERE id = ?3",
            params![val, now, id],
        )?)
    }

    /// Archive or unarchive a person.
    pub fn archive_person(&self, id: &str, archived: bool) -> Result<usize, DbError> {
        let val = if archived { 1 } else { 0 };
        let now = Utc::now().to_rfc3339();
        Ok(self.conn.execute(
            "UPDATE people SET archived = ?1, updated_at = ?2 WHERE id = ?3",
            params![val, now, id],
        )?)
    }

    /// Get archived accounts (top-level + children).
    pub fn get_archived_accounts(&self) -> Result<Vec<DbAccount>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, lifecycle, arr, health, contract_start, contract_end,
                    nps, tracker_path, parent_id, is_internal, updated_at, archived,
                    keywords, keywords_extracted_at
             FROM accounts WHERE archived = 1 ORDER BY name",
        )?;
        let rows = stmt.query_map([], Self::map_account_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Get archived projects.
    pub fn get_archived_projects(&self) -> Result<Vec<DbProject>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, status, milestone, owner, target_date, tracker_path, updated_at, archived,
                    keywords, keywords_extracted_at
             FROM projects WHERE archived = 1 ORDER BY name",
        )?;
        let rows = stmt.query_map([], Self::map_project_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Get archived people with signals.
    pub fn get_archived_people_with_signals(&self) -> Result<Vec<PersonListItem>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT p.id, p.email, p.name, p.organization, p.role, p.relationship,
                    p.notes, p.tracker_path, p.last_seen, p.first_seen, p.meeting_count,
                    p.updated_at, p.archived,
                    COALESCE(m30.cnt, 0) as freq_30d,
                    COALESCE(m90.cnt, 0) as freq_90d,
                    acct_names.names AS account_names
             FROM people p
             LEFT JOIN (
                 SELECT person_id, COUNT(*) as cnt
                 FROM meeting_attendees ma
                 JOIN meetings_history mh ON ma.meeting_id = mh.id
                 WHERE mh.start_time >= datetime('now', '-30 days')
                 GROUP BY person_id
             ) m30 ON m30.person_id = p.id
             LEFT JOIN (
                 SELECT person_id, COUNT(*) as cnt
                 FROM meeting_attendees ma
                 JOIN meetings_history mh ON ma.meeting_id = mh.id
                 WHERE mh.start_time >= datetime('now', '-90 days')
                 GROUP BY person_id
             ) m90 ON m90.person_id = p.id
             LEFT JOIN (
                 SELECT ep.person_id, GROUP_CONCAT(e.name, ', ') AS names
                 FROM entity_people ep
                 JOIN entities e ON e.id = ep.entity_id AND e.entity_type = 'account'
                 GROUP BY ep.person_id
             ) acct_names ON acct_names.person_id = p.id
             WHERE p.archived = 1
             ORDER BY p.name",
        )?;
        let rows = stmt.query_map([], |row| {
            let last_seen: Option<String> = row.get(8)?;
            let freq_30d: i32 = row.get(13)?;
            let freq_90d: i32 = row.get(14)?;
            let temperature = Self::compute_temperature(&last_seen);
            let trend = Self::compute_trend(freq_30d, freq_90d);
            Ok(PersonListItem {
                id: row.get(0)?,
                email: row.get(1)?,
                name: row.get(2)?,
                organization: row.get(3)?,
                role: row.get(4)?,
                relationship: row.get(5)?,
                notes: row.get(6)?,
                tracker_path: row.get(7)?,
                last_seen,
                first_seen: row.get(9)?,
                meeting_count: row.get(10)?,
                updated_at: row.get(11)?,
                archived: row.get::<_, i32>(12).unwrap_or(0) != 0,
                temperature,
                trend,
                account_names: row.get(15)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    // =========================================================================
    // Account Events (I143 — renewal tracking)
    // =========================================================================

    /// Record a lifecycle event for an account.
    pub fn record_account_event(
        &self,
        account_id: &str,
        event_type: &str,
        event_date: &str,
        arr_impact: Option<f64>,
        notes: Option<&str>,
    ) -> Result<i64, DbError> {
        self.conn.execute(
            "INSERT INTO account_events (account_id, event_type, event_date, arr_impact, notes)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![account_id, event_type, event_date, arr_impact, notes],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Get lifecycle events for an account, ordered by date descending.
    pub fn get_account_events(&self, account_id: &str) -> Result<Vec<DbAccountEvent>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, account_id, event_type, event_date, arr_impact, notes, created_at
             FROM account_events WHERE account_id = ?1 ORDER BY event_date DESC, id DESC",
        )?;
        let rows = stmt.query_map(params![account_id], |row| {
            Ok(DbAccountEvent {
                id: row.get(0)?,
                account_id: row.get(1)?,
                event_type: row.get(2)?,
                event_date: row.get(3)?,
                arr_impact: row.get(4)?,
                notes: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Check if an account has any churn events (for auto-rollover logic).
    pub fn has_churn_event(&self, account_id: &str) -> Result<bool, DbError> {
        let count: i32 = self.conn.query_row(
            "SELECT COUNT(*) FROM account_events WHERE account_id = ?1 AND event_type = 'churn'",
            params![account_id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// Get accounts with renewal_date (contract_end) in the past and no churn event.
    pub fn get_accounts_past_renewal(&self) -> Result<Vec<DbAccount>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT a.id, a.name, a.lifecycle, a.arr, a.health, a.contract_start, a.contract_end,
                    a.nps, a.tracker_path, a.parent_id, a.is_internal, a.updated_at, a.archived
             FROM accounts a
             WHERE a.contract_end IS NOT NULL
               AND a.contract_end < date('now')
               AND a.archived = 0
               AND a.id NOT IN (
                   SELECT account_id FROM account_events WHERE event_type = 'churn'
               )
             ORDER BY a.contract_end",
        )?;
        let rows = stmt.query_map([], Self::map_account_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    // =========================================================================
    // Chat Sessions (Sprint 26)
    // =========================================================================

    pub fn create_chat_session(
        &self,
        id: &str,
        entity_id: Option<&str>,
        entity_type: Option<&str>,
        session_start: &str,
        created_at: &str,
    ) -> Result<DbChatSession, DbError> {
        self.conn.execute(
            "INSERT INTO chat_sessions (
                id, entity_id, entity_type, session_start, session_end, turn_count, last_message, created_at
             ) VALUES (?1, ?2, ?3, ?4, NULL, 0, NULL, ?5)",
            params![id, entity_id, entity_type, session_start, created_at],
        )?;
        Ok(DbChatSession {
            id: id.to_string(),
            entity_id: entity_id.map(|s| s.to_string()),
            entity_type: entity_type.map(|s| s.to_string()),
            session_start: session_start.to_string(),
            session_end: None,
            turn_count: 0,
            last_message: None,
            created_at: created_at.to_string(),
        })
    }

    pub fn get_open_chat_session(
        &self,
        entity_id: Option<&str>,
        entity_type: Option<&str>,
    ) -> Result<Option<DbChatSession>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, entity_id, entity_type, session_start, session_end, turn_count, last_message, created_at
             FROM chat_sessions
             WHERE session_end IS NULL
               AND (
                    (?1 IS NULL AND entity_id IS NULL AND entity_type IS NULL)
                    OR (entity_id = ?1 AND ((?2 IS NULL AND entity_type IS NULL) OR entity_type = ?2))
               )
             ORDER BY session_start DESC
             LIMIT 1",
        )?;
        let mut rows = stmt.query(params![entity_id, entity_type])?;
        if let Some(row) = rows.next()? {
            Ok(Some(DbChatSession {
                id: row.get(0)?,
                entity_id: row.get(1)?,
                entity_type: row.get(2)?,
                session_start: row.get(3)?,
                session_end: row.get(4)?,
                turn_count: row.get(5)?,
                last_message: row.get(6)?,
                created_at: row.get(7)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn get_next_chat_turn_index(&self, session_id: &str) -> Result<i32, DbError> {
        let idx: i32 = self.conn.query_row(
            "SELECT COALESCE(MAX(turn_index) + 1, 0) FROM chat_turns WHERE session_id = ?1",
            params![session_id],
            |row| row.get(0),
        )?;
        Ok(idx)
    }

    pub fn append_chat_turn(
        &self,
        id: &str,
        session_id: &str,
        turn_index: i32,
        role: &str,
        content: &str,
        timestamp: &str,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT INTO chat_turns (id, session_id, turn_index, role, content, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, session_id, turn_index, role, content, timestamp],
        )?;
        Ok(())
    }

    pub fn bump_chat_session_stats(
        &self,
        session_id: &str,
        turn_delta: i32,
        last_message: Option<&str>,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "UPDATE chat_sessions
             SET turn_count = turn_count + ?1,
                 last_message = COALESCE(?2, last_message)
             WHERE id = ?3",
            params![turn_delta, last_message, session_id],
        )?;
        Ok(())
    }

    pub fn get_chat_session_turns(
        &self,
        session_id: &str,
        limit: usize,
    ) -> Result<Vec<DbChatTurn>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, turn_index, role, content, timestamp
             FROM chat_turns
             WHERE session_id = ?1
             ORDER BY turn_index ASC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![session_id, limit as i64], |row| {
            Ok(DbChatTurn {
                id: row.get(0)?,
                session_id: row.get(1)?,
                turn_index: row.get(2)?,
                role: row.get(3)?,
                content: row.get(4)?,
                timestamp: row.get(5)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Compute temperature from an optional last-seen timestamp.
    fn compute_temperature(last_seen: &Option<String>) -> String {
        match last_seen {
            Some(dt) => compute_temperature(dt),
            None => "cold".to_string(),
        }
    }

    /// Compute trend from 30d and 90d frequencies.
    fn compute_trend(freq_30d: i32, freq_90d: i32) -> String {
        compute_trend(freq_30d, freq_90d)
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
        let db = ActionDb::open_at(path).expect("Failed to open test database");
        // Disable FK enforcement for unit tests — FK integrity is validated
        // separately in migration tests and production open_at() enables it.
        db.conn_ref()
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .expect("disable FK for tests");
        db
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
            person_id: None,
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
            account_id: Some("acme-corp".to_string()),
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
            };
            db.upsert_meeting(&meeting).expect("upsert");
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
            account_id: None,
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
            };
            db.upsert_meeting(&meeting).expect("insert meeting");
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
            account_id: None,
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
            account_id: None,
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
    fn test_upsert_meeting_auto_links_junction() {
        let db = test_db();
        let now = Utc::now().to_rfc3339();

        // Create account (required: upsert_meeting now resolves name → ID via accounts table)
        db.conn
            .execute(
                "INSERT INTO accounts (id, name, updated_at) VALUES (?1, ?2, ?3)",
                params!["acme-auto", "Acme Auto", &now],
            )
            .expect("insert account");

        // Create entity row (mirrors account, as ensure_entity_for_account would)
        db.conn
            .execute(
                "INSERT INTO entities (id, name, entity_type, updated_at) VALUES (?1, ?2, ?3, ?4)",
                params!["acme-auto", "Acme Auto", "account", &now],
            )
            .expect("insert entity");

        // account_id is the display name (as passed from directive), not the slugified ID
        let meeting = DbMeeting {
            id: "mtg-auto".to_string(),
            title: "Auto-link Test".to_string(),
            meeting_type: "customer".to_string(),
            start_time: now.clone(),
            end_time: None,
            account_id: Some("Acme Auto".to_string()),
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
        };
        db.upsert_meeting(&meeting).expect("upsert");

        // Junction should be auto-populated with the slugified entity ID
        let count: i32 = db
            .conn
            .query_row(
                "SELECT COUNT(*) FROM meeting_entities WHERE meeting_id = ?1 AND entity_id = ?2",
                params!["mtg-auto", "acme-auto"],
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

    #[test]
    fn test_backfill_junction_from_account_id() {
        // Simulate what the migration does: meetings with account_id get junction entries
        let db = test_db();
        let now = Utc::now().to_rfc3339();

        // Insert a meeting with account_id directly (simulating pre-junction data)
        db.conn
            .execute(
                "INSERT INTO meetings_history (id, title, meeting_type, start_time, account_id, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params!["mtg-bf", "Backfill Test", "customer", &now, "acme-bf", &now],
            )
            .expect("insert");

        // Run the backfill SQL
        db.conn
            .execute_batch(
                "INSERT OR IGNORE INTO meeting_entities (meeting_id, entity_id, entity_type)
                 SELECT id, account_id, 'account' FROM meetings_history
                 WHERE account_id IS NOT NULL AND account_id != '';",
            )
            .expect("backfill");

        let count: i32 = db
            .conn
            .query_row(
                "SELECT COUNT(*) FROM meeting_entities WHERE meeting_id = 'mtg-bf' AND entity_id = 'acme-bf'",
                [],
                |row| row.get(0),
            )
            .expect("count");
        assert_eq!(count, 1);
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
            account_id: None,
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
