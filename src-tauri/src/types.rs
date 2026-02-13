use std::collections::HashMap;

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
    #[serde(default = "default_entity_mode")]
    pub entity_mode: String,
    #[serde(default)]
    pub google: GoogleConfig,
    #[serde(default)]
    pub post_meeting_capture: PostMeetingCaptureConfig,
    #[serde(default)]
    pub features: HashMap<String, bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_domain: Option<String>,
    /// Multiple user domains for multi-org classification (I171).
    /// Takes precedence over `user_domain` when set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_domains: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_company: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_focus: Option<String>,
    /// One-time gate: internal team setup completion state (Sprint 20 / ADR-0070).
    #[serde(default)]
    pub internal_team_setup_completed: bool,
    /// Internal setup schema/version marker for future re-prompts.
    #[serde(default)]
    pub internal_team_setup_version: u32,
    /// Root internal org account ID (set after setup).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub internal_org_account_id: Option<String>,
    /// Show developer tools panel (wrench icon). Only effective in debug builds.
    #[serde(default)]
    pub developer_mode: bool,
    /// AI model configuration for tiered operations (I174).
    #[serde(default)]
    pub ai_models: AiModelConfig,
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
    "customer-success".to_string()
}

// =============================================================================
// AI Model Configuration (I174)
// =============================================================================

/// AI model configuration for tiered operations.
///
/// Synthesis: intelligence, briefing, week narrative (needs reasoning).
/// Extraction: emails, preps (structured extraction from context).
/// Mechanical: inbox classification, file summaries (simple tasks).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiModelConfig {
    #[serde(default = "default_synthesis_model")]
    pub synthesis: String,
    #[serde(default = "default_extraction_model")]
    pub extraction: String,
    #[serde(default = "default_mechanical_model")]
    pub mechanical: String,
}

impl Default for AiModelConfig {
    fn default() -> Self {
        Self {
            synthesis: default_synthesis_model(),
            extraction: default_extraction_model(),
            mechanical: default_mechanical_model(),
        }
    }
}

fn default_synthesis_model() -> String {
    "sonnet".to_string()
}

fn default_extraction_model() -> String {
    "sonnet".to_string()
}

fn default_mechanical_model() -> String {
    "haiku".to_string()
}

fn default_entity_mode() -> String {
    "account".to_string()
}

impl Config {
    /// Resolve the list of user domains for internal/external classification.
    ///
    /// Merges `user_domains` (preferred) with legacy `user_domain` field.
    /// Returns an empty vec if neither is set.
    pub fn resolved_user_domains(&self) -> Vec<String> {
        if let Some(ref domains) = self.user_domains {
            if !domains.is_empty() {
                return domains.clone();
            }
        }
        // Fallback to legacy single domain
        match &self.user_domain {
            Some(d) if !d.is_empty() => vec![d.clone()],
            _ => Vec::new(),
        }
    }
}

/// Entity mode type for validation
pub fn validate_entity_mode(mode: &str) -> Result<(), String> {
    match mode {
        "account" | "project" | "both" => Ok(()),
        _ => Err(format!(
            "Invalid entity mode: '{}'. Must be 'account', 'project', or 'both'.",
            mode
        )),
    }
}

/// Derive profile from entity mode for backend compat
pub fn profile_for_entity_mode(mode: &str) -> String {
    match mode {
        "project" => "general".to_string(),
        _ => "customer-success".to_string(), // account + both → customer-success
    }
}

/// Default feature flags for a given profile.
///
/// All features default ON for the profile that supports them.
/// CS-only features default OFF for non-CS profiles.
pub fn default_features(profile: &str) -> HashMap<String, bool> {
    default_features_for_mode(profile, "account")
}

/// Entity-mode-aware feature defaults (I53).
///
/// - `accountTracking`: ON for account or both modes (CS profile)
/// - `projectTracking`: ON for project or both modes
/// - `impactRollup`: CS-only (same as accountTracking)
pub fn default_features_for_mode(profile: &str, entity_mode: &str) -> HashMap<String, bool> {
    let mut features = HashMap::new();
    // Universal features (all profiles)
    features.insert("emailTriage".to_string(), true);
    features.insert("postMeetingCapture".to_string(), true);
    features.insert("meetingPrep".to_string(), true);
    features.insert("weeklyPlanning".to_string(), true);
    features.insert("inboxProcessing".to_string(), true);
    // Entity-mode-aware features
    let is_cs = profile == "customer-success";
    let accounts_on = entity_mode == "account" || entity_mode == "both";
    let projects_on = entity_mode == "project" || entity_mode == "both";
    features.insert("accountTracking".to_string(), is_cs && accounts_on);
    features.insert("projectTracking".to_string(), projects_on);
    features.insert("impactRollup".to_string(), is_cs && accounts_on);
    features
}

