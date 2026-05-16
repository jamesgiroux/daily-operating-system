//! Pure trust scoring factors.
//!
//! The compiler owns composition. This module owns primitive factor math and
//! typed surface extraction so each factor stays deterministic and side-effect
//! free.

mod consistency;
mod contradiction;
mod corroboration;
mod cross_entity;
mod feedback;
mod freshness;
mod lifecycle;
mod linear_issue_state;
mod reliability;
mod sensitivity;
mod subject;
mod surface;

use super::config::TrustFactorWeights;
use super::types::TrustContext;

pub use consistency::internal_consistency;
pub use contradiction::contradiction_penalty;
pub use corroboration::corroboration_weight;
pub use cross_entity::{cross_entity_coherence, CrossEntityCoherenceResult};
pub use feedback::user_feedback_weight;
pub use freshness::{
    freshness_factor_input_for_claim, freshness_threshold_days, freshness_weight,
    FreshnessFactorInput,
};
pub use lifecycle::source_lifecycle_weight;
pub use linear_issue_state::linear_issue_state_weight;
pub use reliability::{source_reliability, source_reliability_aggregated};
pub use sensitivity::sensitivity_aware_filtering;
pub use subject::subject_fit_confidence;
pub use surface::{
    surface_class_for_claim_surface, surface_class_for_render_surface,
    target_surface_for_claim_surface, target_surface_for_render_surface,
};

pub type Claim = crate::types::IntelligenceClaim;

pub const TRUST_FACTOR_COUNT: usize = 11;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustFactorId {
    SourceReliability,
    SourceLifecycleWeight,
    FreshnessWeight,
    CorroborationWeight,
    ContradictionPenalty,
    UserFeedbackWeight,
    SubjectFitConfidence,
    InternalConsistency,
    CrossEntityCoherence,
    SensitivityAwareFiltering,
    LinearIssueStateWeight,
}

impl TrustFactorId {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SourceReliability => "source_reliability",
            Self::SourceLifecycleWeight => "source_lifecycle_weight",
            Self::FreshnessWeight => "freshness_weight",
            Self::CorroborationWeight => "corroboration_weight",
            Self::ContradictionPenalty => "contradiction_penalty",
            Self::UserFeedbackWeight => "user_feedback_weight",
            Self::SubjectFitConfidence => "subject_fit_confidence",
            Self::InternalConsistency => "internal_consistency",
            Self::CrossEntityCoherence => "cross_entity_coherence",
            Self::SensitivityAwareFiltering => "sensitivity_aware_filtering",
            Self::LinearIssueStateWeight => "linear_issue_state_weight",
        }
    }

    pub const fn weight(self, weights: TrustFactorWeights) -> f64 {
        match self {
            Self::SourceReliability => weights.source_reliability,
            Self::SourceLifecycleWeight => weights.source_lifecycle_weight,
            Self::FreshnessWeight => weights.freshness_weight,
            Self::CorroborationWeight => weights.corroboration_weight,
            Self::ContradictionPenalty => weights.contradiction_penalty,
            Self::UserFeedbackWeight => weights.user_feedback_weight,
            Self::SubjectFitConfidence => weights.subject_fit_confidence,
            Self::InternalConsistency => weights.internal_consistency,
            Self::CrossEntityCoherence => weights.cross_entity_coherence,
            Self::SensitivityAwareFiltering => weights.sensitivity_aware_filtering,
            Self::LinearIssueStateWeight => weights.linear_issue_state_weight,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EvaluatedTrustFactor {
    pub id: TrustFactorId,
    pub name: &'static str,
    pub raw_value: f64,
    pub weight: f64,
}

impl EvaluatedTrustFactor {
    pub const fn new(id: TrustFactorId, raw_value: f64, weight: f64) -> Self {
        Self {
            id,
            name: id.as_str(),
            raw_value,
            weight,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FactorEvaluation {
    pub factors: [EvaluatedTrustFactor; TRUST_FACTOR_COUNT],
    pub cross_entity: CrossEntityCoherenceResult,
    pub freshness_input: FreshnessFactorInput,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FactorRegistry;

pub const FACTOR_REGISTRY: FactorRegistry = FactorRegistry;

impl FactorRegistry {
    pub const IDS: [TrustFactorId; TRUST_FACTOR_COUNT] = [
        TrustFactorId::SourceReliability,
        TrustFactorId::SourceLifecycleWeight,
        TrustFactorId::FreshnessWeight,
        TrustFactorId::CorroborationWeight,
        TrustFactorId::ContradictionPenalty,
        TrustFactorId::UserFeedbackWeight,
        TrustFactorId::SubjectFitConfidence,
        TrustFactorId::InternalConsistency,
        TrustFactorId::CrossEntityCoherence,
        TrustFactorId::SensitivityAwareFiltering,
        TrustFactorId::LinearIssueStateWeight,
    ];

    pub fn evaluate(self, claim: &Claim, ctx: &TrustContext) -> FactorEvaluation {
        let cross_entity = cross_entity_coherence(&ctx.cross_entity, &ctx.config);
        let freshness_input = freshness_factor_input_for_claim(
            claim,
            &ctx.factor_inputs.freshness,
            ctx.renewal_context.as_ref(),
            ctx.now,
        );
        let weights = ctx.config.weights;

        FactorEvaluation {
            factors: [
                evaluated_factor(
                    TrustFactorId::SourceReliability,
                    source_reliability(&ctx.factor_inputs),
                    weights,
                ),
                evaluated_factor(
                    TrustFactorId::SourceLifecycleWeight,
                    source_lifecycle_weight(&ctx.factor_inputs),
                    weights,
                ),
                evaluated_factor(
                    TrustFactorId::FreshnessWeight,
                    freshness_weight(&freshness_input, &ctx.config),
                    weights,
                ),
                evaluated_factor(
                    TrustFactorId::CorroborationWeight,
                    corroboration_weight(&ctx.factor_inputs),
                    weights,
                ),
                evaluated_factor(
                    TrustFactorId::ContradictionPenalty,
                    contradiction_penalty(&ctx.factor_inputs, &ctx.config),
                    weights,
                ),
                evaluated_factor(
                    TrustFactorId::UserFeedbackWeight,
                    user_feedback_weight(&ctx.factor_inputs, &ctx.config),
                    weights,
                ),
                evaluated_factor(
                    TrustFactorId::SubjectFitConfidence,
                    subject_fit_confidence(&ctx.factor_inputs),
                    weights,
                ),
                evaluated_factor(
                    TrustFactorId::InternalConsistency,
                    internal_consistency(&ctx.factor_inputs),
                    weights,
                ),
                evaluated_factor(
                    TrustFactorId::CrossEntityCoherence,
                    cross_entity.value,
                    weights,
                ),
                evaluated_factor(
                    TrustFactorId::SensitivityAwareFiltering,
                    sensitivity_aware_filtering(&claim.sensitivity, ctx.target_surface),
                    weights,
                ),
                evaluated_factor(
                    TrustFactorId::LinearIssueStateWeight,
                    linear_issue_state_weight(&ctx.factor_inputs),
                    weights,
                ),
            ],
            cross_entity,
            freshness_input,
        }
    }
}

pub fn evaluate_factors(claim: &Claim, ctx: &TrustContext) -> FactorEvaluation {
    FACTOR_REGISTRY.evaluate(claim, ctx)
}

fn evaluated_factor(
    id: TrustFactorId,
    raw_value: f64,
    weights: TrustFactorWeights,
) -> EvaluatedTrustFactor {
    EvaluatedTrustFactor::new(id, raw_value, id.weight(weights))
}
