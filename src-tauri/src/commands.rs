use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

use tauri::{Emitter, State};

use crate::executor::request_workflow_execution;
use crate::json_loader::{
    check_data_freshness, load_actions_json, load_emails_json, load_prep_json,
    load_schedule_json, DataFreshness,
};
use crate::parser::{count_inbox, list_inbox_files};
use crate::scheduler::get_next_run_time as scheduler_get_next_run_time;
use crate::state::{reload_config, AppState};
use crate::types::{
    Action, CalendarEvent, CapturedOutcome, Config, DashboardData, DayStats, ExecutionRecord,
    FocusBlock, FocusData, FullMeetingPrep, GoogleAuthStatus, InboxFile, MeetingType,
    OverlayStatus, PostMeetingCaptureConfig, Priority, WeekOverview, WeekPlanningState,
    WorkflowId, WorkflowStatus,
};
use crate::SchedulerSender;

/// Result type for dashboard data loading
#[derive(Debug, serde::Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum DashboardResult {
    Success {
        data: DashboardData,
        freshness: DataFreshness,
        #[serde(rename = "googleAuth")]
        google_auth: GoogleAuthStatus,
    },
    Empty {
        message: String,
        #[serde(rename = "googleAuth")]
        google_auth: GoogleAuthStatus,
    },
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
    // Get Google auth status for frontend
    let google_auth = state
        .google_auth
        .lock()
        .map(|g| g.clone())
        .unwrap_or(GoogleAuthStatus::NotConfigured);

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
            message: "Your daily briefing will appear here once generated.".to_string(),
            google_auth,
        };
    }

    // Check for data directory
    let data_dir = today_dir.join("data");
    if !data_dir.exists() {
        return DashboardResult::Empty {
            message: "Your daily briefing will appear here once generated.".to_string(),
            google_auth,
        };
    }

    // Load from JSON
    let (overview, briefing_meetings) = match load_schedule_json(&today_dir) {
        Ok(data) => data,
        Err(e) => {
            return DashboardResult::Error {
                message: format!("Failed to load schedule: {}", e),
            }
        }
    };

    // Merge briefing meetings with live calendar events (ADR-0032)
    let live_events = state
        .calendar_events
        .lock()
        .map(|g| g.clone())
        .unwrap_or_default();
    let tz: chrono_tz::Tz = config
        .schedules
        .today
        .timezone
        .parse()
        .unwrap_or(chrono_tz::America::New_York);
    let mut meetings = crate::calendar_merge::merge_meetings(briefing_meetings, &live_events, &tz);

    // Annotate meetings with prep-reviewed state from SQLite (ADR-0033)
    if let Ok(db_guard) = state.db.lock() {
        if let Some(db) = db_guard.as_ref() {
            if let Ok(reviewed) = db.get_reviewed_preps() {
                for m in &mut meetings {
                    if let Some(ref pf) = m.prep_file {
                        if reviewed.contains_key(pf) {
                            m.prep_reviewed = Some(true);
                        }
                    }
                }
            }
        }
    }

    let mut actions = load_actions_json(&today_dir).unwrap_or_default();

    // Merge non-briefing actions from SQLite (post-meeting capture, inbox) — I17
    if let Ok(db_guard) = state.db.lock() {
        if let Some(db) = db_guard.as_ref() {
            if let Ok(db_actions) = db.get_non_briefing_pending_actions() {
                let json_titles: HashSet<String> = actions
                    .iter()
                    .map(|a| a.title.to_lowercase().trim().to_string())
                    .collect();
                for dba in db_actions {
                    if !json_titles.contains(&dba.title.to_lowercase().trim().to_string()) {
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

    let emails = load_emails_json(&today_dir).ok().filter(|e| !e.is_empty());

    // Calculate stats (exclude cancelled meetings)
    let inbox_count = count_inbox(workspace);
    let active_meetings: Vec<_> = meetings
        .iter()
        .filter(|m| m.overlay_status != Some(OverlayStatus::Cancelled))
        .collect();
    let stats = DayStats {
        total_meetings: active_meetings.len(),
        customer_meetings: active_meetings
            .iter()
            .filter(|m| matches!(m.meeting_type, MeetingType::Customer | MeetingType::Qbr))
            .count(),
        actions_due: actions.len(),
        inbox_count,
    };

    let freshness = check_data_freshness(&today_dir);

    DashboardResult::Success {
        data: DashboardData {
            overview,
            stats,
            meetings,
            actions,
            emails,
        },
        freshness,
        google_auth,
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
        Ok(mut prep) => {
            // Record that this prep was reviewed (ADR-0033)
            // Also compute stakeholder signals from DB (I43)
            if let Ok(db_guard) = state.db.lock() {
                if let Some(db) = db_guard.as_ref() {
                    let _ = db.mark_prep_reviewed(
                        &prep_file,
                        prep.calendar_event_id.as_deref(),
                        &prep.title,
                    );

                    // Compute stakeholder signals if prep has an account
                    if let Some(account) = extract_account_from_prep(&prep_file, &today_dir) {
                        match db.get_stakeholder_signals(&account) {
                            Ok(signals) => {
                                // Only attach if there's meaningful data
                                if signals.meeting_frequency_90d > 0
                                    || signals.last_contact.is_some()
                                {
                                    prep.stakeholder_signals = Some(signals);
                                }
                            }
                            Err(e) => {
                                log::warn!("Failed to compute stakeholder signals: {}", e);
                            }
                        }
                    }

                    // Enrich attendees with person context (I51)
                    if let Some(ref cal_id) = prep.calendar_event_id {
                        if let Ok(events_guard) = state.calendar_events.lock() {
                            if let Some(event) = events_guard.iter().find(|e| e.id == *cal_id) {
                                let mut attendee_ctx = Vec::new();
                                for email in &event.attendees {
                                    if let Ok(Some(person)) = db.get_person_by_email(email) {
                                        let signals = db.get_person_signals(&person.id).ok();
                                        attendee_ctx.push(crate::types::AttendeeContext {
                                            name: person.name,
                                            email: Some(person.email),
                                            role: person.role,
                                            organization: person.organization,
                                            relationship: Some(person.relationship),
                                            meeting_count: Some(person.meeting_count),
                                            last_seen: person.last_seen,
                                            temperature: signals
                                                .as_ref()
                                                .map(|s| s.temperature.clone()),
                                            notes: person.notes,
                                            person_id: Some(person.id),
                                        });
                                    }
                                }
                                if !attendee_ctx.is_empty() {
                                    prep.attendee_context = Some(attendee_ctx);
                                }
                            }
                        }
                    }
                }
            }
            MeetingPrepResult::Success { data: prep }
        }
        Err(e) => MeetingPrepResult::NotFound {
            message: format!("Prep not found: {}", e),
        },
    }
}

/// Extract the account name from a prep JSON file (for stakeholder signal lookup).
fn extract_account_from_prep(prep_file: &str, today_dir: &Path) -> Option<String> {
    let prep_path = if prep_file.starts_with("preps/") {
        today_dir.join("data").join(prep_file)
    } else {
        today_dir
            .join("data")
            .join("preps")
            .join(format!(
                "{}.json",
                prep_file
                    .trim_end_matches(".json")
                    .trim_end_matches(".md")
            ))
    };
    let content = std::fs::read_to_string(prep_path).ok()?;
    let data: serde_json::Value = serde_json::from_str(&content).ok()?;
    data.get("account")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
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
    let profile = config.profile.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let workspace = Path::new(&workspace_path);
        let db_guard = state.db.lock().ok();
        let db_ref = db_guard.as_ref().and_then(|g| g.as_ref());
        crate::processor::process_file(workspace, &filename, db_ref, &profile)
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
    let profile = config.profile.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let workspace = Path::new(&workspace_path);
        let db_guard = state.db.lock().ok();
        let db_ref = db_guard.as_ref().and_then(|g| g.as_ref());
        crate::processor::process_all(workspace, db_ref, &profile)
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
    let profile = config.profile.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let workspace = Path::new(&workspace_path);
        let db_guard = state.db.lock().ok();
        let db_ref = db_guard.as_ref().and_then(|g| g.as_ref());
        crate::processor::enrich::enrich_file(workspace, &filename, db_ref, &profile)
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

/// Refresh emails independently without re-running the full /today pipeline (I20).
///
/// Re-fetches from Gmail, classifies, and updates emails.json.
/// Rejects if /today pipeline is currently running.
#[tauri::command]
pub async fn refresh_emails(
    state: State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    let config = state
        .config
        .lock()
        .map_err(|_| "Lock poisoned")?
        .clone()
        .ok_or("No configuration loaded")?;

    let state_clone = state.inner().clone();
    let workspace_path = config.workspace_path.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let workspace = std::path::Path::new(&workspace_path);
        let executor = crate::executor::Executor::new(state_clone, app_handle);
        executor.execute_email_refresh(workspace)
    })
    .await
    .map_err(|e| format!("Email refresh task failed: {}", e))?
    .map(|_| "Email refresh complete".to_string())
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

    crate::state::create_or_update_config(&state, |config| {
        config.profile = profile.clone();
    })
}

/// Set entity mode (account, project, or both)
///
/// Also derives the correct profile for backend compatibility.
/// Creates Accounts/ dir if switching to account/both mode.
#[tauri::command]
pub fn set_entity_mode(
    mode: String,
    state: State<Arc<AppState>>,
) -> Result<Config, String> {
    crate::types::validate_entity_mode(&mode)?;

    let config = crate::state::create_or_update_config(&state, |config| {
        config.entity_mode = mode.clone();
        config.profile = crate::types::profile_for_entity_mode(&mode);
    })?;

    // If workspace exists, ensure Accounts/ dir is created for account/both
    if !config.workspace_path.is_empty() {
        let workspace = std::path::Path::new(&config.workspace_path);
        if workspace.exists() && (mode == "account" || mode == "both") {
            let accounts_dir = workspace.join("Accounts");
            if !accounts_dir.exists() {
                let _ = std::fs::create_dir_all(&accounts_dir);
            }
        }
    }

    Ok(config)
}

/// Set workspace path and scaffold directory structure
#[tauri::command]
pub fn set_workspace_path(
    path: String,
    state: State<Arc<AppState>>,
) -> Result<Config, String> {
    let workspace = std::path::Path::new(&path);

    // Validate path is absolute
    if !workspace.is_absolute() {
        return Err("Workspace path must be absolute".to_string());
    }

    // Read current entity_mode (or default)
    let entity_mode = state
        .config
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|c| c.entity_mode.clone()))
        .unwrap_or_else(|| "account".to_string());

    // Scaffold workspace dirs
    crate::state::initialize_workspace(workspace, &entity_mode)?;

    crate::state::create_or_update_config(&state, |config| {
        config.workspace_path = path.clone();
    })
}

/// Set schedule for a workflow
#[tauri::command]
pub fn set_schedule(
    workflow: String,
    hour: u32,
    minute: u32,
    timezone: String,
    state: State<Arc<AppState>>,
) -> Result<Config, String> {
    // Validate inputs
    if hour > 23 {
        return Err("Hour must be 0-23".to_string());
    }
    if minute > 59 {
        return Err("Minute must be 0-59".to_string());
    }

    // Validate timezone parses
    timezone
        .parse::<chrono_tz::Tz>()
        .map_err(|_| format!("Invalid timezone: {}", timezone))?;

    let workflow_id: WorkflowId = workflow.parse()?;

    crate::state::create_or_update_config(&state, |config| {
        let cron = match workflow_id {
            WorkflowId::Today => format!("{} {} * * 1-5", minute, hour),
            WorkflowId::Archive => format!("{} {} * * *", minute, hour),
            WorkflowId::InboxBatch => format!("{} {} * * 1-5", minute, hour),
            WorkflowId::Week => format!("{} {} * * 1", minute, hour),
        };

        let entry = match workflow_id {
            WorkflowId::Today => &mut config.schedules.today,
            WorkflowId::Archive => &mut config.schedules.archive,
            WorkflowId::InboxBatch => &mut config.schedules.inbox_batch,
            WorkflowId::Week => &mut config.schedules.week,
        };

        entry.cron = cron;
        entry.timezone = timezone.clone();
    })
}

/// Save user profile fields (name, company, title, focus, domain)
#[tauri::command]
pub fn set_user_profile(
    name: Option<String>,
    company: Option<String>,
    title: Option<String>,
    focus: Option<String>,
    domain: Option<String>,
    state: State<Arc<AppState>>,
) -> Result<String, String> {
    crate::state::create_or_update_config(&state, |config| {
        // Helper: trim, convert empty to None
        fn clean(val: Option<String>) -> Option<String> {
            val.and_then(|s| {
                let trimmed = s.trim().to_string();
                if trimmed.is_empty() { None } else { Some(trimmed) }
            })
        }

        config.user_name = clean(name);
        config.user_company = clean(company);
        config.user_title = clean(title);
        config.user_focus = clean(focus);
        if let Some(d) = domain {
            let trimmed = d.trim().to_lowercase();
            config.user_domain = if trimmed.is_empty() { None } else { Some(trimmed) };
        }
    })?;

    Ok("ok".to_string())
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

/// Get actions from the SQLite database for display.
///
/// Returns pending actions (within `days_ahead` window, default 7) combined
/// with recently completed actions (last 48 hours) so the UI can show both
/// active and done states.
#[tauri::command]
pub fn get_actions_from_db(
    days_ahead: Option<i32>,
    state: State<Arc<AppState>>,
) -> Result<Vec<crate::db::DbAction>, String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let mut actions = db
        .get_due_actions(days_ahead.unwrap_or(7))
        .map_err(|e| e.to_string())?;
    let completed = db
        .get_completed_actions(48)
        .map_err(|e| e.to_string())?;
    actions.extend(completed);
    Ok(actions)
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

/// Reopen a completed action, setting it back to pending.
#[tauri::command]
pub fn reopen_action(
    id: String,
    state: State<Arc<AppState>>,
) -> Result<(), String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    db.reopen_action(&id).map_err(|e| e.to_string())
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

/// Get current Google authentication status.
///
/// Re-checks the token file on disk when the cached state is NotConfigured,
/// so the UI picks up tokens written by external flows (e.g. manual auth).
#[tauri::command]
pub fn get_google_auth_status(state: State<Arc<AppState>>) -> GoogleAuthStatus {
    let cached = state
        .google_auth
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or(GoogleAuthStatus::NotConfigured);

    // If cached state says not configured, re-check disk — token may have
    // been written by a script or the browser auth flow completing late.
    if matches!(cached, GoogleAuthStatus::NotConfigured) {
        let fresh = crate::state::detect_google_auth();
        if matches!(fresh, GoogleAuthStatus::Authenticated { .. }) {
            if let Ok(mut guard) = state.google_auth.lock() {
                *guard = fresh.clone();
            }
            return fresh;
        }
    }

    cached
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

    // Auto-extract domain from email (non-fatal, preserves manual overrides)
    if let Some(at_pos) = email.find('@') {
        let domain = email[at_pos + 1..].to_lowercase();
        if !domain.is_empty() {
            let _ = crate::state::create_or_update_config(&state, |config| {
                if config.user_domain.is_none() {
                    config.user_domain = Some(domain);
                }
            });
        }
    }

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
    crate::state::create_or_update_config(&state, |config| {
        config.post_meeting_capture.enabled = enabled;
    })?;
    Ok(())
}

/// Set post-meeting capture delay (minutes before prompt appears)
#[tauri::command]
pub fn set_capture_delay(
    delay_minutes: u32,
    state: State<Arc<AppState>>,
) -> Result<(), String> {
    crate::state::create_or_update_config(&state, |config| {
        config.post_meeting_capture.delay_minutes = delay_minutes;
    })?;
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

// =============================================================================
// Transcript Intake & Meeting Outcomes (I44 / I45 / ADR-0044)
// =============================================================================

/// Attach and process a transcript for a specific meeting.
///
/// Checks immutability (one transcript per meeting), processes the transcript
/// with full meeting context via Claude, stores outcomes, and routes the file.
#[tauri::command]
pub async fn attach_meeting_transcript(
    file_path: String,
    meeting: CalendarEvent,
    state: State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<crate::types::TranscriptResult, String> {
    // Check immutability — one transcript per meeting
    {
        let guard = state
            .transcript_processed
            .lock()
            .map_err(|_| "Lock poisoned")?;
        if guard.contains_key(&meeting.id) {
            return Err(format!(
                "Meeting '{}' already has a processed transcript",
                meeting.title
            ));
        }
    }

    let config = state
        .config
        .lock()
        .map_err(|_| "Lock poisoned")?
        .clone()
        .ok_or("No configuration loaded")?;

    let state_clone = state.inner().clone();
    let workspace_path = config.workspace_path.clone();
    let profile = config.profile.clone();
    let meeting_id = meeting.id.clone();
    let meeting_clone = meeting.clone();
    let file_path_for_record = file_path.clone();

    let result = tauri::async_runtime::spawn_blocking(move || {
        let workspace = Path::new(&workspace_path);
        let db_guard = state_clone.db.lock().ok();
        let db_ref = db_guard.as_ref().and_then(|g| g.as_ref());
        crate::processor::transcript::process_transcript(
            workspace,
            &file_path,
            &meeting_clone,
            db_ref,
            &profile,
        )
    })
    .await
    .map_err(|e| format!("Transcript processing task failed: {}", e))?;

    // On success, record transcript and mark as captured
    if result.status == "success" {
        let record = crate::types::TranscriptRecord {
            meeting_id: meeting_id.clone(),
            file_path: file_path_for_record,
            destination: result.destination.clone().unwrap_or_default(),
            summary: result.summary.clone(),
            processed_at: chrono::Utc::now().to_rfc3339(),
        };

        if let Ok(mut guard) = state.transcript_processed.lock() {
            guard.insert(meeting_id.clone(), record);
            let _ = crate::state::save_transcript_records(&guard);
        }

        if let Ok(mut guard) = state.capture_captured.lock() {
            guard.insert(meeting_id.clone());
        }

        // Build and emit outcome data for live frontend updates
        let outcome_data = build_outcome_data(&meeting_id, &result, &state);
        let _ = app_handle.emit("transcript-processed", &outcome_data);
    }

    Ok(result)
}

/// Get meeting outcomes (from transcript processing or manual capture).
///
/// Returns `None` if the meeting has no processed transcript.
#[tauri::command]
pub fn get_meeting_outcomes(
    meeting_id: String,
    state: State<Arc<AppState>>,
) -> Result<Option<crate::types::MeetingOutcomeData>, String> {
    // Check transcript records
    let record = state
        .transcript_processed
        .lock()
        .map_err(|_| "Lock poisoned")?
        .get(&meeting_id)
        .cloned();

    let Some(record) = record else {
        return Ok(None);
    };

    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let captures = db
        .get_captures_for_meeting(&meeting_id)
        .map_err(|e| e.to_string())?;
    let actions = db
        .get_actions_for_meeting(&meeting_id)
        .map_err(|e| e.to_string())?;

    let mut wins = Vec::new();
    let mut risks = Vec::new();
    let mut decisions = Vec::new();

    for cap in captures {
        match cap.capture_type.as_str() {
            "win" => wins.push(cap.content),
            "risk" => risks.push(cap.content),
            "decision" => decisions.push(cap.content),
            _ => {}
        }
    }

    Ok(Some(crate::types::MeetingOutcomeData {
        meeting_id,
        summary: record.summary,
        wins,
        risks,
        decisions,
        actions,
        transcript_path: Some(record.destination),
        processed_at: Some(record.processed_at),
    }))
}

/// Update the content of a capture (win/risk/decision) — I45 inline editing.
#[tauri::command]
pub fn update_capture(
    id: String,
    content: String,
    state: State<Arc<AppState>>,
) -> Result<(), String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    db.update_capture(&id, &content).map_err(|e| e.to_string())
}

/// Cycle an action's priority (P1→P2→P3→P1) — I45 interaction.
#[tauri::command]
pub fn update_action_priority(
    id: String,
    priority: String,
    state: State<Arc<AppState>>,
) -> Result<(), String> {
    // Validate priority
    if !matches!(priority.as_str(), "P1" | "P2" | "P3") {
        return Err(format!("Invalid priority: {}. Must be P1, P2, or P3.", priority));
    }
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    db.update_action_priority(&id, &priority)
        .map_err(|e| e.to_string())
}

// =============================================================================
// Processing History (I6)
// =============================================================================

/// Get processing history from the SQLite database.
///
/// Returns recent inbox processing log entries for the History page.
#[tauri::command]
pub fn get_processing_history(
    limit: Option<i32>,
    state: State<Arc<AppState>>,
) -> Result<Vec<crate::db::DbProcessingLog>, String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    db.get_processing_log(limit.unwrap_or(50))
        .map_err(|e| e.to_string())
}

// =============================================================================
// Feature Toggles (I39)
// =============================================================================

/// Feature definition for the Settings UI.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureDefinition {
    pub key: String,
    pub label: String,
    pub description: String,
    pub enabled: bool,
    pub cs_only: bool,
}

/// Get all features with their current enabled state.
#[tauri::command]
pub fn get_features(state: State<Arc<AppState>>) -> Result<Vec<FeatureDefinition>, String> {
    let config = state
        .config
        .lock()
        .map_err(|_| "Lock poisoned")?
        .clone()
        .ok_or("No configuration loaded")?;

    let definitions = vec![
        ("emailTriage", "Email Triage", "Fetch and classify Gmail messages", false),
        ("postMeetingCapture", "Post-Meeting Capture", "Prompt for outcomes after meetings end", false),
        ("meetingPrep", "Meeting Prep", "Generate prep context for upcoming meetings", false),
        ("weeklyPlanning", "Weekly Planning", "Weekly overview and focus block suggestions", false),
        ("inboxProcessing", "Inbox Processing", "Classify and route files from _inbox", false),
        ("accountTracking", "Account Tracking", "Track customer accounts, health, and ARR", true),
        ("impactRollup", "Impact Rollup", "Roll up daily wins and risks to account files", true),
    ];

    Ok(definitions
        .into_iter()
        .map(|(key, label, desc, cs_only)| FeatureDefinition {
            enabled: crate::types::is_feature_enabled(&config, key),
            key: key.to_string(),
            label: label.to_string(),
            description: desc.to_string(),
            cs_only,
        })
        .collect())
}

/// Set a single feature toggle on or off.
#[tauri::command]
pub fn set_feature_enabled(
    feature: String,
    enabled: bool,
    state: State<Arc<AppState>>,
) -> Result<Config, String> {
    crate::state::create_or_update_config(&state, |config| {
        config.features.insert(feature.clone(), enabled);
    })
}

// =============================================================================
// Onboarding: Demo Data
// =============================================================================

/// Install demo data into the user's workspace for the onboarding tour.
///
/// Writes date-patched JSON fixtures to `_today/data/` and seeds SQLite
/// with mock accounts, actions, and meeting history. The demo data is
/// replaced on the first real briefing run.
#[tauri::command]
pub fn install_demo_data(
    state: State<Arc<AppState>>,
) -> Result<String, String> {
    let workspace_path = state
        .config
        .lock()
        .map_err(|_| "Config lock failed")?
        .as_ref()
        .map(|c| c.workspace_path.clone())
        .ok_or("No workspace configured")?;

    let workspace = std::path::Path::new(&workspace_path);
    crate::devtools::write_fixtures(workspace)?;

    let db_guard = state.db.lock().map_err(|_| "DB lock poisoned")?;
    if let Some(db) = db_guard.as_ref() {
        crate::devtools::seed_database(db)?;
    }

    Ok("Demo data installed".into())
}

// =============================================================================
// Onboarding: Populate Workspace (I57)
// =============================================================================

/// Create account/project folders and save user domain during onboarding.
///
/// For each account: creates `Accounts/{name}/` and upserts a minimal DbAccount
/// record (bridge pattern fires `ensure_entity_for_account` automatically).
/// For each project: creates `Projects/{name}/` (filesystem only, no SQLite — I50).
/// DB errors are non-fatal; folder creation is the primary value.
#[tauri::command]
pub fn populate_workspace(
    accounts: Vec<String>,
    projects: Vec<String>,
    state: State<Arc<AppState>>,
) -> Result<String, String> {
    // 1. Get workspace path
    let workspace_path = state
        .config
        .lock()
        .map_err(|_| "Config lock failed")?
        .as_ref()
        .map(|c| c.workspace_path.clone())
        .ok_or("No workspace configured")?;

    let workspace = std::path::Path::new(&workspace_path);
    let now = chrono::Utc::now().to_rfc3339();

    // 3. Process accounts
    let mut account_count = 0;
    for name in &accounts {
        let name = name.trim();
        if name.is_empty() {
            continue;
        }

        // Create folder (idempotent)
        let account_dir = workspace.join("Accounts").join(name);
        if let Err(e) = std::fs::create_dir_all(&account_dir) {
            log::warn!("Failed to create account dir '{}': {}", name, e);
            continue;
        }

        // Upsert to SQLite (non-fatal)
        let slug = crate::util::slugify(name);
        let db_account = crate::db::DbAccount {
            id: slug,
            name: name.to_string(),
            ring: None,
            arr: None,
            health: None,
            contract_start: None,
            contract_end: None,
            csm: None,
            champion: None,
            tracker_path: Some(format!("Accounts/{}", name)),
            updated_at: now.clone(),
        };

        if let Ok(db_guard) = state.db.lock() {
            if let Some(db) = db_guard.as_ref() {
                if let Err(e) = db.upsert_account(&db_account) {
                    log::warn!("Failed to upsert account '{}': {}", name, e);
                }
            }
        }

        account_count += 1;
    }

    // 4. Process projects (filesystem only — I50 tracks projects table)
    let mut project_count = 0;
    for name in &projects {
        let name = name.trim();
        if name.is_empty() {
            continue;
        }

        let project_dir = workspace.join("Projects").join(name);
        if let Err(e) = std::fs::create_dir_all(&project_dir) {
            log::warn!("Failed to create project dir '{}': {}", name, e);
            continue;
        }

        project_count += 1;
    }

    Ok(format!(
        "Created {} accounts, {} projects",
        account_count, project_count
    ))
}

// =============================================================================
// Dev Tools
// =============================================================================

/// Apply a dev scenario (reset, mock_full, mock_no_auth, mock_empty).
///
/// Returns an error in release builds. In debug builds, delegates to
/// `devtools::apply_scenario` which orchestrates the scenario switch.
#[tauri::command]
pub fn dev_apply_scenario(
    scenario: String,
    state: State<Arc<AppState>>,
) -> Result<String, String> {
    if !cfg!(debug_assertions) {
        return Err("Dev tools not available in release builds".into());
    }
    crate::devtools::apply_scenario(&scenario, &state)
}

/// Get current dev state for the dev tools panel.
///
/// Returns an error in release builds. In debug builds, returns counts
/// and status for config, database, today data, and Google auth.
#[tauri::command]
pub fn dev_get_state(
    state: State<Arc<AppState>>,
) -> Result<crate::devtools::DevState, String> {
    if !cfg!(debug_assertions) {
        return Err("Dev tools not available in release builds".into());
    }
    crate::devtools::get_dev_state(&state)
}

/// Build MeetingOutcomeData from a TranscriptResult + state lookups.
fn build_outcome_data(
    meeting_id: &str,
    result: &crate::types::TranscriptResult,
    state: &AppState,
) -> crate::types::MeetingOutcomeData {
    // Try to get actions from DB for richer data
    let actions = state
        .db
        .lock()
        .ok()
        .and_then(|guard| {
            guard
                .as_ref()
                .and_then(|db| db.get_actions_for_meeting(meeting_id).ok())
        })
        .unwrap_or_default();

    let transcript_path = state
        .transcript_processed
        .lock()
        .ok()
        .and_then(|guard| guard.get(meeting_id).map(|r| r.destination.clone()));

    crate::types::MeetingOutcomeData {
        meeting_id: meeting_id.to_string(),
        summary: result.summary.clone(),
        wins: result.wins.clone(),
        risks: result.risks.clone(),
        decisions: result.decisions.clone(),
        actions,
        transcript_path,
        processed_at: Some(chrono::Utc::now().to_rfc3339()),
    }
}

/// Compute executive intelligence signals (I42).
///
/// Cross-references SQLite data + today's schedule to surface decisions due,
/// stale delegations, portfolio alerts, cancelable meetings, and skip-today items.
#[tauri::command]
pub fn get_executive_intelligence(
    state: State<Arc<AppState>>,
) -> Result<crate::intelligence::ExecutiveIntelligence, String> {
    // Load config for profile + workspace
    let config = state
        .config
        .lock()
        .map_err(|_| "Lock poisoned")?
        .clone()
        .ok_or("No configuration loaded")?;

    let workspace = std::path::Path::new(&config.workspace_path);
    let today_dir = workspace.join("_today");

    // Load schedule meetings (merged with live calendar)
    let meetings = if today_dir.join("data").exists() {
        let briefing_meetings = load_schedule_json(&today_dir)
            .map(|(_overview, meetings)| meetings)
            .unwrap_or_default();
        let live_events = state
            .calendar_events
            .lock()
            .map(|g| g.clone())
            .unwrap_or_default();
        let tz: chrono_tz::Tz = config
            .schedules
            .today
            .timezone
            .parse()
            .unwrap_or(chrono_tz::America::New_York);
        crate::calendar_merge::merge_meetings(briefing_meetings, &live_events, &tz)
    } else {
        Vec::new()
    };

    // Load cached skip-today from AI enrichment (if available)
    let skip_today = load_skip_today(&today_dir);

    // Compute intelligence from DB
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    Ok(crate::intelligence::compute_executive_intelligence(
        db,
        &meetings,
        &config.profile,
        skip_today,
    ))
}

/// Load cached SKIP TODAY results from `_today/data/intelligence.json`.
///
/// Written by AI enrichment. Returns empty vec if file doesn't exist or is
/// malformed — fault-tolerant per ADR-0042 principle.
fn load_skip_today(today_dir: &std::path::Path) -> Vec<crate::intelligence::SkipSignal> {
    let path = today_dir.join("data").join("intelligence.json");
    if !path.exists() {
        return Vec::new();
    }

    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str::<Vec<crate::intelligence::SkipSignal>>(&s).ok())
        .unwrap_or_default()
}

