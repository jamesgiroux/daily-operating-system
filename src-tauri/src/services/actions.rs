// Actions service — extracted from commands.rs
// Business logic for action status transitions with signal emission.

use std::{collections::HashMap, sync::Arc};

use crate::commands::{ActionDetail, ActionListItem, CreateActionRequest, UpdateActionRequest};
use crate::db::ActionDb;
use crate::services::context::ServiceContext;
use crate::state::AppState;
use crate::types::{Action, Priority};

/// Emit a propagation signal and warn-log on failure instead of dropping
/// the Result silently. Action signals feed downstream callouts and
/// dashboards; a silent persistence drop here used to make a completed
/// action look like nothing changed in the surfaced state, which the
/// cycle-10 review flagged as the same silent-error class as the trust
/// recompute and feedback paths.
#[allow(clippy::too_many_arguments)]
fn emit_action_signal(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    engine: &crate::signals::propagation::PropagationEngine,
    entity_type: &str,
    entity_id: &str,
    signal_type: &str,
    source: &str,
    value: Option<&str>,
    confidence: f64,
) {
    if let Err(e) = crate::services::signals::emit_and_propagate(
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
            "actions: emit_and_propagate dropped {signal_type} for {entity_type}/{entity_id}: {e}"
        );
    }
}

/// Helper: resolve entity type and ID from an action for signal emission.
fn action_entity_info(action: &crate::db::DbAction, fallback_id: &str) -> (&'static str, String) {
    let entity_type = if action.account_id.is_some() {
        "account"
    } else if action.project_id.is_some() {
        "project"
    } else {
        "action"
    };
    let entity_id = action
        .account_id
        .as_deref()
        .or(action.project_id.as_deref())
        .unwrap_or(fallback_id)
        .to_string();
    (entity_type, entity_id)
}

/// Complete an action and emit the completion signal.
pub fn complete_action(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    engine: &crate::signals::propagation::PropagationEngine,
    id: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let action = db.get_action_by_id(id).ok().flatten();
    db.complete_action(id).map_err(|e| e.to_string())?;

    if let Some(ref action) = action {
        let (entity_type, entity_id) = action_entity_info(action, id);
        emit_action_signal(
            ctx,
            db,
            engine,
            entity_type,
            &entity_id,
            "action_completed",
            action.source_type.as_deref().unwrap_or("unknown"),
            Some(&format!("{{\"action_id\":\"{}\"}}", id)),
            0.7,
        );

        // DOS Work-tab: Commitment lifecycle — delivered + tombstone the bridge.
        if action.action_kind == crate::action_status::KIND_COMMITMENT {
            emit_action_signal(
                ctx,
                db,
                engine,
                entity_type,
                &entity_id,
                "commitment_delivered",
                action.source_type.as_deref().unwrap_or("commitment"),
                Some(&format!(
                    "{{\"action_id\":\"{}\",\"title\":\"{}\"}}",
                    id,
                    action.title.replace('"', "\\\"")
                )),
                0.8,
            );
            if let Err(e) =
                crate::services::commitment_bridge::tombstone_commitment_bridge(ctx, db, id)
            {
                log::warn!("commitment_bridge tombstone on complete failed (non-fatal): {e}");
            }
        }
    }

    Ok(())
}

/// Reopen a completed action, setting it back to pending.
pub fn reopen_action(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    engine: &crate::signals::propagation::PropagationEngine,
    id: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let action = db.get_action_by_id(id).ok().flatten();
    db.reopen_action(id).map_err(|e| e.to_string())?;

    if let Some(ref action) = action {
        let (entity_type, entity_id) = action_entity_info(action, id);
        emit_action_signal(
            ctx,
            db,
            engine,
            entity_type,
            &entity_id,
            "action_reopened",
            "user_correction",
            Some(&format!("{{\"action_id\":\"{}\"}}", id)),
            0.4,
        );
    }

    Ok(())
}