/// Check if a feature is enabled, falling through to profile defaults.
///
/// Priority: explicit config value > profile default > true (safe fallback).
pub fn is_feature_enabled(config: &Config, feature: &str) -> bool {
    // Explicit override in config.features takes priority
    if let Some(&enabled) = config.features.get(feature) {
        return enabled;
    }
    // Fall through to entity-mode-aware defaults
    let defaults = default_features_for_mode(&config.profile, &config.entity_mode);
    defaults.get(feature).copied().unwrap_or(true)
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
#[derive(Default)]
pub enum WorkflowStatus {
    #[default]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_phase: Option<WorkflowPhase>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub can_retry: Option<bool>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processing_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processing_error: Option<String>,
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

/// Calendar overlay status (ADR-0032: hybrid overlay)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OverlayStatus {
    Enriched,     // In both: live timing + briefing enrichment
    Cancelled,    // In briefing only: meeting removed from calendar
    New,          // In live only: no prep available
    BriefingOnly, // No live data (Google not connected)
}

/// Meeting type classification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

impl MeetingType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MeetingType::Customer => "customer",
            MeetingType::Qbr => "qbr",
            MeetingType::Training => "training",
            MeetingType::Internal => "internal",
            MeetingType::TeamSync => "team_sync",
            MeetingType::OneOnOne => "one_on_one",
            MeetingType::Partnership => "partnership",
            MeetingType::AllHands => "all_hands",
            MeetingType::External => "external",
            MeetingType::Personal => "personal",
        }
    }
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
    /// ISO 8601 start timestamp for reliable date parsing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_iso: Option<String>,
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
    /// Calendar overlay status (ADR-0032)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overlay_status: Option<OverlayStatus>,
    /// Whether the user has reviewed this prep (ADR-0033)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prep_reviewed: Option<bool>,
    /// SQLite entity ID for the linked account (populated from junction table)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    /// Entities linked via M2M junction table (I52)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linked_entities: Option<Vec<LinkedEntity>>,
    /// Suggestion to unarchive an account that matched this meeting's domain (I161).
    /// Set when classification matched an archived account. Frontend shows a banner.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_unarchive_account_id: Option<String>,
}

/// An entity linked to a meeting via the junction table.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkedEntity {
    pub id: String,
    pub name: String,
    pub entity_type: String,
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

/// Health status for email sync operations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EmailSyncState {
    Ok,
    Warning,
    Error,
}

/// Pipeline stage associated with an email sync status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EmailSyncStage {
    Fetch,
    Deliver,
    Enrich,
    Refresh,
}

/// Structured status payload for email sync and enrichment health.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmailSyncStatus {
    pub state: EmailSyncState,
    pub stage: EmailSyncStage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub using_last_known_good: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub can_retry: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_attempt_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_success_at: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_sync: Option<EmailSyncStatus>,
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
    /// AI-generated narrative overview of the week (I94 — null until AI enrichment)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub week_narrative: Option<String>,
    /// AI-identified top priority (I94 — null until AI enrichment)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_priority: Option<TopPriority>,
    /// Proactive readiness checks surfacing prep gaps (I93)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readiness_checks: Option<Vec<ReadinessCheck>>,
    /// Per-day density and meeting shape (I93)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub day_shapes: Option<Vec<DayShape>>,
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
    /// Actual overdue action items (I93)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overdue: Option<Vec<WeekAction>>,
    /// Actual due-this-week action items (I93)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_this_week_items: Option<Vec<WeekAction>>,
}

/// A single action item for week view (I93)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeekAction {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date: Option<String>,
    pub priority: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days_overdue: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// Proactive readiness check for the week (I93)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadinessCheck {
    pub check_type: String,
    pub message: String,
    pub severity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meeting_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
}

/// Per-day density shape for the week view (I93)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DayShape {
    pub day_name: String,
    pub date: String,
    pub meeting_count: usize,
    pub meeting_minutes: u32,
    pub density: String,
    pub meetings: Vec<WeekMeeting>,
    pub available_blocks: Vec<TimeBlock>,
}

/// Account hygiene alert
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HygieneAlert {
    pub account: String,
    pub lifecycle: Option<String>,
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

/// AI-identified top priority for the week (I94)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopPriority {
    pub title: String,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meeting_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_id: Option<String>,
}

// =============================================================================
// Focus Data Types
// =============================================================================

