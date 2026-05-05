use chrono::{TimeZone, Utc};
use dailyos_lib::abilities::provenance::SubjectRef;
use dailyos_lib::abilities::trust::{factors::*, *};
use dailyos_lib::db::claims::*;
use serde_json::Value;

const METADATA_JSON: &str = include_str!("fixtures/bundle-6/metadata.json");

#[test]
fn corroboration_spam_does_not_let_five_weak_sources_beat_strong_contradiction() {
    assert_bundle_metadata();

    let weak_corroborators = [(0.20, 0.20); 5];
    let strong_contradiction = (1.0, 1.0);
    let weighted_support_score = weighted_source_reliability(&weak_corroborators);

    assert_eq!(weak_corroborators.len(), 5);
    assert_close(strong_contradiction.1, 1.0);
    assert!(strong_contradiction.0 > weighted_support_score);
    assert!(
        weighted_support_score < 0.5,
        "weighted score for false claim should remain below 0.5, got {weighted_support_score}"
    );

    let config = TrustConfig {
        weights: TrustFactorWeights {
            source_reliability: 1.0,
            contradiction_penalty: 1.0,
            ..zero_weights()
        },
        ..TrustConfig::default()
    };
    let factor_inputs = TrustFactorInputs {
        source_reliability: weighted_support_score,
        freshness: FreshnessContext {
            timestamp_known: true,
            age_days: 0.0,
        },
        corroboration_strength: 1.0,
        contradiction_count: 1,
        user_feedback: UserFeedbackSignal::None,
        subject_fit_confidence: 1.0,
    };

    assert_close(source_reliability(&factor_inputs), weighted_support_score);
    assert_close(
        contradiction_penalty(&factor_inputs, &config),
        1.0 - config.contradiction_multiplier,
    );

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

    let source_factor = computation.evidence.factor_breakdown.iter()
        .find(|factor| factor.name == "source_reliability").expect("source_reliability factor");
    assert_close(source_factor.raw_value, weighted_support_score);
    assert!(
        computation.score.value() < 0.5,
        "false claim should stay below 0.5 despite five corroborators, got {}",
        computation.score.value()
    );
    assert_eq!(computation.band, TrustBand::NeedsVerification);
}

fn assert_bundle_metadata() {
    let metadata: Value = serde_json::from_str(METADATA_JSON).expect("parse metadata");
    let factors = metadata["trust_factors_dominant"].as_array().expect("dominant factors")
        .iter().map(|value| value.as_str().expect("factor string")).collect::<Vec<_>>();
    assert_eq!(factors.as_slice(), ["source_reliability", "contradiction_penalty"]);
    assert!(metadata["pass_fail_definition"].as_str().expect("pass/fail definition")
        .contains("scores below 0.5 trust"));
}

fn weighted_source_reliability(sources: &[(f64, f64)]) -> f64 {
    let weighted_sum = sources.iter().map(|(reliability, weight)| reliability * weight).sum::<f64>();
    let total_weight = sources.iter().map(|(_, weight)| weight).sum::<f64>();
    weighted_sum / total_weight
}

fn clean_cross_entity_input() -> CrossEntityCoherenceInput {
    CrossEntityCoherenceInput {
        claim_text: String::new(),
        target_footprint: TargetFootprint {
            subject: SubjectRef::Account("acct-test-1".to_string()),
            names: Vec::new(),
            domains: Vec::new(),
            related_subjects: Vec::new(),
            allowed_aliases: Vec::new(),
        },
        portfolio_footprints: Vec::new(),
        cross_entity_context_expected: false,
    }
}

fn test_claim() -> IntelligenceClaim {
    IntelligenceClaim {
        id: "claim-bundle-6-false-weak-corroboration".to_string(),
        subject_ref: r#"{"kind":"account","id":"acct-test-1"}"#.to_string(),
        claim_type: "entity_summary".to_string(),
        field_path: Some("executiveAssessment".to_string()),
        topic_key: Some("acct-test-1:weak-corroboration-spam".to_string()),
        text: "Five weak sources repeat a false account claim.".to_string(),
        dedup_key: "acct-test-1|entity_summary|weak-corroboration-spam".to_string(),
        item_hash: Some("sha256:bundle6-false-weak-corroboration".to_string()),
        actor: "agent:fixture".to_string(),
        data_source: "third_party_scrape".to_string(),
        source_ref: None,
        source_asof: Some("2026-05-01T12:00:00Z".to_string()),
        observed_at: "2026-05-04T12:00:00Z".to_string(),
        created_at: "2026-05-04T12:00:00Z".to_string(),
        provenance_json: "{}".to_string(),
        metadata_json: None,
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

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-12,
        "expected {expected}, got {actual}"
    );
}
