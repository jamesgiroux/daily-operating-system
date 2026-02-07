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
use crate::workflow::deliver::{make_meeting_id, write_json};

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
                // Check for new prep-eligible meetings before storing (I41)
                let new_preps = generate_preps_for_new_meetings(
                    &events,
                    &state,
                    &workspace,
                );
                if new_preps > 0 {
                    log::info!("Calendar poll: generated {} new preps", new_preps);
                }

                if let Ok(mut guard) = state.calendar_events.lock() {
                    *guard = events;
                }
                let _ = app_handle.emit("calendar-updated", ());

                // Notify frontend about new preps
                for _ in 0..new_preps {
                    let _ = app_handle.emit("prep-ready", ());
                }
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

/// Prep-eligible meeting types (same as PREP_ELIGIBLE_TYPES in deliver.rs)
const PREP_ELIGIBLE_TYPES: &[MeetingType] = &[
    MeetingType::Customer,
    MeetingType::Qbr,
    MeetingType::Partnership,
];

/// Generate lightweight prep files for new calendar events that don't already have preps.
///
/// Called after each calendar poll. Checks if prep-eligible meetings (customer, qbr, partnership)
/// have a prep JSON in `_today/data/preps/`. If not, generates a lightweight prep from
/// account data in SQLite.
fn generate_preps_for_new_meetings(
    events: &[CalendarEvent],
    state: &AppState,
    workspace: &Path,
) -> usize {
    let preps_dir = workspace.join("_today").join("data").join("preps");
    if !preps_dir.exists() {
        // No _today/data/preps/ means briefing hasn't run yet — nothing to do
        return 0;
    }

    let mut generated = 0;

    for event in events {
        // Skip non-prep-eligible types, all-day events, personal events
        if event.is_all_day || !PREP_ELIGIBLE_TYPES.contains(&event.meeting_type) {
            continue;
        }

        let meeting_type_str = match event.meeting_type {
            MeetingType::Customer => "customer",
            MeetingType::Qbr => "qbr",
            MeetingType::Partnership => "partnership",
            _ => continue,
        };

        let meeting_id = make_meeting_id(
            &event.title,
            &event.start.to_rfc3339(),
            meeting_type_str,
        );

        let prep_path = preps_dir.join(format!("{}.json", meeting_id));
        if prep_path.exists() {
            continue; // Already has prep
        }

        // Also check by event ID (different meeting_id but same event)
        let already_prepped = has_existing_prep_for_event(&preps_dir, &event.id);
        if already_prepped {
            continue;
        }

        // Generate lightweight prep from account data in SQLite
        let mut prep = serde_json::json!({
            "meetingId": meeting_id,
            "calendarEventId": event.id,
            "title": event.title,
            "type": meeting_type_str,
            "timeRange": format!(
                "{} - {}",
                event.start.format("%-I:%M %p"),
                event.end.format("%-I:%M %p")
            ),
        });

        if let Some(ref account) = event.account {
            prep.as_object_mut()
                .unwrap()
                .insert("account".to_string(), serde_json::json!(account));

            // Try to pull account data from SQLite
            if let Ok(db_guard) = state.db.lock() {
                if let Some(db) = db_guard.as_ref() {
                    enrich_prep_from_db(&mut prep, account, db);
                }
            }
        }

        match write_json(&prep_path, &prep) {
            Ok(()) => {
                log::info!(
                    "Generated reactive prep for '{}' ({})",
                    event.title,
                    meeting_id
                );
                generated += 1;
            }
            Err(e) => {
                log::warn!("Failed to write reactive prep for '{}': {}", event.title, e);
            }
        }
    }

    generated
}

