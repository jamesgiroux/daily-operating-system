//! Shared Glean finalization producer.
//!
//! Queue and manual refresh both land parsed Glean intelligence. This service
//! owns the Glean-derived substrate side effects so the trigger path does not
//! change what reaches claims, signals, health, and runtime surfaces.

use std::collections::{BTreeMap, HashSet};

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::db::types::AccountSourceRef;
use crate::db::ActionDb;
use crate::intelligence::io::{IntelligenceJson, OrgHealthData, SupportHealth};
use crate::presets::schema::RolePreset;
use crate::services::context::ServiceContext;
use crate::signals::propagation::PropagationEngine;

const UNKNOWN_SOURCE_OBSERVED_AT: &str = "1970-01-01T00:00:00Z";
const DEGRADED_SIGNAL_TYPE: &str = "glean_finalization_degraded";
const DEGRADED_SIGNAL_SOURCE: &str = "glean_synthesis";

#[derive(Debug, Clone)]
pub struct GleanFinalizationInput<'a> {
    pub entity_type: &'a str,
    pub entity_id: &'a str,
    pub intel: &'a IntelligenceJson,
    pub preset: Option<&'a RolePreset>,
}

#[derive(Debug, Clone, Default)]
pub struct GleanFinalizationReport {
    pub run_key: String,
    pub signals_attempted: usize,
    pub signals_emitted: usize,
    pub schema_promoted: u32,
    pub claims_committed: u32,
    pub recompute_jobs_enqueued: u32,
    pub health_recomputed: bool,
    pub degraded_classes: Vec<GleanFinalizationSideEffect>,
    pub warnings: Vec<GleanFinalizationWarning>,
    pub durable_degraded_marker_emitted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GleanFinalizationSideEffect {
    Signal,
    TechnicalFootprint,
    AccountFact,
    TrustRecompute,
    HealthRecompute,
    DegradedMarker,
}

impl GleanFinalizationSideEffect {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Signal => "signal",
            Self::TechnicalFootprint => "technical_footprint",
            Self::AccountFact => "account_fact",
            Self::TrustRecompute => "trust_recompute",
            Self::HealthRecompute => "health_recompute",
            Self::DegradedMarker => "degraded_marker",
        }
    }
}

#[derive(Debug, Clone)]
pub struct GleanFinalizationWarning {
    pub code: &'static str,
    pub side_effect: GleanFinalizationSideEffect,
    pub signal_type: Option<&'static str>,
    pub field: Option<&'static str>,
    pub source: Option<&'static str>,
    pub entity_type: String,
    pub entity_id: String,
    pub count: Option<usize>,
    pub pii_safe_detail: Option<&'static str>,
}

#[derive(Debug, Clone)]
pub struct GleanFinalizationError {
    message: String,
}

impl std::fmt::Display for GleanFinalizationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for GleanFinalizationError {}

impl GleanFinalizationError {
    fn mutation_not_allowed(error: impl std::fmt::Display) -> Self {
        Self {
            message: format!("glean finalization mutation not allowed: {error}"),
        }
    }

    fn durable_marker_failed(entity_type: &str, entity_id: &str) -> Self {
        Self {
            message: format!(
                "glean finalization degraded without durable marker for {entity_type}:{entity_id}"
            ),
        }
    }
}

pub fn finalize_glean_enrichment(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    engine: &PropagationEngine,
    input: GleanFinalizationInput<'_>,
) -> Result<GleanFinalizationReport, GleanFinalizationError> {
    ctx.check_mutation_allowed()
        .map_err(GleanFinalizationError::mutation_not_allowed)?;

    let mut report = GleanFinalizationReport {
        run_key: run_key(input.entity_type, input.entity_id, input.intel),
        ..GleanFinalizationReport::default()
    };

    promote_account_facts(ctx, db, &input, &mut report);
    emit_glean_signals(ctx, db, engine, &input, &mut report);
    recompute_account_health(ctx, db, &input, &mut report);
    if report.degraded_classes.is_empty() {
        clear_recovered_degraded_marker(db, &input, &mut report);
    }
    if !report.degraded_classes.is_empty() {
        emit_degraded_marker(ctx, db, engine, &input, &mut report);
        if !report.durable_degraded_marker_emitted {
            return Err(GleanFinalizationError::durable_marker_failed(
                input.entity_type,
                input.entity_id,
            ));
        }
    }

    Ok(report)
}

