use chrono::{TimeZone, Utc};
use dailyos_lib::abilities::provenance::SubjectRef;
use dailyos_lib::abilities::trust::{factors::*, *};
use dailyos_lib::db::claims::*;
use serde_json::Value;

const METADATA_JSON: &str = include_str!("fixtures/bundle-8/metadata.json");

#[test]
fn sensitivity_class_leak_scores_confidential_claim_zero_for_public_surface() {
    assert_bundle_metadata();

    let claim = test_claim();
    let raw_sensitivity = sensitivity_aware_filtering(&claim.sensitivity, Some(SurfaceClass::Public));
    assert_close(raw_sensitivity, 0.0);

    let computation = compile_trust(
        &claim,
        TrustContext {
            now: Utc.with_ymd_and_hms(2026, 5, 4, 12, 0, 0).unwrap(),
            config: TrustConfig {
                weights: TrustFactorWeights {
                    sensitivity_aware_filtering: 1.0,
                    ..zero_weights()
                },
                ..TrustConfig::default()
            },
            factor_inputs: clean_factor_inputs(),
            cross_entity: clean_cross_entity_input(),
            target_surface: Some(SurfaceClass::Public),
        },
    )
    .expect("compile trust");

    let sensitivity = factor(&computation, "sensitivity_aware_filtering");
    assert_close(sensitivity.raw_value, 0.0);
    assert_close(sensitivity.value, TrustConfig::default().clamp_floor);
    assert!(
        computation.score.value() < 0.5,
        "confidential claim should not pass public-surface trust boundary, got {}",
        computation.score.value()
    );
    assert_eq!(computation.band, TrustBand::NeedsVerification);
}

fn assert_bundle_metadata() {
    let metadata: Value = serde_json::from_str(METADATA_JSON).expect("parse metadata");
    let factors = metadata["trust_factors_dominant"].as_array().expect("dominant factors")
        .iter().map(|value| value.as_str().expect("factor string")).collect::<Vec<_>>();
    assert_eq!(factors.as_slice(), ["sensitivity_aware_filtering"]);
    assert!(metadata["pass_fail_definition"].as_str().expect("pass/fail definition")
        .contains("public-class assessment output contains zero text segments"));
}

fn clean_factor_inputs() -> TrustFactorInputs {
    TrustFactorInputs {
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
    }
}

fn clean_cross_entity_input() -> CrossEntityCoherenceInput {
    CrossEntityCoherenceInput {
        claim_text: "Confidential stakeholder concern should not render on a public surface."
            .to_string(),
        target_footprint: TargetFootprint {
            subject: SubjectRef::Account("acct-test-1".to_string()),
            names: vec!["Acme Example".to_string()],
            domains: vec!["acme.example.com".to_string()],
            related_subjects: Vec::new(),
            allowed_aliases: Vec::new(),
        },
        portfolio_footprints: vec![EntityFootprint {
            subject: SubjectRef::Account("acct-test-2".to_string()),
            names: vec!["Subsidiary Example".to_string()],
            domains: vec!["subsidiary.com".to_string()],
            infrastructure_ids: Vec::new(),
        }],
        cross_entity_context_expected: false,
    }
}

fn test_claim() -> IntelligenceClaim {
    IntelligenceClaim {
        id: "claim-bundle-8-confidential-public-leak".to_string(),
        subject_ref: r#"{"kind":"account","id":"acct-test-1"}"#.to_string(),
        claim_type: "entity_summary".to_string(),
        field_path: Some("executiveAssessment".to_string()),
        topic_key: Some("acct-test-1:confidential-stakeholder-concern".to_string()),
        text: "Confidential stakeholder concern must not appear in public output.".to_string(),
        dedup_key: "acct-test-1|entity_summary|confidential-stakeholder-concern".to_string(),
        item_hash: Some("sha256:bundle8-confidential-public-leak".to_string()),
        actor: "agent:fixture".to_string(),
        data_source: "internal_note".to_string(),
        source_ref: Some("seeded-source-private".to_string()),
        source_asof: Some("2026-05-01T12:00:00Z".to_string()),
        observed_at: "2026-05-01T12:00:00Z".to_string(),
        created_at: "2026-05-04T12:00:00Z".to_string(),
        provenance_json: "{}".to_string(),
        metadata_json: None,
        claim_state: ClaimState::Active,
        surfacing_state: SurfacingState::Active,
        demotion_reason: None, reactivated_at: None, retraction_reason: None,
        expires_at: None, superseded_by: None, trust_score: None,
        trust_computed_at: None, trust_version: None, thread_id: None,
        temporal_scope: TemporalScope::State,
        sensitivity: ClaimSensitivity::Confidential,
        verification_state: ClaimVerificationState::Active,
        verification_reason: None, needs_user_decision_at: None,
    }
}

fn zero_weights() -> TrustFactorWeights {
    TrustFactorWeights {
        source_reliability: 0.0, source_lifecycle_weight: 0.0, freshness_weight: 0.0,
        corroboration_weight: 0.0, contradiction_penalty: 0.0, user_feedback_weight: 0.0,
        subject_fit_confidence: 0.0, internal_consistency: 0.0,
        cross_entity_coherence: 0.0, sensitivity_aware_filtering: 0.0,
    }
}

fn factor<'a>(computation: &'a TrustComputation, name: &str) -> &'a FactorEvidence {
    computation
        .evidence
        .factor_breakdown
        .iter()
        .find(|factor| factor.name == name)
        .unwrap_or_else(|| panic!("missing factor {name}"))
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-12,
        "expected {expected}, got {actual}"
    );
}
