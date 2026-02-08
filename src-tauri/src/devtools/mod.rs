//! Dev tools for scenario switching and mock data.
//!
//! All public functions check `cfg!(debug_assertions)` at runtime so that
//! `generate_handler!` can resolve them in release builds (where they return
//! errors immediately). The cost is two string comparisons — negligible.

use std::path::Path;

use chrono::{Datelike, Local, TimeZone, Utc};
use serde::Serialize;

use crate::db::ActionDb;
use crate::state::AppState;
use crate::types::{CalendarEvent, GoogleAuthStatus, MeetingType, TranscriptRecord};

// Fixture templates embedded at compile time
const MANIFEST_TMPL: &str = include_str!("fixtures/manifest.json.tmpl");
const SCHEDULE_TMPL: &str = include_str!("fixtures/schedule.json.tmpl");
const ACTIONS_TMPL: &str = include_str!("fixtures/actions.json.tmpl");
const EMAILS_TMPL: &str = include_str!("fixtures/emails.json.tmpl");
const PREP_ACME_TMPL: &str = include_str!("fixtures/prep-acme.json.tmpl");
const PREP_GLOBEX_TMPL: &str = include_str!("fixtures/prep-globex.json.tmpl");
const PREP_INITECH_TMPL: &str = include_str!("fixtures/prep-initech.json.tmpl");
const WEEK_OVERVIEW_TMPL: &str = include_str!("fixtures/week-overview.json.tmpl");

/// Dev workspace path — never touches the real workspace.
fn dev_workspace() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join("Documents")
        .join("DailyOS-dev")
}

/// State snapshot for the dev tools panel.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DevState {
    pub is_debug_build: bool,
    pub has_config: bool,
    pub workspace_path: Option<String>,
    pub has_database: bool,
    pub action_count: usize,
    pub account_count: usize,
    pub meeting_count: usize,
    pub has_today_data: bool,
    pub google_auth_status: String,
}

/// Apply a named scenario. Entry point for the `dev_apply_scenario` command.
pub fn apply_scenario(scenario: &str, state: &AppState) -> Result<String, String> {
    if !cfg!(debug_assertions) {
        return Err("Dev tools not available in release builds".into());
    }

    match scenario {
        "reset" => {
            reset_all(state)?;
            Ok("Reset complete — app is in first-run state".into())
        }
        "mock_full" => {
            install_mock_data(state, true)?;
            Ok("Mock data installed with Google auth".into())
        }
        "mock_no_auth" => {
            install_mock_data(state, false)?;
            Ok("Mock data installed without Google auth".into())
        }
        "mock_empty" => {
            install_mock_empty(state)?;
            Ok("Empty workspace installed with config".into())
        }
        _ => Err(format!("Unknown scenario: {}", scenario)),
    }
}

/// Query current dev state for the panel UI.
pub fn get_dev_state(state: &AppState) -> Result<DevState, String> {
    if !cfg!(debug_assertions) {
        return Err("Dev tools not available in release builds".into());
    }

    let has_config = state
        .config
        .lock()
        .map(|g| g.is_some())
        .unwrap_or(false);

    let workspace_path = state
        .config
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|c| c.workspace_path.clone()));

    let has_today_data = workspace_path
        .as_ref()
        .map(|wp| Path::new(wp).join("_today").join("data").join("manifest.json").exists())
        .unwrap_or(false);

    let (has_database, action_count, account_count, meeting_count) =
        match state.db.lock() {
            Ok(guard) => match guard.as_ref() {
                Some(db) => {
                    let actions = db
                        .conn_ref()
                        .query_row("SELECT COUNT(*) FROM actions", [], |r| r.get::<_, usize>(0))
                        .unwrap_or(0);
                    let accounts = db
                        .conn_ref()
                        .query_row("SELECT COUNT(*) FROM accounts", [], |r| r.get::<_, usize>(0))
                        .unwrap_or(0);
                    let meetings = db
                        .conn_ref()
                        .query_row("SELECT COUNT(*) FROM meetings_history", [], |r| {
                            r.get::<_, usize>(0)
                        })
                        .unwrap_or(0);
                    (true, actions, accounts, meetings)
                }
                None => (false, 0, 0, 0),
            },
            Err(_) => (false, 0, 0, 0),
        };

    let google_auth_status = state
        .google_auth
        .lock()
        .map(|g| match &*g {
            GoogleAuthStatus::NotConfigured => "not_configured".to_string(),
            GoogleAuthStatus::Authenticated { email } => format!("authenticated ({})", email),
            GoogleAuthStatus::TokenExpired => "token_expired".to_string(),
        })
        .unwrap_or_else(|_| "unknown".to_string());

    Ok(DevState {
        is_debug_build: cfg!(debug_assertions),
        has_config,
        workspace_path,
        has_database,
        action_count,
        account_count,
        meeting_count,
        has_today_data,
        google_auth_status,
    })
}