fn emit_glean_signals(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    engine: &PropagationEngine,
    input: &GleanFinalizationInput<'_>,
    report: &mut GleanFinalizationReport,
) {
    let mut slack_context = SlackContextSummary::default();

    if let Some(ref org) = input.intel.org_health {
        let raw_value = serde_json::to_value(org).unwrap_or(Value::Null);
        let value = pii_safe_signal_payload(
            "renewal_data_updated",
            "glean_crm",
            &raw_value,
            1,
            &org_health_fields(org),
        );
        emit_once_and_propagate(
            ctx,
            db,
            engine,
            input,
            report,
            "renewal_data_updated",
            "glean_crm",
            Some(&value),
            0.9,
        );
    }

    if let Some(ref support) = input.intel.support_health {
        let raw_value = serde_json::to_value(support).unwrap_or(Value::Null);
        let value = pii_safe_signal_payload(
            "support_health_updated",
            "glean_zendesk",
            &raw_value,
            1,
            &support_health_fields(support),
        );
        emit_once_and_propagate(
            ctx,
            db,
            engine,
            input,
            report,
            "support_health_updated",
            "glean_zendesk",
            Some(&value),
            0.85,
        );
    }

    write_technical_footprint(ctx, db, input, report);

    if !input.intel.competitive_context.is_empty() {
        let raw_value =
            serde_json::to_value(&input.intel.competitive_context).unwrap_or(Value::Null);
        let value = pii_safe_signal_payload(
            "competitor_mentioned",
            "glean_chat",
            &raw_value,
            input.intel.competitive_context.len(),
            &[
                "competitor",
                "threat_level",
                "context",
                "source",
                "detected_at",
                "item_source",
                "discrepancy",
            ],
        );
        emit_once(
            ctx,
            db,
            input,
            report,
            "competitor_mentioned",
            "glean_chat",
            Some(&value),
            0.7,
        );
        for item in input.intel.competitive_context.iter().filter(|item| {
            source_mentions_slack(item.source.as_deref())
                || item
                    .item_source
                    .as_ref()
                    .is_some_and(|source| source.source == "glean_slack")
        }) {
            slack_context.push("competitive", &item.competitor);
        }
    }

    if !input.intel.organizational_changes.is_empty() {
        let raw_value =
            serde_json::to_value(&input.intel.organizational_changes).unwrap_or(Value::Null);
        let value = pii_safe_signal_payload(
            "glean_org_change",
            "glean_chat",
            &raw_value,
            input.intel.organizational_changes.len(),
            &[
                "change_type",
                "person",
                "from",
                "to",
                "detected_at",
                "source",
                "item_source",
                "discrepancy",
            ],
        );
        emit_once_and_propagate(
            ctx,
            db,
            engine,
            input,
            report,
            "glean_org_change",
            "glean_chat",
            Some(&value),
            0.8,
        );
        for item in input.intel.organizational_changes.iter().filter(|item| {
            source_mentions_slack(item.source.as_deref())
                || item
                    .item_source
                    .as_ref()
                    .is_some_and(|source| source.source == "glean_slack")
        }) {
            slack_context.push("org_change", &item.person);
        }
    }

    if !input.intel.gong_call_summaries.is_empty() {
        let raw_value =
            serde_json::to_value(&input.intel.gong_call_summaries).unwrap_or(Value::Null);
        let value = pii_safe_signal_payload(
            "gong_engagement_updated",
            "glean_gong",
            &raw_value,
            input.intel.gong_call_summaries.len(),
            &["title", "date", "participants", "key_topics", "sentiment"],
        );
        emit_once_and_propagate(
            ctx,
            db,
            engine,
            input,
            report,
            "gong_engagement_updated",
            "glean_gong",
            Some(&value),
            0.8,
        );
    }

    for item in input.intel.risks.iter().filter(|item| {
        source_mentions_slack(item.source.as_deref())
            || item
                .item_source
                .as_ref()
                .is_some_and(|source| source.source == "glean_slack")
    }) {
        slack_context.push("risk", &item.text);
    }
    for item in input.intel.recent_wins.iter().filter(|item| {
        source_mentions_slack(item.source.as_deref())
            || item
                .item_source
                .as_ref()
                .is_some_and(|source| source.source == "glean_slack")
    }) {
        slack_context.push("win", &item.text);
    }
    for item in input.intel.stakeholder_insights.iter().filter(|item| {
        source_mentions_slack(item.source.as_deref())
            || item
                .item_source
                .as_ref()
                .is_some_and(|source| source.source == "glean_slack")
    }) {
        slack_context.push("stakeholder", &item.name);
    }
    if let Some(open_commitments) = input.intel.open_commitments.as_ref() {
        for item in open_commitments.iter().filter(|item| {
            source_mentions_slack(item.source.as_deref())
                || item
                    .item_source
                    .as_ref()
                    .is_some_and(|source| source.source == "glean_slack")
        }) {
            slack_context.push("commitment", &item.description);
        }
    }
    for item in input.intel.expansion_signals.iter().filter(|item| {
        source_mentions_slack(item.source.as_deref())
            || item
                .item_source
                .as_ref()
                .is_some_and(|source| source.source == "glean_slack")
    }) {
        slack_context.push("expansion", &item.opportunity);
    }

    if !slack_context.is_empty() {
        let payload = serde_json::json!({
            "itemHashes": slack_context.item_hashes,
            "categories": slack_context.categories,
            "count": slack_context.count,
        })
        .to_string();
        emit_once_and_propagate(
            ctx,
            db,
            engine,
            input,
            report,
            "slack_context_updated",
            "glean_slack",
            Some(&payload),
            0.5,
        );
    }

    if let Some(ref health) = input.intel.health {
        let dims = &health.dimensions;
        if dims.key_advocate_health.score < 40.0 && dims.key_advocate_health.weight > 0.0 {
            let raw_value = serde_json::json!({
                "score": dims.key_advocate_health.score,
                "evidence": dims.key_advocate_health.evidence,
            });
            let payload = pii_safe_signal_payload(
                "glean_champion_departed",
                "glean_chat",
                &raw_value,
                dims.key_advocate_health.evidence.len(),
                &["key_advocate_health"],
            );
            emit_once_and_propagate(
                ctx,
                db,
                engine,
                input,
                report,
                "glean_champion_departed",
                "glean_chat",
                Some(&payload),
                0.8,
            );
        }
    }
}

