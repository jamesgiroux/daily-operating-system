//! Google authentication and calendar polling
//!
//! - OAuth flow via Python subprocess (reuses existing google_api.py patterns)
//! - Calendar polling loop: every N minutes during work hours
//! - Events stored in AppState, frontend notified via Tauri events

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use chrono::{Timelike, Utc};
use tauri::{AppHandle, Emitter};

use crate::pty::run_python_script;
use crate::state::{google_token_path, AppState};
use crate::types::{CalendarEvent, GoogleAuthStatus, MeetingType};

/// Run the Google OAuth flow via a Python script.
///
/// Opens the user's browser, captures the redirect, saves the token.
/// Returns the authenticated email on success.
pub fn start_auth(workspace: &Path) -> Result<String, String> {
    let script = find_script(workspace, "google_auth.py")
        .ok_or_else(|| "google_auth.py not found".to_string())?;

    let output = run_python_script(&script, workspace, 120)
        .map_err(|e| format!("Google auth failed: {}", e))?;

    // Parse the JSON output from the script
    let result: serde_json::Value = serde_json::from_str(&output.stdout)
        .map_err(|e| format!("Failed to parse auth output: {}", e))?;

    match result.get("status").and_then(|s| s.as_str()) {
        Some("success") => {
            let email = result
                .get("email")
                .and_then(|e| e.as_str())
                .unwrap_or("unknown")
                .to_string();
            Ok(email)
        }
        Some("error") => {
            let msg = result
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown error");
            Err(msg.to_string())
        }
        _ => Err("Unexpected auth script output".to_string()),
    }
}

/// Disconnect Google by removing the token file.
pub fn disconnect() -> Result<(), String> {
    let token_path = google_token_path();
    if token_path.exists() {
        std::fs::remove_file(&token_path)
            .map_err(|e| format!("Failed to remove token: {}", e))?;
    }
    Ok(())
}

/// Poll calendar events from Google via Python script.
///
/// Returns parsed events or an error. Exit code 2 from the script
/// indicates an auth failure (token expired/revoked).
fn poll_calendar(workspace: &Path) -> Result<Vec<CalendarEvent>, PollError> {
    let script = find_script(workspace, "calendar_poll.py")
        .ok_or(PollError::ScriptNotFound)?;

    let output = run_python_script(&script, workspace, 30).map_err(|e| {
        let err_str = e.to_string();
        if err_str.contains("exit code 2") || err_str.contains("exit code: 2") {
            PollError::AuthExpired
        } else {
            PollError::ScriptError(err_str)
        }
    })?;

    // Check exit code for auth failure
    if output.exit_code == 2 {
        return Err(PollError::AuthExpired);
    }

    let raw_events: Vec<RawCalendarEvent> = serde_json::from_str(&output.stdout)
        .map_err(|e| PollError::ParseError(format!("Failed to parse calendar JSON: {}", e)))?;

    let events = raw_events.into_iter().map(|e| e.into()).collect();
    Ok(events)
}

/// Calendar polling errors
enum PollError {
    ScriptNotFound,
    AuthExpired,
    ScriptError(String),
    ParseError(String),
}

/// Raw event from the Python script (before classification)
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawCalendarEvent {
    id: String,
    title: String,
    start: chrono::DateTime<Utc>,
    end: chrono::DateTime<Utc>,
    #[serde(default)]
    meeting_type: Option<String>,
    account: Option<String>,
    #[serde(default)]
    attendees: Vec<String>,
    #[serde(default)]
    is_all_day: bool,
}

