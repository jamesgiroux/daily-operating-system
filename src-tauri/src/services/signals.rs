//! Signal service facade.
//!
//! Service-layer callers use these functions instead of reaching into
//! `crate::signals::bus` directly.  Infrastructure callers that only have
//! a raw `db` handle (prepare/, processor/, gravatar/) stay direct.

use parking_lot::Mutex;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::db::ActionDb;
use crate::embeddings::EmbeddingModel;
use crate::signals::bus::{self, SignalEvent};
use crate::signals::callouts::BriefingCallout;
use crate::signals::propagation::PropagationEngine;

pub const STAKEHOLDERS_CHANGED_SIGNAL: &str = "stakeholders_changed";

/// Internal stakeholder-cache invalidation signal.
///
/// This is a sync, in-transaction signal only: it is not exposed through MCP or
/// Tauri, and it does not run async propagation/evaluation. `mutation_source`
/// is required so cache rebuilds can be traced back to the mutator that changed
/// stakeholder membership.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakeholdersChangedPayload {
    pub entity_id: String,
    pub entity_type: String,
    pub mutation_source: String,
}

/// Emit a signal event (no propagation). Convenience wrapper around bus::emit_signal.
// ServiceContext adds one arg; signal facade mirrors bus shape.
#[allow(clippy::too_many_arguments)]
pub fn emit(
    ctx: &crate::services::context::ServiceContext<'_>,
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    signal_type: &str,
    source: &str,
    value: Option<&str>,
    confidence: f64,
) -> Result<String, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    bus::emit_signal(
        db,
        entity_type,
        entity_id,
        signal_type,
        source,
        value,
        confidence,
    )
    .map_err(|e| e.to_string())
}

/// Emit a signal inside the active transaction and run sync derived-state
/// subscribers for that signal type.
///
/// Errors propagate to the caller, causing `ActionDb::with_transaction` to
/// roll back the source mutation, signal row, and derived-state writes.
#[allow(clippy::too_many_arguments)]
pub fn emit_in_transaction(
    ctx: &crate::services::context::ServiceContext<'_>,
    tx: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    signal_type: &str,
    source: &str,
    payload: Value,
) -> Result<String, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let outcome =
        bus::emit_signal_in_active_tx(tx, entity_type, entity_id, signal_type, source, &payload)
            .map_err(|e| e.to_string())?;

    if outcome.coalesced {
        return Ok(outcome.id);
    }

    for subscriber in crate::signals::derived_state_subscribers::registry() {
        if subscriber.signal_type() == signal_type {
            subscriber.apply(ctx, tx, &payload)?;
        }
    }

    Ok(outcome.id)
}

/// Emit a signal and run cross-entity propagation rules.
// ServiceContext adds one arg; signal facade mirrors bus shape.
#[allow(clippy::too_many_arguments)]
pub fn emit_and_propagate(
    ctx: &crate::services::context::ServiceContext<'_>,
    db: &ActionDb,
    engine: &PropagationEngine,
    entity_type: &str,
    entity_id: &str,
    signal_type: &str,
    source: &str,
    value: Option<&str>,
    confidence: f64,
) -> Result<(String, Vec<String>), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let result = bus::emit_signal_and_propagate(
        db,
        engine,
        entity_type,
        entity_id,
        signal_type,
        source,
        value,
        confidence,
    )
    .map_err(|e| e.to_string())?;

    run_temporal_maintenance_for_signal(
        ctx,
        db,
        TemporalSignalMaintenanceInput {
            entity_type,
            entity_id,
            signal_type,
            source,
            signal_id: &result.0,
            value,
        },
    );
    Ok(result)
}

/// Best-effort wrapper around `emit` that warn-logs on failure rather than
/// dropping the Result silently. Use from service-layer call sites where the
/// emit is a side effect that should NOT fail the parent operation but where
/// a silent drop would lose downstream propagation history. Replaces the
/// `crate::services::signals::emit_or_log(...)` pattern.
#[allow(clippy::too_many_arguments)]
pub fn emit_or_log(
    ctx: &crate::services::context::ServiceContext<'_>,
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    signal_type: &str,
    source: &str,
    value: Option<&str>,
    confidence: f64,
) {
    if let Err(e) = emit(
        ctx,
        db,
        entity_type,
        entity_id,
        signal_type,
        source,
        value,
        confidence,
    ) {
        log::warn!(
            "services::signals::emit dropped {signal_type} on {entity_type}/{entity_id}: {e}"
        );
    }
}

