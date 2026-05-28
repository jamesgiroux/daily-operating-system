//! Type contracts for meeting-row writes.
//!
//! `WriteRequest` is the single shape every production writer sends. The
//! `source` discriminant selects which adapter handles it, which determines
//! which subset of fields are required and which transformations apply.
//!
//! Why a single shape: prior to this substrate every write path had its own
//! ad-hoc field set, and "is end_time required?" / "is calendar_event_id
//! required?" answers diverged silently. The matrix lives in `invariants.rs`;
//! `WriteRequest` carries the union, and `MeetingSource` declares which row a
//! caller is authoring.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeetingSource {
    /// Google Calendar polling and attendance-batch ingestion. Authoritative
    /// for `calendar_event_id`, `end_time`, `attendees`.
    Calendar,
    /// Workspace reconcile (`workflow/reconcile.rs`). Persists meetings from
    /// briefing markdown / prep snapshots; `calendar_event_id` is optional
    /// because some legacy rows predate calendar polling.
    Reconcile,
    /// User-driven entity override or manual add (`services/meetings.rs`).
    /// Never carries `calendar_event_id` — these rows are stub history records
    /// the user is annotating.
    Manual,
    /// Historical filesystem backfill (`backfill_meetings.rs`). Initializes
    /// `intelligence_state = 'archived'` because the source material is
    /// already settled (notes / prep files on disk).
    Backfill,
}

impl MeetingSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            MeetingSource::Calendar => "calendar",
            MeetingSource::Reconcile => "reconcile",
            MeetingSource::Manual => "manual",
            MeetingSource::Backfill => "backfill",
        }
    }
}

/// One meeting-row write request. Carriers fill the fields their source
/// authoritatively knows; adapters validate via `invariants::validate`.
#[derive(Debug, Clone)]
pub struct WriteRequest {
    pub source: MeetingSource,
    pub id: String,
    pub title: String,
    pub meeting_type: String,
    pub start_time: String,
    pub end_time: Option<String>,
    pub calendar_event_id: Option<String>,
    pub attendees: Option<String>,
    pub description: Option<String>,
    pub notes_path: Option<String>,
    pub transcript_path: Option<String>,
    pub prep_context_json: Option<String>,
    pub user_agenda_json: Option<String>,
    pub user_notes: Option<String>,
    pub intelligence_state: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteOutcome {
    /// Row did not exist and was inserted.
    Created,
    /// Row existed and meaningful fields changed.
    Updated,
    /// Row existed and no meaningful fields changed (no-op write).
    Unchanged,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WriteError {
    #[error("missing required field for {meeting_source}: {field}")]
    MissingRequiredField {
        meeting_source: &'static str,
        field: &'static str,
    },
    #[error("forbidden field for {meeting_source}: {field}")]
    ForbiddenField {
        meeting_source: &'static str,
        field: &'static str,
    },
    #[error("invalid value for {field}: {reason}")]
    InvalidValue {
        field: &'static str,
        reason: String,
    },
    #[error("database write failed: {0}")]
    Database(String),
}

impl From<crate::db::DbError> for WriteError {
    fn from(value: crate::db::DbError) -> Self {
        WriteError::Database(value.to_string())
    }
}