fn write_technical_footprint(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    input: &GleanFinalizationInput<'_>,
    report: &mut GleanFinalizationReport,
) {
    if input.entity_type != "account" {
        return;
    }

    let raw_support_tier = input
        .intel
        .org_health
        .as_ref()
        .and_then(|oh| oh.support_tier.clone());
    let support_health_data = input.intel.support_health.as_ref();
    let has_footprint_data = raw_support_tier.is_some() || support_health_data.is_some();
    if !has_footprint_data {
        return;
    }

    let support_tier_tombstoned = if raw_support_tier.is_some() {
        match crate::services::account_fact_claims::has_active_account_fact_tombstone(
            db,
            input.entity_id,
            "support_tier",
        ) {
            Ok(tombstoned) => tombstoned,
            Err(error) => {
                report.warn(GleanFinalizationWarning {
                    code: "technical_footprint_tombstone_check_failed",
                    side_effect: GleanFinalizationSideEffect::TechnicalFootprint,
                    signal_type: None,
                    field: Some("technical_footprint.support_tier"),
                    source: Some("glean_zendesk"),
                    entity_type: input.entity_type.to_string(),
                    entity_id: input.entity_id.to_string(),
                    count: None,
                    pii_safe_detail: Some("support tier tombstone check failed"),
                });
                log::warn!(
                    "glean finalization: support tier tombstone check failed for {}: {}",
                    input.entity_id,
                    error
                );
                true
            }
        }
    } else {
        false
    };
    let support_tier = if support_tier_tombstoned {
        None
    } else {
        raw_support_tier
    };

    let csat = support_health_data.and_then(|sh| sh.csat);
    let open_tickets = support_health_data
        .and_then(|sh| sh.open_tickets)
        .map(|count| count as i64);
    let has_footprint_data = support_tier.is_some() || csat.is_some() || open_tickets.is_some();
    if !has_footprint_data && !support_tier_tombstoned {
        return;
    }
    let observed_at = technical_footprint_observed_at(ctx, input);
    let before = db
        .get_account_technical_footprint(input.entity_id)
        .ok()
        .flatten();
    let projected_fields = projected_technical_footprint_fields(
        before.as_ref(),
        support_tier.as_deref(),
        csat,
        open_tickets,
    );

    match db.with_transaction(|tx| {
        if support_tier_tombstoned {
            clear_glean_owned_technical_footprint_support_tier(tx, input.entity_id)?;
        }
        if has_footprint_data {
            write_technical_footprint_source_refs(
                tx,
                input,
                TechnicalFootprintSourceRefInput {
                    observed_at: &observed_at,
                    projected_fields,
                    support_tier: support_tier.as_deref(),
                    csat_score: csat,
                    open_tickets,
                },
            )?;
            tx.upsert_account_support_technical_footprint(
                input.entity_id,
                support_tier.as_deref(),
                csat,
                open_tickets,
                "glean_zendesk",
                &observed_at,
            )
            .map_err(|error| format!("technical footprint write failed: {error}"))?;
        }
        Ok(())
    }) {
        Ok(()) => {
            if !has_footprint_data {
                return;
            }
            let raw_value = serde_json::json!({
                "supportTier": support_tier,
                "csat": csat,
                "openTickets": open_tickets,
            });
            let fields =
                technical_footprint_signal_fields(support_tier.as_deref(), csat, open_tickets);
            let value = pii_safe_signal_payload(
                "technical_footprint_updated",
                "glean_zendesk",
                &raw_value,
                fields.len(),
                &fields,
            );
            emit_once(
                ctx,
                db,
                input,
                report,
                "technical_footprint_updated",
                "glean_zendesk",
                Some(&value),
                0.85,
            );
        }
        Err(error) => {
            log::warn!(
                "glean finalization: technical footprint write failed for {}: {}",
                input.entity_id,
                error
            );
            report.warn(GleanFinalizationWarning {
                code: "technical_footprint_write_failed",
                side_effect: GleanFinalizationSideEffect::TechnicalFootprint,
                signal_type: None,
                field: Some("technical_footprint"),
                source: Some("glean_zendesk"),
                entity_type: input.entity_type.to_string(),
                entity_id: input.entity_id.to_string(),
                count: None,
                pii_safe_detail: Some("technical footprint write failed"),
            });
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct TechnicalFootprintProjectedFields {
    support_tier: bool,
    csat_score: bool,
    open_tickets: bool,
}

#[derive(Debug, Clone, Copy)]
struct TechnicalFootprintSourceRefInput<'a> {
    observed_at: &'a str,
    projected_fields: TechnicalFootprintProjectedFields,
    support_tier: Option<&'a str>,
    csat_score: Option<f64>,
    open_tickets: Option<i64>,
}

fn projected_technical_footprint_fields(
    before: Option<&crate::db::types::DbAccountTechnicalFootprint>,
    support_tier: Option<&str>,
    csat_score: Option<f64>,
    open_tickets: Option<i64>,
) -> TechnicalFootprintProjectedFields {
    let before_is_glean_owned = before
        .map(|footprint| technical_footprint_source_is_glean_owned(&footprint.source))
        .unwrap_or(false);
    TechnicalFootprintProjectedFields {
        support_tier: support_tier.is_some()
            && (before.is_none()
                || before_is_glean_owned
                || before
                    .and_then(|footprint| footprint.support_tier.as_deref())
                    .is_none()),
        csat_score: csat_score.is_some()
            && (before.is_none()
                || before_is_glean_owned
                || before.and_then(|footprint| footprint.csat_score).is_none()),
        open_tickets: open_tickets.is_some() && (before.is_none() || before_is_glean_owned),
    }
}

fn technical_footprint_source_is_glean_owned(source: &str) -> bool {
    let normalized = source.trim().to_ascii_lowercase();
    normalized.starts_with("glean") || normalized == "source_purged:glean"
}

fn write_technical_footprint_source_refs(
    db: &ActionDb,
    input: &GleanFinalizationInput<'_>,
    source_ref_input: TechnicalFootprintSourceRefInput<'_>,
) -> Result<(), String> {
    let mut refs = Vec::new();
    if source_ref_input.projected_fields.support_tier {
        let Some(value) = source_ref_input.support_tier else {
            return Ok(());
        };
        refs.push(("technical_footprint.support_tier", value.to_string()));
    }
    if source_ref_input.projected_fields.csat_score {
        let Some(value) = source_ref_input.csat_score else {
            return Ok(());
        };
        refs.push(("technical_footprint.csat_score", value.to_string()));
    }
    if source_ref_input.projected_fields.open_tickets {
        let Some(value) = source_ref_input.open_tickets else {
            return Ok(());
        };
        refs.push(("technical_footprint.open_tickets", value.to_string()));
    }

    for (field, value) in refs {
        let reference_id =
            technical_footprint_projection_reference_id(input.entity_id, field, &value);
        db.upsert_account_source_ref(&AccountSourceRef {
            account_id: input.entity_id,
            field,
            source_system: "glean_zendesk",
            source_kind: "technical_footprint",
            source_value: Some(&value),
            observed_at: source_ref_input.observed_at,
            reference_id: Some(reference_id.as_str()),
        })
        .map_err(|error| {
            format!("technical footprint source ref write failed for {field}: {error}")
        })?;
    }
    Ok(())
}

fn clear_glean_owned_technical_footprint_support_tier(
    db: &ActionDb,
    account_id: &str,
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    db.conn_ref()
        .execute(
            "UPDATE account_technical_footprint
             SET support_tier = NULL,
                 updated_at = ?1
             WHERE account_id = ?2
               AND support_tier IS NOT NULL
               AND (
                    lower(coalesce(source, '')) LIKE 'glean%'
                    OR coalesce(source, '') = 'source_purged:glean'
                    OR EXISTS (
                        SELECT 1
                          FROM account_source_refs
                         WHERE account_source_refs.account_id = account_technical_footprint.account_id
                           AND account_source_refs.field = 'technical_footprint.support_tier'
                           AND account_source_refs.source_kind != 'source_purged'
                           AND account_source_refs.source_record_ref LIKE 'glean_technical_footprint:%'
                           AND account_source_refs.source_value = account_technical_footprint.support_tier
                    )
               )",
            rusqlite::params![now, account_id],
        )
        .map_err(|error| format!("clear tombstoned support tier projection failed: {error}"))?;
    db.conn_ref()
        .execute(
            "UPDATE account_source_refs
             SET source_system = 'superseded:tombstone',
                 source_kind = 'source_purged',
                 source_value = NULL,
                 source_record_ref = NULL
             WHERE account_id = ?1
               AND field = 'technical_footprint.support_tier'
               AND source_kind != 'source_purged'
               AND source_record_ref LIKE 'glean_technical_footprint:%'",
            rusqlite::params![account_id],
        )
        .map(|_| ())
        .map_err(|error| format!("retire tombstoned support tier source refs failed: {error}"))
}

fn technical_footprint_observed_at(
    _ctx: &ServiceContext<'_>,
    input: &GleanFinalizationInput<'_>,
) -> String {
    input
        .intel
        .org_health
        .as_ref()
        .and_then(|org| {
            let gathered_at = org.gathered_at.trim();
            (!gathered_at.is_empty()).then(|| gathered_at.to_string())
        })
        .or_else(|| {
            let enriched_at = input.intel.enriched_at.trim();
            (!enriched_at.is_empty()).then(|| enriched_at.to_string())
        })
        .unwrap_or_else(|| UNKNOWN_SOURCE_OBSERVED_AT.to_string())
}

fn promote_account_facts(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    input: &GleanFinalizationInput<'_>,
    report: &mut GleanFinalizationReport,
) {
    if input.entity_type != "account" {
        return;
    }

    let fact_report = crate::services::account_fact_claims::promote_glean_facts_from_intelligence(
        ctx,
        db,
        input.entity_id,
        input.intel,
    );

    report.schema_promoted += fact_report.schema_promoted;
    report.claims_committed += fact_report.claims_committed;
    report.recompute_jobs_enqueued += fact_report.recompute_jobs_enqueued;

    let source_ref_errors = fact_report.source_ref_errors.len();
    if source_ref_errors > 0 {
        report.warn(GleanFinalizationWarning {
            code: "account_fact_source_ref_write_failed",
            side_effect: GleanFinalizationSideEffect::AccountFact,
            signal_type: None,
            field: Some("account_source_refs"),
            source: None,
            entity_type: input.entity_type.to_string(),
            entity_id: input.entity_id.to_string(),
            count: Some(source_ref_errors),
            pii_safe_detail: Some("one or more source refs failed"),
        });
    }

    let claim_errors = fact_report.claim_errors.len();
    if claim_errors > 0 {
        report.warn(GleanFinalizationWarning {
            code: "account_fact_claim_write_failed",
            side_effect: GleanFinalizationSideEffect::AccountFact,
            signal_type: None,
            field: Some("intelligence_claims"),
            source: None,
            entity_type: input.entity_type.to_string(),
            entity_id: input.entity_id.to_string(),
            count: Some(claim_errors),
            pii_safe_detail: Some("one or more account fact claims failed"),
        });
    }

    let recompute_errors = fact_report.recompute_enqueue_errors.len();
    if recompute_errors > 0 {
        report.warn(GleanFinalizationWarning {
            code: "account_fact_recompute_enqueue_failed",
            side_effect: GleanFinalizationSideEffect::TrustRecompute,
            signal_type: None,
            field: Some("claim_recompute"),
            source: None,
            entity_type: input.entity_type.to_string(),
            entity_id: input.entity_id.to_string(),
            count: Some(recompute_errors),
            pii_safe_detail: Some("one or more trust recompute jobs failed"),
        });
    }
}

fn recompute_account_health(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    input: &GleanFinalizationInput<'_>,
    report: &mut GleanFinalizationReport,
) {
    if input.entity_type != "account" {
        return;
    }

    match crate::services::intelligence::recompute_entity_health_with_preset(
        ctx,
        db,
        input.entity_id,
        "account",
        input.preset,
    ) {
        Ok(()) => report.health_recomputed = true,
        Err(_) => report.warn(GleanFinalizationWarning {
            code: "account_health_recompute_failed",
            side_effect: GleanFinalizationSideEffect::HealthRecompute,
            signal_type: None,
            field: Some("account_health"),
            source: None,
            entity_type: input.entity_type.to_string(),
            entity_id: input.entity_id.to_string(),
            count: None,
            pii_safe_detail: Some("account health recompute failed"),
        }),
    }
}

fn emit_degraded_marker(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    engine: &PropagationEngine,
    input: &GleanFinalizationInput<'_>,
    report: &mut GleanFinalizationReport,
) {
    let payload = DegradedMarkerPayload {
        run_key: &report.run_key,
        degraded_classes: report
            .degraded_classes
            .iter()
            .map(|side_effect| side_effect.as_str())
            .collect(),
        warning_count: report.warnings.len(),
    };
    let Ok(value) = serde_json::to_string(&payload) else {
        return;
    };
    let id = degraded_marker_signal_id(&report.run_key);

    report.signals_attempted += 1;
    match crate::services::signals::emit_once_and_propagate(
        ctx,
        db,
        engine,
        &id,
        input.entity_type,
        input.entity_id,
        DEGRADED_SIGNAL_TYPE,
        DEGRADED_SIGNAL_SOURCE,
        Some(&value),
        0.5,
    ) {
        Ok((outcome, _)) => {
            if !outcome.coalesced {
                report.signals_emitted += 1;
            }
            report.durable_degraded_marker_emitted = true;
            if clear_degraded_markers_for_entity(db, input, Some(&id)).is_err() {
                report.warn_without_marker(GleanFinalizationWarning {
                    code: "degraded_marker_stale_clear_failed",
                    side_effect: GleanFinalizationSideEffect::DegradedMarker,
                    signal_type: Some(DEGRADED_SIGNAL_TYPE),
                    field: None,
                    source: Some(DEGRADED_SIGNAL_SOURCE),
                    entity_type: input.entity_type.to_string(),
                    entity_id: input.entity_id.to_string(),
                    count: None,
                    pii_safe_detail: Some("stale degraded marker clear failed"),
                });
            }
        }
        Err(_) => {
            report.warn_without_marker(GleanFinalizationWarning {
                code: "degraded_marker_emit_failed",
                side_effect: GleanFinalizationSideEffect::DegradedMarker,
                signal_type: Some(DEGRADED_SIGNAL_TYPE),
                field: None,
                source: Some(DEGRADED_SIGNAL_SOURCE),
                entity_type: input.entity_type.to_string(),
                entity_id: input.entity_id.to_string(),
                count: None,
                pii_safe_detail: Some("degraded marker signal failed"),
            });
        }
    }
}

fn clear_recovered_degraded_marker(
    db: &ActionDb,
    input: &GleanFinalizationInput<'_>,
    report: &mut GleanFinalizationReport,
) {
    if clear_degraded_markers_for_entity(db, input, None).is_err() {
        report.warn_without_marker(GleanFinalizationWarning {
            code: "degraded_marker_clear_failed",
            side_effect: GleanFinalizationSideEffect::DegradedMarker,
            signal_type: Some(DEGRADED_SIGNAL_TYPE),
            field: None,
            source: Some(DEGRADED_SIGNAL_SOURCE),
            entity_type: input.entity_type.to_string(),
            entity_id: input.entity_id.to_string(),
            count: None,
            pii_safe_detail: Some("degraded marker clear failed"),
        });
    }
}

fn clear_degraded_markers_for_entity(
    db: &ActionDb,
    input: &GleanFinalizationInput<'_>,
    except_signal_id: Option<&str>,
) -> Result<usize, rusqlite::Error> {
    let mut stmt = db.conn_ref().prepare(
        "SELECT id
         FROM signal_events
         WHERE entity_type = ?1
           AND entity_id = ?2
           AND signal_type = 'glean_finalization_degraded'
           AND data_source = 'glean_synthesis'",
    )?;
    let marker_ids = stmt
        .query_map(
            rusqlite::params![input.entity_type, input.entity_id],
            |row| row.get::<_, String>(0),
        )?
        .collect::<Result<Vec<_>, _>>()?;
    drop(stmt);

    let mut deleted = 0usize;
    for id in marker_ids {
        if except_signal_id == Some(id.as_str()) {
            continue;
        }
        deleted += clear_degraded_marker_rows(db, &id)?;
    }
    Ok(deleted)
}

fn clear_degraded_marker_rows(db: &ActionDb, id: &str) -> Result<usize, rusqlite::Error> {
    let mut deleted = 0usize;
    if table_exists(db, "briefing_callouts") {
        deleted += db
            .conn_ref()
            .execute("DELETE FROM briefing_callouts WHERE signal_id = ?1", [id])?;
    }
    if table_exists(db, "signal_derivations") {
        deleted += db.conn_ref().execute(
            "DELETE FROM signal_derivations
             WHERE source_signal_id = ?1 OR derived_signal_id = ?1",
            [id],
        )?;
    }
    deleted += db.conn_ref().execute(
        "DELETE FROM signal_events
         WHERE id = ?1
           AND signal_type = 'glean_finalization_degraded'
           AND data_source = 'glean_synthesis'",
        [id],
    )?;
    Ok(deleted)
}

fn table_exists(db: &ActionDb, table: &str) -> bool {
    db.conn_ref()
        .query_row(
            "SELECT EXISTS(
                SELECT 1
                  FROM sqlite_master
                 WHERE type = 'table'
                   AND name = ?1
            )",
            [table],
            |row| row.get::<_, i64>(0),
        )
        .map(|exists| exists == 1)
        .unwrap_or(false)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DegradedMarkerPayload<'a> {
    run_key: &'a str,
    degraded_classes: Vec<&'static str>,
    warning_count: usize,
}

#[derive(Debug, Default)]
struct SlackContextSummary {
    count: usize,
    categories: BTreeMap<&'static str, usize>,
    item_hashes: Vec<String>,
}

impl SlackContextSummary {
    fn push(&mut self, category: &'static str, identity: &str) {
        self.count += 1;
        *self.categories.entry(category).or_insert(0) += 1;
        self.item_hashes
            .push(stable_id("slack-context", &[category, identity]));
    }

    fn is_empty(&self) -> bool {
        self.count == 0
    }
}

fn pii_safe_signal_payload(
    signal_type: &'static str,
    source: &'static str,
    raw_value: &Value,
    item_count: usize,
    fields: &[&'static str],
) -> String {
    let canonical = canonical_json(raw_value);
    serde_json::json!({
        "payloadHash": stable_id("glean-signal-payload", &[signal_type, source, &canonical]),
        "itemCount": item_count,
        "fields": fields,
    })
    .to_string()
}

fn org_health_fields(org: &OrgHealthData) -> Vec<&'static str> {
    let mut fields = Vec::new();
    if org.health_band.is_some() {
        fields.push("health_band");
    }
    if org.health_score.is_some() {
        fields.push("health_score");
    }
    if org.renewal_likelihood.is_some() {
        fields.push("renewal_likelihood");
    }
    if org.growth_tier.is_some() {
        fields.push("growth_tier");
    }
    if org.customer_stage.is_some() {
        fields.push("customer_stage");
    }
    if org.support_tier.is_some() {
        fields.push("support_tier");
    }
    if org.icp_fit.is_some() {
        fields.push("icp_fit");
    }
    fields
}

fn support_health_fields(support: &SupportHealth) -> Vec<&'static str> {
    let mut fields = Vec::new();
    if support.open_tickets.is_some() {
        fields.push("open_tickets");
    }
    if support.critical_tickets.is_some() {
        fields.push("critical_tickets");
    }
    if support.avg_resolution_time.is_some() {
        fields.push("avg_resolution_time");
    }
    if support.trend.is_some() {
        fields.push("trend");
    }
    if support.csat.is_some() {
        fields.push("csat");
    }
    fields
}