/// Focus page data — assembled from schedule.json + SQLite actions + gap analysis
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focus_statement: Option<String>,
    pub priorities: Vec<crate::db::DbAction>,
    pub key_meetings: Vec<FocusMeeting>,
    pub available_blocks: Vec<TimeBlock>,
    pub total_focus_minutes: u32,
    pub availability: FocusAvailability,
    pub prioritized_actions: Vec<PrioritizedFocusAction>,
    pub top_three: Vec<String>,
    pub implications: FocusImplications,
}

/// Lightweight meeting projection for the Focus page
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusMeeting {
    pub id: String,
    pub title: String,
    pub time: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
    pub meeting_type: String,
    pub has_prep: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prep_file: Option<String>,
}

/// Detailed availability diagnostics and capacity metrics for Focus (I178).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusAvailability {
    pub source: String,
    pub warnings: Vec<String>,
    pub meeting_count: u32,
    pub meeting_minutes: u32,
    pub available_minutes: u32,
    pub deep_work_minutes: u32,
    pub deep_work_blocks: Vec<TimeBlock>,
}

/// Ranked action with deterministic feasibility/risk metadata (I179).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrioritizedFocusAction {
    pub action: crate::db::DbAction,
    pub score: i32,
    pub effort_minutes: u32,
    pub feasible: bool,
    pub at_risk: bool,
    pub reason: String,
}

/// High-level implications for today's focus capacity vs action load (I179).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusImplications {
    pub achievable_count: u32,
    pub total_count: u32,
    pub at_risk_count: u32,
    pub summary: String,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FullMeetingPrep {
    pub file_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calendar_event_id: Option<String>,
    pub title: String,
    pub time_range: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meeting_context: Option<String>,
    /// Calendar event description from Google Calendar (I185)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calendar_notes: Option<String>,
    /// Intelligence-enriched account snapshot (I186)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_snapshot: Option<Vec<AccountSnapshotItem>>,
    /// Quick Context metrics (key-value pairs like Ring, ARR, Health) — legacy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quick_context: Option<Vec<(String, String)>>,
    /// User-authored agenda items (I194 / ADR-0065)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agenda: Option<Vec<String>>,
    /// User-authored notes (I194 / ADR-0065)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_notes: Option<String>,
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
    /// Canonical recent wins for meeting prep (I196).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recent_wins: Option<Vec<String>>,
    /// Structured provenance for recent wins (I196).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recent_win_sources: Option<Vec<SourceReference>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub questions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_principles: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub references: Option<Vec<SourceReference>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_markdown: Option<String>,
    /// Stakeholder relationship signals computed from meeting history (I43)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stakeholder_signals: Option<crate::db::StakeholderSignals>,
    /// Per-attendee context enriched from people DB (I51)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attendee_context: Option<Vec<AttendeeContext>>,
    /// Proposed agenda synthesized from prep data (I80)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proposed_agenda: Option<Vec<AgendaItem>>,
    /// Intelligence summary — executive assessment from intelligence.json (I135)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intelligence_summary: Option<String>,
    /// Entity-level risks from intelligence.json (I135)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_risks: Option<Vec<crate::entity_intel::IntelRisk>>,
    /// Entity meeting readiness items from intelligence.json (I135)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_readiness: Option<Vec<String>>,
    /// Stakeholder insights from intelligence.json (I135)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stakeholder_insights: Option<Vec<crate::entity_intel::StakeholderInsight>>,
}

/// Unified meeting detail payload (ADR-0066).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingIntelligence {
    pub meeting: crate::db::DbMeeting,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prep: Option<FullMeetingPrep>,
    pub is_past: bool,
    pub is_current: bool,
    pub is_frozen: bool,
    pub can_edit_user_layer: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agenda: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcomes: Option<MeetingOutcomeData>,
    #[serde(default)]
    pub captures: Vec<crate::db::DbCapture>,
    #[serde(default)]
    pub actions: Vec<crate::db::DbAction>,
    #[serde(default)]
    pub linked_entities: Vec<LinkedEntity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prep_snapshot_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prep_frozen_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transcript_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transcript_processed_at: Option<String>,
}

/// Attendee context for meeting prep enrichment (I51).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttendeeContext {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relationship: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meeting_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_seen: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub person_id: Option<String>,
}

/// Proposed agenda item for meeting prep (I80)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgendaItem {
    pub topic: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// Account snapshot item for intelligence-enriched Quick Context (I186)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountSnapshotItem {
    pub label: String,
    pub value: String,
    #[serde(rename = "type")]
    pub item_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub urgency: Option<String>,
}

