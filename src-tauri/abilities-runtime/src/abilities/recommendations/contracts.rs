//! Recommendation ability contracts.

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};

use crate::abilities::claim_receipt::{ClaimReceiptSnapshot, ClaimReceiptSurfaceContext};
use crate::abilities::provenance::SubjectRef;
use crate::abilities::registry::ActorKind;
use crate::abilities::trust::types::TrustBand;

pub const SCORE_SALIENCE_ABILITY_NAME: &str = "score_salience";
pub const SCORE_SALIENCE_SCHEMA_VERSION: u32 = 1;
pub const SCORE_SALIENCE_SCOPE: &str = "read.recommendations";
pub const LIST_SUGGESTED_NEXT_STEPS_ABILITY_NAME: &str = "list_suggested_next_steps";
pub const LIST_SUGGESTED_NEXT_STEPS_SCHEMA_VERSION: u16 = 1;
pub const LIST_SUGGESTED_NEXT_STEPS_SCOPE: &str = SCORE_SALIENCE_SCOPE;

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListSuggestedNextStepsInput {
    pub schema_version: u16,
    pub subject: Option<SubjectRef>,
    pub surface: ClaimReceiptSurfaceContext,
    pub max_items: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListSuggestedNextStepsResponse {
    pub schema_version: u16,
    pub items: Vec<SuggestedNextStepItem>,
    #[schemars(with = "String")]
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SuggestedNextStepItem {
    pub claim_id: ClaimId,
    pub headline: String,
    pub why_this_now_surface_text: String,
    pub factor_band: PrimaryFactorBand,
    pub recommended_action: RecommendedActionView,
    pub trust_band: TrustBand,
    pub receipt: ClaimReceiptSnapshot,
    pub feedback_state: FeedbackState,
    pub conversion_state: ConversionState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum PrimaryFactorBand {
    TimeSensitive,
    NewInformation,
    OpenLoopRelated,
    TrustChange,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum RecommendedActionView {
    ScheduleMeeting {
        entity_label: String,
        when_window: String,
    },
    SendMessage {
        entity_label: String,
        channel: String,
    },
    ReviewClaim {
        claim_label: String,
    },
    UpdateRecord {
        entity_label: String,
        field_label: String,
    },
    InvestigateChange {
        entity_label: String,
        change_label: String,
    },
    Custom {
        action_label: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum FeedbackState {
    Pending,
    Decided(RecommendationFeedbackDecision),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum RecommendationFeedbackDecision {
    Accept {
        #[schemars(with = "String")]
        at: DateTime<Utc>,
    },
    Dismiss {
        #[schemars(with = "String")]
        at: DateTime<Utc>,
        reason: DismissReason,
    },
    NotUseful {
        #[schemars(with = "String")]
        at: DateTime<Utc>,
    },
    TooNoisy {
        #[schemars(with = "String")]
        at: DateTime<Utc>,
    },
    Convert {
        #[schemars(with = "String")]
        at: DateTime<Utc>,
        into: ConversionTarget,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum DismissReason {
    NotRelevant,
    AlreadyKnew,
    WrongSubject,
    Other(BoundedNote),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct BoundedNote(String);

impl BoundedNote {
    pub const MAX_CHARS: usize = 200;

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for BoundedNote {
    type Error = BoundedNoteError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let actual = value.chars().count();
        if actual > Self::MAX_CHARS {
            return Err(BoundedNoteError::TooLong {
                max: Self::MAX_CHARS,
                actual,
            });
        }
        Ok(Self(value))
    }
}

impl<'de> Deserialize<'de> for BoundedNote {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)
            .and_then(|value| Self::try_from(value).map_err(serde::de::Error::custom))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BoundedNoteError {
    #[error("recommendation feedback note is {actual} characters; maximum is {max}")]
    TooLong { max: usize, actual: usize },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ConversionTarget {
    Action(String),
    ClaimCorrection(ClaimId),
    ReviewQueue(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ConversionState {
    NotConverted,
    ConvertedToAction { action_id: String },
    ConvertedToClaimCorrection { claim_id: ClaimId },
    ConvertedToReviewQueue { queue_item_id: String },
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

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SuggestedNextStepsReadError {
    #[error("unsupported schema_version `{0}` for list_suggested_next_steps")]
    UnsupportedSchemaVersion(u16),
    #[error("claim `{0}` not found")]
    ClaimNotFound(String),
    #[error("claim `{0}` is not visible to the current actor")]
    ClaimNotVisible(String),
    #[error("{0}")]
    ReadFailed(String),
}