/// Accept a suggested action, moving it to pending.
pub fn accept_suggested_action(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    engine: &crate::signals::propagation::PropagationEngine,
    id: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let action = db.get_action_by_id(id).ok().flatten();
    db.accept_suggested_action(id).map_err(|e| e.to_string())?;

    if let Some(ref action) = action {
        let (entity_type, entity_id) = action_entity_info(action, id);
        emit_action_signal(
            ctx,
            db,
            engine,
            entity_type,
            &entity_id,
            "action_accepted",
            action.source_type.as_deref().unwrap_or("unknown"),
            Some(&format!(
                "{{\"action_id\":\"{}\",\"title\":\"{}\"}}",
                id,
                action.title.replace('"', "\\\"")
            )),
            0.8,
        );

        // DOS Work-tab: Commitment lifecycle — accepted (backlog → unstarted).
        if action.action_kind == crate::action_status::KIND_COMMITMENT {
            emit_action_signal(
                ctx,
                db,
                engine,
                entity_type,
                &entity_id,
                "commitment_accepted",
                action.source_type.as_deref().unwrap_or("commitment"),
                Some(&format!(
                    "{{\"action_id\":\"{}\",\"title\":\"{}\"}}",
                    id,
                    action.title.replace('"', "\\\"")
                )),
                0.7,
            );
        }
    }

    Ok(())
}

/// Reject a suggested action by archiving it.
pub fn reject_suggested_action(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    engine: &crate::signals::propagation::PropagationEngine,
    id: &str,
    source: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let action = db.get_action_by_id(id).ok().flatten();
    db.reject_suggested_action_with_source(id, source)
        .map_err(|e| e.to_string())?;

    // Emit rejection signal for correction learning
    if let Some(ref action) = action {
        let (entity_type, entity_id) = action_entity_info(action, id);
        emit_action_signal(
            ctx,
            db,
            engine,
            entity_type,
            &entity_id,
            "action_rejected",
            action.source_type.as_deref().unwrap_or("unknown"),
            Some(&format!(
                "{{\"action_id\":\"{}\",\"title\":\"{}\"}}",
                id,
                action.title.replace('"', "\\\"")
            )),
            0.3,
        );

        // Record rejection patterns for future suppression
        if let Err(e) = db.record_rejection_pattern(action) {
            log::warn!("Failed to record rejection pattern: {}", e);
        }

        // DOS Work-tab: Commitment lifecycle — rejected + tombstone the bridge.
        if action.action_kind == crate::action_status::KIND_COMMITMENT {
            emit_action_signal(
                ctx,
                db,
                engine,
                entity_type,
                &entity_id,
                "commitment_rejected",
                action.source_type.as_deref().unwrap_or("commitment"),
                Some(&format!(
                    "{{\"action_id\":\"{}\",\"title\":\"{}\"}}",
                    id,
                    action.title.replace('"', "\\\"")
                )),
                0.5,
            );
            if let Err(e) =
                crate::services::commitment_bridge::tombstone_commitment_bridge(ctx, db, id)
            {
                log::warn!("commitment_bridge tombstone on reject failed (non-fatal): {e}");
            }
        }
    }

    Ok(())
}

/// Dismiss a suggested action — preference-based, not quality-based.
///
/// Archives the action and records the rejection-pattern tombstone (so the
/// enrichment pipeline doesn't re-propose it via `is_action_suppressed`),
/// but does NOT emit the `action_rejected` correction signal. The user is
/// saying "I don't want this," not "this is wrong" — Bayesian source
/// weights should be untouched.
///
/// Pairs with `reject_suggested_action` (the "Not accurate" path), which
/// keeps the quality-penalty signal.
pub fn dismiss_suggested_action(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    engine: &crate::signals::propagation::PropagationEngine,
    id: &str,
    source: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let action = db.get_action_by_id(id).ok().flatten();
    db.reject_suggested_action_with_source(id, source)
        .map_err(|e| e.to_string())?;

    if let Some(ref action) = action {
        // Tombstone so the enrichment pipeline suppresses re-proposal
        // (rejected_action_patterns is consulted by is_action_suppressed).
        if let Err(e) = db.record_rejection_pattern(action) {
            log::warn!("Failed to record rejection pattern: {}", e);
        }

        // For commitment-kind actions still tombstone the bridge so the
        // commitment view doesn't resurrect it from the source artifact.
        if action.action_kind == crate::action_status::KIND_COMMITMENT {
            if let Err(e) =
                crate::services::commitment_bridge::tombstone_commitment_bridge(ctx, db, id)
            {
                log::warn!("commitment_bridge tombstone on dismiss failed (non-fatal): {e}");
            }
        }

        // Telemetry-only: lets us see dismissal volume without it counting
        // as a quality penalty against the source. Confidence 0.0 makes it
        // a non-scoring observation.
        let (entity_type, entity_id) = action_entity_info(action, id);
        emit_action_signal(
            ctx,
            db,
            engine,
            entity_type,
            &entity_id,
            "action_dismissed",
            action.source_type.as_deref().unwrap_or("unknown"),
            Some(&format!(
                "{{\"action_id\":\"{}\",\"title\":\"{}\"}}",
                id,
                action.title.replace('"', "\\\"")
            )),
            0.0,
        );
    }

    Ok(())
}

