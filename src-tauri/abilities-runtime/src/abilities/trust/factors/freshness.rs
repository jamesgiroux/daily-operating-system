use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use chrono::{DateTime, Utc};

use crate::abilities::provenance::DataSource;
use crate::types::TemporalScope;

use super::super::config::{TrustConfig, FACTOR_MAX, FACTOR_MIN};
use super::super::freshness_decay::{self, RenewalContext};
use super::super::types::FreshnessContext;

pub type Claim = crate::types::IntelligenceClaim;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FreshnessFactorInput {
    pub data_source: DataSource,
    pub age_days: f64,
    pub temporal_scope: TemporalScope,
    pub timestamp_known: bool,
    pub renewal_context: Option<RenewalContext>,
}

pub fn freshness_factor_input_for_claim(
    claim: &Claim,
    freshness: &FreshnessContext,
    renewal_context: Option<&RenewalContext>,
    now: DateTime<Utc>,
) -> FreshnessFactorInput {
    FreshnessFactorInput {
        data_source: freshness_decay::freshness_data_source_for_claim(claim),
        age_days: freshness.age_days,
        temporal_scope: claim.temporal_scope.clone(),
        timestamp_known: freshness.timestamp_known,
        renewal_context: resolved_renewal_context(renewal_context, now),
    }
}

fn resolved_renewal_context(
    renewal_context: Option<&RenewalContext>,
    now: DateTime<Utc>,
) -> Option<RenewalContext> {
    let mut renewal_context = renewal_context.cloned()?;
    if renewal_context.days_to_renewal.is_none() {
        renewal_context.days_to_renewal = renewal_context.renewal_at.map(|renewal_at| {
            renewal_at
                .date_naive()
                .signed_duration_since(now.date_naive())
                .num_days()
        });
    }
    Some(renewal_context)
}

pub fn freshness_weight(input: &FreshnessFactorInput, config: &TrustConfig) -> f64 {
    let fixed_scope = matches!(
        input.temporal_scope,
        TemporalScope::PointInTime | TemporalScope::Closed
    );
    let base_weight = if fixed_scope || input.age_days <= FACTOR_MIN {
        FACTOR_MAX
    } else {
        freshness_decay::freshness_decay_for_data_source(
            &input.data_source,
            input.age_days,
            input.renewal_context.as_ref(),
        )
    };

    if input.timestamp_known {
        base_weight
    } else {
        base_weight * config.unknown_timestamp_penalty
    }
}

pub fn freshness_threshold_days(input: &FreshnessFactorInput, _config: &TrustConfig) -> f64 {
    freshness_decay::freshness_threshold_days_for_data_source(
        &input.data_source,
        input.renewal_context.as_ref(),
    )
}

#[cfg(test)]
#[path = "freshness_test.rs"]
mod freshness_test;
