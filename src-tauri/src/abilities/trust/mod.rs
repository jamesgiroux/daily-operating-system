//! Trust Compiler value objects and pure scoring entry points.
//!
//! This module deliberately contains no database access, wall-clock reads, or
//! signal emission types. Service code extracts deterministic inputs and passes
//! them into the pure compiler.

pub mod config;
pub mod factors;
pub mod types;

use factors::{
    contradiction_penalty, corroboration_weight, cross_entity_coherence, freshness_weight,
    internal_consistency, sensitivity_aware_filtering, source_lifecycle_weight,
    source_reliability, subject_fit_confidence, user_feedback_weight,
};

pub use config::{TrustConfig, TrustConfigError, TrustFactorWeights};
pub use types::{
    ConfidenceCaveat, ConfidenceEvidence, CorroboratorWeight, CrossEntityCoherenceInput,
    CrossEntityHit, CrossEntityHitKind, EntityFootprint, FactorEvidence, FreshnessContext,
    SourceLifecycleState, SourceReliabilityInput, SurfaceClass, TargetFootprint, TrustBand,
    TrustComputation, TrustContext, TrustFactorInputs, TrustGateKind, TrustScore,
    UserFeedbackSignal,
};

pub type ClaimRow = crate::db::claims::IntelligenceClaim;

const FACTOR_CEILING: f64 = 1.0;

#[derive(Debug, Clone, Copy)]
struct NamedFactor {
    name: &'static str,
    raw_value: f64,
    weight: f64,
}