impl From<RawCalendarEvent> for CalendarEvent {
    fn from(raw: RawCalendarEvent) -> Self {
        let meeting_type = match raw.meeting_type.as_deref() {
            Some("customer") => MeetingType::Customer,
            Some("qbr") => MeetingType::Qbr,
            Some("training") => MeetingType::Training,
            Some("internal") => MeetingType::Internal,
            Some("team_sync") => MeetingType::TeamSync,
            Some("one_on_one") => MeetingType::OneOnOne,
            Some("partnership") => MeetingType::Partnership,
            Some("all_hands") => MeetingType::AllHands,
            Some("external") => MeetingType::External,
            Some("personal") => MeetingType::Personal,
            _ => MeetingType::Internal,
        };

        CalendarEvent {
            id: raw.id,
            title: raw.title,
            start: raw.start,
            end: raw.end,
            meeting_type,
            account: raw.account,
            attendees: raw.attendees,
            is_all_day: raw.is_all_day,
        }
    }
}

/// Start the calendar polling loop.
///
/// Runs as an async task — polls every N minutes during work hours.
/// Updates AppState with events and emits `calendar-updated` to the frontend.
pub async fn run_calendar_poller(state: Arc<AppState>, app_handle: AppHandle) {
    loop {
        let interval = get_poll_interval(&state);
        tokio::time::sleep(Duration::from_secs(interval * 60)).await;

        // Check if we should poll
        if !should_poll(&state) {
            continue;
        }

        // Get workspace path
        let workspace = match get_workspace(&state) {
            Some(p) => p,
            None => continue,
        };

        // Poll calendar
        match poll_calendar(&workspace) {
            Ok(events) => {
                if let Ok(mut guard) = state.calendar_events.lock() {
                    *guard = events;
                }
                let _ = app_handle.emit("calendar-updated", ());
            }
            Err(PollError::AuthExpired) => {
                log::warn!("Calendar poll: token expired");
                if let Ok(mut guard) = state.google_auth.lock() {
                    *guard = GoogleAuthStatus::TokenExpired;
                }
                let _ = app_handle.emit(
                    "google-auth-changed",
                    GoogleAuthStatus::TokenExpired,
                );
            }
            Err(PollError::ScriptNotFound) => {
                log::debug!("Calendar poll: calendar_poll.py not found, skipping");
            }
            Err(PollError::ScriptError(e)) => {
                log::warn!("Calendar poll error: {}", e);
            }
            Err(PollError::ParseError(e)) => {
                log::warn!("Calendar poll parse error: {}", e);
            }
        }
    }
}

/// Check if we should poll now (authenticated + within work hours)
fn should_poll(state: &AppState) -> bool {
    // Check auth status
    let is_authenticated = state
        .google_auth
        .lock()
        .map(|guard| matches!(*guard, GoogleAuthStatus::Authenticated { .. }))
        .unwrap_or(false);

    if !is_authenticated {
        return false;
    }

    // Check work hours
    let config = state.config.lock().ok().and_then(|g| g.clone());
    let (start_hour, end_hour) = match config {
        Some(cfg) => (cfg.google.work_hours_start, cfg.google.work_hours_end),
        None => (8, 18),
    };

    let now_hour = chrono::Local::now().hour() as u8;
    now_hour >= start_hour && now_hour < end_hour
}

/// Get the poll interval in minutes from config
fn get_poll_interval(state: &AppState) -> u64 {
    state
        .config
        .lock()
        .ok()
        .and_then(|g| g.clone())
        .map(|cfg| cfg.google.calendar_poll_interval_minutes as u64)
        .unwrap_or(5)
}

/// Find a script, checking workspace _tools/ then repo scripts/
fn find_script(workspace: &Path, name: &str) -> Option<PathBuf> {
    let workspace_script = workspace.join("_tools").join(name);
    if workspace_script.exists() {
        return Some(workspace_script);
    }
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or(std::path::Path::new("."));
    let repo_script = repo_root.join("scripts").join(name);
    if repo_script.exists() {
        return Some(repo_script);
    }
    None
}

/// Get workspace path from config
fn get_workspace(state: &AppState) -> Option<PathBuf> {
    state
        .config
        .lock()
        .ok()
        .and_then(|g| g.clone())
        .map(|cfg| std::path::PathBuf::from(cfg.workspace_path))
}
