use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::WorkflowError;

/// Configuration stored in ~/.daybreak/config.json
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub workspace_path: String,
    #[serde(default)]
    pub schedules: Schedules,
}

/// Schedule configuration for workflows
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Schedules {
    #[serde(default = "ScheduleEntry::default_today")]
    pub today: ScheduleEntry,
    #[serde(default = "ScheduleEntry::default_archive")]
    pub archive: ScheduleEntry,
}

impl Default for Schedules {
    fn default() -> Self {
        Self {
            today: ScheduleEntry::default_today(),
            archive: ScheduleEntry::default_archive(),
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
}

impl std::fmt::Display for WorkflowId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkflowId::Today => write!(f, "today"),
            WorkflowId::Archive => write!(f, "archive"),
        }
    }
}

impl std::str::FromStr for WorkflowId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "today" => Ok(WorkflowId::Today),
            "archive" => Ok(WorkflowId::Archive),
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
#[serde(rename_all = "lowercase")]
pub enum MeetingType {
    Customer,
    Internal,
    Personal,
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
}

/// A single meeting
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Meeting {
    pub id: String,
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

/// Email priority level
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EmailPriority {
    High,
    Normal,
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
