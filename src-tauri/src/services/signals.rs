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
    let signal_id =
        bus::emit_signal_in_active_tx(tx, entity_type, entity_id, signal_type, source, &payload)
            .map_err(|e| e.to_string())?;

    for subscriber in crate::signals::derived_state_subscribers::registry() {
        if subscriber.signal_type() == signal_type {
            subscriber.apply(ctx, tx, &payload)?;
        }
    }

    Ok(signal_id)
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
    bus::emit_signal_and_propagate(
        db,
        engine,
        entity_type,
        entity_id,
        signal_type,
        source,
        value,
        confidence,
    )
    .map_err(|e| e.to_string())
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

/// Generate signal-based briefing callouts.
pub fn get_callouts(
    db: &ActionDb,
    model: Option<&EmbeddingModel>,
    todays_meetings: &[Value],
) -> Vec<BriefingCallout> {
    let user_entity = crate::services::user_entity::get_user_entity_from_db(db).ok();
    crate::signals::callouts::generate_callouts(db, model, todays_meetings, user_entity.as_ref())
}

/// Run cross-entity propagation rules for a signal.
pub fn run_propagation(
    db: &ActionDb,
    engine: &PropagationEngine,
    signal: &SignalEvent,
) -> Result<Vec<String>, crate::db::DbError> {
    engine.propagate(db, signal)
}

/// Queue affected meeting preps for regeneration after entity correction.
pub fn invalidate_preps(queue: &Arc<Mutex<Vec<String>>>, meeting_ids: Vec<String>) {
    let mut q = queue.lock();
    q.extend(meeting_ids);
}