/// Best-effort wrapper around `emit_and_propagate` that warn-logs on failure
/// rather than dropping the Result. Replaces the
/// `crate::services::signals::emit_and_propagate_or_log(...)` pattern.
#[allow(clippy::too_many_arguments)]
pub fn emit_and_propagate_or_log(
    ctx: &crate::services::context::ServiceContext<'_>,
    db: &ActionDb,
    engine: &PropagationEngine,
    entity_type: &str,
    entity_id: &str,
    signal_type: &str,
    source: &str,
    value: Option<&str>,
    confidence: f64,
) {
    if let Err(e) = emit_and_propagate(
        ctx,
        db,
        engine,
        entity_type,
        entity_id,
        signal_type,
        source,
        value,
        confidence,
    ) {
        log::warn!(
            "services::signals::emit_and_propagate dropped {signal_type} on {entity_type}/{entity_id}: {e}"
        );
    }
}

/// Emit a signal, propagate, and evaluate for self-healing re-enrichment.
// ServiceContext adds one arg; signal facade mirrors bus shape.
#[allow(clippy::too_many_arguments)]
pub fn emit_propagate_and_evaluate(
    ctx: &crate::services::context::ServiceContext<'_>,
    db: &ActionDb,
    engine: &PropagationEngine,
    entity_type: &str,
    entity_id: &str,
    signal_type: &str,
    source: &str,
    value: Option<&str>,
    confidence: f64,
    queue: &crate::intel_queue::IntelligenceQueue,
) -> Result<(String, Vec<String>), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    bus::emit_signal_propagate_and_evaluate(
        db,
        engine,
        entity_type,
        entity_id,
        signal_type,
        source,
        value,
        confidence,
        queue,
    )
    .map_err(|e| e.to_string())
}

/// Get all active (non-superseded) signals for an entity.
pub fn get_for_entity(
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
) -> Result<Vec<SignalEvent>, crate::db::DbError> {
    bus::get_active_signals(db, entity_type, entity_id)
}

/// Get active signals filtered by type.
pub fn get_by_type(
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    signal_type: &str,
) -> Result<Vec<SignalEvent>, crate::db::DbError> {
    bus::get_active_signals_by_type(db, entity_type, entity_id, signal_type)
}

struct TemporalSignalMaintenanceInput<'a> {
    entity_type: &'a str,
    entity_id: &'a str,
    signal_type: &'a str,
    source: &'a str,
    signal_id: &'a str,
    value: Option<&'a str>,
}

fn run_temporal_maintenance_for_signal(
    ctx: &crate::services::context::ServiceContext<'_>,
    db: &ActionDb,
    input: TemporalSignalMaintenanceInput<'_>,
) {
    if input.entity_type != "person" || input.signal_type != "title_change" {
        return;
    }
    let Some((title, org, seniority)) = role_change_from_signal_value(input.value) else {
        return;
    };

    let now = ctx.clock.now();
    let entity_id = input.entity_id.to_string();
    let role_input = crate::abilities::temporal::DetectRoleChangeInput {
        schema_version: 2,
        entity_type: input.entity_type.to_string(),
        entity_id: entity_id.clone(),
        observed_at: Some(now),
        title,
        org,
        seniority,
        source_refs: vec![crate::abilities::provenance::SourceRef::Direct {
            data_source: temporal_source_for_signal(input.source),
            identifier: crate::abilities::provenance::SourceIdentifier::Signal {
                signal_id: crate::abilities::provenance::SignalId::new(input.signal_id.to_string()),
            },
            observed_at: now,
            source_asof: Some(now),
        }],
    };

    if let Err(error) = crate::services::temporal::detect_role_change_in_db(db, role_input, now) {
        log::warn!(
            "temporal role progression maintenance skipped for title_change on person/{}: {error}",
            entity_id
        );
    }
}

