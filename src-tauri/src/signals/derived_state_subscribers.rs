//! Synchronous derived-state subscribers for internal invalidation signals.
//!
//! These subscribers run inside the caller's active database transaction.
//! Returning an error rolls back the source mutation together with the signal
//! row and any derived-state writes.

use serde_json::Value;

use crate::db::ActionDb;
use crate::services::context::ServiceContext;

/// A subscriber that runs synchronously inside the emitting transaction.
/// Errors roll back the user's mutation. Used for derived-state cache
/// invalidation that must be atomic with the source mutation.
pub trait DerivedStateSubscriber: Send + Sync {
    fn signal_type(&self) -> &'static str;
    fn apply(&self, ctx: &ServiceContext<'_>, tx: &ActionDb, payload: &Value)
        -> Result<(), String>;
}

struct StakeholdersChangedSubscriber;

impl DerivedStateSubscriber for StakeholdersChangedSubscriber {
    fn signal_type(&self) -> &'static str {
        crate::services::signals::STAKEHOLDERS_CHANGED_SIGNAL
    }

    fn apply(
        &self,
        ctx: &ServiceContext<'_>,
        tx: &ActionDb,
        payload: &Value,
    ) -> Result<(), String> {
        let payload: crate::services::signals::StakeholdersChangedPayload =
            serde_json::from_value(payload.clone())
                .map_err(|e| format!("invalid stakeholders_changed payload: {e}"))?;

        if payload.entity_id.trim().is_empty() {
            return Err("missing entity_id".to_string());
        }
        if payload.entity_type.trim().is_empty() {
            return Err("missing entity_type".to_string());
        }
        if payload.mutation_source.trim().is_empty() {
            return Err("missing mutation_source".to_string());
        }

        crate::services::derived_state::rebuild_stakeholder_insights_cache_for_entity_inner(
            ctx,
            tx,
            &payload.entity_id,
            &payload.entity_type,
        )
        .map_err(|e| format!("stakeholder cache rebuild failed: {}", e.as_str()))
    }
}

static STAKEHOLDERS_CHANGED_SUBSCRIBER: StakeholdersChangedSubscriber =
    StakeholdersChangedSubscriber;
static REGISTRY: &[&dyn DerivedStateSubscriber] = &[&STAKEHOLDERS_CHANGED_SUBSCRIBER];

/// Compile-time registry. Add new derived-state subscribers here.
pub fn registry() -> &'static [&'static dyn DerivedStateSubscriber] {
    REGISTRY
}