fn technical_footprint_signal_fields(
    support_tier: Option<&str>,
    csat: Option<f64>,
    open_tickets: Option<i64>,
) -> Vec<&'static str> {
    let mut fields = Vec::new();
    if support_tier.is_some() {
        fields.push("support_tier");
    }
    if csat.is_some() {
        fields.push("csat");
    }
    if open_tickets.is_some() {
        fields.push("open_tickets");
    }
    fields
}

#[allow(clippy::too_many_arguments)]
fn emit_once(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    input: &GleanFinalizationInput<'_>,
    report: &mut GleanFinalizationReport,
    signal_type: &'static str,
    source: &'static str,
    value: Option<&str>,
    confidence: f64,
) {
    report.signals_attempted += 1;
    let id = signal_evidence_id(
        input,
        GleanFinalizationSideEffect::Signal,
        signal_type,
        source,
        value,
    );
    match crate::services::signals::emit_once(
        ctx,
        db,
        &id,
        input.entity_type,
        input.entity_id,
        signal_type,
        source,
        value,
        confidence,
    ) {
        Ok(outcome) => {
            if !outcome.coalesced {
                report.signals_emitted += 1;
            }
        }
        Err(_) => report.warn(GleanFinalizationWarning {
            code: "signal_emit_failed",
            side_effect: GleanFinalizationSideEffect::Signal,
            signal_type: Some(signal_type),
            field: None,
            source: Some(source),
            entity_type: input.entity_type.to_string(),
            entity_id: input.entity_id.to_string(),
            count: None,
            pii_safe_detail: Some("signal emission failed"),
        }),
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_once_and_propagate(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    engine: &PropagationEngine,
    input: &GleanFinalizationInput<'_>,
    report: &mut GleanFinalizationReport,
    signal_type: &'static str,
    source: &'static str,
    value: Option<&str>,
    confidence: f64,
) {
    report.signals_attempted += 1;
    let id = signal_evidence_id(
        input,
        GleanFinalizationSideEffect::Signal,
        signal_type,
        source,
        value,
    );
    match crate::services::signals::emit_once_and_propagate(
        ctx,
        db,
        engine,
        &id,
        input.entity_type,
        input.entity_id,
        signal_type,
        source,
        value,
        confidence,
    ) {
        Ok((outcome, _)) => {
            if !outcome.coalesced {
                report.signals_emitted += 1;
            }
        }
        Err(_) => report.warn(GleanFinalizationWarning {
            code: "signal_propagation_failed",
            side_effect: GleanFinalizationSideEffect::Signal,
            signal_type: Some(signal_type),
            field: None,
            source: Some(source),
            entity_type: input.entity_type.to_string(),
            entity_id: input.entity_id.to_string(),
            count: None,
            pii_safe_detail: Some("signal propagation failed"),
        }),
    }
}

impl GleanFinalizationReport {
    fn warn(&mut self, warning: GleanFinalizationWarning) {
        self.add_degraded_class(warning.side_effect);
        log::warn!(
            "glean finalization degraded: code={} side_effect={} entity_type={} entity_id={}",
            warning.code,
            warning.side_effect.as_str(),
            warning.entity_type,
            warning.entity_id
        );
        self.warnings.push(warning);
    }

    fn warn_without_marker(&mut self, warning: GleanFinalizationWarning) {
        self.add_degraded_class(warning.side_effect);
        log::warn!(
            "glean finalization degraded: code={} side_effect={} entity_type={} entity_id={}",
            warning.code,
            warning.side_effect.as_str(),
            warning.entity_type,
            warning.entity_id
        );
        self.warnings.push(warning);
    }

    fn add_degraded_class(&mut self, side_effect: GleanFinalizationSideEffect) {
        let existing: HashSet<GleanFinalizationSideEffect> =
            self.degraded_classes.iter().copied().collect();
        if !existing.contains(&side_effect) {
            self.degraded_classes.push(side_effect);
        }
    }
}

fn source_mentions_slack(source: Option<&str>) -> bool {
    source
        .map(|value| value.to_lowercase())
        .is_some_and(|value| value.contains("slack"))
}

fn run_key(entity_type: &str, entity_id: &str, intel: &IntelligenceJson) -> String {
    let payload = canonical_json(&serde_json::json!({
        "org_health": &intel.org_health,
        "support_health": &intel.support_health,
        "competitive_context": &intel.competitive_context,
        "organizational_changes": &intel.organizational_changes,
        "gong_call_summaries": &intel.gong_call_summaries,
        "risks": &intel.risks,
        "recent_wins": &intel.recent_wins,
        "stakeholder_insights": &intel.stakeholder_insights,
        "open_commitments": &intel.open_commitments,
        "expansion_signals": &intel.expansion_signals,
        "health": &intel.health,
        "contract_context": &intel.contract_context,
        "agreement_outlook": &intel.agreement_outlook,
        "product_classification": &intel.product_classification,
    }));
    stable_id(
        "glean-finalize",
        &[entity_type, entity_id, "glean", &payload],
    )
}

fn signal_id(
    run_key: &str,
    side_effect: GleanFinalizationSideEffect,
    signal_type: &str,
    source: &str,
) -> String {
    stable_id(
        "sig-glean-finalize",
        &[run_key, side_effect.as_str(), signal_type, source],
    )
}

fn degraded_marker_signal_id(run_key: &str) -> String {
    signal_id(
        run_key,
        GleanFinalizationSideEffect::DegradedMarker,
        DEGRADED_SIGNAL_TYPE,
        DEGRADED_SIGNAL_SOURCE,
    )
}

fn signal_evidence_id(
    input: &GleanFinalizationInput<'_>,
    side_effect: GleanFinalizationSideEffect,
    signal_type: &str,
    source: &str,
    value: Option<&str>,
) -> String {
    stable_id(
        "sig-glean-finalize",
        &[
            input.entity_type,
            input.entity_id,
            "glean",
            side_effect.as_str(),
            signal_type,
            source,
            value.unwrap_or(""),
        ],
    )
}

fn technical_footprint_projection_reference_id(
    account_id: &str,
    field: &str,
    value: &str,
) -> String {
    let mut hasher = Sha256::new();
    for component in [account_id, field, value] {
        hasher.update((component.len() as u64).to_be_bytes());
        hasher.update(component.as_bytes());
    }
    let digest = hasher.finalize();
    format!(
        "glean_technical_footprint:projection:{}",
        hex::encode(&digest[..16])
    )
}

fn stable_id(prefix: &str, parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update((part.len() as u64).to_be_bytes());
        hasher.update(part.as_bytes());
    }
    format!("{prefix}-{}", hex::encode(hasher.finalize()))
}

fn canonical_json(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => serde_json::to_string(value).unwrap_or_default(),
        Value::Array(values) => {
            let values = values.iter().map(canonical_json).collect::<Vec<_>>();
            format!("[{}]", values.join(","))
        }
        Value::Object(map) => {
            let mut keys = map.keys().collect::<Vec<_>>();
            keys.sort();
            let fields = keys
                .into_iter()
                .map(|key| {
                    let key_json = serde_json::to_string(key).unwrap_or_default();
                    let value_json = canonical_json(&map[key]);
                    format!("{key_json}:{value_json}")
                })
                .collect::<Vec<_>>();
            format!("{{{}}}", fields.join(","))
        }
    }
}
