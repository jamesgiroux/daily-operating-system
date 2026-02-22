//! Shared type definitions for the database layer.

use chrono::Utc;
use serde::{Deserialize, Serialize};
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
    pub account_name: Option<String>,
    /// Next upcoming meeting title for the action's account (I342).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_meeting_title: Option<String>,
    /// Next upcoming meeting start time for the action's account (I342).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_meeting_start: Option<String>,
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
    /// JSON metadata for preset-driven custom fields (I311).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<String>,
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
    /// Intelligence lifecycle state (ADR-0081).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intelligence_state: Option<String>,
    /// Intelligence quality level (ADR-0081).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intelligence_quality: Option<String>,
    /// UTC timestamp of last enrichment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_enriched_at: Option<String>,
    /// Number of signals associated with this meeting.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal_count: Option<i32>,
    /// Whether new signals have arrived since last view.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_new_signals: Option<i32>,
    /// UTC timestamp of last user view.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_viewed_at: Option<String>,
}

pub struct EnsureMeetingHistoryInput<'a> {
    pub id: &'a str,
    pub title: &'a str,
    pub meeting_type: &'a str,
    pub start_time: &'a str,
    pub end_time: Option<&'a str>,
    pub calendar_event_id: Option<&'a str>,
    pub attendees: Option<&'a str>,
    pub description: Option<&'a str>,
}

/// Outcome of syncing a meeting into meetings_history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeetingSyncOutcome {
    /// Meeting was newly inserted.
    New,
    /// Meeting already existed but title or start_time changed.
    Changed,
    /// Meeting already existed and nothing meaningful changed.
    Unchanged,
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

/// A row from the `emails` table (I368).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbEmail {
    pub email_id: String,
    pub thread_id: Option<String>,
    pub sender_email: Option<String>,
    pub sender_name: Option<String>,
    pub subject: Option<String>,
    pub snippet: Option<String>,
    pub priority: Option<String>,
    pub is_unread: bool,
    pub received_at: Option<String>,
    pub enrichment_state: String,
    pub enrichment_attempts: i32,
    pub last_enrichment_at: Option<String>,
    pub last_seen_at: Option<String>,
    pub resolved_at: Option<String>,
    pub entity_id: Option<String>,
    pub entity_type: Option<String>,
    pub contextual_summary: Option<String>,
    pub sentiment: Option<String>,
    pub urgency: Option<String>,
    pub user_is_last_sender: bool,
    pub last_sender_email: Option<String>,
    pub message_count: i32,
    pub created_at: String,
    pub updated_at: String,
    /// Relevance score from scoring pipeline (I395).
    pub relevance_score: Option<f64>,
    /// Human-readable score reason (I395).
    pub score_reason: Option<String>,
}

/// Email sync statistics for the frontend sync status indicator (I373).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmailSyncStats {
    pub last_fetch_at: Option<String>,
    pub total: i32,
    pub enriched: i32,
    pub pending: i32,
    pub failed: i32,
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
    /// JSON metadata for preset-driven custom fields (I311).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<String>,
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

pub(crate) fn default_quill_source() -> String {
    "quill".to_string()
}

/// Row mapper for quill_sync_state SELECT queries (15 columns including source).
pub(crate) fn map_sync_row(row: &rusqlite::Row) -> rusqlite::Result<DbQuillSyncState> {
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
pub(crate) fn compute_temperature(last_meeting_iso: &str) -> String {
    let days = days_since_iso(last_meeting_iso);
    match days {
        Some(d) if d < 7 => "hot".to_string(),
        Some(d) if d < 30 => "warm".to_string(),
        Some(d) if d < 60 => "cool".to_string(),
        _ => "cold".to_string(),
    }
}

/// Compute meeting trend from 30d vs 90d frequency.
pub(crate) fn compute_trend(count_30d: i32, count_90d: i32) -> String {
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

/// Result of merging two accounts (I198).
#[derive(Debug, serde::Serialize)]
pub struct MergeResult {
    pub actions_moved: usize,
    pub meetings_moved: usize,
    pub people_moved: usize,
    pub events_moved: usize,
    pub children_moved: usize,
}
