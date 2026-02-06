use std::path::Path;
use std::sync::Arc;

use tauri::State;

use crate::executor::request_workflow_execution;
use crate::json_loader::{
    load_actions_json, load_emails_json, load_prep_json, load_schedule_json,
};
use crate::parser::{count_inbox, list_inbox_files};
use crate::scheduler::get_next_run_time as scheduler_get_next_run_time;
use crate::state::{reload_config, AppState};
use crate::types::{
    Action, Config, DashboardData, DayStats, ExecutionRecord, FocusData, FullMeetingPrep,
    InboxFile, MeetingType, WeekOverview, WorkflowId, WorkflowStatus,
};
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

/// Get dashboard data from workspace _today/data/ JSON files
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

    // Check for data directory
    let data_dir = today_dir.join("data");
    if !data_dir.exists() {
        return DashboardResult::Empty {
            message: "No data found. Run /today to generate your daily briefing.".to_string(),
        };
    }

    // Load from JSON
    let (overview, meetings) = match load_schedule_json(&today_dir) {
        Ok(data) => data,
        Err(e) => {
            return DashboardResult::Error {
                message: format!("Failed to load schedule: {}", e),
            }
        }
    };

    let actions = load_actions_json(&today_dir).unwrap_or_default();
    let emails = load_emails_json(&today_dir).ok().filter(|e| !e.is_empty());

    // Calculate stats
    let inbox_count = count_inbox(workspace);
    let stats = DayStats {
        total_meetings: meetings.len(),
        customer_meetings: meetings
            .iter()
            .filter(|m| matches!(m.meeting_type, MeetingType::Customer | MeetingType::Qbr))
            .count(),
        actions_due: actions.len(),
        inbox_count,
    };

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

// =============================================================================
// Meeting Prep Command
// =============================================================================

/// Result type for meeting prep
#[derive(Debug, serde::Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum MeetingPrepResult {
    Success { data: FullMeetingPrep },
    NotFound { message: String },
    Error { message: String },
}

/// Get full meeting prep data by filename
#[tauri::command]
pub fn get_meeting_prep(
    prep_file: String,
    state: State<Arc<AppState>>,
) -> MeetingPrepResult {
    // Get config
    let config = match state.config.lock() {
        Ok(guard) => match guard.clone() {
            Some(c) => c,
            None => {
                return MeetingPrepResult::Error {
                    message: "No configuration loaded".to_string(),
                }
            }
        },
        Err(_) => {
            return MeetingPrepResult::Error {
                message: "Internal error: config lock poisoned".to_string(),
            }
        }
    };

    let workspace = Path::new(&config.workspace_path);
    let today_dir = workspace.join("_today");

    match load_prep_json(&today_dir, &prep_file) {
        Ok(prep) => MeetingPrepResult::Success { data: prep },
        Err(e) => MeetingPrepResult::NotFound {
            message: format!("Prep not found: {}", e),
        },
    }
}

// =============================================================================
// Week Overview Command
// =============================================================================

/// Result type for week data
#[derive(Debug, serde::Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum WeekResult {
    Success { data: WeekOverview },
    NotFound { message: String },
    Error { message: String },
}

/// Get week overview data
#[tauri::command]
pub fn get_week_data(state: State<Arc<AppState>>) -> WeekResult {
    // Get config
    let config = match state.config.lock() {
        Ok(guard) => match guard.clone() {
            Some(c) => c,
            None => {
                return WeekResult::Error {
                    message: "No configuration loaded".to_string(),
                }
            }
        },
        Err(_) => {
            return WeekResult::Error {
                message: "Internal error: config lock poisoned".to_string(),
            }
        }
    };

    let _workspace = Path::new(&config.workspace_path);

    // TODO: Implement week JSON loading
    WeekResult::NotFound {
        message: "Week overview not yet implemented for JSON format.".to_string(),
    }
}

// =============================================================================
// Focus Data Command
// =============================================================================

/// Result type for focus data
#[derive(Debug, serde::Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum FocusResult {
    Success { data: FocusData },
    NotFound { message: String },
    Error { message: String },
}

