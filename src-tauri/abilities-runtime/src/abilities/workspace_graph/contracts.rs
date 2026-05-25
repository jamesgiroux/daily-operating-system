//! Wire contracts for the `workspace_graph` read ability.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::abilities::get_entity_intelligence::contracts::Cursor;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceGraphInput {
    pub schema_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity_filter: Option<WorkspaceGraphEntityFilter>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category_filter: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<Cursor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub if_none_match: Option<String>,
    #[serde(default)]
    pub include_entity_names: bool,
    pub page_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceGraphEntityFilter {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity_types: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceGraphPrivacyProfile {
    FirstParty,
    SurfaceClient,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceGraphReadRequest {
    pub input: WorkspaceGraphInput,
    pub privacy_profile: WorkspaceGraphPrivacyProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WorkspaceGraphResponse {
    Projection(WorkspaceGraphProjection),
    NotModified(WorkspaceGraphNotModified),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceGraphNotModified {
    pub schema_version: u32,
    pub graph_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceGraphProjection {
    pub schema_version: u32,
    pub graph_version: String,
    pub page: WorkspaceGraphPage,
    pub projection: WorkspaceGraphProjectionBody,
    pub audit: WorkspaceGraphAudit,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceGraphPage {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<Cursor>,
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceGraphProjectionBody {
    pub entities: Vec<WorkspaceGraphEntity>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceGraphEntity {
    pub entity_type: String,
    pub entity_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity_name: Option<String>,
    pub file_links: Vec<WorkspaceGraphFileLink>,
    pub claim_summary: WorkspaceGraphClaimSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceGraphFileLink {
    pub link_handle: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_handle: Option<String>,
    pub data_source_kind: String,
    pub workspace_file_kind: String,
    pub lifecycle_state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    pub source_asof: String,
    pub attribution_source: String,
    pub confidence: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_override: Option<WorkspaceGraphUserOverride>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceGraphUserOverride {
    pub present: bool,
    pub at: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceGraphClaimSummary {
    pub total: u32,
    pub by_trust_band: WorkspaceGraphTrustBandSummary,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceGraphTrustBandSummary {
    pub likely_current: u32,
    pub use_with_caution: u32,
    pub needs_verification: u32,
    pub unscored: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceGraphAudit {
    pub schema_version: u32,
    pub graph_version: String,
    pub gap_counts: BTreeMap<String, u32>,
    pub gaps: Vec<WorkspaceGraphAuditGap>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceGraphAuditGap {
    pub category: String,
    pub gap_id: String,
    pub workspace_source_handle: String,
    #[serde(default)]
    pub source_handle: Option<String>,
    #[serde(default)]
    pub link_handle: Option<String>,
    #[serde(default)]
    pub claim_handle: Option<String>,
    #[serde(default)]
    pub entity_type: Option<String>,
    #[serde(default)]
    pub entity_id: Option<String>,
    #[serde(default)]
    pub linked_entity_type: Option<String>,
    #[serde(default)]
    pub linked_entity_id: Option<String>,
    #[serde(default)]
    pub claim_entity_type: Option<String>,
    #[serde(default)]
    pub claim_entity_id: Option<String>,
    #[serde(default)]
    pub workspace_file_kind: Option<String>,
    #[serde(default)]
    pub lifecycle_state: Option<String>,
    pub reason: String,
}
