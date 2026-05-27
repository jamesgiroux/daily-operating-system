//! Wire contracts for the `markdown_preview` read ability.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MarkdownPreviewInput {
    pub schema_version: u32,
    pub source_handle: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MarkdownPreviewOutput {
    pub schema_version: u32,
    pub preview_html: String,
    pub source_asof: String,
    pub lifecycle_state: String,
    pub trust_band_summary: String,
    pub source_label: String,
    pub blocked_asset_count: u32,
    pub asset_resolver_available: bool,
    pub sanitizer_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MarkdownPreviewReadRequest {
    pub input: MarkdownPreviewInput,
}
