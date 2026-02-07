use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::WorkflowError;

/// Configuration stored in ~/.dailyos/config.json
///
/// Accepts both Daybreak format (`workspacePath`) and DailyOS CLI format
/// (`default_workspace`) for backwards compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(alias = "default_workspace")]
    pub workspace_path: String,
    #[serde(default)]
    pub schedules: Schedules,
    #[serde(default = "default_profile")]
    pub profile: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_config: Option<ProfileConfig>,
    #[serde(default)]
    pub google: GoogleConfig,
    #[serde(default)]
    pub post_meeting_capture: PostMeetingCaptureConfig,
}

/// Profile-specific configuration (CSM users)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileConfig {
    /// Relative path to account tracker CSV within workspace
    #[serde(default)]
    pub account_tracker_path: Option<String>,
    /// How many days back to look for meeting history
    #[serde(default = "default_history_lookback")]
    pub history_lookback_days: u32,
    /// How many past meetings to include per account
    #[serde(default = "default_history_count")]
    pub history_meeting_count: u32,
}

fn default_history_lookback() -> u32 {
    30
}

fn default_history_count() -> u32 {
    3
}

fn default_profile() -> String {
    "general".to_string()
}

/// Schedule configuration for workflows
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Schedules {
    #[serde(default = "ScheduleEntry::default_today")]
    pub today: ScheduleEntry,
    #[serde(default = "ScheduleEntry::default_archive")]
    pub archive: ScheduleEntry,
    #[serde(default = "ScheduleEntry::default_inbox_batch")]
    pub inbox_batch: ScheduleEntry,
    #[serde(default = "ScheduleEntry::default_week")]
    pub week: ScheduleEntry,
}

impl Default for Schedules {
    fn default() -> Self {
        Self {
            today: ScheduleEntry::default_today(),
            archive: ScheduleEntry::default_archive(),
            inbox_batch: ScheduleEntry::default_inbox_batch(),
            week: ScheduleEntry::default_week(),
        }
    }
}

/// A single schedule entry
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleEntry {
    pub enabled: bool,
    pub cron: String,
    pub timezone: String,
}

impl ScheduleEntry {
    /// Default schedule for the "today" workflow: 8 AM weekdays
    pub fn default_today() -> Self {
        Self {
            enabled: true,
            cron: "0 8 * * 1-5".to_string(), // 8 AM weekdays
            timezone: "America/New_York".to_string(),
        }
    }

    /// Default schedule for the "archive" workflow: midnight daily
    pub fn default_archive() -> Self {
        Self {
            enabled: true,
            cron: "0 0 * * *".to_string(), // Midnight daily
            timezone: "America/New_York".to_string(),
        }
    }

    /// Default schedule for inbox batch processing: every 2 hours on weekdays
    pub fn default_inbox_batch() -> Self {
        Self {
            enabled: true,
            cron: "0 */2 * * 1-5".to_string(), // Every 2 hours, weekdays
            timezone: "America/New_York".to_string(),
        }
    }

    /// Default schedule for the "week" workflow: Monday 5 AM
    pub fn default_week() -> Self {
        Self {
            enabled: true,
            cron: "0 5 * * 1".to_string(), // Monday 5 AM
            timezone: "America/New_York".to_string(),
        }
    }
}

impl Default for ScheduleEntry {
    fn default() -> Self {
        Self::default_today()
    }
}

/// Workflow identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkflowId {
    Today,
    Archive,
    InboxBatch,
    Week,
}

impl std::fmt::Display for WorkflowId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkflowId::Today => write!(f, "today"),
            WorkflowId::Archive => write!(f, "archive"),
            WorkflowId::InboxBatch => write!(f, "inbox_batch"),
            WorkflowId::Week => write!(f, "week"),
        }
    }
}

impl std::str::FromStr for WorkflowId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "today" => Ok(WorkflowId::Today),
            "archive" => Ok(WorkflowId::Archive),
            "inbox_batch" | "inboxbatch" => Ok(WorkflowId::InboxBatch),
            "week" => Ok(WorkflowId::Week),
            _ => Err(format!("Unknown workflow: {}", s)),
        }
    }
}

