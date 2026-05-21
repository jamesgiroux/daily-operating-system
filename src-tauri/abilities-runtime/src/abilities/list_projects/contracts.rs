//! Input/output contracts for `list_projects`. See
//! `list_accounts::contracts` for shape rationale.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::abilities::get_entity_intelligence::contracts::Cursor;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectListInput {
    pub schema_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<ProjectListFilter>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<Cursor>,
    pub page_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectListFilter {
    /// Restrict to projects whose `status` exactly equals this value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Restrict to projects whose `trajectory` exactly equals this value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trajectory: Option<ProjectTrajectory>,
    /// Restrict to projects rolling up to this canonical account.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_account_id: Option<String>,
    /// Restrict to projects whose name contains this substring
    /// (case-insensitive). Used by the W2 list shell's filter input.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_contains: Option<String>,
}

/// Typed trajectory band — closed set so reviewers can audit the
/// vocabulary without grepping ad-hoc display strings.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectTrajectory {
    Improving,
    Steady,
    Degrading,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummary {
    pub project_id: String,
    pub name: String,
    /// Canonical account this project rolls up to, when one is known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_account_id: Option<String>,
    pub status: String,
    pub trajectory: ProjectTrajectory,
    /// RFC3339 timestamp of the most recent touchpoint reaching this
    /// project, or `None` if the substrate has no touchpoint-backed
    /// claim yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_touchpoint_at: Option<String>,
}
