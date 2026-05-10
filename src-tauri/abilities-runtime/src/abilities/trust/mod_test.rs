use chrono::{Duration, TimeZone, Utc};

use super::*;
use crate::abilities::provenance::SubjectRef;
use crate::sensitivity::ClaimVerificationState;
use crate::types::{ClaimSensitivity, ClaimState, SurfacingState, TemporalScope};

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
        source_asof: Some("2026-05-04T00:00:00Z".to_string()),
        observed_at: "2026-05-04T00:00:00Z".to_string(),
        created_at: "2026-05-04T00:00:00Z".to_string(),
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
        renewal_context: None,
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

fn factors(values: &[f64]) -> Vec<EvaluatedTrustFactor> {
    values
        .iter()
        .enumerate()
        .map(|(idx, value)| EvaluatedTrustFactor::new(factor_id(idx), *value, 1.0))
        .collect()
}

fn factor_id(idx: usize) -> super::factors::TrustFactorId {
    super::factors::FactorRegistry::IDS[idx % super::factors::TRUST_FACTOR_COUNT]
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

fn factor_evidence<'a>(computation: &'a TrustComputation, name: &str) -> &'a FactorEvidence {
    computation
        .evidence
        .factor_breakdown
        .iter()
        .find(|factor| factor.name == name)
        .unwrap_or_else(|| panic!("missing factor evidence for {name}"))
}

#[test]
fn trust_runtime_thresholds_have_no_float_literals_outside_config_and_tests() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/abilities/trust");
    let mut files = Vec::new();
    collect_rust_files(&root, &mut files);

    let mut offenders = Vec::new();
    for path in files {
        let file_name = path
            .file_name()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or_default();
        if file_name.ends_with("_test.rs") || file_name == "config.rs" {
            continue;
        }

        let contents = std::fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
        for (idx, line) in contents.lines().enumerate() {
            if contains_float_literal(line) {
                offenders.push(format!("{}:{}", path.display(), idx + 1));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "float literals in trust runtime files; move thresholds to config.rs or tests:\n{}",
        offenders.join("\n")
    );
}

fn collect_rust_files(dir: &std::path::Path, files: &mut Vec<std::path::PathBuf>) {
    for entry in
        std::fs::read_dir(dir).unwrap_or_else(|err| panic!("read_dir {}: {err}", dir.display()))
    {
        let entry = entry.expect("read trust source directory entry");
        let path = entry.path();
        if path.is_dir() {
            collect_rust_files(&path, files);
        } else if path.extension().and_then(std::ffi::OsStr::to_str) == Some("rs") {
            files.push(path);
        }
    }
}

fn contains_float_literal(line: &str) -> bool {
    let bytes = line.as_bytes();
    let mut idx = 0usize;
    while idx < bytes.len() {
        if !bytes[idx].is_ascii_digit() {
            idx += 1;
            continue;
        }

        let start = idx;
        while idx < bytes.len() && bytes[idx].is_ascii_digit() {
            idx += 1;
        }
        if idx >= bytes.len() || bytes[idx] != b'.' {
            continue;
        }
        let dot = idx;
        idx += 1;
        if idx >= bytes.len() || !bytes[idx].is_ascii_digit() {
            continue;
        }
        if start > 0 && is_identifier_byte(bytes[start - 1]) {
            continue;
        }

        while idx < bytes.len() && (bytes[idx].is_ascii_digit() || bytes[idx] == b'_') {
            idx += 1;
        }
        if dot + 1 < idx {
            return true;
        }
    }
    false
}

fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
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
fn factor_registry_public_api_evaluates_canonical_order() {
    let evaluation = super::factors::evaluate_factors(&test_claim(), &test_context());
    let names: Vec<&str> = evaluation
        .factors
        .iter()
        .map(|factor| factor.name)
        .collect();
    let registry_names: Vec<&str> = super::factors::FactorRegistry::IDS
        .iter()
        .map(|id| id.as_str())
        .collect();

    assert_eq!(names, registry_names);
    assert_eq!(evaluation.factors.len(), super::factors::TRUST_FACTOR_COUNT);
}

#[test]
fn factor_registry_resolves_renewal_at_for_imminent_renewal_notes() {
    let now = Utc.with_ymd_and_hms(2026, 5, 10, 12, 0, 0).unwrap();
    let mut claim = test_claim();
    claim.claim_type = "renewal_note".to_string();
    claim.field_path = Some("renewal.notes".to_string());
    claim.text = "Renewal note says the buyer is aligned.".to_string();
    claim.data_source = "manual".to_string();

    let mut ctx = test_context();
    ctx.now = now;
    ctx.renewal_context = Some(RenewalContext {
        renewal_at: Some(now + Duration::days(30)),
        days_to_renewal: None,
    });
    ctx.factor_inputs.freshness.age_days = 330.0;

    let evaluation = super::factors::evaluate_factors(&claim, &ctx);
    let freshness = evaluation
        .factors
        .iter()
        .find(|factor| factor.name == "freshness_weight")
        .expect("freshness factor present");
    let expected_imminent = 2.0_f64.powf(-ctx.factor_inputs.freshness.age_days / 400.0);
    let non_imminent = 2.0_f64.powf(-ctx.factor_inputs.freshness.age_days / 90.0);

    assert_close(freshness.raw_value, expected_imminent);
    assert!(
        (freshness.raw_value - non_imminent).abs() > 0.1,
        "freshness factor should not use the 90d non-imminent rule"
    );
    assert_eq!(
        evaluation
            .freshness_input
            .renewal_context
            .as_ref()
            .and_then(|context| context.days_to_renewal),
        Some(30)
    );
}

#[test]
fn compile_trust_applies_unknown_timestamp_penalty_from_typed_freshness_input() {
    let mut claim = test_claim();
    claim.created_at = "not-rfc3339".to_string();
    let mut ctx = test_context();
    ctx.config.weights = TrustFactorWeights {
        freshness_weight: 1.0,
        ..zero_weights()
    };
    ctx.factor_inputs.freshness.timestamp_known = false;
    ctx.factor_inputs.freshness.age_days = 0.0;

    let computation = compile_trust(&claim, ctx).unwrap();
    let freshness = factor_evidence(&computation, "freshness_weight");

    assert_close(
        freshness.raw_value,
        TrustConfig::default().unknown_timestamp_penalty,
    );
    assert!(computation
        .evidence
        .caveats
        .contains(&ConfidenceCaveat::UnknownTimestamp));
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
            CorroboratorWeight {
                evidence_weight: 0.2,
                confirms: true,
            },
            CorroboratorWeight {
                evidence_weight: 0.2,
                confirms: true,
            },
            CorroboratorWeight {
                evidence_weight: 0.2,
                confirms: true,
            },
            CorroboratorWeight {
                evidence_weight: 0.2,
                confirms: true,
            },
            CorroboratorWeight {
                evidence_weight: 0.2,
                confirms: true,
            },
            CorroboratorWeight {
                evidence_weight: 1.0,
                confirms: false,
            },
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
            CorroboratorWeight {
                evidence_weight: 1.0,
                confirms: true,
            },
            CorroboratorWeight {
                evidence_weight: 1.0,
                confirms: true,
            },
        ],
    };

    assert_close(factors::source_reliability_aggregated(&input), 1.0);
}

