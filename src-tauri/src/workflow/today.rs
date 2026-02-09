//! Today workflow implementation
//!
//! Per-operation pipeline (ADR-0042, ADR-0049):
//! 1. Rust-native prepare — fetch calendar/emails, classify, write directive
//! 2. Rust-native mechanical delivery — schedule, actions, preps, emails
//! 3. AI enrichment — Claude Code enriches emails + briefing narrative
//!
//! Post-processing:
//! - sync_actions_to_db() - Upsert actions from JSON into SQLite

use std::path::Path;

use chrono::Utc;

use crate::db::ActionDb;
use crate::json_loader::load_actions_json;
use crate::types::WorkflowId;
use crate::workflow::Workflow;

/// The /today workflow configuration
pub const TODAY_WORKFLOW: Workflow = Workflow {
    id: WorkflowId::Today,
    claude_command: "/today",
};

/// Sync actions from _today/data/actions.json into the SQLite database.
///
/// Merge logic: new actions are inserted, existing actions are updated,
/// but user-set `completed` status is never overwritten by the briefing.
pub fn sync_actions_to_db(workspace: &Path, db: &ActionDb) -> Result<usize, String> {
    let today_dir = workspace.join("_today");
    let actions = load_actions_json(&today_dir)?;

    if actions.is_empty() {
        log::info!("No actions to sync to database");
        return Ok(0);
    }

    let now = Utc::now().to_rfc3339();
    let mut synced = 0;

    for action in &actions {
        let db_action = crate::db::DbAction {
            id: action.id.clone(),
            title: action.title.clone(),
            priority: format!("{:?}", action.priority),
            status: format!("{:?}", action.status).to_lowercase(),
            created_at: now.clone(),
            due_date: action.due_date.clone(),
            completed_at: None,
            account_id: action.account.clone(),
            project_id: None,
            source_type: action.source.as_ref().map(|_| "briefing".to_string()),
            source_id: None,
            source_label: action.source.clone(),
            context: action.context.clone(),
            waiting_on: None,
            updated_at: now.clone(),
            person_id: None,
        };

        // Use upsert — this preserves user-set completed status because the
        // ON CONFLICT clause only updates fields from the briefing. If the user
        // already marked an action as completed, the briefing's "pending" won't
        // overwrite it because we check before upserting.
        if let Err(e) = db.upsert_action_if_not_completed(&db_action) {
            log::warn!("Failed to upsert action {}: {}", action.id, e);
        } else {
            synced += 1;
        }
    }

    log::info!("Synced {} actions to database", synced);
    Ok(synced)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_today_workflow_config() {
        assert_eq!(TODAY_WORKFLOW.claude_command, "/today");
    }
}