/// Action item with context (for prep files)
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Default)]
pub enum GoogleAuthStatus {
    #[default]
    NotConfigured,
    Authenticated {
        email: String,
    },
    TokenExpired,
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
// Transcript Processing Types (I44 / ADR-0044)
// =============================================================================

/// Result of transcript processing
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptResult {
    pub status: String, // "success" | "error"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<String>,
    #[serde(default)]
    pub wins: Vec<String>,
    #[serde(default)]
    pub risks: Vec<String>,
    #[serde(default)]
    pub decisions: Vec<String>,
    #[serde(default)]
    pub actions: Vec<CapturedAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Outcomes for a meeting (query response)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingOutcomeData {
    pub meeting_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default)]
    pub wins: Vec<String>,
    #[serde(default)]
    pub risks: Vec<String>,
    #[serde(default)]
    pub decisions: Vec<String>,
    #[serde(default)]
    pub actions: Vec<crate::db::DbAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transcript_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processed_at: Option<String>,
}

/// Persisted record of a processed transcript (for immutability tracking)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptRecord {
    pub meeting_id: String,
    pub file_path: String,
    pub destination: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    pub processed_at: String,
}

// =============================================================================
// User Context for AI Enrichment (I58)
// =============================================================================

/// User context injected into AI enrichment prompts for personalization.
///
/// Built from config fields (user_name, user_company, user_title, user_focus).
/// Returns an empty prompt fragment when no fields are set.
#[derive(Debug, Clone, Default)]
pub struct UserContext {
    pub name: Option<String>,
    pub company: Option<String>,
    pub title: Option<String>,
    pub focus: Option<String>,
}

impl UserContext {
    /// Build a UserContext from the app config.
    pub fn from_config(config: &Config) -> Self {
        Self {
            name: config.user_name.clone(),
            company: config.user_company.clone(),
            title: config.user_title.clone(),
            focus: config.user_focus.clone(),
        }
    }

    /// Generate a prompt fragment describing the user. Returns "" if no fields set.
    pub fn prompt_fragment(&self) -> String {
        let mut parts = Vec::new();

        if let Some(ref name) = self.name {
            parts.push(format!("The user is {}", name));
        }

        if let Some(ref title) = self.title {
            if let Some(ref company) = self.company {
                parts.push(format!("{} at {}", title, company));
            } else {
                parts.push(title.clone());
            }
        } else if let Some(ref company) = self.company {
            parts.push(format!("working at {}", company));
        }

        if parts.is_empty() {
            return String::new();
        }

        let mut result = parts.join(", ");
        result.push('.');

        if let Some(ref focus) = self.focus {
            result.push_str(&format!(" Current focus: {}.", focus));
        }

        result.push('\n');
        result
    }

    /// Get the user's title or a fallback label.
    pub fn title_or_default(&self) -> &str {
        self.title.as_deref().unwrap_or("a professional")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(profile: &str) -> Config {
        Config {
            workspace_path: "/tmp/test".to_string(),
            schedules: Schedules::default(),
            profile: profile.to_string(),
            profile_config: None,
            entity_mode: "account".to_string(),
            google: GoogleConfig::default(),
            post_meeting_capture: PostMeetingCaptureConfig::default(),
            features: HashMap::new(),
            user_domain: None,
            user_domains: None,
            user_name: None,
            user_company: None,
            user_title: None,
            user_focus: None,
            internal_team_setup_completed: false,
            internal_team_setup_version: 0,
            internal_org_account_id: None,
            developer_mode: false,
            ai_models: AiModelConfig::default(),
        }
    }

    #[test]
    fn test_default_features_cs_profile() {
        let defaults = default_features("customer-success");
        assert_eq!(defaults.get("emailTriage"), Some(&true));
        assert_eq!(defaults.get("accountTracking"), Some(&true));
        assert_eq!(defaults.get("impactRollup"), Some(&true));
    }

    #[test]
    fn test_default_features_general_profile() {
        let defaults = default_features("general");
        assert_eq!(defaults.get("emailTriage"), Some(&true));
        assert_eq!(defaults.get("accountTracking"), Some(&false));
        assert_eq!(defaults.get("impactRollup"), Some(&false));
    }

    #[test]
    fn test_is_feature_enabled_defaults() {
        let config = test_config("customer-success");
        // Empty features HashMap → falls through to defaults
        assert!(is_feature_enabled(&config, "emailTriage"));
        assert!(is_feature_enabled(&config, "impactRollup"));
    }

    #[test]
    fn test_is_feature_enabled_explicit_override() {
        let mut config = test_config("customer-success");
        config.features.insert("emailTriage".to_string(), false);
        assert!(!is_feature_enabled(&config, "emailTriage"));
        // Other features still use defaults
        assert!(is_feature_enabled(&config, "impactRollup"));
    }

    #[test]
    fn test_is_feature_enabled_general_profile_cs_features() {
        let config = test_config("general");
        // CS-only features default off for general profile
        assert!(!is_feature_enabled(&config, "accountTracking"));
        assert!(!is_feature_enabled(&config, "impactRollup"));
        // Universal features still on
        assert!(is_feature_enabled(&config, "emailTriage"));
    }

    #[test]
    fn test_is_feature_enabled_unknown_feature() {
        let config = test_config("customer-success");
        // Unknown features fall through to true (safe fallback)
        assert!(is_feature_enabled(&config, "unknownFeature"));
    }

    #[test]
    fn test_config_deserializes_without_features() {
        // Backwards compat: config.json without a "features" key
        let json = r#"{
            "workspacePath": "/tmp/test",
            "profile": "customer-success"
        }"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert!(config.features.is_empty());
        assert!(is_feature_enabled(&config, "emailTriage"));
    }