pub fn compile_trust(
    claim: &ClaimRow,
    ctx: TrustContext,
) -> Result<TrustComputation, TrustConfigError> {
    validate_config(&ctx.config)?;
    validate_weights(ctx.config.weights)?;

    let cross_entity = cross_entity_coherence(&ctx.cross_entity, &ctx.config);
    let factors = [
        NamedFactor {
            name: "source_reliability",
            raw_value: source_reliability(&ctx.factor_inputs),
            weight: ctx.config.weights.source_reliability,
        },
        NamedFactor {
            name: "source_lifecycle_weight",
            raw_value: source_lifecycle_weight(&ctx.factor_inputs),
            weight: ctx.config.weights.source_lifecycle_weight,
        },
        NamedFactor {
            name: "freshness_weight",
            raw_value: freshness_weight(
                &ctx.factor_inputs.freshness,
                &claim.temporal_scope,
                &ctx.config,
            ),
            weight: ctx.config.weights.freshness_weight,
        },
        NamedFactor {
            name: "corroboration_weight",
            raw_value: corroboration_weight(&ctx.factor_inputs),
            weight: ctx.config.weights.corroboration_weight,
        },
        NamedFactor {
            name: "contradiction_penalty",
            raw_value: contradiction_penalty(&ctx.factor_inputs, &ctx.config),
            weight: ctx.config.weights.contradiction_penalty,
        },
        NamedFactor {
            name: "user_feedback_weight",
            raw_value: user_feedback_weight(&ctx.factor_inputs, &ctx.config),
            weight: ctx.config.weights.user_feedback_weight,
        },
        NamedFactor {
            name: "subject_fit_confidence",
            raw_value: subject_fit_confidence(&ctx.factor_inputs),
            weight: ctx.config.weights.subject_fit_confidence,
        },
        NamedFactor {
            name: "internal_consistency",
            raw_value: internal_consistency(&ctx.factor_inputs),
            weight: ctx.config.weights.internal_consistency,
        },
        NamedFactor {
            name: "cross_entity_coherence",
            raw_value: cross_entity.value,
            weight: ctx.config.weights.cross_entity_coherence,
        },
        NamedFactor {
            name: "sensitivity_aware_filtering",
            raw_value: sensitivity_aware_filtering(&claim.sensitivity, ctx.target_surface),
            weight: ctx.config.weights.sensitivity_aware_filtering,
        },
    ];

    let geometric_score = aggregate_geometric_mean(&factors, ctx.config.clamp_floor)?;
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
        &factors,
        &ctx,
        EvidenceContext {
            cross_entity_hit_count: cross_entity.hits.len(),
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

    if !(0.0..=FACTOR_CEILING).contains(&config.clamp_floor) || config.clamp_floor == 0.0 {
        return Err(TrustConfigError::InvalidValue {
            name: "clamp_floor",
        });
    }
    if config.freshness_half_life_days <= 0.0 {
        return Err(TrustConfigError::InvalidValue {
            name: "freshness_half_life_days",
        });
    }
    if !(0.0..=1.0).contains(&config.likely_current_min) {
        return Err(TrustConfigError::InvalidValue {
            name: "likely_current_min",
        });
    }
    if !(0.0..=1.0).contains(&config.use_with_caution_min) {
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
    let mut denominator = 0.0;

    for (name, weight) in weights.as_named_weights() {
        if !weight.is_finite() {
            return Err(TrustConfigError::NonFiniteWeight { name });
        }
        if weight < 0.0 {
            return Err(TrustConfigError::NegativeWeight { name });
        }
        if weight > 0.0 {
            positive_count += 1;
            denominator += weight;
        }
    }

    if positive_count == 0 {
        return Err(TrustConfigError::NoPositiveWeights);
    }
    if denominator <= 0.0 || !denominator.is_finite() {
        return Err(TrustConfigError::NonPositiveDenominator);
    }

    Ok(())
}

fn aggregate_geometric_mean(
    factors: &[NamedFactor],
    clamp_floor: f64,
) -> Result<f64, TrustConfigError> {
    let mut weighted_log_sum = 0.0;
    let mut denominator = 0.0;
    let mut positive_count = 0usize;

    for factor in factors {
        if !factor.raw_value.is_finite() {
            return Err(TrustConfigError::NonFiniteValue { name: factor.name });
        }
        if !factor.weight.is_finite() {
            return Err(TrustConfigError::NonFiniteWeight { name: factor.name });
        }
        if factor.weight < 0.0 {
            return Err(TrustConfigError::NegativeWeight { name: factor.name });
        }
        if factor.weight == 0.0 {
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
    if denominator <= 0.0 || !denominator.is_finite() {
        return Err(TrustConfigError::NonPositiveDenominator);
    }

    Ok((weighted_log_sum / denominator)
        .exp()
        .clamp(TrustScore::MIN, TrustScore::MAX))
}

fn clamp_factor(value: f64, clamp_floor: f64) -> f64 {
    value.clamp(clamp_floor, FACTOR_CEILING)
}

/// Hard-policy gates that run BEFORE the weighted geometric mean. Returning
/// these as separate evidence keeps the trust math composable for ordinary
/// factors while preventing dilution of blockers under default equal weights
/// (a single 0.0 across 10 weight-1 factors only drops the geometric mean to
/// roughly 0.74 — well above NeedsVerification).
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
        if sensitivity_aware_filtering(&claim.sensitivity, Some(surface)) <= 0.0 {
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
        if max_contradicting_weight >= 0.8 && confirming <= 0.5 * contradicting {
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
    let mut confirming = 0.0;
    let mut contradicting = 0.0;
    let mut max_contradicting_weight = 0.0_f64;
    for c in corroborators {
        if !c.evidence_weight.is_finite() {
            continue;
        }
        let w = c.evidence_weight.clamp(0.0, FACTOR_CEILING);
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
    (config.use_with_caution_min - config.clamp_floor.max(0.05))
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
    triggered_gates: &'a [TriggeredGate],
}

fn confidence_evidence(
    claim: &ClaimRow,
    outcome: ScoreOutcome,
    factors: &[NamedFactor],
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
                contribution: if factor.weight == 0.0 {
                    0.0
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
        caveats: caveats(claim, ctx, extras.cross_entity_hit_count, extras.triggered_gates),
    }
}

fn caveats(
    claim: &ClaimRow,
    ctx: &TrustContext,
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

    if ctx.factor_inputs.corroboration_strength <= 0.0 {
        caveats.push(ConfidenceCaveat::FewSources);
    }
    if !ctx.factor_inputs.freshness.timestamp_known {
        caveats.push(ConfidenceCaveat::UnknownTimestamp);
    }
    if ctx.factor_inputs.freshness.age_days > ctx.config.freshness_half_life_days {
        caveats.push(ConfidenceCaveat::StaleSource {
            source: claim.data_source.clone(),
            age_days: ctx.factor_inputs.freshness.age_days,
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
mod tests {
    use chrono::{TimeZone, Utc};

    use super::*;
    use crate::abilities::provenance::SubjectRef;
    use crate::db::claims::{
        ClaimSensitivity, ClaimState, ClaimVerificationState, SurfacingState, TemporalScope,
    };

    fn test_claim() -> ClaimRow {
        ClaimRow {
            id: "claim-1".to_string(),
            subject_ref: r#"{"account":"acct-target"}"#.to_string(),
            claim_type: "risk".to_string(),
            field_path: Some("risk.summary".to_string()),
            topic_key: None,
            text: "The target account has elevated renewal risk.".to_string(),
            dedup_key: "dedup-1".to_string(),
            item_hash: Some("hash-1".to_string()),
            actor: "agent:test".to_string(),
            data_source: "glean".to_string(),
            source_ref: None,
            source_asof: Some("2026-05-01T00:00:00Z".to_string()),
            observed_at: "2026-05-01T00:00:00Z".to_string(),
            created_at: "2026-05-01T00:00:00Z".to_string(),
            provenance_json: "{}".to_string(),
            metadata_json: None,
            claim_state: ClaimState::Active,
            surfacing_state: SurfacingState::Active,
            demotion_reason: None,
            reactivated_at: None,
            retraction_reason: None,
            expires_at: None,
            superseded_by: None,
            trust_score: None,
            trust_computed_at: None,
            trust_version: None,
            thread_id: None,
            temporal_scope: TemporalScope::State,
            sensitivity: ClaimSensitivity::Internal,
            verification_state: ClaimVerificationState::Active,
            verification_reason: None,
            needs_user_decision_at: None,
        }
    }

    fn test_context() -> TrustContext {
        TrustContext {
            now: Utc.with_ymd_and_hms(2026, 5, 4, 0, 0, 0).unwrap(),
            config: TrustConfig::default(),
            factor_inputs: TrustFactorInputs {
                source_reliability: 1.0,
                source_reliability_corroborators: Vec::new(),
                freshness: FreshnessContext {
                    timestamp_known: true,
                    age_days: 0.0,
                },
                corroboration_strength: 1.0,
                contradiction_count: 0,
                user_feedback: UserFeedbackSignal::None,
                subject_fit_confidence: 1.0,
                internal_consistency: 1.0,
                source_lifecycle: SourceLifecycleState::Active,
                read_state_indeterminate: false,
            },
            cross_entity: CrossEntityCoherenceInput {
                claim_text: "The target account has elevated renewal risk.".to_string(),
                target_footprint: TargetFootprint {
                    subject: SubjectRef::Account("acct-target".to_string()),
                    names: vec!["Target Account".to_string()],
                    domains: vec!["target.example".to_string()],
                    related_subjects: Vec::new(),
                    allowed_aliases: Vec::new(),
                },
                portfolio_footprints: vec![EntityFootprint {
                    subject: SubjectRef::Account("acct-other".to_string()),
                    names: vec!["Other Company".to_string()],
                    domains: vec!["other.example".to_string()],
                    infrastructure_ids: Vec::new(),
                }],
                cross_entity_context_expected: false,
            },
            target_surface: None,
        }
    }

    fn factors(values: &[f64]) -> Vec<NamedFactor> {
        values
            .iter()
            .enumerate()
            .map(|(idx, value)| NamedFactor {
                name: match idx {
                    0 => "factor_0",
                    1 => "factor_1",
                    2 => "factor_2",
                    3 => "factor_3",
                    4 => "factor_4",
                    5 => "factor_5",
                    _ => "factor_6",
                },
                raw_value: *value,
                weight: 1.0,
            })
            .collect()
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-12,
            "expected {expected}, got {actual}"
        );
    }

    fn zero_weights() -> TrustFactorWeights {
        TrustFactorWeights {
            source_reliability: 0.0,
            source_lifecycle_weight: 0.0,
            freshness_weight: 0.0,
            corroboration_weight: 0.0,
            contradiction_penalty: 0.0,
            user_feedback_weight: 0.0,
            subject_fit_confidence: 0.0,
            internal_consistency: 0.0,
            cross_entity_coherence: 0.0,
            sensitivity_aware_filtering: 0.0,
        }
    }

    fn factor_evidence<'a>(
        computation: &'a TrustComputation,
        name: &str,
    ) -> &'a FactorEvidence {
        computation
            .evidence
            .factor_breakdown
            .iter()
            .find(|factor| factor.name == name)
            .unwrap_or_else(|| panic!("missing factor evidence for {name}"))
    }

    #[test]
    fn trust_geometric_mean_all_floor_05_returns_floor() {
        let score = aggregate_geometric_mean(&factors(&[0.0; 10]), 0.05).unwrap();
        assert_close(score, 0.05);
    }

    #[test]
    fn trust_geometric_mean_all_one_returns_one() {
        let score = aggregate_geometric_mean(&factors(&[1.0; 10]), 0.05).unwrap();
        assert_close(score, 1.0);
    }

    #[test]
    fn trust_geometric_mean_mixed_08_in_band() {
        let score = aggregate_geometric_mean(&factors(&[0.64, 1.0]), 0.05).unwrap();
        assert_close(score, 0.8);
        assert_eq!(
            band_for_score(score, &TrustConfig::default()),
            TrustBand::LikelyCurrent
        );
    }

    #[test]
    fn trust_feedback_boost_clamped_to_ceiling() {
        let claim = test_claim();
        let mut ctx = test_context();
        ctx.config.weights = TrustFactorWeights {
            user_feedback_weight: 1.0,
            ..zero_weights()
        };
        ctx.factor_inputs.user_feedback = UserFeedbackSignal::Confirmed;

        let computation = compile_trust(&claim, ctx).unwrap();
        let feedback = factor_evidence(&computation, "user_feedback_weight");

        assert_close(feedback.raw_value, TrustConfig::default().feedback_boost);
        assert_close(feedback.value, 1.0);
        assert_close(computation.score.value(), 1.0);
    }

    #[test]
    fn trust_contradiction_present_downranks() {
        let claim = test_claim();
        let mut ctx = test_context();
        ctx.config.weights = TrustFactorWeights {
            contradiction_penalty: 1.0,
            ..zero_weights()
        };
        ctx.factor_inputs.contradiction_count = 1;

        let computation = compile_trust(&claim, ctx).unwrap();

        assert_close(
            computation.score.value(),
            1.0 - TrustConfig::default().contradiction_multiplier,
        );
        assert_eq!(computation.band, TrustBand::UseWithCaution);
        assert!(computation
            .evidence
            .caveats
            .contains(&ConfidenceCaveat::UnresolvedContradiction));
    }

    #[test]
    fn trust_factor_count_is_ten_canonical_factors() {
        let computation = compile_trust(&test_claim(), test_context()).unwrap();
        let names: Vec<&str> = computation
            .evidence
            .factor_breakdown
            .iter()
            .map(|factor| factor.name.as_str())
            .collect();

        assert_eq!(
            names,
            vec![
                "source_reliability",
                "source_lifecycle_weight",
                "freshness_weight",
                "corroboration_weight",
                "contradiction_penalty",
                "user_feedback_weight",
                "subject_fit_confidence",
                "internal_consistency",
                "cross_entity_coherence",
                "sensitivity_aware_filtering",
            ]
        );
        assert_eq!(names.len(), 10, "trust factor count");
    }

    #[test]
    fn internal_consistency_factor_returns_input_value() {
        let mut ctx = test_context();
        ctx.factor_inputs.internal_consistency = 0.37;

        assert_close(
            internal_consistency(&ctx.factor_inputs),
            ctx.factor_inputs.internal_consistency,
        );
    }

    #[test]
    fn compile_trust_low_internal_consistency_drops_score_into_needs_verification() {
        let claim = test_claim();
        let mut ctx = test_context();
        ctx.config.weights = TrustFactorWeights {
            internal_consistency: 1.0,
            ..zero_weights()
        };
        ctx.factor_inputs.internal_consistency = 0.20;

        let computation = compile_trust(&claim, ctx).unwrap();
        let consistency = factor_evidence(&computation, "internal_consistency");

        assert_close(consistency.raw_value, 0.20);
        assert_close(computation.score.value(), 0.20);
        assert_eq!(computation.band, TrustBand::NeedsVerification);
    }

    #[test]
    fn source_lifecycle_weight_withdrawn_and_dismissed_return_zero() {
        let mut ctx = test_context();

        ctx.factor_inputs.source_lifecycle = SourceLifecycleState::Withdrawn;
        assert_close(source_lifecycle_weight(&ctx.factor_inputs), 0.0);

        ctx.factor_inputs.source_lifecycle = SourceLifecycleState::Dismissed;
        assert_close(source_lifecycle_weight(&ctx.factor_inputs), 0.0);
    }

    #[test]
    fn compile_trust_withdrawn_source_lifecycle_clamps_to_floor() {
        let claim = test_claim();
        let mut ctx = test_context();
        ctx.config.weights = TrustFactorWeights {
            source_lifecycle_weight: 1.0,
            ..zero_weights()
        };
        ctx.factor_inputs.source_lifecycle = SourceLifecycleState::Withdrawn;

        let computation = compile_trust(&claim, ctx).unwrap();
        let lifecycle = factor_evidence(&computation, "source_lifecycle_weight");

        assert_close(lifecycle.raw_value, 0.0);
        assert_close(lifecycle.value, TrustConfig::default().clamp_floor);
        assert_close(
            computation.score.value(),
            TrustConfig::default().clamp_floor,
        );
        assert_eq!(computation.band, TrustBand::NeedsVerification);
    }

    #[test]
    fn source_reliability_aggregated_dominates_5_weak_with_1_strong_contradiction() {
        let input = SourceReliabilityInput {
            corroborators: vec![
                CorroboratorWeight { evidence_weight: 0.2, confirms: true },
                CorroboratorWeight { evidence_weight: 0.2, confirms: true },
                CorroboratorWeight { evidence_weight: 0.2, confirms: true },
                CorroboratorWeight { evidence_weight: 0.2, confirms: true },
                CorroboratorWeight { evidence_weight: 0.2, confirms: true },
                CorroboratorWeight { evidence_weight: 1.0, confirms: false },
            ],
        };

        assert_close(factors::source_reliability_aggregated(&input), 0.5);
    }

    #[test]
    fn source_reliability_aggregated_clamps_to_zero_when_no_corroborators() {
        let input = SourceReliabilityInput {
            corroborators: Vec::new(),
        };

        assert_close(factors::source_reliability_aggregated(&input), 0.0);
    }

    #[test]
    fn source_reliability_aggregated_clamps_to_one_with_all_strong_confirms() {
        let input = SourceReliabilityInput {
            corroborators: vec![
                CorroboratorWeight { evidence_weight: 1.0, confirms: true },
                CorroboratorWeight { evidence_weight: 1.0, confirms: true },
            ],
        };

        assert_close(factors::source_reliability_aggregated(&input), 1.0);
    }

    #[test]
    fn source_reliability_uses_corroborators_when_present() {
        let mut ctx = test_context();
        ctx.factor_inputs.source_reliability = 1.0;
        ctx.factor_inputs.source_reliability_corroborators = vec![
            CorroboratorWeight { evidence_weight: 0.2, confirms: true },
            CorroboratorWeight { evidence_weight: 1.0, confirms: false },
        ];

        assert_close(source_reliability(&ctx.factor_inputs), 1.0 / 6.0);
    }

    #[test]
    fn compile_trust_scores_low_for_private_claim_on_public_surface_via_floor_clamp() {
        let mut claim = test_claim();
        claim.sensitivity = ClaimSensitivity::Confidential;
        let mut ctx = test_context();
        ctx.target_surface = Some(SurfaceClass::Public);
        ctx.config.weights = TrustFactorWeights {
            sensitivity_aware_filtering: 1.0,
            ..zero_weights()
        };

        let computation = compile_trust(&claim, ctx).unwrap();
        let sensitivity = factor_evidence(&computation, "sensitivity_aware_filtering");

        assert_close(sensitivity.raw_value, 0.0);
        assert_close(sensitivity.value, TrustConfig::default().clamp_floor);
        assert_close(
            computation.score.value(),
            TrustConfig::default().clamp_floor,
        );
        assert!(computation.score.value() < 0.5);
        assert_eq!(computation.band, TrustBand::NeedsVerification);
    }

    #[test]
    fn compile_trust_passes_for_public_claim_on_public_surface_with_normal_other_factors() {
        let mut claim = test_claim();
        claim.sensitivity = ClaimSensitivity::Public;
        let mut ctx = test_context();
        ctx.target_surface = Some(SurfaceClass::Public);

        let computation = compile_trust(&claim, ctx).unwrap();
        let sensitivity = factor_evidence(&computation, "sensitivity_aware_filtering");

        assert_close(sensitivity.raw_value, 1.0);
        assert_close(sensitivity.value, 1.0);
        assert_close(computation.score.value(), 1.0);
        assert_eq!(computation.band, TrustBand::LikelyCurrent);
    }

    #[test]
    fn compile_trust_target_surface_none_does_not_apply_sensitivity_filter() {
        let mut claim = test_claim();
        claim.sensitivity = ClaimSensitivity::UserOnly;
        let ctx = test_context();

        let computation = compile_trust(&claim, ctx).unwrap();
        let sensitivity = factor_evidence(&computation, "sensitivity_aware_filtering");

        assert_close(sensitivity.raw_value, 1.0);
        assert_close(sensitivity.value, 1.0);
        assert_close(computation.score.value(), 1.0);
        assert_eq!(computation.band, TrustBand::LikelyCurrent);
    }

    #[test]
    fn corroboration_zero_rows_clamps_to_floor() {
        let claim = test_claim();
        let mut ctx = test_context();
        ctx.config.weights = TrustFactorWeights {
            corroboration_weight: 1.0,
            ..zero_weights()
        };
        ctx.factor_inputs.corroboration_strength = 0.0;

        let computation = compile_trust(&claim, ctx).unwrap();
        let corroboration = factor_evidence(&computation, "corroboration_weight");

        assert_close(corroboration.raw_value, 0.0);
        assert_close(corroboration.value, TrustConfig::default().clamp_floor);
        assert_close(
            computation.score.value(),
            TrustConfig::default().clamp_floor,
        );
        assert!(computation
            .evidence
            .caveats
            .contains(&ConfidenceCaveat::FewSources));
    }

    #[test]
    fn trust_rejects_non_finite_factor() {
        let claim = test_claim();
        let mut ctx = test_context();
        ctx.factor_inputs.source_reliability = f64::NAN;

        assert!(matches!(
            compile_trust(&claim, ctx),
            Err(TrustConfigError::NonFiniteValue {
                name: "source_reliability"
            })
        ));
    }

    #[test]
    fn trust_rejects_non_finite_weight() {
        let claim = test_claim();
        let mut ctx = test_context();
        ctx.config.weights.source_reliability = f64::INFINITY;

        assert!(matches!(
            compile_trust(&claim, ctx),
            Err(TrustConfigError::NonFiniteWeight {
                name: "source_reliability"
            })
        ));
    }

    #[test]
    fn trust_rejects_negative_weight() {
        let claim = test_claim();
        let mut ctx = test_context();
        ctx.config.weights.source_reliability = -0.1;

        assert!(matches!(
            compile_trust(&claim, ctx),
            Err(TrustConfigError::NegativeWeight {
                name: "source_reliability"
            })
        ));
    }

    #[test]
    fn trust_rejects_zero_positive_weight_denominator() {
        let claim = test_claim();
        let mut ctx = test_context();
        ctx.config.weights = TrustFactorWeights {
            source_reliability: 0.0,
            source_lifecycle_weight: 0.0,
            freshness_weight: 0.0,
            corroboration_weight: 0.0,
            contradiction_penalty: 0.0,
            user_feedback_weight: 0.0,
            subject_fit_confidence: 0.0,
            internal_consistency: 0.0,
            cross_entity_coherence: 0.0,
            sensitivity_aware_filtering: 0.0,
        };

        assert!(matches!(
            compile_trust(&claim, ctx),
            Err(TrustConfigError::NoPositiveWeights)
        ));
    }

    #[test]
    fn trust_band_mapping_at_canonical_thresholds() {
        let config = TrustConfig::default();

        assert_eq!(band_for_score(0.75, &config), TrustBand::LikelyCurrent);
        assert_eq!(
            band_for_score(0.749_999, &config),
            TrustBand::UseWithCaution
        );
        assert_eq!(band_for_score(0.50, &config), TrustBand::UseWithCaution);
        assert_eq!(
            band_for_score(0.499_999, &config),
            TrustBand::NeedsVerification
        );
    }

    #[test]
    fn trust_clamps_factor_to_floor_before_log() {
        let score = aggregate_geometric_mean(&factors(&[0.0, 1.0]), 0.05).unwrap();
        assert_close(score, 0.05_f64.sqrt());
    }

    #[test]
    fn trust_random_factor_tuples_nan_never_produced() {
        let mut state = 0xD05_0002_5EED_u64;

        for _ in 0..1_000 {
            let mut tuple = Vec::with_capacity(7);
            for idx in 0..7 {
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                let raw = ((state >> 32) % 20_001) as f64 / 10_000.0 - 0.5;
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                let weight = ((state >> 32) % 1_001) as f64 / 100.0;
                tuple.push(NamedFactor {
                    name: match idx {
                        0 => "factor_0",
                        1 => "factor_1",
                        2 => "factor_2",
                        3 => "factor_3",
                        4 => "factor_4",
                        5 => "factor_5",
                        _ => "factor_6",
                    },
                    raw_value: raw,
                    weight,
                });
            }
            tuple[0].weight = tuple[0].weight.max(0.1);

            let score = aggregate_geometric_mean(&tuple, 0.05).unwrap();
            assert!(score.is_finite());
            assert!((0.0..=1.0).contains(&score));
        }
    }

    #[test]
    fn trust_compiler_p99_under_5ms_claim_volume() {
        let claim = test_claim();
        let base_ctx = test_context();
        let mut samples = Vec::with_capacity(1_000);

        for idx in 0..1_000 {
            let mut ctx = base_ctx.clone();
            ctx.factor_inputs.source_reliability = 0.50 + (idx % 50) as f64 / 100.0;
            ctx.factor_inputs.corroboration_strength = if idx % 3 == 0 { 0.5 } else { 1.0 };
            ctx.factor_inputs.contradiction_count = if idx % 31 == 0 { 1 } else { 0 };
            ctx.cross_entity.claim_text = if idx % 17 == 0 {
                "The target account mentions other.example in a support note.".to_string()
            } else {
                "The target account has elevated renewal risk.".to_string()
            };

            let start = std::time::Instant::now();
            compile_trust(&claim, ctx).unwrap();
            samples.push(start.elapsed().as_micros());
        }

        samples.sort_unstable();
        let p99 = samples[(samples.len() * 99) / 100];
        eprintln!(
            "[trust compiler] p99={}us samples={} threshold=5000us",
            p99,
            samples.len()
        );
        assert!(
            p99 < 5_000,
            "trust compiler p99 {p99}us exceeded 5ms budget"
        );
    }

    fn gate_caveat(computation: &TrustComputation) -> Option<&ConfidenceCaveat> {
        computation
            .evidence
            .caveats
            .iter()
            .find(|c| matches!(c, ConfidenceCaveat::TrustGateTriggered { .. }))
    }

    #[test]
    fn confidential_on_public_caps_at_needs_verification_under_default_weights() {
        // Without a gate, geometric mean over 10 weight-1 factors with one 0.0
        // (clamped to 0.05) is exp(ln(0.05)/10) ≈ 0.741, which is UseWithCaution.
        // The sensitivity gate must override that and force NeedsVerification.
        let mut claim = test_claim();
        claim.sensitivity = ClaimSensitivity::Confidential;
        let mut ctx = test_context();
        ctx.target_surface = Some(SurfaceClass::Public);

        let computation = compile_trust(&claim, ctx).unwrap();
        assert!(
            computation.score.value() < 0.5,
            "confidential-on-public must be NeedsVerification, got {}",
            computation.score.value()
        );
        assert_eq!(computation.band, TrustBand::NeedsVerification);
        let caveat = gate_caveat(&computation).expect("sensitivity gate caveat");
        assert!(
            matches!(
                caveat,
                ConfidenceCaveat::TrustGateTriggered {
                    gate: TrustGateKind::SensitivityViolation,
                    ..
                }
            ),
            "expected SensitivityViolation, got {caveat:?}"
        );
    }

    #[test]
    fn withdrawn_source_caps_at_needs_verification_under_default_weights() {
        let claim = test_claim();
        let mut ctx = test_context();
        ctx.factor_inputs.source_lifecycle = SourceLifecycleState::Withdrawn;

        let computation = compile_trust(&claim, ctx).unwrap();
        assert!(
            computation.score.value() < 0.5,
            "withdrawn source must be NeedsVerification, got {}",
            computation.score.value()
        );
        assert_eq!(computation.band, TrustBand::NeedsVerification);
        let caveat = gate_caveat(&computation).expect("withdrawn source gate caveat");
        assert!(
            matches!(
                caveat,
                ConfidenceCaveat::TrustGateTriggered {
                    gate: TrustGateKind::SourceWithdrawn,
                    ..
                }
            ),
            "expected SourceWithdrawn, got {caveat:?}"
        );
    }

    #[test]
    fn indeterminate_read_state_caps_at_needs_verification_even_with_strong_confirming_evidence() {
        // A partial DB read from corroborator/contradiction queries previously
        // let strong confirming evidence preserve a LikelyCurrent score
        // because the synthetic-contradicting fail-closed path could be
        // outweighed. The IndeterminateReadState gate fires regardless of
        // factor weights and pins the band to NeedsVerification.
        let claim = test_claim();
        let mut ctx = test_context();
        ctx.factor_inputs.read_state_indeterminate = true;
        ctx.factor_inputs.source_reliability_corroborators = vec![
            CorroboratorWeight { evidence_weight: 1.0, confirms: true },
            CorroboratorWeight { evidence_weight: 1.0, confirms: true },
            CorroboratorWeight { evidence_weight: 1.0, confirms: true },
        ];

        let computation = compile_trust(&claim, ctx).unwrap();
        assert_eq!(computation.band, TrustBand::NeedsVerification);
        let caveat = gate_caveat(&computation).expect("indeterminate gate caveat");
        assert!(
            matches!(
                caveat,
                ConfidenceCaveat::TrustGateTriggered {
                    gate: TrustGateKind::IndeterminateReadState,
                    ..
                }
            ),
            "expected IndeterminateReadState, got {caveat:?}"
        );
    }

    #[test]
    fn single_strong_contradiction_caps_at_needs_verification_under_default_weights() {
        // 5×0.2 weak confirms (sum 1.0) vs 1×1.0 strong contradiction. Confirming
        // (1.0) is exactly equal to contradicting (1.0), so the existing factor-level
        // arithmetic would not flag it. The authoritative gate fires once the strong
        // contradicting weight (≥0.8) outweighs confirming by 2×, so we lift the
        // contradicting side to make the asymmetry explicit.
        let claim = test_claim();
        let mut ctx = test_context();
        ctx.factor_inputs.source_reliability_corroborators = vec![
            CorroboratorWeight { evidence_weight: 0.2, confirms: true },
            CorroboratorWeight { evidence_weight: 0.2, confirms: true },
            CorroboratorWeight { evidence_weight: 0.2, confirms: true },
            CorroboratorWeight { evidence_weight: 0.2, confirms: true },
            CorroboratorWeight { evidence_weight: 0.2, confirms: true },
            CorroboratorWeight { evidence_weight: 1.0, confirms: false },
            CorroboratorWeight { evidence_weight: 1.0, confirms: false },
        ];

        let computation = compile_trust(&claim, ctx).unwrap();
        assert!(
            computation.score.value() < 0.5,
            "authoritative contradiction must be NeedsVerification, got {}",
            computation.score.value()
        );
        assert_eq!(computation.band, TrustBand::NeedsVerification);
        let caveat = gate_caveat(&computation).expect("contradiction gate caveat");
        assert!(
            matches!(
                caveat,
                ConfidenceCaveat::TrustGateTriggered {
                    gate: TrustGateKind::AuthoritativeContradiction,
                    ..
                }
            ),
            "expected AuthoritativeContradiction, got {caveat:?}"
        );
    }
}
