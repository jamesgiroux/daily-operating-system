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
    FocusData, FocusMeeting, FullMeetingPrep, GoogleAuthStatus, InboxFile, MeetingType,
    OverlayStatus, PostMeetingCaptureConfig, Priority, TimeBlock, WeekOverview,
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
    let guard = state.config.read().map_err(|_| "Lock poisoned")?;
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
    let config = match state.config.read() {
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
        Err(_) => {
            return DashboardResult::Empty {
                message: "Your daily briefing will appear here once generated.".to_string(),
                google_auth,
            }
        }
    };

    // Merge briefing meetings with live calendar events (ADR-0032)
    let live_events = state
        .calendar_events
        .read()
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
        .read()
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
    let config = match state.config.read() {
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
                        if let Ok(events_guard) = state.calendar_events.read() {
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
    let config = match state.config.read() {
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

/// Get focus/priority data — assembled from schedule.json + SQLite actions + gap analysis
#[tauri::command]
pub fn get_focus_data(state: State<Arc<AppState>>) -> FocusResult {
    // Get config
    let config = match state.config.read() {
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

    let workspace = Path::new(&config.workspace_path);
    let today_dir = workspace.join("_today");

    // 1. Load schedule.json — if missing, nothing to show
    let (overview, meetings) = match load_schedule_json(&today_dir) {
        Ok(data) => data,
        Err(_) => {
            return FocusResult::NotFound {
                message: "No briefing data available. Run a briefing first.".to_string(),
            }
        }
    };

    // 2. Focus statement from schedule
    let focus_statement = overview.focus;

    // 3. Filter meetings to "key" types (where prep matters)
    let key_meetings: Vec<FocusMeeting> = meetings
        .iter()
        .filter(|m| matches!(
            m.meeting_type,
            MeetingType::Customer
                | MeetingType::Qbr
                | MeetingType::Partnership
                | MeetingType::External
                | MeetingType::OneOnOne
        ))
        .map(|m| {
            let type_str = match m.meeting_type {
                MeetingType::Customer => "customer",
                MeetingType::Qbr => "qbr",
                MeetingType::Partnership => "partnership",
                MeetingType::External => "external",
                MeetingType::OneOnOne => "one_on_one",
                MeetingType::Training => "training",
                MeetingType::Internal => "internal",
                MeetingType::TeamSync => "team_sync",
                MeetingType::AllHands => "all_hands",
                MeetingType::Personal => "personal",
            };
            FocusMeeting {
                id: m.id.clone(),
                title: m.title.clone(),
                time: m.time.clone(),
                end_time: m.end_time.clone(),
                meeting_type: type_str.to_string(),
                has_prep: m.has_prep,
                account: m.account.clone(),
                prep_file: m.prep_file.clone(),
            }
        })
        .collect();

    // 4. Priority actions from SQLite (due today or overdue)
    let priorities = if let Ok(db_guard) = state.db.lock() {
        if let Some(db) = db_guard.as_ref() {
            db.get_due_actions(1).unwrap_or_default()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    // 5. Compute available time blocks from today's meetings
    let today_date = chrono::Local::now().date_naive();
    let meeting_events: Vec<serde_json::Value> = meetings
        .iter()
        .filter_map(|m| {
            // Need start + end times for gap computation
            let end = m.end_time.as_ref()?;
            Some(serde_json::json!({
                "start": m.time,
                "end": end,
            }))
        })
        .collect();
    let gaps = crate::prepare::gaps::compute_gaps(&meeting_events, today_date);
    let available_blocks: Vec<TimeBlock> = gaps
        .iter()
        .filter_map(|g| {
            let start = g.get("start")?.as_str()?;
            let end = g.get("end")?.as_str()?;
            let duration = g.get("duration_minutes")?.as_u64()? as u32;
            let hour: u32 = start
                .find('T')
                .and_then(|t| start[t + 1..].split(':').next())
                .and_then(|h| h.parse().ok())
                .unwrap_or(12);
            let suggested_use = if hour < 12 {
                "Deep Work"
            } else {
                "Admin / Follow-up"
            };
            Some(TimeBlock {
                day: String::new(),
                start: start.to_string(),
                end: end.to_string(),
                duration_minutes: duration,
                suggested_use: Some(suggested_use.to_string()),
            })
        })
        .collect();
    let total_focus_minutes: u32 = available_blocks.iter().map(|b| b.duration_minutes).sum();

    FocusResult::Success {
        data: FocusData {
            focus_statement,
            priorities,
            key_meetings,
            available_blocks,
            total_focus_minutes,
        },
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

    let workspace = Path::new(&config.workspace_path);
    let today_dir = workspace.join("_today");

    let mut actions = load_actions_json(&today_dir).unwrap_or_default();

    // Merge non-briefing actions from SQLite (same logic as dashboard)
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

    if actions.is_empty() {
        ActionsResult::Empty {
            message: "No actions yet. Actions appear after your first briefing.".to_string(),
        }
    } else {
        ActionsResult::Success { data: actions }
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
    let config = match state.config.read() {
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
        .read()
        .map_err(|_| "Internal error")?
        .clone()
        .ok_or("No configuration loaded")?;

    let state = state.inner().clone();
    let workspace_path = config.workspace_path.clone();
    let profile = config.profile.clone();

    // Validate filename before processing (I60: path traversal guard)
    let workspace = Path::new(&workspace_path);
    crate::util::validate_inbox_path(workspace, &filename)?;

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
        .read()
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
        .read()
        .map_err(|_| "Internal error")?
        .clone()
        .ok_or("No configuration loaded")?;

    let state = state.inner().clone();
    let workspace_path = config.workspace_path.clone();
    let profile = config.profile.clone();

    // Validate filename before enriching (I60: path traversal guard)
    let workspace = Path::new(&workspace_path);
    crate::util::validate_inbox_path(workspace, &filename)?;

    let user_ctx = crate::types::UserContext::from_config(&config);

    tauri::async_runtime::spawn_blocking(move || {
        let workspace = Path::new(&workspace_path);
        let db_guard = state.db.lock().ok();
        let db_ref = db_guard.as_ref().and_then(|g| g.as_ref());
        crate::processor::enrich::enrich_file(workspace, &filename, db_ref, &profile, Some(&user_ctx))
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
        .read()
        .map_err(|_| "Lock poisoned")?
        .clone()
        .ok_or("No configuration loaded")?;

    let workspace = Path::new(&config.workspace_path);
    let file_path = crate::util::validate_inbox_path(workspace, &filename)?;

    if !file_path.exists() {
        return Err(format!("File not found: {}", filename));
    }

    // Extract text content — works for both text and binary document formats
    use crate::processor::extract;

    let format = extract::detect_format(&file_path);
    if matches!(format, extract::SupportedFormat::Unsupported) {
        // Truly unsupported format — show descriptive message
        let size = std::fs::metadata(&file_path)
            .map(|m| m.len())
            .unwrap_or(0);
        let ext = file_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("unknown");
        return Ok(format!(
            "[Unsupported file — .{} — {} bytes]\n\nText extraction is not available for this format. Use \"Process\" to archive it.",
            ext, size
        ));
    }

    match extract::extract_text(&file_path) {
        Ok(content) => Ok(content),
        Err(e) => {
            // Extraction failed — show error with fallback info
            let size = std::fs::metadata(&file_path)
                .map(|m| m.len())
                .unwrap_or(0);
            let ext = file_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("unknown");
            Ok(format!(
                "[Extraction failed — .{} — {} bytes]\n\nError: {}\n\nUse \"Process\" to let DailyOS handle it.",
                ext, size, e
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
        .read()
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
    let config = match state.config.read() {
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
        .read()
        .map_err(|_| "Lock poisoned")?
        .clone()
        .ok_or("No configuration loaded")?;

    let state_clone = state.inner().clone();
    let workspace_path = config.workspace_path.clone();

    tauri::async_runtime::spawn(async move {
        let workspace = std::path::Path::new(&workspace_path);
        let executor = crate::executor::Executor::new(state_clone, app_handle);
        executor.execute_email_refresh(workspace).await
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

    // If workspace exists, ensure entity dirs are created based on mode
    if !config.workspace_path.is_empty() {
        let workspace = std::path::Path::new(&config.workspace_path);
        if workspace.exists() {
            if mode == "account" || mode == "both" {
                let accounts_dir = workspace.join("Accounts");
                if !accounts_dir.exists() {
                    let _ = std::fs::create_dir_all(&accounts_dir);
                }
            }
            if mode == "project" || mode == "both" {
                let projects_dir = workspace.join("Projects");
                if !projects_dir.exists() {
                    let _ = std::fs::create_dir_all(&projects_dir);
                }
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
        .read()
        .ok()
        .and_then(|g| g.as_ref().map(|c| c.entity_mode.clone()))
        .unwrap_or_else(|| "account".to_string());

    // Scaffold workspace dirs
    crate::state::initialize_workspace(workspace, &entity_mode)?;

    let config = crate::state::create_or_update_config(&state, |config| {
        config.workspace_path = path.clone();
    })?;

    // Sync entities from the new workspace
    if let Ok(db_guard) = state.db.lock() {
        if let Some(db) = db_guard.as_ref() {
            let _ = crate::people::sync_people_from_workspace(
                workspace,
                db,
                config.user_domain.as_deref(),
            );
            let _ = crate::accounts::sync_accounts_from_workspace(workspace, db);
            let _ = crate::projects::sync_projects_from_workspace(workspace, db);
        }
    }

    Ok(config)
}

/// Toggle developer mode (shows/hides devtools panel)
#[tauri::command]
pub fn set_developer_mode(
    enabled: bool,
    state: State<Arc<AppState>>,
) -> Result<Config, String> {
    crate::state::create_or_update_config(&state, |config| {
        config.developer_mode = enabled;
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
        .read()
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

/// Action with resolved account name for list display.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionListItem {
    #[serde(flatten)]
    pub action: crate::db::DbAction,
    pub account_name: Option<String>,
}

/// Get actions from the SQLite database for display.
///
/// Returns pending actions (within `days_ahead` window, default 7) combined
/// with recently completed actions (last 48 hours) so the UI can show both
/// active and done states. Account names are resolved from the accounts table.
#[tauri::command]
pub fn get_actions_from_db(
    days_ahead: Option<i32>,
    state: State<Arc<AppState>>,
) -> Result<Vec<ActionListItem>, String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let mut actions = db
        .get_due_actions(days_ahead.unwrap_or(7))
        .map_err(|e| e.to_string())?;
    let completed = db
        .get_completed_actions(48)
        .map_err(|e| e.to_string())?;
    actions.extend(completed);

    // Batch-resolve account names: collect unique IDs, single query each
    let mut name_cache: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
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

/// Assembled detail for a single past meeting: metadata + captures + actions.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingHistoryDetail {
    pub id: String,
    pub title: String,
    pub meeting_type: String,
    pub start_time: String,
    pub end_time: Option<String>,
    pub account_id: Option<String>,
    pub account_name: Option<String>,
    pub summary: Option<String>,
    pub attendees: Vec<String>,
    pub captures: Vec<crate::db::DbCapture>,
    pub actions: Vec<crate::db::DbAction>,
}

/// Get full detail for a single past meeting by ID.
///
/// Assembles the meeting row, its captures, actions, and resolves the account name.
#[tauri::command]
pub fn get_meeting_history_detail(
    meeting_id: String,
    state: State<Arc<AppState>>,
) -> Result<MeetingHistoryDetail, String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let meeting = db
        .get_meeting_by_id(&meeting_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Meeting not found: {meeting_id}"))?;

    let captures = db
        .get_captures_for_meeting(&meeting_id)
        .map_err(|e| e.to_string())?;

    let actions = db
        .get_actions_for_meeting(&meeting_id)
        .map_err(|e| e.to_string())?;

    // Resolve account name from account_id
    let account_name = if let Some(ref aid) = meeting.account_id {
        db.get_account(aid)
            .ok()
            .flatten()
            .map(|a| a.name)
    } else {
        None
    };

    // Parse attendees from comma-separated string
    let attendees: Vec<String> = meeting
        .attendees
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    Ok(MeetingHistoryDetail {
        id: meeting.id,
        title: meeting.title,
        meeting_type: meeting.meeting_type,
        start_time: meeting.start_time,
        end_time: meeting.end_time,
        account_id: meeting.account_id,
        account_name,
        summary: meeting.summary,
        attendees,
        captures,
        actions,
    })
}

// =============================================================================
// Action Detail
// =============================================================================

/// Enriched action with resolved account name and source meeting title.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionDetail {
    #[serde(flatten)]
    pub action: crate::db::DbAction,
    pub account_name: Option<String>,
    pub source_meeting_title: Option<String>,
}

/// Get full detail for a single action, with resolved relationships.
#[tauri::command]
pub fn get_action_detail(
    action_id: String,
    state: State<Arc<AppState>>,
) -> Result<ActionDetail, String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let action = db
        .get_action_by_id(&action_id)
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
        .read()
        .map_err(|_| "Lock poisoned")?
        .clone()
        .ok_or("No configuration loaded")?;

    let workspace_path = config.workspace_path.clone();

    // Run the native Rust OAuth flow
    let workspace = std::path::Path::new(&workspace_path);
    let email = crate::google::start_auth(workspace).await?;

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
    if let Ok(mut guard) = state.calendar_events.write() {
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
        .read()
        .map(|guard| guard.clone())
        .unwrap_or_default()
}

/// Get the currently active meeting (if any)
#[tauri::command]
pub fn get_current_meeting(state: State<Arc<AppState>>) -> Option<CalendarEvent> {
    let now = chrono::Utc::now();
    state
        .calendar_events
        .read()
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
        .read()
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
        .read()
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
                person_id: None,
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
        .read()
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
    // Check immutability + insert sentinel to prevent TOCTOU race (I61).
    // The sentinel blocks concurrent callers while async work runs below.
    {
        let mut guard = state
            .transcript_processed
            .lock()
            .map_err(|_| "Lock poisoned")?;
        if guard.contains_key(&meeting.id) {
            return Err(format!(
                "Meeting '{}' already has a processed transcript",
                meeting.title
            ));
        }
        // Insert a sentinel record — concurrent calls will now see a key and bail.
        guard.insert(
            meeting.id.clone(),
            crate::types::TranscriptRecord {
                meeting_id: meeting.id.clone(),
                file_path: String::new(),
                destination: String::new(),
                summary: None,
                processed_at: "processing".to_string(),
            },
        );
    }

    let config = state
        .config
        .read()
        .map_err(|_| "Lock poisoned")?
        .clone()
        .ok_or("No configuration loaded")?;

    let state_clone = state.inner().clone();
    let workspace_path = config.workspace_path.clone();
    let profile = config.profile.clone();
    let meeting_id = meeting.id.clone();
    let meeting_clone = meeting.clone();
    let file_path_for_record = file_path.clone();

    let result = match tauri::async_runtime::spawn_blocking(move || {
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
    {
        Ok(r) => r,
        Err(e) => {
            // I61: Remove sentinel on task failure so retry is possible
            if let Ok(mut guard) = state.transcript_processed.lock() {
                guard.remove(&meeting_id);
            }
            return Err(format!("Transcript processing task failed: {}", e));
        }
    };

    // On success, overwrite sentinel with real record.
    // On failure, remove sentinel so retry is possible (I61).
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
    } else {
        // Processing failed — remove sentinel so the user can retry
        if let Ok(mut guard) = state.transcript_processed.lock() {
            guard.remove(&meeting_id);
        }
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
// Manual Action CRUD (I127 / I128)
// =============================================================================

/// Create a new action manually (not from briefing/transcript/inbox).
///
/// Returns the new action's UUID. Priority defaults to P2 if not provided.
#[tauri::command]
pub fn create_action(
    title: String,
    priority: Option<String>,
    due_date: Option<String>,
    account_id: Option<String>,
    project_id: Option<String>,
    person_id: Option<String>,
    context: Option<String>,
    source_label: Option<String>,
    state: State<Arc<AppState>>,
) -> Result<String, String> {
    let priority = priority.unwrap_or_else(|| "P2".to_string());
    if !matches!(priority.as_str(), "P1" | "P2" | "P3") {
        return Err(format!("Invalid priority: {}. Must be P1, P2, or P3.", priority));
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
    };

    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    db.upsert_action(&action).map_err(|e| e.to_string())?;
    Ok(id)
}

/// Update arbitrary fields on an existing action (I128).
///
/// Only provided fields are updated; `None` means "don't touch".
/// To clear a nullable field, pass the corresponding `clear_*` flag.
#[tauri::command]
pub fn update_action(
    id: String,
    title: Option<String>,
    due_date: Option<String>,
    clear_due_date: Option<bool>,
    context: Option<String>,
    clear_context: Option<bool>,
    source_label: Option<String>,
    clear_source_label: Option<bool>,
    account_id: Option<String>,
    clear_account: Option<bool>,
    project_id: Option<String>,
    clear_project: Option<bool>,
    person_id: Option<String>,
    clear_person: Option<bool>,
    priority: Option<String>,
    state: State<Arc<AppState>>,
) -> Result<(), String> {
    if let Some(ref p) = priority {
        if !matches!(p.as_str(), "P1" | "P2" | "P3") {
            return Err(format!("Invalid priority: {}. Must be P1, P2, or P3.", p));
        }
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
        .read()
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
        ("projectTracking", "Project Tracking", "Track projects, milestones, and deliverables", false),
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
        .read()
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
        .read()
        .map_err(|_| "Config lock failed")?
        .as_ref()
        .map(|c| c.workspace_path.clone())
        .ok_or("No workspace configured")?;

    let workspace = std::path::Path::new(&workspace_path);
    let now = chrono::Utc::now().to_rfc3339();

    // 3. Process accounts
    let mut account_count = 0;
    for name in &accounts {
        let name = match crate::util::validate_entity_name(name) {
            Ok(n) => n,
            Err(e) => {
                log::warn!("Skipping invalid account name '{}': {}", name, e);
                continue;
            }
        };

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
            lifecycle: None,
            arr: None,
            health: None,
            contract_start: None,
            contract_end: None,
            csm: None,
            champion: None,
            nps: None,
            tracker_path: Some(format!("Accounts/{}", name)),
            parent_id: None,
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

    // 4. Process projects (I50: full dashboard.json + SQLite)
    let mut project_count = 0;
    for name in &projects {
        let name = match crate::util::validate_entity_name(name) {
            Ok(n) => n,
            Err(e) => {
                log::warn!("Skipping invalid project name '{}': {}", name, e);
                continue;
            }
        };

        let slug = crate::util::slugify(name);
        let db_project = crate::db::DbProject {
            id: slug,
            name: name.to_string(),
            status: "active".to_string(),
            milestone: None,
            owner: None,
            target_date: None,
            tracker_path: Some(format!("Projects/{}", name)),
            updated_at: now.clone(),
        };

        if let Ok(db_guard) = state.db.lock() {
            if let Some(db) = db_guard.as_ref() {
                if let Err(e) = db.upsert_project(&db_project) {
                    log::warn!("Failed to upsert project '{}': {}", name, e);
                }
                // Write dashboard.json + dashboard.md
                let json = crate::projects::default_project_json(&db_project);
                let _ = crate::projects::write_project_json(
                    workspace, &db_project, Some(&json), db,
                );
                let _ = crate::projects::write_project_markdown(
                    workspace, &db_project, Some(&json), db,
                );
            }
        }

        project_count += 1;
    }

    Ok(format!(
        "Created {} accounts, {} projects",
        account_count, project_count
    ))
}

// =============================================================================
// Onboarding: Claude Code Status (I79)
// =============================================================================

/// Check whether Claude Code CLI is installed and authenticated.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeStatus {
    pub installed: bool,
    pub authenticated: bool,
}

#[tauri::command]
pub fn check_claude_status() -> ClaudeStatus {
    let installed = crate::pty::PtyManager::is_claude_available();
    let authenticated = if installed {
        crate::pty::PtyManager::is_claude_authenticated().unwrap_or(false)
    } else {
        false
    };
    ClaudeStatus {
        installed,
        authenticated,
    }
}

// =============================================================================
// Onboarding: Inbox Training Sample (I78)
// =============================================================================

/// Copy a bundled sample meeting notes file into _inbox/ for onboarding training.
///
/// Returns the filename of the installed sample.
#[tauri::command]
pub fn install_inbox_sample(
    state: State<Arc<AppState>>,
) -> Result<String, String> {
    let workspace_path = state
        .config
        .read()
        .map_err(|_| "Config lock failed")?
        .as_ref()
        .map(|c| c.workspace_path.clone())
        .ok_or("No workspace configured")?;

    let workspace = std::path::Path::new(&workspace_path);
    let inbox_dir = workspace.join("_inbox");

    // Ensure _inbox/ exists
    if !inbox_dir.exists() {
        std::fs::create_dir_all(&inbox_dir)
            .map_err(|e| format!("Failed to create _inbox: {}", e))?;
    }

    let filename = "sample-meeting-notes.md";
    let content = include_str!("../resources/sample-meeting-notes.md");
    let dest = inbox_dir.join(filename);

    std::fs::write(&dest, content)
        .map_err(|e| format!("Failed to write sample file: {}", e))?;

    Ok(filename.to_string())
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

/// Daily briefing — mechanical delivery only (no AI).
///
/// Requires `simulate_briefing` scenario first. Delivers schedule, actions,
/// preps, emails, manifest from the seeded today-directive.json.
#[tauri::command]
pub fn dev_run_today_mechanical(
    state: State<Arc<AppState>>,
) -> Result<String, String> {
    if !cfg!(debug_assertions) {
        return Err("Dev tools not available in release builds".into());
    }
    crate::devtools::run_today_mechanical(&state)
}

/// Daily briefing — full pipeline with AI enrichment.
///
/// Requires `simulate_briefing` scenario + Claude Code CLI installed.
/// Mechanical delivery + enrich_emails, enrich_preps, enrich_briefing.
#[tauri::command]
pub fn dev_run_today_full(
    state: State<Arc<AppState>>,
) -> Result<String, String> {
    if !cfg!(debug_assertions) {
        return Err("Dev tools not available in release builds".into());
    }
    crate::devtools::run_today_full(&state)
}

/// Weekly prep — mechanical delivery only (no AI).
///
/// Requires `simulate_briefing` scenario first. Delivers week-overview.json
/// from the seeded week-directive.json.
#[tauri::command]
pub fn dev_run_week_mechanical(
    state: State<Arc<AppState>>,
) -> Result<String, String> {
    if !cfg!(debug_assertions) {
        return Err("Dev tools not available in release builds".into());
    }
    crate::devtools::run_week_mechanical(&state)
}

/// Weekly prep — full pipeline with AI enrichment.
///
/// Requires `simulate_briefing` scenario + Claude Code CLI installed.
/// Runs Claude /week then delivers week-overview.json.
#[tauri::command]
pub fn dev_run_week_full(
    state: State<Arc<AppState>>,
) -> Result<String, String> {
    if !cfg!(debug_assertions) {
        return Err("Dev tools not available in release builds".into());
    }
    crate::devtools::run_week_full(&state)
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
        .read()
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
            .read()
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

/// Get all people with pre-computed signals, optionally filtered by relationship.
#[tauri::command]
pub fn get_people(
    relationship: Option<String>,
    state: State<Arc<AppState>>,
) -> Result<Vec<crate::db::PersonListItem>, String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    db.get_people_with_signals(relationship.as_deref())
        .map_err(|e| e.to_string())
}

/// Person detail result including signals, linked entities, and recent meetings.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonDetailResult {
    #[serde(flatten)]
    pub person: crate::db::DbPerson,
    pub signals: Option<crate::db::PersonSignals>,
    pub entities: Vec<EntitySummary>,
    pub recent_meetings: Vec<MeetingSummary>,
    pub intelligence: Option<crate::entity_intel::IntelligenceJson>,
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
    pub meeting_type: String,
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
            meeting_type: m.meeting_type,
        })
        .collect();

    // Load intelligence from person dir (if exists)
    let intelligence = {
        let config = state.config.read().map_err(|_| "Lock poisoned")?;
        if let Some(ref config) = *config {
            let person_dir = crate::people::person_dir(
                Path::new(&config.workspace_path),
                &person.name,
            );
            crate::entity_intel::read_intelligence_json(&person_dir).ok()
        } else {
            None
        }
    };

    Ok(PersonDetailResult {
        person,
        signals,
        entities,
        recent_meetings,
        intelligence,
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
        let config = state.config.read().map_err(|_| "Lock poisoned")?;
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
        let config = state.config.read().map_err(|_| "Lock poisoned")?;
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
        let config = state.config.read().map_err(|_| "Lock poisoned")?;
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

// =========================================================================
// Meeting-Entity M2M (I52)
// =========================================================================

/// Link a meeting to an entity (account/project) via the junction table.
#[tauri::command]
pub fn link_meeting_entity(
    meeting_id: String,
    entity_id: String,
    entity_type: String,
    state: State<Arc<AppState>>,
) -> Result<(), String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    db.link_meeting_entity(&meeting_id, &entity_id, &entity_type)
        .map_err(|e| e.to_string())
}

/// Remove a meeting-entity link from the junction table.
#[tauri::command]
pub fn unlink_meeting_entity(
    meeting_id: String,
    entity_id: String,
    state: State<Arc<AppState>>,
) -> Result<(), String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    db.unlink_meeting_entity(&meeting_id, &entity_id)
        .map_err(|e| e.to_string())
}

/// Get all entities linked to a meeting via the junction table.
#[tauri::command]
pub fn get_meeting_entities(
    meeting_id: String,
    state: State<Arc<AppState>>,
) -> Result<Vec<crate::entity::DbEntity>, String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    db.get_meeting_entities(&meeting_id)
        .map_err(|e| e.to_string())
}

// =========================================================================
// Person Creation (I129)
// =========================================================================

/// Create a new person manually. Returns the generated person ID.
#[tauri::command]
pub fn create_person(
    email: String,
    name: String,
    organization: Option<String>,
    role: Option<String>,
    relationship: Option<String>,
    state: State<Arc<AppState>>,
) -> Result<String, String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let id = crate::util::slugify(&email);
    let now = chrono::Utc::now().to_rfc3339();

    let person = crate::db::DbPerson {
        id: id.clone(),
        email,
        name,
        organization,
        role,
        relationship: relationship.unwrap_or_else(|| "unknown".to_string()),
        notes: None,
        tracker_path: None,
        last_seen: None,
        first_seen: Some(now.clone()),
        meeting_count: 0,
        updated_at: now,
    };

    db.upsert_person(&person).map_err(|e| e.to_string())?;
    Ok(id)
}

/// Enrich a person with intelligence assessment (relationship intelligence).
#[tauri::command]
pub async fn enrich_person(
    person_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::entity_intel::IntelligenceJson, String> {
    let workspace_path = {
        let guard = state.config.read().map_err(|_| "Lock poisoned")?;
        let config = guard.as_ref().ok_or("Config not loaded")?;
        config.workspace_path.clone()
    };

    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let person = db
        .get_person(&person_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Person not found: {}", person_id))?;

    let pty = crate::pty::PtyManager::new().with_timeout(180);
    crate::entity_intel::enrich_entity_intelligence(
        std::path::Path::new(&workspace_path),
        db,
        &person_id,
        &person.name,
        "person",
        None,
        None,
        &pty,
    )
}

// =============================================================================
// I72: Account Dashboards
// =============================================================================

/// Account list item with computed fields for the list page.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountListItem {
    pub id: String,
    pub name: String,
    pub lifecycle: Option<String>,
    pub arr: Option<f64>,
    pub health: Option<String>,
    pub nps: Option<i32>,
    pub csm: Option<String>,
    pub champion: Option<String>,
    pub renewal_date: Option<String>,
    pub open_action_count: usize,
    pub days_since_last_meeting: Option<i64>,
    pub parent_id: Option<String>,
    pub parent_name: Option<String>,
    pub child_count: usize,
    pub is_parent: bool,
}

/// Full account detail for the detail page.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountDetailResult {
    pub id: String,
    pub name: String,
    pub lifecycle: Option<String>,
    pub arr: Option<f64>,
    pub health: Option<String>,
    pub nps: Option<i32>,
    pub csm: Option<String>,
    pub champion: Option<String>,
    pub renewal_date: Option<String>,
    pub contract_start: Option<String>,
    pub company_overview: Option<crate::accounts::CompanyOverview>,
    pub strategic_programs: Vec<crate::accounts::StrategicProgram>,
    pub notes: Option<String>,
    pub open_actions: Vec<crate::db::DbAction>,
    pub upcoming_meetings: Vec<MeetingSummary>,
    pub recent_meetings: Vec<MeetingSummary>,
    pub linked_people: Vec<crate::db::DbPerson>,
    pub signals: Option<crate::db::StakeholderSignals>,
    pub recent_captures: Vec<crate::db::DbCapture>,
    pub parent_id: Option<String>,
    pub parent_name: Option<String>,
    pub children: Vec<AccountChildSummary>,
    pub parent_aggregate: Option<crate::db::ParentAggregate>,
    /// Entity intelligence (ADR-0057) — synthesized assessment from enrichment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intelligence: Option<crate::entity_intel::IntelligenceJson>,
}

/// Compact child account summary for parent detail pages (I114).
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountChildSummary {
    pub id: String,
    pub name: String,
    pub health: Option<String>,
    pub arr: Option<f64>,
    pub open_action_count: usize,
}

/// Get top-level accounts with computed summary fields for the list page (I114).
///
/// Returns only accounts where `parent_id IS NULL`. Each parent account
/// includes a `child_count` so the UI can show an expand chevron.
#[tauri::command]
pub fn get_accounts_list(
    state: State<Arc<AppState>>,
) -> Result<Vec<AccountListItem>, String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let accounts = db.get_top_level_accounts().map_err(|e| e.to_string())?;

    let items: Vec<AccountListItem> = accounts
        .into_iter()
        .map(|a| {
            let child_count = db
                .get_child_accounts(&a.id)
                .map(|c| c.len())
                .unwrap_or(0);

            account_to_list_item(&a, db, child_count)
        })
        .collect();

    Ok(items)
}

/// Lightweight list of ALL accounts (parents + children) for entity pickers.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PickerAccount {
    pub id: String,
    pub name: String,
    pub parent_name: Option<String>,
}

#[tauri::command]
pub fn get_accounts_for_picker(
    state: State<Arc<AppState>>,
) -> Result<Vec<PickerAccount>, String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let all = db.get_all_accounts().map_err(|e| e.to_string())?;

    // Build a parent name lookup from the same list
    let parent_names: std::collections::HashMap<String, String> = all
        .iter()
        .filter(|a| a.parent_id.is_none())
        .map(|a| (a.id.clone(), a.name.clone()))
        .collect();

    let items: Vec<PickerAccount> = all
        .into_iter()
        .map(|a| {
            let parent_name = a.parent_id.as_ref().and_then(|pid| parent_names.get(pid).cloned());
            PickerAccount {
                id: a.id,
                name: a.name,
                parent_name,
            }
        })
        .collect();

    Ok(items)
}

/// Get child accounts for a parent (I114).
#[tauri::command]
pub fn get_child_accounts_list(
    parent_id: String,
    state: State<Arc<AppState>>,
) -> Result<Vec<AccountListItem>, String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let children = db.get_child_accounts(&parent_id).map_err(|e| e.to_string())?;

    // Look up parent name for breadcrumb context
    let parent_name = db
        .get_account(&parent_id)
        .ok()
        .flatten()
        .map(|a| a.name);

    let items: Vec<AccountListItem> = children
        .into_iter()
        .map(|a| {
            let mut item = account_to_list_item(&a, db, 0);
            item.parent_name = parent_name.clone();
            item
        })
        .collect();

    Ok(items)
}

/// Convert a DbAccount to an AccountListItem with computed signals.
fn account_to_list_item(
    a: &crate::db::DbAccount,
    db: &crate::db::ActionDb,
    child_count: usize,
) -> AccountListItem {
    let open_action_count = db
        .get_account_actions(&a.id)
        .map(|actions| actions.len())
        .unwrap_or(0);

    let signals = db.get_stakeholder_signals(&a.id).ok();
    let days_since_last_meeting = signals.as_ref().and_then(|s| {
        s.last_meeting.as_ref().and_then(|lm| {
            chrono::DateTime::parse_from_rfc3339(lm)
                .or_else(|_| {
                    chrono::DateTime::parse_from_rfc3339(
                        &format!("{}+00:00", lm.trim_end_matches('Z')),
                    )
                })
                .ok()
                .map(|dt| {
                    (chrono::Utc::now() - dt.with_timezone(&chrono::Utc)).num_days()
                })
        })
    });

    AccountListItem {
        id: a.id.clone(),
        name: a.name.clone(),
        lifecycle: a.lifecycle.clone(),
        arr: a.arr,
        health: a.health.clone(),
        nps: a.nps,
        csm: a.csm.clone(),
        champion: a.champion.clone(),
        renewal_date: a.contract_end.clone(),
        open_action_count,
        days_since_last_meeting,
        parent_id: a.parent_id.clone(),
        parent_name: None,
        child_count,
        is_parent: child_count > 0,
    }
}

/// Get full detail for an account (DB fields + narrative JSON + computed data).
#[tauri::command]
pub fn get_account_detail(
    account_id: String,
    state: State<Arc<AppState>>,
) -> Result<AccountDetailResult, String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let account = db
        .get_account(&account_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Account not found: {}", account_id))?;

    // Read narrative fields from dashboard.json + intelligence.json if they exist
    let config = state.config.read().map_err(|_| "Lock poisoned")?;
    let (overview, programs, notes, intelligence) = if let Some(ref config) = *config {
        let workspace = Path::new(&config.workspace_path);
        let account_dir = crate::accounts::resolve_account_dir(workspace, &account);
        let json_path = account_dir.join("dashboard.json");
        let (ov, prg, nt) = if json_path.exists() {
            match crate::accounts::read_account_json(&json_path) {
                Ok(result) => (
                    result.json.company_overview,
                    result.json.strategic_programs,
                    result.json.notes,
                ),
                Err(_) => (None, Vec::new(), None),
            }
        } else {
            (None, Vec::new(), None)
        };
        // Read intelligence.json (ADR-0057), migrate from CompanyOverview if needed
        let intel = crate::entity_intel::read_intelligence_json(&account_dir).ok().or_else(|| {
            // Auto-migrate from legacy CompanyOverview on first access
            ov.as_ref().and_then(|overview| {
                crate::entity_intel::migrate_company_overview_to_intelligence(
                    workspace, &account, overview,
                )
            })
        });
        (ov, prg, nt, intel)
    } else {
        (None, Vec::new(), None, None)
    };
    drop(config); // Release config lock before more DB queries

    let open_actions = db
        .get_account_actions(&account_id)
        .map_err(|e| e.to_string())?;

    let upcoming_meetings: Vec<MeetingSummary> = db
        .get_upcoming_meetings_for_account(&account_id, 5)
        .unwrap_or_default()
        .into_iter()
        .map(|m| MeetingSummary {
            id: m.id,
            title: m.title,
            start_time: m.start_time,
            meeting_type: m.meeting_type,
        })
        .collect();

    let recent_meetings = db
        .get_meetings_for_account(&account_id, 10)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|m| MeetingSummary {
            id: m.id,
            title: m.title,
            start_time: m.start_time,
            meeting_type: m.meeting_type,
        })
        .collect();

    let linked_people = db
        .get_people_for_entity(&account_id)
        .unwrap_or_default();

    let signals = db.get_stakeholder_signals(&account_id).ok();

    let recent_captures = db
        .get_captures_for_account(&account_id, 90)
        .unwrap_or_default();

    // I114: Resolve parent name for child accounts, children for parent accounts
    let parent_name = account.parent_id.as_ref().and_then(|pid| {
        db.get_account(pid).ok().flatten().map(|a| a.name)
    });

    let child_accounts = db.get_child_accounts(&account.id).unwrap_or_default();
    let parent_aggregate = if !child_accounts.is_empty() {
        db.get_parent_aggregate(&account.id).ok()
    } else {
        None
    };
    let children: Vec<AccountChildSummary> = child_accounts
        .iter()
        .map(|child| {
            let open_action_count = db
                .get_account_actions(&child.id)
                .map(|a| a.len())
                .unwrap_or(0);
            AccountChildSummary {
                id: child.id.clone(),
                name: child.name.clone(),
                health: child.health.clone(),
                arr: child.arr,
                open_action_count,
            }
        })
        .collect();

    Ok(AccountDetailResult {
        id: account.id,
        name: account.name,
        lifecycle: account.lifecycle,
        arr: account.arr,
        health: account.health,
        nps: account.nps,
        csm: account.csm,
        champion: account.champion,
        renewal_date: account.contract_end,
        contract_start: account.contract_start,
        company_overview: overview,
        strategic_programs: programs,
        notes,
        open_actions,
        upcoming_meetings,
        recent_meetings,
        linked_people,
        signals,
        recent_captures,
        parent_id: account.parent_id,
        parent_name,
        children,
        parent_aggregate,
        intelligence,
    })
}

/// Update a single structured field on an account.
/// Writes to SQLite, then regenerates dashboard.json + dashboard.md.
#[tauri::command]
pub fn update_account_field(
    account_id: String,
    field: String,
    value: String,
    state: State<Arc<AppState>>,
) -> Result<(), String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    db.update_account_field(&account_id, &field, &value)
        .map_err(|e| e.to_string())?;

    // Regenerate workspace files
    if let Ok(Some(account)) = db.get_account(&account_id) {
        let config = state.config.read().map_err(|_| "Lock poisoned")?;
        if let Some(ref config) = *config {
            let workspace = Path::new(&config.workspace_path);
            // Read existing JSON to preserve narrative fields
            let json_path = crate::accounts::resolve_account_dir(workspace, &account)
                .join("dashboard.json");
            let existing = if json_path.exists() {
                crate::accounts::read_account_json(&json_path)
                    .ok()
                    .map(|r| r.json)
            } else {
                None
            };
            let _ = crate::accounts::write_account_json(
                workspace,
                &account,
                existing.as_ref(),
                db,
            );
            let _ = crate::accounts::write_account_markdown(
                workspace,
                &account,
                existing.as_ref(),
                db,
            );
        }
    }

    Ok(())
}

/// Update account notes (narrative field — JSON only, not SQLite).
/// Writes dashboard.json + regenerates dashboard.md.
#[tauri::command]
pub fn update_account_notes(
    account_id: String,
    notes: String,
    state: State<Arc<AppState>>,
) -> Result<(), String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let account = db
        .get_account(&account_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Account not found: {}", account_id))?;

    let config = state.config.read().map_err(|_| "Lock poisoned")?;
    let config = config.as_ref().ok_or("Config not loaded")?;
    let workspace = Path::new(&config.workspace_path);

    // Read existing JSON
    let json_path = crate::accounts::resolve_account_dir(workspace, &account)
        .join("dashboard.json");
    let mut existing = if json_path.exists() {
        crate::accounts::read_account_json(&json_path)
            .map(|r| r.json)
            .unwrap_or_else(|_| default_account_json(&account))
    } else {
        default_account_json(&account)
    };

    // Update notes
    existing.notes = if notes.is_empty() { None } else { Some(notes) };

    let _ = crate::accounts::write_account_json(workspace, &account, Some(&existing), db);
    let _ = crate::accounts::write_account_markdown(workspace, &account, Some(&existing), db);

    Ok(())
}

/// Update account strategic programs (narrative field — JSON only).
/// Writes dashboard.json + regenerates dashboard.md.
#[tauri::command]
pub fn update_account_programs(
    account_id: String,
    programs_json: String,
    state: State<Arc<AppState>>,
) -> Result<(), String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let account = db
        .get_account(&account_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Account not found: {}", account_id))?;

    let programs: Vec<crate::accounts::StrategicProgram> =
        serde_json::from_str(&programs_json)
            .map_err(|e| format!("Invalid programs JSON: {}", e))?;

    let config = state.config.read().map_err(|_| "Lock poisoned")?;
    let config = config.as_ref().ok_or("Config not loaded")?;
    let workspace = Path::new(&config.workspace_path);

    let json_path = crate::accounts::resolve_account_dir(workspace, &account)
        .join("dashboard.json");
    let mut existing = if json_path.exists() {
        crate::accounts::read_account_json(&json_path)
            .map(|r| r.json)
            .unwrap_or_else(|_| default_account_json(&account))
    } else {
        default_account_json(&account)
    };

    existing.strategic_programs = programs;

    let _ = crate::accounts::write_account_json(workspace, &account, Some(&existing), db);
    let _ = crate::accounts::write_account_markdown(workspace, &account, Some(&existing), db);

    Ok(())
}

/// Create a new account. Creates SQLite record + workspace files.
/// If `parent_id` is provided, creates a child (BU) account under that parent.
#[tauri::command]
pub fn create_account(
    name: String,
    parent_id: Option<String>,
    state: State<Arc<AppState>>,
) -> Result<String, String> {
    // I60: validate name before using as directory
    let name = crate::util::validate_entity_name(&name)?.to_string();

    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    // Derive ID and tracker_path based on whether this is a child account
    let (id, tracker_path) = if let Some(ref pid) = parent_id {
        let parent = db
            .get_account(pid)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Parent account not found: {}", pid))?;
        let child_id = format!("{}--{}", pid, crate::util::slugify(&name));
        let parent_dir = parent.tracker_path.unwrap_or_else(|| format!("Accounts/{}", parent.name));
        let tp = format!("{}/{}", parent_dir, name);
        (child_id, tp)
    } else {
        let id = crate::util::slugify(&name);
        (id, format!("Accounts/{}", name))
    };

    let now = chrono::Utc::now().to_rfc3339();

    let account = crate::db::DbAccount {
        id: id.clone(),
        name: name.clone(),
        lifecycle: None,
        arr: None,
        health: None,
        contract_start: None,
        contract_end: None,
        csm: None,
        champion: None,
        nps: None,
        tracker_path: Some(tracker_path),
        parent_id,
        updated_at: now,
    };

    db.upsert_account(&account).map_err(|e| e.to_string())?;

    // Create workspace files
    let config = state.config.read().map_err(|_| "Lock poisoned")?;
    if let Some(ref config) = *config {
        let workspace = Path::new(&config.workspace_path);
        let _ = crate::accounts::write_account_json(workspace, &account, None, db);
        let _ = crate::accounts::write_account_markdown(workspace, &account, None, db);
    }

    Ok(id)
}

// =============================================================================
// I124: Content Index
// =============================================================================

/// Get indexed files for an entity.
#[tauri::command]
pub fn get_entity_files(
    entity_id: String,
    state: State<Arc<AppState>>,
) -> Result<Vec<crate::db::DbContentFile>, String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    db.get_entity_files(&entity_id).map_err(|e| e.to_string())
}

/// Re-scan an entity's directory and return the updated file list.
#[tauri::command]
pub fn index_entity_files(
    entity_id: String,
    state: State<Arc<AppState>>,
) -> Result<Vec<crate::db::DbContentFile>, String> {
    let config = state.config.read().map_err(|_| "Lock poisoned")?;
    let workspace_path = config
        .as_ref()
        .ok_or("Config not loaded")?
        .workspace_path
        .clone();
    let workspace = Path::new(&workspace_path);

    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let account = db
        .get_account(&entity_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Account not found: {}", entity_id))?;

    crate::accounts::sync_content_index_for_account(workspace, db, &account)?;
    db.get_entity_files(&entity_id).map_err(|e| e.to_string())
}

/// Reveal a file in macOS Finder.
#[tauri::command]
pub fn reveal_in_finder(path: String) -> Result<(), String> {
    std::process::Command::new("open")
        .arg("-R")
        .arg(&path)
        .spawn()
        .map_err(|e| format!("Failed to open Finder: {}", e))?;
    Ok(())
}

// ── I74/I131: Entity Intelligence Enrichment via Claude Code ────────

#[tauri::command]
pub async fn enrich_account(
    account_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::entity_intel::IntelligenceJson, String> {
    let workspace_path = {
        let guard = state.config.read().map_err(|_| "Lock poisoned")?;
        let config = guard.as_ref().ok_or("Config not loaded")?;
        config.workspace_path.clone()
    };

    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let account = db
        .get_account(&account_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Account not found: {}", account_id))?;

    let pty = crate::pty::PtyManager::new().with_timeout(180);
    crate::entity_intel::enrich_entity_intelligence(
        std::path::Path::new(&workspace_path),
        db,
        &account_id,
        &account.name,
        "account",
        Some(&account),
        None,
        &pty,
    )
}

// =============================================================================
// I50: Project Dashboards
// =============================================================================

/// Project list item with computed fields for the list page.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectListItem {
    pub id: String,
    pub name: String,
    pub status: String,
    pub milestone: Option<String>,
    pub owner: Option<String>,
    pub target_date: Option<String>,
    pub open_action_count: usize,
    pub days_since_last_meeting: Option<i64>,
}

/// Full project detail for the detail page.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDetailResult {
    pub id: String,
    pub name: String,
    pub status: String,
    pub milestone: Option<String>,
    pub owner: Option<String>,
    pub target_date: Option<String>,
    pub description: Option<String>,
    pub milestones: Vec<crate::projects::ProjectMilestone>,
    pub notes: Option<String>,
    pub open_actions: Vec<crate::db::DbAction>,
    pub recent_meetings: Vec<MeetingSummary>,
    pub linked_people: Vec<crate::db::DbPerson>,
    pub signals: Option<crate::db::ProjectSignals>,
    pub recent_captures: Vec<crate::db::DbCapture>,
    /// Entity intelligence (ADR-0057) — synthesized assessment from enrichment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intelligence: Option<crate::entity_intel::IntelligenceJson>,
}

/// Get all projects with computed summary fields for the list page.
#[tauri::command]
pub fn get_projects_list(
    state: State<Arc<AppState>>,
) -> Result<Vec<ProjectListItem>, String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let projects = db.get_all_projects().map_err(|e| e.to_string())?;

    let items: Vec<ProjectListItem> = projects
        .into_iter()
        .map(|p| {
            let open_action_count = db
                .get_project_actions(&p.id)
                .map(|a| a.len())
                .unwrap_or(0);
            let days_since_last_meeting = db
                .get_project_signals(&p.id)
                .ok()
                .and_then(|s| {
                    s.last_meeting
                        .as_ref()
                        .and_then(|lm| {
                            chrono::DateTime::parse_from_rfc3339(lm)
                                .ok()
                                .map(|dt| {
                                    (chrono::Utc::now() - dt.with_timezone(&chrono::Utc))
                                        .num_days()
                                })
                        })
                });
            ProjectListItem {
                id: p.id,
                name: p.name,
                status: p.status,
                milestone: p.milestone,
                owner: p.owner,
                target_date: p.target_date,
                open_action_count,
                days_since_last_meeting,
            }
        })
        .collect();

    Ok(items)
}

/// Get full detail for a project.
#[tauri::command]
pub fn get_project_detail(
    project_id: String,
    state: State<Arc<AppState>>,
) -> Result<ProjectDetailResult, String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let project = db
        .get_project(&project_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Project not found: {}", project_id))?;

    // Read narrative fields from dashboard.json + intelligence.json if they exist
    let config = state.config.read().map_err(|_| "Lock poisoned")?;
    let (description, milestones, notes, intelligence) = if let Some(ref config) = *config {
        let workspace = Path::new(&config.workspace_path);
        let project_dir = crate::projects::project_dir(workspace, &project.name);
        let json_path = project_dir.join("dashboard.json");
        let (desc, ms, nt) = if json_path.exists() {
            match crate::projects::read_project_json(&json_path) {
                Ok(result) => (
                    result.json.description,
                    result.json.milestones,
                    result.json.notes,
                ),
                Err(_) => (None, Vec::new(), None),
            }
        } else {
            (None, Vec::new(), None)
        };
        let intel = crate::entity_intel::read_intelligence_json(&project_dir).ok();
        (desc, ms, nt, intel)
    } else {
        (None, Vec::new(), None, None)
    };
    drop(config);

    let open_actions = db
        .get_project_actions(&project_id)
        .map_err(|e| e.to_string())?;

    let recent_meetings = db
        .get_meetings_for_project(&project_id, 10)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|m| MeetingSummary {
            id: m.id,
            title: m.title,
            start_time: m.start_time,
            meeting_type: m.meeting_type,
        })
        .collect();

    let linked_people = db
        .get_people_for_entity(&project_id)
        .unwrap_or_default();

    let signals = db.get_project_signals(&project_id).ok();

    // Get captures linked to project meetings
    let recent_captures = db.get_captures_for_project(&project_id, 90).unwrap_or_default();

    Ok(ProjectDetailResult {
        id: project.id,
        name: project.name,
        status: project.status,
        milestone: project.milestone,
        owner: project.owner,
        target_date: project.target_date,
        description,
        milestones,
        notes,
        open_actions,
        recent_meetings,
        linked_people,
        signals,
        recent_captures,
        intelligence,
    })
}

/// Create a new project.
#[tauri::command]
pub fn create_project(
    name: String,
    state: State<Arc<AppState>>,
) -> Result<String, String> {
    let validated_name = crate::util::validate_entity_name(&name)?;
    let id = crate::util::slugify(validated_name);
    let now = chrono::Utc::now().to_rfc3339();

    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    // Check for duplicate
    if let Ok(Some(_)) = db.get_project(&id) {
        return Err(format!("Project '{}' already exists", validated_name));
    }

    let project = crate::db::DbProject {
        id: id.clone(),
        name: validated_name.to_string(),
        status: "active".to_string(),
        milestone: None,
        owner: None,
        target_date: None,
        tracker_path: Some(format!("Projects/{}", validated_name)),
        updated_at: now,
    };

    db.upsert_project(&project).map_err(|e| e.to_string())?;

    // Create workspace files
    let config = state.config.read().map_err(|_| "Lock poisoned")?;
    if let Some(ref config) = *config {
        let workspace = Path::new(&config.workspace_path);
        let _ = crate::projects::write_project_json(workspace, &project, None, db);
        let _ = crate::projects::write_project_markdown(workspace, &project, None, db);
    }

    Ok(id)
}

/// Update a single structured field on a project.
#[tauri::command]
pub fn update_project_field(
    project_id: String,
    field: String,
    value: String,
    state: State<Arc<AppState>>,
) -> Result<(), String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    db.update_project_field(&project_id, &field, &value)
        .map_err(|e| e.to_string())?;

    // Regenerate workspace files
    if let Ok(Some(project)) = db.get_project(&project_id) {
        let config = state.config.read().map_err(|_| "Lock poisoned")?;
        if let Some(ref config) = *config {
            let workspace = Path::new(&config.workspace_path);
            let json_path = crate::projects::project_dir(workspace, &project.name)
                .join("dashboard.json");
            let existing_json = if json_path.exists() {
                crate::projects::read_project_json(&json_path)
                    .ok()
                    .map(|r| r.json)
            } else {
                None
            };
            let _ = crate::projects::write_project_json(
                workspace,
                &project,
                existing_json.as_ref(),
                db,
            );
            let _ = crate::projects::write_project_markdown(
                workspace,
                &project,
                existing_json.as_ref(),
                db,
            );
        }
    }

    Ok(())
}

/// Update the notes field on a project.
#[tauri::command]
pub fn update_project_notes(
    project_id: String,
    notes: String,
    state: State<Arc<AppState>>,
) -> Result<(), String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let project = db
        .get_project(&project_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Project not found: {}", project_id))?;

    let config = state.config.read().map_err(|_| "Lock poisoned")?;
    if let Some(ref config) = *config {
        let workspace = Path::new(&config.workspace_path);
        let json_path = crate::projects::project_dir(workspace, &project.name)
            .join("dashboard.json");

        let mut json = if json_path.exists() {
            crate::projects::read_project_json(&json_path)
                .map(|r| r.json)
                .unwrap_or_else(|_| crate::projects::default_project_json(&project))
        } else {
            crate::projects::default_project_json(&project)
        };

        json.notes = if notes.is_empty() { None } else { Some(notes) };

        crate::projects::write_project_json(workspace, &project, Some(&json), db)?;
        crate::projects::write_project_markdown(workspace, &project, Some(&json), db)?;
    }

    Ok(())
}

/// Enrich a project via Claude Code intelligence enrichment.
#[tauri::command]
pub async fn enrich_project(
    project_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::entity_intel::IntelligenceJson, String> {
    let workspace_path = {
        let guard = state.config.read().map_err(|_| "Lock poisoned")?;
        let config = guard.as_ref().ok_or("Config not loaded")?;
        config.workspace_path.clone()
    };

    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let project = db
        .get_project(&project_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Project not found: {}", project_id))?;

    let pty = crate::pty::PtyManager::new().with_timeout(180);
    crate::entity_intel::enrich_entity_intelligence(
        std::path::Path::new(&workspace_path),
        db,
        &project_id,
        &project.name,
        "project",
        None,
        Some(&project),
        &pty,
    )
}

// ── I76: Database Backup & Rebuild ──────────────────────────────────

#[tauri::command]
pub async fn backup_database(state: tauri::State<'_, Arc<AppState>>) -> Result<String, String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    crate::db_backup::backup_database(db)
}

#[tauri::command]
pub async fn rebuild_database(state: tauri::State<'_, Arc<AppState>>) -> Result<(usize, usize, usize), String> {
    let (workspace_path, user_domain) = {
        let guard = state.config.read().map_err(|_| "Lock poisoned")?;
        let config = guard.as_ref().ok_or("Config not loaded")?;
        (config.workspace_path.clone(), config.user_domain.clone())
    };

    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    crate::db_backup::rebuild_from_filesystem(
        std::path::Path::new(&workspace_path),
        db,
        user_domain.as_deref(),
    )
}

/// Helper: create a default AccountJson from a DbAccount.
fn default_account_json(account: &crate::db::DbAccount) -> crate::accounts::AccountJson {
    crate::accounts::AccountJson {
        version: 1,
        entity_type: "account".to_string(),
        structured: crate::accounts::AccountStructured {
            arr: account.arr,
            health: account.health.clone(),
            lifecycle: account.lifecycle.clone(),
            renewal_date: account.contract_end.clone(),
            nps: account.nps,
            csm: account.csm.clone(),
            champion: account.champion.clone(),
        },
        company_overview: None,
        strategic_programs: Vec::new(),
        notes: None,
        custom_sections: Vec::new(),
        parent_id: account.parent_id.clone(),
    }
}