fn temporal_source_for_signal(source: &str) -> crate::abilities::provenance::DataSource {
    match source.trim().to_ascii_lowercase().as_str() {
        "clay" => crate::abilities::provenance::DataSource::Clay,
        "google" => crate::abilities::provenance::DataSource::Google,
        "local_enrichment" => crate::abilities::provenance::DataSource::LocalEnrichment,
        other => crate::abilities::provenance::DataSource::Other(
            crate::abilities::provenance::SourceName::new(other),
        ),
    }
}

fn role_change_from_signal_value(
    value: Option<&str>,
) -> Option<(String, Option<String>, Option<String>)> {
    let raw = value?.trim();
    if raw.is_empty() {
        return None;
    }

    if let Ok(parsed) = serde_json::from_str::<Value>(raw) {
        let title = string_field(&parsed, &["title", "new_title"])
            .or_else(|| {
                string_field(&parsed, &["new_value"])
                    .and_then(split_title_org)
                    .map(|pair| pair.0)
            })
            .or_else(|| string_field(&parsed, &["new_value"]));
        let org = string_field(&parsed, &["org", "organization", "company"]).or_else(|| {
            string_field(&parsed, &["new_value"])
                .and_then(split_title_org)
                .and_then(|pair| pair.1)
        });
        let seniority = string_field(&parsed, &["seniority"]);
        return title.and_then(|title| normalized_role_change(title, org, seniority));
    }

    let (title, org) = split_title_org(raw.to_string()).unwrap_or_else(|| (raw.to_string(), None));
    normalized_role_change(title, org, None)
}

fn string_field(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        value
            .get(*key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    })
}

fn split_title_org(value: String) -> Option<(String, Option<String>)> {
    let (title, org) = value.rsplit_once(" at ")?;
    Some((title.trim().to_string(), Some(org.trim().to_string())))
}

fn normalized_role_change(
    title: String,
    org: Option<String>,
    seniority: Option<String>,
) -> Option<(String, Option<String>, Option<String>)> {
    let title = normalize_signal_field(Some(title))?;
    Some((
        title,
        normalize_signal_field(org),
        normalize_signal_field(seniority),
    ))
}

fn normalize_signal_field(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty() && value != "Unknown")
}

/// Generate signal-based briefing callouts. Discards the persistence-outcome
/// metadata in this convenience accessor — callers that need the degradation
/// signal call `signals::callouts::generate_callouts` directly.
pub fn get_callouts(
    db: &ActionDb,
    model: Option<&EmbeddingModel>,
    todays_meetings: &[Value],
) -> Vec<BriefingCallout> {
    let user_entity = crate::services::user_entity::get_user_entity_from_db(db).ok();
    let (list, _outcome) = crate::signals::callouts::generate_callouts(
        db,
        model,
        todays_meetings,
        user_entity.as_ref(),
    );
    list
}

/// Run cross-entity propagation rules for a signal.
pub fn run_propagation(
    db: &ActionDb,
    engine: &PropagationEngine,
    signal: &SignalEvent,
) -> Result<Vec<String>, crate::db::DbError> {
    engine.propagate(db, signal)
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn emit_fixture_event(
    db: &ActionDb,
    id: &str,
    entity_type: &str,
    entity_id: &str,
    signal_type: &str,
    source: &str,
    value: Option<&str>,
    confidence: f64,
    decay_half_life_days: Option<i32>,
    created_at: &str,
) -> Result<String, crate::db::DbError> {
    bus::emit_signal_fixture_event(
        db,
        id,
        entity_type,
        entity_id,
        signal_type,
        source,
        value,
        confidence,
        decay_half_life_days,
        created_at,
    )
}

/// Queue affected meeting preps for regeneration after entity correction.
pub fn invalidate_preps(queue: &Arc<Mutex<Vec<String>>>, meeting_ids: Vec<String>) {
    let mut q = queue.lock();
    q.extend(meeting_ids);
}
