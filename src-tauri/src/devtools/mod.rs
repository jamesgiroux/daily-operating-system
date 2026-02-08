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
const TODAY_DIRECTIVE_TMPL: &str = include_str!("fixtures/today-directive.json.tmpl");
const WEEK_DIRECTIVE_TMPL: &str = include_str!("fixtures/week-directive.json.tmpl");

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
        "simulate_briefing" => {
            install_simulate_briefing(state)?;
            Ok("Simulate briefing: workspace + directives seeded. Run dev_run_delivery to execute Phase 2+3.".into())
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
        .read()
        .map(|g| g.is_some())
        .unwrap_or(false);

    let workspace_path = state
        .config
        .read()
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
        .read()
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
    if let Ok(mut guard) = state.config.write() {
        *guard = None;
    }
    if let Ok(mut guard) = state.db.lock() {
        // Reopen a fresh DB
        *guard = ActionDb::open().ok();
    }
    if let Ok(mut guard) = state.google_auth.lock() {
        *guard = GoogleAuthStatus::NotConfigured;
    }
    if let Ok(mut guard) = state.workflow_status.write() {
        guard.clear();
    }
    if let Ok(mut guard) = state.execution_history.lock() {
        guard.clear();
    }
    if let Ok(mut guard) = state.last_scheduled_run.write() {
        guard.clear();
    }
    if let Ok(mut guard) = state.calendar_events.write() {
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

/// Install full mock data + workspace markdown + directive JSONs for pipeline testing.
///
/// After this, `dev_run_delivery` will execute Phase 2+3 (mechanical delivery)
/// from the pre-written directive — no Google API or Python needed.
fn install_simulate_briefing(state: &AppState) -> Result<(), String> {
    // Start with full mock data (includes DB + calendar events + fixtures)
    install_mock_data(state, true)?;

    let workspace = dev_workspace();

    // Write workspace markdown files so prepare/ has content to parse
    write_workspace_markdown(&workspace)?;

    // Write directive JSONs so delivery can run without Phase 1
    write_directive_fixtures(&workspace)?;

    Ok(())
}

/// Ensure the simulate_briefing scenario has been applied.
/// If the directive JSON is missing, seed everything automatically.
fn ensure_briefing_seeded(state: &AppState) -> Result<(), String> {
    let workspace = get_workspace(state)?;
    let directive_path = workspace.join("_today").join("data").join("today-directive.json");
    if !directive_path.exists() {
        log::info!("Directive not found — auto-seeding simulate_briefing scenario");
        install_simulate_briefing(state)?;
    }
    Ok(())
}

/// Daily briefing — mechanical only.
///
/// Loads today-directive.json → delivers schedule, actions, preps, emails, manifest.
/// No AI enrichment. Tests the full Rust delivery pipeline.
pub fn run_today_mechanical(state: &AppState) -> Result<String, String> {
    if !cfg!(debug_assertions) {
        return Err("Dev tools not available in release builds".into());
    }

    ensure_briefing_seeded(state)?;

    let workspace = get_workspace(state)?;
    let today_dir = workspace.join("_today");
    let data_dir = today_dir.join("data");

    let directive = crate::json_loader::load_directive(&today_dir)
        .map_err(|e| format!("Failed to load directive: {}", e))?;

    let schedule_data = crate::workflow::deliver::deliver_schedule(&directive, &data_dir)?;

    let db_guard = state.db.lock().map_err(|_| "DB lock poisoned")?;
    let db_ref = db_guard.as_ref();
    let actions_data = crate::workflow::deliver::deliver_actions(&directive, &data_dir, db_ref)?;
    if let Some(db) = db_ref {
        let _ = crate::workflow::today::sync_actions_to_db(&workspace, db);
    }
    drop(db_guard);

    let prep_paths = crate::workflow::deliver::deliver_preps(&directive, &data_dir)?;

    let emails_data = crate::workflow::deliver::deliver_emails(&directive, &data_dir)
        .unwrap_or_else(|_| serde_json::json!({}));

    crate::workflow::deliver::deliver_manifest(
        &directive, &schedule_data, &actions_data, &emails_data, &prep_paths, &data_dir, false,
    )?;

    Ok(format!(
        "Today (mechanical): schedule, actions, {} preps, emails, manifest",
        prep_paths.len()
    ))
}

/// Daily briefing — full pipeline including AI enrichment.
///
/// Same as mechanical + enrich_emails, enrich_preps, enrich_briefing via Claude Code CLI.
/// Requires Claude Code installed and authenticated.
pub fn run_today_full(state: &AppState) -> Result<String, String> {
    if !cfg!(debug_assertions) {
        return Err("Dev tools not available in release builds".into());
    }

    ensure_briefing_seeded(state)?;

    let workspace = get_workspace(state)?;
    let today_dir = workspace.join("_today");
    let data_dir = today_dir.join("data");

    let directive = crate::json_loader::load_directive(&today_dir)
        .map_err(|e| format!("Failed to load directive: {}", e))?;

    // --- Mechanical delivery ---
    let schedule_data = crate::workflow::deliver::deliver_schedule(&directive, &data_dir)?;

    let db_guard = state.db.lock().map_err(|_| "DB lock poisoned")?;
    let db_ref = db_guard.as_ref();
    let actions_data = crate::workflow::deliver::deliver_actions(&directive, &data_dir, db_ref)?;
    if let Some(db) = db_ref {
        let _ = crate::workflow::today::sync_actions_to_db(&workspace, db);
    }
    drop(db_guard);

    let prep_paths = crate::workflow::deliver::deliver_preps(&directive, &data_dir)?;

    let emails_data = crate::workflow::deliver::deliver_emails(&directive, &data_dir)
        .unwrap_or_else(|_| serde_json::json!({}));

    // Partial manifest (AI enrichment pending)
    crate::workflow::deliver::deliver_manifest(
        &directive, &schedule_data, &actions_data, &emails_data, &prep_paths, &data_dir, true,
    )?;

    // --- AI enrichment ---
    let pty = crate::pty::PtyManager::new();
    let user_ctx = state.config.read().ok()
        .and_then(|g| g.as_ref().map(crate::types::UserContext::from_config))
        .unwrap_or_else(|| crate::types::UserContext { name: None, company: None, title: None, focus: None });

    let mut enriched = Vec::new();

    match crate::workflow::deliver::enrich_emails(&data_dir, &pty, &workspace, &user_ctx) {
        Ok(()) => enriched.push("emails"),
        Err(e) => log::warn!("Email enrichment failed (non-fatal): {}", e),
    }

    match crate::workflow::deliver::enrich_preps(&data_dir, &pty, &workspace) {
        Ok(()) => enriched.push("preps"),
        Err(e) => log::warn!("Prep enrichment failed (non-fatal): {}", e),
    }

    match crate::workflow::deliver::enrich_briefing(&data_dir, &pty, &workspace, &user_ctx) {
        Ok(()) => enriched.push("briefing"),
        Err(e) => log::warn!("Briefing enrichment failed (non-fatal): {}", e),
    }

    // Final manifest
    crate::workflow::deliver::deliver_manifest(
        &directive, &schedule_data, &actions_data, &emails_data, &prep_paths, &data_dir, false,
    )?;

    Ok(format!(
        "Today (full): schedule, actions, {} preps, emails, manifest. AI enriched: [{}]",
        prep_paths.len(),
        enriched.join(", ")
    ))
}

/// Weekly prep — mechanical only.
///
/// Loads week-directive.json → delivers week-overview.json.
pub fn run_week_mechanical(state: &AppState) -> Result<String, String> {
    if !cfg!(debug_assertions) {
        return Err("Dev tools not available in release builds".into());
    }

    ensure_briefing_seeded(state)?;

    let workspace = get_workspace(state)?;
    crate::prepare::orchestrate::deliver_week(&workspace)?;
    Ok("Week (mechanical): week-overview.json delivered".into())
}

/// Weekly prep — full pipeline including AI enrichment.
///
/// Runs Claude Code with /week skill (reads week-directive.json from workspace),
/// then delivers week-overview.json.
pub fn run_week_full(state: &AppState) -> Result<String, String> {
    if !cfg!(debug_assertions) {
        return Err("Dev tools not available in release builds".into());
    }

    ensure_briefing_seeded(state)?;

    let workspace = get_workspace(state)?;

    // Phase 2: AI enrichment via Claude Code /week
    let pty = crate::pty::PtyManager::new();
    let output = pty.spawn_claude(&workspace, "/week")
        .map_err(|e| format!("Claude /week failed: {}", e))?;
    log::info!("Week AI enrichment: {} bytes output", output.stdout.len());

    // Phase 3: Mechanical delivery
    crate::prepare::orchestrate::deliver_week(&workspace)?;

    Ok("Week (full): Claude /week + week-overview.json delivered".into())
}

/// Helper: get workspace path from config.
fn get_workspace(state: &AppState) -> Result<std::path::PathBuf, String> {
    state
        .config
        .read()
        .ok()
        .and_then(|g| g.as_ref().map(|c| std::path::PathBuf::from(&c.workspace_path)))
        .ok_or_else(|| "No workspace configured".to_string())
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
        "INSERT OR REPLACE INTO accounts (id, name, lifecycle, arr, health, tracker_path, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params!["acme-corp", "Acme Corp", "steady-state", 1_200_000.0, "green", "Accounts/Acme Corp/dashboard.md", &today],
    ).map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT OR REPLACE INTO accounts (id, name, lifecycle, arr, health, tracker_path, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params!["globex-industries", "Globex Industries", "at-risk", 800_000.0, "yellow", "Accounts/Globex Industries/dashboard.md", &today],
    ).map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT OR REPLACE INTO accounts (id, name, lifecycle, arr, health, tracker_path, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params!["initech", "Initech", "onboarding", 350_000.0, "green", "Accounts/Initech/dashboard.md", &today],
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

    if let Ok(mut guard) = state.calendar_events.write() {
        *guard = events;
    }

    Ok(())
}

/// Write workspace markdown files that the prepare/ module can parse.
///
/// Creates actions.md + account dashboard files for Acme, Globex, Initech.
fn write_workspace_markdown(workspace: &Path) -> Result<(), String> {
    let today = Local::now();
    let yesterday = (today - chrono::Duration::days(1)).format("%Y-%m-%d");
    let last_week = (today - chrono::Duration::days(7)).format("%Y-%m-%d");
    let tomorrow = (today + chrono::Duration::days(1)).format("%Y-%m-%d");
    let friday = {
        let weekday = today.weekday().num_days_from_monday() as i64;
        (today + chrono::Duration::days(4 - weekday)).format("%Y-%m-%d")
    };

    // --- actions.md (checkbox format that actions.rs parses) ---
    let actions_content = format!(
        r#"# Actions

## Overdue
- [ ] Send updated SOW to Acme legal team due:{yesterday} P1 @acme-corp #legal
- [ ] Follow up on NPS survey responses due:{last_week} P2 @acme-corp #customer-health

## Due Today
- [ ] Review Globex QBR deck with AE due:{today_date} P1 @globex-industries #qbr-prep

## This Week
- [ ] Schedule Phase 2 kickoff with Initech due:{tomorrow} P2 @initech #project
- [ ] Create knowledge transfer plan for Alex Torres departure due:{friday} P1 @acme-corp #risk-mitigation

## Waiting On
- [ ] Phase 2 budget approval — waiting on Initech finance team @initech #budget
"#,
        yesterday = yesterday,
        last_week = last_week,
        today_date = today.format("%Y-%m-%d"),
        tomorrow = tomorrow,
        friday = friday,
    );

    std::fs::write(workspace.join("actions.md"), actions_content)
        .map_err(|e| format!("Failed to write actions.md: {}", e))?;

    // --- Account dashboards ---
    let accounts_dir = workspace.join("Accounts");

    // Acme Corp
    let acme_dir = accounts_dir.join("Acme Corp");
    std::fs::create_dir_all(&acme_dir)
        .map_err(|e| format!("Failed to create Acme dir: {}", e))?;

    let acme_dashboard = r#"# Acme Corp

## Quick View
| Field | Value |
|-------|-------|
| ARR | $1,200,000 |
| Health | Green |
| Lifecycle | Steady-state |
| Renewal Date | 2025-09-15 |
| CSM | You |

## Key Stakeholders
- Sarah Chen (VP Engineering) — Executive Sponsor
- Alex Torres (Tech Lead) — Day-to-day contact, departing March
- Pat Kim (CTO) — Strategic alignment

## Recent Wins
- Phase 1 migration completed ahead of schedule
- Performance benchmarks exceeded targets by 15%
- Executive sponsorship confirmed for Phase 2

## Active Risks
- Alex Torres leaving in March — knowledge transfer gap
- NPS trending down — 3 detractors in last survey
- Phase 2 budget approval still pending from finance

## Notes
Phase 2 scoping underway with April kickoff target. Need KT plan before Alex departs.
"#;
    std::fs::write(acme_dir.join("dashboard.md"), acme_dashboard)
        .map_err(|e| format!("Failed to write Acme dashboard: {}", e))?;

    let acme_json = serde_json::json!({
        "name": "Acme Corp",
        "lifecycle": "steady-state",
        "arr": 1200000,
        "health": "green",
        "renewal_date": "2025-09-15",
        "csm": "You",
        "key_stakeholders": ["Sarah Chen (VP Eng)", "Alex Torres (Tech Lead)", "Pat Kim (CTO)"],
        "notes": "Phase 2 scoping underway. KT plan needed before Alex departs."
    });
    std::fs::write(
        acme_dir.join("dashboard.json"),
        serde_json::to_string_pretty(&acme_json).unwrap(),
    )
    .map_err(|e| format!("Failed to write Acme dashboard.json: {}", e))?;

    // Globex Industries
    let globex_dir = accounts_dir.join("Globex Industries");
    std::fs::create_dir_all(&globex_dir)
        .map_err(|e| format!("Failed to create Globex dir: {}", e))?;

    let globex_dashboard = r#"# Globex Industries

## Quick View
| Field | Value |
|-------|-------|
| ARR | $800,000 |
| Health | Yellow |
| Lifecycle | At-risk |
| Renewal Date | 2025-06-30 |
| CSM | You |

## Key Stakeholders
- Pat Reynolds (VP Product) — Executive Sponsor, departing Q2
- Jamie Morrison (Eng Director) — Technical champion
- Casey Lee (Head of Ops) — Usage & adoption

## Recent Wins
- Expanded to 3 new teams this quarter
- Team A usage up 40% since January
- CSAT score improved from 7.2 to 8.1

## Active Risks
- Pat Reynolds (key stakeholder) departing Q2
- Usage declining in Team B — down 20% MoM
- Renewal in 90 days with health at Yellow
- Competitor (Contoso) actively pitching their team

## Strategic Programs
- **Team B Recovery** [At Risk]: Engagement plan to reverse usage decline
- **APAC Expansion** [Proposed]: Extend deployment to Singapore and Sydney offices

## Notes
QBR is the highest-stakes meeting. Renewal decision expected this quarter.
"#;
    std::fs::write(globex_dir.join("dashboard.md"), globex_dashboard)
        .map_err(|e| format!("Failed to write Globex dashboard: {}", e))?;

    let globex_json = serde_json::json!({
        "name": "Globex Industries",
        "lifecycle": "at-risk",
        "arr": 800000,
        "health": "yellow",
        "renewal_date": "2025-06-30",
        "csm": "You",
        "key_stakeholders": ["Pat Reynolds (VP Product)", "Jamie Morrison (Eng Director)", "Casey Lee (Head of Ops)"],
        "notes": "Renewal at risk. QBR critical. Team B recovery plan underway."
    });
    std::fs::write(
        globex_dir.join("dashboard.json"),
        serde_json::to_string_pretty(&globex_json).unwrap(),
    )
    .map_err(|e| format!("Failed to write Globex dashboard.json: {}", e))?;

    // Initech
    let initech_dir = accounts_dir.join("Initech");
    std::fs::create_dir_all(&initech_dir)
        .map_err(|e| format!("Failed to create Initech dir: {}", e))?;

    let initech_dashboard = r#"# Initech

## Quick View
| Field | Value |
|-------|-------|
| ARR | $350,000 |
| Health | Green |
| Lifecycle | Onboarding |
| CSM | You |

## Recent Wins
- Phase 1 delivered on time and under budget

## Active Risks
- Budget approval pending from finance
- Team bandwidth concerns for Q2

## Notes
Phase 1 complete. Phase 2 kickoff meeting to align on scope and confirm executive sponsor.
"#;
    std::fs::write(initech_dir.join("dashboard.md"), initech_dashboard)
        .map_err(|e| format!("Failed to write Initech dashboard: {}", e))?;

    let initech_json = serde_json::json!({
        "name": "Initech",
        "lifecycle": "onboarding",
        "arr": 350000,
        "health": "green",
        "csm": "You",
        "notes": "Phase 2 kickoff planned. Budget approval pending."
    });
    std::fs::write(
        initech_dir.join("dashboard.json"),
        serde_json::to_string_pretty(&initech_json).unwrap(),
    )
    .map_err(|e| format!("Failed to write Initech dashboard.json: {}", e))?;

    Ok(())
}

/// Write directive JSON fixtures for pipeline testing (bypass Phase 1).
fn write_directive_fixtures(workspace: &Path) -> Result<(), String> {
    let data_dir = workspace.join("_today").join("data");
    std::fs::create_dir_all(&data_dir)
        .map_err(|e| format!("Failed to create data dir: {}", e))?;

    // Today directive
    let today_content = patch_dates(TODAY_DIRECTIVE_TMPL);
    std::fs::write(data_dir.join("today-directive.json"), today_content)
        .map_err(|e| format!("Failed to write today-directive.json: {}", e))?;

    // Week directive
    let week_content = patch_dates(WEEK_DIRECTIVE_TMPL);
    std::fs::write(data_dir.join("week-directive.json"), week_content)
        .map_err(|e| format!("Failed to write week-directive.json: {}", e))?;

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