/// Get focus/priority data
#[tauri::command]
pub fn get_focus_data(state: State<Arc<AppState>>) -> FocusResult {
    // Get config
    let config = match state.config.lock() {
        Ok(guard) => match guard.clone() {
            Some(c) => c,
            None => {
                return FocusResult::Error {
                    message: "No configuration loaded".to_string(),
                }
            }
        },
        Err(_) => {
            return FocusResult::Error {
                message: "Internal error: config lock poisoned".to_string(),
            }
        }
    };

    let _workspace = Path::new(&config.workspace_path);

    // TODO: Implement focus JSON loading
    FocusResult::NotFound {
        message: "Focus data not yet implemented for JSON format.".to_string(),
    }
}

// =============================================================================
// Actions Command
// =============================================================================

/// Result type for all actions
#[derive(Debug, serde::Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum ActionsResult {
    Success { data: Vec<Action> },
    Empty { message: String },
    Error { message: String },
}

/// Get all actions with full context
#[tauri::command]
pub fn get_all_actions(state: State<Arc<AppState>>) -> ActionsResult {
    // Get config
    let config = match state.config.lock() {
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

    let workspace = Path::new(&config.workspace_path);
    let today_dir = workspace.join("_today");

    match load_actions_json(&today_dir) {
        Ok(actions) => {
            if actions.is_empty() {
                ActionsResult::Empty {
                    message: "No actions found.".to_string(),
                }
            } else {
                ActionsResult::Success { data: actions }
            }
        }
        Err(e) => ActionsResult::Error {
            message: format!("Failed to load actions: {}", e),
        },
    }
}

// =============================================================================
// Inbox Command
// =============================================================================

/// Result type for inbox files
#[derive(Debug, serde::Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum InboxResult {
    Success {
        files: Vec<InboxFile>,
        count: usize,
    },
    Empty {
        message: String,
        files: Vec<InboxFile>,
        count: usize,
    },
    Error {
        message: String,
        files: Vec<InboxFile>,
        count: usize,
    },
}

/// Get files from the _inbox/ directory
#[tauri::command]
pub fn get_inbox_files(state: State<Arc<AppState>>) -> InboxResult {
    let config = match state.config.lock() {
        Ok(guard) => match guard.clone() {
            Some(c) => c,
            None => {
                return InboxResult::Error {
                    message: "No configuration loaded".to_string(),
                    files: Vec::new(),
                    count: 0,
                }
            }
        },
        Err(_) => {
            return InboxResult::Error {
                message: "Internal error: config lock poisoned".to_string(),
                files: Vec::new(),
                count: 0,
            }
        }
    };

    let workspace = Path::new(&config.workspace_path);
    let files = list_inbox_files(workspace);
    let count = files.len();

    if files.is_empty() {
        InboxResult::Empty {
            message: "Inbox is clear".to_string(),
            files,
            count,
        }
    } else {
        InboxResult::Success { files, count }
    }
}

/// Process a single inbox file (classify, route, log).
#[tauri::command]
pub fn process_inbox_file(
    filename: String,
    state: State<Arc<AppState>>,
) -> crate::processor::ProcessingResult {
    let config = match state.config.lock() {
        Ok(guard) => match guard.clone() {
            Some(c) => c,
            None => {
                return crate::processor::ProcessingResult::Error {
                    message: "No configuration loaded".to_string(),
                }
            }
        },
        Err(_) => {
            return crate::processor::ProcessingResult::Error {
                message: "Internal error".to_string(),
            }
        }
    };

    let workspace = Path::new(&config.workspace_path);
    let db_guard = state.db.lock().ok();
    let db_ref = db_guard.as_ref().and_then(|g| g.as_ref());

    crate::processor::process_file(workspace, &filename, db_ref)
}

/// Process all inbox files (batch).
#[tauri::command]
pub fn process_all_inbox(
    state: State<Arc<AppState>>,
) -> Vec<(String, crate::processor::ProcessingResult)> {
    let config = match state.config.lock() {
        Ok(guard) => match guard.clone() {
            Some(c) => c,
            None => return Vec::new(),
        },
        Err(_) => return Vec::new(),
    };

    let workspace = Path::new(&config.workspace_path);
    let db_guard = state.db.lock().ok();
    let db_ref = db_guard.as_ref().and_then(|g| g.as_ref());

    crate::processor::process_all(workspace, db_ref)
}