/// Current phase of workflow execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkflowPhase {
    Preparing,
    Enriching,
    Delivering,
}

impl std::fmt::Display for WorkflowPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkflowPhase::Preparing => write!(f, "Preparing"),
            WorkflowPhase::Enriching => write!(f, "Enriching with AI"),
            WorkflowPhase::Delivering => write!(f, "Delivering"),
        }
    }
}

/// Current status of a workflow
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum WorkflowStatus {
    Idle,
    Running {
        #[serde(rename = "startedAt")]
        started_at: DateTime<Utc>,
        phase: WorkflowPhase,
        #[serde(rename = "executionId")]
        execution_id: String,
    },
    Completed {
        #[serde(rename = "finishedAt")]
        finished_at: DateTime<Utc>,
        #[serde(rename = "durationSecs")]
        duration_secs: u64,
        #[serde(rename = "executionId")]
        execution_id: String,
    },
    Failed {
        error: WorkflowError,
        #[serde(rename = "executionId")]
        execution_id: String,
    },
}

impl Default for WorkflowStatus {
    fn default() -> Self {
        WorkflowStatus::Idle
    }
}

/// Record of a workflow execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionRecord {
    pub id: String,
    pub workflow: WorkflowId,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub duration_secs: Option<u64>,
    pub success: bool,
    pub error_message: Option<String>,
    pub trigger: ExecutionTrigger,
}

/// What triggered the execution
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionTrigger {
    Scheduled,
    Manual,
    Missed,
}

/// High-level file type for inbox display
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InboxFileType {
    Markdown,
    Image,
    Spreadsheet,
    Document,
    Data,
    Text,
    Other,
}

/// A file in the _inbox/ directory
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InboxFile {
    pub filename: String,
    pub path: String,
    pub size_bytes: u64,
    pub modified: String,
    pub preview: Option<String>,
    pub file_type: InboxFileType,
}

/// Day overview parsed from _today/overview.md
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DayOverview {
    pub greeting: String,
    pub date: String,
    pub summary: String,
    pub focus: Option<String>,
}

/// Meeting type classification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeetingType {
    Customer,
    Qbr,
    Training,
    Internal,
    TeamSync,
    OneOnOne,
    Partnership,
    AllHands,
    External,
    Personal,
}

/// Stakeholder information for meeting prep
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Stakeholder {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focus: Option<String>,
}

/// Source reference for actions and context
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceReference {
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_updated: Option<String>,
}

/// Meeting prep details
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MeetingPrep {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risks: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wins: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stakeholders: Option<Vec<Stakeholder>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub questions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_items: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub historical_context: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_references: Option<Vec<SourceReference>>,
}

/// A single meeting
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Meeting {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calendar_event_id: Option<String>,
    pub time: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
    pub title: String,
    #[serde(rename = "type")]
    pub meeting_type: MeetingType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prep: Option<MeetingPrep>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_current: Option<bool>,
    /// Path to the prep file (e.g., "01-1630-customer-acme-prep.md")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prep_file: Option<String>,
    /// Whether this meeting has a dedicated prep file
    pub has_prep: bool,
}

/// Action priority level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Priority {
    P1,
    P2,
    P3,
}

/// Action completion status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActionStatus {
    Pending,
    Completed,
}

/// A single action item
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Action {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date: Option<String>,
    pub priority: Priority,
    pub status: ActionStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_overdue: Option<bool>,
    /// Additional context for the action
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    /// Source of the action (e.g., meeting, email)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Days overdue (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days_overdue: Option<i32>,
}

/// Daily statistics
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DayStats {
    pub total_meetings: usize,
    pub customer_meetings: usize,
    pub actions_due: usize,
    pub inbox_count: usize,
}

/// Email priority level (three-tier: high / medium / low)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EmailPriority {
    High,
    Medium,
    Low,
}

