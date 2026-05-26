//! Recommendation ability contracts.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::abilities::registry::ActorKind;
use crate::abilities::trust::types::TrustBand;

pub const SCORE_SALIENCE_ABILITY_NAME: &str = "score_salience";
pub const SCORE_SALIENCE_SCHEMA_VERSION: u32 = 1;
pub const SCORE_SALIENCE_SCOPE: &str = "read.recommendations";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct ClaimId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScoreSalienceInput {
    pub schema_version: u32,
    pub claim_id: ClaimId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScoreSalienceReadRequest {
    pub schema_version: u32,
    pub claim_id: ClaimId,
    pub actor: ActorKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScoreSalienceResponse {
    pub schema_version: u32,
    pub claim_id: ClaimId,
    pub computed_at: String,
    pub persistence: SaliencePersistence,
    pub salience: SalienceScore,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SaliencePersistence {
    Preview,
    Stored { evaluation_id: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SalienceScore {
    pub total: f64,
    pub factors: Vec<SalienceFactor>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SalienceFactor {
    pub kind: SalienceFactorKind,
    pub value: Option<f64>,
    pub weight: f64,
    pub rationale: FactorRationale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum SalienceFactorKind {
    Importance,
    Novelty,
    Urgency,
    Timing,
    UserFit,
    Freshness,
    Trust,
    Corroboration,
    Contradiction,
    OpenLoopRelevance,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum FactorRationale {
    Importance {
        trust_band: TrustBand,
        source_authority: f64,
    },
    Novelty {
        vector_distance: f64,
        neighbor_count: u32,
    },
    Urgency {
        deadline: Option<String>,
        decay_factor: f64,
    },
    Timing {
        signal_age_secs: i64,
        calendar_proximity_secs: Option<i64>,
    },
    UserFit {
        feedback_history_score: f64,
    },
    Freshness {
        decay_factor: f64,
    },
    Trust {
        trust_band: TrustBand,
    },
    Corroboration {
        corroboration_count: u32,
    },
    Contradiction {
        contradiction_count: u32,
    },
    OpenLoopRelevance {
        open_loop_count: u32,
        has_action: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SalienceReadError {
    #[error("unsupported schema_version `{0}` for score_salience")]
    UnsupportedSchemaVersion(u32),
    #[error("claim `{0}` not found")]
    ClaimNotFound(String),
    #[error("claim `{0}` is not visible to the current actor")]
    ClaimNotVisible(String),
    #[error("{0}")]
    ReadFailed(String),
}
