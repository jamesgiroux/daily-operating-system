use std::path::Path;
use std::sync::Arc;

use tauri::{Emitter, State};

use crate::executor::request_workflow_execution;
use crate::json_loader::{
    load_actions_json, load_emails_json, load_prep_json, load_schedule_json,
};
use crate::parser::{count_inbox, list_inbox_files};
use crate::scheduler::get_next_run_time as scheduler_get_next_run_time;
use crate::state::{reload_config, AppState};
use crate::types::{
    Action, CalendarEvent, CapturedOutcome, Config, DashboardData, DayStats, ExecutionRecord,
    FocusBlock, FocusData, FullMeetingPrep, GoogleAuthStatus, InboxFile, MeetingType,
    PostMeetingCaptureConfig, WeekOverview, WeekPlanningState, WorkflowId, WorkflowStatus,
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
        "No configuration loaded. Create ~/.dailyos/config.json".to_string()
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
                    message: "No configuration. Create ~/.dailyos/config.json with { \"workspacePath\": \"/path/to/workspace\" }".to_string(),
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
        WorkflowId::InboxBatch => &config.schedules.inbox_batch,
        WorkflowId::Week => &config.schedules.week,
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

    let workspace = Path::new(&config.workspace_path);
    let today_dir = workspace.join("_today");

    match crate::json_loader::load_week_json(&today_dir) {
        Ok(week) => WeekResult::Success { data: week },
        Err(e) => WeekResult::NotFound {
            message: format!("No week data: {}", e),
        },
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
///
/// Runs on a background thread to avoid blocking the main thread.
#[tauri::command]
pub async fn process_inbox_file(
    filename: String,
    state: State<'_, Arc<AppState>>,
) -> Result<crate::processor::ProcessingResult, String> {
    let config = state
        .config
        .lock()
        .map_err(|_| "Internal error")?
        .clone()
        .ok_or("No configuration loaded")?;

    let state = state.inner().clone();
    let workspace_path = config.workspace_path.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let workspace = Path::new(&workspace_path);
        let db_guard = state.db.lock().ok();
        let db_ref = db_guard.as_ref().and_then(|g| g.as_ref());
        crate::processor::process_file(workspace, &filename, db_ref)
    })
    .await
    .map_err(|e| format!("Processing task failed: {}", e))
}

/// Process all inbox files (batch).
///
/// Runs on a background thread to avoid blocking the main thread.
#[tauri::command]
pub async fn process_all_inbox(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<(String, crate::processor::ProcessingResult)>, String> {
    let config = state
        .config
        .lock()
        .map_err(|_| "Internal error")?
        .clone()
        .ok_or("No configuration loaded")?;

    let state = state.inner().clone();
    let workspace_path = config.workspace_path.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let workspace = Path::new(&workspace_path);
        let db_guard = state.db.lock().ok();
        let db_ref = db_guard.as_ref().and_then(|g| g.as_ref());
        crate::processor::process_all(workspace, db_ref)
    })
    .await
    .map_err(|e| format!("Batch processing failed: {}", e))
}

/// Process an inbox file with AI enrichment via Claude Code.
///
/// Used for files that the quick classifier couldn't categorize.
/// Runs on a background thread — Claude Code can take 1-2 minutes.
#[tauri::command]
pub async fn enrich_inbox_file(
    filename: String,
    state: State<'_, Arc<AppState>>,
) -> Result<crate::processor::enrich::EnrichResult, String> {
    let config = state
        .config
        .lock()
        .map_err(|_| "Internal error")?
        .clone()
        .ok_or("No configuration loaded")?;

    let state = state.inner().clone();
    let workspace_path = config.workspace_path.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let workspace = Path::new(&workspace_path);
        let db_guard = state.db.lock().ok();
        let db_ref = db_guard.as_ref().and_then(|g| g.as_ref());
        crate::processor::enrich::enrich_file(workspace, &filename, db_ref)
    })
    .await
    .map_err(|e| format!("AI processing task failed: {}", e))
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

    // Try reading as text; binary files will fail gracefully
    match std::fs::read_to_string(&file_path) {
        Ok(content) => Ok(content),
        Err(_) => {
            // Binary file — return a descriptive message instead of erroring
            let size = std::fs::metadata(&file_path)
                .map(|m| m.len())
                .unwrap_or(0);
            let ext = file_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("unknown");
            Ok(format!(
                "[Binary file — .{} — {} bytes]\n\nThis file cannot be previewed as text. Use \"Process\" to let DailyOS handle it.",
                ext, size
            ))
        }
    }
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
    let config_path = home.join(".dailyos").join("config.json");
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

// =============================================================================
// Phase 3.0: Google Auth Commands
// =============================================================================

/// Get current Google authentication status
#[tauri::command]
pub fn get_google_auth_status(state: State<Arc<AppState>>) -> GoogleAuthStatus {
    state
        .google_auth
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or(GoogleAuthStatus::NotConfigured)
}

