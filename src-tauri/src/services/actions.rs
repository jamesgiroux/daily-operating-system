// Actions service — extracted from commands.rs
// Business logic for action status transitions with signal emission.

use std::collections::HashMap;

use crate::commands::{ActionDetail, ActionListItem, CreateActionRequest, UpdateActionRequest};
use crate::db::ActionDb;
use crate::state::AppState;
use crate::types::{Action, Priority};

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
    db: &ActionDb,
    engine: &crate::signals::propagation::PropagationEngine,
    id: &str,
) -> Result<(), String> {
    let action = db.get_action_by_id(id).ok().flatten();
    db.complete_action(id).map_err(|e| e.to_string())?;

    if let Some(ref action) = action {
        let (entity_type, entity_id) = action_entity_info(action, id);
        let _ = crate::services::signals::emit_and_propagate(
            db,
            engine,
            entity_type,
            &entity_id,
            "action_completed",
            action.source_type.as_deref().unwrap_or("unknown"),
            Some(&format!("{{\"action_id\":\"{}\"}}", id)),
            0.7,
        );
    }

    Ok(())
}

/// Reopen a completed action, setting it back to pending.
pub fn reopen_action(
    db: &ActionDb,
    engine: &crate::signals::propagation::PropagationEngine,
    id: &str,
) -> Result<(), String> {
    let action = db.get_action_by_id(id).ok().flatten();
    db.reopen_action(id).map_err(|e| e.to_string())?;

    if let Some(ref action) = action {
        let (entity_type, entity_id) = action_entity_info(action, id);
        let _ = crate::services::signals::emit_and_propagate(
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

/// Accept a suggested action, moving it to pending (I256).
pub fn accept_suggested_action(
    db: &ActionDb,
    engine: &crate::signals::propagation::PropagationEngine,
    id: &str,
) -> Result<(), String> {
    let action = db.get_action_by_id(id).ok().flatten();
    db.accept_suggested_action(id).map_err(|e| e.to_string())?;

    if let Some(ref action) = action {
        let (entity_type, entity_id) = action_entity_info(action, id);
        let _ = crate::services::signals::emit_and_propagate(
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
    }

    Ok(())
}

/// Reject a suggested action by archiving it (I256).
pub fn reject_suggested_action(
    db: &ActionDb,
    engine: &crate::signals::propagation::PropagationEngine,
    id: &str,
    source: &str,
) -> Result<(), String> {
    let action = db.get_action_by_id(id).ok().flatten();
    db.reject_suggested_action_with_source(id, source)
        .map_err(|e| e.to_string())?;

    // Emit rejection signal for correction learning (I307)
    if let Some(ref action) = action {
        let (entity_type, entity_id) = action_entity_info(action, id);
        let _ = crate::services::signals::emit_and_propagate(
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

        // Record rejection patterns for future suppression (DOS-18)
        if let Err(e) = db.record_rejection_pattern(action) {
            log::warn!("Failed to record rejection pattern: {}", e);
        }
    }

    Ok(())
}

/// Cycle an action's priority with signal emission.
pub fn update_action_priority(
    db: &ActionDb,
    engine: &crate::signals::propagation::PropagationEngine,
    id: &str,
    priority: &str,
) -> Result<(), String> {
    let action = db.get_action_by_id(id).ok().flatten();
    db.update_action_priority(id, priority)
        .map_err(|e| e.to_string())?;

    if let Some(ref action) = action {
        let (entity_type, entity_id) = action_entity_info(action, id);
        let _ = crate::services::signals::emit_and_propagate(
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

/// Get all actions with full context from SQLite (I513 — DB is sole source).
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
    request: CreateActionRequest,
    state: &AppState,
) -> Result<String, String> {
    let CreateActionRequest {
        title,
        priority,
        due_date,
        account_id,
        project_id,
        person_id,
        context,
        source_label,
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

    let now = chrono::Utc::now().to_rfc3339();
    let id = uuid::Uuid::new_v4().to_string();

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
        context,
        waiting_on: None,
        updated_at: now,
        person_id,
        account_name: None,
        next_meeting_title: None,
        next_meeting_start: None,
    };

    let engine = state.signals.engine.clone();
    state
        .db_write(move |db| {
            db.upsert_action(&action).map_err(|e| e.to_string())?;

            // Emit signal for manually created actions
            let (entity_type, entity_id) = action_entity_info(&action, &action.id);
            let _ = crate::services::signals::emit_and_propagate(
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

            Ok(id)
        })
        .await
}

/// Update arbitrary fields on an existing action (I128).
pub async fn update_action(request: UpdateActionRequest, state: &AppState) -> Result<(), String> {
    let UpdateActionRequest {
        id,
        title,
        due_date,
        clear_due_date,
        context,
        clear_context,
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

    crate::util::validate_id_slug(&id, "id")?;
    if let Some(ref p) = priority {
        let pv: i32 = p
            .parse()
            .map_err(|_| format!("Invalid priority: {p}"))?;
        if !(0..=4).contains(&pv) {
            return Err(format!("Priority must be 0-4, got: {pv}"));
        }
    }
    if let Some(ref t) = title {
        crate::util::validate_bounded_string(t, "title", 1, 280)?;
    }
    if let Some(ref d) = due_date {
        crate::util::validate_yyyy_mm_dd(d, "due_date")?;
    }
    if let Some(ref c) = context {
        crate::util::validate_bounded_string(c, "context", 1, 2000)?;
    }
    if let Some(ref s) = source_label {
        crate::util::validate_bounded_string(s, "source_label", 1, 200)?;
    }
    if let Some(ref a) = account_id {
        crate::util::validate_id_slug(a, "account_id")?;
    }
    if let Some(ref p) = project_id {
        crate::util::validate_id_slug(p, "project_id")?;
    }
    if let Some(ref p) = person_id {
        crate::util::validate_id_slug(p, "person_id")?;
    }

    state
        .db_write(move |db| {
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

            action.updated_at = chrono::Utc::now().to_rfc3339();
            db.upsert_action(&action).map_err(|e| e.to_string())
        })
        .await
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

/// Get all suggested (AI-suggested) actions (I256).
pub fn get_suggested_actions(db: &ActionDb) -> Result<Vec<crate::db::DbAction>, String> {
    db.get_suggested_actions().map_err(|e| e.to_string())
}