/// Cycle an action's priority with signal emission.
pub fn update_action_priority(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    engine: &crate::signals::propagation::PropagationEngine,
    id: &str,
    priority: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let action = db.get_action_by_id(id).ok().flatten();
    db.update_action_priority(id, priority)
        .map_err(|e| e.to_string())?;

    if let Some(ref action) = action {
        let (entity_type, entity_id) = action_entity_info(action, id);
        emit_action_signal(
            ctx,
            db,
            engine,
            entity_type,
            &entity_id,
            "priority_corrected",
            action.source_type.as_deref().unwrap_or("unknown"),
            Some(&format!(
                "{{\"action_id\":\"{}\",\"old\":\"{}\",\"new\":\"{}\"}}",
                id, action.priority, priority
            )),
            0.5,
        );
    }

    Ok(())
}

/// Result type for all actions
#[derive(Debug, serde::Serialize)]
#[allow(clippy::large_enum_variant)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum ActionsResult {
    Success { data: Vec<Action> },
    Empty { message: String },
    Error { message: String },
}

/// Get all actions with full context from SQLite (DB is sole source).
pub async fn get_all_actions(state: &AppState) -> ActionsResult {
    // Load all pending actions from DB
    let actions: Vec<Action> = state
        .db_read(|db| {
            db.get_non_briefing_pending_actions()
                .map_err(|e| e.to_string())
        })
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|dba| {
            let priority = Priority::from_i32(dba.priority);
            Action {
                id: dba.id,
                title: dba.title,
                account: dba.account_id,
                due_date: dba.due_date,
                priority,
                status: crate::types::ActionStatus::Unstarted,
                is_overdue: None,
                context: dba.context,
                source: dba.source_label,
                days_overdue: None,
            }
        })
        .collect();

    if actions.is_empty() {
        ActionsResult::Empty {
            message: "No actions yet. Actions appear after your first briefing.".to_string(),
        }
    } else {
        ActionsResult::Success { data: actions }
    }
}