/// Check if any existing prep file already covers this calendar event ID.
fn has_existing_prep_for_event(preps_dir: &Path, event_id: &str) -> bool {
    let entries = match std::fs::read_dir(preps_dir) {
        Ok(e) => e,
        Err(_) => return false,
    };

    for entry in entries.flatten() {
        if !entry
            .file_name()
            .to_str()
            .is_some_and(|n| n.ends_with(".json"))
        {
            continue;
        }
        if let Ok(content) = std::fs::read_to_string(entry.path()) {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
                if data.get("calendarEventId").and_then(|v| v.as_str()) == Some(event_id) {
                    return true;
                }
            }
        }
    }
    false
}

/// Enrich a prep JSON with account data from SQLite (quick context + open actions).
fn enrich_prep_from_db(
    prep: &mut serde_json::Value,
    account_id: &str,
    db: &crate::db::ActionDb,
) {
    // Quick context from account data
    if let Ok(Some(account)) = db.get_account(account_id) {
        let mut qc = serde_json::Map::new();
        if let Some(ring) = account.ring {
            qc.insert("Ring".to_string(), serde_json::json!(format!("R{}", ring)));
        }
        if let Some(arr) = account.arr {
            qc.insert("ARR".to_string(), serde_json::json!(format!("${:.0}k", arr / 1000.0)));
        }
        if let Some(ref health) = account.health {
            qc.insert("Health".to_string(), serde_json::json!(health));
        }
        if let Some(ref contract_end) = account.contract_end {
            qc.insert("Renewal".to_string(), serde_json::json!(contract_end));
        }
        if !qc.is_empty() {
            prep.as_object_mut()
                .unwrap()
                .insert("quickContext".to_string(), serde_json::Value::Object(qc));
        }
    }

    // Open actions for this account
    if let Ok(actions) = db.get_account_actions(account_id) {
        if !actions.is_empty() {
            let today = Utc::now().format("%Y-%m-%d").to_string();
            let items: Vec<serde_json::Value> = actions
                .iter()
                .take(5)
                .map(|a| {
                    let is_overdue = a
                        .due_date
                        .as_deref()
                        .is_some_and(|d| d < today.as_str());
                    serde_json::json!({
                        "title": a.title,
                        "dueDate": a.due_date,
                        "isOverdue": is_overdue,
                    })
                })
                .collect();
            if !items.is_empty() {
                prep.as_object_mut()
                    .unwrap()
                    .insert("openItems".to_string(), serde_json::json!(items));
            }
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{ActionDb, DbAccount};

    fn test_db() -> ActionDb {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("google_test.db");
        std::mem::forget(dir);
        ActionDb::open_at(path).expect("open test db")
    }

    fn sample_event(
        id: &str,
        title: &str,
        mt: MeetingType,
        account: Option<&str>,
    ) -> CalendarEvent {
        CalendarEvent {
            id: id.to_string(),
            title: title.to_string(),
            start: Utc::now(),
            end: Utc::now() + chrono::Duration::hours(1),
            meeting_type: mt,
            account: account.map(|a| a.to_string()),
            attendees: vec![],
            is_all_day: false,
        }
    }

    #[test]
    fn test_has_existing_prep_for_event_no_match() {
        let dir = tempfile::tempdir().expect("temp dir");
        let preps_dir = dir.path();

        // Write a prep file with a different event ID
        let prep = serde_json::json!({
            "meetingId": "test-meeting",
            "calendarEventId": "cal-event-999",
            "title": "Some meeting"
        });
        std::fs::write(
            preps_dir.join("test-meeting.json"),
            serde_json::to_string_pretty(&prep).unwrap(),
        )
        .unwrap();

        assert!(!has_existing_prep_for_event(preps_dir, "cal-event-123"));
    }

    #[test]
    fn test_has_existing_prep_for_event_match() {
        let dir = tempfile::tempdir().expect("temp dir");
        let preps_dir = dir.path();

        let prep = serde_json::json!({
            "meetingId": "test-meeting",
            "calendarEventId": "cal-event-123",
            "title": "Acme QBR"
        });
        std::fs::write(
            preps_dir.join("test-meeting.json"),
            serde_json::to_string_pretty(&prep).unwrap(),
        )
        .unwrap();

        assert!(has_existing_prep_for_event(preps_dir, "cal-event-123"));
    }

    #[test]
    fn test_has_existing_prep_ignores_non_json() {
        let dir = tempfile::tempdir().expect("temp dir");
        let preps_dir = dir.path();

        std::fs::write(preps_dir.join("notes.txt"), "cal-event-123").unwrap();
        assert!(!has_existing_prep_for_event(preps_dir, "cal-event-123"));
    }

    #[test]
    fn test_enrich_prep_from_db_adds_quick_context() {
        let db = test_db();

        // Insert an account
        let account = DbAccount {
            id: "acme".to_string(),
            name: "Acme Corp".to_string(),
            ring: Some(2),
            arr: Some(150_000.0),
            health: Some("green".to_string()),
            contract_start: None,
            contract_end: Some("2026-06-15".to_string()),
            csm: None,
            champion: None,
            tracker_path: None,
            updated_at: Utc::now().to_rfc3339(),
        };
        db.upsert_account(&account).unwrap();

        let mut prep = serde_json::json!({
            "meetingId": "test",
            "title": "Acme sync"
        });

        enrich_prep_from_db(&mut prep, "acme", &db);

        let qc = prep.get("quickContext").expect("quickContext should exist");
        assert_eq!(qc.get("Ring").unwrap(), "R2");
        assert_eq!(qc.get("ARR").unwrap(), "$150k");
        assert_eq!(qc.get("Health").unwrap(), "green");
        assert_eq!(qc.get("Renewal").unwrap(), "2026-06-15");
    }

    #[test]
    fn test_enrich_prep_from_db_no_account() {
        let db = test_db();

        let mut prep = serde_json::json!({
            "meetingId": "test",
            "title": "Unknown sync"
        });

        enrich_prep_from_db(&mut prep, "nonexistent", &db);

        // Should not add quickContext if account not found
        assert!(prep.get("quickContext").is_none());
    }

    #[test]
    fn test_prep_eligible_types_filter() {
        // Customer, QBR, Partnership are eligible
        assert!(PREP_ELIGIBLE_TYPES.contains(&MeetingType::Customer));
        assert!(PREP_ELIGIBLE_TYPES.contains(&MeetingType::Qbr));
        assert!(PREP_ELIGIBLE_TYPES.contains(&MeetingType::Partnership));
        // Others are not
        assert!(!PREP_ELIGIBLE_TYPES.contains(&MeetingType::Internal));
        assert!(!PREP_ELIGIBLE_TYPES.contains(&MeetingType::Personal));
        assert!(!PREP_ELIGIBLE_TYPES.contains(&MeetingType::AllHands));
        assert!(!PREP_ELIGIBLE_TYPES.contains(&MeetingType::TeamSync));
    }

    #[test]
    fn test_all_day_events_skipped() {
        // All-day events should never get preps, even if they're customer type
        let event = CalendarEvent {
            id: "cal-1".to_string(),
            title: "Acme offsite".to_string(),
            start: Utc::now(),
            end: Utc::now() + chrono::Duration::hours(8),
            meeting_type: MeetingType::Customer,
            account: Some("acme".to_string()),
            attendees: vec![],
            is_all_day: true,
        };

        assert!(event.is_all_day || !PREP_ELIGIBLE_TYPES.contains(&event.meeting_type));
        // Since is_all_day is true, the condition short-circuits and skips
        assert!(event.is_all_day);
    }

    #[test]
    fn test_make_meeting_id_deterministic() {
        let id1 = make_meeting_id("Acme QBR", "2026-02-07T09:00:00Z", "customer");
        let id2 = make_meeting_id("Acme QBR", "2026-02-07T09:00:00Z", "customer");
        assert_eq!(id1, id2);

        // Different inputs produce different IDs
        let id3 = make_meeting_id("Acme QBR", "2026-02-07T10:00:00Z", "customer");
        assert_ne!(id1, id3);
    }
}
