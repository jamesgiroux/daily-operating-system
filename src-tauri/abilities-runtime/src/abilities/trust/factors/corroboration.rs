use super::super::types::TrustFactorInputs;

pub fn corroboration_weight(input: &TrustFactorInputs) -> f64 {
    input.corroboration_strength
}
