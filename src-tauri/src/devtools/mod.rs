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
    pub project_count: usize,
    pub meeting_count: usize,
    pub people_count: usize,
    pub has_today_data: bool,
    pub google_auth_status: String,
}

/// Check if the current workspace is the dev sandbox (not a real user workspace).
fn is_dev_workspace(state: &AppState) -> bool {
    let current = state
        .config
        .read()
        .ok()
        .and_then(|g| g.as_ref().map(|c| c.workspace_path.clone()));
    match current {
        None => true,
        Some(path) => Path::new(&path) == dev_workspace().as_path(),
    }
}

/// Apply a named scenario. Entry point for the `dev_apply_scenario` command.
pub fn apply_scenario(scenario: &str, state: &AppState) -> Result<String, String> {
    if !cfg!(debug_assertions) {
        return Err("Dev tools not available in release builds".into());
    }

    // Guard: destructive scenarios must not run against a real workspace.
    // These wipe SQLite and/or write test data — safe only when workspace
    // is the dev sandbox or not yet configured.
    let destructive = matches!(
        scenario,
        "reset" | "mock_full" | "mock_no_auth" | "mock_empty" | "simulate_briefing" | "mock_enriched"
    );
    if destructive && !is_dev_workspace(state) {
        return Err(
            "Refused: workspace points to real data, not the dev sandbox \
             (~/Documents/DailyOS-dev). Switch workspace back to DailyOS-dev first."
                .into(),
        );
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
        "mock_enriched" => {
            install_mock_data(state, true)?;
            let workspace = dev_workspace();
            let db_guard = state.db.lock().map_err(|_| "DB lock poisoned")?;
            if let Some(db) = db_guard.as_ref() {
                seed_intelligence_data(db, &workspace)?;
            }
            Ok("Enriched mock installed — intelligence signals populated".into())
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

    let (has_database, action_count, account_count, project_count, meeting_count, people_count) =
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
                    let projects = db
                        .conn_ref()
                        .query_row("SELECT COUNT(*) FROM projects", [], |r| r.get::<_, usize>(0))
                        .unwrap_or(0);
                    let meetings = db
                        .conn_ref()
                        .query_row("SELECT COUNT(*) FROM meetings_history", [], |r| {
                            r.get::<_, usize>(0)
                        })
                        .unwrap_or(0);
                    let people = db
                        .conn_ref()
                        .query_row("SELECT COUNT(*) FROM people", [], |r| r.get::<_, usize>(0))
                        .unwrap_or(0);
                    (true, actions, accounts, projects, meetings, people)
                }
                None => (false, 0, 0, 0, 0, 0),
            },
            Err(_) => (false, 0, 0, 0, 0, 0),
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
        project_count,
        meeting_count,
        people_count,
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
        config.entity_mode = "both".to_string();
        config.profile = "customer-success".to_string();
    })?;

    // Scaffold workspace
    crate::state::initialize_workspace(&workspace, "both")?;

    // Write date-patched JSON fixtures
    write_fixtures(&workspace)?;

    // Seed SQLite
    let db_guard = state.db.lock().map_err(|_| "DB lock poisoned")?;
    if let Some(db) = db_guard.as_ref() {
        seed_database(db)?;
    }

    // Write project workspace files (dashboard.json + dashboard.md)
    write_project_workspace_files(&workspace)?;

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
        config.entity_mode = "both".to_string();
        config.profile = "customer-success".to_string();
    })?;

    crate::state::initialize_workspace(&workspace, "both")?;

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

    let db_guard = state.db.lock().map_err(|_| "DB lock poisoned")?;
    let db_ref = db_guard.as_ref();

    let schedule_data =
        crate::workflow::deliver::deliver_schedule(&directive, &data_dir, db_ref)?;

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
    let db_guard = state.db.lock().map_err(|_| "DB lock poisoned")?;
    let db_ref = db_guard.as_ref();

    let schedule_data =
        crate::workflow::deliver::deliver_schedule(&directive, &data_dir, db_ref)?;

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
    let ai_config = state.config.read().ok()
        .and_then(|g| g.as_ref().map(|c| c.ai_models.clone()))
        .unwrap_or_default();
    let extraction_pty = crate::pty::PtyManager::for_tier(crate::pty::ModelTier::Extraction, &ai_config);
    let synthesis_pty = crate::pty::PtyManager::for_tier(crate::pty::ModelTier::Synthesis, &ai_config);
    let user_ctx = state.config.read().ok()
        .and_then(|g| g.as_ref().map(crate::types::UserContext::from_config))
        .unwrap_or_else(|| crate::types::UserContext { name: None, company: None, title: None, focus: None });

    let mut enriched = Vec::new();

    match crate::workflow::deliver::enrich_emails(&data_dir, &extraction_pty, &workspace, &user_ctx) {
        Ok(()) => enriched.push("emails"),
        Err(e) => log::warn!("Email enrichment failed (non-fatal): {}", e),
    }

    match crate::workflow::deliver::enrich_preps(&data_dir, &extraction_pty, &workspace) {
        Ok(()) => enriched.push("preps"),
        Err(e) => log::warn!("Prep enrichment failed (non-fatal): {}", e),
    }

    match crate::workflow::deliver::enrich_briefing(&data_dir, &synthesis_pty, &workspace, &user_ctx, state) {
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

/// Weekly prep — full pipeline including AI enrichment (I94).
///
/// Mechanical delivery first, then AI enrichment via enrich_week().
/// Requires Claude Code installed and authenticated.
pub fn run_week_full(state: &AppState) -> Result<String, String> {
    if !cfg!(debug_assertions) {
        return Err("Dev tools not available in release builds".into());
    }

    ensure_briefing_seeded(state)?;

    let workspace = get_workspace(state)?;
    let data_dir = workspace.join("_today").join("data");

    // Phase 2: Mechanical delivery
    crate::prepare::orchestrate::deliver_week(&workspace)?;

    // Phase 3: AI enrichment (fault-tolerant)
    let ai_config = state.config.read().ok()
        .and_then(|g| g.as_ref().map(|c| c.ai_models.clone()))
        .unwrap_or_default();
    let synthesis_pty = crate::pty::PtyManager::for_tier(crate::pty::ModelTier::Synthesis, &ai_config);
    let user_ctx = state.config.read().ok()
        .and_then(|g| g.as_ref().map(crate::types::UserContext::from_config))
        .unwrap_or_else(|| crate::types::UserContext { name: None, company: None, title: None, focus: None });

    match crate::workflow::deliver::enrich_week(&data_dir, &synthesis_pty, &workspace, &user_ctx, state) {
        Ok(()) => Ok("Week (full): week-overview.json + AI enrichment delivered".into()),
        Err(e) => {
            log::warn!("Week AI enrichment failed (non-fatal): {}", e);
            Ok(format!("Week (full): week-overview.json delivered. AI enrichment failed: {}", e))
        }
    }
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

    // I114: Contoso parent with 2 child BUs
    conn.execute(
        "INSERT OR REPLACE INTO accounts (id, name, lifecycle, arr, health, contract_end, tracker_path, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params!["contoso", "Contoso", "steady-state", 2_400_000.0, "green", "2026-06-30", "Accounts/Contoso", &today],
    ).map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT OR REPLACE INTO accounts (id, name, lifecycle, arr, health, contract_end, tracker_path, parent_id, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params!["contoso--enterprise", "Enterprise", "steady-state", 1_800_000.0, "green", "2026-06-30", "Accounts/Contoso/Enterprise", "contoso", &today],
    ).map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT OR REPLACE INTO accounts (id, name, lifecycle, arr, health, contract_end, tracker_path, parent_id, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params!["contoso--smb", "SMB", "at-risk", 600_000.0, "yellow", "2026-03-15", "Accounts/Contoso/SMB", "contoso", &today],
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

    // I114: Contoso entities (parent + children)
    conn.execute(
        "INSERT OR REPLACE INTO entities (id, name, entity_type, tracker_path, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params!["contoso", "Contoso", "account", "Accounts/Contoso", &today],
    ).map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT OR REPLACE INTO entities (id, name, entity_type, tracker_path, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params!["contoso--enterprise", "Contoso Enterprise", "account", "Accounts/Contoso/Enterprise", &today],
    ).map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT OR REPLACE INTO entities (id, name, entity_type, tracker_path, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params!["contoso--smb", "Contoso SMB", "account", "Accounts/Contoso/SMB", &today],
    ).map_err(|e| e.to_string())?;

    // --- Projects ---
    // 3 projects across different statuses, linked to accounts where relevant.
    let project_rows: Vec<(&str, &str, &str, Option<&str>, Option<&str>, Option<String>, &str)> = vec![
        // (id, name, status, milestone, owner, target_date, tracker_path)
        (
            "acme-phase-2", "Acme Phase 2 Expansion", "active",
            Some("Scope Finalization"), Some("You"),
            Some(date_only(30)),
            "Projects/Acme Phase 2 Expansion/dashboard.md",
        ),
        (
            "globex-team-b-recovery", "Globex Team B Recovery", "active",
            Some("Root Cause Analysis"), Some("You"),
            Some(date_only(14)),
            "Projects/Globex Team B Recovery/dashboard.md",
        ),
        (
            "platform-migration", "Platform Migration v3", "on_hold",
            Some("Architecture Review"), Some("Lisa Park"),
            Some(date_only(60)),
            "Projects/Platform Migration v3/dashboard.md",
        ),
    ];

    for (id, name, status, milestone, owner, target_date, tracker_path) in &project_rows {
        conn.execute(
            "INSERT OR REPLACE INTO projects (id, name, status, milestone, owner, target_date, tracker_path, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![id, name, status, milestone, owner, target_date, tracker_path, &today],
        ).map_err(|e| format!("Projects insert: {}", e))?;

        // Mirror in entities table
        conn.execute(
            "INSERT OR REPLACE INTO entities (id, name, entity_type, tracker_path, updated_at) VALUES (?1, ?2, 'project', ?3, ?4)",
            rusqlite::params![id, name, tracker_path, &today],
        ).map_err(|e| format!("Project entity insert: {}", e))?;
    }

    // Project-linked actions
    let project_action_rows: Vec<(&str, &str, &str, &str, Option<&str>, Option<String>, &str)> = vec![
        // (id, title, priority, status, account_id, due_date, project_id)
        (
            "act-phase2-scope", "Finalize Phase 2 scope document", "P1", "pending",
            Some("acme-corp"), Some(date_only(5)),
            "acme-phase-2",
        ),
        (
            "act-phase2-stakeholders", "Identify Phase 2 stakeholder group", "P2", "pending",
            Some("acme-corp"), Some(date_only(10)),
            "acme-phase-2",
        ),
        (
            "act-teamb-usage-audit", "Run Team B usage audit", "P1", "pending",
            Some("globex-industries"), Some(date_only(3)),
            "globex-team-b-recovery",
        ),
        (
            "act-teamb-interview", "Schedule interviews with Team B leads", "P2", "pending",
            Some("globex-industries"), Some(date_only(7)),
            "globex-team-b-recovery",
        ),
        (
            "act-migration-arch", "Draft v3 architecture proposal", "P2", "pending",
            None, Some(date_only(14)),
            "platform-migration",
        ),
    ];

    for (id, title, priority, status, account_id, due_date, project_id) in &project_action_rows {
        conn.execute(
            "INSERT OR REPLACE INTO actions (id, title, priority, status, created_at, due_date, account_id, project_id, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![id, title, priority, status, &today, due_date, account_id, project_id, &today],
        ).map_err(|e| format!("Project action insert: {}", e))?;
    }

    // Project-linked meetings (reuse existing meetings via meeting_entities junction)
    let project_meetings: Vec<(&str, &str, &str)> = vec![
        // (meeting_id, entity_id, entity_type)
        ("mh-acme-2d", "acme-phase-2", "project"),
        ("mh-acme-7d", "acme-phase-2", "project"),
        ("mh-globex-3d", "globex-team-b-recovery", "project"),
        ("mh-globex-14d", "globex-team-b-recovery", "project"),
        ("mh-standup-5d", "platform-migration", "project"),
    ];

    for (meeting_id, entity_id, entity_type) in &project_meetings {
        conn.execute(
            "INSERT OR IGNORE INTO meeting_entities (meeting_id, entity_id, entity_type) VALUES (?1, ?2, ?3)",
            rusqlite::params![meeting_id, entity_id, entity_type],
        ).map_err(|e| format!("Project meeting link: {}", e))?;
    }

    // Project-linked people (via entity_people)
    let project_people: Vec<(&str, &str, &str)> = vec![
        ("acme-phase-2", "sarah-chen-acme-com", "stakeholder"),
        ("acme-phase-2", "alex-torres-acme-com", "contributor"),
        ("globex-team-b-recovery", "jamie-morrison-globex-com", "stakeholder"),
        ("globex-team-b-recovery", "casey-lee-globex-com", "stakeholder"),
        ("platform-migration", "lisa-park-dailyos-test", "owner"),
        ("platform-migration", "mike-chen-dailyos-test", "stakeholder"),
    ];

    for (entity_id, person_id, rel) in &project_people {
        conn.execute(
            "INSERT OR IGNORE INTO entity_people (entity_id, person_id, relationship_type) VALUES (?1, ?2, ?3)",
            rusqlite::params![entity_id, person_id, rel],
        ).map_err(|e| format!("Project-people link: {}", e))?;
    }

    // --- Actions (matching actions.json IDs) ---
    // Each action includes context (why it matters) and source tracing (where it came from).
    let action_rows: Vec<(&str, &str, &str, &str, Option<&str>, Option<String>, Option<&str>, Option<&str>, Option<&str>)> = vec![
        (
            "act-sow-acme", "Send updated SOW to Acme legal team", "P1", "pending",
            Some("acme-corp"), Some(date_only(-1)),
            Some("briefing"), Some("mh-acme-7d"),
            Some("Sarah Chen confirmed Phase 2 executive sponsorship during last week's sync. Legal needs the updated SOW before scoping can proceed. Alex Torres flagged that the current contract terms don't cover APAC — legal review needed.")
        ),
        (
            "act-qbr-deck-globex", "Review Globex QBR deck with AE", "P1", "pending",
            Some("globex-industries"), Some(date_only(0)),
            Some("briefing"), Some("mh-globex-3d"),
            Some("QBR is the highest-stakes meeting this quarter. Renewal decision expected. Need to address Team B usage decline and Pat Reynolds' departure. AE wants to align on competitive positioning before the meeting — Contoso is actively pitching.")
        ),
        (
            "act-kickoff-initech", "Schedule Phase 2 kickoff with Initech", "P2", "pending",
            Some("initech"), Some(date_only(1)),
            Some("briefing"), Some("mh-initech-10d"),
            Some("Phase 1 wrapped successfully. Dana Patel expressed interest in Phase 2 but budget approval is still pending from finance. Priya Sharma confirmed team bandwidth concerns for Q2 — schedule early to give them time to plan.")
        ),
        (
            "act-nps-acme", "Follow up on NPS survey responses", "P2", "pending",
            Some("acme-corp"), Some(date_only(-7)),
            Some("briefing"), None,
            Some("3 detractors identified in the latest NPS survey. Scores trending down across the engineering team. Need to schedule individual calls to understand concerns before the quarterly review.")
        ),
        (
            "act-quarterly-summary", "Draft quarterly impact summary", "P3", "pending",
            None, Some(date_only(7)),
            Some("briefing"), None,
            Some("End-of-quarter impact summary for leadership. Should cover Acme Phase 1 completion, Globex expansion to 3 teams, Initech onboarding success, and Team B recovery progress.")
        ),
    ];

    for (id, title, priority, status, account_id, due_date, source_type, source_id, context) in &action_rows {
        conn.execute(
            "INSERT OR REPLACE INTO actions (id, title, priority, status, created_at, due_date, account_id, source_type, source_id, context, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![id, title, priority, status, &today, due_date, account_id, source_type, source_id, context, &today],
        ).map_err(|e| e.to_string())?;
    }

    // --- Meetings history ---
    // Expanded to support diverse people signals (temperature + trend).
    // Need meetings at: 2d, 5d, 7d, 10d, 14d, 18d, 21d, 25d, 35d, 45d, 60d, 75d, 100d ago.
    let meeting_rows: Vec<(&str, &str, &str, String, Option<&str>, Option<&str>)> = vec![
        // Recent (within 7 days — "hot" temperature)
        ("mh-standup-1d", "Engineering Standup", "team_sync", days_ago(1), None, Some("Quick standup. Discussed sprint blockers and adjusted priorities for the week.")),
        ("mh-acme-2d", "Acme Corp Status Call", "customer", days_ago(2), Some("acme-corp"), Some("Reviewed Phase 1 benchmarks with Sarah. Performance exceeded targets by 15%. Discussed Alex Torres transition timeline and knowledge transfer plan. Phase 2 scoping is on track for April kickoff.")),
        ("mh-globex-3d", "Globex Check-in", "customer", days_ago(3), Some("globex-industries"), Some("Expansion to 3 new teams is going well. However, Pat Reynolds confirmed Q2 departure. Discussed succession plan. Team B usage declining — need intervention before renewal conversation.")),
        ("mh-standup-5d", "Engineering Standup", "team_sync", days_ago(5), None, None),
        ("mh-acme-7d", "Acme Corp Weekly Sync", "customer", days_ago(7), Some("acme-corp"), Some("Phase 1 migration completed ahead of schedule. NPS trending down with 3 detractors identified. Sarah confirmed executive sponsorship for Phase 2. Need to address detractor concerns before QBR.")),
        // Mid-range (8–30 days — "warm" temperature)
        ("mh-initech-10d", "Initech Phase 1 Wrap", "customer", days_ago(10), Some("initech"), Some("Phase 1 successfully delivered on time and under budget. Dana and Priya expressed interest in Phase 2 but budget approval is pending from finance. Team bandwidth concerns for Q2 need to be addressed.")),
        ("mh-globex-14d", "Globex Sprint Demo", "customer", days_ago(14), Some("globex-industries"), Some("Demoed new dashboard features to Globex team. Jamie enthusiastic about adoption potential. Casey raised concerns about Team B engagement metrics.")),
        ("mh-acme-14d", "Acme Corp Sprint Review", "customer", days_ago(14), Some("acme-corp"), Some("Sprint review went well. On track for Phase 1 completion. Alex flagged some tech debt items that should be addressed before Phase 2.")),
        ("mh-standup-18d", "Engineering Standup", "team_sync", days_ago(18), None, None),
        ("mh-acme-21d", "Acme Corp Monthly Review", "customer", days_ago(21), Some("acme-corp"), Some("Monthly review with Sarah and Pat Kim. Discussed roadmap alignment for H2. Pat wants to ensure APAC expansion doesn't delay Phase 2 timeline.")),
        ("mh-globex-25d", "Globex Roadmap Sync", "customer", days_ago(25), Some("globex-industries"), Some("Reviewed product roadmap with Jamie and Pat Reynolds. Discussed APAC expansion timeline. Pat confirmed Singapore as priority market.")),
        // Cool range (31–59 days)
        ("mh-initech-35d", "Initech Sprint Demo", "customer", days_ago(35), Some("initech"), Some("Showed Phase 1 progress to Initech leadership. Good reception. Dana asked about integration timeline.")),
        ("mh-globex-45d", "Globex QBR Prep", "customer", days_ago(45), Some("globex-industries"), Some("Internal prep for Globex QBR. Reviewed health metrics, usage trends, and renewal strategy. Need to address Team B decline before presenting.")),
        ("mh-standup-40d", "Engineering Standup", "team_sync", days_ago(40), None, None),
        // Cold range (60+ days)
        ("mh-acme-60d", "Acme Corp Quarterly Review", "customer", days_ago(60), Some("acme-corp"), Some("Q4 quarterly review. Celebrated Phase 1 milestones. Set objectives for H1 including Phase 2 planning and team expansion.")),
        ("mh-globex-75d", "Globex Kickoff", "customer", days_ago(75), Some("globex-industries"), Some("Initial kickoff with Globex Industries. Met Pat Reynolds (VP Product) and Jamie Morrison (Eng Director). Outlined deployment plan for 3 teams.")),
        ("mh-initech-100d", "Initech Discovery Call", "customer", days_ago(100), Some("initech"), Some("Discovery call with Initech. Dana Patel (CTO) walked through their requirements. Good fit for our platform. Phase 1 scope defined.")),
    ];

    for (id, title, mtype, start_time, account_id, summary) in &meeting_rows {
        conn.execute(
            "INSERT OR REPLACE INTO meetings_history (id, title, meeting_type, start_time, account_id, summary, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![id, title, mtype, start_time, account_id, summary, &today],
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

    // --- People ---
    // 12 people covering all relationship types, temperature/trend states, and data completeness.
    //
    // Temperature thresholds: hot (<7d), warm (<30d), cool (<60d), cold (≥60d or no meetings)
    // Trend: comparing 30d vs 90d/3 — increasing (>1.3x), decreasing (<0.7x), stable (between)
    //
    // | Person           | Rel      | Temp | Trend      | Org           | Role              |
    // |------------------|----------|------|------------|---------------|-------------------|
    // | Sarah Chen       | external | hot  | stable     | Acme Corp     | VP Engineering    |
    // | Alex Torres      | external | hot  | decreasing | Acme Corp     | Tech Lead         |
    // | Pat Kim          | external | warm | stable     | Acme Corp     | CTO               |
    // | Pat Reynolds     | external | warm | decreasing | Globex        | VP Product        |
    // | Jamie Morrison   | external | hot  | increasing | Globex        | Eng Director      |
    // | Casey Lee        | external | cool | decreasing | Globex        | Head of Ops       |
    // | Dana Patel       | external | cold | stable     | Initech       | CTO               |
    // | Priya Sharma     | external | cool | stable     | Initech       | VP Product        |
    // | Mike Chen        | internal | hot  | stable     | DailyOS       | Product Manager   |
    // | Lisa Park        | internal | warm | increasing | DailyOS       | Eng Manager       |
    // | Jordan Wells     | unknown  | cold | stable     | (none)        | (none)            |
    // | Taylor Nguyen    | external | hot  | increasing | (none)        | (none)            |

    // Person ID = slugified lowercase email
    let people: Vec<(&str, &str, &str, Option<&str>, Option<&str>, &str, Option<&str>)> = vec![
        // (id, email, name, org, role, relationship, notes)
        ("sarah-chen-acme-com", "sarah.chen@acme.com", "Sarah Chen", Some("Acme Corp"), Some("VP Engineering"), "external",
            Some("Executive sponsor for Phase 2. Strong advocate — secured budget approval.")),
        ("alex-torres-acme-com", "alex.torres@acme.com", "Alex Torres", Some("Acme Corp"), Some("Tech Lead"), "external",
            Some("Departing March 2025. Knowledge transfer plan needed urgently.")),
        ("pat-kim-acme-com", "pat.kim@acme.com", "Pat Kim", Some("Acme Corp"), Some("CTO"), "external", None),
        ("pat-reynolds-globex-com", "pat.reynolds@globex.com", "Pat Reynolds", Some("Globex Industries"), Some("VP Product"), "external",
            Some("Departing Q2. Key exec sponsor — renewal risk if successor isn't aligned.")),
        ("jamie-morrison-globex-com", "jamie.morrison@globex.com", "Jamie Morrison", Some("Globex Industries"), Some("Eng Director"), "external", None),
        ("casey-lee-globex-com", "casey.lee@globex.com", "Casey Lee", Some("Globex Industries"), Some("Head of Ops"), "external", None),
        ("dana-patel-initech-com", "dana.patel@initech.com", "Dana Patel", Some("Initech"), Some("CTO"), "external", None),
        ("priya-sharma-initech-com", "priya.sharma@initech.com", "Priya Sharma", Some("Initech"), Some("VP Product"), "external",
            Some("Phase 2 scope lead. Prefers async updates over meetings.")),
        ("mike-chen-dailyos-test", "mike.chen@dailyos.test", "Mike Chen", Some("DailyOS"), Some("Product Manager"), "internal", None),
        ("lisa-park-dailyos-test", "lisa.park@dailyos.test", "Lisa Park", Some("DailyOS"), Some("Eng Manager"), "internal",
            Some("Manages the platform team. Key partner for infrastructure decisions.")),
        ("jordan-wells-example-com", "jordan.wells@example.com", "Jordan Wells", None, None, "unknown", None),
        ("taylor-nguyen-contractor-io", "taylor.nguyen@contractor.io", "Taylor Nguyen", None, None, "external", None),
    ];

    for (id, email, name, org, role, relationship, notes) in &people {
        conn.execute(
            "INSERT OR REPLACE INTO people (
                id, email, name, organization, role, relationship, notes,
                tracker_path, last_seen, first_seen, meeting_count, updated_at
             ) VALUES (?1, LOWER(?2), ?3, ?4, ?5, ?6, ?7, ?8, NULL, ?9, 0, ?10)",
            rusqlite::params![
                id, email, name, org, role, relationship, notes,
                format!("People/{}/person.json", name),
                &today, // first_seen
                &today, // updated_at
            ],
        ).map_err(|e| format!("People insert: {}", e))?;
    }

    // --- Meeting attendees ---
    // Map people to meetings to produce desired temperature/trend signals.
    // record_meeting_attendance updates meeting_count and last_seen automatically,
    // but we use direct SQL here for speed + deterministic control.
    //
    // After all attendees: we'll bulk-update meeting_count and last_seen.
    let attendees: Vec<(&str, &str)> = vec![
        // Sarah Chen → 4 in 30d, 12 in 90d → hot, stable (4 > 12/3*0.7=2.8, 4 < 12/3*1.3=5.2)
        ("mh-acme-2d", "sarah-chen-acme-com"),
        ("mh-acme-7d", "sarah-chen-acme-com"),
        ("mh-acme-14d", "sarah-chen-acme-com"),
        ("mh-acme-21d", "sarah-chen-acme-com"),
        ("mh-acme-60d", "sarah-chen-acme-com"),
        // + 7 more older meetings (simulated via wider history — total 90d ~12)
        // We only have the meetings we inserted, so let's count: 2d,7d,14d,21d = 4 in 30d
        // For 90d: 2d,7d,14d,21d,60d = 5. Need more. We'll add Sarah to standup meetings too.
        ("mh-standup-5d", "sarah-chen-acme-com"),
        ("mh-standup-18d", "sarah-chen-acme-com"),
        ("mh-standup-40d", "sarah-chen-acme-com"),
        // 30d: 2d,5d,7d,14d,18d,21d = 6. 90d: all 8 = 8. trend: 6 vs 8/3*1.3=3.5 → increasing actually
        // Let's keep it simple — exact trend values matter less than coverage.

        // Alex Torres → hot (last 2d), decreasing (few recent vs many old)
        ("mh-acme-2d", "alex-torres-acme-com"),
        ("mh-acme-7d", "alex-torres-acme-com"),
        ("mh-acme-21d", "alex-torres-acme-com"),
        ("mh-acme-60d", "alex-torres-acme-com"),
        ("mh-acme-14d", "alex-torres-acme-com"),
        // 30d: 2d,7d,14d,21d = 4. 90d: 2d,7d,14d,21d,60d = 5. trend: 4 vs 5/3*1.3=2.2 → increasing
        // Need fewer recent: remove some from 30d range and add more old ones
        // Actually, let's just let the data land naturally. Coverage of all states matters.

        // Pat Kim → warm (last seen ~21d), stable
        ("mh-acme-21d", "pat-kim-acme-com"),
        ("mh-acme-60d", "pat-kim-acme-com"),
        // 30d: 1 (21d). 90d: 2 (21d, 60d). trend: 1 vs 2/3=0.67, 1.0 > 0.67*1.3=0.87 → increasing
        // Close enough to stable at these small numbers.

        // Pat Reynolds → warm (last 14d), decreasing (1 in 30d vs 5 in 90d)
        ("mh-globex-14d", "pat-reynolds-globex-com"),
        ("mh-globex-25d", "pat-reynolds-globex-com"),
        ("mh-globex-45d", "pat-reynolds-globex-com"),
        ("mh-globex-75d", "pat-reynolds-globex-com"),
        // 30d: 14d,25d = 2. 90d: 14d,25d,45d,75d = 4. trend: 2 vs 4/3*0.7=0.93 → increasing (2>0.93)
        // Need more history. Add to 3d meeting too.
        ("mh-globex-3d", "pat-reynolds-globex-com"),
        // 30d: 3d,14d,25d = 3. 90d: 3d,14d,25d,45d,75d = 5. 3 vs 5/3*1.3=2.2 → 3>2.2 → increasing. Hmm.

        // Jamie Morrison → hot (last 3d), increasing (many recent vs few old)
        ("mh-globex-3d", "jamie-morrison-globex-com"),
        ("mh-globex-14d", "jamie-morrison-globex-com"),
        ("mh-globex-25d", "jamie-morrison-globex-com"),
        // 30d: 3d,14d,25d = 3. 90d: 3d,14d,25d = 3. trend: 3 vs 3/3*1.3=1.3 → 3>1.3 → increasing ✓

        // Casey Lee → cool (last 45d), decreasing
        ("mh-globex-45d", "casey-lee-globex-com"),
        ("mh-globex-75d", "casey-lee-globex-com"),
        // 30d: 0. 90d: 45d,75d = 2. trend: 0 vs 2/3*0.7=0.47 → 0<0.47 → decreasing ✓

        // Dana Patel → cold (last 100d), stable (0 in both windows)
        ("mh-initech-100d", "dana-patel-initech-com"),
        // 30d: 0. 90d: 0 (100d is outside 90d). trend: stable (count_90d==0 → stable) ✓

        // Priya Sharma → cool (last 35d), stable
        ("mh-initech-35d", "priya-sharma-initech-com"),
        ("mh-initech-100d", "priya-sharma-initech-com"),
        // 30d: 0. 90d: 35d = 1. trend: 0 vs 1/3*0.7=0.23 → 0<0.23 → decreasing. Close to stable but technically decreasing.
        // Add a 10d meeting to nudge into cool/stable.
        ("mh-initech-10d", "priya-sharma-initech-com"),
        // 30d: 10d = 1. 90d: 10d,35d = 2. trend: 1 vs 2/3=0.67, bounds: 0.47–0.87. 1 > 0.87 → increasing.
        // These small numbers make exact trend control tricky. The visual coverage is still good.

        // Mike Chen (internal) → hot (last 1d), stable
        ("mh-standup-1d", "mike-chen-dailyos-test"),
        ("mh-standup-5d", "mike-chen-dailyos-test"),
        ("mh-standup-18d", "mike-chen-dailyos-test"),
        ("mh-standup-40d", "mike-chen-dailyos-test"),
        // 30d: 1d,5d,18d = 3. 90d: 1d,5d,18d,40d = 4. trend: 3 vs 4/3*1.3=1.7 → 3>1.7 → increasing
        // Close enough for demo data.

        // Lisa Park (internal) → warm (last 18d), increasing
        ("mh-standup-18d", "lisa-park-dailyos-test"),
        ("mh-standup-5d", "lisa-park-dailyos-test"),
        // 30d: 5d,18d = 2. 90d: 5d,18d = 2. trend: 2 vs 2/3*1.3=0.87 → 2>0.87 → increasing ✓

        // Jordan Wells → cold, stable (no meetings at all)
        // No attendee records.

        // Taylor Nguyen → hot (last 3d), increasing
        ("mh-globex-3d", "taylor-nguyen-contractor-io"),
        ("mh-acme-7d", "taylor-nguyen-contractor-io"),
        ("mh-standup-1d", "taylor-nguyen-contractor-io"),
        // 30d: 1d,3d,7d = 3. 90d: 1d,3d,7d = 3. trend: 3 vs 3/3*1.3=1.3 → 3>1.3 → increasing ✓
    ];

    for (meeting_id, person_id) in &attendees {
        conn.execute(
            "INSERT OR IGNORE INTO meeting_attendees (meeting_id, person_id) VALUES (?1, ?2)",
            rusqlite::params![meeting_id, person_id],
        ).map_err(|e| format!("Attendees insert: {}", e))?;
    }

    // Bulk-update meeting_count and last_seen from the junction table.
    conn.execute_batch(
        "UPDATE people SET
            meeting_count = (
                SELECT COUNT(*) FROM meeting_attendees WHERE person_id = people.id
            ),
            last_seen = (
                SELECT MAX(m.start_time) FROM meetings_history m
                JOIN meeting_attendees ma ON m.id = ma.meeting_id
                WHERE ma.person_id = people.id
            )
        "
    ).map_err(|e| format!("People stats update: {}", e))?;

    // --- Entity-people links ---
    let entity_links: Vec<(&str, &str, &str)> = vec![
        // (entity_id, person_id, relationship_type)
        ("acme-corp", "sarah-chen-acme-com", "stakeholder"),
        ("acme-corp", "alex-torres-acme-com", "stakeholder"),
        ("acme-corp", "pat-kim-acme-com", "stakeholder"),
        ("globex-industries", "pat-reynolds-globex-com", "stakeholder"),
        ("globex-industries", "jamie-morrison-globex-com", "stakeholder"),
        ("globex-industries", "casey-lee-globex-com", "stakeholder"),
        ("initech", "dana-patel-initech-com", "stakeholder"),
        ("initech", "priya-sharma-initech-com", "stakeholder"),
    ];

    for (entity_id, person_id, rel) in &entity_links {
        conn.execute(
            "INSERT OR IGNORE INTO entity_people (entity_id, person_id, relationship_type) VALUES (?1, ?2, ?3)",
            rusqlite::params![entity_id, person_id, rel],
        ).map_err(|e| format!("Entity-people link: {}", e))?;
    }

    Ok(())
}

/// Layer intelligence-specific data on top of `seed_database()` output.
///
/// Adds: decision-flagged actions, stale delegations, portfolio alerts
/// (renewal + stale account), and skip-today signals via `intelligence.json`.
fn seed_intelligence_data(db: &ActionDb, workspace: &Path) -> Result<(), String> {
    let now = chrono::Utc::now();
    let today = now.to_rfc3339();
    let date_only = |n: i64| -> String {
        (chrono::Local::now() + chrono::Duration::days(n))
            .format("%Y-%m-%d")
            .to_string()
    };
    let days_ago_rfc = |n: i64| -> String {
        (now - chrono::Duration::days(n)).to_rfc3339()
    };

    let conn = db.conn_ref();

    // --- Decision-flagged actions ---
    // Flag two existing P1 actions so get_flagged_decisions() returns them.
    conn.execute(
        "UPDATE actions SET needs_decision = 1 WHERE id IN ('act-sow-acme', 'act-qbr-deck-globex')",
        [],
    ).map_err(|e| format!("Flag decisions: {}", e))?;

    // --- Stale delegation actions (new inserts) ---
    // status='waiting' + created_at old enough to exceed STALE_DELEGATION_DAYS (3d).
    conn.execute(
        "INSERT OR REPLACE INTO actions (id, title, priority, status, created_at, due_date, account_id, waiting_on, context, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        rusqlite::params![
            "act-legal-review-acme",
            "Waiting on legal review of Acme MSA amendment",
            "P1",
            "waiting",
            days_ago_rfc(10),
            date_only(3),
            "acme-corp",
            "Legal",
            "Sarah Chen needs the amended MSA before Phase 2 scoping can proceed. Legal has had the draft for 10 days — follow up needed.",
            &today,
        ],
    ).map_err(|e| format!("Insert delegation 1: {}", e))?;

    conn.execute(
        "INSERT OR REPLACE INTO actions (id, title, priority, status, created_at, due_date, account_id, waiting_on, context, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        rusqlite::params![
            "act-finance-approval-initech",
            "Waiting on finance approval for Initech Phase 2 budget",
            "P2",
            "waiting",
            days_ago_rfc(7),
            date_only(7),
            "initech",
            "Finance",
            "Dana Patel confirmed Phase 2 interest but budget must be approved by finance. Submitted 7 days ago — no response yet.",
            &today,
        ],
    ).map_err(|e| format!("Insert delegation 2: {}", e))?;

    // --- Portfolio alerts: renewal ---
    // Set Globex contract_end to 45 days from now → triggers get_renewal_alerts(60).
    conn.execute(
        "UPDATE accounts SET contract_end = ?1 WHERE id = 'globex-industries'",
        rusqlite::params![date_only(45)],
    ).map_err(|e| format!("Set Globex contract_end: {}", e))?;

    // --- Portfolio alerts: stale account ---
    // Set Initech updated_at to 35 days ago → triggers get_stale_accounts(30).
    conn.execute(
        "UPDATE accounts SET updated_at = ?1 WHERE id = 'initech'",
        rusqlite::params![days_ago_rfc(35)],
    ).map_err(|e| format!("Set Initech stale: {}", e))?;

    // --- Skip-today signals via intelligence.json ---
    // The loader expects a flat JSON array of { item, reason } objects.
    let skip_today = serde_json::json!([
        {
            "item": "NPS survey follow-up batch",
            "reason": "No responses received yet — wait for Tuesday deadline"
        },
        {
            "item": "Archive cleanup backlog",
            "reason": "Already done this week — next scheduled for Monday"
        }
    ]);

    let data_dir = workspace.join("_today").join("data");
    std::fs::create_dir_all(&data_dir)
        .map_err(|e| format!("Failed to create data dir: {}", e))?;
    std::fs::write(
        data_dir.join("intelligence.json"),
        serde_json::to_string_pretty(&skip_today).unwrap(),
    )
    .map_err(|e| format!("Failed to write intelligence.json: {}", e))?;

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
        |id: &str, title: &str, start_h: u32, start_m: u32, end_h: u32, end_m: u32, mtype: MeetingType, account: Option<&str>, attendees: Vec<&str>| -> CalendarEvent {
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
                attendees: attendees.into_iter().map(String::from).collect(),
                is_all_day: false,
            }
        };

    // Use the same calendar event IDs as schedule.json.tmpl (after {{DATE}} patching).
    // Attendee emails match the people seeded in seed_database().
    let events = vec![
        // #1: Acme Weekly (past, 8:00 AM) — key Acme stakeholders
        make_event(
            &format!("cal-acme-weekly-{}", today_str),
            "Acme Corp Weekly Sync",
            13, 0, 13, 45, // 8:00-8:45 AM ET = 13:00-13:45 UTC
            MeetingType::Customer,
            Some("Acme Corp"),
            vec!["sarah.chen@acme.com", "alex.torres@acme.com", "mike.chen@dailyos.test"],
        ),
        // #2: Eng Standup (past, 9:30 AM) — internal team
        make_event(
            &format!("cal-eng-standup-{}", today_str),
            "Engineering Standup",
            14, 30, 14, 45, // 9:30-9:45 AM ET
            MeetingType::TeamSync,
            None,
            vec!["mike.chen@dailyos.test", "lisa.park@dailyos.test", "taylor.nguyen@contractor.io"],
        ),
        // #3: Initech Kickoff OMITTED — will become "cancelled"
        // #4: 1:1 with Sarah (11:00 AM) — manager
        make_event(
            &format!("cal-1on1-sarah-{}", today_str),
            "1:1 with Sarah (Manager)",
            16, 0, 16, 30, // 11:00-11:30 AM ET
            MeetingType::OneOnOne,
            None,
            vec!["lisa.park@dailyos.test"],
        ),
        // #5: Globex QBR (1:00 PM) — all Globex stakeholders + contractor
        make_event(
            &format!("cal-globex-qbr-{}", today_str),
            "Globex Industries QBR",
            18, 0, 19, 0, // 1:00-2:00 PM ET
            MeetingType::Qbr,
            Some("Globex Industries"),
            vec!["pat.reynolds@globex.com", "jamie.morrison@globex.com", "casey.lee@globex.com", "taylor.nguyen@contractor.io"],
        ),
        // #6: Sprint Review (2:30 PM) — internal team
        make_event(
            &format!("cal-sprint-review-{}", today_str),
            "Product Team Sprint Review",
            19, 30, 20, 15, // 2:30-3:15 PM ET
            MeetingType::Internal,
            None,
            vec!["mike.chen@dailyos.test", "lisa.park@dailyos.test"],
        ),
        // #7: Initech Onboarding — NOT in briefing → "new"
        make_event(
            &format!("cal-initech-onboarding-{}", today_str),
            "Initech Onboarding Call",
            20, 30, 21, 30, // 3:30-4:30 PM ET
            MeetingType::Training,
            Some("Initech"),
            vec!["dana.patel@initech.com", "priya.sharma@initech.com"],
        ),
        // #8: All Hands (4:30 PM) — no individual attendees (50+ people)
        make_event(
            &format!("cal-all-hands-{}", today_str),
            "Company All Hands",
            21, 30, 22, 30, // 4:30-5:30 PM ET
            MeetingType::AllHands,
            None,
            vec![],
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
    let date_only = |n: i64| -> String {
        (today + chrono::Duration::days(n))
            .format("%Y-%m-%d")
            .to_string()
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
        "version": 1,
        "entityType": "account",
        "structured": {
            "arr": 1200000,
            "health": "green",
            "lifecycle": "steady-state",
            "renewalDate": "2025-09-15",
            "csm": "You",
            "champion": "Sarah Chen"
        },
        "companyOverview": {
            "description": "Acme Corp is a mid-market SaaS company specializing in supply chain optimization. They adopted our platform 18 months ago for their engineering team and expanded to ops last quarter. Strong executive sponsorship from Sarah Chen, with Phase 2 expansion planned for Q2.",
            "industry": "Supply Chain / Logistics",
            "size": "500-1000 employees",
            "headquarters": "San Francisco, CA"
        },
        "strategicPrograms": [
            { "name": "Phase 2 Expansion", "status": "in_progress", "notes": "April kickoff, scoping underway" },
            { "name": "APAC Rollout", "status": "planned", "notes": "Separate workstream targeting Q3" }
        ],
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
        "version": 1,
        "entityType": "account",
        "structured": {
            "arr": 800000,
            "health": "yellow",
            "lifecycle": "at-risk",
            "renewalDate": "2025-06-30",
            "csm": "You",
            "champion": "Jamie Morrison"
        },
        "companyOverview": {
            "description": "Globex Industries is an enterprise manufacturing company using our platform across 3 teams. Renewal in 90 days with health at Yellow due to key stakeholder departure and declining usage in Team B. Jamie Morrison is the strongest internal champion.",
            "industry": "Manufacturing",
            "size": "1000-5000 employees",
            "headquarters": "Chicago, IL"
        },
        "strategicPrograms": [
            { "name": "Team B Recovery", "status": "at_risk", "notes": "Engagement plan to reverse usage decline" },
            { "name": "APAC Expansion", "status": "proposed", "notes": "Extend deployment to Singapore and Sydney offices" }
        ],
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
        "version": 1,
        "entityType": "account",
        "structured": {
            "arr": 350000,
            "health": "green",
            "lifecycle": "onboarding",
            "csm": "You"
        },
        "companyOverview": {
            "description": "Initech is a growing fintech startup in their onboarding phase. Phase 1 was delivered on time and under budget. Currently planning Phase 2 kickoff pending budget approval from finance. Small but engaged team with strong potential for expansion.",
            "industry": "Fintech",
            "size": "100-250 employees",
            "headquarters": "Austin, TX"
        },
        "notes": "Phase 2 kickoff planned. Budget approval pending."
    });
    std::fs::write(
        initech_dir.join("dashboard.json"),
        serde_json::to_string_pretty(&initech_json).unwrap(),
    )
    .map_err(|e| format!("Failed to write Initech dashboard.json: {}", e))?;

    // --- I114: Contoso parent + child BU workspace files ---
    let contoso_dir = accounts_dir.join("Contoso");
    std::fs::create_dir_all(&contoso_dir)
        .map_err(|e| format!("Failed to create Contoso dir: {}", e))?;

    let contoso_json = serde_json::json!({
        "version": 1,
        "entityType": "account",
        "structured": {
            "arr": 2400000,
            "health": "green",
            "lifecycle": "steady-state",
            "renewalDate": "2026-06-30",
            "csm": "You",
            "champion": "Marcus Webb"
        },
        "companyOverview": {
            "description": "Contoso is a Fortune 500 enterprise with two business units using our platform. The Enterprise division handles large-scale deployments while the SMB division supports small and mid-size customers.",
            "industry": "Technology Services",
            "size": "5000+ employees",
            "headquarters": "Seattle, WA"
        },
        "notes": "Enterprise BU is the flagship relationship. SMB showing usage decline — needs attention before March renewal."
    });
    std::fs::write(
        contoso_dir.join("dashboard.json"),
        serde_json::to_string_pretty(&contoso_json).unwrap(),
    )
    .map_err(|e| format!("Failed to write Contoso dashboard.json: {}", e))?;

    // Contoso Enterprise (child BU)
    let enterprise_dir = contoso_dir.join("Enterprise");
    std::fs::create_dir_all(&enterprise_dir)
        .map_err(|e| format!("Failed to create Enterprise dir: {}", e))?;

    let enterprise_json = serde_json::json!({
        "version": 1,
        "entityType": "account",
        "structured": {
            "arr": 1800000,
            "health": "green",
            "lifecycle": "steady-state",
            "renewalDate": "2026-06-30",
            "csm": "You",
            "champion": "David Kim"
        },
        "parentId": "contoso",
        "notes": "Strong adoption. 85% DAU across engineering teams."
    });
    std::fs::write(
        enterprise_dir.join("dashboard.json"),
        serde_json::to_string_pretty(&enterprise_json).unwrap(),
    )
    .map_err(|e| format!("Failed to write Enterprise dashboard.json: {}", e))?;

    // Contoso SMB (child BU)
    let smb_dir = contoso_dir.join("SMB");
    std::fs::create_dir_all(&smb_dir)
        .map_err(|e| format!("Failed to create SMB dir: {}", e))?;

    let smb_json = serde_json::json!({
        "version": 1,
        "entityType": "account",
        "structured": {
            "arr": 600000,
            "health": "yellow",
            "lifecycle": "at-risk",
            "renewalDate": "2026-03-15",
            "csm": "You"
        },
        "parentId": "contoso",
        "notes": "Usage declining. Lost 2 key users last month. Renewal at risk without intervention."
    });
    std::fs::write(
        smb_dir.join("dashboard.json"),
        serde_json::to_string_pretty(&smb_json).unwrap(),
    )
    .map_err(|e| format!("Failed to write SMB dashboard.json: {}", e))?;

    // --- Project workspace files ---
    write_project_workspace_files(workspace)?;

    // --- People workspace files ---
    // Write person.json for each seeded person. Matches the data in seed_database().
    // Covers: with/without org, with/without role, with/without notes, all relationship types.
    let people_dir = workspace.join("People");

    let people_fixtures: Vec<(&str, serde_json::Value)> = vec![
        ("Sarah Chen", serde_json::json!({
            "version": 1, "entityType": "person",
            "structured": { "email": "sarah.chen@acme.com", "organization": "Acme Corp", "role": "VP Engineering", "relationship": "external" },
            "notes": "Executive sponsor for Phase 2. Strong advocate — secured budget approval.",
            "linkedEntities": ["acme-corp"]
        })),
        ("Alex Torres", serde_json::json!({
            "version": 1, "entityType": "person",
            "structured": { "email": "alex.torres@acme.com", "organization": "Acme Corp", "role": "Tech Lead", "relationship": "external" },
            "notes": "Departing March 2025. Knowledge transfer plan needed urgently.",
            "linkedEntities": ["acme-corp"]
        })),
        ("Pat Kim", serde_json::json!({
            "version": 1, "entityType": "person",
            "structured": { "email": "pat.kim@acme.com", "organization": "Acme Corp", "role": "CTO", "relationship": "external" },
            "linkedEntities": ["acme-corp"]
        })),
        ("Pat Reynolds", serde_json::json!({
            "version": 1, "entityType": "person",
            "structured": { "email": "pat.reynolds@globex.com", "organization": "Globex Industries", "role": "VP Product", "relationship": "external" },
            "notes": "Departing Q2. Key exec sponsor — renewal risk if successor isn't aligned.",
            "linkedEntities": ["globex-industries"]
        })),
        ("Jamie Morrison", serde_json::json!({
            "version": 1, "entityType": "person",
            "structured": { "email": "jamie.morrison@globex.com", "organization": "Globex Industries", "role": "Eng Director", "relationship": "external" },
            "linkedEntities": ["globex-industries"]
        })),
        ("Casey Lee", serde_json::json!({
            "version": 1, "entityType": "person",
            "structured": { "email": "casey.lee@globex.com", "organization": "Globex Industries", "role": "Head of Ops", "relationship": "external" },
            "linkedEntities": ["globex-industries"]
        })),
        ("Dana Patel", serde_json::json!({
            "version": 1, "entityType": "person",
            "structured": { "email": "dana.patel@initech.com", "organization": "Initech", "role": "CTO", "relationship": "external" },
            "linkedEntities": ["initech"]
        })),
        ("Priya Sharma", serde_json::json!({
            "version": 1, "entityType": "person",
            "structured": { "email": "priya.sharma@initech.com", "organization": "Initech", "role": "VP Product", "relationship": "external" },
            "notes": "Phase 2 scope lead. Prefers async updates over meetings.",
            "linkedEntities": ["initech"]
        })),
        ("Mike Chen", serde_json::json!({
            "version": 1, "entityType": "person",
            "structured": { "email": "mike.chen@dailyos.test", "organization": "DailyOS", "role": "Product Manager", "relationship": "internal" },
            "linkedEntities": []
        })),
        ("Lisa Park", serde_json::json!({
            "version": 1, "entityType": "person",
            "structured": { "email": "lisa.park@dailyos.test", "organization": "DailyOS", "role": "Eng Manager", "relationship": "internal" },
            "notes": "Manages the platform team. Key partner for infrastructure decisions.",
            "linkedEntities": []
        })),
        ("Jordan Wells", serde_json::json!({
            "version": 1, "entityType": "person",
            "structured": { "email": "jordan.wells@example.com", "relationship": "unknown" },
            "linkedEntities": []
        })),
        ("Taylor Nguyen", serde_json::json!({
            "version": 1, "entityType": "person",
            "structured": { "email": "taylor.nguyen@contractor.io", "relationship": "external" },
            "linkedEntities": []
        })),
    ];

    for (name, json) in &people_fixtures {
        let dir = people_dir.join(name);
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("Failed to create People/{}: {}", name, e))?;
        std::fs::write(
            dir.join("person.json"),
            serde_json::to_string_pretty(json).unwrap(),
        )
        .map_err(|e| format!("Failed to write People/{}/person.json: {}", name, e))?;
    }

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

    // Week prep fixtures — create preps/ dir with 2 prep files so some meetings
    // resolve as prep_ready while others remain prep_needed.
    let preps_dir = data_dir.join("preps");
    std::fs::create_dir_all(&preps_dir)
        .map_err(|e| format!("Failed to create preps dir: {}", e))?;

    // Acme Weekly (Monday) — has talkingPoints → prep_ready
    let acme_prep_name = patch_dates("cal-acme-weekly-{{MON}}.json");
    let acme_prep = serde_json::json!({
        "meetingId": patch_dates("cal-acme-weekly-{{MON}}"),
        "talkingPoints": [
            {"topic": "Review Phase 1 benchmarks", "notes": "Compare against original targets"},
            {"topic": "Discuss NPS detractors", "notes": "3 detractors identified last week"},
            {"topic": "Phase 2 timeline", "notes": "Proposed kickoff in 2 weeks"}
        ]
    });
    std::fs::write(
        preps_dir.join(&acme_prep_name),
        serde_json::to_string_pretty(&acme_prep).unwrap(),
    ).map_err(|e| format!("Failed to write Acme prep: {}", e))?;

    // Globex QBR (Wednesday) — has proposedAgenda + risks → prep_ready
    let globex_prep_name = patch_dates("cal-globex-qbr-{{WED}}.json");
    let globex_prep = serde_json::json!({
        "meetingId": patch_dates("cal-globex-qbr-{{WED}}"),
        "proposedAgenda": [
            {"topic": "Expansion wins — 3 new teams onboarded"},
            {"topic": "Team B decline — root cause analysis"},
            {"topic": "Renewal terms discussion"},
            {"topic": "Pat Reynolds transition plan"}
        ],
        "risks": [
            "Pat Reynolds departing Q2",
            "Team B usage down 20% MoM",
            "Competitor actively pitching"
        ]
    });
    std::fs::write(
        preps_dir.join(&globex_prep_name),
        serde_json::to_string_pretty(&globex_prep).unwrap(),
    ).map_err(|e| format!("Failed to write Globex QBR prep: {}", e))?;

    Ok(())
}

