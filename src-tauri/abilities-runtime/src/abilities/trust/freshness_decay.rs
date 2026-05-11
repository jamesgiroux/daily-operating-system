use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use chrono::{DateTime, Duration, Utc};
use parking_lot::Mutex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::abilities::provenance::{DataSource, GleanDownstream, SourceName};
use crate::services::context::Clock;
use crate::types::{ClaimState, TemporalScope};

use super::config::{
    TrustConfig, FACTOR_MAX, FACTOR_MIN, FRESHNESS_EXPONENTIAL_BASE, FRESHNESS_FLOOR,
    SALESFORCE_FIELD_UPDATE_STALE_WEIGHT, SECONDS_PER_DAY,
};
use super::types::FreshnessContext;

pub type Claim = crate::types::IntelligenceClaim;

const CONFIG_TOML: &str =
    include_str!("../../../../src/abilities/trust/config/trust_compiler.toml");
const SALESFORCE_FIELD_UPDATE_THRESHOLD_DAYS: i64 = 30;

const ZENDESK_ESCALATION: &str = "zendesk_escalation";
const ZENDESK_GENERAL: &str = "zendesk_general";
const SALESFORCE_FIELD_UPDATE: &str = "salesforce_field_update";
const SALESFORCE_OPP_NOTES: &str = "salesforce_opp_notes";
const GONG_TRANSCRIPT: &str = "gong_transcript";
const GONG_SENTIMENT: &str = "gong_sentiment";
const EMAIL: &str = "email";
const SLACK: &str = "slack";
const GLEAN_P2: &str = "glean_p2";
const GLEAN_WORDPRESS: &str = "glean_wordpress";
const GLEAN_ORG_DIRECTORY: &str = "glean_org_directory";
const GLEAN_DOCUMENTS: &str = "glean_documents";
const GLEAN_UNKNOWN: &str = "glean_unknown";
const LINEAR_ISSUE: &str = "linear_issue";
const CLAY_ENRICHMENT: &str = "clay_enrichment";
const AI: &str = "ai";
const CO_ATTENDANCE: &str = "co_attendance";
const LOCAL_ENRICHMENT: &str = "local_enrichment";
const LEGACY_UNATTRIBUTED: &str = "legacy_unattributed";
const USER_CORRECTION: &str = "user_correction";
const RENEWAL_NOTES: &str = "renewal_notes";
const RENEWAL_NOTES_IMMINENT: &str = "renewal_notes_with_imminent_renewal";
const RENEWAL_NOTES_NO_CONTEXT: &str = "renewal_notes_no_renewal_context";
const DEFAULT: &str = "default";

const REQUIRED_CONFIG_KEYS: &[&str] = &[
    ZENDESK_ESCALATION,
    ZENDESK_GENERAL,
    SALESFORCE_FIELD_UPDATE,
    SALESFORCE_OPP_NOTES,
    GONG_TRANSCRIPT,
    GONG_SENTIMENT,
    EMAIL,
    SLACK,
    GLEAN_P2,
    GLEAN_WORDPRESS,
    GLEAN_ORG_DIRECTORY,
    GLEAN_DOCUMENTS,
    GLEAN_UNKNOWN,
    LINEAR_ISSUE,
    CLAY_ENRICHMENT,
    AI,
    CO_ATTENDANCE,
    LOCAL_ENRICHMENT,
    LEGACY_UNATTRIBUTED,
    USER_CORRECTION,
    RENEWAL_NOTES_IMMINENT,
    RENEWAL_NOTES_NO_CONTEXT,
    DEFAULT,
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RenewalContext {
    #[schemars(with = "Option<String>")]
    pub renewal_at: Option<DateTime<Utc>>,
    pub days_to_renewal: Option<i64>,
}

pub struct ScoringContext<'a> {
    pub clock: &'a dyn Clock,
    pub renewal_context: Option<RenewalContext>,
}

#[derive(Debug, Deserialize)]
struct TrustCompilerToml {
    half_life_days: BTreeMap<String, HalfLifeConfigValue>,
    freshness_policy: FreshnessPolicyToml,
}

