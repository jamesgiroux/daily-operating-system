//! Frozen cross-lane contracts for the recommendations subsystem.
//!
//! Owns the `RecommendationClaim` DTO, salience-factor types,
//! `SurfacingDecision` shape, `EngagementSignal`, recommendation feedback,
//! and every other type consumed by more than one lane. Wire format
//! is camelCase JSON via serde; Rust/TS golden parity fixtures
//! belong here.

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};

use abilities_runtime::abilities::provenance::envelope::Provenance;
use abilities_runtime::abilities::provenance::subject::SubjectRef;
use abilities_runtime::abilities::trust::types::TrustBand;
use abilities_runtime::sensitivity::{ClaimVerificationState, RenderSurface};
use abilities_runtime::types::{ClaimState, SurfacingState};

pub const RECOMMENDATION_METADATA_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct ClaimId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceRef {
    pub source: String,
    pub chunk: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RecommendationDraft {
    pub subject: SubjectRef,
    pub recommended_action: RecommendedAction,
    pub evidence: Vec<EvidenceRef>,
    pub provenance_json: String,
    pub source_ref: Option<String>,
    pub source_asof: Option<DateTime<Utc>>,
    pub observed_at: DateTime<Utc>,
    pub text: String,
    pub salience: SalienceScore,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RecommendationClaim {
    pub claim_id: ClaimId,
    pub subject: SubjectRef,
    pub recommended_action: RecommendedAction,
    pub evidence: Vec<EvidenceRef>,
    pub provenance: Provenance,
    pub trust: TrustBand,
    pub salience: SalienceScore,
    pub feedback_state: FeedbackState,
    pub conversion_state: ConversionState,
    pub claim_state: ClaimState,
    pub surfacing_state: SurfacingState,
    pub verification_state: ClaimVerificationState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum RecommendedAction {
    ScheduleMeeting {
        entity_id: String,
        when_window: String,
        rationale: String,
    },
    SendMessage {
        entity_id: String,
        channel: String,
        suggested_topic: String,
    },
    ReviewClaim {
        claim_id: ClaimId,
        reason: String,
    },
    UpdateRecord {
        entity_id: String,
        field_path: String,
        suggested_value: String,
    },
    InvestigateChange {
        entity_id: String,
        change_summary: String,
    },
    Custom {
        action_kind: String,
        payload: serde_json::Value,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum FeedbackState {
    Pending,
    Decided(RecommendationFeedbackDecision),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum RecommendationFeedbackDecision {
    Accept {
        at: DateTime<Utc>,
    },
    Dismiss {
        at: DateTime<Utc>,
        reason: DismissReason,
    },
    NotUseful {
        at: DateTime<Utc>,
    },
    TooNoisy {
        at: DateTime<Utc>,
    },
    Convert {
        at: DateTime<Utc>,
        into: ConversionTarget,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
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

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl TryFrom<String> for BoundedNote {
    type Error = BoundedNoteError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let char_count = value.chars().count();
        if char_count > Self::MAX_CHARS {
            return Err(BoundedNoteError::TooLong {
                max: Self::MAX_CHARS,
                actual: char_count,
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
pub struct RecommendationFeedbackContext {
    pub surface: RenderSurface,
    pub invocation_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
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
        deadline: Option<DateTime<Utc>>,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WhyThisNow {
    pub primary_factor: SalienceFactorKind,
    pub text: String,
    pub triggers: Vec<TriggerRef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SurfacingDecision {
    Render {
        tier: SurfacingTier,
        why_this_now: WhyThisNow,
    },
    Defer {
        until: DateTime<Utc>,
        reason: DeferReason,
    },
    Suppress {
        reason: SuppressReason,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum SurfacingTier {
    Critical,
    Notable,
    Background,
    Quiet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum DeferReason {
    CooldownActive,
    BudgetExhausted,
    AwaitingCorroboration,
    PendingTrigger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum SuppressReason {
    BelowThreshold,
    UserMutedSubject,
    DismissedRecently,
    ContradictedWithStrongerEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TriggerRef {
    pub trigger_kind: TriggerKind,
    pub at: DateTime<Utc>,
    pub source: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum TriggerKind {
    SignalArrival,
    EntityChange,
    ScheduledScan,
    FeedbackEcho,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum EngagementSignal {
    Rendered {
        surface: RenderSurface,
        at: DateTime<Utc>,
    },
    Clicked {
        surface: RenderSurface,
        at: DateTime<Utc>,
    },
    Dismissed {
        surface: RenderSurface,
        at: DateTime<Utc>,
    },
    Ignored {
        surface: RenderSurface,
        render_at: DateTime<Utc>,
        ignored_at: DateTime<Utc>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RecommendationMetadataEnvelope {
    pub recommendation: RecommendationMetadataPayload,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RecommendationMetadataPayload {
    pub schema_version: u16,
    pub recommended_action: RecommendedAction,
    pub evidence: Vec<EvidenceRef>,
    pub salience: SalienceScore,
    pub feedback_state: FeedbackState,
    pub conversion_state: ConversionState,
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_json::json;

    use crate::abilities::claims::{
        metadata_for_claim_type, CanonicalSubjectType, ClaimActorClass, ClaimType,
        CommitPolicyClass, FreshnessDecayClass,
    };
    use abilities_runtime::types::{ClaimSensitivity, TemporalScope};

    #[test]
    fn recommendation_claim_type_registry_matches_adr_0125_defaults() {
        let metadata = metadata_for_claim_type(ClaimType::Recommendation);

        assert_eq!(ClaimType::Recommendation.as_str(), "recommendation");
        assert_eq!(metadata.name, "recommendation");
        assert_eq!(metadata.default_temporal_scope, TemporalScope::State);
        assert_eq!(metadata.default_sensitivity, ClaimSensitivity::Internal);
        assert_eq!(metadata.freshness_decay_class, FreshnessDecayClass::Medium);
        assert_eq!(metadata.commit_policy_class, CommitPolicyClass::Replace);
        assert_eq!(
            metadata.canonical_subject_types,
            &[
                CanonicalSubjectType::Account,
                CanonicalSubjectType::Project,
                CanonicalSubjectType::Person,
            ]
        );
        assert_eq!(metadata.allowed_actor_classes, &[ClaimActorClass::Agent]);
    }

    #[test]
    fn recommendation_contract_golden_wire_shape() {
        let at = "2026-05-26T12:00:00Z"
            .parse::<DateTime<Utc>>()
            .expect("valid fixture time");
        let deadline = "2026-05-30T12:00:00Z"
            .parse::<DateTime<Utc>>()
            .expect("valid fixture deadline");

        let fixture = json!({
            "metadata": RecommendationMetadataEnvelope {
                recommendation: RecommendationMetadataPayload {
                    schema_version: RECOMMENDATION_METADATA_SCHEMA_VERSION,
                    recommended_action: RecommendedAction::ScheduleMeeting {
                        entity_id: "acct-example".to_string(),
                        when_window: "next_week".to_string(),
                        rationale: "recent support trend needs follow-up".to_string(),
                    },
                    evidence: vec![EvidenceRef {
                        source: "claim:claim-source-1".to_string(),
                        chunk: Some("chunk-1".to_string()),
                    }],
                    salience: SalienceScore {
                        total: 0.73,
                        factors: vec![SalienceFactor {
                            kind: SalienceFactorKind::Urgency,
                            value: Some(0.8),
                            weight: 0.17,
                            rationale: FactorRationale::Urgency {
                                deadline: Some(deadline),
                                decay_factor: 0.92,
                            },
                        }],
                    },
                    feedback_state: FeedbackState::Pending,
                    conversion_state: ConversionState::NotConverted,
                },
            },
            "decidedFeedback": FeedbackState::Decided(
                RecommendationFeedbackDecision::Dismiss {
                    at,
                    reason: DismissReason::Other(
                        BoundedNote::try_from("already handled elsewhere".to_string())
                            .expect("fixture note fits"),
                    ),
                },
            ),
            "conversionTarget": ConversionTarget::ClaimCorrection(ClaimId("claim-correction-1".to_string())),
            "conversionState": ConversionState::ConvertedToReviewQueue {
                queue_item_id: "queue-item-1".to_string(),
            },
            "surfacing": SurfacingDecision::Render {
                tier: SurfacingTier::Notable,
                why_this_now: WhyThisNow {
                    primary_factor: SalienceFactorKind::Urgency,
                    text: "Salience driven by urgency.".to_string(),
                    triggers: vec![TriggerRef {
                        trigger_kind: TriggerKind::SignalArrival,
                        at,
                        source: "signal:workspace-file-changed".to_string(),
                    }],
                },
            },
            "engagement": EngagementSignal::Ignored {
                surface: RenderSurface::TauriEntityDetail,
                render_at: at,
                ignored_at: at,
            },
        });

        assert_eq!(
            fixture,
            json!({
                "metadata": {
                    "recommendation": {
                        "schemaVersion": 1,
                        "recommendedAction": {
                            "kind": "scheduleMeeting",
                            "entityId": "acct-example",
                            "whenWindow": "next_week",
                            "rationale": "recent support trend needs follow-up",
                        },
                        "evidence": [
                            {
                                "source": "claim:claim-source-1",
                                "chunk": "chunk-1",
                            },
                        ],
                        "salience": {
                            "total": 0.73,
                            "factors": [
                                {
                                    "kind": "urgency",
                                    "value": 0.8,
                                    "weight": 0.17,
                                    "rationale": {
                                        "kind": "urgency",
                                        "deadline": "2026-05-30T12:00:00Z",
                                        "decayFactor": 0.92,
                                    },
                                },
                            ],
                        },
                        "feedbackState": "pending",
                        "conversionState": {
                            "kind": "notConverted",
                        },
                    },
                },
                "decidedFeedback": {
                    "decided": {
                        "kind": "dismiss",
                        "at": "2026-05-26T12:00:00Z",
                        "reason": {
                            "other": "already handled elsewhere",
                        },
                    },
                },
                "conversionTarget": {
                    "claimCorrection": "claim-correction-1",
                },
                "conversionState": {
                    "kind": "convertedToReviewQueue",
                    "queueItemId": "queue-item-1",
                },
                "surfacing": {
                    "kind": "render",
                    "tier": "notable",
                    "whyThisNow": {
                        "primaryFactor": "urgency",
                        "text": "Salience driven by urgency.",
                        "triggers": [
                            {
                                "triggerKind": "signalArrival",
                                "at": "2026-05-26T12:00:00Z",
                                "source": "signal:workspace-file-changed",
                            },
                        ],
                    },
                },
                "engagement": {
                    "kind": "ignored",
                    "surface": "tauri_entity_detail",
                    "renderAt": "2026-05-26T12:00:00Z",
                    "ignoredAt": "2026-05-26T12:00:00Z",
                },
            })
        );
    }

    #[test]
    fn bounded_note_rejects_more_than_two_hundred_characters() {
        let oversized = "x".repeat(BoundedNote::MAX_CHARS + 1);

        assert!(matches!(
            BoundedNote::try_from(oversized),
            Err(BoundedNoteError::TooLong {
                max: 200,
                actual: 201
            })
        ));
    }
}
