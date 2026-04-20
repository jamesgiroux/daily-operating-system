pub mod calendar_adapter;
pub mod cascade;
pub mod email_adapter;
pub mod evidence;
pub mod phases;
pub mod primitives;
pub mod repository;
pub mod rules;
pub mod types;

pub use types::{
    Candidate, EntityRef, LinkOutcome, LinkRole, LinkTier, LinkingContext, OwnerRef, OwnerType,
    Participant, ParticipantRole, RuleOutcome, Trigger,
};

use std::sync::Arc;

use crate::state::AppState;

// ---------------------------------------------------------------------------
// Public evaluate — synchronous, called from calendar and email pipelines
// ---------------------------------------------------------------------------

/// Run the four-phase linking engine for a single owner.
///
/// Reads from DB only; all writes (linked_entities_raw, linking_dismissals,
/// entity_linking_evaluations, account_stakeholders) happen inside this call
/// in a single transaction per the concurrency contract in DOS-258.
pub fn evaluate(
    _state: Arc<AppState>,
    _ctx: LinkingContext,
    _trigger: Trigger,
) -> Result<LinkOutcome, String> {
    // TODO(Lane-C): implement — delegate to phases::run_phases
    unimplemented!("Lane C: evaluate")
}

// ---------------------------------------------------------------------------
// Manual overrides — async because they may emit signals or update queues
// ---------------------------------------------------------------------------

/// User explicitly sets (or clears) the primary entity for an owner.
pub async fn manual_set_primary(
    _state: Arc<AppState>,
    _owner_type: OwnerType,
    _owner_id: &str,
    _entity: Option<EntityRef>,
) -> Result<LinkOutcome, String> {
    // TODO(Lane-C): write source='user' row + trigger re-evaluate
    unimplemented!("Lane C: manual_set_primary")
}

/// User dismisses a suggested entity link for an owner.
///
/// Writes a linking_dismissals row AND sets source='user_dismissed' on the
/// linked_entities_raw row in the same transaction (dismissal-wins-race).
pub async fn manual_dismiss(
    _state: Arc<AppState>,
    _owner_type: OwnerType,
    _owner_id: &str,
    _entity: EntityRef,
) -> Result<LinkOutcome, String> {
    // TODO(Lane-C): transactional dismiss + re-evaluate
    unimplemented!("Lane C: manual_dismiss")
}

/// Undo a previous dismissal, removing the linking_dismissals row.
pub async fn manual_undismiss(
    _state: Arc<AppState>,
    _owner_type: OwnerType,
    _owner_id: &str,
    _entity: EntityRef,
) -> Result<LinkOutcome, String> {
    // TODO(Lane-C): delete linking_dismissals row + re-evaluate
    unimplemented!("Lane C: manual_undismiss")
}

// ---------------------------------------------------------------------------
// Stakeholder queue — sole post-migration writers to account_stakeholders (C2)
// ---------------------------------------------------------------------------

/// Promote a pending_review stakeholder suggestion to status='active'.
///
/// This is the ONLY function that confirms a stakeholder after auto-suggestion.
/// No other code may set account_stakeholders.status = 'active' from 'pending_review'.
pub async fn confirm_stakeholder_suggestion(
    _state: Arc<AppState>,
    _account_id: &str,
    _person_id: &str,
) -> Result<(), String> {
    // TODO(Lane-C): UPDATE account_stakeholders SET status='active' WHERE ...
    unimplemented!("Lane C: confirm_stakeholder_suggestion")
}

/// Dismiss a pending_review stakeholder suggestion, hiding it from the queue.
///
/// Sets status='dismissed' and blocks future re-surfacing of this person
/// on this account.
pub async fn dismiss_stakeholder_suggestion(
    _state: Arc<AppState>,
    _account_id: &str,
    _person_id: &str,
) -> Result<(), String> {
    // TODO(Lane-C): UPDATE account_stakeholders SET status='dismissed' WHERE ...
    unimplemented!("Lane C: dismiss_stakeholder_suggestion")
}
