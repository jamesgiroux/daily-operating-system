//! Wire contracts for the `source_management_ledger` read ability.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::abilities::get_entity_intelligence::contracts::Cursor;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceManagementLedgerInput {
    pub schema_version: u32,
    pub entity_type: String,
    pub entity_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<Cursor>,
    pub page_size: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceManagementLedgerPrivacyProfile {
    FirstParty,
    SurfaceClient,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceManagementLedgerReadRequest {
    pub input: SourceManagementLedgerInput,
    pub privacy_profile: SourceManagementLedgerPrivacyProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceManagementActionInput {
    pub schema_version: u32,
    pub entity_type: String,
    pub entity_id: String,
    pub source_key: String,
    pub action: SourceManagementActionKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceManagementActionKind {
    Reingest,
    Quarantine,
    Relink,
    Ignore,
    Scratchpad,
    Archive,
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceManagementActionRequest {
    pub input: SourceManagementActionInput,
    pub actor_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceManagementActionReceipt {
    pub schema_version: u32,
    pub action: SourceManagementActionKind,
    pub status: String,
    pub source_key: String,
    pub lifecycle_state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_run: Option<SourceManagementIngestionRun>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceManagementLedgerResponse {
    pub schema_version: u32,
    pub page: SourceManagementLedgerPage,
    pub action_policy: SourceManagementActionPolicy,
    pub sources: Vec<SourceManagementSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceManagementLedgerPage {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<Cursor>,
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceManagementActionPolicy {
    pub reingest_enabled: bool,
    pub quarantine_enabled: bool,
    pub relink_enabled: bool,
    pub ignore_enabled: bool,
    pub scratchpad_enabled: bool,
    pub archive_enabled: bool,
    pub delete_enabled: bool,
    pub disabled_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceManagementSource {
    pub source_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_handle: Option<String>,
    pub source_kind: String,
    pub lifecycle_state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    pub source_asof: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity: Option<SourceManagementEntity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_override: Option<SourceManagementUserOverride>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_run: Option<SourceManagementIngestionRun>,
    pub ingestion_runs: Vec<SourceManagementIngestionRun>,
    pub trust_band_summary: SourceManagementTrustBandSummary,
    pub preview_available: bool,
    pub actions: SourceManagementSourceActions,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceManagementEntity {
    pub entity_type: String,
    pub entity_id: String,
    pub attribution_source: String,
    pub confidence_bps: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_override_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceManagementUserOverride {
    pub present: bool,
    pub at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceManagementIngestionRun {
    pub status: String,
    pub mode: String,
    pub started_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    pub claim_count_produced: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceManagementTrustBandSummary {
    pub total: u32,
    pub likely_current: u32,
    pub use_with_caution: u32,
    pub needs_verification: u32,
    pub unscored: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceManagementSourceActions {
    pub can_reingest: bool,
    pub can_quarantine: bool,
    pub can_relink: bool,
    pub can_ignore: bool,
    pub can_scratchpad: bool,
    pub can_archive: bool,
    pub can_delete: bool,
    pub disabled_reason: String,
}