/// Process an inbox file with AI enrichment via Claude Code.
///
/// Used for files that the quick classifier couldn't categorize.
#[tauri::command]
pub fn enrich_inbox_file(
    filename: String,
    state: State<Arc<AppState>>,
) -> crate::processor::enrich::EnrichResult {
    let config = match state.config.lock() {
        Ok(guard) => match guard.clone() {
            Some(c) => c,
            None => {
                return crate::processor::enrich::EnrichResult::Error {
                    message: "No configuration loaded".to_string(),
                }
            }
        },
        Err(_) => {
            return crate::processor::enrich::EnrichResult::Error {
                message: "Internal error".to_string(),
            }
        }
    };

    let workspace = Path::new(&config.workspace_path);
    let db_guard = state.db.lock().ok();
    let db_ref = db_guard.as_ref().and_then(|g| g.as_ref());

    crate::processor::enrich::enrich_file(workspace, &filename, db_ref)
}

/// Get the content of a specific inbox file for preview
#[tauri::command]
pub fn get_inbox_file_content(
    filename: String,
    state: State<Arc<AppState>>,
) -> Result<String, String> {
    let config = state
        .config
        .lock()
        .map_err(|_| "Lock poisoned")?
        .clone()
        .ok_or("No configuration loaded")?;

    let workspace = Path::new(&config.workspace_path);
    let file_path = workspace.join("_inbox").join(&filename);

    // Prevent path traversal
    if !file_path.starts_with(workspace.join("_inbox")) {
        return Err("Invalid filename".to_string());
    }

    if !file_path.exists() {
        return Err(format!("File not found: {}", filename));
    }

    std::fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read file: {}", e))
}

// =============================================================================
// Inbox Drop Zone
// =============================================================================

/// Copy files into the _inbox/ directory (used by drop zone).
///
/// Accepts absolute file paths from the drag-drop event.
/// Returns the number of files successfully copied.
#[tauri::command]
pub fn copy_to_inbox(
    paths: Vec<String>,
    state: State<Arc<AppState>>,
) -> Result<usize, String> {
    let config = state
        .config
        .lock()
        .map_err(|_| "Lock poisoned")?
        .clone()
        .ok_or("No configuration loaded")?;

    let workspace = Path::new(&config.workspace_path);
    let inbox_dir = workspace.join("_inbox");

    // Ensure _inbox/ exists
    if !inbox_dir.exists() {
        std::fs::create_dir_all(&inbox_dir)
            .map_err(|e| format!("Failed to create _inbox: {}", e))?;
    }

    let mut copied = 0;

    for path_str in &paths {
        let source = Path::new(path_str);

        // Skip directories
        if !source.is_file() {
            continue;
        }

        let filename = match source.file_name() {
            Some(name) => name.to_owned(),
            None => continue,
        };

        let mut dest = inbox_dir.join(&filename);

        // Handle duplicates: append (1), (2), etc.
        if dest.exists() {
            let stem = dest.file_stem().and_then(|s| s.to_str()).unwrap_or("file").to_string();
            let ext = dest.extension().and_then(|e| e.to_str()).unwrap_or("").to_string();
            let mut counter = 1;
            loop {
                let new_name = if ext.is_empty() {
                    format!("{} ({})", stem, counter)
                } else {
                    format!("{} ({}).{}", stem, counter, ext)
                };
                dest = inbox_dir.join(new_name);
                if !dest.exists() {
                    break;
                }
                counter += 1;
            }
        }

        match std::fs::copy(source, &dest) {
            Ok(_) => {
                log::info!("Copied '{}' to inbox", filename.to_string_lossy());
                copied += 1;
            }
            Err(e) => {
                log::warn!("Failed to copy '{}' to inbox: {}", path_str, e);
            }
        }
    }

    Ok(copied)
}

// =============================================================================
// Emails Command
// =============================================================================

/// Result type for email summary
#[derive(Debug, serde::Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum EmailsResult {
    Success { data: Vec<crate::types::Email> },
    NotFound { message: String },
    Error { message: String },
}

