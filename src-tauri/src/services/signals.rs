//! Signal service facade (I403).
//!
//! Service-layer callers use these functions instead of reaching into
//! `crate::signals::bus` directly.  Infrastructure callers that only have
//! a raw `db` handle (prepare/, processor/, gravatar/) stay direct.

use std::sync::Arc;
use parking_lot::Mutex;

use serde_json::Value;

use crate::db::ActionDb;
use crate::embeddings::EmbeddingModel;
use crate::signals::bus::{self, SignalEvent};
use crate::signals::callouts::BriefingCallout;
use crate::signals::propagation::PropagationEngine;

/// Emit a signal event (no propagation). Convenience wrapper around bus::emit_signal.
pub fn emit(
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    signal_type: &str,
    source: &str,
    value: Option<&str>,
    confidence: f64,
) -> Result<String, crate::db::DbError> {
    bus::emit_signal(
        db,
        entity_type,
        entity_id,
        signal_type,
        source,
        value,
        confidence,
    )
}

/// Emit a signal and run cross-entity propagation rules.
#[allow(clippy::too_many_arguments)]
pub fn emit_and_propagate(
    db: &ActionDb,
    engine: &PropagationEngine,
    entity_type: &str,
    entity_id: &str,
    signal_type: &str,
    source: &str,
    value: Option<&str>,
    confidence: f64,
) -> Result<(String, Vec<String>), crate::db::DbError> {
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
}

/// Emit a signal, propagate, and evaluate for self-healing re-enrichment.
#[allow(clippy::too_many_arguments)]
pub fn emit_propagate_and_evaluate(
    db: &ActionDb,
    engine: &PropagationEngine,
    entity_type: &str,
    entity_id: &str,
    signal_type: &str,
    source: &str,
    value: Option<&str>,
    confidence: f64,
    queue: &crate::intel_queue::IntelligenceQueue,
) -> Result<(String, Vec<String>), crate::db::DbError> {
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