// =============================================================================
// People Commands (I51)
// =============================================================================

/// Get all people, optionally filtered by relationship.
#[tauri::command]
pub fn get_people(
    relationship: Option<String>,
    state: State<Arc<AppState>>,
) -> Result<Vec<crate::db::DbPerson>, String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    db.get_people(relationship.as_deref())
        .map_err(|e| e.to_string())
}

/// Person detail result including signals, linked entities, and recent meetings.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonDetailResult {
    pub person: crate::db::DbPerson,
    pub signals: Option<crate::db::PersonSignals>,
    pub entities: Vec<EntitySummary>,
    pub recent_meetings: Vec<MeetingSummary>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntitySummary {
    pub id: String,
    pub name: String,
    pub entity_type: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingSummary {
    pub id: String,
    pub title: String,
    pub start_time: String,
}

/// Get full detail for a person (person + signals + entities + recent meetings).
#[tauri::command]
pub fn get_person_detail(
    person_id: String,
    state: State<Arc<AppState>>,
) -> Result<PersonDetailResult, String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let person = db
        .get_person(&person_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Person not found: {}", person_id))?;

    let signals = db.get_person_signals(&person_id).ok();

    let entities = db
        .get_entities_for_person(&person_id)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|e| EntitySummary {
            id: e.id,
            name: e.name,
            entity_type: e.entity_type.as_str().to_string(),
        })
        .collect();

    let recent_meetings = db
        .get_person_meetings(&person_id, 10)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|m| MeetingSummary {
            id: m.id,
            title: m.title,
            start_time: m.start_time,
        })
        .collect();

    Ok(PersonDetailResult {
        person,
        signals,
        entities,
        recent_meetings,
    })
}

