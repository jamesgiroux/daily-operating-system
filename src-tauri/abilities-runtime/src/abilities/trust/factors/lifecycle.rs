use super::super::config::{FACTOR_MAX, FACTOR_MIN};
use super::super::types::{SourceLifecycleState, TrustFactorInputs};

pub fn source_lifecycle_weight(input: &TrustFactorInputs) -> f64 {
    match input.source_lifecycle {
        SourceLifecycleState::Active => FACTOR_MAX,
        SourceLifecycleState::Withdrawn | SourceLifecycleState::Dismissed => FACTOR_MIN,
    }
}
