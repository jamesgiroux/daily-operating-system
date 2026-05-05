use chrono::{TimeZone, Utc};
use dailyos_lib::abilities::provenance::SubjectRef;
use dailyos_lib::abilities::trust::{factors::*, *};
use dailyos_lib::db::claims::*;
use serde_json::Value;

const METADATA_JSON: &str = include_str!("fixtures/bundle-7/metadata.json");

#[test]
fn temporal_scope_closed_keeps_freshness_fixed_at_window_end() {
    assert_bundle_metadata();

    let config = TrustConfig {
        weights: TrustFactorWeights {
            freshness_weight: 1.0,
            ..zero_weights()
        },
        ..TrustConfig::default()
    };
    let pre_closure = FreshnessContext {
        timestamp_known: true,
        age_days: 0.0,
    };
    let post_closure = FreshnessContext {
        timestamp_known: true,
        age_days: 150.0,
    };

    let state_post_closure_weight = freshness_weight(&post_closure, &TemporalScope::State, &config);
    let pre_closure_weight = freshness_weight(&pre_closure, &TemporalScope::Closed, &config);
    let post_closure_weight = freshness_weight(&post_closure, &TemporalScope::Closed, &config);

    assert_close(pre_closure_weight, 1.0);
    assert_close(post_closure_weight, 1.0);
    assert!(state_post_closure_weight < post_closure_weight);

    let computation = compile_trust(
        &test_claim(),
        TrustContext {
            now: Utc.with_ymd_and_hms(2026, 5, 4, 12, 0, 0).unwrap(),
            config,
            factor_inputs: TrustFactorInputs {
                source_reliability: 1.0,
                source_reliability_corroborators: Vec::new(),
                freshness: post_closure,
                corroboration_strength: 1.0,
                contradiction_count: 0,
                user_feedback: UserFeedbackSignal::None,
                subject_fit_confidence: 1.0,
                internal_consistency: 1.0,
                source_lifecycle: SourceLifecycleState::Active,
            },
            cross_entity: clean_cross_entity_input(),
            target_surface: None,
        },
    )
    .expect("compile trust");

    assert_close(computation.score.value(), 1.0);
    assert_eq!(computation.band, TrustBand::LikelyCurrent);
}

fn assert_bundle_metadata() {
    let metadata: Value = serde_json::from_str(METADATA_JSON).expect("parse metadata");
    let factors = metadata["trust_factors_dominant"].as_array().expect("dominant factors")
        .iter().map(|value| value.as_str().expect("factor string")).collect::<Vec<_>>();
    assert_eq!(factors.as_slice(), ["freshness_weight", "temporal_scope_validation"]);
    assert!(metadata["pass_fail_definition"].as_str().expect("pass/fail definition")
        .contains("freshness_weight"));
}

fn clean_cross_entity_input() -> CrossEntityCoherenceInput {
    CrossEntityCoherenceInput {
        claim_text: "Post-closure evidence repeats a closed-window renewal checklist.".to_string(),
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
        id: "claim-bundle-7-post-closure".to_string(),
        subject_ref: r#"{"kind":"account","id":"acct-test-1"}"#.to_string(),
        claim_type: "entity_summary".to_string(),
        field_path: Some("executiveAssessment".to_string()),
        topic_key: Some("acct-test-1:renewal-checklist:closed-window".to_string()),
        text: "Post-closure observation attempts to refresh a closed-window claim.".to_string(),
        dedup_key: "acct-test-1|entity_summary|renewal-checklist|closed-window".to_string(),
        item_hash: Some("sha256:bundle7-post-closure".to_string()),
        actor: "agent:fixture".to_string(),
        data_source: "provider_completion".to_string(),
        source_ref: Some("src-test-source-stale-postclosure".to_string()),
        source_asof: Some("2026-04-01T12:00:00Z".to_string()),
        observed_at: "2026-04-01T12:00:00Z".to_string(),
        created_at: "2026-05-04T12:00:00Z".to_string(),
        provenance_json: "{}".to_string(),
        metadata_json: Some(r#"{"target_temporal_scope":"closed"}"#.to_string()),
        claim_state: ClaimState::Active,
        surfacing_state: SurfacingState::Active,
        demotion_reason: None, reactivated_at: None, retraction_reason: None,
        expires_at: None, superseded_by: None, trust_score: None,
        trust_computed_at: None, trust_version: None, thread_id: None,
        temporal_scope: TemporalScope::Closed,
        sensitivity: ClaimSensitivity::Internal,
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

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-12,
        "expected {expected}, got {actual}"
    );
}
