//! Trust Compiler value objects and pure scoring entry points.
//!
//! This module deliberately contains no database access, wall-clock reads, or
//! signal emission types. Service code extracts deterministic inputs and passes
//! them into the pure compiler.

pub mod config;
pub mod factors;
pub mod freshness_decay;
pub mod types;

use factors::{
    freshness_threshold_days, sensitivity_aware_filtering, EvaluatedTrustFactor,
    FreshnessFactorInput, FACTOR_REGISTRY,
};

#[cfg(test)]
use factors::{internal_consistency, source_lifecycle_weight, source_reliability};

pub use config::{TrustConfig, TrustConfigError, TrustFactorWeights};
use config::{
    AUTHORITATIVE_CONFIRMING_RATIO, AUTHORITATIVE_CONTRADICTION_MIN_WEIGHT, DEFAULT_CLAMP_FLOOR,
    FACTOR_MAX, FACTOR_MIN,
};
pub use freshness_decay::{
    freshness_weight, half_life_for, validate_freshness_decay_config, RenewalContext,
    ScoringContext,
};
pub use types::{
    ConfidenceCaveat, ConfidenceEvidence, CorroboratorWeight, CrossEntityCoherenceInput,
    CrossEntityHit, CrossEntityHitKind, EntityFootprint, FactorEvidence, FreshnessContext,
    SourceLifecycleState, SourceReliabilityInput, SurfaceClass, TargetFootprint, TrustBand,
    TrustComputation, TrustContext, TrustFactorInputs, TrustGateKind, TrustScore,
    UserFeedbackSignal,
};

pub type ClaimRow = crate::types::IntelligenceClaim;

pub fn compile_trust(
    claim: &ClaimRow,
    ctx: TrustContext,
) -> Result<TrustComputation, TrustConfigError> {
    validate_config(&ctx.config)?;
    validate_weights(ctx.config.weights)?;

    let factor_evaluation = FACTOR_REGISTRY.evaluate(claim, &ctx);
    let factors = factor_evaluation.factors.as_slice();

    let geometric_score = aggregate_geometric_mean(factors, ctx.config.clamp_floor)?;
    let triggered_gates = evaluate_trust_gates(claim, &ctx);
    let (score, band) = if triggered_gates.is_empty() {
        let s = geometric_score;
        (s, band_for_score(s, &ctx.config))
    } else {
        // Force NeedsVerification on any triggered gate regardless of how
        // TrustConfig thresholds are tuned. The numeric cap stays as
        // additional belt-and-suspenders for callers that read the score
        // directly, but the band is the contract surface.
        let s = gate_cap_score(&ctx.config).min(geometric_score);
        (s, TrustBand::NeedsVerification)
    };
    let evidence = confidence_evidence(
        claim,
        ScoreOutcome { score, band },
        factors,
        &ctx,
        EvidenceContext {
            cross_entity_hit_count: factor_evaluation.cross_entity.hits.len(),
            freshness_input: &factor_evaluation.freshness_input,
            triggered_gates: &triggered_gates,
        },
    );

    Ok(TrustComputation {
        score: TrustScore(score),
        band,
        evidence,
    })
}

fn validate_config(config: &TrustConfig) -> Result<(), TrustConfigError> {
    let finite_values = [
        ("clamp_floor", config.clamp_floor),
        ("likely_current_min", config.likely_current_min),
        ("use_with_caution_min", config.use_with_caution_min),
        ("freshness_half_life_days", config.freshness_half_life_days),
        (
            "unknown_timestamp_penalty",
            config.unknown_timestamp_penalty,
        ),
        ("contradiction_multiplier", config.contradiction_multiplier),
        ("feedback_boost", config.feedback_boost),
        ("feedback_penalty", config.feedback_penalty),
        ("cross_entity_hit_penalty", config.cross_entity_hit_penalty),
    ];

    for (name, value) in finite_values {
        if !value.is_finite() {
            return Err(TrustConfigError::NonFiniteValue { name });
        }
    }

    if !(FACTOR_MIN..=FACTOR_MAX).contains(&config.clamp_floor) || config.clamp_floor == FACTOR_MIN
    {
        return Err(TrustConfigError::InvalidValue {
            name: "clamp_floor",
        });
    }
    if config.freshness_half_life_days <= FACTOR_MIN {
        return Err(TrustConfigError::InvalidValue {
            name: "freshness_half_life_days",
        });
    }
    if !(FACTOR_MIN..=FACTOR_MAX).contains(&config.likely_current_min) {
        return Err(TrustConfigError::InvalidValue {
            name: "likely_current_min",
        });
    }
    if !(FACTOR_MIN..=FACTOR_MAX).contains(&config.use_with_caution_min) {
        return Err(TrustConfigError::InvalidValue {
            name: "use_with_caution_min",
        });
    }
    if config.likely_current_min < config.use_with_caution_min {
        return Err(TrustConfigError::InvalidValue {
            name: "likely_current_min",
        });
    }

    Ok(())
}

