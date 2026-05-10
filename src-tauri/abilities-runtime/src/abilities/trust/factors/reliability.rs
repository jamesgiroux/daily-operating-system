use super::super::config::{FACTOR_AVERAGE_DENOMINATOR, FACTOR_MAX, FACTOR_MIN};
use super::super::types::{CorroboratorWeight, SourceReliabilityInput, TrustFactorInputs};

pub fn source_reliability(input: &TrustFactorInputs) -> f64 {
    if input.source_reliability_corroborators.is_empty() {
        input.source_reliability
    } else {
        source_reliability_from_corroborators(&input.source_reliability_corroborators)
    }
}

pub fn source_reliability_aggregated(input: &SourceReliabilityInput) -> f64 {
    source_reliability_from_corroborators(&input.corroborators)
}

fn source_reliability_from_corroborators(corroborators: &[CorroboratorWeight]) -> f64 {
    if corroborators.is_empty() {
        return FACTOR_MIN;
    }

    let confirm_sum: f64 = corroborators
        .iter()
        .filter(|corroborator| corroborator.confirms)
        .map(|corroborator| corroborator.evidence_weight)
        .sum();
    let contradict_sum: f64 = corroborators
        .iter()
        .filter(|corroborator| !corroborator.confirms)
        .map(|corroborator| corroborator.evidence_weight)
        .sum();
    let net = (confirm_sum - contradict_sum) / (confirm_sum + contradict_sum).max(FACTOR_MAX);
    ((net + FACTOR_MAX) / FACTOR_AVERAGE_DENOMINATOR).clamp(FACTOR_MIN, FACTOR_MAX)
}