/// Create a new action with validation and signal emission.
pub async fn create_action(
    ctx: &ServiceContext<'_>,
    request: CreateActionRequest,
    state: &Arc<AppState>,
) -> Result<String, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let CreateActionRequest {
        title,
        priority,
        due_date,
        account_id,
        project_id,
        person_id,
        context,
        source_label,
        action_kind,
    } = request;

    let title = crate::util::validate_bounded_string(&title, "title", 1, 280)?;
    let priority_str = priority.unwrap_or_else(|| "3".to_string());
    let priority: i32 = priority_str
        .parse()
        .map_err(|_| format!("Invalid priority: {priority_str}"))?;
    if !(0..=4).contains(&priority) {
        return Err(format!("Priority must be 0-4, got: {priority}"));
    }
    if let Some(ref date) = due_date {
        crate::util::validate_yyyy_mm_dd(date, "due_date")?;
    }
    if let Some(ref id) = account_id {
        crate::util::validate_id_slug(id, "account_id")?;
    }
    if let Some(ref id) = project_id {
        crate::util::validate_id_slug(id, "project_id")?;
    }
    if let Some(ref id) = person_id {
        crate::util::validate_id_slug(id, "person_id")?;
    }
    if let Some(ref value) = context {
        crate::util::validate_bounded_string(value, "context", 1, 2000)?;
    }
    if let Some(ref value) = source_label {
        crate::util::validate_bounded_string(value, "source_label", 1, 200)?;
    }

    let now = ctx.clock.now().to_rfc3339();
    let id = uuid::Uuid::new_v4().to_string();

    let action_kind = action_kind
        .filter(|k| !k.trim().is_empty())
        .unwrap_or_else(|| crate::action_status::KIND_TASK.to_string());
    if !crate::action_status::ALL_KINDS.contains(&action_kind.as_str()) {
        return Err(format!("Invalid action_kind: {action_kind}"));
    }

    let action = crate::db::DbAction {
        id: id.clone(),
        title,
        priority,
        status: crate::action_status::UNSTARTED.to_string(),
        created_at: now.clone(),
        due_date,
        completed_at: None,
        account_id,
        project_id,
        source_type: Some("user_manual".to_string()),
        source_id: None,
        source_label,
        action_kind,
        commitment_id: None,
        owner_raw: None,
        owner_entity_id: None,
        owner_confidence: None,
        owner_source: None,
        trust_score: None,
        trust_band: None,
        commitment_source_count: None,
        context,
        waiting_on: None,
        updated_at: now,
        person_id,
        account_name: None,
        next_meeting_title: None,
        next_meeting_start: None,
        needs_decision: false,
        decision_owner: None,
        decision_stakes: None,
        linear_identifier: None,
        linear_url: None,
    };

    let engine = state.signals.engine.clone();
    let state_for_ctx = state.clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            db.upsert_action(&action).map_err(|e| e.to_string())?;

            // Emit signal for manually created actions
            let (entity_type, entity_id) = action_entity_info(&action, &action.id);
            emit_action_signal(
                &ctx,
                db,
                &engine,
                entity_type,
                &entity_id,
                "action_created_manually",
                "user_action",
                Some(&format!(
                    "{{\"action_id\":\"{}\",\"title\":\"{}\"}}",
                    action.id,
                    action.title.replace('"', "\\\"")
                )),
                1.0,
            );

            // Scan for decision-indicating keywords after creation
            #[allow(
                clippy::let_underscore_must_use,
                reason = "intentional best-effort discard; preserves existing non-blocking behavior"
            )]
            let _ = db.scan_and_flag_decisions();

            // Best-effort auto-link to matching objectives
            if let Some(ref acct_id) = action.account_id {
                if let Err(e) =
                    auto_link_action_to_objectives(&ctx, db, &action.id, &action.title, acct_id)
                {
                    log::warn!("Auto-link action to objectives failed (non-fatal): {}", e);
                }
            }

            Ok(id)
        })
        .await
}

/// Auto-link a newly created action to objectives with similar titles.
///
/// Uses Jaccard word similarity (threshold 0.6) to find matching objectives
/// for the action's account. Emits an `action_auto_linked` signal per match.
fn auto_link_action_to_objectives(
    ctx: &ServiceContext<'_>,
    db: &crate::db::ActionDb,
    action_id: &str,
    action_title: &str,
    account_id: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let objectives = db
        .get_account_objectives(account_id)
        .map_err(|e| e.to_string())?;

    for objective in &objectives {
        if objective.status != "active" {
            continue;
        }
        let score = crate::helpers::jaccard_word_similarity(action_title, &objective.title);
        if score > 0.6 {
            if let Err(e) = db.link_action_to_objective(action_id, &objective.id) {
                log::warn!(
                    "Failed to auto-link action {} to objective {}: {}",
                    action_id,
                    objective.id,
                    e
                );
                continue;
            }
            // Emit signal — warn-log on failure so the auto-link history isn't
            // silently lost when downstream propagation needs it.
            if let Err(e) = crate::services::signals::emit(
                ctx,
                db,
                "account",
                account_id,
                "action_auto_linked",
                "system",
                Some(&format!(
                    "{{\"action_id\":\"{}\",\"objective_id\":\"{}\",\"score\":{:.2}}}",
                    action_id, objective.id, score
                )),
                score,
            ) {
                log::warn!(
                    "actions: action_auto_linked emit dropped for account={account_id} action={action_id} objective={}: {e}",
                    objective.id
                );
            }
            log::info!(
                "Auto-linked action {} to objective {} (score: {:.2})",
                action_id,
                objective.id,
                score
            );
        }
    }
    Ok(())
}