/// A single email needing attention
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Email {
    pub id: String,
    pub sender: String,
    pub sender_email: String,
    pub subject: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,
    pub priority: EmailPriority,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    /// AI-generated one-line summary of the email
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// Suggested next action (e.g. "Reply with counter-proposal")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommended_action: Option<String>,
    /// Thread history arc (e.g. "Initial outreach → follow-up → this response")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_arc: Option<String>,
    /// Email category from AI classification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_type: Option<String>,
}

/// Complete dashboard data payload
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardData {
    pub overview: DayOverview,
    pub stats: DayStats,
    pub meetings: Vec<Meeting>,
    pub actions: Vec<Action>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emails: Option<Vec<Email>>,
}

// =============================================================================
// Week Overview Types
// =============================================================================

/// Week overview parsed from week-00-overview.md
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeekOverview {
    pub week_number: String,
    pub date_range: String,
    pub days: Vec<WeekDay>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_summary: Option<WeekActionSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hygiene_alerts: Option<Vec<HygieneAlert>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focus_areas: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available_time_blocks: Option<Vec<TimeBlock>>,
}

/// A single day in the week overview
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeekDay {
    pub date: String,
    pub day_name: String,
    pub meetings: Vec<WeekMeeting>,
}

/// Simplified meeting info for week view
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeekMeeting {
    pub time: String,
    pub title: String,
    pub account: Option<String>,
    #[serde(rename = "type")]
    pub meeting_type: MeetingType,
    pub prep_status: PrepStatus,
}

/// Prep status for week view
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrepStatus {
    PrepNeeded,
    AgendaNeeded,
    BringUpdates,
    ContextNeeded,
    PrepReady,
    DraftReady,
    Done,
}

/// Weekly action summary
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeekActionSummary {
    pub overdue_count: usize,
    pub due_this_week: usize,
    pub critical_items: Vec<String>,
}

/// Account hygiene alert
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HygieneAlert {
    pub account: String,
    pub ring: Option<String>,
    pub arr: Option<String>,
    pub issue: String,
    pub severity: AlertSeverity,
}

/// Alert severity level
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlertSeverity {
    Critical,
    Warning,
    Info,
}

/// Available time block
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeBlock {
    pub day: String,
    pub start: String,
    pub end: String,
    pub duration_minutes: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_use: Option<String>,
}

// =============================================================================
// Focus Data Types
// =============================================================================

/// Focus suggestions parsed from 81-suggested-focus.md
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusData {
    pub priorities: Vec<FocusPriority>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_blocks: Option<Vec<TimeBlock>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quick_wins: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub energy_notes: Option<EnergyNotes>,
}

/// Priority tier for focus
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusPriority {
    pub level: String,
    pub label: String,
    pub items: Vec<String>,
}

/// Energy-aware scheduling notes
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnergyNotes {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub morning: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub afternoon: Option<String>,
}

// =============================================================================
// Extended Email Types
// =============================================================================

/// Extended email with conversation context
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmailDetail {
    pub id: String,
    pub sender: String,
    pub sender_email: String,
    pub subject: String,
    pub received: Option<String>,
    pub priority: EmailPriority,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_arc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommended_action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_priority: Option<String>,
}

/// Email summary from 83-email-summary.md
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmailSummaryData {
    pub high_priority: Vec<EmailDetail>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub medium_priority: Option<Vec<EmailDetail>>,
    pub stats: EmailStats,
}

/// Email statistics
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmailStats {
    pub high_count: usize,
    pub medium_count: usize,
    pub low_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub needs_action: Option<usize>,
}

// =============================================================================
// Full Meeting Prep (from individual prep files)
// =============================================================================

/// Complete meeting prep from individual prep file
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FullMeetingPrep {
    pub file_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calendar_event_id: Option<String>,
    pub title: String,
    pub time_range: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meeting_context: Option<String>,
    /// Quick Context metrics (key-value pairs like Ring, ARR, Health)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quick_context: Option<Vec<(String, String)>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attendees: Option<Vec<Stakeholder>>,
    /// Since Last Meeting section items
    #[serde(skip_serializing_if = "Option::is_none")]
    pub since_last: Option<Vec<String>>,
    /// Current Strategic Programs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategic_programs: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_state: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_items: Option<Vec<ActionWithContext>>,
    /// Current Risks to Monitor
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risks: Option<Vec<String>>,
    /// Suggested Talking Points
    #[serde(skip_serializing_if = "Option::is_none")]
    pub talking_points: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub questions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_principles: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub references: Option<Vec<SourceReference>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_markdown: Option<String>,
}

