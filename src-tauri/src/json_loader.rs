//! JSON data loader with markdown fallback
//!
//! This module provides functions to load data from JSON files in the `_today/data/`
//! directory, falling back to markdown parsing when JSON is not available.
//!
//! Migration strategy:
//! 1. Check for `_today/data/` directory
//! 2. If JSON exists and is valid, use it (fast path)
//! 3. If JSON missing or invalid, fall back to markdown parsing (legacy path)

use std::fs;
use std::path::Path;

use crate::types::LinkedEntity;

/// Whether the data in _today/data/ is from today or stale
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "freshness", rename_all = "camelCase")]
pub enum DataFreshness {
    Fresh {
        generated_at: String,
    },
    Stale {
        data_date: String,
        generated_at: String,
    },
    Unknown,
}

/// JSON schedule format
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonSchedule {
    pub date: String,
    pub greeting: Option<String>,
    pub summary: Option<String>,
    pub focus: Option<String>,
    pub meetings: Vec<JsonMeeting>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonMeeting {
    pub id: String,
    pub calendar_event_id: Option<String>,
    pub time: String,
    pub end_time: Option<String>,
    #[serde(default)]
    pub start_iso: Option<String>,
    pub title: String,
    #[serde(rename = "type")]
    pub meeting_type: String,
    pub account: Option<String>,
    #[serde(default)]
    pub is_current: bool,
    pub has_prep: bool,
    pub prep_file: Option<String>,
    pub prep_summary: Option<JsonPrepSummary>,
    /// Entities linked via M2M junction table or entity resolution
    #[serde(default)]
    pub linked_entities: Option<Vec<LinkedEntity>>,
    /// Raw calendar attendees from Google Calendar (not AI-enriched)
    #[serde(default, rename = "calendarAttendees")]
    pub calendar_attendees: Option<Vec<JsonCalendarAttendee>>,
    /// Calendar event description from Google Calendar
    #[serde(default, rename = "calendarDescription")]
    pub calendar_description: Option<String>,
}

/// Raw attendee from Google Calendar event.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct JsonCalendarAttendee {
    pub email: String,
    pub name: String,
    #[serde(default)]
    pub rsvp: String,
    #[serde(default)]
    pub domain: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonPrepSummary {
    pub at_a_glance: Option<Vec<String>>,
    pub discuss: Option<Vec<String>>,
    pub watch: Option<Vec<String>>,
    pub wins: Option<Vec<String>>,
    pub context: Option<String>,
    pub stakeholders: Option<Vec<JsonStakeholder>>,
    pub open_items: Option<Vec<String>>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonStakeholder {
    pub name: String,
    pub role: Option<String>,
    pub focus: Option<String>,
}

// load_actions_json and supporting types removed — DB is the source of truth.
// sync_actions_to_db (JSON→DB) was the only caller and has been eliminated.

// =============================================================================
// Directive Loading (ADR-0042: per-operation pipelines)
// =============================================================================

/// The today-directive.json produced by Phase 1 (prepare_today.py).
///
/// Uses serde defaults throughout so missing keys don't cause parse failures.
/// The Rust delivery functions read what they need; unknown fields are ignored.
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct Directive {
    #[serde(default)]
    pub context: DirectiveContext,
    #[serde(default)]
    pub calendar: DirectiveCalendar,
    #[serde(default)]
    pub meetings: std::collections::HashMap<String, Vec<DirectiveMeeting>>,
    #[serde(default)]
    pub meeting_contexts: Vec<DirectiveMeetingContext>,
    #[serde(default)]
    pub actions: DirectiveActions,
    #[serde(default)]
    pub emails: DirectiveEmails,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct DirectiveContext {
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub profile: Option<String>,
    #[serde(default)]
    pub greeting: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub focus: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct DirectiveCalendar {
    #[serde(default)]
    pub events: Vec<DirectiveEvent>,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct DirectiveEvent {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub start: Option<String>,
    #[serde(default)]
    pub end: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    /// Raw attendee email list from Google Calendar.
    #[serde(default)]
    pub attendees: Vec<String>,
    /// Email → display name map from Google Calendar.
    #[serde(default)]
    pub attendee_names: std::collections::HashMap<String, String>,
    /// Email → RSVP status map (accepted/tentative/declined/needsAction).
    #[serde(default)]
    pub attendee_rsvp: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct DirectiveMeeting {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub event_id: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub start: Option<String>,
    #[serde(default)]
    pub end: Option<String>,
    #[serde(default)]
    pub account: Option<String>,
    #[serde(default)]
    pub start_display: Option<String>,
    #[serde(default)]
    pub end_display: Option<String>,
    #[serde(rename = "type", default)]
    pub meeting_type: Option<String>,
    /// Resolved entities from entity-generic classification.
    #[serde(default)]
    pub entities: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct DirectiveMeetingContext {
    #[serde(default)]
    pub event_id: Option<String>,
    #[serde(default)]
    pub account: Option<String>,
    #[serde(default)]
    pub account_data: Option<serde_json::Value>,
    /// Resolved entity ID (account, project, or person).
    #[serde(default)]
    pub entity_id: Option<String>,
    /// Resolved entity type ("account", "project", "person").
    #[serde(default)]
    pub entity_type: Option<String>,
    /// Structured primary entity data.
    #[serde(default)]
    pub primary_entity: Option<serde_json::Value>,
    /// Project-specific data when entity is a project.
    #[serde(default)]
    pub project_data: Option<serde_json::Value>,
    /// Person-specific data when entity is a person.
    #[serde(default)]
    pub person_data: Option<serde_json::Value>,
    /// Relationship signals when entity is a person.
    #[serde(default)]
    pub relationship_signals: Option<serde_json::Value>,
    /// Shared entities (accounts/projects) when entity is a person.
    #[serde(default)]
    pub shared_entities: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub attendees: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub narrative: Option<String>,
    #[serde(default)]
    pub talking_points: Option<Vec<String>>,
    #[serde(default)]
    pub risks: Option<Vec<String>>,
    #[serde(default)]
    pub wins: Option<Vec<String>>,
    #[serde(default)]
    pub questions: Option<Vec<String>>,
    #[serde(default)]
    pub key_principles: Option<Vec<String>>,
    #[serde(default)]
    pub since_last: Option<Vec<String>>,
    #[serde(default)]
    pub current_state: Option<Vec<String>>,
    #[serde(default)]
    pub strategic_programs: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub open_items: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub references: Option<Vec<serde_json::Value>>,
    // Raw data from meeting_context.rs (SQLite queries) — used to synthesize prep content
    #[serde(default)]
    pub open_actions: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub recent_captures: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub meeting_history: Option<Vec<serde_json::Value>>,
    // Entity intelligence (from intelligence.json) — persistent prep context
    #[serde(default)]
    pub executive_assessment: Option<String>,
    #[serde(default)]
    pub entity_risks: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub entity_readiness: Option<Vec<String>>,
    #[serde(default)]
    pub stakeholder_insights: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub recent_email_signals: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub consistency_status: Option<crate::intelligence::ConsistencyStatus>,
    #[serde(default)]
    pub consistency_findings: Option<Vec<crate::intelligence::ConsistencyFinding>>,
    /// Calendar event description.
    #[serde(default)]
    pub description: Option<String>,
    /// Pre-meeting email context gathered from email signals/bridge.
    #[serde(default)]
    pub pre_meeting_email_context: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct DirectiveActions {
    #[serde(default)]
    pub overdue: Vec<DirectiveAction>,
    #[serde(default)]
    pub due_today: Vec<DirectiveAction>,
    #[serde(default)]
    pub due_this_week: Vec<DirectiveAction>,
    #[serde(default)]
    pub waiting_on: Vec<DirectiveWaiting>,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct DirectiveAction {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub account: Option<String>,
    #[serde(default)]
    pub due_date: Option<String>,
    #[serde(default, alias = "due")]
    pub due: Option<String>,
    #[serde(default)]
    pub days_overdue: Option<u32>,
    #[serde(default)]
    pub context: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
}

impl DirectiveAction {
    /// Get the due date, trying due_date first then due
    pub fn effective_due_date(&self) -> Option<&str> {
        self.due_date.as_deref().or(self.due.as_deref())
    }
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct DirectiveWaiting {
    #[serde(default)]
    pub what: Option<String>,
    #[serde(default)]
    pub who: Option<String>,
    #[serde(default)]
    pub context: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct DirectiveEmails {
    #[serde(default)]
    pub classified: Vec<DirectiveEmail>,
    #[serde(default)]
    pub high_priority: Vec<DirectiveEmail>,
    #[serde(default)]
    pub medium_count: u32,
    #[serde(default)]
    pub low_count: u32,
    #[serde(default, alias = "syncError")]
    pub sync_error: Option<DirectiveEmailSyncError>,
    /// AI-synthesized email narrative
    #[serde(default)]
    pub narrative: Option<String>,
    /// Threads awaiting user reply
    #[serde(default, alias = "repliesNeeded")]
    pub replies_needed: Vec<DirectiveReplyNeeded>,
}

/// A thread awaiting the user's reply ("ball in your court").
#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectiveReplyNeeded {
    #[serde(default)]
    pub thread_id: String,
    #[serde(default)]
    pub subject: String,
    #[serde(default)]
    pub from: String,
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub wait_duration: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct DirectiveEmail {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub from_email: Option<String>,
    #[serde(default)]
    pub subject: Option<String>,
    #[serde(default)]
    pub snippet: Option<String>,
    #[serde(default)]
    pub priority: Option<String>,
    /// RFC3339 timestamp of the most recent message in the thread (Gate 1 newness check).
    #[serde(default)]
    pub last_response_date: Option<String>,
    /// Message count in the thread for reference context.
    #[serde(default)]
    pub thread_message_count: Option<usize>,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectiveEmailSyncError {
    #[serde(default)]
    pub stage: Option<String>,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
}

/// Load the today-directive.json produced by Phase 1.
///
/// Checks `_today/data/today-directive.json` first, then falls back to
/// `_today/.today-directive.json` (legacy location).
pub fn load_directive(today_dir: &Path) -> Result<Directive, String> {
    let primary = today_dir.join("data").join("today-directive.json");
    let legacy = today_dir.join(".today-directive.json");

    let path = if primary.exists() {
        &primary
    } else if legacy.exists() {
        &legacy
    } else {
        return Err(format!(
            "Directive not found. Checked:\n  {}\n  {}",
            primary.display(),
            legacy.display()
        ));
    };

    let content =
        fs::read_to_string(path).map_err(|e| format!("Failed to read directive: {}", e))?;
    serde_json::from_str(&content).map_err(|e| format!("Failed to parse directive: {}", e))
}

// =============================================================================
// Week JSON Loading (Phase 3C)
// =============================================================================

// load_week_json removed — WeekOverview is now built from DB in services/dashboard.rs.
