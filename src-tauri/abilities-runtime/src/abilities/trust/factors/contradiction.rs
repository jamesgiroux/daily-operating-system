use super::super::config::{TrustConfig, FACTOR_MAX};
use super::super::types::TrustFactorInputs;

pub fn contradiction_penalty(input: &TrustFactorInputs, config: &TrustConfig) -> f64 {
    if input.contradiction_count == 0 {
        return FACTOR_MAX;
    }

    (FACTOR_MAX - config.contradiction_multiplier).powi(input.contradiction_count as i32)
}