fn validate_weights(weights: TrustFactorWeights) -> Result<(), TrustConfigError> {
    let mut positive_count = 0usize;
    let mut denominator = FACTOR_MIN;

    for (name, weight) in weights.as_named_weights() {
        if !weight.is_finite() {
            return Err(TrustConfigError::NonFiniteWeight { name });
        }
        if weight < FACTOR_MIN {
            return Err(TrustConfigError::NegativeWeight { name });
        }
        if weight > FACTOR_MIN {
            positive_count += 1;
            denominator += weight;
        }
    }

    if positive_count == 0 {
        return Err(TrustConfigError::NoPositiveWeights);
    }
    if denominator <= FACTOR_MIN || !denominator.is_finite() {
        return Err(TrustConfigError::NonPositiveDenominator);
    }

    Ok(())
}

fn aggregate_geometric_mean(
    factors: &[EvaluatedTrustFactor],
    clamp_floor: f64,
) -> Result<f64, TrustConfigError> {
    let mut weighted_log_sum = FACTOR_MIN;
    let mut denominator = FACTOR_MIN;
    let mut positive_count = 0usize;

    for factor in factors {
        if !factor.raw_value.is_finite() {
            return Err(TrustConfigError::NonFiniteValue { name: factor.name });
        }
        if !factor.weight.is_finite() {
            return Err(TrustConfigError::NonFiniteWeight { name: factor.name });
        }
        if factor.weight < FACTOR_MIN {
            return Err(TrustConfigError::NegativeWeight { name: factor.name });
        }
        if factor.weight == FACTOR_MIN {
            continue;
        }

        let clamped = clamp_factor(factor.raw_value, clamp_floor);
        weighted_log_sum += factor.weight * clamped.ln();
        denominator += factor.weight;
        positive_count += 1;
    }

    if positive_count == 0 {
        return Err(TrustConfigError::NoPositiveWeights);
    }
    if denominator <= FACTOR_MIN || !denominator.is_finite() {
        return Err(TrustConfigError::NonPositiveDenominator);
    }

    Ok((weighted_log_sum / denominator)
        .exp()
        .clamp(TrustScore::MIN, TrustScore::MAX))
}

fn clamp_factor(value: f64, clamp_floor: f64) -> f64 {
    value.clamp(clamp_floor, FACTOR_MAX)
}

/// Hard-policy gates that run BEFORE the weighted geometric mean. Returning
/// these as separate evidence keeps the trust math composable for ordinary
/// factors while preventing dilution of blockers under default equal weights.
#[derive(Debug, Clone)]
struct TriggeredGate {
    kind: TrustGateKind,
    detail: String,
}

fn evaluate_trust_gates(claim: &ClaimRow, ctx: &TrustContext) -> Vec<TriggeredGate> {
    let mut gates = Vec::new();

    if ctx.factor_inputs.read_state_indeterminate {
        gates.push(TriggeredGate {
            kind: TrustGateKind::IndeterminateReadState,
            detail: "one or more upstream trust-input reads failed; recompute cannot proceed on a partial picture".to_string(),
        });
    }

    if let Some(surface) = ctx.target_surface {
        if sensitivity_aware_filtering(&claim.sensitivity, Some(surface)) <= FACTOR_MIN {
            gates.push(TriggeredGate {
                kind: TrustGateKind::SensitivityViolation,
                detail: format!(
                    "{:?} claim cannot render on {:?} surface",
                    claim.sensitivity, surface
                ),
            });
        }
    }

    if matches!(
        ctx.factor_inputs.source_lifecycle,
        SourceLifecycleState::Withdrawn
    ) {
        gates.push(TriggeredGate {
            kind: TrustGateKind::SourceWithdrawn,
            detail: "source is withdrawn; cannot resurrect with fresh content".to_string(),
        });
    }

    if let Some((confirming, contradicting, max_contradicting_weight)) =
        contradicting_corroborator_summary(&ctx.factor_inputs.source_reliability_corroborators)
    {
        if max_contradicting_weight >= AUTHORITATIVE_CONTRADICTION_MIN_WEIGHT
            && confirming <= AUTHORITATIVE_CONFIRMING_RATIO * contradicting
        {
            gates.push(TriggeredGate {
                kind: TrustGateKind::AuthoritativeContradiction,
                detail: format!(
                    "authoritative contradicting evidence (max weight {max_contradicting_weight:.2}) \
                     outweighs confirming evidence ({confirming:.2} vs {contradicting:.2})"
                ),
            });
        }
    }

    gates
}

