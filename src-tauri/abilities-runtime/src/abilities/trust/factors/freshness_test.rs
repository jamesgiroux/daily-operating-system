use chrono::{DateTime, Duration, TimeZone, Utc};

use crate::abilities::provenance::{DataSource, SourceName};
use crate::abilities::trust::config::FACTOR_MAX;
use crate::abilities::trust::types::FreshnessContext;
use crate::abilities::trust::RenewalContext;
use crate::abilities::trust::TrustConfig;
use crate::sensitivity::ClaimVerificationState;
use crate::types::{
    ClaimSensitivity, ClaimState, IntelligenceClaim, SurfacingState, TemporalScope,
};

use super::{freshness_factor_input_for_claim, freshness_weight, FreshnessFactorInput};

fn at() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 5, 10, 23, 30, 0).unwrap()
}

fn claim_with_source(source: &str, created_at: DateTime<Utc>) -> IntelligenceClaim {
    IntelligenceClaim {
        id: "claim-1".to_string(),
        claim_version: 1,
        subject_ref: r#"{"kind":"account","id":"acct-1"}"#.to_string(),
        claim_type: "risk".to_string(),
        field_path: None,
        topic_key: None,
        text: "Customer health is current.".to_string(),
        dedup_key: "dedup-1".to_string(),
        item_hash: Some("hash-1".to_string()),
        actor: "agent:test".to_string(),
        data_source: source.to_string(),
        source_ref: None,
        source_asof: Some(created_at.to_rfc3339()),
        observed_at: created_at.to_rfc3339(),
        created_at: created_at.to_rfc3339(),
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

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-12,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn factor_input_preserves_explicit_days_to_renewal() {
    let claim = claim_with_source("renewal_notes", at());
    let freshness = FreshnessContext {
        age_days: 0.0,
        timestamp_known: true,
    };
    let renewal_context = RenewalContext {
        renewal_at: Some(at() + Duration::days(90)),
        days_to_renewal: Some(30),
    };

    let input = freshness_factor_input_for_claim(&claim, &freshness, Some(&renewal_context), at());

    assert_eq!(
        input
            .renewal_context
            .and_then(|context| context.days_to_renewal),
        Some(30)
    );
}

#[test]
fn factor_input_derives_days_to_renewal_with_date_naive_math() {
    let claim = claim_with_source("renewal_notes", at());
    let freshness = FreshnessContext {
        age_days: 0.0,
        timestamp_known: true,
    };
    let renewal_context = RenewalContext {
        renewal_at: Some(Utc.with_ymd_and_hms(2026, 5, 12, 0, 30, 0).unwrap()),
        days_to_renewal: None,
    };

    let input = freshness_factor_input_for_claim(&claim, &freshness, Some(&renewal_context), at());

    assert_eq!(
        input
            .renewal_context
            .and_then(|context| context.days_to_renewal),
        Some(2)
    );
}

#[test]
fn factor_freshness_unknown_timestamp_still_decays_by_age() {
    let config = TrustConfig::default();
    let input = FreshnessFactorInput {
        data_source: DataSource::Other(SourceName::new("email")),
        age_days: 365.0,
        temporal_scope: TemporalScope::State,
        timestamp_known: false,
        renewal_context: None,
    };

    let expected = 2.0_f64.powf(-input.age_days / 14.0) * config.unknown_timestamp_penalty;
    let actual = freshness_weight(&input, &config);

    assert_close(actual, expected);
    assert!(
        (actual - config.unknown_timestamp_penalty).abs() > 0.1,
        "unknown timestamp must not discard age decay"
    );
}

#[test]
fn factor_freshness_fixed_scope_unknown_timestamp_applies_penalty() {
    let config = TrustConfig::default();
    let input = FreshnessFactorInput {
        data_source: DataSource::Other(SourceName::new("email")),
        age_days: 365.0,
        temporal_scope: TemporalScope::PointInTime,
        timestamp_known: false,
        renewal_context: None,
    };

    assert_close(
        freshness_weight(&input, &config),
        FACTOR_MAX * config.unknown_timestamp_penalty,
    );
}
