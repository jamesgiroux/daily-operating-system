use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
            clamp_floor: 0.05,
            likely_current_min: 0.75,
            use_with_caution_min: 0.50,
            freshness_half_life_days: 90.0,
            unknown_timestamp_penalty: 0.8,
            contradiction_multiplier: 0.35,
            feedback_boost: 1.2,
            feedback_penalty: 0.25,
            cross_entity_hit_penalty: 0.55,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TrustFactorWeights {
    pub source_reliability: f64,
    pub freshness_weight: f64,
    pub corroboration_weight: f64,
    pub contradiction_penalty: f64,
    pub user_feedback_weight: f64,
    pub subject_fit_confidence: f64,
    pub cross_entity_coherence: f64,
}

impl Default for TrustFactorWeights {
    fn default() -> Self {
        Self {
            source_reliability: 1.0,
            freshness_weight: 1.0,
            corroboration_weight: 1.0,
            contradiction_penalty: 1.0,
            user_feedback_weight: 1.0,
            subject_fit_confidence: 1.0,
            cross_entity_coherence: 1.0,
        }
    }
}

impl TrustFactorWeights {
    pub const fn as_named_weights(self) -> [(&'static str, f64); 7] {
        [
            ("source_reliability", self.source_reliability),
            ("freshness_weight", self.freshness_weight),
            ("corroboration_weight", self.corroboration_weight),
            ("contradiction_penalty", self.contradiction_penalty),
            ("user_feedback_weight", self.user_feedback_weight),
            ("subject_fit_confidence", self.subject_fit_confidence),
            ("cross_entity_coherence", self.cross_entity_coherence),
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