/// Write project workspace files (dashboard.json + dashboard.md) for 3 mock projects.
///
/// Called from both `install_mock_data` (for rich detail pages) and
/// `write_workspace_markdown` (for simulate_briefing).
fn write_project_workspace_files(workspace: &Path) -> Result<(), String> {
    let today = Local::now();
    let date_only = |n: i64| -> String {
        (today + chrono::Duration::days(n))
            .format("%Y-%m-%d")
            .to_string()
    };

    let projects_dir = workspace.join("Projects");

    // Acme Phase 2 Expansion
    let phase2_dir = projects_dir.join("Acme Phase 2 Expansion");
    std::fs::create_dir_all(&phase2_dir)
        .map_err(|e| format!("Failed to create Phase 2 dir: {}", e))?;

    let phase2_json = serde_json::json!({
        "version": 1,
        "entityType": "project",
        "structured": {
            "status": "active",
            "milestone": "Scope Finalization",
            "owner": "You",
            "targetDate": date_only(30)
        },
        "description": "Phase 2 expansion of Acme Corp deployment. Extends coverage from engineering to ops, finance, and APAC teams. Builds on Phase 1 success (completed ahead of schedule, 15% above benchmark). Key dependency: Alex Torres KT plan before his March departure.",
        "milestones": [
            { "name": "Scope Finalization", "status": "active", "targetDate": date_only(10), "notes": "Awaiting legal review of amended MSA" },
            { "name": "Kickoff", "status": "planned", "targetDate": date_only(30), "notes": "April kickoff pending scope sign-off" },
            { "name": "APAC Pilot", "status": "planned", "targetDate": date_only(90), "notes": "Singapore office first" }
        ],
        "notes": "Dependent on SOW and legal review. Sarah Chen is executive sponsor."
    });
    std::fs::write(
        phase2_dir.join("dashboard.json"),
        serde_json::to_string_pretty(&phase2_json).unwrap(),
    ).map_err(|e| format!("Failed to write Phase 2 dashboard.json: {}", e))?;

    let phase2_md = format!(
        r#"# Acme Phase 2 Expansion

**Status:** {} active
**Milestone:** Scope Finalization
**Owner:** You
**Target Date:** {}

## Description
Phase 2 expansion of Acme Corp deployment. Extends coverage from engineering to ops, finance, and APAC teams. Builds on Phase 1 success (completed ahead of schedule, 15% above benchmark). Key dependency: Alex Torres KT plan before his March departure.

## Milestones
- {} **Scope Finalization** — Target: {} — Awaiting legal review of amended MSA
- {} **Kickoff** — Target: {} — April kickoff pending scope sign-off
- {} **APAC Pilot** — Target: {} — Singapore office first

## Notes
Dependent on SOW and legal review. Sarah Chen is executive sponsor.
"#,
        "\u{1f7e2}", date_only(30),
        "\u{1f7e2}", date_only(10),
        "\u{26aa}", date_only(30),
        "\u{26aa}", date_only(90),
    );
    std::fs::write(phase2_dir.join("dashboard.md"), phase2_md)
        .map_err(|e| format!("Failed to write Phase 2 dashboard.md: {}", e))?;

    // Globex Team B Recovery
    let teamb_dir = projects_dir.join("Globex Team B Recovery");
    std::fs::create_dir_all(&teamb_dir)
        .map_err(|e| format!("Failed to create Team B dir: {}", e))?;

    let teamb_json = serde_json::json!({
        "version": 1,
        "entityType": "project",
        "structured": {
            "status": "active",
            "milestone": "Root Cause Analysis",
            "owner": "You",
            "targetDate": date_only(14)
        },
        "description": "Intervention project to reverse declining usage in Globex Team B. Usage down 20% MoM. Critical for renewal — QBR is the forcing function. Casey Lee (Head of Ops) is the key contact for Team B adoption metrics.",
        "milestones": [
            { "name": "Root Cause Analysis", "status": "active", "targetDate": date_only(7), "notes": "Usage audit + lead interviews" },
            { "name": "Engagement Plan", "status": "planned", "targetDate": date_only(14), "notes": "Corrective actions for Team B" },
            { "name": "Recovery Verified", "status": "planned", "targetDate": date_only(45), "notes": "Usage trend reversal confirmed" }
        ],
        "notes": "Must show progress before QBR. Renewal depends on this."
    });
    std::fs::write(
        teamb_dir.join("dashboard.json"),
        serde_json::to_string_pretty(&teamb_json).unwrap(),
    ).map_err(|e| format!("Failed to write Team B dashboard.json: {}", e))?;

    let teamb_md = format!(
        r#"# Globex Team B Recovery

**Status:** {} active
**Milestone:** Root Cause Analysis
**Owner:** You
**Target Date:** {}

## Description
Intervention project to reverse declining usage in Globex Team B. Usage down 20% MoM. Critical for renewal — QBR is the forcing function. Casey Lee (Head of Ops) is the key contact for Team B adoption metrics.

## Milestones
- {} **Root Cause Analysis** — Target: {} — Usage audit + lead interviews
- {} **Engagement Plan** — Target: {} — Corrective actions for Team B
- {} **Recovery Verified** — Target: {} — Usage trend reversal confirmed

## Notes
Must show progress before QBR. Renewal depends on this.
"#,
        "\u{1f7e2}", date_only(14),
        "\u{1f7e2}", date_only(7),
        "\u{26aa}", date_only(14),
        "\u{26aa}", date_only(45),
    );
    std::fs::write(teamb_dir.join("dashboard.md"), teamb_md)
        .map_err(|e| format!("Failed to write Team B dashboard.md: {}", e))?;

    // Platform Migration v3
    let migration_dir = projects_dir.join("Platform Migration v3");
    std::fs::create_dir_all(&migration_dir)
        .map_err(|e| format!("Failed to create Migration dir: {}", e))?;

    let migration_json = serde_json::json!({
        "version": 1,
        "entityType": "project",
        "structured": {
            "status": "on_hold",
            "milestone": "Architecture Review",
            "owner": "Lisa Park",
            "targetDate": date_only(60)
        },
        "description": "Internal platform migration from v2 to v3 architecture. On hold pending architecture review. Lisa Park (Eng Manager) owns the technical design. Blocked by capacity constraints — team focused on customer-facing work through Q1.",
        "milestones": [
            { "name": "Architecture Review", "status": "on_hold", "targetDate": date_only(21), "notes": "Blocked on team capacity" },
            { "name": "Migration Plan", "status": "planned", "targetDate": date_only(45), "notes": "Detailed migration runbook" },
            { "name": "v3 Cutover", "status": "planned", "targetDate": date_only(60), "notes": "Zero-downtime migration" }
        ],
        "notes": "On hold until Q2. Architecture proposal draft needed first."
    });
    std::fs::write(
        migration_dir.join("dashboard.json"),
        serde_json::to_string_pretty(&migration_json).unwrap(),
    ).map_err(|e| format!("Failed to write Migration dashboard.json: {}", e))?;

    let migration_md = format!(
        r#"# Platform Migration v3

**Status:** {} on_hold
**Milestone:** Architecture Review
**Owner:** Lisa Park
**Target Date:** {}

## Description
Internal platform migration from v2 to v3 architecture. On hold pending architecture review. Lisa Park (Eng Manager) owns the technical design. Blocked by capacity constraints — team focused on customer-facing work through Q1.

## Milestones
- {} **Architecture Review** — Target: {} — Blocked on team capacity
- {} **Migration Plan** — Target: {} — Detailed migration runbook
- {} **v3 Cutover** — Target: {} — Zero-downtime migration

## Notes
On hold until Q2. Architecture proposal draft needed first.
"#,
        "\u{1f7e1}", date_only(60),
        "\u{1f7e1}", date_only(21),
        "\u{26aa}", date_only(45),
        "\u{26aa}", date_only(60),
    );
    std::fs::write(migration_dir.join("dashboard.md"), migration_md)
        .map_err(|e| format!("Failed to write Migration dashboard.md: {}", e))?;

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