/// Update arbitrary fields on an existing action.
pub async fn update_action(
    ctx: &ServiceContext<'_>,
    request: UpdateActionRequest,
    state: &Arc<AppState>,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    validate_update_action_request(&request)?;

    let state_for_ctx = state.clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            apply_update_action(&ctx, db, request)
        })
        .await
}

fn validate_update_action_request(request: &UpdateActionRequest) -> Result<(), String> {
    crate::util::validate_id_slug(&request.id, "id")?;
    if let Some(ref p) = request.priority {
        let pv: i32 = p.parse().map_err(|_| format!("Invalid priority: {p}"))?;
        if !(0..=4).contains(&pv) {
            return Err(format!("Priority must be 0-4, got: {pv}"));
        }
    }
    if let Some(ref t) = request.title {
        crate::util::validate_bounded_string(t, "title", 1, 280)?;
    }
    if let Some(ref d) = request.due_date {
        crate::util::validate_yyyy_mm_dd(d, "due_date")?;
    }
    if let Some(ref c) = request.context {
        crate::util::validate_bounded_string(c, "context", 1, 2000)?;
    }
    if let Some(ref owner) = request.owner_raw {
        crate::util::validate_bounded_string(owner, "owner_raw", 1, 200)?;
    }
    if let Some(ref s) = request.source_label {
        crate::util::validate_bounded_string(s, "source_label", 1, 200)?;
    }
    if let Some(ref a) = request.account_id {
        crate::util::validate_id_slug(a, "account_id")?;
    }
    if let Some(ref p) = request.project_id {
        crate::util::validate_id_slug(p, "project_id")?;
    }
    if let Some(ref p) = request.person_id {
        crate::util::validate_id_slug(p, "person_id")?;
    }
    Ok(())
}

