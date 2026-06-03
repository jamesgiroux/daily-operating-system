//! Wire contracts for readable claim-file projection abilities.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const RENDER_ENTITY_CLAIM_FILE_ABILITY_NAME: &str = "render_entity_claim_file";
pub const APPLY_CLAIM_FILE_CORRECTIONS_ABILITY_NAME: &str = "apply_claim_file_corrections";
pub const CLAIM_FILES_SCHEMA_VERSION: u32 = 1;
pub const CLAIM_FILE_RENDER_SCOPE: &str = "write.claim_files";
pub const CLAIM_FILE_APPLY_SCOPE: &str = "submit.claim_file_corrections";

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenderEntityClaimFileInput {
    pub schema_version: u32,
    pub subject_ref: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplyClaimFileCorrectionsInput {
    pub schema_version: u32,
    pub markdown_rel_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClaimFileRenderRequest {
    pub input: RenderEntityClaimFileInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClaimFileApplyRequest {
    pub input: ApplyClaimFileCorrectionsInput,
    pub actor_principal_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClaimFileProjectionResult {
    pub run_id: String,
    pub markdown_rel_path: String,
    pub sidecar_rel_path: String,
    pub claim_count: usize,
    pub markdown_checksum: String,
    pub sidecar_checksum: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClaimFileApplyResult {
    pub applied_count: usize,
    pub skipped_count: usize,
    pub failures: Vec<ClaimFileApplyFailure>,
    pub responses: Vec<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rerender: Option<ClaimFileProjectionResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClaimFileApplyFailure {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claim_id: Option<String>,
    pub error_class: String,
    pub error_detail_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ClaimFileOperationError {
    #[error("{0}")]
    InvalidRequest(String),
    #[error("{0}")]
    MutationBlocked(String),
    #[error("{0}")]
    OperationFailed(String),
}
