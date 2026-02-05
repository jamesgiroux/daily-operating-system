use std::path::Path;
use std::sync::Arc;

use tauri::State;

use crate::executor::request_workflow_execution;
use crate::parser::{calculate_stats, count_inbox, parse_actions, parse_emails, parse_meetings, parse_overview};
use crate::scheduler::get_next_run_time as scheduler_get_next_run_time;
use crate::state::{reload_config, AppState};
use crate::types::{Config, DashboardData, ExecutionRecord, WorkflowId, WorkflowStatus};
use crate::SchedulerSender;

/// Result type for dashboard data loading
#[derive(Debug, serde::Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum DashboardResult {
    Success { data: DashboardData },
    Empty { message: String },
    Error { message: String },
}

/// Get current configuration
#[tauri::command]
pub fn get_config(state: State<Arc<AppState>>) -> Result<Config, String> {
    let guard = state.config.lock().map_err(|_| "Lock poisoned")?;
    guard.clone().ok_or_else(|| {
        "No configuration loaded. Create ~/.daybreak/config.json".to_string()
    })
}

/// Reload configuration from disk
#[tauri::command]
pub fn reload_configuration(state: State<Arc<AppState>>) -> Result<Config, String> {
    reload_config(&state)
}

/// Get dashboard data from workspace _today/ files
#[tauri::command]
pub fn get_dashboard_data(state: State<Arc<AppState>>) -> DashboardResult {
    // Get config
    let config = match state.config.lock() {
        Ok(guard) => match guard.clone() {
            Some(c) => c,
            None => {
                return DashboardResult::Error {
                    message: "No configuration. Create ~/.daybreak/config.json with { \"workspacePath\": \"/path/to/workspace\" }".to_string(),
                }
            }
        },
        Err(_) => {
            return DashboardResult::Error {
                message: "Internal error: config lock poisoned".to_string(),
            }
        }
    };

    let workspace = Path::new(&config.workspace_path);
    let today_dir = workspace.join("_today");

    // Check if _today directory exists
    if !today_dir.exists() {
        return DashboardResult::Empty {
            message: "No briefing yet. Run /today to generate your daily overview.".to_string(),
        };
    }

    // Parse overview
    let overview_path = today_dir.join("overview.md");
    let overview = if overview_path.exists() {
        match parse_overview(&overview_path) {
            Ok(o) => o,
            Err(e) => {
                return DashboardResult::Error {
                    message: format!("Failed to parse overview: {}", e),
                }
            }
        }
    } else {
        return DashboardResult::Empty {
            message: "No overview found. Run /today to generate your daily briefing.".to_string(),
        };
    };

    // Parse meetings (optional)
    let meetings_path = today_dir.join("meetings.md");
    let meetings = if meetings_path.exists() {
        parse_meetings(&meetings_path).unwrap_or_default()
    } else {
        Vec::new()
    };

    // Parse actions (optional)
    let actions_path = today_dir.join("actions.md");
    let actions = if actions_path.exists() {
        parse_actions(&actions_path).unwrap_or_default()
    } else {
        Vec::new()
    };

    // Parse emails (optional)
    let emails_path = today_dir.join("emails.md");
    let emails = if emails_path.exists() {
        let parsed = parse_emails(&emails_path).unwrap_or_default();
        if parsed.is_empty() { None } else { Some(parsed) }
    } else {
        None
    };

    // Count inbox
    let inbox_count = count_inbox(workspace);

    // Calculate stats
    let stats = calculate_stats(&meetings, &actions, inbox_count);

    DashboardResult::Success {
        data: DashboardData {
            overview,
            stats,
            meetings,
            actions,
            emails,
        },
    }
}

/// Trigger a workflow execution
#[tauri::command]
pub fn run_workflow(
    workflow: String,
    sender: State<SchedulerSender>,
) -> Result<String, String> {
    let workflow_id: WorkflowId = workflow
        .parse()
        .map_err(|e: String| e)?;

    request_workflow_execution(&sender.0, workflow_id)?;

    Ok(format!("Workflow '{}' queued for execution", workflow))
}

/// Get the current status of a workflow
#[tauri::command]
pub fn get_workflow_status(
    workflow: String,
    state: State<Arc<AppState>>,
) -> Result<WorkflowStatus, String> {
    let workflow_id: WorkflowId = workflow.parse()?;
    Ok(state.get_workflow_status(workflow_id))
}

/// Get execution history
#[tauri::command]
pub fn get_execution_history(
    limit: Option<usize>,
    state: State<Arc<AppState>>,
) -> Vec<ExecutionRecord> {
    state.get_execution_history(limit.unwrap_or(10))
}

/// Get the next scheduled run time for a workflow
#[tauri::command]
pub fn get_next_run_time(
    workflow: String,
    state: State<Arc<AppState>>,
) -> Result<Option<String>, String> {
    let workflow_id: WorkflowId = workflow.parse()?;

    let config = state
        .config
        .lock()
        .map_err(|_| "Lock poisoned")?
        .clone()
        .ok_or("No configuration loaded")?;

    let entry = match workflow_id {
        WorkflowId::Today => &config.schedules.today,
        WorkflowId::Archive => &config.schedules.archive,
    };

    if !entry.enabled {
        return Ok(None);
    }

    scheduler_get_next_run_time(entry)
        .map(|dt| Some(dt.to_rfc3339()))
        .map_err(|e| e.to_string())
}
