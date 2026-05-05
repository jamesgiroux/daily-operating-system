use chrono::{TimeZone, Utc};
use dailyos_lib::abilities::provenance::SubjectRef;
use dailyos_lib::abilities::trust::{factors::*, *};
use dailyos_lib::db::claims::*;
use serde_json::Value;

const METADATA_JSON: &str = include_str!("fixtures/bundle-4/metadata.json");

#[test]
fn cross_entity_person_ambiguity_scores_bleeding_account_b_context_low() {
    assert_bundle_metadata();

    let config = TrustConfig {
        weights: TrustFactorWeights {
            cross_entity_coherence: 1.0,
            ..zero_weights()
        },
        ..TrustConfig::default()
    };
    let cross_entity = bleeding_person_context_input();
    let result = cross_entity_coherence(&cross_entity, &config);

    assert_eq!(result.hits.len(), 2);
    assert!(
        result.value < 0.3,
        "cross entity coherence should score below 0.3, got {}",
        result.value
    );

    let computation = compile_trust(
        &test_claim(),
        TrustContext {
            now: Utc.with_ymd_and_hms(2026, 5, 4, 12, 0, 0).unwrap(),
            config,
            factor_inputs: clean_factor_inputs(),
            cross_entity,
            target_surface: None,
        },
    )
    .expect("compile trust");

    assert!(
        computation.score.value() < 0.3,
        "cross entity factor should dominate final score, got {}",
        computation.score.value()
    );
    assert_eq!(computation.band, TrustBand::NeedsVerification);
}

fn assert_bundle_metadata() {
    let metadata: Value = serde_json::from_str(METADATA_JSON).expect("parse metadata");
    let factors = metadata["trust_factors_dominant"].as_array().expect("dominant factors")
        .iter().map(|value| value.as_str().expect("factor string")).collect::<Vec<_>>();
    assert_eq!(factors.as_slice(), ["cross_entity_coherence", "subject_fit_confidence"]);
    assert!(metadata["pass_fail_definition"].as_str().expect("pass/fail definition")
        .contains("cross_entity_coherence factor scores below 0.3"));
}

fn bleeding_person_context_input() -> CrossEntityCoherenceInput {
    CrossEntityCoherenceInput {
        claim_text: "jane.doe@example.com is driving subsidiary.com work on the Subsidiary Launch Plan."
            .to_string(),
        target_footprint: TargetFootprint {
            subject: SubjectRef::Account("acct-test-1".to_string()),
            names: vec!["Acme Example".to_string()],
            domains: vec!["acme.example.com".to_string()],
            related_subjects: vec![SubjectRef::Person("person-jane-doe".to_string())],
            allowed_aliases: Vec::new(),
        },
        portfolio_footprints: vec![EntityFootprint {
            subject: SubjectRef::Account("acct-test-2".to_string()),
            names: vec!["Subsidiary Launch Plan".to_string()],
            domains: vec!["subsidiary.com".to_string()],
            infrastructure_ids: Vec::new(),
        }],
        cross_entity_context_expected: false,
    }
}

fn clean_factor_inputs() -> TrustFactorInputs {
    TrustFactorInputs {
        source_reliability: 1.0,
        freshness: FreshnessContext {
            timestamp_known: true,
            age_days: 0.0,
        },
        corroboration_strength: 1.0,
        contradiction_count: 0,
        user_feedback: UserFeedbackSignal::None,
        subject_fit_confidence: 1.0,
    }
}

fn test_claim() -> IntelligenceClaim {
    IntelligenceClaim {
        id: "claim-bundle-4-account-b-bleed".to_string(),
        subject_ref: r#"{"kind":"person","id":"person-jane-doe"}"#.to_string(),
        claim_type: "stakeholder_insight".to_string(),
        field_path: Some("stakeholderInsights".to_string()),
        topic_key: Some("person-jane-doe:acct-test-2-project-bleed".to_string()),
        text: "Account B project context was attached to the shared Person.".to_string(),
        dedup_key: "person-jane-doe|stakeholder_insight|acct-test-2-project-bleed".to_string(),
        item_hash: Some("sha256:bundle4-account-b-bleed".to_string()),
        actor: "agent:fixture".to_string(),
        data_source: "provider_replay".to_string(),
        source_ref: None,
        source_asof: Some("2026-05-04T12:00:00Z".to_string()),
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
