//! Today workflow implementation
//!
//! Three-phase workflow for daily briefing:
//! 1. prepare_today.py - Fetch calendar, emails, generate directive
//! 2. /today - Claude enriches with AI synthesis
//! 3. deliver_today.py - Write final files to _today/
//!
//! Post-processing (Phase 2.1):
//! 4. generate_json.py - Convert markdown to JSON for dashboard
//! 5. sync_actions_to_db() - Upsert actions into SQLite

use std::path::Path;
use std::process::Command;

use chrono::Utc;

use crate::db::ActionDb;
use crate::json_loader::load_actions_json;
use crate::types::WorkflowId;
use crate::workflow::Workflow;

/// The /today workflow configuration
pub const TODAY_WORKFLOW: Workflow = Workflow {
    id: WorkflowId::Today,
    prepare_script: "prepare_today.py",
    claude_command: "/today",
    deliver_script: "deliver_today.py",
};

/// Timeout for JSON generation script (30 seconds — deterministic, fast)
const JSON_GEN_TIMEOUT_SECS: u64 = 30;

/// Run generate_json.py to convert _today/ markdown into _today/data/*.json
///
/// This is a post-processing step after the three-phase workflow completes.
/// Failure is non-fatal — the dashboard falls back to empty state.
pub fn run_json_generation(workspace: &Path) -> Result<(), String> {
    // Look for generate_json.py in known locations
    let script = find_generate_json_script(workspace)?;

    log::info!(
        "Running JSON generation: {} {}",
        script.display(),
        workspace.display()
    );

    let output = Command::new("python3")
        .arg(&script)
        .arg(workspace)
        .current_dir(workspace)
        .env("WORKSPACE", workspace)
        .output()
        .map_err(|e| format!("Failed to run generate_json.py: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "generate_json.py failed (exit {}): {}",
            output.status.code().unwrap_or(-1),
            stderr
        ));
    }

    log::info!("JSON generation completed successfully");
    Ok(())
}

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

/// Find the generate_json.py script in known locations
fn find_generate_json_script(workspace: &Path) -> Result<std::path::PathBuf, String> {
    // 1. Workspace _tools/ override
    let workspace_script = workspace.join("_tools").join("generate_json.py");
    if workspace_script.exists() {
        return Ok(workspace_script);
    }

    // 2. Templates directory (development layout)
    let templates_script = workspace
        .join("templates")
        .join("scripts")
        .join("daily")
        .join("generate_json.py");
    if templates_script.exists() {
        return Ok(templates_script);
    }

    // 3. Relative to the repo root (check parent dirs for repo structure)
    // In development, workspace is ~/Documents/VIP but the script is in the repo
    if let Some(home) = dirs::home_dir() {
        let repo_script = home
            .join("Documents")
            .join("daily-operating-system-daybreak")
            .join("templates")
            .join("scripts")
            .join("daily")
            .join("generate_json.py");
        if repo_script.exists() {
            return Ok(repo_script);
        }
    }

    Err(format!(
        "generate_json.py not found. Checked: {}, {}",
        workspace_script.display(),
        templates_script.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_today_workflow_config() {
        assert_eq!(TODAY_WORKFLOW.prepare_script, "prepare_today.py");
        assert_eq!(TODAY_WORKFLOW.claude_command, "/today");
        assert_eq!(TODAY_WORKFLOW.deliver_script, "deliver_today.py");
    }
}