fn contradicting_corroborator_summary(
    corroborators: &[CorroboratorWeight],
) -> Option<(f64, f64, f64)> {
    if corroborators.is_empty() {
        return None;
    }
    let mut confirming = FACTOR_MIN;
    let mut contradicting = FACTOR_MIN;
    let mut max_contradicting_weight = FACTOR_MIN;
    for c in corroborators {
        if !c.evidence_weight.is_finite() {
            continue;
        }
        let w = c.evidence_weight.clamp(FACTOR_MIN, FACTOR_MAX);
        if c.confirms {
            confirming += w;
        } else {
            contradicting += w;
            if w > max_contradicting_weight {
                max_contradicting_weight = w;
            }
        }
    }
    Some((confirming, contradicting, max_contradicting_weight))
}

/// Cap a gated score safely below `use_with_caution_min` so the band lands at
/// NeedsVerification regardless of weight tuning. Sits one clamp_floor below
/// the threshold to keep room for adjacent factor evidence to differentiate.
fn gate_cap_score(config: &TrustConfig) -> f64 {
    (config.use_with_caution_min - config.clamp_floor.max(DEFAULT_CLAMP_FLOOR))
        .max(TrustScore::MIN)
        .min(config.use_with_caution_min)
}

fn band_for_score(score: f64, config: &TrustConfig) -> TrustBand {
    if score >= config.likely_current_min {
        TrustBand::LikelyCurrent
    } else if score >= config.use_with_caution_min {
        TrustBand::UseWithCaution
    } else {
        TrustBand::NeedsVerification
    }
}

struct ScoreOutcome {
    score: f64,
    band: TrustBand,
}

struct EvidenceContext<'a> {
    cross_entity_hit_count: usize,
    freshness_input: &'a FreshnessFactorInput,
    triggered_gates: &'a [TriggeredGate],
}

fn confidence_evidence(
    claim: &ClaimRow,
    outcome: ScoreOutcome,
    factors: &[EvaluatedTrustFactor],
    ctx: &TrustContext,
    extras: EvidenceContext<'_>,
) -> ConfidenceEvidence {
    let clamp_floor = ctx.config.clamp_floor;
    let factor_breakdown = factors
        .iter()
        .map(|factor| {
            let clamped = clamp_factor(factor.raw_value, clamp_floor);
            FactorEvidence {
                name: factor.name.to_string(),
                weight: factor.weight,
                raw_value: factor.raw_value,
                value: clamped,
                contribution: if factor.weight == FACTOR_MIN {
                    FACTOR_MIN
                } else {
                    factor.weight * clamped.ln()
                },
            }
        })
        .collect();

    ConfidenceEvidence {
        score: outcome.score,
        band_label: band_label(outcome.band).to_string(),
        factor_breakdown,
        caveats: caveats(
            claim,
            ctx,
            extras.freshness_input,
            extras.cross_entity_hit_count,
            extras.triggered_gates,
        ),
    }
}

fn caveats(
    claim: &ClaimRow,
    ctx: &TrustContext,
    freshness_input: &FreshnessFactorInput,
    cross_entity_hit_count: usize,
    triggered_gates: &[TriggeredGate],
) -> Vec<ConfidenceCaveat> {
    let mut caveats = Vec::new();
    for gate in triggered_gates {
        caveats.push(ConfidenceCaveat::TrustGateTriggered {
            gate: gate.kind,
            detail: gate.detail.clone(),
        });
    }

    if ctx.factor_inputs.corroboration_strength <= FACTOR_MIN {
        caveats.push(ConfidenceCaveat::FewSources);
    }
    if !freshness_input.timestamp_known {
        caveats.push(ConfidenceCaveat::UnknownTimestamp);
    }
    if freshness_input.age_days > freshness_threshold_days(freshness_input, &ctx.config) {
        caveats.push(ConfidenceCaveat::StaleSource {
            source: claim.data_source.clone(),
            age_days: freshness_input.age_days,
        });
    }
    if ctx.factor_inputs.contradiction_count > 0 {
        caveats.push(ConfidenceCaveat::UnresolvedContradiction);
    }
    if cross_entity_hit_count > 0 {
        caveats.push(ConfidenceCaveat::CrossEntityReferences {
            hit_count: cross_entity_hit_count,
        });
    }

    caveats
}

fn band_label(band: TrustBand) -> &'static str {
    match band {
        TrustBand::LikelyCurrent => "likely_current",
        TrustBand::UseWithCaution => "use_with_caution",
        TrustBand::NeedsVerification => "needs_verification",
        TrustBand::Unscored => "unscored",
    }
}

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