// =============================================================================
// Scenario implementations
// =============================================================================

/// Reset everything to first-run state.
fn reset_all(state: &AppState) -> Result<(), String> {
    let home = dirs::home_dir().ok_or("Could not find home directory")?;
    let dailyos_dir = home.join(".dailyos");

    // 1. Read workspace path before deleting config
    let workspace_path = state
        .config
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|c| c.workspace_path.clone()));

    // 2. Delete config and state files
    let files_to_delete = [
        dailyos_dir.join("config.json"),
        dailyos_dir.join("actions.db"),
        dailyos_dir.join("actions.db-wal"),
        dailyos_dir.join("actions.db-shm"),
        dailyos_dir.join("execution_history.json"),
        dailyos_dir.join("transcript_records.json"),
        dailyos_dir.join("google").join("token.json"),
    ];

    for path in &files_to_delete {
        if path.exists() {
            let _ = std::fs::remove_file(path);
        }
    }

    // 3. Clear workspace _today/data/ contents (not the dir itself)
    if let Some(wp) = &workspace_path {
        let data_dir = Path::new(wp).join("_today").join("data");
        if data_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&data_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let _ = std::fs::remove_dir_all(&path);
                    } else {
                        let _ = std::fs::remove_file(&path);
                    }
                }
            }
        }
    }

    // 4. Reset all AppState mutexes in-place
    if let Ok(mut guard) = state.config.lock() {
        *guard = None;
    }
    if let Ok(mut guard) = state.db.lock() {
        // Reopen a fresh DB
        *guard = ActionDb::open().ok();
    }
    if let Ok(mut guard) = state.google_auth.lock() {
        *guard = GoogleAuthStatus::NotConfigured;
    }
    if let Ok(mut guard) = state.workflow_status.lock() {
        guard.clear();
    }
    if let Ok(mut guard) = state.execution_history.lock() {
        guard.clear();
    }
    if let Ok(mut guard) = state.last_scheduled_run.lock() {
        guard.clear();
    }
    if let Ok(mut guard) = state.calendar_events.lock() {
        guard.clear();
    }
    if let Ok(mut guard) = state.capture_dismissed.lock() {
        guard.clear();
    }
    if let Ok(mut guard) = state.capture_captured.lock() {
        guard.clear();
    }
    if let Ok(mut guard) = state.week_planning_state.lock() {
        *guard = crate::types::WeekPlanningState::default();
    }
    if let Ok(mut guard) = state.transcript_processed.lock() {
        guard.clear();
    }

    Ok(())
}

/// Install full mock data with optional Google auth.
fn install_mock_data(state: &AppState, with_auth: bool) -> Result<(), String> {
    // Start from clean slate
    reset_all(state)?;

    let workspace = dev_workspace();

    // Create config
    crate::state::create_or_update_config(state, |config| {
        config.workspace_path = workspace.to_string_lossy().to_string();
        config.entity_mode = "account".to_string();
        config.profile = "customer-success".to_string();
    })?;

    // Scaffold workspace
    crate::state::initialize_workspace(&workspace, "account")?;

    // Write date-patched JSON fixtures
    write_fixtures(&workspace)?;

    // Seed SQLite
    let db_guard = state.db.lock().map_err(|_| "DB lock poisoned")?;
    if let Some(db) = db_guard.as_ref() {
        seed_database(db)?;
    }

    // Seed transcript record for today's past Acme meeting (#1)
    let today_str = Local::now().format("%Y-%m-%d").to_string();
    let acme_meeting_id = format!("mtg-acme-weekly-{}", today_str);
    if let Ok(mut guard) = state.transcript_processed.lock() {
        guard.insert(
            acme_meeting_id.clone(),
            TranscriptRecord {
                meeting_id: acme_meeting_id,
                file_path: "transcript-acme-weekly.md".to_string(),
                destination: format!("_archive/{}/transcripts/acme-weekly.md", today_str),
                summary: Some("Discussed Phase 1 completion benchmarks, NPS trends, Phase 2 timeline, and APAC expansion strategy.".to_string()),
                processed_at: chrono::Utc::now().to_rfc3339(),
            },
        );
    }

    // Seed calendar events for merge overlay statuses.
    // 7 events: all meetings EXCEPT #3 (Initech kickoff) → it becomes "cancelled".
    // Plus Initech Onboarding (not in briefing) → it becomes "new".
    seed_calendar_events(state)?;

    // Google auth
    if with_auth {
        write_mock_google_token()?;
        if let Ok(mut guard) = state.google_auth.lock() {
            *guard = GoogleAuthStatus::Authenticated {
                email: "dev@dailyos.test".to_string(),
            };
        }
    }

    Ok(())
}