/// Action item with context (for prep files)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionWithContext {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    pub is_overdue: bool,
}

// =============================================================================
// Google Configuration & Calendar Types (Phase 3.0 / 3A)
// =============================================================================

/// Google integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_path: Option<String>,
    #[serde(default = "default_poll_interval")]
    pub calendar_poll_interval_minutes: u32,
    #[serde(default = "default_work_hours_start")]
    pub work_hours_start: u8,
    #[serde(default = "default_work_hours_end")]
    pub work_hours_end: u8,
}

fn default_poll_interval() -> u32 {
    5
}
fn default_work_hours_start() -> u8 {
    8
}
fn default_work_hours_end() -> u8 {
    18
}

impl Default for GoogleConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            token_path: None,
            calendar_poll_interval_minutes: default_poll_interval(),
            work_hours_start: default_work_hours_start(),
            work_hours_end: default_work_hours_end(),
        }
    }
}

/// Google authentication status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum GoogleAuthStatus {
    NotConfigured,
    Authenticated { email: String },
    TokenExpired,
}

impl Default for GoogleAuthStatus {
    fn default() -> Self {
        GoogleAuthStatus::NotConfigured
    }
}

/// A calendar event from Google Calendar
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarEvent {
    pub id: String,
    pub title: String,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    #[serde(rename = "type")]
    pub meeting_type: MeetingType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,
    #[serde(default)]
    pub attendees: Vec<String>,
    #[serde(default)]
    pub is_all_day: bool,
}

// =============================================================================
// Post-Meeting Capture Types (Phase 3B)
// =============================================================================

/// Post-meeting capture configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostMeetingCaptureConfig {
    #[serde(default = "default_capture_enabled")]
    pub enabled: bool,
    #[serde(default = "default_delay_minutes")]
    pub delay_minutes: u32,
    #[serde(default = "default_auto_dismiss_secs")]
    pub auto_dismiss_secs: u32,
    /// How long to wait for a transcript to appear before showing a fallback prompt
    #[serde(default = "default_transcript_wait_minutes")]
    pub transcript_wait_minutes: u32,
}

fn default_capture_enabled() -> bool {
    true
}
fn default_delay_minutes() -> u32 {
    5
}
fn default_auto_dismiss_secs() -> u32 {
    60
}
fn default_transcript_wait_minutes() -> u32 {
    10
}

impl Default for PostMeetingCaptureConfig {
    fn default() -> Self {
        Self {
            enabled: default_capture_enabled(),
            delay_minutes: default_delay_minutes(),
            auto_dismiss_secs: default_auto_dismiss_secs(),
            transcript_wait_minutes: default_transcript_wait_minutes(),
        }
    }
}

/// Captured outcome from a post-meeting prompt
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapturedOutcome {
    pub meeting_id: String,
    pub meeting_title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,
    pub captured_at: DateTime<Utc>,
    #[serde(default)]
    pub wins: Vec<String>,
    #[serde(default)]
    pub risks: Vec<String>,
    #[serde(default)]
    pub actions: Vec<CapturedAction>,
}

/// A single captured action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapturedAction {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date: Option<String>,
}

// =============================================================================
// Weekly Planning Types (Phase 3C)
// =============================================================================

/// State of the weekly planning wizard
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WeekPlanningState {
    NotReady,
    DataReady,
    InProgress,
    Completed,
    DefaultsApplied,
}

impl Default for WeekPlanningState {
    fn default() -> Self {
        WeekPlanningState::NotReady
    }
}

/// A focus block suggestion from the weekly planner
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusBlock {
    pub day: String,
    pub start: String,
    pub end: String,
    pub duration_minutes: u32,
    pub suggested_activity: String,
    #[serde(default)]
    pub selected: bool,
}