#[test]
fn source_reliability_uses_corroborators_when_present() {
    let mut ctx = test_context();
    ctx.factor_inputs.source_reliability = 1.0;
    ctx.factor_inputs.source_reliability_corroborators = vec![
        CorroboratorWeight {
            evidence_weight: 0.2,
            confirms: true,
        },
        CorroboratorWeight {
            evidence_weight: 1.0,
            confirms: false,
        },
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
            tuple.push(EvaluatedTrustFactor::new(factor_id(idx), raw, weight));
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
        CorroboratorWeight {
            evidence_weight: 1.0,
            confirms: true,
        },
        CorroboratorWeight {
            evidence_weight: 1.0,
            confirms: true,
        },
        CorroboratorWeight {
            evidence_weight: 1.0,
            confirms: true,
        },
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
        CorroboratorWeight {
            evidence_weight: 0.2,
            confirms: true,
        },
        CorroboratorWeight {
            evidence_weight: 0.2,
            confirms: true,
        },
        CorroboratorWeight {
            evidence_weight: 0.2,
            confirms: true,
        },
        CorroboratorWeight {
            evidence_weight: 0.2,
            confirms: true,
        },
        CorroboratorWeight {
            evidence_weight: 0.2,
            confirms: true,
        },
        CorroboratorWeight {
            evidence_weight: 1.0,
            confirms: false,
        },
        CorroboratorWeight {
            evidence_weight: 1.0,
            confirms: false,
        },
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