/// Get all emails
#[tauri::command]
pub fn get_all_emails(state: State<Arc<AppState>>) -> EmailsResult {
    // Get config
    let config = match state.config.lock() {
        Ok(guard) => match guard.clone() {
            Some(c) => c,
            None => {
                return EmailsResult::Error {
                    message: "No configuration loaded".to_string(),
                }
            }
        },
        Err(_) => {
            return EmailsResult::Error {
                message: "Internal error: config lock poisoned".to_string(),
            }
        }
    };

    let workspace = Path::new(&config.workspace_path);
    let today_dir = workspace.join("_today");

    match load_emails_json(&today_dir) {
        Ok(emails) => {
            if emails.is_empty() {
                EmailsResult::NotFound {
                    message: "No emails found.".to_string(),
                }
            } else {
                EmailsResult::Success { data: emails }
            }
        }
        Err(e) => EmailsResult::NotFound {
            message: format!("No emails: {}", e),
        },
    }
}

/// Set user profile (customer-success or general)
#[tauri::command]
pub fn set_profile(
    profile: String,
    state: State<Arc<AppState>>,
) -> Result<Config, String> {
    // Validate profile value
    if profile != "customer-success" && profile != "general" {
        return Err(format!("Invalid profile: {}. Must be 'customer-success' or 'general'.", profile));
    }

    // Load current config
    let mut config = state
        .config
        .lock()
        .map_err(|_| "Lock poisoned")?
        .clone()
        .ok_or("No configuration loaded")?;

    // Update profile
    config.profile = profile;

    // Write back to disk
    let home = dirs::home_dir().ok_or("Could not find home directory")?;
    let config_path = home.join(".daybreak").join("config.json");
    let content = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    std::fs::write(&config_path, content)
        .map_err(|e| format!("Failed to write config: {}", e))?;

    // Update in-memory state
    let mut guard = state.config.lock().map_err(|_| "Lock poisoned")?;
    *guard = Some(config.clone());

    Ok(config)
}

/// List available meeting prep files
#[tauri::command]
pub fn list_meeting_preps(state: State<Arc<AppState>>) -> Result<Vec<String>, String> {
    // Get config
    let config = state
        .config
        .lock()
        .map_err(|_| "Lock poisoned")?
        .clone()
        .ok_or("No configuration loaded")?;

    let workspace = Path::new(&config.workspace_path);
    let preps_dir = workspace.join("_today").join("data").join("preps");

    if !preps_dir.exists() {
        return Ok(Vec::new());
    }

    let mut preps = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&preps_dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.ends_with(".json") {
                    preps.push(name.trim_end_matches(".json").to_string());
                }
            }
        }
    }

    Ok(preps)
}

// =============================================================================
// SQLite Database Commands
// =============================================================================

/// Get actions from the SQLite database, filtered by due date window.
///
/// Returns pending actions where `due_date` is within `days_ahead` days (default 7)
/// or where `due_date` is NULL. Overdue actions appear first.
#[tauri::command]
pub fn get_actions_from_db(
    days_ahead: Option<i32>,
    state: State<Arc<AppState>>,
) -> Result<Vec<crate::db::DbAction>, String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    db.get_due_actions(days_ahead.unwrap_or(7))
        .map_err(|e| e.to_string())
}

/// Mark an action as completed in the SQLite database.
///
/// Sets `status = 'completed'` and `completed_at` to the current UTC timestamp.
#[tauri::command]
pub fn complete_action(
    id: String,
    state: State<Arc<AppState>>,
) -> Result<(), String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    db.complete_action(&id).map_err(|e| e.to_string())
}

/// Get recent meeting history for an account from the SQLite database.
///
/// Returns meetings within `lookback_days` (default 30), limited to `limit` results (default 3).
#[tauri::command]
pub fn get_meeting_history(
    account_id: String,
    lookback_days: Option<i32>,
    limit: Option<i32>,
    state: State<Arc<AppState>>,
) -> Result<Vec<crate::db::DbMeeting>, String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    db.get_meeting_history(&account_id, lookback_days.unwrap_or(30), limit.unwrap_or(3))
        .map_err(|e| e.to_string())
}
