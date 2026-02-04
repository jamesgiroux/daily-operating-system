use serde::{Deserialize, Serialize};

/// Configuration stored in ~/.daybreak/config.json
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub workspace_path: String,
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

/// Complete dashboard data payload
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardData {
    pub overview: DayOverview,
    pub stats: DayStats,
    pub meetings: Vec<Meeting>,
    pub actions: Vec<Action>,
}
