// Actions service — extracted from commands.rs
// Business logic for action status transitions with signal emission.

use std::collections::HashSet;

use crate::commands::{CreateActionRequest, UpdateActionRequest};
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
pub fn complete_action(db: &ActionDb, id: &str) -> Result<(), String> {
    let action = db.get_action_by_id(id).ok().flatten();
    db.complete_action(id).map_err(|e| e.to_string())?;

    if let Some(ref action) = action {
        let (entity_type, entity_id) = action_entity_info(action, id);
        let _ = crate::signals::bus::emit_signal(
            db,
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
pub fn reopen_action(db: &ActionDb, id: &str) -> Result<(), String> {
    let action = db.get_action_by_id(id).ok().flatten();
    db.reopen_action(id).map_err(|e| e.to_string())?;

    if let Some(ref action) = action {
        let (entity_type, entity_id) = action_entity_info(action, id);
        let _ = crate::signals::bus::emit_signal(
            db,
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

/// Accept a proposed action, moving it to pending (I256).
pub fn accept_proposed_action(db: &ActionDb, id: &str) -> Result<(), String> {
    let action = db.get_action_by_id(id).ok().flatten();
    db.accept_proposed_action(id).map_err(|e| e.to_string())?;

    if let Some(ref action) = action {
        let (entity_type, entity_id) = action_entity_info(action, id);
        let _ = crate::signals::bus::emit_signal(
            db,
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

/// Reject a proposed action by archiving it (I256).
pub fn reject_proposed_action(db: &ActionDb, id: &str) -> Result<(), String> {
    let action = db.get_action_by_id(id).ok().flatten();
    db.reject_proposed_action(id).map_err(|e| e.to_string())?;

    // Emit rejection signal for correction learning (I307)
    if let Some(ref action) = action {
        let (entity_type, entity_id) = action_entity_info(action, id);
        let _ = crate::signals::bus::emit_signal(
            db,
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
    }

    Ok(())
}

/// Cycle an action's priority with signal emission.
pub fn update_action_priority(db: &ActionDb, id: &str, priority: &str) -> Result<(), String> {
    let action = db.get_action_by_id(id).ok().flatten();
    db.update_action_priority(id, priority)
        .map_err(|e| e.to_string())?;

    if let Some(ref action) = action {
        let (entity_type, entity_id) = action_entity_info(action, id);
        let _ = crate::signals::bus::emit_signal(
            db,
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

/// Get all actions with full context (merges briefing JSON + SQLite).
pub fn get_all_actions(state: &AppState) -> ActionsResult {
    let config = match state.config.read() {
        Ok(guard) => match guard.clone() {
            Some(c) => c,
            None => {
                return ActionsResult::Error {
                    message: "No configuration loaded".to_string(),
                }
            }
        },
        Err(_) => {
            return ActionsResult::Error {
                message: "Internal error: config lock poisoned".to_string(),
            }
        }
    };

    let workspace = std::path::Path::new(&config.workspace_path);
    let today_dir = workspace.join("_today");

    let mut actions = crate::json_loader::load_actions_json(&today_dir).unwrap_or_default();

    // Merge non-briefing actions from SQLite (same logic as dashboard)
    if let Ok(db_guard) = state.db.lock() {
        if let Some(db) = db_guard.as_ref() {
            if let Ok(db_actions) = db.get_non_briefing_pending_actions() {
                let json_titles: HashSet<String> = actions
                    .iter()
                    .map(|a| a.title.to_lowercase().trim().to_string())
                    .collect();
                for dba in db_actions {
                    if !json_titles.contains(dba.title.to_lowercase().trim()) {
                        let priority = match dba.priority.as_str() {
                            "P1" => Priority::P1,
                            "P3" => Priority::P3,
                            _ => Priority::P2,
                        };
                        actions.push(Action {
                            id: dba.id,
                            title: dba.title,
                            account: dba.account_id,
                            due_date: dba.due_date,
                            priority,
                            status: crate::types::ActionStatus::Pending,
                            is_overdue: None,
                            context: dba.context,
                            source: dba.source_label,
                            days_overdue: None,
                        });
                    }
                }
            }
        }
    }

    if actions.is_empty() {
        ActionsResult::Empty {
            message: "No actions yet. Actions appear after your first briefing.".to_string(),
        }
    } else {
        ActionsResult::Success { data: actions }
    }
}

/// Create a new action with validation.
pub fn create_action(
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
    let priority = priority.unwrap_or_else(|| "P2".to_string());
    crate::util::validate_enum_string(priority.as_str(), "priority", &["P1", "P2", "P3"])?;
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
        status: "pending".to_string(),
        created_at: now.clone(),
        due_date,
        completed_at: None,
        account_id,
        project_id,
        source_type: Some("manual".to_string()),
        source_id: None,
        source_label,
        context,
        waiting_on: None,
        updated_at: now,
        person_id,
        account_name: None,
    };

    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    db.upsert_action(&action).map_err(|e| e.to_string())?;
    Ok(id)
}

/// Update arbitrary fields on an existing action (I128).
pub fn update_action(
    request: UpdateActionRequest,
    state: &AppState,
) -> Result<(), String> {
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
        crate::util::validate_enum_string(p.as_str(), "priority", &["P1", "P2", "P3"])?;
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

    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let mut action = db
        .get_action_by_id(&id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Action not found: {id}"))?;

    if let Some(t) = title {
        action.title = t;
    }
    if let Some(p) = priority {
        action.priority = p;
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
}