#[derive(Debug, Deserialize)]
struct FreshnessPolicyToml {
    floor: f64,
    salesforce_field_update_stale_weight: f64,
    renewal_imminent_window_days: i64,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum HalfLifeConfigValue {
    Days(i64),
    Rule(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HalfLifeRule {
    Days(i64),
    Step30dThreshold,
}

impl HalfLifeRule {
    fn threshold_days(self) -> i64 {
        match self {
            Self::Days(days) => days,
            Self::Step30dThreshold => SALESFORCE_FIELD_UPDATE_THRESHOLD_DAYS,
        }
    }
}

#[derive(Debug)]
struct TrustFreshnessConfig {
    half_life_days: BTreeMap<String, HalfLifeRule>,
    policy: FreshnessPolicy,
}

#[derive(Debug, Clone, Copy)]
struct FreshnessPolicy {
    floor: f64,
    salesforce_field_update_stale_weight: f64,
    renewal_imminent_window_days: i64,
}

#[derive(Debug, Clone, Copy)]
struct FreshnessTimestamp {
    at: DateTime<Utc>,
    timestamp_known: bool,
}

static CONFIG: OnceLock<TrustFreshnessConfig> = OnceLock::new();
static WARNED_DEFAULT_SOURCES: OnceLock<Mutex<BTreeSet<String>>> = OnceLock::new();

#[cfg(test)]
static DEFAULT_WARNING_COUNT: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

pub fn half_life_for(source_type: &DataSource, ctx: &ScoringContext<'_>) -> Duration {
    Duration::days(rule_for_data_source(source_type, ctx).threshold_days())
}

pub fn freshness_weight(claim: &Claim, ctx: &ScoringContext<'_>) -> f64 {
    freshness_weight_at(
        claim,
        ctx.clock.now(),
        ctx.renewal_context.as_ref(),
        None,
        &TrustConfig::default(),
    )
}

pub(crate) fn freshness_weight_at(
    claim: &Claim,
    now: DateTime<Utc>,
    renewal_context: Option<&RenewalContext>,
    freshness_context: Option<&FreshnessContext>,
    trust_config: &TrustConfig,
) -> f64 {
    let (age_days, timestamp_known) =
        freshness_age_days_and_timestamp_known(claim, now, freshness_context);

    let base = if claim.claim_state == ClaimState::Tombstoned
        || matches!(
            claim.temporal_scope,
            TemporalScope::PointInTime | TemporalScope::Closed
        ) {
        FACTOR_MAX
    } else {
        let data_source = freshness_data_source_for_claim(claim);
        freshness_weight_for_data_source_at(&data_source, age_days, Some(now), renewal_context)
    };

    apply_unknown_timestamp_penalty(base, timestamp_known, trust_config)
}

fn apply_unknown_timestamp_penalty(
    base: f64,
    timestamp_known: bool,
    trust_config: &TrustConfig,
) -> f64 {
    if timestamp_known {
        base
    } else {
        base * trust_config.unknown_timestamp_penalty
    }
}

fn freshness_age_days_and_timestamp_known(
    claim: &Claim,
    now: DateTime<Utc>,
    freshness_context: Option<&FreshnessContext>,
) -> (f64, bool) {
    match freshness_timestamp_for_claim(claim) {
        Some(timestamp) => {
            let age_days =
                now.signed_duration_since(timestamp.at).num_seconds() as f64 / SECONDS_PER_DAY;
            let timestamp_known = freshness_context
                .map(|freshness| freshness.timestamp_known)
                .unwrap_or(timestamp.timestamp_known);
            (age_days.max(FACTOR_MIN), timestamp_known)
        }
        None => freshness_context
            .map(|freshness| (freshness.age_days.max(FACTOR_MIN), freshness.timestamp_known))
            .unwrap_or((FACTOR_MIN, false)),
    }
}

pub(crate) fn freshness_data_source_for_claim(claim: &Claim) -> DataSource {
    let source_key = normalize_key(&claim.data_source);
    if let Some(named_source) = refined_named_source_for_claim(&source_key, claim) {
        return named_data_source(named_source);
    }
    if rule_for_named_source(&source_key, None, None).is_some() {
        return named_data_source(source_key);
    }
    if is_renewal_note(claim) {
        return named_data_source(RENEWAL_NOTES);
    }

    data_source_for_claim(&source_key)
}

pub(crate) fn freshness_decay_for_data_source(
    data_source: &DataSource,
    age_days: f64,
    renewal_context: Option<&RenewalContext>,
) -> f64 {
    freshness_decay_for_data_source_at(data_source, age_days, None, renewal_context)
}

fn freshness_weight_for_data_source_at(
    data_source: &DataSource,
    age_days: f64,
    now: Option<DateTime<Utc>>,
    renewal_context: Option<&RenewalContext>,
) -> f64 {
    freshness_decay_for_data_source_at(data_source, age_days, now, renewal_context)
        .max(freshness_config().policy.floor)
}

pub(crate) fn freshness_decay_for_data_source_at(
    data_source: &DataSource,
    age_days: f64,
    now: Option<DateTime<Utc>>,
    renewal_context: Option<&RenewalContext>,
) -> f64 {
    let rule = rule_for_freshness_data_source(data_source, now, renewal_context);
    freshness_decay_for_rule(rule, age_days)
}

pub(crate) fn freshness_threshold_days_for_data_source(
    data_source: &DataSource,
    renewal_context: Option<&RenewalContext>,
) -> f64 {
    rule_for_freshness_data_source(data_source, None, renewal_context).threshold_days() as f64
}

#[cfg(test)]
fn freshness_threshold_days(
    claim: &Claim,
    now: DateTime<Utc>,
    renewal_context: Option<&RenewalContext>,
) -> f64 {
    let data_source = freshness_data_source_for_claim(claim);
    rule_for_freshness_data_source(&data_source, Some(now), renewal_context).threshold_days() as f64
}

pub fn validate_freshness_decay_config(ctx: &ScoringContext<'_>) {
    let _config = freshness_config();
    for source in representative_data_sources() {
        let _duration = half_life_for(&source, ctx);
    }
}

fn freshness_config() -> &'static TrustFreshnessConfig {
    CONFIG.get_or_init(|| {
        load_embedded_config()
            .unwrap_or_else(|err| panic!("invalid embedded trust freshness config: {err}"))
    })
}

fn load_embedded_config() -> Result<TrustFreshnessConfig, String> {
    let parsed: TrustCompilerToml =
        toml::from_str(CONFIG_TOML).map_err(|err| format!("toml parse failed: {err}"))?;
    let mut half_life_days = BTreeMap::new();
    let policy = parse_freshness_policy(parsed.freshness_policy)?;

    for (raw_key, raw_value) in parsed.half_life_days {
        let key = normalize_key(&raw_key);
        let rule = match raw_value {
            HalfLifeConfigValue::Days(days) if days > 0 => HalfLifeRule::Days(days),
            HalfLifeConfigValue::Days(days) => {
                return Err(format!("half_life_days.{key} must be positive, got {days}"));
            }
            HalfLifeConfigValue::Rule(rule) if rule == "step_30d_threshold" => {
                HalfLifeRule::Step30dThreshold
            }
            HalfLifeConfigValue::Rule(rule) => {
                return Err(format!("half_life_days.{key} has unknown rule {rule:?}"));
            }
        };
        half_life_days.insert(key, rule);
    }

    for required in REQUIRED_CONFIG_KEYS {
        if !half_life_days.contains_key(*required) {
            return Err(format!("missing half_life_days.{required}"));
        }
    }

    Ok(TrustFreshnessConfig {
        half_life_days,
        policy,
    })
}

fn parse_freshness_policy(raw: FreshnessPolicyToml) -> Result<FreshnessPolicy, String> {
    validate_unit_interval(
        "freshness_policy.floor",
        raw.floor,
        UnitIntervalBoundary::GreaterThanZero,
    )?;
    validate_compiled_constant("freshness_policy.floor", raw.floor, FRESHNESS_FLOOR)?;
    validate_unit_interval(
        "freshness_policy.salesforce_field_update_stale_weight",
        raw.salesforce_field_update_stale_weight,
        UnitIntervalBoundary::InclusiveZero,
    )?;
    validate_compiled_constant(
        "freshness_policy.salesforce_field_update_stale_weight",
        raw.salesforce_field_update_stale_weight,
        SALESFORCE_FIELD_UPDATE_STALE_WEIGHT,
    )?;
    if raw.renewal_imminent_window_days < 0 {
        return Err(format!(
            "freshness_policy.renewal_imminent_window_days must be non-negative, got {}",
            raw.renewal_imminent_window_days
        ));
    }

    Ok(FreshnessPolicy {
        floor: raw.floor,
        salesforce_field_update_stale_weight: raw.salesforce_field_update_stale_weight,
        renewal_imminent_window_days: raw.renewal_imminent_window_days,
    })
}

#[derive(Debug, Clone, Copy)]
enum UnitIntervalBoundary {
    InclusiveZero,
    GreaterThanZero,
}

fn validate_unit_interval(
    name: &'static str,
    value: f64,
    boundary: UnitIntervalBoundary,
) -> Result<(), String> {
    if !value.is_finite() {
        return Err(format!("{name} must be finite, got {value}"));
    }

    let lower_bound_ok = match boundary {
        UnitIntervalBoundary::InclusiveZero => value >= FACTOR_MIN,
        UnitIntervalBoundary::GreaterThanZero => value > FACTOR_MIN,
    };
    if !lower_bound_ok || value > FACTOR_MAX {
        let range = match boundary {
            UnitIntervalBoundary::InclusiveZero => "[0, 1]",
            UnitIntervalBoundary::GreaterThanZero => "(0, 1]",
        };
        return Err(format!("{name} must be in {range}, got {value}"));
    }

    Ok(())
}

fn validate_compiled_constant(name: &'static str, value: f64, expected: f64) -> Result<(), String> {
    if (value - expected).abs() > f64::EPSILON {
        return Err(format!(
            "{name} must match compiled trust constant {expected}, got {value}"
        ));
    }

    Ok(())
}

fn representative_data_sources() -> Vec<DataSource> {
    vec![
        DataSource::User,
        DataSource::Google,
        DataSource::Glean {
            downstream: GleanDownstream::Salesforce,
        },
        DataSource::Glean {
            downstream: GleanDownstream::Zendesk,
        },
        DataSource::Glean {
            downstream: GleanDownstream::Gong,
        },
        DataSource::Glean {
            downstream: GleanDownstream::Slack,
        },
        DataSource::Glean {
            downstream: GleanDownstream::P2,
        },
        DataSource::Glean {
            downstream: GleanDownstream::Wordpress,
        },
        DataSource::Glean {
            downstream: GleanDownstream::OrgDirectory,
        },
        DataSource::Glean {
            downstream: GleanDownstream::Documents,
        },
        DataSource::Glean {
            downstream: GleanDownstream::Unknown,
        },
        DataSource::Clay,
        DataSource::Ai,
        DataSource::CoAttendance,
        DataSource::LocalEnrichment,
        DataSource::Other(SourceName::new(LINEAR_ISSUE)),
        DataSource::Other(SourceName::new(RENEWAL_NOTES)),
        DataSource::LegacyUnattributed,
    ]
}

fn rule_for_data_source(source_type: &DataSource, ctx: &ScoringContext<'_>) -> HalfLifeRule {
    rule_for_freshness_data_source(
        source_type,
        Some(ctx.clock.now()),
        ctx.renewal_context.as_ref(),
    )
}

fn rule_for_freshness_data_source(
    source_type: &DataSource,
    now: Option<DateTime<Utc>>,
    renewal_context: Option<&RenewalContext>,
) -> HalfLifeRule {
    match source_type {
        DataSource::User => configured_rule(USER_CORRECTION),
        DataSource::Google => configured_rule(EMAIL),
        DataSource::Glean {
            downstream: GleanDownstream::Salesforce,
        } => configured_rule(SALESFORCE_OPP_NOTES),
        DataSource::Glean {
            downstream: GleanDownstream::Zendesk,
        } => configured_rule(ZENDESK_GENERAL),
        DataSource::Glean {
            downstream: GleanDownstream::Gong,
        } => configured_rule(GONG_TRANSCRIPT),
        DataSource::Glean {
            downstream: GleanDownstream::Slack,
        } => configured_rule(SLACK),
        DataSource::Glean {
            downstream: GleanDownstream::P2,
        } => configured_rule(GLEAN_P2),
        DataSource::Glean {
            downstream: GleanDownstream::Wordpress,
        } => configured_rule(GLEAN_WORDPRESS),
        DataSource::Glean {
            downstream: GleanDownstream::OrgDirectory,
        } => configured_rule(GLEAN_ORG_DIRECTORY),
        DataSource::Glean {
            downstream: GleanDownstream::Documents,
        } => configured_rule(GLEAN_DOCUMENTS),
        DataSource::Glean {
            downstream: GleanDownstream::Unknown,
        } => configured_rule(GLEAN_UNKNOWN),
        DataSource::Clay => configured_rule(CLAY_ENRICHMENT),
        DataSource::Other(name) => {
            let key = normalize_key(name.as_str());
            rule_for_named_source(&key, now, renewal_context)
                .unwrap_or_else(|| default_rule_for_unmapped(&format!("other:{key}")))
        }
        DataSource::Ai => configured_rule(AI),
        DataSource::CoAttendance => configured_rule(CO_ATTENDANCE),
        DataSource::LocalEnrichment => configured_rule(LOCAL_ENRICHMENT),
        DataSource::LegacyUnattributed => configured_rule(LEGACY_UNATTRIBUTED),
    }
}

fn refined_named_source_for_claim(source_key: &str, claim: &Claim) -> Option<&'static str> {
    match source_key {
        "zendesk" | "glean_zendesk" | "glean_support" | ZENDESK_GENERAL => {
            if is_zendesk_escalation(claim) {
                Some(ZENDESK_ESCALATION)
            } else {
                None
            }
        }
        "salesforce" | "sfdc" | "glean_salesforce" | "glean_crm" | SALESFORCE_OPP_NOTES => {
            if is_salesforce_field_update(claim) {
                Some(SALESFORCE_FIELD_UPDATE)
            } else {
                None
            }
        }
        "gong" | "glean_gong" | GONG_TRANSCRIPT => {
            if is_gong_sentiment(claim) {
                Some(GONG_SENTIMENT)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn freshness_decay_for_rule(rule: HalfLifeRule, age_days: f64) -> f64 {
    match rule {
        HalfLifeRule::Days(days) => {
            let half_life = days as f64;
            FRESHNESS_EXPONENTIAL_BASE.powf(-age_days / half_life)
        }
        HalfLifeRule::Step30dThreshold => {
            if age_days <= SALESFORCE_FIELD_UPDATE_THRESHOLD_DAYS as f64 {
                FACTOR_MAX
            } else {
                freshness_config().policy.salesforce_field_update_stale_weight
            }
        }
    }
}

fn named_data_source(name: impl Into<String>) -> DataSource {
    DataSource::Other(SourceName::new(name))
}

fn rule_for_named_source(
    key: &str,
    now: Option<DateTime<Utc>>,
    renewal_context: Option<&RenewalContext>,
) -> Option<HalfLifeRule> {
    match key {
        RENEWAL_NOTES | "renewal_note" => Some(renewal_notes_rule(now, renewal_context)),
        DEFAULT => Some(configured_rule(DEFAULT)),
        "gmail" | "google_email" | "email_thread" | "email_enrichment" => {
            Some(configured_rule(EMAIL))
        }
        "linear" => Some(configured_rule(LINEAR_ISSUE)),
        "clay" | "gravatar" => Some(configured_rule(CLAY_ENRICHMENT)),
        "zendesk" | "glean_zendesk" | "glean_support" => Some(configured_rule(ZENDESK_GENERAL)),
        "salesforce" | "sfdc" | "glean_salesforce" | "glean_crm" => {
            Some(configured_rule(SALESFORCE_OPP_NOTES))
        }
        "gong" | "glean_gong" | "transcript" | "notes" => Some(configured_rule(GONG_TRANSCRIPT)),
        "glean_slack" => Some(configured_rule(SLACK)),
        "p2" | "glean_p2" => Some(configured_rule(GLEAN_P2)),
        "wordpress" | "word_press" | "glean_wordpress" => Some(configured_rule(GLEAN_WORDPRESS)),
        "org_directory" | "glean_org_directory" => Some(configured_rule(GLEAN_ORG_DIRECTORY)),
        "documents" | "document" | "glean_documents" => Some(configured_rule(GLEAN_DOCUMENTS)),
        _ if freshness_config().half_life_days.contains_key(key) && key != DEFAULT => {
            Some(configured_rule(key))
        }
        _ => None,
    }
}

fn configured_rule(key: &str) -> HalfLifeRule {
    freshness_config()
        .half_life_days
        .get(key)
        .copied()
        .unwrap_or_else(|| panic!("validated freshness config missing key {key}"))
}

fn default_rule_for_unmapped(source_label: &str) -> HalfLifeRule {
    warn_unmapped_source(source_label);
    configured_rule(DEFAULT)
}

fn renewal_notes_rule(
    now: Option<DateTime<Utc>>,
    renewal_context: Option<&RenewalContext>,
) -> HalfLifeRule {
    if renewal_is_imminent(now, renewal_context) {
        configured_rule(RENEWAL_NOTES_IMMINENT)
    } else {
        configured_rule(RENEWAL_NOTES_NO_CONTEXT)
    }
}

fn renewal_is_imminent(
    now: Option<DateTime<Utc>>,
    renewal_context: Option<&RenewalContext>,
) -> bool {
    let Some(renewal_context) = renewal_context else {
        return false;
    };

    let days_to_renewal = renewal_context.days_to_renewal.or_else(|| {
        let now = now?;
        let renewal_at = renewal_context.renewal_at?;
        Some(
            renewal_at
                .date_naive()
                .signed_duration_since(now.date_naive())
                .num_days(),
        )
    });

    let renewal_window_days = freshness_config().policy.renewal_imminent_window_days;
    days_to_renewal.is_some_and(|days| (0..=renewal_window_days).contains(&days))
}

fn freshness_timestamp_for_claim(claim: &Claim) -> Option<FreshnessTimestamp> {
    if let Some(source_asof) = claim
        .source_asof
        .as_deref()
        .map(str::trim)
        .filter(|source_asof| !source_asof.is_empty())
    {
        match DateTime::parse_from_rfc3339(source_asof) {
            Ok(parsed) => {
                return Some(FreshnessTimestamp {
                    at: parsed.with_timezone(&Utc),
                    timestamp_known: true,
                });
            }
            Err(_) => warn_malformed_timestamp(claim, "source_asof", source_asof),
        }
    }

    for (field_name, raw_timestamp) in [
        ("observed_at", claim.observed_at.as_str()),
        ("created_at", claim.created_at.as_str()),
    ] {
        match DateTime::parse_from_rfc3339(raw_timestamp) {
            Ok(parsed) => {
                return Some(FreshnessTimestamp {
                    at: parsed.with_timezone(&Utc),
                    timestamp_known: false,
                });
            }
            Err(_) => warn_malformed_timestamp(claim, field_name, raw_timestamp),
        }
    }

    None
}

fn data_source_for_claim(source_key: &str) -> DataSource {
    match source_key {
        "user" | "manual" | "user_input" | "user_correction" | "user_feedback" => DataSource::User,
        "google" | "gmail" | "email" | "email_thread" | "email_enrichment" => DataSource::Google,
        "zendesk" | "glean_zendesk" | "glean_support" => DataSource::Glean {
            downstream: GleanDownstream::Zendesk,
        },
        "salesforce" | "sfdc" | "glean_salesforce" | "glean_crm" => DataSource::Glean {
            downstream: GleanDownstream::Salesforce,
        },
        "gong" | "glean_gong" | "transcript" | "notes" => DataSource::Glean {
            downstream: GleanDownstream::Gong,
        },
        "slack" | "glean_slack" => DataSource::Glean {
            downstream: GleanDownstream::Slack,
        },
        "p2" | "glean_p2" => DataSource::Glean {
            downstream: GleanDownstream::P2,
        },
        "wordpress" | "word_press" | "glean_wordpress" => DataSource::Glean {
            downstream: GleanDownstream::Wordpress,
        },
        "org_directory" | "glean_org_directory" => DataSource::Glean {
            downstream: GleanDownstream::OrgDirectory,
        },
        "documents" | "document" | "glean_documents" => DataSource::Glean {
            downstream: GleanDownstream::Documents,
        },
        "clay" | "clay_enrichment" | "gravatar" => DataSource::Clay,
        "linear" | "linear_issue" => DataSource::Other(SourceName::new(LINEAR_ISSUE)),
        "local_enrichment" => DataSource::LocalEnrichment,
        "ai" | "agent" | "intel_queue" => DataSource::Ai,
        other => DataSource::Other(SourceName::new(other)),
    }
}

fn is_renewal_note(claim: &Claim) -> bool {
    let claim_type = normalize_key(&claim.claim_type);
    if matches!(claim_type.as_str(), RENEWAL_NOTES | "renewal_note") {
        return true;
    }

    claim
        .field_path
        .as_deref()
        .map(normalize_key)
        .is_some_and(|field_path| matches!(field_path.as_str(), RENEWAL_NOTES | "renewal_note"))
}

fn is_zendesk_escalation(claim: &Claim) -> bool {
    contains_any(
        &claim_haystack(claim),
        &[
            "escalation",
            "escalated",
            "sev",
            "severity",
            "urgent",
            "p0",
            "p1",
            "priority",
        ],
    )
}

fn is_salesforce_field_update(claim: &Claim) -> bool {
    let haystack = claim_haystack(claim);
    if contains_any(
        &haystack,
        &[
            "opp_note",
            "opportunity_note",
            "opportunity notes",
            "salesforce note",
            "sf note",
            "notes",
        ],
    ) {
        return false;
    }

    claim.field_path.is_some()
        || contains_any(
            &haystack,
            &["field_update", "field update", "field_changed"],
        )
}

fn is_gong_sentiment(claim: &Claim) -> bool {
    contains_any(
        &claim_haystack(claim),
        &["sentiment", "positive", "negative", "neutral"],
    )
}

fn claim_haystack(claim: &Claim) -> String {
    let mut values = Vec::with_capacity(7);
    values.push(claim.claim_type.as_str());
    values.push(claim.data_source.as_str());
    values.push(claim.text.as_str());
    if let Some(field_path) = claim.field_path.as_deref() {
        values.push(field_path);
    }
    if let Some(topic_key) = claim.topic_key.as_deref() {
        values.push(topic_key);
    }
    if let Some(metadata_json) = claim.metadata_json.as_deref() {
        values.push(metadata_json);
    }
    if let Some(source_ref) = claim.source_ref.as_deref() {
        values.push(source_ref);
    }
    values.join(" ").to_ascii_lowercase()
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn normalize_key(value: &str) -> String {
    let mut normalized = String::with_capacity(value.len());
    let mut previous_was_separator = false;
    for ch in value.trim().chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            normalized.push(ch);
            previous_was_separator = false;
        } else if !previous_was_separator {
            normalized.push('_');
            previous_was_separator = true;
        }
    }
    normalized.trim_matches('_').to_string()
}

fn warned_default_sources() -> &'static Mutex<BTreeSet<String>> {
    WARNED_DEFAULT_SOURCES.get_or_init(|| Mutex::new(BTreeSet::new()))
}

fn warn_unmapped_source(source_label: &str) {
    let inserted = warned_default_sources()
        .lock()
        .insert(source_label.to_string());
    if inserted {
        #[cfg(test)]
        DEFAULT_WARNING_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        tracing::warn!(
            source = source_label,
            default_half_life_days = configured_rule(DEFAULT).threshold_days(),
            "Trust freshness decay: unmapped DataSource uses default half-life"
        );
    }
}

fn warn_malformed_timestamp(claim: &Claim, field_name: &'static str, timestamp: &str) {
    tracing::warn!(
        claim_id = claim.id.as_str(),
        field = field_name,
        timestamp = timestamp,
        "Trust freshness decay: malformed claim freshness timestamp"
    );
}

#[cfg(test)]
mod tests {
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

    fn assert_float_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-9,
            "expected {expected}, got {actual}"
        );
    }

    fn half_weight() -> f64 {
        FACTOR_MAX / FRESHNESS_EXPONENTIAL_BASE
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
            ("glean_p2", 21),
            ("glean_wordpress", 21),
            ("glean_org_directory", 21),
            ("glean_documents", 21),
            ("glean_unknown", 21),
            ("linear_issue", 45),
            ("clay_enrichment", 90),
            ("ai", 21),
            ("co_attendance", 21),
            ("local_enrichment", 21),
            ("legacy_unattributed", 21),
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
        assert_days(
            half_life_for(
                &DataSource::Glean {
                    downstream: GleanDownstream::P2,
                },
                &ctx,
            ),
            21,
        );
        assert_days(
            half_life_for(
                &DataSource::Glean {
                    downstream: GleanDownstream::Wordpress,
                },
                &ctx,
            ),
            21,
        );
        assert_days(
            half_life_for(
                &DataSource::Glean {
                    downstream: GleanDownstream::OrgDirectory,
                },
                &ctx,
            ),
            21,
        );
        assert_days(
            half_life_for(
                &DataSource::Glean {
                    downstream: GleanDownstream::Documents,
                },
                &ctx,
            ),
            21,
        );
        assert_days(
            half_life_for(
                &DataSource::Glean {
                    downstream: GleanDownstream::Unknown,
                },
                &ctx,
            ),
            21,
        );
        assert_days(half_life_for(&DataSource::Clay, &ctx), 90);
        assert_days(half_life_for(&DataSource::Ai, &ctx), 21);
        assert_days(half_life_for(&DataSource::CoAttendance, &ctx), 21);
        assert_days(half_life_for(&DataSource::LocalEnrichment, &ctx), 21);
        assert_days(half_life_for(&DataSource::LegacyUnattributed, &ctx), 21);
    }

    #[test]
    fn renewal_window_branch_extends_renewal_note_freshness() {
        let clock = FixedClock::new(at());
        let mut claim = claim_with_source("renewal_notes", at() - Duration::days(330));
        claim.claim_type = "renewal_note".to_string();
        claim.field_path = Some("renewal.notes".to_string());
        claim.text = "Renewal note says the buyer is aligned.".to_string();

        let imminent = ctx_with_renewal(&clock, Some(30));
        let no_renewal = ctx(&clock);

        assert!(freshness_weight(&claim, &imminent) > half_weight());
        assert!(
            freshness_weight(&claim, &no_renewal)
                < freshness_config().policy.floor * FRESHNESS_EXPONENTIAL_BASE
        );
    }

    #[test]
    fn renewal_mentions_do_not_override_source_specific_decay() {
        let cases = [
            ("email", "Customer asked about renewal timing.", None, 14_f64),
            (
                "zendesk",
                "Urgent escalation: renewal blockers are growing.",
                None,
                7_f64,
            ),
            (
                "salesforce",
                "Renewal forecast field changed.",
                Some("account.health"),
                30_f64,
            ),
            (
                "gong",
                "Discussed renewal planning on the call.",
                None,
                30_f64,
            ),
        ];

        for (source, text, field_path, expected_days) in cases {
            let mut claim = claim_with_source(source, at());
            claim.text = text.to_string();
            claim.field_path = field_path.map(str::to_string);

            assert_eq!(freshness_threshold_days(&claim, at(), None), expected_days);
        }
    }

    #[test]
    fn default_unmapped_warns_and_uses_21_days() {
        DEFAULT_WARNING_COUNT.store(0, std::sync::atomic::Ordering::SeqCst);
        warned_default_sources().lock().clear();

        let clock = FixedClock::new(at());
        let ctx = ctx(&clock);
        let claim = claim_with_source("unknown_source", at() - Duration::days(21));

        assert_float_close(freshness_weight(&claim, &ctx), half_weight());
        assert!(DEFAULT_WARNING_COUNT.load(std::sync::atomic::Ordering::SeqCst) >= 1);
    }

    #[test]
    fn tombstone_always_returns_one() {
        let clock = FixedClock::new(at());
        let ctx = ctx(&clock);
        let mut claim = claim_with_source("clay", at() - Duration::days(10_000));
        claim.claim_state = ClaimState::Tombstoned;

        assert_eq!(freshness_weight(&claim, &ctx), FACTOR_MAX);
    }

    #[test]
    fn future_dated_created_at_clamps_to_one() {
        let clock = FixedClock::new(at());
        let ctx = ctx(&clock);
        let claim = claim_with_source("email", at() + Duration::days(1));

        assert_eq!(freshness_weight(&claim, &ctx), FACTOR_MAX);
    }

    #[test]
    fn floor_holds_for_any_age() {
        let clock = FixedClock::new(at());
        let ctx = ctx(&clock);
        let claim = claim_with_source("email", at() - Duration::days(20_000));

        assert_eq!(
            freshness_weight(&claim, &ctx),
            freshness_config().policy.floor
        );
    }

    #[test]
    fn source_asof_precedes_observed_at_and_created_at() {
        let clock = FixedClock::new(at());
        let ctx = ctx(&clock);
        let mut claim = claim_with_source("email", at());
        claim.source_asof = Some((at() - Duration::days(14)).to_rfc3339());
        claim.observed_at = at().to_rfc3339();
        claim.created_at = at().to_rfc3339();

        assert_float_close(freshness_weight(&claim, &ctx), half_weight());
    }

    #[test]
    fn observed_at_precedes_created_at_and_preserves_unknown_timestamp_penalty() {
        let clock = FixedClock::new(at());
        let mut claim = claim_with_source("email", at());
        claim.source_asof = None;
        claim.observed_at = (at() - Duration::days(14)).to_rfc3339();
        claim.created_at = at().to_rfc3339();
        let freshness_context = FreshnessContext {
            timestamp_known: false,
            age_days: 14_f64,
        };

        assert_float_close(
            freshness_weight_at(
                &claim,
                clock.now(),
                None,
                Some(&freshness_context),
                &TrustConfig::default(),
            ),
            half_weight() * TrustConfig::default().unknown_timestamp_penalty,
        );
    }

    #[test]
    fn all_three_timestamps_malformed_applies_unknown_timestamp_penalty() {
        let clock = FixedClock::new(at());
        let mut claim = claim_with_source("email", at());
        claim.source_asof = Some("not-a-timestamp".to_string());
        claim.observed_at = "also-not-a-timestamp".to_string();
        claim.created_at = "still-not-a-timestamp".to_string();
        let freshness_context = FreshnessContext {
            timestamp_known: false,
            age_days: 14_f64,
        };

        assert_float_close(
            freshness_weight_at(
                &claim,
                clock.now(),
                None,
                Some(&freshness_context),
                &TrustConfig::default(),
            ),
            half_weight() * TrustConfig::default().unknown_timestamp_penalty,
        );
    }

    #[test]
    fn exhaustive_return_paths_apply_unknown_timestamp_penalty() {
        let clock = FixedClock::new(at());
        let scoring_ctx = ctx(&clock);
        let trust_config = TrustConfig::default();
        let penalty = trust_config.unknown_timestamp_penalty;
        let stale_unknown_freshness = FreshnessContext {
            timestamp_known: false,
            age_days: 14_f64,
        };
        let current_unknown_freshness = FreshnessContext {
            timestamp_known: false,
            age_days: FACTOR_MIN,
        };

        let mut malformed_with_context = claim_with_source("email", at());
        malformed_with_context.source_asof = Some("not-a-timestamp".to_string());
        malformed_with_context.observed_at = "also-not-a-timestamp".to_string();
        malformed_with_context.created_at = "still-not-a-timestamp".to_string();
        assert_float_close(
            freshness_weight_at(
                &malformed_with_context,
                clock.now(),
                None,
                Some(&stale_unknown_freshness),
                &trust_config,
            ),
            half_weight() * penalty,
        );

        let mut malformed_without_context = claim_with_source("email", at());
        malformed_without_context.source_asof = Some("not-a-timestamp".to_string());
        malformed_without_context.observed_at = "also-not-a-timestamp".to_string();
        malformed_without_context.created_at = "still-not-a-timestamp".to_string();
        assert_float_close(
            freshness_weight(&malformed_without_context, &scoring_ctx),
            penalty,
        );

        let mut observed_now = claim_with_source("email", at());
        observed_now.source_asof = None;
        observed_now.observed_at = at().to_rfc3339();
        assert_float_close(
            freshness_weight_at(
                &observed_now,
                clock.now(),
                None,
                Some(&current_unknown_freshness),
                &trust_config,
            ),
            penalty,
        );

        let mut future_created_at = claim_with_source("email", at());
        future_created_at.source_asof = None;
        future_created_at.observed_at = "not-a-timestamp".to_string();
        future_created_at.created_at = (at() + Duration::seconds(1)).to_rfc3339();
        assert_float_close(
            freshness_weight_at(
                &future_created_at,
                clock.now(),
                None,
                Some(&current_unknown_freshness),
                &trust_config,
            ),
            penalty,
        );
    }

    #[test]
    fn clock_is_injected_from_scoring_context() {
        let created_at = at();
        let fresh_clock = FixedClock::new(created_at);
        let stale_clock = FixedClock::new(created_at + Duration::days(14));
        let claim = claim_with_source("email", created_at);

        assert_eq!(freshness_weight(&claim, &ctx(&fresh_clock)), FACTOR_MAX);
        assert!(freshness_weight(&claim, &ctx(&stale_clock)) < FACTOR_MAX);
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

        assert_eq!(freshness_weight(&fresh, &ctx), FACTOR_MAX);
        assert_eq!(
            freshness_weight(&stale, &ctx),
            freshness_config()
                .policy
                .salesforce_field_update_stale_weight
        );
    }
}

#[cfg(test)]
#[path = "freshness_decay_test.rs"]
mod freshness_decay_test;
