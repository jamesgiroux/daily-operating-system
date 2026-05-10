use crate::abilities::provenance::{DataSource, SourceName};
use crate::abilities::trust::config::FACTOR_MAX;
use crate::abilities::trust::TrustConfig;
use crate::types::TemporalScope;

use super::{freshness_weight, FreshnessFactorInput};

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-12,
        "expected {expected}, got {actual}"
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
