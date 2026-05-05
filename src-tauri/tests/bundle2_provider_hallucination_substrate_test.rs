use chrono::{TimeZone, Utc};
use dailyos_lib::abilities::provenance::SubjectRef;
use dailyos_lib::abilities::trust::{factors::*, *};
use dailyos_lib::db::claims::*;
use serde_json::Value;

const METADATA_JSON: &str = include_str!("fixtures/bundle-2/metadata.json");
const INTERNAL_CONSISTENCY_PROXY: &str = "subject_fit_confidence";

#[test]
fn provider_hallucination_scores_needs_verification_on_low_source_and_consistency() {
    assert_bundle_metadata();

    let source_score = 0.40;
    let internal_consistency_score = 0.30;
    let claim = test_claim("claim-bundle-2-hallucinated");
    let ctx = test_context(source_score, internal_consistency_score);

    assert_close(source_reliability(&ctx.factor_inputs), source_score);
    assert_close(
        subject_fit_confidence(&ctx.factor_inputs),
        internal_consistency_score,
    );

    let computation = compile_trust(&claim, ctx).expect("compile trust");

    assert_factor(&computation, "source_reliability", source_score);
    assert_factor(
        &computation,
        INTERNAL_CONSISTENCY_PROXY,
        internal_consistency_score,
    );
    assert!(
        computation.score.value() < 0.5,
        "hallucinated claim should stay below NeedsVerification threshold, got {}",
        computation.score.value()
    );
    assert_eq!(computation.band, TrustBand::NeedsVerification);
}

fn assert_bundle_metadata() {
    let metadata: Value = serde_json::from_str(METADATA_JSON).expect("parse metadata");
    let factors = metadata["trust_factors_dominant"].as_array().expect("dominant factors")
        .iter().map(|value| value.as_str().expect("factor string")).collect::<Vec<_>>();
    assert_eq!(factors.as_slice(), ["source_reliability", "internal_consistency"]);
    assert!(metadata["pass_fail_definition"].as_str().expect("pass/fail definition")
        .contains("trust_score < 0.5"));
}

fn test_context(source_score: f64, internal_consistency_score: f64) -> TrustContext {
    TrustContext {
        now: Utc.with_ymd_and_hms(2026, 5, 4, 12, 0, 0).unwrap(),
        config: TrustConfig {
            weights: TrustFactorWeights {
                source_reliability: 1.0,
                subject_fit_confidence: 1.0,
                ..zero_weights()
            },
            ..TrustConfig::default()
        },
        factor_inputs: TrustFactorInputs {
            source_reliability: source_score,
            freshness: FreshnessContext {
                timestamp_known: true,
                age_days: 7.0,
            },
            corroboration_strength: 1.0,
            contradiction_count: 0,
            user_feedback: UserFeedbackSignal::None,
            subject_fit_confidence: internal_consistency_score,
        },
        cross_entity: clean_cross_entity_input(),
        target_surface: None,
    }
}

fn clean_cross_entity_input() -> CrossEntityCoherenceInput {
    CrossEntityCoherenceInput {
        claim_text: "acct-test-1 has a provider-attributed renewal claim.".to_string(),
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

fn test_claim(id: &str) -> IntelligenceClaim {
    IntelligenceClaim {
        id: id.to_string(),
        subject_ref: r#"{"kind":"account","id":"acct-test-1"}"#.to_string(),
        claim_type: "entity_summary".to_string(),
        field_path: Some("executiveAssessment".to_string()),
        topic_key: Some("acct-test-1:hallucinated-topic".to_string()),
        text: "Hallucinated provider content conflicts with fixture source truth.".to_string(),
        dedup_key: "acct-test-1|entity_summary|executiveAssessment|hallucinated".to_string(),
        item_hash: Some("sha256:bundle2-hallucinated".to_string()),
        actor: "agent:fixture".to_string(),
        data_source: "provider_replay".to_string(),
        source_ref: None,
        source_asof: Some("2026-04-27T12:00:00Z".to_string()),
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

fn assert_factor(computation: &TrustComputation, name: &str, expected: f64) {
    let factor = computation
        .evidence
        .factor_breakdown
        .iter()
        .find(|factor| factor.name == name)
        .unwrap_or_else(|| panic!("missing factor {name}"));
    assert_close(factor.raw_value, expected);
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-12,
        "expected {expected}, got {actual}"
    );
}