pub(crate) fn apply_update_action(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    request: UpdateActionRequest,
) -> Result<(), String> {
    let UpdateActionRequest {
        id,
        title,
        due_date,
        clear_due_date,
        context,
        clear_context,
        owner_raw,
        clear_owner,
        source_label,
        clear_source_label,
        account_id,
        clear_account,
        project_id,
        clear_project,
        person_id,
        clear_person,
        priority,
    } = request;

    let mut action = db
        .get_action_by_id(&id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Action not found: {id}"))?;

    if let Some(t) = title {
        action.title = t;
    }
    if let Some(p) = priority {
        action.priority = p.parse::<i32>().unwrap_or(3);
    }
    if clear_due_date == Some(true) {
        action.due_date = None;
    } else if let Some(d) = due_date {
        action.due_date = Some(d);
    }
    if clear_context == Some(true) {
        action.context = None;
    } else if let Some(c) = context {
        action.context = Some(c);
    }
    if clear_owner == Some(true) {
        action.owner_raw = None;
        action.owner_entity_id = None;
        action.owner_confidence = None;
        action.owner_source = Some("user_reassigned".to_string());
    } else if let Some(owner) = owner_raw {
        let commitment_id = action
            .commitment_id
            .clone()
            .unwrap_or_else(|| action.id.clone());
        let account_id_for_resolution = action.account_id.as_deref().unwrap_or("");
        let resolved = crate::abilities::read::resolve_owner::resolve_owner_with_mode(
            db,
            account_id_for_resolution,
            &commitment_id,
            Some(&owner),
            crate::abilities::read::resolve_owner::OwnerResolutionMode::FreshInput,
        )?;
        action.owner_raw = Some(owner);
        action.owner_entity_id = resolved.owner_entity_id;
        action.owner_confidence = resolved.owner_confidence.or(Some(1.0));
        action.owner_source = Some("user_reassigned".to_string());
        action.context = crate::services::commitment_bridge::strip_owner_context_for_action(
            action.context.as_deref(),
        );
    }
    if clear_source_label == Some(true) {
        action.source_label = None;
    } else if let Some(s) = source_label {
        action.source_label = Some(s);
    }
    if clear_account == Some(true) {
        action.account_id = None;
    } else if let Some(a) = account_id {
        action.account_id = Some(a);
    }
    if clear_project == Some(true) {
        action.project_id = None;
    } else if let Some(p) = project_id {
        action.project_id = Some(p);
    }
    if clear_person == Some(true) {
        action.person_id = None;
    } else if let Some(p) = person_id {
        action.person_id = Some(p);
    }

    action.updated_at = ctx.clock.now().to_rfc3339();
    db.upsert_action(&action).map_err(|e| e.to_string())
}

/// Get full detail for a single action, with resolved relationships.
pub fn get_action_detail(db: &ActionDb, action_id: &str) -> Result<ActionDetail, String> {
    let action = db
        .get_action_by_id(action_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Action not found: {action_id}"))?;

    // Resolve account name
    let account_name = if let Some(ref aid) = action.account_id {
        db.get_account(aid).ok().flatten().map(|a| a.name)
    } else {
        None
    };

    // Resolve source meeting title
    let source_meeting_title = if let Some(ref sid) = action.source_id {
        db.get_meeting_by_id(sid).ok().flatten().map(|m| m.title)
    } else {
        None
    };

    Ok(ActionDetail {
        action,
        account_name,
        source_meeting_title,
    })
}

/// Get actions from the SQLite database for display.
///
/// Returns pending actions (within `days_ahead` window) combined with recently
/// completed actions (last 48 hours). Account names are batch-resolved.
pub fn get_actions_from_db(db: &ActionDb, days_ahead: i32) -> Result<Vec<ActionListItem>, String> {
    let mut actions = db.get_due_actions(days_ahead).map_err(|e| e.to_string())?;
    let completed = db.get_completed_actions(48).map_err(|e| e.to_string())?;
    actions.extend(completed);

    // Batch-resolve account names: collect unique IDs, single query each
    let mut name_cache: HashMap<String, String> = HashMap::new();
    for a in &actions {
        if let Some(ref aid) = a.account_id {
            if !name_cache.contains_key(aid) {
                if let Ok(Some(account)) = db.get_account(aid) {
                    name_cache.insert(aid.clone(), account.name);
                }
            }
        }
    }

    let items = actions
        .into_iter()
        .map(|a| {
            let account_name = a
                .account_id
                .as_ref()
                .and_then(|aid| name_cache.get(aid).cloned());
            ActionListItem {
                action: a,
                account_name,
            }
        })
        .collect();

    Ok(items)
}

/// Get all suggested (AI-suggested) actions.
///
/// Unfiltered. Use `get_suggested_actions_for_user` in the command layer so
/// the user only sees their own commitments + unassigned items by default —
/// AI extraction tags every speaker in transcripts as a potential owner, so
/// the unfiltered list on a real workspace is 90%+ other people's work.
pub fn get_suggested_actions(db: &ActionDb) -> Result<Vec<crate::db::DbAction>, String> {
    db.get_suggested_actions().map_err(|e| e.to_string())
}

/// Get suggested actions filtered to the current user + unassigned rows.
///
/// Reads the user's name from `user_entity` (the /me page) and applies a
/// case-insensitive owner-prefix match on `actions.context`. Ambiguous rows
/// without a recognisable owner prefix still surface so the user doesn't
/// miss triage work.
pub fn get_suggested_actions_for_user(db: &ActionDb) -> Result<Vec<crate::db::DbAction>, String> {
    let user_name = crate::services::user_entity::get_user_entity_from_db(db)
        .ok()
        .and_then(|u| u.name)
        .filter(|n| !n.trim().is_empty());
    db.get_suggested_actions_for_user(user_name.as_deref())
        .map_err(|e| e.to_string())
}

/// DOS Work-tab Phase 3: open, user-accepted commitments for an account.
///
/// Thin wrapper over `ActionDb::get_account_commitments` that surfaces
/// `action_kind='commitment'` rows in (unstarted, started) — the read side
/// of the Commitments chapter. Backlog commitments are unaccepted
/// suggestions and live in `get_account_suggestions` until the user
/// promotes them.
pub fn get_account_commitments(
    db: &ActionDb,
    account_id: &str,
) -> Result<Vec<crate::db::DbAction>, String> {
    db.get_account_commitments(account_id)
        .map_err(|e| e.to_string())
}

/// DOS Work-tab Phase 3: backlog suggestions for an account.
///
/// Thin wrapper over `ActionDb::get_account_suggestions` — backlog tasks
/// and backlog commitments both appear as suggestions until accepted
/// (backlog → unstarted) or rejected (→ archived).
pub fn get_account_suggestions(
    db: &ActionDb,
    account_id: &str,
) -> Result<Vec<crate::db::DbAction>, String> {
    db.get_account_suggestions(account_id)
        .map_err(|e| e.to_string())
}

/// DOS Work-tab Phase 3: recently landed actions for an account (30-day
/// window, cap 20).
///
/// Thin wrapper over `ActionDb::get_account_recently_landed`.
pub fn get_account_recently_landed(
    db: &ActionDb,
    account_id: &str,
) -> Result<Vec<crate::db::DbAction>, String> {
    db.get_account_recently_landed(account_id)
        .map_err(|e| e.to_string())
}

/// Resolve a decision: clear needs_decision flag and emit signal.
pub fn resolve_decision(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    engine: &crate::signals::propagation::PropagationEngine,
    id: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let action = db.get_action_by_id(id).ok().flatten();
    let updated = db.resolve_decision(id).map_err(|e| e.to_string())?;
    if !updated {
        return Err(format!("Action not found or not flagged as decision: {id}"));
    }

    if let Some(ref action) = action {
        let (entity_type, entity_id) = action_entity_info(action, id);
        emit_action_signal(
            ctx,
            db,
            engine,
            entity_type,
            &entity_id,
            "decision_resolved",
            "user_action",
            Some(&format!("{{\"action_id\":\"{}\"}}", id)),
            0.8,
        );
    }

    Ok(())
}

/// Count actions approaching the 30-day auto-archive threshold.
///
/// Returns the number of actions that are older than 14 days but not yet 30 days,
/// in backlog/unstarted status, with priority > 1 and not waiting on anyone.
/// These are at risk of aging out without being acted on.
pub fn get_aging_action_count(db: &ActionDb) -> Result<i64, String> {
    db.conn_ref()
        .query_row(
            "SELECT COUNT(*) FROM actions
             WHERE status IN ('backlog', 'unstarted')
               AND created_at < datetime('now', '-14 days')
               AND created_at >= datetime('now', '-30 days')
               AND priority > 1
               AND waiting_on IS NULL",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())
}

/// Scan unstarted/backlog actions for decision-indicating keywords and flag them.
///
/// Called after action creation and from the scheduler.
pub fn scan_and_flag_decisions(db: &ActionDb) -> Result<usize, String> {
    db.scan_and_flag_decisions().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::test_db;
    use crate::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};
    use chrono::TimeZone;

    macro_rules! make_ctx {
        ($ctx:ident) => {
            let clock =
                FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
            let rng = SeedableRng::new(42);
            let ext = ExternalClients::default();
            let $ctx = ServiceContext::test_live(&clock, &rng, &ext);
        };
    }

    fn seed_owner_fixture(db: &ActionDb) {
        db.conn_ref()
            .execute(
                "INSERT INTO people (id, email, name, updated_at)
                 VALUES ('p-alex', 'alex@example.com', 'Alex Chen', '2026-01-01'),
                        ('p-jamie', 'jamie@example.com', 'Jamie Lee', '2026-01-01')",
                [],
            )
            .unwrap();
        db.conn_ref()
            .execute(
                "INSERT INTO account_stakeholders (account_id, person_id, data_source)
                 VALUES ('acct-1', 'p-alex', 'user'),
                        ('acct-1', 'p-jamie', 'user')",
                [],
            )
            .unwrap();
        db.conn_ref()
            .execute(
                "INSERT INTO actions
                 (id, title, priority, status, created_at, updated_at, account_id,
                  action_kind, commitment_id, context)
                 VALUES ('action-1', 'Send deck', 3, 'unstarted', '2026-01-01',
                         '2026-01-01', 'acct-1', 'commitment', 'commitment:test',
                         'owner: stale owner')",
                [],
            )
            .unwrap();
    }

    fn owner_request(owner: Option<&str>, clear_owner: Option<bool>) -> UpdateActionRequest {
        UpdateActionRequest {
            id: "action-1".to_string(),
            title: None,
            due_date: None,
            clear_due_date: None,
            context: None,
            clear_context: None,
            owner_raw: owner.map(ToString::to_string),
            clear_owner,
            source_label: None,
            clear_source_label: None,
            account_id: None,
            clear_account: None,
            project_id: None,
            clear_project: None,
            person_id: None,
            clear_person: None,
            priority: None,
        }
    }

    fn owner_state(
        db: &ActionDb,
    ) -> (
        Option<String>,
        Option<String>,
        Option<f64>,
        Option<String>,
        Option<String>,
    ) {
        db.conn_ref()
            .query_row(
                "SELECT owner_raw, owner_entity_id, owner_confidence, owner_source, context
                 FROM actions WHERE id = 'action-1'",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .unwrap()
    }

    #[test]
    fn first_user_reassignment_resolves_new_owner() {
        let db = test_db();
        make_ctx!(ctx);
        seed_owner_fixture(&db);

        apply_update_action(&ctx, &db, owner_request(Some("Alex Chen"), None)).unwrap();

        let (owner_raw, owner_entity_id, owner_confidence, owner_source, context) =
            owner_state(&db);
        assert_eq!(owner_raw.as_deref(), Some("Alex Chen"));
        assert_eq!(owner_entity_id.as_deref(), Some("p-alex"));
        assert_eq!(owner_confidence, Some(0.96));
        assert_eq!(owner_source.as_deref(), Some("user_reassigned"));
        assert_eq!(context, None);
    }

    #[test]
    fn second_user_reassignment_resolves_against_new_raw_owner() {
        let db = test_db();
        make_ctx!(ctx);
        seed_owner_fixture(&db);

        apply_update_action(&ctx, &db, owner_request(Some("Alex Chen"), None)).unwrap();
        apply_update_action(&ctx, &db, owner_request(Some("Jamie Lee"), None)).unwrap();

        let (owner_raw, owner_entity_id, owner_confidence, owner_source, _) = owner_state(&db);
        assert_eq!(owner_raw.as_deref(), Some("Jamie Lee"));
        assert_eq!(owner_entity_id.as_deref(), Some("p-jamie"));
        assert_eq!(owner_confidence, Some(0.96));
        assert_eq!(owner_source.as_deref(), Some("user_reassigned"));
    }

    #[test]
    fn clear_user_reassignment_clears_structural_owner() {
        let db = test_db();
        make_ctx!(ctx);
        seed_owner_fixture(&db);

        apply_update_action(&ctx, &db, owner_request(Some("Alex Chen"), None)).unwrap();
        apply_update_action(&ctx, &db, owner_request(None, Some(true))).unwrap();

        let (owner_raw, owner_entity_id, owner_confidence, owner_source, _) = owner_state(&db);
        assert_eq!(owner_raw, None);
        assert_eq!(owner_entity_id, None);
        assert_eq!(owner_confidence, None);
        assert_eq!(owner_source.as_deref(), Some("user_reassigned"));
    }

    #[test]
    fn clear_then_user_reassignment_resolves_against_new_raw_owner() {
        let db = test_db();
        make_ctx!(ctx);
        seed_owner_fixture(&db);

        apply_update_action(&ctx, &db, owner_request(Some("Alex Chen"), None)).unwrap();
        apply_update_action(&ctx, &db, owner_request(None, Some(true))).unwrap();
        apply_update_action(&ctx, &db, owner_request(Some("Jamie Lee"), None)).unwrap();

        let (owner_raw, owner_entity_id, owner_confidence, owner_source, _) = owner_state(&db);
        assert_eq!(owner_raw.as_deref(), Some("Jamie Lee"));
        assert_eq!(owner_entity_id.as_deref(), Some("p-jamie"));
        assert_eq!(owner_confidence, Some(0.96));
        assert_eq!(owner_source.as_deref(), Some("user_reassigned"));
    }
}
