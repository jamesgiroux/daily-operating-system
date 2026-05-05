use chrono::{TimeZone, Utc};
use dailyos_lib::abilities::provenance::SubjectRef;
use dailyos_lib::abilities::trust::{factors::*, *};
use dailyos_lib::db::claims::*;
use serde_json::Value;

const METADATA_JSON: &str = include_str!("fixtures/bundle-3/metadata.json");

#[test]
fn stale_source_resurrection_keeps_wrong_subject_claim_below_boundary() {
    assert_bundle_metadata();

    let config = TrustConfig {
        weights: TrustFactorWeights {
            freshness_weight: 1.0,
            user_feedback_weight: 1.0,
            ..zero_weights()
        },
        ..TrustConfig::default()
    };
    let factor_inputs = TrustFactorInputs {
        source_reliability: 1.0,
        freshness: FreshnessContext {
            timestamp_known: true,
            age_days: 180.0,
        },
        corroboration_strength: 1.0,
        contradiction_count: 0,
        user_feedback: UserFeedbackSignal::WrongSubject,
        subject_fit_confidence: 1.0,
    };

    let stale_freshness = freshness_weight(&factor_inputs.freshness, &TemporalScope::State, &config);
    let feedback_penalty = user_feedback_weight(&factor_inputs, &config);
    assert!(stale_freshness < 0.3, "stale freshness was {stale_freshness}");
    assert_close(feedback_penalty, config.feedback_penalty);

    let computation = compile_trust(
        &test_claim(),
        TrustContext {
            now: Utc.with_ymd_and_hms(2026, 5, 4, 12, 0, 0).unwrap(),
            config,
            factor_inputs,
            cross_entity: clean_cross_entity_input(),
            target_surface: None,
        },
    )
    .expect("compile trust");

    assert!(factor_raw(&computation, "freshness_weight") < 0.3);
    assert_close(factor_raw(&computation, "user_feedback_weight"), feedback_penalty);
    assert!(
        computation.score.value() < 0.4,
        "resurrected claim should stay below 0.4, got {}",
        computation.score.value()
    );
    assert_eq!(computation.band, TrustBand::NeedsVerification);
}

fn assert_bundle_metadata() {
    let metadata: Value = serde_json::from_str(METADATA_JSON).expect("parse metadata");
    let factors = metadata["trust_factors_dominant"].as_array().expect("dominant factors")
        .iter().map(|value| value.as_str().expect("factor string")).collect::<Vec<_>>();
    assert_eq!(factors.as_slice(), ["user_feedback_weight", "freshness_weight"]);
    assert!(metadata["pass_fail_definition"].as_str().expect("pass/fail definition")
        .contains("trust_score < 0.4"));
}

fn clean_cross_entity_input() -> CrossEntityCoherenceInput {
    CrossEntityCoherenceInput {
        claim_text: "Dismissed source content is returned by the provider.".to_string(),
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
        id: "claim-bundle-3-resurrected".to_string(),
        subject_ref: r#"{"kind":"account","id":"acct-test-1"}"#.to_string(),
        claim_type: "stakeholder_insight".to_string(),
        field_path: Some("stakeholderInsights".to_string()),
        topic_key: Some("acct-test-1:withdrawn-source-topic".to_string()),
        text: "Previously dismissed source content was resurrected.".to_string(),
        dedup_key: "acct-test-1|stakeholder_insight|withdrawn-source-topic".to_string(),
        item_hash: Some("sha256:bundle3-resurrected".to_string()),
        actor: "agent:fixture".to_string(),
        data_source: "seeded-source-withdrawn".to_string(),
        source_ref: Some("seeded-source-withdrawn".to_string()),
        source_asof: Some("2025-11-05T12:00:00Z".to_string()),
        observed_at: "2026-05-04T12:00:00Z".to_string(),
        created_at: "2026-05-04T12:00:00Z".to_string(),
        provenance_json: "{}".to_string(),
        metadata_json: Some(r#"{"source_lifecycle_state":"withdrawn"}"#.to_string()),
        claim_state: ClaimState::Active,
        surfacing_state: SurfacingState::Active,
        demotion_reason: None, reactivated_at: None, retraction_reason: None,
        expires_at: None, superseded_by: None, trust_score: None,
        trust_computed_at: None, trust_version: None, thread_id: None,
        temporal_scope: TemporalScope::State,
        sensitivity: ClaimSensitivity::Internal,
        verification_state: ClaimVerificationState::Active,
        verification_reason: None, needs_user_decision_at: None,
    }
}

fn zero_weights() -> TrustFactorWeights {
    TrustFactorWeights {
        source_reliability: 0.0, freshness_weight: 0.0, corroboration_weight: 0.0,
        contradiction_penalty: 0.0, user_feedback_weight: 0.0, subject_fit_confidence: 0.0,
        cross_entity_coherence: 0.0, sensitivity_aware_filtering: 0.0,
    }
}

fn factor_raw(computation: &TrustComputation, name: &str) -> f64 {
    computation.evidence.factor_breakdown.iter()
        .find(|factor| factor.name == name)
        .unwrap_or_else(|| panic!("missing factor {name}"))
        .raw_value
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-12,
        "expected {expected}, got {actual}"
    );
}