/// Search people by name, email, or organization.
#[tauri::command]
pub fn search_people(
    query: String,
    state: State<Arc<AppState>>,
) -> Result<Vec<crate::db::DbPerson>, String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    db.search_people(&query, 50).map_err(|e| e.to_string())
}

/// Update a single field on a person (role, organization, notes, relationship).
/// Also updates the person's workspace files.
#[tauri::command]
pub fn update_person(
    person_id: String,
    field: String,
    value: String,
    state: State<Arc<AppState>>,
) -> Result<(), String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    db.update_person_field(&person_id, &field, &value)
        .map_err(|e| e.to_string())?;

    // Regenerate workspace files
    if let Ok(Some(person)) = db.get_person(&person_id) {
        let config = state.config.lock().map_err(|_| "Lock poisoned")?;
        if let Some(ref config) = *config {
            let workspace = Path::new(&config.workspace_path);
            let _ = crate::people::write_person_json(workspace, &person, db);
            let _ = crate::people::write_person_markdown(workspace, &person, db);
        }
    }

    Ok(())
}

/// Link a person to an entity (account/project).
/// Regenerates person.json so the link persists in the filesystem (ADR-0048).
#[tauri::command]
pub fn link_person_entity(
    person_id: String,
    entity_id: String,
    relationship_type: String,
    state: State<Arc<AppState>>,
) -> Result<(), String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    db.link_person_to_entity(&person_id, &entity_id, &relationship_type)
        .map_err(|e| e.to_string())?;

    // Regenerate person.json so linked_entities persists in filesystem (ADR-0048)
    if let Ok(Some(person)) = db.get_person(&person_id) {
        let config = state.config.lock().map_err(|_| "Lock poisoned")?;
        if let Some(ref config) = *config {
            let workspace = Path::new(&config.workspace_path);
            let _ = crate::people::write_person_json(workspace, &person, db);
            let _ = crate::people::write_person_markdown(workspace, &person, db);
        }
    }

    Ok(())
}

