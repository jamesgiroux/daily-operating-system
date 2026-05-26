//! Input/output contracts for `portfolio_attention`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::abilities::get_entity_intelligence::contracts::{Cursor, CursorState};
use crate::abilities::provenance::SchemaVersion;
use crate::abilities::trust::types::TrustBand;

pub const PORTFOLIO_ATTENTION_ABILITY_NAME: &str = "portfolio_attention";
pub const PORTFOLIO_ATTENTION_SCHEMA_VERSION: u32 = 1;
pub const PORTFOLIO_ATTENTION_SCOPE: &str = "read.portfolio_attention";

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PortfolioAttentionInput {
    pub schema_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_size: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<Cursor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity_types: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioAttentionResult {
    pub schema_version: SchemaVersion,
    pub generated_at: String,
    pub items: Vec<PortfolioAttentionItem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<Cursor>,
    pub total_hint: u64,
    pub cursor_state: CursorState,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioAttentionItem {
    pub rank: u32,
    pub subject: PortfolioAttentionSubject,
    pub score: AttentionScore,
    pub trust_band: TrustBand,
    pub reasons: Vec<AttentionReason>,
    pub evidence: Vec<AttentionEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioAttentionSubject {
    pub entity_type: String,
    pub entity_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AttentionScore {
    pub total: f64,
    pub risk: f64,
    pub open_loops: f64,
    pub freshness: f64,
    pub trust: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub salience: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AttentionReason {
    pub kind: String,
    pub summary: String,
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AttentionEvidence {
    pub text: String,
    pub claim_type: String,
    pub source_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_asof: Option<String>,
    pub observed_at: String,
    pub trust_band: TrustBand,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub salience: Option<f64>,
}
