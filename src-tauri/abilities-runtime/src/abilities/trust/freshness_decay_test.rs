use chrono::{TimeZone, Utc};

use super::*;
use crate::abilities::provenance::SourceName;
use crate::sensitivity::ClaimVerificationState;
use crate::services::context::FixedClock;
use crate::types::{ClaimSensitivity, SurfacingState};

fn at() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 5, 10, 12, 0, 0).unwrap()
}

fn ctx(clock: &FixedClock) -> ScoringContext<'_> {
    ScoringContext {
        clock,
        renewal_context: None,
    }
}

fn ctx_with_renewal(clock: &FixedClock, days_to_renewal: Option<i64>) -> ScoringContext<'_> {
    ScoringContext {
        clock,
        renewal_context: Some(RenewalContext {
            renewal_at: days_to_renewal.map(|days| clock.now() + Duration::days(days)),
            days_to_renewal,
        }),
    }
}

fn claim_with_source(source: &str, created_at: DateTime<Utc>) -> Claim {
    Claim {
        id: "claim-1".to_string(),
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

fn assert_days(actual: Duration, expected_days: i64) {
    assert_eq!(actual, Duration::days(expected_days));
}

#[test]
fn every_signal_type_returns_configured_half_life() {
    let clock = FixedClock::new(at());
    let ctx = ctx(&clock);
    let cases = [
        ("zendesk_escalation", 7),
        ("zendesk_general", 30),
        ("salesforce_field_update", 30),
        ("salesforce_opp_notes", 60),
        ("gong_transcript", 30),
        ("gong_sentiment", 21),
        ("email", 14),
        ("slack", 14),
        ("linear_issue", 45),
        ("clay_enrichment", 90),
        ("user_correction", 365),
        ("renewal_notes_no_renewal_context", 90),
        ("default", 21),
    ];

    for (key, days) in cases {
        assert_days(
            half_life_for(&DataSource::Other(SourceName::new(key)), &ctx),
            days,
        );
    }

    let imminent = ctx_with_renewal(&clock, Some(30));
    assert_days(
        half_life_for(
            &DataSource::Other(SourceName::new("renewal_notes")),
            &imminent,
        ),
        400,
    );
}

#[test]
fn broad_data_source_variants_return_configured_half_life() {
    let clock = FixedClock::new(at());
    let ctx = ctx(&clock);
    assert_days(half_life_for(&DataSource::User, &ctx), 365);
    assert_days(half_life_for(&DataSource::Google, &ctx), 14);
    assert_days(
        half_life_for(
            &DataSource::Glean {
                downstream: GleanDownstream::Zendesk,
            },
            &ctx,
        ),
        30,
    );
    assert_days(
        half_life_for(
            &DataSource::Glean {
                downstream: GleanDownstream::Salesforce,
            },
            &ctx,
        ),
        60,
    );
    assert_days(
        half_life_for(
            &DataSource::Glean {
                downstream: GleanDownstream::Gong,
            },
            &ctx,
        ),
        30,
    );
    assert_days(
        half_life_for(
            &DataSource::Glean {
                downstream: GleanDownstream::Slack,
            },
            &ctx,
        ),
        14,
    );
    assert_days(half_life_for(&DataSource::Clay, &ctx), 90);
}

#[test]
fn renewal_window_branch_extends_renewal_note_freshness() {
    let clock = FixedClock::new(at());
    let mut claim = claim_with_source("manual", at() - Duration::days(330));
    claim.claim_type = "renewal_note".to_string();
    claim.field_path = Some("renewal.notes".to_string());
    claim.text = "Renewal note says the buyer is aligned.".to_string();

    let imminent = ctx_with_renewal(&clock, Some(30));
    let no_renewal = ctx(&clock);

    assert!(freshness_weight(&claim, &imminent) > 0.5);
    assert!(freshness_weight(&claim, &no_renewal) < 0.1);
}

#[test]
fn renewal_context_with_only_renewal_at_uses_scoring_clock() {
    let clock = FixedClock::new(at());
    let mut claim = claim_with_source("manual", at() - Duration::days(330));
    claim.claim_type = "renewal_note".to_string();
    claim.field_path = Some("renewal.notes".to_string());
    claim.text = "Renewal note says the buyer is aligned.".to_string();
    let ctx = ScoringContext {
        clock: &clock,
        renewal_context: Some(RenewalContext {
            renewal_at: Some(clock.now() + Duration::days(30)),
            days_to_renewal: None,
        }),
    };

    assert!(freshness_weight(&claim, &ctx) > 0.5);
}

#[test]
fn default_unmapped_warns_and_uses_21_days() {
    DEFAULT_WARNING_COUNT.store(0, std::sync::atomic::Ordering::SeqCst);
    warned_default_sources().lock().clear();

    let clock = FixedClock::new(at());
    let ctx = ctx(&clock);
    assert_days(
        half_life_for(&DataSource::Other(SourceName::new("boot_validation_unmapped")), &ctx),
        21,
    );
    assert!(DEFAULT_WARNING_COUNT.load(std::sync::atomic::Ordering::SeqCst) >= 1);
}

#[test]
fn tombstone_always_returns_one() {
    let clock = FixedClock::new(at());
    let ctx = ctx(&clock);
    let mut claim = claim_with_source("clay", at() - Duration::days(10_000));
    claim.claim_state = ClaimState::Tombstoned;

    assert_eq!(freshness_weight(&claim, &ctx), 1.0);
}

#[test]
fn future_dated_created_at_clamps_to_one() {
    let clock = FixedClock::new(at());
    let ctx = ctx(&clock);
    let claim = claim_with_source("email", at() + Duration::days(1));

    assert_eq!(freshness_weight(&claim, &ctx), 1.0);
}

#[test]
fn floor_holds_for_any_age() {
    let clock = FixedClock::new(at());
    let ctx = ctx(&clock);
    let claim = claim_with_source("email", at() - Duration::days(20_000));

    assert_eq!(freshness_weight(&claim, &ctx), freshness_config().policy.floor);
}

#[test]
fn clock_is_injected_from_scoring_context() {
    let created_at = at();
    let fresh_clock = FixedClock::new(created_at);
    let stale_clock = FixedClock::new(created_at + Duration::days(14));
    let claim = claim_with_source("email", created_at);

    assert_eq!(freshness_weight(&claim, &ctx(&fresh_clock)), 1.0);
    assert!(freshness_weight(&claim, &ctx(&stale_clock)) < 1.0);
}

#[test]
fn salesforce_field_update_uses_step_threshold() {
    let clock = FixedClock::new(at());
    let ctx = ctx(&clock);
    let mut fresh = claim_with_source("salesforce", at() - Duration::days(30));
    fresh.field_path = Some("account.health".to_string());
    let mut stale = fresh.clone();
    stale.source_asof = Some((at() - Duration::days(31)).to_rfc3339());
    stale.observed_at = (at() - Duration::days(31)).to_rfc3339();
    stale.created_at = (at() - Duration::days(31)).to_rfc3339();

    assert_eq!(freshness_weight(&fresh, &ctx), 1.0);
    assert_eq!(
        freshness_weight(&stale, &ctx),
        freshness_config()
            .policy
            .salesforce_field_update_stale_weight
    );
}