/// Unlink a person from an entity.
/// Regenerates person.json so the removal persists in the filesystem (ADR-0048).
#[tauri::command]
pub fn unlink_person_entity(
    person_id: String,
    entity_id: String,
    state: State<Arc<AppState>>,
) -> Result<(), String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    db.unlink_person_from_entity(&person_id, &entity_id)
        .map_err(|e| e.to_string())?;

    // Regenerate person.json so linked_entities reflects removal (ADR-0048)
    if let Ok(Some(person)) = db.get_person(&person_id) {
        let config = state.config.lock().map_err(|_| "Lock poisoned")?;
        if let Some(ref config) = *config {
            let workspace = Path::new(&config.workspace_path);
            let _ = crate::people::write_person_json(workspace, &person, db);
            let _ = crate::people::write_person_markdown(workspace, &person, db);
        }
    }

    Ok(())
}

/// Get people linked to an entity.
#[tauri::command]
pub fn get_people_for_entity(
    entity_id: String,
    state: State<Arc<AppState>>,
) -> Result<Vec<crate::db::DbPerson>, String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    db.get_people_for_entity(&entity_id)
        .map_err(|e| e.to_string())
}

/// Get people who attended a specific meeting.
#[tauri::command]
pub fn get_meeting_attendees(
    meeting_id: String,
    state: State<Arc<AppState>>,
) -> Result<Vec<crate::db::DbPerson>, String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    db.get_meeting_attendees(&meeting_id)
        .map_err(|e| e.to_string())
}
