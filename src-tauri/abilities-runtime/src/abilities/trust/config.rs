use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const FACTOR_MIN: f64 = 0.0;
pub const FACTOR_MAX: f64 = 1.0;
pub(crate) const FACTOR_AVERAGE_DENOMINATOR: f64 = 2.0;
pub(crate) const DEFAULT_CLAMP_FLOOR: f64 = 0.05;
pub(crate) const DEFAULT_LIKELY_CURRENT_MIN: f64 = 0.75;
pub(crate) const DEFAULT_USE_WITH_CAUTION_MIN: f64 = 0.50;
pub(crate) const DEFAULT_FRESHNESS_HALF_LIFE_DAYS: f64 = 90.0;
pub(crate) const DEFAULT_UNKNOWN_TIMESTAMP_PENALTY: f64 = 0.8;
pub(crate) const DEFAULT_CONTRADICTION_MULTIPLIER: f64 = 0.35;
pub(crate) const DEFAULT_FEEDBACK_BOOST: f64 = 1.2;
pub(crate) const DEFAULT_FEEDBACK_PENALTY: f64 = 0.25;
pub(crate) const DEFAULT_CROSS_ENTITY_HIT_PENALTY: f64 = 0.55;
pub(crate) const FRESHNESS_FLOOR: f64 = DEFAULT_CLAMP_FLOOR;
pub(crate) const FRESHNESS_EXPONENTIAL_BASE: f64 = 2.0;
pub(crate) const SECONDS_PER_DAY: f64 = 86_400.0;
pub(crate) const SALESFORCE_FIELD_UPDATE_STALE_WEIGHT: f64 = 0.3;
pub(crate) const AUTHORITATIVE_CONTRADICTION_MIN_WEIGHT: f64 = 0.8;
pub(crate) const AUTHORITATIVE_CONFIRMING_RATIO: f64 = 0.5;
pub(crate) const LINEAR_KNOWN_ATTRIBUTE_CHANGE_WEIGHT: f64 = 0.85;
pub(crate) const LINEAR_UNCATEGORIZED_ISSUE_WEIGHT: f64 = 0.65;
pub(crate) const LINEAR_SUBJECT_MISMATCH_WEIGHT: f64 = 0.50;

/// Tunable Trust Compiler configuration.
///
/// The compiler validates this shape before scoring. Defaults are intentionally
/// conservative and live with the trust composer until a shared scoring config
/// module exists.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TrustConfig {
    pub weights: TrustFactorWeights,
    pub clamp_floor: f64,
    pub likely_current_min: f64,
    pub use_with_caution_min: f64,
    pub freshness_half_life_days: f64,
    pub unknown_timestamp_penalty: f64,
    pub contradiction_multiplier: f64,
    pub feedback_boost: f64,
    pub feedback_penalty: f64,
    pub cross_entity_hit_penalty: f64,
}

impl Default for TrustConfig {
    fn default() -> Self {
        Self {
            weights: TrustFactorWeights::default(),
            clamp_floor: DEFAULT_CLAMP_FLOOR,
            likely_current_min: DEFAULT_LIKELY_CURRENT_MIN,
            use_with_caution_min: DEFAULT_USE_WITH_CAUTION_MIN,
            freshness_half_life_days: DEFAULT_FRESHNESS_HALF_LIFE_DAYS,
            unknown_timestamp_penalty: DEFAULT_UNKNOWN_TIMESTAMP_PENALTY,
            contradiction_multiplier: DEFAULT_CONTRADICTION_MULTIPLIER,
            feedback_boost: DEFAULT_FEEDBACK_BOOST,
            feedback_penalty: DEFAULT_FEEDBACK_PENALTY,
            cross_entity_hit_penalty: DEFAULT_CROSS_ENTITY_HIT_PENALTY,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TrustFactorWeights {
    pub source_reliability: f64,
    pub source_lifecycle_weight: f64,
    pub freshness_weight: f64,
    pub corroboration_weight: f64,
    pub contradiction_penalty: f64,
    pub user_feedback_weight: f64,
    pub subject_fit_confidence: f64,
    pub internal_consistency: f64,
    pub cross_entity_coherence: f64,
    pub sensitivity_aware_filtering: f64,
    pub linear_issue_state_weight: f64,
}

impl Default for TrustFactorWeights {
    fn default() -> Self {
        Self {
            source_reliability: FACTOR_MAX,
            source_lifecycle_weight: FACTOR_MAX,
            freshness_weight: FACTOR_MAX,
            corroboration_weight: FACTOR_MAX,
            contradiction_penalty: FACTOR_MAX,
            user_feedback_weight: FACTOR_MAX,
            subject_fit_confidence: FACTOR_MAX,
            internal_consistency: FACTOR_MAX,
            cross_entity_coherence: FACTOR_MAX,
            sensitivity_aware_filtering: FACTOR_MAX,
            linear_issue_state_weight: FACTOR_MAX,
        }
    }
}

impl TrustFactorWeights {
    pub const fn as_named_weights(self) -> [(&'static str, f64); 11] {
        [
            ("source_reliability", self.source_reliability),
            ("source_lifecycle_weight", self.source_lifecycle_weight),
            ("freshness_weight", self.freshness_weight),
            ("corroboration_weight", self.corroboration_weight),
            ("contradiction_penalty", self.contradiction_penalty),
            ("user_feedback_weight", self.user_feedback_weight),
            ("subject_fit_confidence", self.subject_fit_confidence),
            ("internal_consistency", self.internal_consistency),
            ("cross_entity_coherence", self.cross_entity_coherence),
            (
                "sensitivity_aware_filtering",
                self.sensitivity_aware_filtering,
            ),
            ("linear_issue_state_weight", self.linear_issue_state_weight),
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TrustConfigError {
    #[error("trust config value {name} must be finite")]
    NonFiniteValue { name: &'static str },
    #[error("trust config value {name} is invalid")]
    InvalidValue { name: &'static str },
    #[error("trust factor weight {name} must be finite")]
    NonFiniteWeight { name: &'static str },
    #[error("trust factor weight {name} must be non-negative")]
    NegativeWeight { name: &'static str },
    #[error("trust config must have at least one positive weight")]
    NoPositiveWeights,
    #[error("trust config denominator must be positive")]
    NonPositiveDenominator,
}