/// Install config + workspace but no briefing data.
fn install_mock_empty(state: &AppState) -> Result<(), String> {
    reset_all(state)?;

    let workspace = dev_workspace();

    crate::state::create_or_update_config(state, |config| {
        config.workspace_path = workspace.to_string_lossy().to_string();
        config.entity_mode = "account".to_string();
        config.profile = "customer-success".to_string();
    })?;

    crate::state::initialize_workspace(&workspace, "account")?;

    // Write mock Google token so we pass the auth check
    write_mock_google_token()?;
    if let Ok(mut guard) = state.google_auth.lock() {
        *guard = GoogleAuthStatus::Authenticated {
            email: "dev@dailyos.test".to_string(),
        };
    }

    Ok(())
}

// =============================================================================
// Helpers
// =============================================================================

/// Replace date tokens in a template string.
pub(crate) fn patch_dates(template: &str) -> String {
    let now = Local::now();
    let date = now.format("%Y-%m-%d").to_string();
    let generated_at = now.to_rfc3339();
    let yesterday = (now - chrono::Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();
    let last_week = (now - chrono::Duration::days(7))
        .format("%Y-%m-%d")
        .to_string();
    let tomorrow = (now + chrono::Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();
    let next_week = (now + chrono::Duration::days(7))
        .format("%Y-%m-%d")
        .to_string();

    // Weekday tokens: {{MON}} through {{FRI}} resolve to this week's dates.
    // chrono Weekday: Mon=0 .. Sun=6. We compute offset from today's weekday.
    let today_weekday = now.weekday().num_days_from_monday() as i64; // 0=Mon
    let fmt = |offset_from_mon: i64| -> String {
        let delta = offset_from_mon - today_weekday;
        (now + chrono::Duration::days(delta))
            .format("%Y-%m-%d")
            .to_string()
    };
    let mon = fmt(0);
    let tue = fmt(1);
    let wed = fmt(2);
    let thu = fmt(3);
    let fri = fmt(4);

    template
        .replace("{{GENERATED_AT}}", &generated_at)
        .replace("{{YESTERDAY}}", &yesterday)
        .replace("{{LAST_WEEK}}", &last_week)
        .replace("{{TOMORROW}}", &tomorrow)
        .replace("{{NEXT_WEEK}}", &next_week)
        .replace("{{MON}}", &mon)
        .replace("{{TUE}}", &tue)
        .replace("{{WED}}", &wed)
        .replace("{{THU}}", &thu)
        .replace("{{FRI}}", &fri)
        .replace("{{DATE}}", &date)
}

/// Write all fixture JSON files to `_today/data/`.
pub(crate) fn write_fixtures(workspace: &Path) -> Result<(), String> {
    let data_dir = workspace.join("_today").join("data");
    let preps_dir = data_dir.join("preps");

    std::fs::create_dir_all(&preps_dir)
        .map_err(|e| format!("Failed to create preps dir: {}", e))?;

    let fixtures: Vec<(&str, &str)> = vec![
        ("manifest.json", MANIFEST_TMPL),
        ("schedule.json", SCHEDULE_TMPL),
        ("actions.json", ACTIONS_TMPL),
        ("emails.json", EMAILS_TMPL),
        ("week-overview.json", WEEK_OVERVIEW_TMPL),
    ];

    for (filename, template) in fixtures {
        let content = patch_dates(template);
        std::fs::write(data_dir.join(filename), content)
            .map_err(|e| format!("Failed to write {}: {}", filename, e))?;
    }

    // Prep files go into preps/ subdirectory
    let prep_fixtures: Vec<(&str, &str)> = vec![
        ("acme-corp-quarterly-sync.json", PREP_ACME_TMPL),
        ("globex-industries-qbr.json", PREP_GLOBEX_TMPL),
        ("initech-phase2-kickoff.json", PREP_INITECH_TMPL),
    ];

    for (filename, template) in prep_fixtures {
        let content = patch_dates(template);
        std::fs::write(preps_dir.join(filename), content)
            .map_err(|e| format!("Failed to write prep {}: {}", filename, e))?;
    }

    Ok(())
}

/// Seed SQLite with realistic mock data.
pub(crate) fn seed_database(db: &ActionDb) -> Result<(), String> {
    let now = chrono::Utc::now();
    let today = now.to_rfc3339();

    // Helper to format relative dates
    let days_ago = |n: i64| -> String {
        (now - chrono::Duration::days(n)).to_rfc3339()
    };
    let date_only = |n: i64| -> String {
        (chrono::Local::now() + chrono::Duration::days(n))
            .format("%Y-%m-%d")
            .to_string()
    };

    let conn = db.conn_ref();

    // --- Accounts ---
    conn.execute(
        "INSERT OR REPLACE INTO accounts (id, name, ring, arr, health, tracker_path, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params!["acme-corp", "Acme Corp", 1, 1_200_000.0, "green", "Accounts/Acme Corp/dashboard.md", &today],
    ).map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT OR REPLACE INTO accounts (id, name, ring, arr, health, tracker_path, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params!["globex-industries", "Globex Industries", 2, 800_000.0, "yellow", "Accounts/Globex Industries/dashboard.md", &today],
    ).map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT OR REPLACE INTO accounts (id, name, ring, arr, health, tracker_path, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params!["initech", "Initech", 3, 350_000.0, "green", "Accounts/Initech/dashboard.md", &today],
    ).map_err(|e| e.to_string())?;

    // --- Entities (mirrors accounts) ---
    conn.execute(
        "INSERT OR REPLACE INTO entities (id, name, entity_type, tracker_path, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params!["acme-corp", "Acme Corp", "account", "Accounts/Acme Corp/dashboard.md", &today],
    ).map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT OR REPLACE INTO entities (id, name, entity_type, tracker_path, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params!["globex-industries", "Globex Industries", "account", "Accounts/Globex Industries/dashboard.md", &today],
    ).map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT OR REPLACE INTO entities (id, name, entity_type, tracker_path, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params!["initech", "Initech", "account", "Accounts/Initech/dashboard.md", &today],
    ).map_err(|e| e.to_string())?;

    // --- Actions (matching actions.json IDs) ---
    let action_rows: Vec<(&str, &str, &str, &str, Option<&str>, Option<String>)> = vec![
        ("act-sow-acme", "Send updated SOW to Acme legal team", "P1", "pending", Some("acme-corp"), Some(date_only(-1))),
        ("act-qbr-deck-globex", "Review Globex QBR deck with AE", "P1", "pending", Some("globex-industries"), Some(date_only(0))),
        ("act-kickoff-initech", "Schedule Phase 2 kickoff with Initech", "P2", "pending", Some("initech"), Some(date_only(1))),
        ("act-nps-acme", "Follow up on NPS survey responses", "P2", "pending", Some("acme-corp"), Some(date_only(-7))),
        ("act-quarterly-summary", "Draft quarterly impact summary", "P3", "pending", None, Some(date_only(7))),
    ];

    for (id, title, priority, status, account_id, due_date) in &action_rows {
        conn.execute(
            "INSERT OR REPLACE INTO actions (id, title, priority, status, created_at, due_date, account_id, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![id, title, priority, status, &today, due_date, account_id, &today],
        ).map_err(|e| e.to_string())?;
    }

    // --- Meetings history ---
    let meeting_rows: Vec<(&str, &str, &str, String, Option<&str>)> = vec![
        ("mh-acme-7d", "Acme Corp Weekly Sync", "customer", days_ago(7), Some("acme-corp")),
        ("mh-acme-21d", "Acme Corp Monthly Review", "customer", days_ago(21), Some("acme-corp")),
        ("mh-globex-3d", "Globex Check-in", "customer", days_ago(3), Some("globex-industries")),
        ("mh-globex-14d", "Globex Sprint Demo", "customer", days_ago(14), Some("globex-industries")),
        ("mh-initech-10d", "Initech Phase 1 Wrap", "customer", days_ago(10), Some("initech")),
        ("mh-standup-1d", "Engineering Standup", "team_sync", days_ago(1), None),
    ];

    for (id, title, mtype, start_time, account_id) in &meeting_rows {
        conn.execute(
            "INSERT OR REPLACE INTO meetings_history (id, title, meeting_type, start_time, account_id, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![id, title, mtype, start_time, account_id, &today],
        ).map_err(|e| e.to_string())?;
    }

    // --- Captures ---
    let capture_rows: Vec<(&str, &str, &str, Option<&str>, &str, &str)> = vec![
        // Historical captures (past meetings)
        ("cap-acme-win-1", "mh-acme-7d", "Acme Corp Weekly Sync", Some("acme-corp"), "win", "Completed Phase 1 migration ahead of schedule"),
        ("cap-acme-risk-1", "mh-acme-7d", "Acme Corp Weekly Sync", Some("acme-corp"), "risk", "NPS trending down — 3 detractors identified"),
        ("cap-globex-win-1", "mh-globex-3d", "Globex Check-in", Some("globex-industries"), "win", "Expanded to 3 new teams this quarter"),
        ("cap-globex-risk-1", "mh-globex-3d", "Globex Check-in", Some("globex-industries"), "risk", "Key stakeholder (Pat Reynolds) departing Q2"),
    ];

    for (id, meeting_id, meeting_title, account_id, ctype, content) in &capture_rows {
        conn.execute(
            "INSERT OR REPLACE INTO captures (id, meeting_id, meeting_title, account_id, capture_type, content, captured_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![id, meeting_id, meeting_title, account_id, ctype, content, &today],
        ).map_err(|e| e.to_string())?;
    }

    // --- Transcript-sourced captures for today's Acme meeting (meeting #1) ---
    let today_acme_id = format!("mtg-acme-weekly-{}", date_only(0));
    let transcript_captures: Vec<(&str, &str, &str)> = vec![
        ("cap-today-acme-win-1", "win", "Phase 1 performance benchmarks exceeded targets by 15%"),
        ("cap-today-acme-win-2", "win", "Sarah confirmed executive sponsorship for Phase 2"),
        ("cap-today-acme-risk-1", "risk", "Alex Torres leaving in March — need knowledge transfer plan by next week"),
        ("cap-today-acme-decision-1", "decision", "Phase 2 kickoff moved to April to allow proper scoping"),
        ("cap-today-acme-decision-2", "decision", "Will pursue APAC expansion as separate workstream in Q3"),
    ];

    for (id, ctype, content) in &transcript_captures {
        conn.execute(
            "INSERT OR REPLACE INTO captures (id, meeting_id, meeting_title, account_id, capture_type, content, captured_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![id, &today_acme_id, "Acme Corp Weekly Sync", "acme-corp", ctype, content, &today],
        ).map_err(|e| e.to_string())?;
    }

    // --- Transcript-sourced actions for today's Acme meeting ---
    let transcript_actions: Vec<(&str, &str, &str, &str)> = vec![
        ("act-transcript-kt-plan", "Create knowledge transfer plan for Alex Torres departure", "P1", "acme-corp"),
        ("act-transcript-phase2-scope", "Draft Phase 2 scope document for April kickoff", "P2", "acme-corp"),
    ];

    for (id, title, priority, account_id) in &transcript_actions {
        conn.execute(
            "INSERT OR REPLACE INTO actions (id, title, priority, status, created_at, due_date, account_id, source_type, source_id, updated_at) VALUES (?1, ?2, ?3, 'pending', ?4, ?5, ?6, 'transcript', ?7, ?8)",
            rusqlite::params![id, title, priority, &today, date_only(3), account_id, &today_acme_id, &today],
        ).map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Seed calendar events to produce overlay statuses via merge.
///
/// All briefing meetings get a matching calendar event EXCEPT Initech Kickoff
/// (missing → "cancelled"). An extra Initech Onboarding event has no briefing
/// match (→ "new").
fn seed_calendar_events(state: &AppState) -> Result<(), String> {
    let today_str = Local::now().format("%Y-%m-%d").to_string();

    // Helper: build a CalendarEvent from hour/minute (UTC — merge will convert to tz).
    // For mock purposes we use today's date at the given UTC hour.
    let today = Utc::now().date_naive();
    let make_event =
        |id: &str, title: &str, start_h: u32, start_m: u32, end_h: u32, end_m: u32, mtype: MeetingType, account: Option<&str>| -> CalendarEvent {
            CalendarEvent {
                id: id.to_string(),
                title: title.to_string(),
                start: Utc.from_utc_datetime(
                    &today.and_hms_opt(start_h, start_m, 0).unwrap(),
                ),
                end: Utc.from_utc_datetime(
                    &today.and_hms_opt(end_h, end_m, 0).unwrap(),
                ),
                meeting_type: mtype,
                account: account.map(|s| s.to_string()),
                attendees: vec![],
                is_all_day: false,
            }
        };

    // Use the same calendar event IDs as schedule.json.tmpl (after {{DATE}} patching).
    let events = vec![
        // #1: Acme Weekly (past, 8:00 AM)
        make_event(
            &format!("cal-acme-weekly-{}", today_str),
            "Acme Corp Weekly Sync",
            13, 0, 13, 45, // 8:00-8:45 AM ET = 13:00-13:45 UTC
            MeetingType::Customer,
            Some("Acme Corp"),
        ),
        // #2: Eng Standup (past, 9:30 AM)
        make_event(
            &format!("cal-eng-standup-{}", today_str),
            "Engineering Standup",
            14, 30, 14, 45, // 9:30-9:45 AM ET
            MeetingType::TeamSync,
            None,
        ),
        // #3: Initech Kickoff OMITTED — will become "cancelled"
        // #4: 1:1 with Sarah (11:00 AM)
        make_event(
            &format!("cal-1on1-sarah-{}", today_str),
            "1:1 with Sarah (Manager)",
            16, 0, 16, 30, // 11:00-11:30 AM ET
            MeetingType::OneOnOne,
            None,
        ),
        // #5: Globex QBR (1:00 PM)
        make_event(
            &format!("cal-globex-qbr-{}", today_str),
            "Globex Industries QBR",
            18, 0, 19, 0, // 1:00-2:00 PM ET
            MeetingType::Qbr,
            Some("Globex Industries"),
        ),
        // #6: Sprint Review (2:30 PM)
        make_event(
            &format!("cal-sprint-review-{}", today_str),
            "Product Team Sprint Review",
            19, 30, 20, 15, // 2:30-3:15 PM ET
            MeetingType::Internal,
            None,
        ),
        // #7: Initech Onboarding — NOT in briefing → "new"
        make_event(
            &format!("cal-initech-onboarding-{}", today_str),
            "Initech Onboarding Call",
            20, 30, 21, 30, // 3:30-4:30 PM ET
            MeetingType::Training,
            Some("Initech"),
        ),
        // #8: All Hands (4:30 PM)
        make_event(
            &format!("cal-all-hands-{}", today_str),
            "Company All Hands",
            21, 30, 22, 30, // 4:30-5:30 PM ET
            MeetingType::AllHands,
            None,
        ),
    ];

    if let Ok(mut guard) = state.calendar_events.lock() {
        *guard = events;
    }

    Ok(())
}

/// Write a mock Google token file.
fn write_mock_google_token() -> Result<(), String> {
    let token_path = crate::state::google_token_path();
    if let Some(parent) = token_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create google dir: {}", e))?;
    }

    let token = serde_json::json!({
        "token": "mock-dev-token",
        "refresh_token": "mock-refresh",
        "email": "dev@dailyos.test"
    });

    std::fs::write(&token_path, token.to_string())
        .map_err(|e| format!("Failed to write mock token: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_patch_dates_replaces_all_tokens() {
        let template = "date={{DATE}} gen={{GENERATED_AT}} y={{YESTERDAY}} lw={{LAST_WEEK}} t={{TOMORROW}} nw={{NEXT_WEEK}} mon={{MON}} tue={{TUE}} wed={{WED}} thu={{THU}} fri={{FRI}}";
        let result = patch_dates(template);
        assert!(!result.contains("{{"));
        assert!(!result.contains("}}"));
    }

    #[test]
    fn test_patch_dates_preserves_non_tokens() {
        let template = "Hello world, no tokens here";
        assert_eq!(patch_dates(template), "Hello world, no tokens here");
    }

    #[test]
    fn test_dev_workspace_path() {
        let path = dev_workspace();
        assert!(path.to_string_lossy().contains("DailyOS-dev"));
    }
}