/// Start Google OAuth flow
#[tauri::command]
pub async fn start_google_auth(
    state: State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<GoogleAuthStatus, String> {
    let config = state
        .config
        .lock()
        .map_err(|_| "Lock poisoned")?
        .clone()
        .ok_or("No configuration loaded")?;

    let workspace_path = config.workspace_path.clone();

    // Run the blocking Python subprocess off the main thread
    let email = tauri::async_runtime::spawn_blocking(move || {
        let workspace = std::path::Path::new(&workspace_path);
        crate::google::start_auth(workspace)
    })
    .await
    .map_err(|e| format!("Auth task failed: {}", e))?
    .map_err(|e| e)?;

    let new_status = GoogleAuthStatus::Authenticated {
        email: email.clone(),
    };

    // Update state
    if let Ok(mut guard) = state.google_auth.lock() {
        *guard = new_status.clone();
    }

    // Emit event
    let _ = app_handle.emit("google-auth-changed", &new_status);

    Ok(new_status)
}

/// Disconnect Google account
#[tauri::command]
pub fn disconnect_google(
    state: State<Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    crate::google::disconnect()?;

    let new_status = GoogleAuthStatus::NotConfigured;

    // Update state
    if let Ok(mut guard) = state.google_auth.lock() {
        *guard = new_status.clone();
    }

    // Clear calendar events
    if let Ok(mut guard) = state.calendar_events.lock() {
        guard.clear();
    }

    // Emit event
    let _ = app_handle.emit("google-auth-changed", &new_status);

    Ok(())
}

// =============================================================================
// Phase 3A: Calendar Commands
// =============================================================================

/// Get calendar events from the polling cache
#[tauri::command]
pub fn get_calendar_events(state: State<Arc<AppState>>) -> Vec<CalendarEvent> {
    state
        .calendar_events
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_default()
}

/// Get the currently active meeting (if any)
#[tauri::command]
pub fn get_current_meeting(state: State<Arc<AppState>>) -> Option<CalendarEvent> {
    let now = chrono::Utc::now();
    state
        .calendar_events
        .lock()
        .ok()
        .and_then(|guard| {
            guard
                .iter()
                .find(|e| e.start <= now && e.end > now && !e.is_all_day)
                .cloned()
        })
}

/// Get the next upcoming meeting
#[tauri::command]
pub fn get_next_meeting(state: State<Arc<AppState>>) -> Option<CalendarEvent> {
    let now = chrono::Utc::now();
    state
        .calendar_events
        .lock()
        .ok()
        .and_then(|guard| {
            guard
                .iter()
                .filter(|e| e.start > now && !e.is_all_day)
                .min_by_key(|e| e.start)
                .cloned()
        })
}

// =============================================================================
// Phase 3B: Post-Meeting Capture Commands
// =============================================================================

/// Capture meeting outcomes (wins, risks, actions)
#[tauri::command]
pub fn capture_meeting_outcome(
    outcome: CapturedOutcome,
    state: State<Arc<AppState>>,
) -> Result<(), String> {
    let config = state
        .config
        .lock()
        .map_err(|_| "Lock poisoned")?
        .clone()
        .ok_or("No configuration loaded")?;

    let workspace = std::path::Path::new(&config.workspace_path);

    // Mark as captured
    if let Ok(mut guard) = state.capture_captured.lock() {
        guard.insert(outcome.meeting_id.clone());
    }

    // Persist actions to SQLite
    let db_guard = state.db.lock().ok();
    let db_ref = db_guard.as_ref().and_then(|g| g.as_ref());

    if let Some(db) = db_ref {
        for action in &outcome.actions {
            let now = chrono::Utc::now().to_rfc3339();
            let db_action = crate::db::DbAction {
                id: uuid::Uuid::new_v4().to_string(),
                title: action.title.clone(),
                priority: "P2".to_string(),
                status: "pending".to_string(),
                created_at: now.clone(),
                due_date: action.due_date.clone(),
                completed_at: None,
                account_id: outcome.account.clone(),
                project_id: None,
                source_type: Some("post_meeting".to_string()),
                source_id: Some(outcome.meeting_id.clone()),
                source_label: Some(outcome.meeting_title.clone()),
                context: action.owner.clone(),
                waiting_on: None,
                updated_at: now,
            };
            if let Err(e) = db.upsert_action(&db_action) {
                log::warn!("Failed to save captured action: {}", e);
            }
        }
    }

    // Persist captures (wins + risks) to SQLite captures table
    if let Some(db) = db_ref {
        for win in &outcome.wins {
            let _ = db.insert_capture(
                &outcome.meeting_id,
                &outcome.meeting_title,
                outcome.account.as_deref(),
                "win",
                win,
            );
        }
        for risk in &outcome.risks {
            let _ = db.insert_capture(
                &outcome.meeting_id,
                &outcome.meeting_title,
                outcome.account.as_deref(),
                "risk",
                risk,
            );
        }
    }

    // Append wins to impact log
    let impact_log = workspace.join("_today").join("90-impact-log.md");
    if !outcome.wins.is_empty() {
        let mut content = String::new();
        if !impact_log.exists() {
            content.push_str("# Impact Log\n\n");
        }
        for win in &outcome.wins {
            content.push_str(&format!(
                "- **{}**: {} ({})\n",
                outcome
                    .account
                    .as_deref()
                    .unwrap_or(&outcome.meeting_title),
                win,
                outcome.captured_at.format("%H:%M")
            ));
        }
        if impact_log.exists() {
            let existing = std::fs::read_to_string(&impact_log).unwrap_or_default();
            let _ = std::fs::write(&impact_log, format!("{}{}", existing, content));
        } else {
            let _ = std::fs::write(&impact_log, content);
        }
    }

    Ok(())
}

/// Dismiss a post-meeting capture prompt (skip)
#[tauri::command]
pub fn dismiss_meeting_prompt(
    meeting_id: String,
    state: State<Arc<AppState>>,
) -> Result<(), String> {
    if let Ok(mut guard) = state.capture_dismissed.lock() {
        guard.insert(meeting_id);
    }
    Ok(())
}

/// Get post-meeting capture settings
#[tauri::command]
pub fn get_capture_settings(state: State<Arc<AppState>>) -> PostMeetingCaptureConfig {
    state
        .config
        .lock()
        .ok()
        .and_then(|g| g.clone())
        .map(|c| c.post_meeting_capture)
        .unwrap_or_default()
}

/// Toggle post-meeting capture on/off
#[tauri::command]
pub fn set_capture_enabled(
    enabled: bool,
    state: State<Arc<AppState>>,
) -> Result<(), String> {
    let mut config = state
        .config
        .lock()
        .map_err(|_| "Lock poisoned")?
        .clone()
        .ok_or("No configuration loaded")?;

    config.post_meeting_capture.enabled = enabled;

    // Write back to disk
    let home = dirs::home_dir().ok_or("Could not find home directory")?;
    let config_path = home.join(".dailyos").join("config.json");
    let content = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    std::fs::write(&config_path, content)
        .map_err(|e| format!("Failed to write config: {}", e))?;

    // Update in-memory state
    let mut guard = state.config.lock().map_err(|_| "Lock poisoned")?;
    *guard = Some(config);

    Ok(())
}

// =============================================================================
// Phase 3C: Weekly Planning Commands
// =============================================================================

/// Get current weekly planning state
#[tauri::command]
pub fn get_week_planning_state(state: State<Arc<AppState>>) -> WeekPlanningState {
    state
        .week_planning_state
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_default()
}

/// Get prepared week data for the wizard
#[tauri::command]
pub fn get_week_prep_data(state: State<Arc<AppState>>) -> WeekResult {
    // Just delegates to get_week_data — the wizard reads the same JSON
    get_week_data(state)
}

/// Submit user's priority selections from wizard step 1
#[tauri::command]
pub fn submit_week_priorities(
    priorities: Vec<String>,
    state: State<Arc<AppState>>,
) -> Result<(), String> {
    let config = state
        .config
        .lock()
        .map_err(|_| "Lock poisoned")?
        .clone()
        .ok_or("No configuration loaded")?;

    // Write priorities to a file for reference
    let workspace = std::path::Path::new(&config.workspace_path);
    let priorities_path = workspace.join("_today").join("data").join("week-priorities.json");
    let content = serde_json::to_string_pretty(&priorities)
        .map_err(|e| format!("Serialize error: {}", e))?;
    std::fs::write(&priorities_path, content)
        .map_err(|e| format!("Write error: {}", e))?;

    // Update planning state
    if let Ok(mut guard) = state.week_planning_state.lock() {
        *guard = WeekPlanningState::InProgress;
    }

    Ok(())
}

/// Submit selected focus blocks from wizard step 3
#[tauri::command]
pub fn submit_focus_blocks(
    blocks: Vec<FocusBlock>,
    state: State<Arc<AppState>>,
) -> Result<(), String> {
    let config = state
        .config
        .lock()
        .map_err(|_| "Lock poisoned")?
        .clone()
        .ok_or("No configuration loaded")?;

    // Write focus blocks to a file
    let workspace = std::path::Path::new(&config.workspace_path);
    let blocks_path = workspace.join("_today").join("data").join("week-focus-selected.json");
    let content = serde_json::to_string_pretty(&blocks)
        .map_err(|e| format!("Serialize error: {}", e))?;
    std::fs::write(&blocks_path, content)
        .map_err(|e| format!("Write error: {}", e))?;

    // Mark planning as completed
    if let Ok(mut guard) = state.week_planning_state.lock() {
        *guard = WeekPlanningState::Completed;
    }

    Ok(())
}

/// Skip weekly planning entirely (apply defaults)
#[tauri::command]
pub fn skip_week_planning(state: State<Arc<AppState>>) -> Result<(), String> {
    if let Ok(mut guard) = state.week_planning_state.lock() {
        *guard = WeekPlanningState::DefaultsApplied;
    }
    Ok(())
}