    // =========================================================================
    // UserContext tests (I58)
    // =========================================================================

    #[test]
    fn test_user_context_full() {
        let ctx = UserContext {
            name: Some("Jamie".to_string()),
            company: Some("TestCo".to_string()),
            title: Some("CSM".to_string()),
            focus: Some("renewals".to_string()),
        };
        let frag = ctx.prompt_fragment();
        assert!(frag.contains("Jamie"));
        assert!(frag.contains("CSM at TestCo"));
        assert!(frag.contains("Current focus: renewals"));
    }

    #[test]
    fn test_user_context_empty() {
        let ctx = UserContext::default();
        assert_eq!(ctx.prompt_fragment(), "");
    }

    #[test]
    fn test_user_context_name_only() {
        let ctx = UserContext {
            name: Some("Alex".to_string()),
            ..Default::default()
        };
        let frag = ctx.prompt_fragment();
        assert!(frag.contains("Alex"));
        assert!(!frag.contains("at"));
    }

    #[test]
    fn test_user_context_from_config() {
        let mut config = test_config("customer-success");
        config.user_name = Some("Jamie".to_string());
        config.user_company = Some("TestCo".to_string());
        config.user_title = Some("CSM".to_string());
        config.user_focus = Some("renewals".to_string());
        let ctx = UserContext::from_config(&config);
        assert_eq!(ctx.name.as_deref(), Some("Jamie"));
        assert_eq!(ctx.title_or_default(), "CSM");
    }

    #[test]
    fn test_user_context_title_or_default() {
        let ctx = UserContext::default();
        assert_eq!(ctx.title_or_default(), "a professional");
    }

    // =========================================================================
    // I53: Entity-mode-aware feature defaults
    // =========================================================================

    #[test]
    fn test_project_tracking_defaults_by_entity_mode() {
        // account mode: projectTracking OFF
        let defaults = default_features_for_mode("customer-success", "account");
        assert_eq!(defaults.get("projectTracking"), Some(&false));
        assert_eq!(defaults.get("accountTracking"), Some(&true));

        // project mode: projectTracking ON, accountTracking OFF (general profile)
        let defaults = default_features_for_mode("general", "project");
        assert_eq!(defaults.get("projectTracking"), Some(&true));
        assert_eq!(defaults.get("accountTracking"), Some(&false));

        // both mode: both ON
        let defaults = default_features_for_mode("customer-success", "both");
        assert_eq!(defaults.get("projectTracking"), Some(&true));
        assert_eq!(defaults.get("accountTracking"), Some(&true));
        assert_eq!(defaults.get("impactRollup"), Some(&true));
    }

    #[test]
    fn test_is_feature_enabled_project_mode() {
        let mut config = test_config("general");
        config.entity_mode = "project".to_string();

        assert!(is_feature_enabled(&config, "projectTracking"));
        assert!(!is_feature_enabled(&config, "accountTracking"));
        // Universal still on
        assert!(is_feature_enabled(&config, "emailTriage"));
    }

    #[test]
    fn test_is_feature_enabled_both_mode() {
        let mut config = test_config("customer-success");
        config.entity_mode = "both".to_string();

        assert!(is_feature_enabled(&config, "projectTracking"));
        assert!(is_feature_enabled(&config, "accountTracking"));
        assert!(is_feature_enabled(&config, "impactRollup"));
    }
}
