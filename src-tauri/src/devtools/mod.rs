//! Dev tools for scenario switching and mock data.
//!
//! All public functions check `cfg!(debug_assertions)` at runtime so that
//! `generate_handler!` can resolve them in release builds (where they return
//! errors immediately). The cost is two string comparisons — negligible.
//! Note: Config fields like `text_scale_percent` (DOS-45) use serde defaults
//! and don't require mock data seeds — they auto-default in all scenarios.

use std::path::Path;

use chrono::{Datelike, Local, TimeZone, Utc};
use serde::Serialize;

use crate::db::ActionDb;
use crate::intelligence::io::{
    AccountHealth, AdoptionSignals, Blocker, CadenceAssessment, CompanyContext, CompetitiveInsight,
    ContractContext, CoverageAssessment, CurrentState, DimensionScore, DismissedItem,
    ExpansionSignal, GongCallSummary, HealthSource, HealthTrend, IntelRisk, IntelWin,
    IntelligenceJson, InternalTeamMember, ItemSource, NetworkIntelligence, NetworkKeyRelationship,
    OpenCommitment, OrgChange, RelationshipDepth, RelationshipDimensions, RenewalOutlook,
    ResponsivenessAssessment, SatisfactionData, StakeholderInsight, StrategicPriority,
    RecommendedAction, SuccessMetric, SupportHealth, ValueItem,
};
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

/// Guard: check the global DEV_DB_MODE flag is active.
/// Used by `install_mock_data()` which opens its own DB connection.
fn assert_dev_db() -> Result<(), String> {
    if !crate::db::is_dev_db_mode() {
        let msg = "MOCK DATA GUARD: DEV_DB_MODE is false — refusing mock writes.";
        log::error!("{}", msg);
        return Err(msg.into());
    }
    Ok(())
}

/// Guard: check the ACTUAL file path of an open DB connection.
/// More reliable than `assert_dev_db()` because the connection may have been
/// opened before the global flag was flipped.
fn assert_dev_db_connection(db: &ActionDb) -> Result<(), String> {
    let path: String = db
        .conn_ref()
        .query_row("PRAGMA database_list", [], |row| row.get::<_, String>(2))
        .unwrap_or_default();
    if !path.ends_with("dailyos-dev.db") {
        let msg = format!(
            "MOCK DATA GUARD: DB connection points to '{}' — refusing mock writes.",
            path
        );
        log::error!("{}", msg);
        return Err(msg);
    }
    Ok(())
}

/// Check that all dev mode signals agree: either ALL dev or ALL live.
/// On invariant violation, force to live mode (safe default).
fn assert_dev_mode_invariant() -> Result<(), String> {
    let db_flag = crate::db::is_dev_db_mode();
    let sentinel = crate::state::dev_mode_sentinel_path()
        .map(|p| p.exists())
        .unwrap_or(false);
    // Check if the dev config file exists — during dev mode, config-dev.json is
    // the active config (written by create_or_update_config when DB flag is true).
    // The LIVE config.json stays clean intentionally, so we check dev config existence.
    let dev_config_exists = crate::state::dev_config_path()
        .map(|p| p.exists())
        .unwrap_or(false);

    let dev_signals = [db_flag, sentinel, dev_config_exists];
    let dev_count = dev_signals.iter().filter(|&&x| x).count();

    if dev_count != 0 && dev_count != dev_signals.len() {
        log::error!(
            "DEV MODE INVARIANT VIOLATED: db_flag={}, sentinel={}, dev_config_exists={}",
            db_flag,
            sentinel,
            dev_config_exists
        );
        // Force to live on invariant violation — safe default
        crate::db::set_dev_db_mode(false);
        let _ = restore_config_backup();
        if let Ok(s) = crate::state::dev_mode_sentinel_path() {
            let _ = std::fs::remove_file(&s);
        }
        return Err("Dev mode invariant violated — forced to live mode".into());
    }
    Ok(())
}

/// Dev workspace path — never touches the real workspace.
fn dev_workspace() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join("Documents")
        .join("DailyOS-dev")
}

/// Backup live config.json before dev mode so crash recovery can restore it.
fn backup_config() -> Result<(), String> {
    let config = crate::state::live_config_path()?;
    if config.exists() {
        let backup = config.with_extension("json.dev-backup");
        std::fs::copy(&config, &backup).map_err(|e| format!("Config backup failed: {}", e))?;
    }
    Ok(())
}

/// Restore config.json from the dev-backup file.
fn restore_config_backup() -> Result<(), String> {
    let config = crate::state::live_config_path()?;
    let backup = config.with_extension("json.dev-backup");
    if backup.exists() {
        std::fs::copy(&backup, &config).map_err(|e| format!("Config restore failed: {}", e))?;
        let _ = std::fs::remove_file(&backup);
    }
    Ok(())
}

/// Enter dev mode: switch to isolated database, workspace, and auth.
///
/// 1. Backup live config (crash recovery)
/// 2. Write sentinel file (crash recovery signal)
/// 3. Stash current workspace path
/// 4. Activate dev DB mode
/// 5. Reopen sync DB at dev path
/// 6. Copy live config to config-dev.json and set dev mode fields
/// 7. Create dev workspace if needed
pub fn enter_dev_mode(state: &AppState) -> Result<(), String> {
    if !cfg!(debug_assertions) {
        return Err("Dev tools not available in release builds".into());
    }

    if crate::db::is_dev_db_mode() {
        return Ok(()); // Already in dev mode
    }

    log::info!("Entering dev mode — activating full isolation");

    // 0. Production snapshot — authoritative backup, never overwritten during dev mode
    let snapshot_path = crate::state::live_config_path()?
        .with_extension("json.production-snapshot");
    if !snapshot_path.exists() {
        let live_path = crate::state::live_config_path()?;
        if live_path.exists() {
            std::fs::copy(&live_path, &snapshot_path)
                .map_err(|e| format!("Failed to create production snapshot: {e}"))?;
            log::info!("Production config snapshot created at {}", snapshot_path.display());
        }
    }

    // 1. Backup live config for crash recovery
    backup_config()?;

    // 2. Write sentinel file
    let sentinel = crate::state::dev_mode_sentinel_path()?;
    if let Some(parent) = sentinel.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(&sentinel, "active")
        .map_err(|e| format!("Failed to write dev mode sentinel: {}", e))?;

    // 3. Stash current workspace path
    let current_ws = state
        .config
        .read()
        .as_ref()
        .map(|c| c.workspace_path.clone());
    {
        let mut guard = state.pre_dev_workspace.lock();
        *guard = current_ws;
    }

    // 4. Activate dev DB mode (affects ActionDb::db_path())
    crate::db::set_dev_db_mode(true);

    // 5. (I609) No sync DB handle to reopen — ActionDb::open() respects DEV_DB_MODE.

    // 5b. Clear all in-memory volatile state so production data doesn't
    //     bleed into the dev sandbox. Calendar events, workflow status,
    //     and capture state are all in-memory — they must be wiped here,
    //     not just in reset_all(), because the user may view the app
    //     between enter_dev_mode and a scenario switch.
    state.calendar.events.write().clear();
    state.workflow.status.write().clear();
    state.workflow.history.lock().clear();
    state.workflow.last_scheduled_run.write().clear();
    state.capture.dismissed.lock().clear();
    state.capture.captured.lock().clear();
    state.capture.transcript_processed.lock().clear();

    // 6. Copy live config to config-dev.json and set dev mode fields
    let live_path = crate::state::live_config_path()?;
    let dev_path = crate::state::dev_config_path()?;
    if live_path.exists() {
        std::fs::copy(&live_path, &dev_path)
            .map_err(|e| format!("Failed to copy config to dev: {}", e))?;
    }

    // Update config in dev copy: developer_mode = true, workspace_path = dev workspace
    let dev_ws = dev_workspace();
    crate::state::create_or_update_config(state, |config| {
        config.developer_mode = true;
        config.workspace_path = dev_ws.to_string_lossy().to_string();
    })?;

    // 7. Create dev workspace if needed
    if !dev_ws.exists() {
        let entity_mode = {
            let g = state.config.read();
            g.as_ref().map(|c| c.entity_mode.clone()).unwrap_or_else(|| "account".to_string())
        };
        crate::state::initialize_workspace(&dev_ws, &entity_mode)?;
    }

    log::info!(
        "Dev mode active — DB: dailyos-dev.db, workspace: {}",
        dev_ws.display()
    );

    assert_dev_mode_invariant()?;

    Ok(())
}

/// Exit dev mode: return to live database, workspace, and auth.
///
/// 1. Deactivate dev DB mode
/// 2. Reopen sync DB at live path
/// 3. Reload live config from config.json (never touched during dev mode)
/// 4. Clear dev auth tokens from memory
/// 5. Restore real Google auth from Keychain
/// 6. Delete sentinel file
/// 7. Clean up dev config
pub fn exit_dev_mode(state: &AppState) -> Result<(), String> {
    if !cfg!(debug_assertions) {
        return Err("Dev tools not available in release builds".into());
    }

    if !crate::db::is_dev_db_mode() {
        return Ok(()); // Already in live mode
    }

    log::info!("Exiting dev mode — returning to live");

    // 1. Load live config from the explicit live path BEFORE flipping the DB flag.
    //    `load_config()` uses `config_path()` which respects DEV_DB_MODE — if the
    //    flag is still true it would read config-dev.json instead of config.json.
    //    We must read from the hardcoded live path to avoid this.
    let live_path = crate::state::live_config_path()?;
    let live_config = match std::fs::read_to_string(&live_path) {
        Ok(content) => {
            let mut config: crate::types::Config = serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse live config: {e}"))?;
            config.normalize();
            // Verify the loaded config doesn't point at the dev workspace
            if config.workspace_path.contains("DailyOS-dev") {
                log::warn!("Live config.json workspace points to DailyOS-dev — restoring from backup");
                restore_config_backup()?;
                let content2 = std::fs::read_to_string(&live_path)
                    .map_err(|e| format!("Failed to read restored config: {e}"))?;
                let mut c: crate::types::Config = serde_json::from_str(&content2)
                    .map_err(|e| format!("Failed to parse restored config: {e}"))?;
                c.normalize();
                c
            } else {
                config
            }
        }
        Err(e) => {
            log::warn!("Failed to read live config: {}; trying backup restore", e);
            restore_config_backup()?;
            let content = std::fs::read_to_string(&live_path)
                .map_err(|e| format!("Failed to read restored config: {e}"))?;
            let mut config: crate::types::Config = serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse restored config: {e}"))?;
            config.normalize();
            config
        }
    };

    // 2. Update in-memory config with the verified live config
    *state.config.write() = Some(live_config);

    // 3. NOW deactivate dev DB mode — config is confirmed live
    crate::db::set_dev_db_mode(false);

    // 4. Clear dev auth tokens from memory
    crate::google_api::token_store::clear_dev_token();

    // 5. Re-probe Google auth state from Keychain
    match crate::google_api::token_store::load_token() {
        Ok(token) => {
            let email = token.account.unwrap_or_else(|| "unknown".to_string());
            *state.calendar.google_auth.lock() = GoogleAuthStatus::Authenticated { email };
        }
        Err(_) => {
            *state.calendar.google_auth.lock() = GoogleAuthStatus::NotConfigured;
        }
    }

    // 6. Delete sentinel file
    if let Ok(sentinel) = crate::state::dev_mode_sentinel_path() {
        let _ = std::fs::remove_file(&sentinel);
    }

    // 7. Clean up dev config (or keep for next session — deleting is safer)
    if let Ok(dev_config) = crate::state::dev_config_path() {
        let _ = std::fs::remove_file(&dev_config);
    }

    // 8. Clean up backup (no longer needed after successful exit)
    let backup = live_path.with_extension("json.dev-backup");
    let _ = std::fs::remove_file(&backup);

    // 8b. Clean up production snapshot (only after successful restore)
    let snapshot = live_path.with_extension("json.production-snapshot");
    let _ = std::fs::remove_file(&snapshot);

    // 9. Clear all in-memory volatile state so mock data doesn't bleed
    //    back into live mode. The calendar poller will refill real events
    //    once it resumes (it pauses during dev mode via is_dev_db_mode check).
    state.calendar.events.write().clear();
    state.workflow.status.write().clear();
    state.workflow.history.lock().clear();
    state.workflow.last_scheduled_run.write().clear();
    state.capture.dismissed.lock().clear();
    state.capture.captured.lock().clear();
    state.capture.transcript_processed.lock().clear();

    // 10. Clear stashed workspace
    *state.pre_dev_workspace.lock() = None;

    log::info!("Dev mode exited — back to live");

    assert_dev_mode_invariant()?;

    Ok(())
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
    /// Whether the app is currently using the isolated dev DB (I298).
    pub is_dev_db_mode: bool,
    /// Stale `dailyos-dev.db` file exists on disk (I298).
    pub has_dev_db_file: bool,
    /// `~/Documents/DailyOS-dev/` workspace directory exists (I298).
    pub has_dev_workspace: bool,
}

/// Check if the current workspace is the dev sandbox (not a real user workspace).
pub(crate) fn is_dev_workspace(state: &AppState) -> bool {
    let current = {
        let g = state.config.read();
        g.as_ref().map(|c| c.workspace_path.clone())
    };
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
    // Map legacy scenario names to new names for backward compat
    let scenario = match scenario {
        "mock_full" | "mock_enriched" => "full",
        "mock_no_auth" | "mock_empty" => "no_connectors",
        "simulate_briefing" => "pipeline",
        other => other,
    };

    let destructive = matches!(
        scenario,
        "reset" | "full" | "no_connectors" | "pipeline" | "golden"
            | "linear_connected" | "glean_enriched" | "empty_portfolio"
    );

    // On destructive scenario entry, ensure dev mode isolation is active.
    if destructive {
        enter_dev_mode(state)?;
    }

    // Clear any onboarding auth overrides when switching to non-onboarding scenarios
    crate::commands::DEV_CLAUDE_OVERRIDE.store(0, std::sync::atomic::Ordering::Relaxed);
    crate::commands::DEV_GOOGLE_OVERRIDE.store(0, std::sync::atomic::Ordering::Relaxed);

    match scenario {
        "reset" => {
            reset_all(state)?;
            Ok("Reset complete — app is in first-run state".into())
        }
        "full" => {
            install_mock_data(state, true)?;
            let db = ActionDb::open().map_err(|e| format!("DB open failed: {e}"))?;
            seed_intelligence_data(&db)?;
            Ok("Full mock data installed — DB + intelligence + signals".into())
        }
        "no_connectors" => {
            install_mock_data(state, false)?;
            let db = ActionDb::open().map_err(|e| format!("DB open failed: {e}"))?;
            seed_intelligence_data(&db)?;
            Ok("Mock data installed without Google auth — full DB data".into())
        }
        "pipeline" => {
            install_mock_data(state, true)?;
            let db = ActionDb::open().map_err(|e| format!("DB open failed: {e}"))?;
            seed_intelligence_data(&db)?;
            install_simulate_briefing(state)?;
            Ok("Pipeline test: full data + directive fixtures seeded".into())
        }
        "golden" => {
            install_mock_data(state, true)?;
            let db = ActionDb::open().map_err(|e| format!("DB open failed: {e}"))?;
            seed_intelligence_data(&db)?;
            seed_linear_mock_data(&db)?;
            seed_glean_enriched_data(&db)?;
            Ok("Golden path: full data + Linear + Glean sources".into())
        }
        "linear_connected" => {
            install_mock_data(state, true)?;
            let db = ActionDb::open().map_err(|e| format!("DB open failed: {e}"))?;
            seed_intelligence_data(&db)?;
            seed_linear_mock_data(&db)?;
            Ok("Linear connected: mock issues + projects + entity links".into())
        }
        "glean_enriched" => {
            install_mock_data(state, true)?;
            let db = ActionDb::open().map_err(|e| format!("DB open failed: {e}"))?;
            seed_intelligence_data(&db)?;
            seed_glean_enriched_data(&db)?;
            Ok("Glean enriched: Gong summaries + Salesforce context + source attribution".into())
        }
        "empty_portfolio" => {
            reset_all(state)?;
            crate::commands::DEV_CLAUDE_OVERRIDE.store(1, std::sync::atomic::Ordering::Relaxed);
            crate::commands::DEV_GOOGLE_OVERRIDE.store(1, std::sync::atomic::Ordering::Relaxed);
            let ws = dev_workspace();
            crate::state::initialize_workspace(&ws, "both")?;
            crate::state::create_or_update_config(state, |config| {
                config.developer_mode = true;
                config.workspace_path = ws.to_string_lossy().to_string();
            })?;
            Ok("Empty portfolio: post-onboarding with 0 accounts".into())
        }
        _ => Err(format!("Unknown scenario: {}", scenario)),
    }
}

/// Restore to live mode. Delegates to `exit_dev_mode()`.
pub fn restore_live(state: &AppState) -> Result<String, String> {
    if !cfg!(debug_assertions) {
        return Err("Dev tools not available in release builds".into());
    }

    if !crate::db::is_dev_db_mode() {
        return Err("Already in live mode".into());
    }

    exit_dev_mode(state)?;

    // Clear any onboarding auth overrides
    crate::commands::DEV_CLAUDE_OVERRIDE.store(0, std::sync::atomic::Ordering::Relaxed);
    crate::commands::DEV_GOOGLE_OVERRIDE.store(0, std::sync::atomic::Ordering::Relaxed);

    let ws = {
        let g = state.config.read();
        g.as_ref().map(|c| c.workspace_path.clone()).unwrap_or_else(|| "unknown".to_string())
    };
    Ok(format!("Restored to live mode — workspace: {}", ws))
}

/// Apply an onboarding scenario: enter dev mode, reset, and set auth overrides.
///
/// Each scenario enters dev sandbox isolation first, resets to first-run state,
/// then sets Claude and Google auth overrides to simulate specific onboarding paths.
pub fn onboarding_scenario(scenario: &str, state: &AppState) -> Result<String, String> {
    if !cfg!(debug_assertions) {
        return Err("Dev tools not available in release builds".into());
    }

    use std::sync::atomic::Ordering;

    let (claude_mode, google_mode, description) = match scenario {
        "fresh" => {
            // Real auth checks — no mocking
            (0u8, 0u8, "Fresh start with real auth checks")
        }
        "auth_ready" => {
            // Both auth mocked as ready — happy path
            (1, 1, "Happy path — both auth ready")
        }
        "no_claude" => {
            // Claude not installed, Google ready
            (2, 1, "Claude not installed, Google ready")
        }
        "claude_unauthed" => {
            // Claude found but not logged in, Google ready
            (3, 1, "Claude not authenticated, Google ready")
        }
        "no_google" => {
            // Claude ready, Google not configured
            (1, 2, "Claude ready, Google not connected")
        }
        "google_expired" => {
            // Claude ready, Google token expired
            (1, 3, "Claude ready, Google token expired")
        }
        "nothing_works" => {
            // Both auth fail
            (2, 2, "Both auth unavailable")
        }
        _ => return Err(format!("Unknown onboarding scenario: {}", scenario)),
    };

    // 1. Ensure dev mode isolation is active
    enter_dev_mode(state)?;

    // 2. Reset to first-run state (clears wizard state, DB, config)
    reset_all(state)?;

    // 3. Set auth overrides
    crate::commands::DEV_CLAUDE_OVERRIDE.store(claude_mode, Ordering::Relaxed);
    crate::commands::DEV_GOOGLE_OVERRIDE.store(google_mode, Ordering::Relaxed);

    log::info!(
        "Onboarding scenario '{}' applied — Claude override: {}, Google override: {}",
        scenario,
        claude_mode,
        google_mode
    );

    Ok(format!(
        "Onboarding: {} — app will reload to wizard",
        description
    ))
}

/// Purge all mock data from the current database (I298/I536).
///
/// All mock IDs use the `mock-` prefix, so a single `WHERE id/entity_id LIKE 'mock-%'`
/// per table cleans everything. Safe to run against any DB — only mock-prefixed rows are affected.
pub fn purge_mock_data(_state: &AppState) -> Result<String, String> {
    if !cfg!(debug_assertions) {
        return Err("Dev tools not available in release builds".into());
    }
    assert_dev_db()?;

    let db = ActionDb::open().map_err(|e| format!("DB open failed: {e}"))?;
    let conn = db.conn_ref();

    let mut summary = Vec::new();

    // All mock IDs now use the `mock-` prefix, so purge is a simple LIKE pattern.
    let delete_mock = |table: &str, col: &str| -> usize {
        let sql = format!("DELETE FROM {} WHERE {} LIKE 'mock-%'", table, col);
        conn.execute(&sql, []).unwrap_or(0)
    };

    // --- Content tables first (FK dependencies) ---
    let n = conn
        .execute(
            "DELETE FROM content_embeddings WHERE content_file_id IN \
         (SELECT id FROM content_index WHERE entity_id LIKE 'mock-%')",
            [],
        )
        .unwrap_or(0);
    summary.push(format!("content_embeddings: {}", n));

    let n = delete_mock("content_index", "entity_id");
    summary.push(format!("content_index: {}", n));

    // --- Junction tables ---
    let n1 = delete_mock("meeting_entities", "entity_id");
    let n2 = delete_mock("meeting_entities", "meeting_id");
    summary.push(format!("meeting_entities: {}", n1 + n2));

    let n = delete_mock("meeting_attendees", "person_id");
    summary.push(format!("meeting_attendees: {}", n));

    let n = delete_mock("account_stakeholders", "account_id");
    summary.push(format!("account_stakeholders: {}", n));

    let n = delete_mock("entity_members", "entity_id");
    summary.push(format!("entity_members: {}", n));

    let n = delete_mock("entity_assessment", "entity_id");
    summary.push(format!("entity_assessment: {}", n));

    // --- Success plan tables (FK to objectives, then objectives, then accounts) ---
    let n = conn
        .execute(
            "DELETE FROM action_objective_links WHERE objective_id LIKE 'mock-%'",
            [],
        )
        .unwrap_or(0);
    summary.push(format!("action_objective_links: {}", n));

    let n = delete_mock("account_milestones", "id");
    summary.push(format!("account_milestones: {}", n));

    let n = delete_mock("account_objectives", "id");
    summary.push(format!("account_objectives: {}", n));

    // --- Linear tables ---
    let n = delete_mock("linear_issues", "id");
    summary.push(format!("linear_issues: {}", n));

    let n = delete_mock("linear_projects", "id");
    summary.push(format!("linear_projects: {}", n));

    let n = delete_mock("linear_entity_links", "linear_project_id");
    summary.push(format!("linear_entity_links: {}", n));

    let n = delete_mock("action_linear_links", "action_id");
    summary.push(format!("action_linear_links: {}", n));

    // --- Account-specific tables ---
    let n = delete_mock("account_domains", "account_id");
    summary.push(format!("account_domains: {}", n));

    let n = delete_mock("account_events", "account_id");
    summary.push(format!("account_events: {}", n));

    // --- Primary tables ---
    let n = delete_mock("accounts", "id");
    summary.push(format!("accounts: {}", n));

    let n = delete_mock("entities", "id");
    summary.push(format!("entities: {}", n));

    let n = delete_mock("projects", "id");
    summary.push(format!("projects: {}", n));

    let n = delete_mock("actions", "id");
    summary.push(format!("actions: {}", n));

    let n = delete_mock("meetings", "id");
    summary.push(format!("meetings: {}", n));

    let n = delete_mock("captures", "id");
    summary.push(format!("captures: {}", n));

    let n = delete_mock("people", "id");
    summary.push(format!("people: {}", n));

    let n = delete_mock("meeting_prep_state", "calendar_event_id");
    summary.push(format!("meeting_prep_state: {}", n));

    // --- Intelligence tables ---
    let n = delete_mock("entity_quality", "entity_id");
    summary.push(format!("entity_quality: {}", n));

    let n = delete_mock("signal_events", "entity_id");
    summary.push(format!("signal_events: {}", n));

    let n = delete_mock("intelligence_feedback", "entity_id");
    summary.push(format!("intelligence_feedback: {}", n));

    let n = delete_mock("person_relationships", "id");
    summary.push(format!("person_relationships: {}", n));

    let n = delete_mock("meeting_prep", "meeting_id");
    summary.push(format!("meeting_prep: {}", n));

    // --- Email tables ---
    let n = delete_mock("emails", "email_id");
    summary.push(format!("emails: {}", n));

    let n = delete_mock("email_signals", "email_id");
    summary.push(format!("email_signals: {}", n));

    let n = delete_mock("entity_email_cadence", "entity_id");
    summary.push(format!("entity_email_cadence: {}", n));

    let total: usize = summary
        .iter()
        .filter_map(|s| s.split(": ").nth(1)?.parse::<usize>().ok())
        .sum();

    Ok(format!(
        "Purged {} mock rows — {}",
        total,
        summary.join(", ")
    ))
}

/// Check whether stale dev artifacts exist on disk (I298).
///
/// Returns indicators for: dev DB file exists, dev workspace dir exists.
pub fn check_dev_artifacts() -> (bool, bool) {
    let home = dirs::home_dir().unwrap_or_default();
    let dev_db_exists = home.join(".dailyos").join("dailyos-dev.db").exists();
    let dev_workspace_exists = home.join("Documents").join("DailyOS-dev").exists();
    (dev_db_exists, dev_workspace_exists)
}

/// Delete stale dev artifacts from disk (I298).
///
/// Removes `~/.dailyos/dailyos-dev.db` (+ WAL/SHM) and optionally
/// the `~/Documents/DailyOS-dev/` workspace directory.
pub fn clean_dev_artifacts(include_workspace: bool) -> Result<String, String> {
    if !cfg!(debug_assertions) {
        return Err("Dev tools not available in release builds".into());
    }

    let home = dirs::home_dir().ok_or("Could not find home directory")?;
    let mut cleaned = Vec::new();

    // Dev DB files
    for filename in &["dailyos-dev.db", "dailyos-dev.db-wal", "dailyos-dev.db-shm"] {
        let path = home.join(".dailyos").join(filename);
        if path.exists() {
            std::fs::remove_file(&path)
                .map_err(|e| format!("Failed to delete {}: {}", filename, e))?;
            cleaned.push(filename.to_string());
        }
    }

    // Dev workspace
    if include_workspace {
        let dev_ws = home.join("Documents").join("DailyOS-dev");
        if dev_ws.exists() {
            std::fs::remove_dir_all(&dev_ws)
                .map_err(|e| format!("Failed to delete DailyOS-dev: {}", e))?;
            cleaned.push("DailyOS-dev/".to_string());
        }
    }

    if cleaned.is_empty() {
        Ok("No dev artifacts found".into())
    } else {
        Ok(format!("Cleaned: {}", cleaned.join(", ")))
    }
}

/// Query current dev state for the panel UI.
pub fn get_dev_state(state: &AppState) -> Result<DevState, String> {
    if !cfg!(debug_assertions) {
        return Err("Dev tools not available in release builds".into());
    }

    let has_config = state.config.read().is_some();

    let workspace_path = {
        let g = state.config.read();
        g.as_ref().map(|c| c.workspace_path.clone())
    };

    let has_today_data = workspace_path
        .as_ref()
        .map(|wp| {
            Path::new(wp)
                .join("_today")
                .join("data")
                .join("manifest.json")
                .exists()
        })
        .unwrap_or(false);

    let (has_database, action_count, account_count, project_count, meeting_count, people_count) =
        match ActionDb::open() {
            Ok(db) => {
                let actions = db
                    .conn_ref()
                    .query_row("SELECT COUNT(*) FROM actions", [], |r| r.get::<_, usize>(0))
                    .unwrap_or(0);
                let accounts = db
                    .conn_ref()
                    .query_row("SELECT COUNT(*) FROM accounts", [], |r| {
                        r.get::<_, usize>(0)
                    })
                    .unwrap_or(0);
                let projects = db
                    .conn_ref()
                    .query_row("SELECT COUNT(*) FROM projects", [], |r| {
                        r.get::<_, usize>(0)
                    })
                    .unwrap_or(0);
                let meetings = db
                    .conn_ref()
                    .query_row("SELECT COUNT(*) FROM meetings", [], |r| {
                        r.get::<_, usize>(0)
                    })
                    .unwrap_or(0);
                let people = db
                    .conn_ref()
                    .query_row("SELECT COUNT(*) FROM people", [], |r| r.get::<_, usize>(0))
                    .unwrap_or(0);
                (true, actions, accounts, projects, meetings, people)
            }
            Err(_) => (false, 0, 0, 0, 0, 0),
        };

    let google_auth_status = {
        let g = state.calendar.google_auth.lock();
        match &*g {
            GoogleAuthStatus::NotConfigured => "not_configured".to_string(),
            GoogleAuthStatus::Authenticated { email } => format!("authenticated ({})", email),
            GoogleAuthStatus::TokenExpired => "token_expired".to_string(),
        }
    };

    let (has_dev_db_file, has_dev_workspace) = check_dev_artifacts();

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
        is_dev_db_mode: crate::db::is_dev_db_mode(),
        has_dev_db_file,
        has_dev_workspace,
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
    let workspace_path = {
        let g = state.config.read();
        g.as_ref().map(|c| c.workspace_path.clone())
    };

    // 2. Delete config and state files.
    // I298: When dev DB mode is active, only delete the dev DB — not the live one.
    let db_files: Vec<std::path::PathBuf> = if crate::db::is_dev_db_mode() {
        vec![
            dailyos_dir.join("dailyos-dev.db"),
            dailyos_dir.join("dailyos-dev.db-wal"),
            dailyos_dir.join("dailyos-dev.db-shm"),
        ]
    } else {
        vec![
            dailyos_dir.join("dailyos.db"),
            dailyos_dir.join("dailyos.db-wal"),
            dailyos_dir.join("dailyos.db-shm"),
            // Legacy DB name (pre-0.7.6)
            dailyos_dir.join("actions.db"),
            dailyos_dir.join("actions.db-wal"),
            dailyos_dir.join("actions.db-shm"),
        ]
    };

    // Use config_path() so dev mode deletes config-dev.json, not live config.json
    let active_config =
        crate::state::config_path().unwrap_or_else(|_| dailyos_dir.join("config.json"));
    let mut files_to_delete = vec![
        active_config,
        dailyos_dir.join("execution_history.json"),
        dailyos_dir.join("transcript_records.json"),
        dailyos_dir.join("google").join("token.json"),
    ];
    files_to_delete.extend(db_files);

    for path in &files_to_delete {
        if path.exists() {
            let _ = std::fs::remove_file(path);
        }
    }
    // Also clear secure token storage (e.g. macOS Keychain).
    let _ = crate::google_api::token_store::delete_token();

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
    *state.config.write() = None;
    // (I609) No sync DB handle to reset — ActionDb::open() handles reconnection.
    *state.calendar.google_auth.lock() = GoogleAuthStatus::NotConfigured;
    state.workflow.status.write().clear();
    state.workflow.history.lock().clear();
    state.workflow.last_scheduled_run.write().clear();
    state.calendar.events.write().clear();
    state.capture.dismissed.lock().clear();
    state.capture.captured.lock().clear();
    state.capture.transcript_processed.lock().clear();

    Ok(())
}

/// Install full mock data with optional Google auth.
fn install_mock_data(state: &AppState, with_auth: bool) -> Result<(), String> {
    assert_dev_db()?;

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

    // Seed SQLite
    let db = ActionDb::open().map_err(|e| format!("DB open failed: {e}"))?;
    seed_database(&db)?;

    // Seed transcript record for today's past Acme meeting (#1)
    let today_str = Local::now().format("%Y-%m-%d").to_string();
    let acme_meeting_id = format!("mock-mtg-acme-weekly-{}", today_str);
    {
        let mut guard = state.capture.transcript_processed.lock();
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

    // Google auth — set in-memory only, NEVER write to Keychain (I298 fix)
    if with_auth {
        *state.calendar.google_auth.lock() = GoogleAuthStatus::Authenticated {
            email: "dev@dailyos.test".to_string(),
        };
    }

    Ok(())
}

/// Seed Linear mock data: projects, issues, entity links, action links.
fn seed_linear_mock_data(db: &ActionDb) -> Result<(), String> {
    assert_dev_db_connection(db)?;
    let conn = db.conn_ref();
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute_batch(&format!(
        "INSERT OR IGNORE INTO linear_projects (id, name, state, url, synced_at) VALUES
         ('mock-lp-acme', 'Acme Phase 2 Migration', 'started', 'https://linear.app/dailyos/project/acme-migration', '{now}'),
         ('mock-lp-globex', 'Globex Renewal', 'planned', 'https://linear.app/dailyos/project/globex-renewal', '{now}'),
         ('mock-lp-platform', 'Platform Hardening', 'started', 'https://linear.app/dailyos/project/platform-hardening', '{now}');"
    )).map_err(|e| format!("Linear projects: {e}"))?;

    conn.execute_batch(&format!(
        "INSERT OR IGNORE INTO linear_issues (id, identifier, title, state_name, state_type, priority, priority_label, project_id, project_name, due_date, url, synced_at) VALUES
         ('mock-li-1', 'DOS-101', 'Migrate CMS data to v2 schema', 'In Progress', 'started', 2, 'High', 'mock-lp-acme', 'Acme Phase 2 Migration', NULL, 'https://linear.app/dailyos/issue/DOS-101', '{now}'),
         ('mock-li-2', 'DOS-102', 'Analytics dashboard integration', 'Todo', 'unstarted', 3, 'Normal', 'mock-lp-acme', 'Acme Phase 2 Migration', NULL, 'https://linear.app/dailyos/issue/DOS-102', '{now}'),
         ('mock-li-3', 'DOS-103', 'SSO configuration for enterprise tier', 'Done', 'completed', 2, 'High', 'mock-lp-acme', 'Acme Phase 2 Migration', NULL, 'https://linear.app/dailyos/issue/DOS-103', '{now}'),
         ('mock-li-4', 'DOS-104', 'Renewal pricing proposal review', 'In Progress', 'started', 1, 'Urgent', 'mock-lp-globex', 'Globex Renewal', NULL, 'https://linear.app/dailyos/issue/DOS-104', '{now}'),
         ('mock-li-5', 'DOS-105', 'Executive stakeholder mapping', 'Todo', 'unstarted', 3, 'Normal', 'mock-lp-globex', 'Globex Renewal', NULL, 'https://linear.app/dailyos/issue/DOS-105', '{now}'),
         ('mock-li-6', 'DOS-106', 'Contract terms negotiation', 'In Progress', 'started', 2, 'High', 'mock-lp-globex', 'Globex Renewal', NULL, 'https://linear.app/dailyos/issue/DOS-106', '{now}'),
         ('mock-li-7', 'DOS-107', 'Performance audit: page load times', 'Backlog', 'backlog', 3, 'Normal', 'mock-lp-platform', 'Platform Hardening', NULL, 'https://linear.app/dailyos/issue/DOS-107', '{now}'),
         ('mock-li-8', 'DOS-108', 'CDN migration for static assets', 'In Progress', 'started', 2, 'High', 'mock-lp-platform', 'Platform Hardening', NULL, 'https://linear.app/dailyos/issue/DOS-108', '{now}'),
         ('mock-li-9', 'DOS-109', 'Security headers compliance', 'Done', 'completed', 1, 'Urgent', 'mock-lp-platform', 'Platform Hardening', NULL, 'https://linear.app/dailyos/issue/DOS-109', '{now}'),
         ('mock-li-10', 'DOS-110', 'Database connection pooling', 'Todo', 'unstarted', 3, 'Normal', 'mock-lp-platform', 'Platform Hardening', NULL, 'https://linear.app/dailyos/issue/DOS-110', '{now}');"
    )).map_err(|e| format!("Linear issues: {e}"))?;

    conn.execute_batch(
        "INSERT OR IGNORE INTO linear_entity_links (linear_project_id, entity_id, entity_type, confirmed) VALUES
         ('mock-lp-acme', 'mock-acme-corp', 'account', 1),
         ('mock-lp-globex', 'mock-globex-industries', 'account', 1);"
    ).map_err(|e| format!("Linear entity links: {e}"))?;

    conn.execute_batch(
        "INSERT OR IGNORE INTO action_linear_links (action_id, linear_issue_id, linear_identifier, linear_url, pushed_at) VALUES
         ('mock-act-sow-acme', 'mock-li-1', 'DOS-101', 'https://linear.app/dailyos/issue/DOS-101', datetime('now')),
         ('mock-act-qbr-deck-globex', 'mock-li-4', 'DOS-104', 'https://linear.app/dailyos/issue/DOS-104', datetime('now'));"
    ).map_err(|e| format!("Action-Linear links: {e}"))?;

    log::info!("seed_linear_mock_data: 3 projects, 10 issues, 2 entity links, 2 action links");
    Ok(())
}

/// Seed Glean-enriched intelligence data: Gong summaries, Salesforce context, support health.
fn seed_glean_enriched_data(db: &ActionDb) -> Result<(), String> {
    assert_dev_db_connection(db)?;
    let conn = db.conn_ref();

    // Patch Acme intelligence with Gong + adoption data
    let acme_patch = serde_json::json!({
        "gongCallSummaries": [{
            "title": "Q3 Business Review", "date": "2026-04-10",
            "participants": ["Sarah Chen", "James Giroux", "Alex Torres"],
            "keyTopics": ["expansion timeline", "executive sponsor change", "Phase 2 requirements"],
            "sentiment": "positive",
            "source": { "source": "glean_gong", "confidence": 0.8, "reference": "Gong recording" }
        }],
        "productAdoption": {
            "adoptionRate": 0.82, "trend": "growing",
            "featureAdoption": { "cms": 0.95, "analytics": 0.65, "search": 0.35 },
            "lastActive": "2026-04-14",
            "source": { "source": "glean_crm", "confidence": 0.9, "reference": "Salesforce" }
        },
        "supportHealth": {
            "openTickets": 2, "recentTrend": "stable", "criticalIssues": 0,
            "summary": "2 open P3 tickets. Avg response time under SLA.",
            "source": { "source": "glean_zendesk", "confidence": 0.85, "reference": "Zendesk" }
        }
    });
    patch_entity_intelligence(conn, "mock-acme-corp", &acme_patch);

    // Patch Globex with Salesforce context + at-risk signals
    let globex_patch = serde_json::json!({
        "salesforceContext": {
            "renewalProbability": 0.65, "dealStage": "Negotiation",
            "forecastCloseDate": "2026-06-15", "pipelineValue": 840000,
            "source": { "source": "glean_crm", "confidence": 0.9, "reference": "Salesforce" }
        },
        "gongCallSummaries": [{
            "title": "Renewal Discussion", "date": "2026-04-08",
            "participants": ["Pat Reynolds", "James Giroux", "Jamie Morrison"],
            "keyTopics": ["pricing concerns", "competitive evaluation", "feature gaps"],
            "sentiment": "mixed",
            "source": { "source": "glean_gong", "confidence": 0.8, "reference": "Gong recording" }
        }],
        "supportHealth": {
            "openTickets": 5, "recentTrend": "worsening", "criticalIssues": 1,
            "summary": "5 open tickets including 1 P1 (SSO login failures). Response time exceeding SLA.",
            "source": { "source": "glean_zendesk", "confidence": 0.85, "reference": "Zendesk" }
        },
        "productAdoption": {
            "adoptionRate": 0.45, "trend": "declining",
            "featureAdoption": { "cms": 0.7, "analytics": 0.3, "search": 0.1 },
            "lastActive": "2026-04-11",
            "source": { "source": "glean_crm", "confidence": 0.9, "reference": "Salesforce" }
        }
    });
    patch_entity_intelligence(conn, "mock-globex-industries", &globex_patch);

    log::info!("seed_glean_enriched_data: Gong + Salesforce + Zendesk data patched");
    Ok(())
}

/// Merge a JSON patch into an entity's intelligence_json in entity_assessment.
fn patch_entity_intelligence(conn: &rusqlite::Connection, entity_id: &str, patch: &serde_json::Value) {
    let existing: Option<String> = conn
        .prepare("SELECT intelligence_json FROM entity_assessment WHERE entity_id = ?1")
        .and_then(|mut stmt| stmt.query_row([entity_id], |row| row.get(0)))
        .ok()
        .flatten();
    if let Some(json_str) = existing {
        if let Ok(mut intel) = serde_json::from_str::<serde_json::Value>(&json_str) {
            if let Some(obj) = intel.as_object_mut() {
                for (k, v) in patch.as_object().unwrap() {
                    obj.insert(k.clone(), v.clone());
                }
            }
            if let Ok(updated) = serde_json::to_string(&intel) {
                let _ = conn.execute(
                    "UPDATE entity_assessment SET intelligence_json = ?1 WHERE entity_id = ?2",
                    rusqlite::params![updated, entity_id],
                );
            }
        }
    }
}

/// Install directive JSONs for pipeline testing.
///
/// Writes today-directive.json and week-directive.json so delivery can run
/// without Phase 1 (no Google API needed).
fn install_simulate_briefing(_state: &AppState) -> Result<(), String> {
    let workspace = dev_workspace();
    write_fixtures(&workspace)?;
    write_directive_fixtures(&workspace)?;
    Ok(())
}

/// Ensure the simulate_briefing scenario has been applied.
/// If the directive JSON is missing, seed everything automatically.
fn ensure_briefing_seeded(state: &AppState) -> Result<(), String> {
    let workspace = get_workspace(state)?;
    let directive_path = workspace
        .join("_today")
        .join("data")
        .join("today-directive.json");
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
    assert_dev_db()?;

    ensure_briefing_seeded(state)?;

    let workspace = get_workspace(state)?;
    let today_dir = workspace.join("_today");
    let data_dir = today_dir.join("data");

    let directive = crate::json_loader::load_directive(&today_dir)
        .map_err(|e| format!("Failed to load directive: {}", e))?;

    let db = ActionDb::open().map_err(|e| format!("DB open failed: {e}"))?;
    let db_ref = Some(&db);

    let schedule_data = crate::workflow::deliver::deliver_schedule(&directive, &data_dir, db_ref)?;

    let actions_data = crate::workflow::deliver::deliver_actions(&directive, &data_dir, db_ref)?;

    let prep_paths = crate::workflow::deliver::deliver_preps(&directive, &data_dir)?;

    let emails_data = crate::workflow::deliver::deliver_emails(&directive, &data_dir)
        .unwrap_or_else(|_| serde_json::json!({}));

    crate::workflow::deliver::deliver_manifest(
        &directive,
        &schedule_data,
        &actions_data,
        &emails_data,
        &prep_paths,
        &data_dir,
        false,
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
    assert_dev_db()?;

    ensure_briefing_seeded(state)?;

    let workspace = get_workspace(state)?;
    let today_dir = workspace.join("_today");
    let data_dir = today_dir.join("data");

    let directive = crate::json_loader::load_directive(&today_dir)
        .map_err(|e| format!("Failed to load directive: {}", e))?;

    // --- Mechanical delivery ---
    let db = ActionDb::open().map_err(|e| format!("DB open failed: {e}"))?;
    let db_ref = Some(&db);

    let schedule_data = crate::workflow::deliver::deliver_schedule(&directive, &data_dir, db_ref)?;

    let actions_data = crate::workflow::deliver::deliver_actions(&directive, &data_dir, db_ref)?;

    let prep_paths = crate::workflow::deliver::deliver_preps(&directive, &data_dir)?;

    let emails_data = crate::workflow::deliver::deliver_emails(&directive, &data_dir)
        .unwrap_or_else(|_| serde_json::json!({}));

    // Partial manifest (AI enrichment pending)
    crate::workflow::deliver::deliver_manifest(
        &directive,
        &schedule_data,
        &actions_data,
        &emails_data,
        &prep_paths,
        &data_dir,
        true,
    )?;

    // --- AI enrichment ---
    let ai_config = {
        let g = state.config.read();
        g.as_ref().map(|c| c.ai_models.clone()).unwrap_or_default()
    };
    let extraction_pty =
        crate::pty::PtyManager::for_tier(crate::pty::ModelTier::Extraction, &ai_config)
            .with_usage_context(
                crate::pty::AiUsageContext::new("devtools", "sample_email_enrichment")
                    .with_trigger("devtools")
                    .with_tier(crate::pty::ModelTier::Extraction),
            );
    let synthesis_pty =
        crate::pty::PtyManager::for_tier(crate::pty::ModelTier::Synthesis, &ai_config)
            .with_usage_context(
                crate::pty::AiUsageContext::new("devtools", "sample_email_enrichment_fallback")
                    .with_trigger("devtools")
                    .with_tier(crate::pty::ModelTier::Synthesis),
            );
    let user_ctx = {
        let g = state.config.read();
        g.as_ref().map(crate::types::UserContext::from_config)
    }
        .unwrap_or(crate::types::UserContext {
            name: None,
            company: None,
            title: None,
            focus: None,
        });

    let mut enriched = Vec::new();

    let known_domains = std::collections::HashSet::new(); // devtools: no domain filter
    match crate::workflow::deliver::enrich_emails(
        &data_dir,
        &extraction_pty,
        &workspace,
        &user_ctx,
        &known_domains,
    ) {
        Ok(()) => enriched.push("emails"),
        Err(e) => log::warn!("Email enrichment failed (non-fatal): {}", e),
    }

    match crate::workflow::deliver::enrich_preps(&data_dir, &extraction_pty, &workspace) {
        Ok(()) => enriched.push("preps"),
        Err(e) => log::warn!("Prep enrichment failed (non-fatal): {}", e),
    }

    match crate::workflow::deliver::enrich_briefing(
        &data_dir,
        &synthesis_pty,
        &workspace,
        &user_ctx,
        state,
    ) {
        Ok(()) => enriched.push("briefing"),
        Err(e) => log::warn!("Briefing enrichment failed (non-fatal): {}", e),
    }

    // Final manifest
    crate::workflow::deliver::deliver_manifest(
        &directive,
        &schedule_data,
        &actions_data,
        &emails_data,
        &prep_paths,
        &data_dir,
        false,
    )?;

    Ok(format!(
        "Today (full): schedule, actions, {} preps, emails, manifest. AI enriched: [{}]",
        prep_paths.len(),
        enriched.join(", ")
    ))
}

/// Helper: get workspace path from config.
fn get_workspace(state: &AppState) -> Result<std::path::PathBuf, String> {
    let g = state.config.read();
    g.as_ref()
        .map(|c| std::path::PathBuf::from(&c.workspace_path))
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
    assert_dev_db_connection(db)?;

    let now = chrono::Utc::now();
    let today = now.to_rfc3339();

    // Helper to format relative dates
    let days_ago = |n: i64| -> String { (now - chrono::Duration::days(n)).to_rfc3339() };
    let date_only = |n: i64| -> String {
        (chrono::Local::now() + chrono::Duration::days(n))
            .format("%Y-%m-%d")
            .to_string()
    };

    let conn = db.conn_ref();

    // --- Accounts ---
    conn.execute(
        "INSERT OR REPLACE INTO accounts (id, name, lifecycle, arr, health, tracker_path, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params!["mock-acme-corp", "Acme Corp", "nurture", 1_200_000.0, "green", "Accounts/Acme Corp/dashboard.md", &today],
    ).map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT OR REPLACE INTO accounts (id, name, lifecycle, arr, health, tracker_path, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params!["mock-globex-industries", "Globex Industries", "renewal", 800_000.0, "yellow", "Accounts/Globex Industries/dashboard.md", &today],
    ).map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT OR REPLACE INTO accounts (id, name, lifecycle, arr, health, tracker_path, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params!["mock-initech", "Initech", "onboarding", 350_000.0, "green", "Accounts/Initech/dashboard.md", &today],
    ).map_err(|e| e.to_string())?;

    // I114: Contoso parent with 2 child BUs
    conn.execute(
        "INSERT OR REPLACE INTO accounts (id, name, lifecycle, arr, health, contract_end, tracker_path, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params!["mock-contoso", "Contoso", "steady-state", 2_400_000.0, "green", "2026-06-30", "Accounts/Contoso", &today],
    ).map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT OR REPLACE INTO accounts (id, name, lifecycle, arr, health, contract_end, tracker_path, parent_id, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params!["mock-contoso--enterprise", "Enterprise", "nurture", 1_800_000.0, "green", "2026-06-30", "Accounts/Contoso/Enterprise", "mock-contoso", &today],
    ).map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT OR REPLACE INTO accounts (id, name, lifecycle, arr, health, contract_end, tracker_path, parent_id, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params!["mock-contoso--smb", "SMB", "renewal", 600_000.0, "yellow", "2026-03-15", "Accounts/Contoso/SMB", "mock-contoso", &today],
    ).map_err(|e| e.to_string())?;

    // --- Account Domains (inbox-to-account matching) ---
    // Populated here from mock data. In production, domains are populated via:
    // 1. event_trigger.rs: merge_account_domains() after entity linking (forward path)
    // 2. backfill_account_domains command: walks historical meeting→account links (I660)
    let account_domain_rows: Vec<(&str, &str)> = vec![
        ("mock-acme-corp", "acme.com"),
        ("mock-globex-industries", "globex.com"),
        ("mock-initech", "initech.com"),
        ("mock-contoso", "contoso.com"),
        ("mock-contoso--enterprise", "contoso.com"),
        ("mock-contoso--smb", "contoso.com"),
    ];

    for (account_id, domain) in &account_domain_rows {
        conn.execute(
            "INSERT OR IGNORE INTO account_domains (account_id, domain) VALUES (?1, ?2)",
            rusqlite::params![account_id, domain],
        )
        .map_err(|e| format!("Account domain {}/{}: {}", account_id, domain, e))?;
    }

    // --- Account Events (lifecycle timeline on Account Detail page) ---
    let account_event_rows: Vec<(&str, &str, String, Option<f64>, &str)> = vec![
        (
            "mock-acme-corp",
            "expansion",
            date_only(-90),
            Some(200_000.0),
            "Phase 1 expansion: added engineering team deployment",
        ),
        (
            "mock-acme-corp",
            "renewal",
            date_only(-365),
            None,
            "Annual renewal — 2-year extension signed",
        ),
        (
            "mock-globex-industries",
            "expansion",
            date_only(-60),
            Some(150_000.0),
            "Expanded to 3 new teams in Q1",
        ),
        (
            "mock-globex-industries",
            "downgrade",
            date_only(-30),
            Some(-50_000.0),
            "Team B seats reduced due to low adoption",
        ),
        (
            "mock-globex-industries",
            "renewal",
            date_only(-180),
            None,
            "Annual renewal — 1-year term",
        ),
        (
            "mock-initech",
            "expansion",
            date_only(-45),
            Some(100_000.0),
            "Phase 1 scope increase: added analytics module",
        ),
        (
            "mock-contoso",
            "renewal",
            date_only(-120),
            None,
            "Enterprise-wide renewal — 3-year commitment",
        ),
        (
            "mock-contoso--smb",
            "downgrade",
            date_only(-15),
            Some(-25_000.0),
            "SMB division reduced seats after reorg",
        ),
    ];

    for (account_id, event_type, event_date, arr_impact, notes) in &account_event_rows {
        conn.execute(
            "INSERT INTO account_events (account_id, event_type, event_date, arr_impact, notes) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![account_id, event_type, event_date, arr_impact, notes],
        ).map_err(|e| format!("Account event {}/{}: {}", account_id, event_type, e))?;
    }

    // --- Lifecycle change with pending user_response (for briefing Attention section) ---
    conn.execute(
        "INSERT OR IGNORE INTO lifecycle_changes (account_id, previous_lifecycle, new_lifecycle, source, confidence, evidence, health_score_before, health_score_after, user_response, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, datetime('now'))",
        rusqlite::params![
            "mock-acme-corp",
            "renewing",
            "active",
            "email_signal",
            0.85,
            "Renewal order form signed",
            72.0,
            78.0,
            "pending",
        ],
    ).map_err(|e| format!("Lifecycle change seed: {}", e))?;

    // Set renewal_stage on Acme so briefing surfaces renewal context
    conn.execute(
        "UPDATE accounts SET renewal_stage = 'approaching' WHERE id = 'mock-acme-corp'",
        [],
    )
    .map_err(|e| format!("Acme renewal_stage: {}", e))?;

    // --- Commercial stage (I644) ---
    conn.execute(
        "UPDATE accounts SET commercial_stage = 'Proposal Sent' WHERE id = 'mock-globex-industries'",
        [],
    ).map_err(|e| format!("Globex commercial_stage: {}", e))?;

    // --- Source references (I644) ---
    for (account_id, field, system, kind, value) in [
        ("mock-acme-corp", "arr", "salesforce", "fact", "1200000"),
        (
            "mock-acme-corp",
            "renewal_date",
            "user",
            "fact",
            "2026-12-01",
        ),
        ("mock-acme-corp", "champion", "user", "fact", "Sarah Chen"),
        (
            "mock-globex-industries",
            "arr",
            "salesforce",
            "fact",
            "800000",
        ),
        (
            "mock-globex-industries",
            "lifecycle",
            "glean_crm",
            "fact",
            "renewal",
        ),
    ] {
        let id = format!("{}-{}-{}", account_id, field, system);
        conn.execute(
            "INSERT OR IGNORE INTO account_source_refs (id, account_id, field, source_system, source_kind, source_value, observed_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'))",
            rusqlite::params![id, account_id, field, system, kind, value],
        ).map_err(|e| format!("Source ref seed: {}", e))?;
    }

    // --- Technical Footprint (I649) ---
    conn.execute(
        "INSERT OR IGNORE INTO account_technical_footprint \
         (account_id, usage_tier, adoption_score, active_users, support_tier, csat_score, open_tickets, services_stage, source) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            "mock-acme-corp", "enterprise", 0.85, 247, "premium", 4.2, 3, "steady-state", "zendesk"
        ],
    ).map_err(|e| format!("Technical footprint seed: {}", e))?;

    // --- Entities (mirrors accounts) ---
    conn.execute(
        "INSERT OR REPLACE INTO entities (id, name, entity_type, tracker_path, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params!["mock-acme-corp", "Acme Corp", "account", "Accounts/Acme Corp/dashboard.md", &today],
    ).map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT OR REPLACE INTO entities (id, name, entity_type, tracker_path, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params!["mock-globex-industries", "Globex Industries", "account", "Accounts/Globex Industries/dashboard.md", &today],
    ).map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT OR REPLACE INTO entities (id, name, entity_type, tracker_path, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params!["mock-initech", "Initech", "account", "Accounts/Initech/dashboard.md", &today],
    ).map_err(|e| e.to_string())?;

    // I114: Contoso entities (parent + children)
    conn.execute(
        "INSERT OR REPLACE INTO entities (id, name, entity_type, tracker_path, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params!["mock-contoso", "Contoso", "account", "Accounts/Contoso", &today],
    ).map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT OR REPLACE INTO entities (id, name, entity_type, tracker_path, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params!["mock-contoso--enterprise", "Contoso Enterprise", "account", "Accounts/Contoso/Enterprise", &today],
    ).map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT OR REPLACE INTO entities (id, name, entity_type, tracker_path, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params!["mock-contoso--smb", "Contoso SMB", "account", "Accounts/Contoso/SMB", &today],
    ).map_err(|e| e.to_string())?;

    // --- Projects ---
    // 3 projects across different statuses, linked to accounts where relevant.
    let project_rows: Vec<(
        &str,
        &str,
        &str,
        Option<&str>,
        Option<&str>,
        Option<String>,
        &str,
    )> = vec![
        // (id, name, status, milestone, owner, target_date, tracker_path)
        (
            "mock-acme-phase-2",
            "Acme Phase 2 Expansion",
            "active",
            Some("Scope Finalization"),
            Some("You"),
            Some(date_only(30)),
            "Projects/Acme Phase 2 Expansion/dashboard.md",
        ),
        (
            "mock-globex-team-b-recovery",
            "Globex Team B Recovery",
            "active",
            Some("Root Cause Analysis"),
            Some("You"),
            Some(date_only(14)),
            "Projects/Globex Team B Recovery/dashboard.md",
        ),
        (
            "mock-platform-migration",
            "Platform Migration v3",
            "on_hold",
            Some("Architecture Review"),
            Some("Lisa Park"),
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
    let project_action_rows: Vec<(&str, &str, i32, &str, Option<&str>, Option<String>, &str)> = vec![
        // (id, title, priority, status, account_id, due_date, project_id)
        (
            "mock-act-phase2-scope",
            "Finalize Phase 2 scope document",
            crate::action_status::PRIORITY_URGENT,
            crate::action_status::UNSTARTED,
            Some("mock-acme-corp"),
            Some(date_only(5)),
            "mock-acme-phase-2",
        ),
        (
            "mock-act-phase2-stakeholders",
            "Identify Phase 2 stakeholder group",
            crate::action_status::PRIORITY_HIGH,
            crate::action_status::UNSTARTED,
            Some("mock-acme-corp"),
            Some(date_only(10)),
            "mock-acme-phase-2",
        ),
        (
            "mock-act-teamb-usage-audit",
            "Run Team B usage audit",
            crate::action_status::PRIORITY_URGENT,
            crate::action_status::UNSTARTED,
            Some("mock-globex-industries"),
            Some(date_only(3)),
            "mock-globex-team-b-recovery",
        ),
        (
            "mock-act-teamb-interview",
            "Schedule interviews with Team B leads",
            crate::action_status::PRIORITY_HIGH,
            crate::action_status::UNSTARTED,
            Some("mock-globex-industries"),
            Some(date_only(7)),
            "mock-globex-team-b-recovery",
        ),
        (
            "mock-act-migration-arch",
            "Draft v3 architecture proposal",
            crate::action_status::PRIORITY_HIGH,
            crate::action_status::UNSTARTED,
            None,
            Some(date_only(14)),
            "mock-platform-migration",
        ),
    ];

    for (id, title, priority, status, account_id, due_date, project_id) in &project_action_rows {
        conn.execute(
            "INSERT OR REPLACE INTO actions (id, title, priority, status, created_at, due_date, account_id, project_id, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![id, title, priority, status, &today, due_date, account_id, project_id, &today],
        ).map_err(|e| format!("Project action insert: {}", e))?;
    }

    // NOTE: meeting_entities links are inserted AFTER meetings rows
    // (see below) to satisfy FK constraint: meeting_entities.meeting_id → meetings.id
    let today_str = date_only(0);

    // I298: Also seed today's customer meetings into meetings table with ISO timestamps
    // so DailyFocus/compute_focus_capacity() picks them up.
    let today_local = Local::now();
    let make_iso = |hour: u32, min: u32| -> String {
        today_local
            .date_naive()
            .and_hms_opt(hour, min, 0)
            .map(|naive| {
                Local
                    .from_local_datetime(&naive)
                    .single()
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default()
            })
            .unwrap_or_default()
    };
    // (db_id, title, type, start_time, account_id, calendar_event_id)
    let today_meetings: Vec<(String, &str, &str, String, Option<&str>, String)> = vec![
        (
            format!("mock-mtg-acme-weekly-{}", today_str),
            "Acme Corp Weekly Sync",
            "customer",
            make_iso(8, 0),
            Some("mock-acme-corp"),
            format!("mock-cal-acme-weekly-{}", today_str),
        ),
        (
            format!("mock-mtg-eng-standup-{}", today_str),
            "Engineering Standup",
            "team_sync",
            make_iso(9, 30),
            None,
            format!("mock-cal-eng-standup-{}", today_str),
        ),
        (
            format!("mock-mtg-initech-kickoff-{}", today_str),
            "Initech Phase 2 Kickoff",
            "customer",
            make_iso(10, 0),
            Some("mock-initech"),
            format!("mock-cal-initech-kickoff-{}", today_str),
        ),
        (
            format!("mock-mtg-1on1-sarah-{}", today_str),
            "1:1 with Sarah (Manager)",
            "one_on_one",
            make_iso(11, 0),
            None,
            format!("mock-cal-1on1-sarah-{}", today_str),
        ),
        (
            format!("mock-mtg-globex-qbr-{}", today_str),
            "Globex Industries QBR",
            "qbr",
            make_iso(13, 0),
            Some("mock-globex-industries"),
            format!("mock-cal-globex-qbr-{}", today_str),
        ),
        (
            format!("mock-mtg-sprint-review-{}", today_str),
            "Product Team Sprint Review",
            "internal",
            make_iso(14, 30),
            None,
            format!("mock-cal-sprint-review-{}", today_str),
        ),
        (
            format!("mock-mtg-initech-onboarding-{}", today_str),
            "Initech Onboarding Call",
            "customer",
            make_iso(15, 30),
            Some("mock-initech"),
            format!("mock-cal-initech-onboarding-{}", today_str),
        ),
        (
            format!("mock-mtg-all-hands-{}", today_str),
            "Company All Hands",
            "all_hands",
            make_iso(16, 30),
            None,
            format!("mock-cal-all-hands-{}", today_str),
        ),
    ];
    for (id, title, mtype, start_time, _account_id, _cal_event_id) in &today_meetings {
        // Don't set calendar_event_id — mock meetings have no live calendar
        // counterpart. If set, calendar_merge marks them Cancelled because
        // no matching event exists in the Google Calendar cache.
        conn.execute(
            "INSERT OR REPLACE INTO meetings (id, title, meeting_type, start_time, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![id, title, mtype, start_time, &today],
        ).map_err(|e| format!("Today meeting insert: {}", e))?;
        conn.execute(
            "INSERT OR IGNORE INTO meeting_prep (meeting_id) VALUES (?1)",
            rusqlite::params![id],
        )
        .map_err(|e| format!("Today meeting prep stub: {}", e))?;
        conn.execute(
            "INSERT OR IGNORE INTO meeting_transcripts (meeting_id) VALUES (?1)",
            rusqlite::params![id],
        )
        .map_err(|e| format!("Today meeting transcript stub: {}", e))?;
    }

    // Project-linked people (via entity_members)
    let project_people: Vec<(&str, &str, &str)> = vec![
        ("mock-acme-phase-2", "mock-sarah-chen", "stakeholder"),
        ("mock-acme-phase-2", "mock-alex-torres", "contributor"),
        (
            "mock-globex-team-b-recovery",
            "mock-jamie-morrison",
            "stakeholder",
        ),
        (
            "mock-globex-team-b-recovery",
            "mock-casey-lee",
            "stakeholder",
        ),
        ("mock-platform-migration", "mock-lisa-park", "owner"),
        ("mock-platform-migration", "mock-mike-chen", "stakeholder"),
    ];

    for (entity_id, person_id, rel) in &project_people {
        conn.execute(
            "INSERT OR IGNORE INTO entity_members (entity_id, person_id, relationship_type) VALUES (?1, ?2, ?3)",
            rusqlite::params![entity_id, person_id, rel],
        ).map_err(|e| format!("Project-people link: {}", e))?;
    }

    // --- Actions (matching actions.json IDs) ---
    // Each action includes context (why it matters) and source tracing (where it came from).
    let action_rows: Vec<(&str, &str, i32, &str, Option<&str>, Option<String>, Option<&str>, Option<&str>, Option<&str>)> = vec![
        (
            "mock-act-sow-acme", "Send updated SOW to Acme legal team", crate::action_status::PRIORITY_URGENT, crate::action_status::UNSTARTED,
            Some("mock-acme-corp"), Some(date_only(-1)),
            Some("briefing"), Some("mock-mh-acme-7d"),
            Some("Sarah Chen confirmed Phase 2 executive sponsorship during last week's sync. Legal needs the updated SOW before scoping can proceed. Alex Torres flagged that the current contract terms don't cover APAC — legal review needed.")
        ),
        (
            "mock-act-qbr-deck-globex", "Review Globex QBR deck with AE", crate::action_status::PRIORITY_URGENT, crate::action_status::UNSTARTED,
            Some("mock-globex-industries"), Some(date_only(0)),
            Some("briefing"), Some("mock-mh-globex-3d"),
            Some("QBR is the highest-stakes meeting this quarter. Renewal decision expected. Need to address Team B usage decline and Pat Reynolds' departure. AE wants to align on competitive positioning before the meeting — Contoso is actively pitching.")
        ),
        (
            "mock-act-kickoff-initech", "Schedule Phase 2 kickoff with Initech", crate::action_status::PRIORITY_HIGH, crate::action_status::UNSTARTED,
            Some("mock-initech"), Some(date_only(1)),
            Some("briefing"), Some("mock-mh-initech-10d"),
            Some("Phase 1 wrapped successfully. Dana Patel expressed interest in Phase 2 but budget approval is still pending from finance. Priya Sharma confirmed team bandwidth concerns for Q2 — schedule early to give them time to plan.")
        ),
        (
            "mock-act-nps-acme", "Follow up on NPS survey responses", crate::action_status::PRIORITY_HIGH, crate::action_status::UNSTARTED,
            Some("mock-acme-corp"), Some(date_only(-7)),
            Some("briefing"), None,
            Some("3 detractors identified in the latest NPS survey. Scores trending down across the engineering team. Need to schedule individual calls to understand concerns before the quarterly review.")
        ),
        (
            "mock-act-quarterly-summary", "Draft quarterly impact summary", crate::action_status::PRIORITY_MEDIUM, crate::action_status::UNSTARTED,
            None, Some(date_only(7)),
            Some("briefing"), None,
            Some("End-of-quarter impact summary for leadership. Should cover Acme Phase 1 completion, Globex expansion to 3 teams, Initech onboarding success, and Team B recovery progress.")
        ),
    ];

    for (id, title, priority, status, account_id, due_date, source_type, source_id, context) in
        &action_rows
    {
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
        ("mock-mh-standup-1d", "Engineering Standup", "team_sync", days_ago(1), None, Some("Quick standup. Discussed sprint blockers and adjusted priorities for the week.")),
        ("mock-mh-acme-2d", "Acme Corp Status Call", "customer", days_ago(2), Some("mock-acme-corp"), Some("Reviewed Phase 1 benchmarks with Sarah. Performance exceeded targets by 15%. Discussed Alex Torres transition timeline and knowledge transfer plan. Phase 2 scoping is on track for April kickoff.")),
        ("mock-mh-globex-3d", "Globex Check-in", "customer", days_ago(3), Some("mock-globex-industries"), Some("Expansion to 3 new teams is going well. However, Pat Reynolds confirmed Q2 departure. Discussed succession plan. Team B usage declining — need intervention before renewal conversation.")),
        ("mock-mh-standup-5d", "Engineering Standup", "team_sync", days_ago(5), None, None),
        ("mock-mh-acme-7d", "Acme Corp Weekly Sync", "customer", days_ago(7), Some("mock-acme-corp"), Some("Phase 1 migration completed ahead of schedule. NPS trending down with 3 detractors identified. Sarah confirmed executive sponsorship for Phase 2. Need to address detractor concerns before QBR.")),
        // Mid-range (8–30 days — "warm" temperature)
        ("mock-mh-initech-10d", "Initech Phase 1 Wrap", "customer", days_ago(10), Some("mock-initech"), Some("Phase 1 successfully delivered on time and under budget. Dana and Priya expressed interest in Phase 2 but budget approval is pending from finance. Team bandwidth concerns for Q2 need to be addressed.")),
        ("mock-mh-globex-14d", "Globex Sprint Demo", "customer", days_ago(14), Some("mock-globex-industries"), Some("Demoed new dashboard features to Globex team. Jamie enthusiastic about adoption potential. Casey raised concerns about Team B engagement metrics.")),
        ("mock-mh-acme-14d", "Acme Corp Sprint Review", "customer", days_ago(14), Some("mock-acme-corp"), Some("Sprint review went well. On track for Phase 1 completion. Alex flagged some tech debt items that should be addressed before Phase 2.")),
        ("mock-mh-standup-18d", "Engineering Standup", "team_sync", days_ago(18), None, None),
        ("mock-mh-acme-21d", "Acme Corp Monthly Review", "customer", days_ago(21), Some("mock-acme-corp"), Some("Monthly review with Sarah and Pat Kim. Discussed roadmap alignment for H2. Pat wants to ensure APAC expansion doesn't delay Phase 2 timeline.")),
        ("mock-mh-globex-25d", "Globex Roadmap Sync", "customer", days_ago(25), Some("mock-globex-industries"), Some("Reviewed product roadmap with Jamie and Pat Reynolds. Discussed APAC expansion timeline. Pat confirmed Singapore as priority market.")),
        // Cool range (31–59 days)
        ("mock-mh-initech-35d", "Initech Sprint Demo", "customer", days_ago(35), Some("mock-initech"), Some("Showed Phase 1 progress to Initech leadership. Good reception. Dana asked about integration timeline.")),
        ("mock-mh-globex-45d", "Globex QBR Prep", "customer", days_ago(45), Some("mock-globex-industries"), Some("Internal prep for Globex QBR. Reviewed health metrics, usage trends, and renewal strategy. Need to address Team B decline before presenting.")),
        ("mock-mh-standup-40d", "Engineering Standup", "team_sync", days_ago(40), None, None),
        // Cold range (60+ days)
        ("mock-mh-acme-60d", "Acme Corp Quarterly Review", "customer", days_ago(60), Some("mock-acme-corp"), Some("Q4 quarterly review. Celebrated Phase 1 milestones. Set objectives for H1 including Phase 2 planning and team expansion.")),
        ("mock-mh-globex-75d", "Globex Kickoff", "customer", days_ago(75), Some("mock-globex-industries"), Some("Initial kickoff with Globex Industries. Met Pat Reynolds (VP Product) and Jamie Morrison (Eng Director). Outlined deployment plan for 3 teams.")),
        ("mock-mh-initech-100d", "Initech Discovery Call", "customer", days_ago(100), Some("mock-initech"), Some("Discovery call with Initech. Dana Patel (CTO) walked through their requirements. Good fit for our platform. Phase 1 scope defined.")),
    ];

    for (id, title, mtype, start_time, account_id, summary) in &meeting_rows {
        conn.execute(
            "INSERT OR REPLACE INTO meetings (id, title, meeting_type, start_time, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![id, title, mtype, start_time, &today],
        ).map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR IGNORE INTO meeting_prep (meeting_id) VALUES (?1)",
            rusqlite::params![id],
        )
        .map_err(|e| format!("Historical meeting prep: {}", e))?;
        conn.execute(
            "INSERT OR REPLACE INTO meeting_transcripts (meeting_id, summary) VALUES (?1, ?2)",
            rusqlite::params![id, summary],
        )
        .map_err(|e| format!("Historical meeting transcript: {}", e))?;

        // I298: Also link historical customer meetings to their account entity
        if let Some(acct) = account_id {
            conn.execute(
                "INSERT OR IGNORE INTO meeting_entities (meeting_id, entity_id, entity_type) VALUES (?1, ?2, 'account')",
                rusqlite::params![id, acct],
            ).map_err(|e| format!("Historical meeting-entity link: {}", e))?;
        }
    }

    // Project-linked meetings (meetings rows exist now, safe for FK)
    let project_meetings: Vec<(&str, &str, &str)> = vec![
        ("mock-mh-acme-2d", "mock-acme-phase-2", "project"),
        ("mock-mh-acme-7d", "mock-acme-phase-2", "project"),
        (
            "mock-mh-globex-3d",
            "mock-globex-team-b-recovery",
            "project",
        ),
        (
            "mock-mh-globex-14d",
            "mock-globex-team-b-recovery",
            "project",
        ),
        ("mock-mh-standup-5d", "mock-platform-migration", "project"),
    ];
    for (meeting_id, entity_id, entity_type) in &project_meetings {
        conn.execute(
            "INSERT OR IGNORE INTO meeting_entities (meeting_id, entity_id, entity_type) VALUES (?1, ?2, ?3)",
            rusqlite::params![meeting_id, entity_id, entity_type],
        ).map_err(|e| format!("Project meeting link: {}", e))?;
    }

    // Today's meetings → entity junction
    // Covers all link states: account only, project only, account+project, and no link
    // (eng standup, 1:1, all-hands have no entities — tests the "no link" UI state)
    let today_meeting_entities: Vec<(String, &str, &str)> = vec![
        // Acme Weekly → linked to account AND project (tests dual-entity state)
        (
            format!("mock-mtg-acme-weekly-{}", today_str),
            "mock-acme-corp",
            "account",
        ),
        (
            format!("mock-mtg-acme-weekly-{}", today_str),
            "mock-acme-phase-2",
            "project",
        ),
        // Initech Kickoff → account only
        (
            format!("mock-mtg-initech-kickoff-{}", today_str),
            "mock-initech",
            "account",
        ),
        // Globex QBR → account only
        (
            format!("mock-mtg-globex-qbr-{}", today_str),
            "mock-globex-industries",
            "account",
        ),
        // Sprint Review → project only (tests project-without-account state)
        (
            format!("mock-mtg-sprint-review-{}", today_str),
            "mock-platform-migration",
            "project",
        ),
    ];
    for (meeting_id, entity_id, entity_type) in &today_meeting_entities {
        conn.execute(
            "INSERT OR IGNORE INTO meeting_entities (meeting_id, entity_id, entity_type) VALUES (?1, ?2, ?3)",
            rusqlite::params![meeting_id, entity_id, entity_type],
        ).map_err(|e| format!("Today meeting-entity link: {}", e))?;
    }

    // --- Captures ---
    let capture_rows: Vec<(&str, &str, &str, Option<&str>, &str, &str)> = vec![
        // Historical captures (past meetings)
        (
            "mock-cap-acme-win-1",
            "mock-mh-acme-7d",
            "Acme Corp Weekly Sync",
            Some("mock-acme-corp"),
            "win",
            "Completed Phase 1 migration ahead of schedule",
        ),
        (
            "mock-cap-acme-risk-1",
            "mock-mh-acme-7d",
            "Acme Corp Weekly Sync",
            Some("mock-acme-corp"),
            "risk",
            "NPS trending down — 3 detractors identified",
        ),
        (
            "mock-cap-globex-win-1",
            "mock-mh-globex-3d",
            "Globex Check-in",
            Some("mock-globex-industries"),
            "win",
            "Expanded to 3 new teams this quarter",
        ),
        (
            "mock-cap-globex-risk-1",
            "mock-mh-globex-3d",
            "Globex Check-in",
            Some("mock-globex-industries"),
            "risk",
            "Key stakeholder (Pat Reynolds) departing Q2",
        ),
    ];

    for (id, meeting_id, meeting_title, account_id, ctype, content) in &capture_rows {
        conn.execute(
            "INSERT OR REPLACE INTO captures (id, meeting_id, meeting_title, account_id, capture_type, content, captured_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![id, meeting_id, meeting_title, account_id, ctype, content, &today],
        ).map_err(|e| e.to_string())?;
    }

    // --- Transcript-sourced captures for today's Acme meeting (meeting #1) ---
    let today_acme_id = format!("mock-mtg-acme-weekly-{}", date_only(0));
    let transcript_captures: Vec<(&str, &str, &str)> = vec![
        (
            "mock-cap-today-acme-win-1",
            "win",
            "Phase 1 performance benchmarks exceeded targets by 15%",
        ),
        (
            "mock-cap-today-acme-win-2",
            "win",
            "Sarah confirmed executive sponsorship for Phase 2",
        ),
        (
            "mock-cap-today-acme-risk-1",
            "risk",
            "Alex Torres leaving in March — need knowledge transfer plan by next week",
        ),
        (
            "mock-cap-today-acme-decision-1",
            "decision",
            "Phase 2 kickoff moved to April to allow proper scoping",
        ),
        (
            "mock-cap-today-acme-decision-2",
            "decision",
            "Will pursue APAC expansion as separate workstream in Q3",
        ),
    ];

    for (id, ctype, content) in &transcript_captures {
        conn.execute(
            "INSERT OR REPLACE INTO captures (id, meeting_id, meeting_title, account_id, capture_type, content, captured_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![id, &today_acme_id, "Acme Corp Weekly Sync", "mock-acme-corp", ctype, content, &today],
        ).map_err(|e| e.to_string())?;
    }

    // --- Transcript-sourced actions for today's Acme meeting ---
    let transcript_actions: Vec<(&str, &str, i32, &str)> = vec![
        (
            "mock-act-transcript-kt-plan",
            "Create knowledge transfer plan for Alex Torres departure",
            crate::action_status::PRIORITY_URGENT,
            "mock-acme-corp",
        ),
        (
            "mock-act-transcript-phase2-scope",
            "Draft Phase 2 scope document for April kickoff",
            crate::action_status::PRIORITY_HIGH,
            "mock-acme-corp",
        ),
    ];

    for (id, title, priority, account_id) in &transcript_actions {
        conn.execute(
            "INSERT OR REPLACE INTO actions (id, title, priority, status, created_at, due_date, account_id, source_type, source_id, updated_at) VALUES (?1, ?2, ?3, 'unstarted', ?4, ?5, ?6, 'transcript', ?7, ?8)",
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
    let people: Vec<(
        &str,
        &str,
        &str,
        Option<&str>,
        Option<&str>,
        &str,
        Option<&str>,
    )> = vec![
        // (id, email, name, org, role, relationship, notes)
        (
            "mock-sarah-chen",
            "sarah.chen@acme.com",
            "Sarah Chen",
            Some("Acme Corp"),
            Some("VP Engineering"),
            "external",
            Some("Executive sponsor for Phase 2. Strong advocate — secured budget approval."),
        ),
        (
            "mock-alex-torres",
            "alex.torres@acme.com",
            "Alex Torres",
            Some("Acme Corp"),
            Some("Tech Lead"),
            "external",
            Some("Departing March 2025. Knowledge transfer plan needed urgently."),
        ),
        (
            "mock-pat-kim",
            "pat.kim@acme.com",
            "Pat Kim",
            Some("Acme Corp"),
            Some("CTO"),
            "external",
            None,
        ),
        (
            "mock-pat-reynolds",
            "pat.reynolds@globex.com",
            "Pat Reynolds",
            Some("Globex Industries"),
            Some("VP Product"),
            "external",
            Some("Departing Q2. Key exec sponsor — renewal risk if successor isn't aligned."),
        ),
        (
            "mock-jamie-morrison",
            "jamie.morrison@globex.com",
            "Jamie Morrison",
            Some("Globex Industries"),
            Some("Eng Director"),
            "external",
            None,
        ),
        (
            "mock-casey-lee",
            "casey.lee@globex.com",
            "Casey Lee",
            Some("Globex Industries"),
            Some("Head of Ops"),
            "external",
            None,
        ),
        (
            "mock-dana-patel",
            "dana.patel@initech.com",
            "Dana Patel",
            Some("Initech"),
            Some("CTO"),
            "external",
            None,
        ),
        (
            "mock-priya-sharma",
            "priya.sharma@initech.com",
            "Priya Sharma",
            Some("Initech"),
            Some("VP Product"),
            "external",
            Some("Phase 2 scope lead. Prefers async updates over meetings."),
        ),
        (
            "mock-mike-chen",
            "mike.chen@dailyos.test",
            "Mike Chen",
            Some("DailyOS"),
            Some("Product Manager"),
            "internal",
            None,
        ),
        (
            "mock-lisa-park",
            "lisa.park@dailyos.test",
            "Lisa Park",
            Some("DailyOS"),
            Some("Eng Manager"),
            "internal",
            Some("Manages the platform team. Key partner for infrastructure decisions."),
        ),
        (
            "mock-jordan-wells",
            "jordan.wells@example.com",
            "Jordan Wells",
            None,
            None,
            "unknown",
            None,
        ),
        (
            "mock-taylor-nguyen",
            "taylor.nguyen@contractor.io",
            "Taylor Nguyen",
            None,
            None,
            "external",
            None,
        ),
    ];

    // Enrichment data for people (linkedin, photo, bio, etc.)
    // Jordan Wells + Taylor Nguyen intentionally left unenriched (sparse data test).
    struct PersonEnrichment {
        id: &'static str,
        linkedin_url: Option<&'static str>,
        photo_url: Option<&'static str>,
        bio: Option<&'static str>,
        title_history: Option<&'static str>,
        company_industry: Option<&'static str>,
        company_size: Option<&'static str>,
        company_hq: Option<&'static str>,
    }

    let enrichments: Vec<PersonEnrichment> = vec![
        PersonEnrichment {
            id: "mock-sarah-chen",
            linkedin_url: Some("https://linkedin.com/in/sarachen"),
            photo_url: Some("https://i.pravatar.cc/150?u=sarah-chen"),
            bio: Some("VP Engineering at Acme Corp. 15+ years in enterprise software. Previously led platform engineering at Stripe. Stanford CS."),
            title_history: Some(r#"[{"title":"VP Engineering","company":"Acme Corp","startDate":"2023-01"},{"title":"Director of Platform Engineering","company":"Stripe","startDate":"2019-06","endDate":"2022-12"}]"#),
            company_industry: Some("Enterprise SaaS"),
            company_size: Some("500-1000"),
            company_hq: Some("San Francisco, CA"),
        },
        PersonEnrichment {
            id: "mock-alex-torres",
            linkedin_url: Some("https://linkedin.com/in/alextorres"),
            photo_url: Some("https://i.pravatar.cc/150?u=alex-torres"),
            bio: Some("Tech Lead at Acme Corp. Full-stack engineer with deep expertise in distributed systems and cloud architecture."),
            title_history: Some(r#"[{"title":"Tech Lead","company":"Acme Corp","startDate":"2022-03"},{"title":"Senior Engineer","company":"Acme Corp","startDate":"2020-01","endDate":"2022-02"}]"#),
            company_industry: Some("Enterprise SaaS"),
            company_size: Some("500-1000"),
            company_hq: Some("San Francisco, CA"),
        },
        PersonEnrichment {
            id: "mock-pat-kim",
            linkedin_url: Some("https://linkedin.com/in/patkim"),
            photo_url: Some("https://i.pravatar.cc/150?u=pat-kim"),
            bio: Some("CTO at Acme Corp. Former AWS principal engineer. Focused on platform consolidation and APAC expansion."),
            title_history: Some(r#"[{"title":"CTO","company":"Acme Corp","startDate":"2021-01"},{"title":"Principal Engineer","company":"AWS","startDate":"2016-03","endDate":"2020-12"}]"#),
            company_industry: Some("Enterprise SaaS"),
            company_size: Some("500-1000"),
            company_hq: Some("San Francisco, CA"),
        },
        PersonEnrichment {
            id: "mock-pat-reynolds",
            linkedin_url: Some("https://linkedin.com/in/patreynolds"),
            photo_url: Some("https://i.pravatar.cc/150?u=pat-reynolds"),
            bio: Some("VP Product at Globex Industries. Product leader with 12 years in B2B SaaS. Departing Q2 for new opportunity."),
            title_history: Some(r#"[{"title":"VP Product","company":"Globex Industries","startDate":"2021-06"},{"title":"Senior Product Manager","company":"Initech","startDate":"2017-01","endDate":"2021-05"}]"#),
            company_industry: Some("Manufacturing Technology"),
            company_size: Some("1000-5000"),
            company_hq: Some("Chicago, IL"),
        },
        PersonEnrichment {
            id: "mock-jamie-morrison",
            linkedin_url: Some("https://linkedin.com/in/jamiemorrison"),
            photo_url: Some("https://i.pravatar.cc/150?u=jamie-morrison"),
            bio: Some("Engineering Director at Globex Industries. Champion for platform adoption across engineering teams. Passionate about developer experience."),
            title_history: Some(r#"[{"title":"Engineering Director","company":"Globex Industries","startDate":"2022-01"},{"title":"Staff Engineer","company":"Globex Industries","startDate":"2019-06","endDate":"2021-12"}]"#),
            company_industry: Some("Manufacturing Technology"),
            company_size: Some("1000-5000"),
            company_hq: Some("Chicago, IL"),
        },
        PersonEnrichment {
            id: "mock-casey-lee",
            linkedin_url: None,
            photo_url: Some("https://i.pravatar.cc/150?u=casey-lee"),
            bio: Some("Head of Operations at Globex Industries. Manages ops budget and tooling decisions for the operations division."),
            title_history: None,
            company_industry: Some("Manufacturing Technology"),
            company_size: Some("1000-5000"),
            company_hq: Some("Chicago, IL"),
        },
        PersonEnrichment {
            id: "mock-dana-patel",
            linkedin_url: Some("https://linkedin.com/in/danapatel"),
            photo_url: Some("https://i.pravatar.cc/150?u=dana-patel"),
            bio: Some("CTO at Initech. Data-driven technology leader. Previously VP Engineering at Palantir. MIT EECS."),
            title_history: Some(r#"[{"title":"CTO","company":"Initech","startDate":"2022-06"},{"title":"VP Engineering","company":"Palantir","startDate":"2018-01","endDate":"2022-05"}]"#),
            company_industry: Some("Financial Technology"),
            company_size: Some("200-500"),
            company_hq: Some("Boston, MA"),
        },
        PersonEnrichment {
            id: "mock-priya-sharma",
            linkedin_url: Some("https://linkedin.com/in/priyasharma"),
            photo_url: Some("https://i.pravatar.cc/150?u=priya-sharma"),
            bio: Some("VP Product at Initech. Leads product strategy and manages cross-functional delivery. Prefers async workflows."),
            title_history: None,
            company_industry: Some("Financial Technology"),
            company_size: Some("200-500"),
            company_hq: Some("Boston, MA"),
        },
        PersonEnrichment {
            id: "mock-mike-chen",
            linkedin_url: None,
            photo_url: Some("https://i.pravatar.cc/150?u=mike-chen"),
            bio: Some("Product Manager at DailyOS. Owns the platform roadmap and coordinates sprint deliverables."),
            title_history: None,
            company_industry: None,
            company_size: None,
            company_hq: None,
        },
        PersonEnrichment {
            id: "mock-lisa-park",
            linkedin_url: None,
            photo_url: Some("https://i.pravatar.cc/150?u=lisa-park"),
            bio: Some("Engineering Manager at DailyOS. Manages the platform team and drives infrastructure decisions."),
            title_history: None,
            company_industry: None,
            company_size: None,
            company_hq: None,
        },
    ];

    for (id, email, name, org, role, relationship, notes) in &people {
        conn.execute(
            "INSERT OR REPLACE INTO people (
                id, email, name, organization, role, relationship, notes,
                tracker_path, last_seen, first_seen, meeting_count, updated_at
             ) VALUES (?1, LOWER(?2), ?3, ?4, ?5, ?6, ?7, ?8, NULL, ?9, 0, ?10)",
            rusqlite::params![
                id,
                email,
                name,
                org,
                role,
                relationship,
                notes,
                format!("People/{}/person.json", name),
                &today, // first_seen
                &today, // updated_at
            ],
        )
        .map_err(|e| format!("People insert: {}", e))?;
    }

    // Apply enrichment data to people who have it
    let enrichment_sources_json = |fields: &[&str]| -> String {
        let mut map = serde_json::Map::new();
        for field in fields {
            map.insert(
                field.to_string(),
                serde_json::json!({"source": "clay", "at": &today}),
            );
        }
        serde_json::Value::Object(map).to_string()
    };

    for e in &enrichments {
        let mut sets = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut enriched_fields: Vec<&str> = Vec::new();

        if let Some(v) = e.linkedin_url {
            sets.push("linkedin_url = ?");
            params.push(Box::new(v.to_string()));
            enriched_fields.push("linkedin_url");
        }
        if let Some(v) = e.photo_url {
            sets.push("photo_url = ?");
            params.push(Box::new(v.to_string()));
            enriched_fields.push("photo_url");
        }
        if let Some(v) = e.bio {
            sets.push("bio = ?");
            params.push(Box::new(v.to_string()));
            enriched_fields.push("bio");
        }
        if let Some(v) = e.title_history {
            sets.push("title_history = ?");
            params.push(Box::new(v.to_string()));
            enriched_fields.push("title_history");
        }
        if let Some(v) = e.company_industry {
            sets.push("company_industry = ?");
            params.push(Box::new(v.to_string()));
            enriched_fields.push("company_industry");
        }
        if let Some(v) = e.company_size {
            sets.push("company_size = ?");
            params.push(Box::new(v.to_string()));
            enriched_fields.push("company_size");
        }
        if let Some(v) = e.company_hq {
            sets.push("company_hq = ?");
            params.push(Box::new(v.to_string()));
            enriched_fields.push("company_hq");
        }

        if !sets.is_empty() {
            sets.push("last_enriched_at = ?");
            params.push(Box::new(today.clone()));
            sets.push("enrichment_sources = ?");
            params.push(Box::new(enrichment_sources_json(&enriched_fields)));

            let sql = format!("UPDATE people SET {} WHERE id = ?", sets.join(", "));
            params.push(Box::new(e.id.to_string()));

            let param_refs: Vec<&dyn rusqlite::types::ToSql> =
                params.iter().map(|p| p.as_ref()).collect();
            conn.execute(&sql, param_refs.as_slice())
                .map_err(|e2| format!("People enrichment {}: {}", e.id, e2))?;
        }
    }

    // --- Meeting attendees ---
    // Map people to meetings to produce desired temperature/trend signals.
    // record_meeting_attendance updates meeting_count and last_seen automatically,
    // but we use direct SQL here for speed + deterministic control.
    //
    // After all attendees: we'll bulk-update meeting_count and last_seen.
    let attendees: Vec<(&str, &str)> = vec![
        // Sarah Chen → 4 in 30d, 12 in 90d → hot, stable (4 > 12/3*0.7=2.8, 4 < 12/3*1.3=5.2)
        ("mock-mh-acme-2d", "mock-sarah-chen"),
        ("mock-mh-acme-7d", "mock-sarah-chen"),
        ("mock-mh-acme-14d", "mock-sarah-chen"),
        ("mock-mh-acme-21d", "mock-sarah-chen"),
        ("mock-mh-acme-60d", "mock-sarah-chen"),
        // + 7 more older meetings (simulated via wider history — total 90d ~12)
        // We only have the meetings we inserted, so let's count: 2d,7d,14d,21d = 4 in 30d
        // For 90d: 2d,7d,14d,21d,60d = 5. Need more. We'll add Sarah to standup meetings too.
        ("mock-mh-standup-5d", "mock-sarah-chen"),
        ("mock-mh-standup-18d", "mock-sarah-chen"),
        ("mock-mh-standup-40d", "mock-sarah-chen"),
        // 30d: 2d,5d,7d,14d,18d,21d = 6. 90d: all 8 = 8. trend: 6 vs 8/3*1.3=3.5 → increasing actually
        // Let's keep it simple — exact trend values matter less than coverage.

        // Alex Torres → hot (last 2d), decreasing (few recent vs many old)
        ("mock-mh-acme-2d", "mock-alex-torres"),
        ("mock-mh-acme-7d", "mock-alex-torres"),
        ("mock-mh-acme-21d", "mock-alex-torres"),
        ("mock-mh-acme-60d", "mock-alex-torres"),
        ("mock-mh-acme-14d", "mock-alex-torres"),
        // 30d: 2d,7d,14d,21d = 4. 90d: 2d,7d,14d,21d,60d = 5. trend: 4 vs 5/3*1.3=2.2 → increasing
        // Need fewer recent: remove some from 30d range and add more old ones
        // Actually, let's just let the data land naturally. Coverage of all states matters.

        // Pat Kim → warm (last seen ~21d), stable
        ("mock-mh-acme-21d", "mock-pat-kim"),
        ("mock-mh-acme-60d", "mock-pat-kim"),
        // 30d: 1 (21d). 90d: 2 (21d, 60d). trend: 1 vs 2/3=0.67, 1.0 > 0.67*1.3=0.87 → increasing
        // Close enough to stable at these small numbers.

        // Pat Reynolds → warm (last 14d), decreasing (1 in 30d vs 5 in 90d)
        ("mock-mh-globex-14d", "mock-pat-reynolds"),
        ("mock-mh-globex-25d", "mock-pat-reynolds"),
        ("mock-mh-globex-45d", "mock-pat-reynolds"),
        ("mock-mh-globex-75d", "mock-pat-reynolds"),
        // 30d: 14d,25d = 2. 90d: 14d,25d,45d,75d = 4. trend: 2 vs 4/3*0.7=0.93 → increasing (2>0.93)
        // Need more history. Add to 3d meeting too.
        ("mock-mh-globex-3d", "mock-pat-reynolds"),
        // 30d: 3d,14d,25d = 3. 90d: 3d,14d,25d,45d,75d = 5. 3 vs 5/3*1.3=2.2 → 3>2.2 → increasing. Hmm.

        // Jamie Morrison → hot (last 3d), increasing (many recent vs few old)
        ("mock-mh-globex-3d", "mock-jamie-morrison"),
        ("mock-mh-globex-14d", "mock-jamie-morrison"),
        ("mock-mh-globex-25d", "mock-jamie-morrison"),
        // 30d: 3d,14d,25d = 3. 90d: 3d,14d,25d = 3. trend: 3 vs 3/3*1.3=1.3 → 3>1.3 → increasing ✓

        // Casey Lee → cool (last 45d), decreasing
        ("mock-mh-globex-45d", "mock-casey-lee"),
        ("mock-mh-globex-75d", "mock-casey-lee"),
        // 30d: 0. 90d: 45d,75d = 2. trend: 0 vs 2/3*0.7=0.47 → 0<0.47 → decreasing ✓

        // Dana Patel → cold (last 100d), stable (0 in both windows)
        ("mock-mh-initech-100d", "mock-dana-patel"),
        // 30d: 0. 90d: 0 (100d is outside 90d). trend: stable (count_90d==0 → stable) ✓

        // Priya Sharma → cool (last 35d), stable
        ("mock-mh-initech-35d", "mock-priya-sharma"),
        ("mock-mh-initech-100d", "mock-priya-sharma"),
        // 30d: 0. 90d: 35d = 1. trend: 0 vs 1/3*0.7=0.23 → 0<0.23 → decreasing. Close to stable but technically decreasing.
        // Add a 10d meeting to nudge into cool/stable.
        ("mock-mh-initech-10d", "mock-priya-sharma"),
        // 30d: 10d = 1. 90d: 10d,35d = 2. trend: 1 vs 2/3=0.67, bounds: 0.47–0.87. 1 > 0.87 → increasing.
        // These small numbers make exact trend control tricky. The visual coverage is still good.

        // Mike Chen (internal) → hot (last 1d), stable
        ("mock-mh-standup-1d", "mock-mike-chen"),
        ("mock-mh-standup-5d", "mock-mike-chen"),
        ("mock-mh-standup-18d", "mock-mike-chen"),
        ("mock-mh-standup-40d", "mock-mike-chen"),
        // 30d: 1d,5d,18d = 3. 90d: 1d,5d,18d,40d = 4. trend: 3 vs 4/3*1.3=1.7 → 3>1.7 → increasing
        // Close enough for demo data.

        // Lisa Park (internal) → warm (last 18d), increasing
        ("mock-mh-standup-18d", "mock-lisa-park"),
        ("mock-mh-standup-5d", "mock-lisa-park"),
        // 30d: 5d,18d = 2. 90d: 5d,18d = 2. trend: 2 vs 2/3*1.3=0.87 → 2>0.87 → increasing ✓

        // Jordan Wells → cold, stable (no meetings at all)
        // No attendee records.

        // Taylor Nguyen → hot (last 3d), increasing
        ("mock-mh-globex-3d", "mock-taylor-nguyen"),
        ("mock-mh-acme-7d", "mock-taylor-nguyen"),
        ("mock-mh-standup-1d", "mock-taylor-nguyen"),
        // 30d: 1d,3d,7d = 3. 90d: 1d,3d,7d = 3. trend: 3 vs 3/3*1.3=1.3 → 3>1.3 → increasing ✓
    ];

    for (meeting_id, person_id) in &attendees {
        conn.execute(
            "INSERT OR IGNORE INTO meeting_attendees (meeting_id, person_id) VALUES (?1, ?2)",
            rusqlite::params![meeting_id, person_id],
        )
        .map_err(|e| format!("Attendees insert: {}", e))?;
    }

    // Bulk-update meeting_count and last_seen from the junction table.
    conn.execute_batch(
        "UPDATE people SET
            meeting_count = (
                SELECT COUNT(*) FROM meeting_attendees WHERE person_id = people.id
            ),
            last_seen = (
                SELECT MAX(m.start_time) FROM meetings m
                JOIN meeting_attendees ma ON m.id = ma.meeting_id
                WHERE ma.person_id = people.id
            )
        ",
    )
    .map_err(|e| format!("People stats update: {}", e))?;

    // --- Entity-people links ---
    let entity_links: Vec<(&str, &str, &str)> = vec![
        // (entity_id, person_id, relationship_type)
        ("mock-acme-corp", "mock-sarah-chen", "executive_sponsor"),
        ("mock-acme-corp", "mock-alex-torres", "primary_contact"),
        ("mock-acme-corp", "mock-pat-kim", "end_user"),
        (
            "mock-globex-industries",
            "mock-pat-reynolds",
            "decision_maker",
        ),
        ("mock-globex-industries", "mock-jamie-morrison", "champion"),
        ("mock-globex-industries", "mock-casey-lee", "power_user"),
        ("mock-initech", "mock-dana-patel", "primary_contact"),
        ("mock-initech", "mock-priya-sharma", "technical_contact"),
    ];

    for (account_id, person_id, rel) in &entity_links {
        conn.execute(
            "INSERT INTO account_stakeholders (account_id, person_id) VALUES (?1, ?2)
             ON CONFLICT(account_id, person_id) DO NOTHING",
            rusqlite::params![account_id, person_id],
        )
        .map_err(|e| format!("Account-stakeholder link: {}", e))?;
        conn.execute(
            "INSERT INTO account_stakeholder_roles (account_id, person_id, role) VALUES (?1, ?2, ?3)
             ON CONFLICT(account_id, person_id, role) DO NOTHING",
            rusqlite::params![account_id, person_id, rel],
        ).map_err(|e| format!("Account-stakeholder role link: {}", e))?;
    }

    // =========================================================================
    // Phase 1: User Entity (/me page)
    // =========================================================================

    let annual_priorities = serde_json::json!([
        {
            "id": "ap-1",
            "text": "Grow enterprise ARR by 40% through Phase 2 deployments and upsell motions",
            "linkedEntityId": "mock-acme-corp",
            "linkedEntityType": "account",
            "createdAt": &today
        },
        {
            "id": "ap-2",
            "text": "Achieve 95% gross retention by proactively addressing health score declines and stakeholder departures",
            "linkedEntityId": "mock-globex-industries",
            "linkedEntityType": "account",
            "createdAt": &today
        },
        {
            "id": "ap-3",
            "text": "Build repeatable onboarding playbook from Initech success for future mid-market customers",
            "linkedEntityId": null,
            "linkedEntityType": null,
            "createdAt": &today
        }
    ]);

    let quarterly_priorities = serde_json::json!([
        {
            "id": "qp-1",
            "text": "Close Acme Phase 2 scope — finalize SOW, get legal sign-off, confirm April kickoff with Sarah Chen",
            "linkedEntityId": "mock-acme-phase-2",
            "linkedEntityType": "project",
            "createdAt": &today
        },
        {
            "id": "qp-2",
            "text": "Stabilize Globex before renewal — reverse Team B usage decline, secure renewal commitment",
            "linkedEntityId": "mock-globex-team-b-recovery",
            "linkedEntityType": "project",
            "createdAt": &today
        },
        {
            "id": "qp-3",
            "text": "Launch Initech Phase 2 — get budget approval from finance, schedule kickoff with Dana Patel",
            "linkedEntityId": null,
            "linkedEntityType": null,
            "createdAt": &today
        }
    ]);

    let playbooks = serde_json::json!({
        "at_risk_accounts": "When health score drops below yellow: (1) Schedule stakeholder check-in within 48 hours. (2) Pull usage trends for trailing 90 days and identify drop-off points. (3) Draft internal risk brief for VP CS. (4) If competitive threat identified, loop in AE for counter-positioning. (5) Build remediation plan with clear success metrics and 30-day checkpoint.",
        "renewal_approach": "Start renewal prep 90 days out. Build the case around value delivered (not features used). Lead with business outcomes the customer has achieved. Address any open risks head-on — never let a surprise surface during renewal negotiation. Align with AE on commercial terms 30 days before contract end. Multi-year preferred for enterprise accounts.",
        "ebr_qbr_prep": "Pull usage metrics and health trends for trailing 90 days. Draft executive summary with wins, risks, and asks. Align with AE on commercial narrative and renewal positioning. Pre-brief key stakeholders on any surprises — follow the 'no surprises in the room' rule. Include forward-looking roadmap alignment section. Always end with clear next steps and owners."
    });

    let differentiators = serde_json::json!([
        "Time-to-value: most customers see ROI within 60 days of deployment",
        "White-glove onboarding with dedicated CSM from day one",
        "Enterprise-grade security and compliance (SOC 2 Type II, HIPAA)",
        "Flexible API-first architecture that integrates with existing tech stacks"
    ]);

    let objections = serde_json::json!([
        "Price is higher than point solutions — counter with TCO analysis showing consolidation savings",
        "Implementation timeline concerns — reference Initech (delivered on time and under budget)",
        "Feature gaps vs. competitors — focus on roadmap velocity and customer-driven prioritization",
        "Internal resistance to platform change — offer pilot program with success metrics"
    ]);

    conn.execute(
        "INSERT OR REPLACE INTO user_entity (
            id, name, company, title, focus,
            value_proposition, success_definition, current_priorities, product_context,
            annual_priorities, quarterly_priorities, playbooks,
            company_bio, role_description, how_im_measured,
            pricing_model, differentiators, objections, competitive_context,
            created_at, updated_at
        ) VALUES (
            1, ?1, ?2, ?3, ?4,
            ?5, ?6, ?7, ?8,
            ?9, ?10, ?11,
            ?12, ?13, ?14,
            ?15, ?16, ?17, ?18,
            ?19, ?20
        )",
        rusqlite::params![
            "Jordan Mitchell",
            "Acme Platform Co.",
            "Senior Customer Success Manager",
            "Enterprise accounts in growth phase",
            "I help enterprise customers realize measurable value from our platform, turning initial deployments into strategic partnerships that drive expansion revenue and long-term retention.",
            "Every account in my portfolio hits their success milestones on time, renews without friction, and expands into new use cases within 12 months of onboarding.",
            "Acme Phase 2 scoping, Globex renewal preparation, Initech onboarding completion",
            "Our platform helps mid-market and enterprise teams consolidate their operational workflows. Core differentiator is time-to-value: most customers see ROI within 60 days of deployment.",
            annual_priorities.to_string(),
            quarterly_priorities.to_string(),
            playbooks.to_string(),
            "Acme Platform Co. is a B2B SaaS company serving mid-market and enterprise customers across manufacturing, fintech, and logistics verticals. ~200 employees, Series C, $45M ARR.",
            "Own a portfolio of 8 enterprise accounts ($4.2M combined ARR). Responsible for onboarding, adoption, expansion, and renewal. Report to VP of Customer Success.",
            "Gross revenue retention (target: 95%), net revenue retention (target: 115%), time-to-value for new deployments (target: <60 days), CSAT/NPS trends, expansion pipeline generation.",
            "Per-seat licensing with volume tiers. Enterprise plans start at $15K/year. Custom pricing for 500+ seat deployments. Annual billing with multi-year discounts (10% for 2-year, 15% for 3-year).",
            differentiators.to_string(),
            objections.to_string(),
            "Primary competitors: Contoso (enterprise, lower price but less flexible), WorkflowPro (mid-market, faster setup but limited scale), and several point solutions. We win on time-to-value and enterprise readiness. We lose when price sensitivity is the primary decision factor or when the buyer wants a narrower tool.",
            &today,
            &today,
        ],
    ).map_err(|e| format!("User entity insert: {}", e))?;

    // User context entries
    conn.execute(
        "INSERT OR REPLACE INTO user_context_entries (id, title, content, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            "mock-uctx-q1-focus",
            "Q1 Focus Areas",
            "This quarter is about closing two critical motions: Acme Phase 2 expansion (largest upsell opportunity in the portfolio) and Globex renewal (highest churn risk). Initech onboarding is going well but needs Phase 2 budget approval before we can maintain momentum. Secondary focus: building the QBR playbook into something repeatable.",
            &today,
            &today,
        ],
    ).map_err(|e| format!("User context entry 1: {}", e))?;

    conn.execute(
        "INSERT OR REPLACE INTO user_context_entries (id, title, content, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            "mock-uctx-leadership-context",
            "Leadership Team Context",
            "VP CS (my manager, Sarah) is focused on proving the CS org drives expansion revenue, not just retention. She wants QBR decks that lead with business outcomes, not feature adoption. The CRO is watching Globex closely — it's the largest renewal this quarter and a bellwether for the enterprise segment.",
            &today,
            &today,
        ],
    ).map_err(|e| format!("User context entry 2: {}", e))?;

    conn.execute(
        "INSERT OR REPLACE INTO user_context_entries (id, title, content, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            "mock-uctx-working-style",
            "Working Style Notes",
            "I prefer to front-load meeting prep the evening before. I block 7-8am for briefing review and action triage. I use the 'no surprises' rule with stakeholders — if there's bad news, I share it before the QBR, never during. I over-communicate with my AE partners and expect the same.",
            &today,
            &today,
        ],
    ).map_err(|e| format!("User context entry 3: {}", e))?;

    // =========================================================================
    // Phase 2: Meeting Prep Data (prep_frozen_json on today's meetings)
    // =========================================================================

    // Acme Weekly Sync — full prep
    let acme_prep = serde_json::json!({
        "meetingContext": "Weekly sync with Acme Corp engineering leadership. Phase 1 migration completed last week — this is the first meeting focused on Phase 2 scoping.",
        "attendees": [
            { "name": "Sarah Chen", "role": "VP Engineering", "org": "Acme Corp", "temperature": "warm", "notes": "Executive sponsor. Confirmed Phase 2 budget." },
            { "name": "Alex Torres", "role": "Tech Lead", "org": "Acme Corp", "temperature": "warm", "notes": "Departing March. Knowledge transfer priority." },
            { "name": "Mike Chen", "role": "Product Manager", "org": "DailyOS", "temperature": "neutral" }
        ],
        "sinceLast": [
            "Completed Phase 1 migration ahead of schedule — performance exceeded targets by 15%",
            "Sarah confirmed executive sponsorship for Phase 2 expansion",
            "Alex Torres confirmed March departure timeline — KT plan needed this week"
        ],
        "openItems": [
            { "title": "Send updated SOW to Acme legal team", "isOverdue": true, "context": "Legal needs the amended MSA before Phase 2 scoping can proceed." },
            { "title": "Finalize Phase 2 scope document", "isOverdue": false, "context": "Sarah Chen confirmed scope requirements last week." }
        ],
        "risks": [
            "Alex Torres departing in March — knowledge transfer gap if not addressed this week",
            "NPS trending down with 3 detractors — need to understand root causes before QBR"
        ],
        "talkingPoints": [
            "Celebrate Phase 1 success — 15% above benchmark, delivered ahead of schedule",
            "Align on Phase 2 scope: which teams beyond engineering? APAC inclusion?",
            "Alex's departure: propose KT plan with documentation sprint this week"
        ],
        "intelligenceSummary": "Acme is in a strong position post-Phase 1. The key risk is Alex Torres' departure creating a knowledge gap right as Phase 2 scoping begins. Sarah Chen is fully bought in as executive sponsor. NPS detractors need attention but aren't blocking expansion discussions.",
        "stakeholderInsights": [
            { "name": "Sarah Chen", "assessment": "Secured budget approval for Phase 2 independently — strong internal champion. Prefers data-driven updates." },
            { "name": "Alex Torres", "assessment": "Has been the technical backbone of Phase 1. His departure creates urgency around documentation and handoff." }
        ],
        "proposedAgenda": [
            { "topic": "Phase 1 retrospective", "why": "Celebrate wins, review benchmarks" },
            { "topic": "Phase 2 scope discussion", "why": "Teams in scope, APAC decision, timeline" },
            { "topic": "Knowledge transfer planning", "why": "Alex's transition, documentation needs" },
            { "topic": "NPS detractor follow-up", "why": "Plan for individual outreach" }
        ],
        "recentWins": [
            "Phase 1 migration completed ahead of schedule",
            "Performance benchmarks exceeded targets by 15%"
        ]
    });

    conn.execute(
        "INSERT OR REPLACE INTO meeting_prep (meeting_id, prep_frozen_json) VALUES (?2, ?1)",
        rusqlite::params![
            acme_prep.to_string(),
            format!("mock-mtg-acme-weekly-{}", today_str)
        ],
    )
    .map_err(|e| format!("Acme prep frozen: {}", e))?;

    // Globex QBR — full prep
    let globex_prep = serde_json::json!({
        "meetingContext": "Quarterly Business Review with Globex Industries. Highest-stakes meeting this quarter — renewal decision expected. Must address Team B usage decline and Pat Reynolds departure.",
        "attendees": [
            { "name": "Pat Reynolds", "role": "VP Product", "org": "Globex Industries", "temperature": "cool", "notes": "Departing Q2. Key exec sponsor — renewal risk." },
            { "name": "Jamie Morrison", "role": "Eng Director", "org": "Globex Industries", "temperature": "warm", "notes": "Technical champion. Enthusiastic about adoption." },
            { "name": "Casey Lee", "role": "Head of Ops", "org": "Globex Industries", "temperature": "cool", "notes": "Team B contact. Raised engagement concerns." },
            { "name": "Taylor Nguyen", "role": "Contractor", "org": "", "temperature": "neutral" }
        ],
        "sinceLast": [
            "Expanded deployment to 3 new teams this quarter — Team A usage up 40%",
            "Pat Reynolds confirmed Q2 departure — need succession plan",
            "CSAT score improved from 7.2 to 8.1 across active teams",
            "Team B usage declining 20% month-over-month — intervention needed"
        ],
        "openItems": [
            { "title": "Review Globex QBR deck with AE", "isOverdue": false, "context": "QBR is the highest-stakes meeting this quarter." },
            { "title": "Run Team B usage audit", "isOverdue": false, "context": "Need data before presenting recovery plan." }
        ],
        "risks": [
            "Pat Reynolds (executive sponsor) departing Q2 — successor unknown",
            "Team B usage down 20% MoM — could become a churn argument in renewal negotiation",
            "Competitor (Contoso) actively pitching the Globex team"
        ],
        "talkingPoints": [
            "Lead with expansion wins: 3 new teams, 40% usage growth in Team A, CSAT improvement",
            "Address Team B proactively — present root cause analysis and recovery plan",
            "Discuss Pat's transition: who becomes the executive sponsor?",
            "Renewal positioning: multi-year discount for early commitment"
        ],
        "intelligenceSummary": "Globex is a mixed picture: strong expansion momentum (3 new teams, rising CSAT) offset by Team B decline and Pat Reynolds' departure. The QBR is the pivotal moment — need to control the narrative around Team B before the competitor pitch gains traction. Jamie Morrison is the strongest internal champion and likely successor sponsor.",
        "stakeholderInsights": [
            { "name": "Pat Reynolds", "assessment": "Departing Q2 but still engaged. Will influence successor choice. Worth investing in a graceful handoff." },
            { "name": "Jamie Morrison", "assessment": "Most enthusiastic about the platform. Could be elevated to executive sponsor role if positioned correctly." },
            { "name": "Casey Lee", "assessment": "Skeptical about Team B ROI. Needs concrete data showing value before she'll advocate for renewal." }
        ],
        "proposedAgenda": [
            { "topic": "Q1 wins and adoption metrics", "why": "Lead with the positive story — 3 teams, 40% growth, CSAT gains" },
            { "topic": "Team B engagement review", "why": "Root cause analysis, recovery plan, timeline" },
            { "topic": "Leadership transition planning", "why": "Pat's departure, successor identification" },
            { "topic": "Renewal and expansion discussion", "why": "Multi-year terms, APAC expansion pilot" },
            { "topic": "Action items and next steps", "why": "Clear owners and deadlines" }
        ],
        "recentWins": [
            "Expanded to 3 new teams this quarter",
            "Team A usage up 40% since January",
            "CSAT improved from 7.2 to 8.1"
        ]
    });

    conn.execute(
        "INSERT OR REPLACE INTO meeting_prep (meeting_id, prep_frozen_json) VALUES (?2, ?1)",
        rusqlite::params![
            globex_prep.to_string(),
            format!("mock-mtg-globex-qbr-{}", today_str)
        ],
    )
    .map_err(|e| format!("Globex prep frozen: {}", e))?;

    // Initech Kickoff — full prep
    let initech_prep = serde_json::json!({
        "meetingContext": "Phase 2 kickoff planning with Initech. Phase 1 delivered on time and under budget. This meeting establishes scope, timeline, and team alignment for the next phase.",
        "attendees": [
            { "name": "Dana Patel", "role": "CTO", "org": "Initech", "temperature": "neutral", "notes": "Decision maker. Interested in Phase 2 but budget pending." },
            { "name": "Priya Sharma", "role": "VP Product", "org": "Initech", "temperature": "neutral", "notes": "Scope lead. Prefers async communication." }
        ],
        "sinceLast": [
            "Phase 1 delivered on time and under budget — strong foundation",
            "Dana expressed interest in Phase 2 expansion",
            "Budget approval pending from finance — submitted 7 days ago"
        ],
        "openItems": [
            { "title": "Schedule Phase 2 kickoff with Initech", "isOverdue": false, "context": "Phase 1 completed, need to maintain momentum." },
            { "title": "Waiting on finance approval for Initech Phase 2 budget", "isOverdue": false, "context": "Submitted 7 days ago, no response." }
        ],
        "risks": [
            "Budget approval still pending from finance — could delay Phase 2 start",
            "Team bandwidth concerns for Q2 — Priya flagged resource constraints"
        ],
        "talkingPoints": [
            "Review Phase 1 outcomes and key metrics",
            "Define Phase 2 scope: which capabilities, which teams",
            "Discuss timeline: what needs to be true for an April start?",
            "Address bandwidth concerns — what support do they need from us?"
        ],
        "intelligenceSummary": "Initech is a promising expansion opportunity. Phase 1 success gives us strong credibility. The main blocker is budget approval from finance. Priya's bandwidth concerns are real but manageable if we offer implementation support. Dana is the decision maker — focus on business case reinforcement.",
        "stakeholderInsights": [
            { "name": "Dana Patel", "assessment": "Data-driven CTO. Phase 1 ROI numbers will be the most compelling argument for Phase 2 budget approval." },
            { "name": "Priya Sharma", "assessment": "Concerned about Q2 team capacity. Will need a phased rollout plan that doesn't overload her team." }
        ],
        "proposedAgenda": [
            { "topic": "Phase 1 retrospective", "why": "Key metrics, lessons learned, team feedback" },
            { "topic": "Phase 2 scope definition", "why": "Capabilities, teams, integration requirements" },
            { "topic": "Timeline and resource planning", "why": "Budget status, team bandwidth, phased approach" },
            { "topic": "Next steps", "why": "Action items with owners" }
        ],
        "recentWins": [
            "Phase 1 delivered on time and under budget"
        ]
    });

    conn.execute(
        "INSERT OR REPLACE INTO meeting_prep (meeting_id, prep_frozen_json) VALUES (?2, ?1)",
        rusqlite::params![
            initech_prep.to_string(),
            format!("mock-mtg-initech-kickoff-{}", today_str)
        ],
    )
    .map_err(|e| format!("Initech prep frozen: {}", e))?;

    // =========================================================================
    // Phase 3: Proposed Actions + Completed Actions
    // =========================================================================

    // 3a. Suggested actions (status = 'suggested') surfaced from meeting prep
    let acme_mtg_id = format!("mock-mtg-acme-weekly-{}", today_str);
    let globex_mtg_id = format!("mock-mtg-globex-qbr-{}", today_str);
    let initech_mtg_id = format!("mock-mtg-initech-kickoff-{}", today_str);

    let proposed_actions: Vec<(&str, &str, i32, &str, &str, &str, &str)> = vec![
        // (id, title, priority, account_id, source_type, source_id, context)
        (
            "mock-act-proposed-tech-dive",
            "Schedule technical deep-dive with Acme engineering",
            crate::action_status::PRIORITY_HIGH,
            "mock-acme-corp",
            "meeting_prep",
            &acme_mtg_id,
            "Prep identified gap in technical alignment for Phase 2 scope. Alex Torres' departure makes this urgent — need to capture architectural knowledge before March.",
        ),
        (
            "mock-act-proposed-successor",
            "Identify Pat Reynolds' successor at Globex",
            crate::action_status::PRIORITY_URGENT,
            "mock-globex-industries",
            "meeting_prep",
            &globex_mtg_id,
            "Pat Reynolds departing Q2 with no identified successor as executive sponsor. Renewal risk increases significantly without aligned replacement. Jamie Morrison is a candidate.",
        ),
        (
            "mock-act-proposed-teamb-plan",
            "Draft Team B engagement recovery plan",
            crate::action_status::PRIORITY_URGENT,
            "mock-globex-industries",
            "meeting_prep",
            &globex_mtg_id,
            "Team B usage declining 20% MoM. Need a concrete recovery plan before presenting to Globex leadership at QBR. Should include root cause analysis, corrective actions, and success metrics.",
        ),
        (
            "mock-act-proposed-phase1-roi",
            "Compile Phase 1 ROI report for Initech finance",
            crate::action_status::PRIORITY_HIGH,
            "mock-initech",
            "meeting_prep",
            &initech_mtg_id,
            "Budget approval is pending from Initech finance. A compelling ROI report from Phase 1 would accelerate approval. Dana Patel (CTO) can champion it internally.",
        ),
    ];

    for (id, title, priority, account_id, source_type, source_id, context) in &proposed_actions {
        conn.execute(
            "INSERT OR REPLACE INTO actions (id, title, priority, status, created_at, account_id, source_type, source_id, context, updated_at) \
             VALUES (?1, ?2, ?3, 'backlog', ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![id, title, priority, &today, account_id, source_type, source_id, context, &today],
        ).map_err(|e| format!("Suggested action insert: {}", e))?;
    }

    // 3b. Completed actions for history
    conn.execute(
        "INSERT OR REPLACE INTO actions (id, title, priority, status, created_at, completed_at, account_id, source_type, context, updated_at) \
         VALUES (?1, ?2, ?3, 'completed', ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            "mock-act-done-phase1-report",
            "Send Phase 1 completion report to Acme",
            crate::action_status::PRIORITY_URGENT,
            days_ago(5),
            days_ago(2),
            "mock-acme-corp",
            "briefing",
            "Final Phase 1 completion report with benchmarks showing 15% above target. Sent to Sarah Chen and Pat Kim for executive review.",
            days_ago(2),
        ],
    ).map_err(|e| format!("Completed action 1: {}", e))?;

    conn.execute(
        "INSERT OR REPLACE INTO actions (id, title, priority, status, created_at, completed_at, account_id, source_type, context, updated_at) \
         VALUES (?1, ?2, ?3, 'completed', ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            "mock-act-done-globex-expansion",
            "Coordinate Team A expansion onboarding",
            crate::action_status::PRIORITY_HIGH,
            days_ago(14),
            days_ago(7),
            "mock-globex-industries",
            "briefing",
            "Successfully onboarded 3 new teams at Globex. Team A showing 40% usage growth. Coordination with Jamie Morrison on training schedule completed.",
            days_ago(7),
        ],
    ).map_err(|e| format!("Completed action 2: {}", e))?;

    // =========================================================================
    // Phase 4: Meeting Outcomes — transcript data on historical meetings
    // =========================================================================

    conn.execute(
        "INSERT INTO meeting_transcripts (meeting_id, transcript_path, transcript_processed_at) \
         VALUES ('mock-mh-acme-2d', ?1, ?2) ON CONFLICT(meeting_id) DO UPDATE SET \
         transcript_path = excluded.transcript_path, transcript_processed_at = excluded.transcript_processed_at",
        rusqlite::params!["Accounts/Acme Corp/meetings/acme-status-call-2d.md", days_ago(2)],
    ).map_err(|e| format!("Transcript mh-acme-2d: {}", e))?;

    conn.execute(
        "INSERT INTO meeting_transcripts (meeting_id, transcript_path, transcript_processed_at) \
         VALUES ('mock-mh-globex-3d', ?1, ?2) ON CONFLICT(meeting_id) DO UPDATE SET \
         transcript_path = excluded.transcript_path, transcript_processed_at = excluded.transcript_processed_at",
        rusqlite::params!["Accounts/Globex Industries/meetings/globex-checkin-3d.md", days_ago(3)],
    ).map_err(|e| format!("Transcript mh-globex-3d: {}", e))?;

    conn.execute(
        "INSERT INTO meeting_transcripts (meeting_id, transcript_path, transcript_processed_at) \
         VALUES ('mock-mh-acme-7d', ?1, ?2) ON CONFLICT(meeting_id) DO UPDATE SET \
         transcript_path = excluded.transcript_path, transcript_processed_at = excluded.transcript_processed_at",
        rusqlite::params!["Accounts/Acme Corp/meetings/acme-weekly-7d.md", days_ago(7)],
    ).map_err(|e| format!("Transcript mh-acme-7d: {}", e))?;

    // =========================================================================
    // Phase 5: Entity Context Entries
    // =========================================================================

    let context_entries: Vec<(&str, &str, &str, &str, &str)> = vec![
        // Account context
        (
            "mock-ectx-acme-renewal",
            "account",
            "mock-acme-corp",
            "Renewal Strategy",
            "Multi-year preferred. Champion (Sarah Chen) is supportive and has already secured internal budget approval for Phase 2. Legal review of amended MSA is the current blocker. Renewal date: September 2025 — plenty of runway but Phase 2 SOW needs to be signed before renewal conversation.",
        ),
        (
            "mock-ectx-acme-competitive",
            "account",
            "mock-acme-corp",
            "Competitive Landscape",
            "No active competitive threat at Acme. Strong platform lock-in after Phase 1 migration. Pat Kim (CTO) evaluated alternatives 6 months ago and chose to expand with us. APAC expansion could change this if we can't support the Singapore timezone.",
        ),
        (
            "mock-ectx-globex-risk",
            "account",
            "mock-globex-industries",
            "Team B Risk Assessment",
            "Usage declining 15% MoM across Team B. Jamie Morrison flagged resource constraints and competing priorities as root causes. Casey Lee (Head of Ops) is skeptical about ROI for Team B's use case. Need data-driven recovery plan before QBR to prevent this becoming a renewal objection.",
        ),
        (
            "mock-ectx-globex-competitor",
            "account",
            "mock-globex-industries",
            "Competitive Intel",
            "Contoso is actively pitching Globex. Pat Reynolds mentioned they had an introductory call. Jamie Morrison is not impressed with their offering but Casey Lee is evaluating options. Price is not the issue — it's Team B's perceived lack of value. Must address usage decline to neutralize the competitive angle.",
        ),
        (
            "mock-ectx-initech-growth",
            "account",
            "mock-initech",
            "Growth Potential",
            "Initech is a small account today ($350K ARR) but has strong expansion potential. Dana Patel's vision includes platform-wide adoption across all engineering teams. Phase 2 could double the ARR if scoped correctly. Key constraint: finance team is conservative and needs strong ROI data before approving expansion budget.",
        ),
        // Project context
        (
            "mock-ectx-phase2-scope",
            "project",
            "mock-acme-phase-2",
            "Scope Decision Log",
            "Agreed to include APAC in Phase 2 scope after Pat Kim's request (21 days ago). APAC will be a separate workstream targeting Q3 to avoid delaying the core Phase 2 kickoff in April. Singapore office is the pilot site. Sarah Chen confirmed engineering team is ready for expanded scope.",
        ),
        (
            "mock-ectx-teamb-analysis",
            "project",
            "mock-globex-team-b-recovery",
            "Root Cause Hypotheses",
            "Three hypotheses for Team B usage decline: (1) Competing internal tool launched in January that overlaps 30% of our feature set. (2) Team lead turnover — two of three leads changed roles in Q4. (3) Initial deployment was too broad — Team B may need a narrower, more focused configuration. Usage audit scheduled to validate.",
        ),
        // Person context
        (
            "mock-ectx-sarah-comms",
            "person",
            "mock-sarah-chen",
            "Communication Preferences",
            "Prefers async updates via Slack for routine matters. Escalates to email for decisions that need a paper trail. Likes data-heavy presentations — always include metrics. Best meeting times: Tuesday/Thursday mornings. Avoid Mondays (all-hands day at Acme).",
        ),
        (
            "mock-ectx-jamie-champion",
            "person",
            "mock-jamie-morrison",
            "Champion Development Notes",
            "Jamie is our strongest internal advocate at Globex. He proactively shares wins with his leadership team. Consider positioning him as the next executive sponsor when Pat Reynolds departs. He needs visibility into our roadmap to make the case internally. Schedule a roadmap preview before the QBR.",
        ),
        (
            "mock-ectx-dana-decisionmaking",
            "person",
            "mock-dana-patel",
            "Decision-Making Style",
            "Dana is a data-driven CTO who makes decisions based on quantitative outcomes, not relationship warmth. Phase 2 approval will hinge on a clear ROI narrative from Phase 1. She respects directness — don't oversell. She also values speed of implementation over feature completeness.",
        ),
        (
            "mock-ectx-alex-transition",
            "person",
            "mock-alex-torres",
            "Transition Notes",
            "Alex is departing in March. He's been the technical backbone of Phase 1 and holds critical institutional knowledge about Acme's deployment architecture. His replacement hasn't been named yet. Priority: get the KT documentation finalized and schedule a handoff meeting with the broader engineering team before his last day.",
        ),
        (
            "mock-ectx-pat-kim-priorities",
            "person",
            "mock-pat-kim",
            "Strategic Priorities",
            "Pat is focused on two things this quarter: (1) proving APAC viability with the Singapore pilot, and (2) reducing total cost of ownership across the engineering stack. He views our platform as a consolidation play — fewer tools, lower ops burden. Phase 2 should be positioned as cost reduction, not feature expansion.",
        ),
        (
            "mock-ectx-pat-reynolds-departure",
            "person",
            "mock-pat-reynolds",
            "Departure Context",
            "Pat is leaving for a VP role at a competitor (not Contoso). He's been our primary executive sponsor since the Globex engagement started. His departure creates a sponsorship vacuum — Jamie Morrison is the most likely successor but hasn't been formally elevated. Pat is willing to do a proper handoff if we schedule it before mid-April.",
        ),
        (
            "mock-ectx-casey-engagement",
            "person",
            "mock-casey-lee",
            "Engagement Concerns",
            "Casey has been increasingly skeptical about ROI since Team B adoption stalled. She manages the ops budget and could influence the renewal decision negatively. She responds well to concrete data — anecdotes won't move her. Need to present Team A success metrics as a counter-narrative and propose a targeted Team B recovery plan with measurable milestones.",
        ),
        (
            "mock-ectx-priya-workstyle",
            "person",
            "mock-priya-sharma",
            "Working Style",
            "Priya strongly prefers async communication — Slack threads over meetings, docs over presentations. She reads every document thoroughly before responding (expect 24-48h turnaround). She's protective of her team's bandwidth and will push back on aggressive timelines. Best approach: send a concise written proposal with clear asks, let her process it, then follow up with a focused 30-minute call.",
        ),
    ];

    for (id, entity_type, entity_id, title, content) in &context_entries {
        conn.execute(
            "INSERT OR REPLACE INTO entity_context_entries (id, entity_type, entity_id, title, content, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![id, entity_type, entity_id, title, content, &today, &today],
        ).map_err(|e| format!("Entity context entry {}: {}", id, e))?;
    }

    // =========================================================================
    // Phase 6: Account Team (roles for account detail pages)
    // =========================================================================

    let account_team_rows: Vec<(&str, &str, &str)> = vec![
        // Acme Corp team
        ("mock-acme-corp", "mock-sarah-chen", "champion"),
        ("mock-acme-corp", "mock-alex-torres", "technical_lead"),
        ("mock-acme-corp", "mock-pat-kim", "executive_sponsor"),
        // Globex Industries team
        (
            "mock-globex-industries",
            "mock-pat-reynolds",
            "executive_sponsor",
        ),
        ("mock-globex-industries", "mock-jamie-morrison", "champion"),
        (
            "mock-globex-industries",
            "mock-casey-lee",
            "operations_lead",
        ),
        // Initech team
        ("mock-initech", "mock-dana-patel", "executive_sponsor"),
        ("mock-initech", "mock-priya-sharma", "technical_lead"),
    ];

    // I652: get_person_stakeholder_roles() reads from account_stakeholder_roles above.
    // No additional mock data needed — roles are already seeded per account_team_rows.

    // I652: Seed engagement/assessment data alongside stakeholder links
    let engagement_seeds: std::collections::HashMap<(&str, &str), (&str, &str)> = [
        (
            ("mock-globex-industries", "mock-sarah-chen"),
            ("strong_advocate", "user"),
        ),
        (
            ("mock-globex-industries", "mock-alex-torres"),
            ("engaged", "ai"),
        ),
        (
            ("mock-globex-industries", "mock-casey-lee"),
            ("neutral", "ai"),
        ),
        (("mock-initech", "mock-dana-patel"), ("engaged", "user")),
        (("mock-initech", "mock-priya-sharma"), ("engaged", "ai")),
    ]
    .into_iter()
    .collect();

    let assessment_seeds: std::collections::HashMap<(&str, &str), &str> = [
        (
            ("mock-globex-industries", "mock-sarah-chen"),
            "Secured budget approval for Phase 2 independently — strong internal champion.",
        ),
        (
            ("mock-globex-industries", "mock-alex-torres"),
            "Technical backbone of Phase 1. Departure creates urgency around documentation.",
        ),
        (
            ("mock-initech", "mock-dana-patel"),
            "Data-driven CTO. Phase 1 ROI numbers will be compelling for Phase 2.",
        ),
    ]
    .into_iter()
    .collect();

    for (account_id, person_id, role) in &account_team_rows {
        let (engagement, ds_eng) = engagement_seeds
            .get(&(*account_id, *person_id))
            .copied()
            .unwrap_or(("unknown", "ai"));
        let assessment = assessment_seeds.get(&(*account_id, *person_id)).copied();
        let ds_assess = "ai";

        conn.execute(
            "INSERT INTO account_stakeholders (account_id, person_id, engagement, data_source_engagement, assessment, data_source_assessment, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(account_id, person_id) DO UPDATE SET
                engagement = COALESCE(excluded.engagement, account_stakeholders.engagement),
                data_source_engagement = excluded.data_source_engagement,
                assessment = COALESCE(excluded.assessment, account_stakeholders.assessment),
                data_source_assessment = excluded.data_source_assessment",
            rusqlite::params![account_id, person_id, engagement, ds_eng, assessment, ds_assess, &today],
        ).map_err(|e| format!("Account stakeholder insert: {}", e))?;
        conn.execute(
            "INSERT INTO account_stakeholder_roles (account_id, person_id, role, created_at) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(account_id, person_id, role) DO UPDATE SET created_at = excluded.created_at",
            rusqlite::params![account_id, person_id, role, &today],
        ).map_err(|e| format!("Account stakeholder role insert: {}", e))?;
    }

    // =========================================================================
    // Phase 6b: Stakeholder Suggestions (I652 phase 2)
    // =========================================================================
    // Test data for get_stakeholder_suggestions() 3-month filter:
    // - Recent suggestions (within 3 months) with pending status
    // - Older suggestions (4+ months ago) that should be filtered out
    // - Non-pending statuses to verify filter correctness

    let suggestions: Vec<(&str, Option<&str>, &str, &str, &str, &str, &str, &str, i64)> = vec![
        // (account_id, person_id, suggested_name, suggested_email, suggested_role, suggested_engagement, source, status, days_offset)
        // Recent pending suggestions for Acme (should appear)
        (
            "mock-acme-corp",
            None,
            "Jordan Lee",
            "jordan.lee@example.com",
            "technical_contact",
            "high",
            "email_signal",
            "pending",
            15,
        ),
        (
            "mock-acme-corp",
            None,
            "Morgan Park",
            "morgan@example.com",
            "procurement_lead",
            "medium",
            "crm_signal",
            "pending",
            20,
        ),
        // Recent pending suggestion for Globex (should appear)
        (
            "mock-globex-industries",
            None,
            "Riley Knight",
            "riley.knight@globex.com",
            "expansion_champion",
            "high",
            "usage_pattern",
            "pending",
            10,
        ),
        // Old pending suggestion for Acme (should be filtered out by 3-month check)
        (
            "mock-acme-corp",
            None,
            "Old Suggestion Person",
            "old.person@example.com",
            "operations",
            "low",
            "email_signal",
            "pending",
            120,
        ),
        // Recent non-pending suggestions (should be filtered out by status check)
        (
            "mock-acme-corp",
            None,
            "Already Added Person",
            "added@example.com",
            "stakeholder",
            "medium",
            "crm_signal",
            "accepted",
            30,
        ),
        (
            "mock-globex-industries",
            None,
            "Rejected Suggestion",
            "rejected@example.com",
            "technical_lead",
            "low",
            "email_signal",
            "rejected",
            25,
        ),
    ];

    for (
        account_id,
        person_id,
        suggested_name,
        suggested_email,
        suggested_role,
        suggested_engagement,
        source,
        status,
        days_offset,
    ) in suggestions
    {
        let created_at_str = days_ago(days_offset);

        conn.execute(
            "INSERT INTO stakeholder_suggestions
             (account_id, person_id, suggested_name, suggested_email, suggested_role, suggested_engagement, source, status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![account_id, person_id, suggested_name, suggested_email, suggested_role, suggested_engagement, source, status, &created_at_str],
        ).map_err(|e| format!("Stakeholder suggestion insert: {}", e))?;
    }

    // =========================================================================
    // Phase 7: Person Relationships (person-to-person network)
    // =========================================================================

    let person_rels: Vec<(
        &str,
        &str,
        &str,
        &str,
        &str,
        f64,
        Option<&str>,
        Option<&str>,
    )> = vec![
        // (id, from, to, relationship_type, direction, confidence, context_entity_id, context_entity_type)

        // Sarah Chen and Alex Torres are peers at Acme
        (
            "mock-prel-sarah-alex",
            "mock-sarah-chen",
            "mock-alex-torres",
            "peer",
            "symmetric",
            0.9,
            Some("mock-acme-corp"),
            Some("account"),
        ),
        // Sarah reports to Pat Kim
        (
            "mock-prel-sarah-pat",
            "mock-sarah-chen",
            "mock-pat-kim",
            "manager",
            "directed",
            0.85,
            Some("mock-acme-corp"),
            Some("account"),
        ),
        // Jamie and Casey are collaborators at Globex
        (
            "mock-prel-jamie-casey",
            "mock-jamie-morrison",
            "mock-casey-lee",
            "collaborator",
            "symmetric",
            0.8,
            Some("mock-globex-industries"),
            Some("account"),
        ),
        // Pat Reynolds is Jamie's manager at Globex
        (
            "mock-prel-pat-jamie",
            "mock-pat-reynolds",
            "mock-jamie-morrison",
            "manager",
            "directed",
            0.85,
            Some("mock-globex-industries"),
            Some("account"),
        ),
        // Dana and Priya are peers at Initech
        (
            "mock-prel-dana-priya",
            "mock-dana-patel",
            "mock-priya-sharma",
            "peer",
            "symmetric",
            0.8,
            Some("mock-initech"),
            Some("account"),
        ),
        // Mike and Lisa are internal collaborators
        (
            "mock-prel-mike-lisa",
            "mock-mike-chen",
            "mock-lisa-park",
            "collaborator",
            "symmetric",
            0.9,
            None,
            None,
        ),
        // Sarah introduced us to Pat Kim (cross-org relationship insight)
        (
            "mock-prel-sarah-intro-pat",
            "mock-sarah-chen",
            "mock-pat-kim",
            "introduced_by",
            "directed",
            0.7,
            Some("mock-acme-corp"),
            Some("account"),
        ),
        // Taylor Nguyen is a partner/ally
        (
            "mock-prel-taylor-jamie",
            "mock-taylor-nguyen",
            "mock-jamie-morrison",
            "partner",
            "symmetric",
            0.6,
            Some("mock-globex-industries"),
            Some("account"),
        ),
    ];

    for (id, from_id, to_id, rel_type, direction, confidence, ctx_entity_id, ctx_entity_type) in
        &person_rels
    {
        conn.execute(
            "INSERT OR IGNORE INTO person_relationships (id, from_person_id, to_person_id, relationship_type, direction, confidence, context_entity_id, context_entity_type, source, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'mock_data', ?9, ?10)",
            rusqlite::params![id, from_id, to_id, rel_type, direction, confidence, ctx_entity_id, ctx_entity_type, &today, &today],
        ).map_err(|e| format!("Person relationship {}: {}", id, e))?;
    }

    // =========================================================================
    // Phase 8: Today's Meeting Attendees
    // =========================================================================

    let today_attendees: Vec<(String, &str)> = vec![
        // Acme Weekly Sync
        (
            format!("mock-mtg-acme-weekly-{}", today_str),
            "mock-sarah-chen",
        ),
        (
            format!("mock-mtg-acme-weekly-{}", today_str),
            "mock-alex-torres",
        ),
        (
            format!("mock-mtg-acme-weekly-{}", today_str),
            "mock-mike-chen",
        ),
        // Initech Phase 2 Kickoff
        (
            format!("mock-mtg-initech-kickoff-{}", today_str),
            "mock-dana-patel",
        ),
        (
            format!("mock-mtg-initech-kickoff-{}", today_str),
            "mock-priya-sharma",
        ),
        // 1:1 with Sarah
        (
            format!("mock-mtg-1on1-sarah-{}", today_str),
            "mock-lisa-park",
        ),
        // Globex QBR
        (
            format!("mock-mtg-globex-qbr-{}", today_str),
            "mock-pat-reynolds",
        ),
        (
            format!("mock-mtg-globex-qbr-{}", today_str),
            "mock-jamie-morrison",
        ),
        (
            format!("mock-mtg-globex-qbr-{}", today_str),
            "mock-casey-lee",
        ),
        (
            format!("mock-mtg-globex-qbr-{}", today_str),
            "mock-taylor-nguyen",
        ),
        // Sprint Review
        (
            format!("mock-mtg-sprint-review-{}", today_str),
            "mock-mike-chen",
        ),
        (
            format!("mock-mtg-sprint-review-{}", today_str),
            "mock-lisa-park",
        ),
    ];

    for (meeting_id, person_id) in &today_attendees {
        conn.execute(
            "INSERT OR IGNORE INTO meeting_attendees (meeting_id, person_id) VALUES (?1, ?2)",
            rusqlite::params![meeting_id, person_id],
        )
        .map_err(|e| format!("Today meeting attendee: {}", e))?;
    }

    // Re-run the bulk meeting_count / last_seen update to include today's meetings
    conn.execute_batch(
        "UPDATE people SET
            meeting_count = (
                SELECT COUNT(*) FROM meeting_attendees WHERE person_id = people.id
            ),
            last_seen = (
                SELECT MAX(m.start_time) FROM meetings m
                JOIN meeting_attendees ma ON m.id = ma.meeting_id
                WHERE ma.person_id = people.id
            )
        ",
    )
    .map_err(|e| format!("People stats re-update: {}", e))?;

    // =========================================================================
    // Phase 9: Email Signals (email timeline on account/person detail pages)
    // =========================================================================

    // email_signals: email_id, sender_email, person_id, entity_id, entity_type,
    //                signal_type, signal_text, confidence, sentiment, urgency
    // 12+ rows covering: follow_up, handoff, positive_signal, risk_signal, status_update,
    //                     transition, commitment, org_change, question, competitive_mention
    let email_signals: Vec<(&str, &str, &str, &str, &str, &str, &str, f64, &str, &str)> = vec![
        (
            "mock-email-acme-1", "sarah.chen@acme.com", "mock-sarah-chen",
            "mock-acme-corp", "account",
            "follow_up", "Sarah Chen following up on platform migration timeline — engineering has concerns about cutover window",
            0.85, "neutral", "medium",
        ),
        (
            "mock-email-acme-2", "sarah.chen@acme.com", "mock-sarah-chen",
            "mock-acme-corp", "account",
            "question", "Sarah Chen asking about analytics module timeline and Q2 resource allocation changes",
            0.8, "neutral", "medium",
        ),
        (
            "mock-email-acme-3", "alex.torres@acme.com", "mock-alex-torres",
            "mock-acme-corp", "account",
            "positive_signal", "POC results show strong performance across all three test scenarios — expansion case strengthened",
            0.9, "positive", "low",
        ),
        (
            "mock-email-acme-4", "sarah.chen@acme.com", "mock-sarah-chen",
            "mock-acme-corp", "account",
            "commitment", "Sarah Chen confirms Phase 2 budget approved — will confirm exact allocation by Friday",
            0.9, "positive", "high",
        ),
        (
            "mock-email-globex-1", "jamie.morrison@globex.com", "mock-jamie-morrison",
            "mock-globex-industries", "account",
            "status_update", "Jamie Morrison revisiting contract renewal terms — needs updated terms by EOW to maintain internal momentum",
            0.85, "neutral", "medium",
        ),
        (
            "mock-email-globex-2", "lisa.park@globex.com", "mock-lisa-park",
            "mock-globex-industries", "account",
            "handoff", "Lisa Park introduced as new primary contact at Globex, taking over from Jamie Morrison starting next week",
            0.9, "neutral", "high",
        ),
        (
            "mock-email-globex-2", "lisa.park@globex.com", "mock-lisa-park",
            "mock-globex-industries", "account",
            "org_change", "Account contact transition: Lisa Park replacing Jamie Morrison as primary contact at Globex",
            0.85, "neutral", "high",
        ),
        (
            "mock-email-globex-4", "casey.lee@globex.com", "mock-casey-lee",
            "mock-globex-industries", "account",
            "risk_signal", "Casey Lee raising concerns about Team B ROI — questioning whether tool fits their workflow",
            0.85, "negative", "high",
        ),
        (
            "mock-email-globex-4", "casey.lee@globex.com", "mock-casey-lee",
            "mock-globex-industries", "account",
            "competitive_mention", "Casey Lee's Team B concerns overlap with competitive evaluation — Contoso actively pitching",
            0.75, "negative", "high",
        ),
        (
            "mock-email-globex-5", "jamie.morrison@globex.com", "mock-jamie-morrison",
            "mock-globex-industries", "account",
            "follow_up", "Jamie Morrison requesting to reschedule Thursday review to next week — wants latest numbers first",
            0.7, "neutral", "low",
        ),
        (
            "mock-email-initech-1", "dana.patel@initech.com", "mock-dana-patel",
            "mock-initech", "account",
            "positive_signal", "Dana Patel engaged after onboarding kickoff — following up proactively on access provisioning",
            0.8, "positive", "low",
        ),
        (
            "mock-email-initech-1", "dana.patel@initech.com", "mock-dana-patel",
            "mock-initech", "account",
            "transition", "Initech onboarding moving from kickoff to active provisioning phase — team ready to start",
            0.75, "positive", "medium",
        ),
    ];

    for (
        email_id,
        sender,
        person_id,
        entity_id,
        entity_type,
        sig_type,
        sig_text,
        confidence,
        sentiment,
        urgency,
    ) in &email_signals
    {
        conn.execute(
            "INSERT OR IGNORE INTO email_signals (email_id, sender_email, person_id, entity_id, entity_type, signal_type, signal_text, confidence, sentiment, urgency, detected_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![email_id, sender, person_id, entity_id, entity_type, sig_type, sig_text, confidence, sentiment, urgency, days_ago(2)],
        ).map_err(|e| format!("Email signal {}: {}", email_id, e))?;
    }

    // =========================================================================
    // Phase 10: Waiting Actions ("Waiting On" bucket on Actions page)
    // =========================================================================

    conn.execute(
        "INSERT OR REPLACE INTO actions (id, title, priority, status, created_at, due_date, account_id, waiting_on, context, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        rusqlite::params![
            "mock-act-legal-review-acme",
            "Waiting on legal review of Acme MSA amendment",
            crate::action_status::PRIORITY_URGENT,
            crate::action_status::STARTED,
            days_ago(10),
            date_only(3),
            "mock-acme-corp",
            "Legal",
            "Sarah Chen needs the amended MSA before Phase 2 scoping can proceed. Legal has had the draft for 10 days — follow up needed.",
            &today,
        ],
    ).map_err(|e| format!("Insert waiting action 1: {}", e))?;

    conn.execute(
        "INSERT OR REPLACE INTO actions (id, title, priority, status, created_at, due_date, account_id, waiting_on, context, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        rusqlite::params![
            "mock-act-finance-approval-initech",
            "Waiting on finance approval for Initech Phase 2 budget",
            crate::action_status::PRIORITY_HIGH,
            crate::action_status::STARTED,
            days_ago(7),
            date_only(7),
            "mock-initech",
            "Finance",
            "Dana Patel confirmed Phase 2 interest but budget must be approved by finance. Submitted 7 days ago — no response yet.",
            &today,
        ],
    ).map_err(|e| format!("Insert waiting action 2: {}", e))?;

    // =========================================================================
    // Phase 11: Emails (Email page with entity chips, priority buckets, summaries)
    // =========================================================================

    let ago_0 = days_ago(0);
    let ago_1 = days_ago(1);
    let ago_2 = days_ago(2);
    let ago_3 = days_ago(3);
    let ago_4 = days_ago(4);
    let ago_5 = days_ago(5);

    // Tuple: (email_id, thread_id, sender_email, sender_name, subject, snippet, priority,
    //         is_unread, received_at, entity_id, entity_type, contextual_summary, sentiment,
    //         urgency, enrichment_state, last_seen_at, relevance_score, user_is_last_sender, message_count)
    let email_rows: Vec<(&str, &str, &str, &str, &str, &str, &str, i32, &str, &str, &str, Option<&str>, Option<&str>, &str, &str, &str, Option<f64>, i32, i32)> = vec![
        // ── Acme Corp (5 emails) ──
        (
            "mock-email-acme-1", "thread-acme-migration", "sarah.chen@acme.com", "Sarah Chen",
            "Re: Platform Migration Timeline",
            "Hi — wanted to circle back on the migration timeline. Engineering has a few concerns about the…",
            "high", 1, &ago_3,
            "mock-acme-corp", "account",
            Some("Sarah Chen is following up on the platform migration timeline. Engineering has concerns about the cutover window and needs clarity before committing to the Q2 date."),
            Some("neutral"), "medium", "enriched", &ago_0,
            Some(0.92), 0, 4,
        ),
        (
            "mock-email-acme-2", "thread-acme-q2", "sarah.chen@acme.com", "Sarah Chen",
            "Re: Q2 Planning Discussion",
            "Following up on our Q2 planning call — a few open questions on the analytics module timeline and…",
            "high", 1, &ago_1,
            "mock-acme-corp", "account",
            Some("Sarah Chen following up on Q2 planning. Open questions about the analytics module timeline and how resource allocation will shift after Phase 1 wraps."),
            Some("neutral"), "medium", "enriched", &ago_0,
            Some(0.88), 0, 3,
        ),
        (
            "mock-email-acme-3", "thread-acme-poc", "alex.torres@acme.com", "Alex Torres",
            "POC Results Summary",
            "Attached the final POC results. Numbers look strong across all three test scenarios — happy to…",
            "medium", 0, &ago_2,
            "mock-acme-corp", "account",
            Some("Alex Torres shared POC results showing strong performance across all three test scenarios. Positive signal for Phase 2 expansion case."),
            Some("positive"), "low", "enriched", &ago_1,
            Some(0.35), 1, 2,
        ),
        (
            "mock-email-acme-4", "thread-acme-budget", "sarah.chen@acme.com", "Sarah Chen",
            "Budget Approval for Phase 2",
            "Great news — budget for Phase 2 has been approved. I'll confirm the exact allocation by Friday…",
            "high", 1, &ago_0,
            "mock-acme-corp", "account",
            Some("Sarah Chen confirms Phase 2 budget approval. Will confirm exact allocation by Friday. Critical milestone for expansion."),
            Some("positive"), "high", "enriched", &ago_0,
            Some(0.94), 0, 2,
        ),
        (
            "mock-email-acme-5", "thread-acme-support", "noreply@acme.com", "noreply@acme.com",
            "Your Acme Support Ticket #4521",
            "Your support ticket #4521 has been updated. A technician has been assigned and will respond within…",
            "low", 1, &ago_1,
            "", "",
            None,
            None, "low", "pending", &ago_1,
            None, 0, 1,
        ),
        // ── Globex Industries (5 emails) ──
        (
            "mock-email-globex-1", "thread-globex-renewal", "jamie.morrison@globex.com", "Jamie Morrison",
            "Re: Contract Renewal Discussion",
            "Wanted to revisit the renewal terms we discussed last week. I think we can get this done but need…",
            "high", 1, &ago_5,
            "mock-globex-industries", "account",
            Some("Jamie Morrison revisiting contract renewal terms. Believes deal is achievable but needs updated contract terms by end of week to keep internal momentum."),
            Some("neutral"), "medium", "enriched", &ago_0,
            Some(0.96), 0, 6,
        ),
        (
            "mock-email-globex-2", "thread-globex-intro", "lisa.park@globex.com", "Lisa Park",
            "Intro from Jamie — Taking Over Account",
            "Hi — Jamie Morrison introduced us. I'll be taking over as your primary contact starting next…",
            "high", 1, &ago_2,
            "mock-globex-industries", "account",
            Some("Lisa Park introduced as new primary contact at Globex, taking over from Jamie Morrison. Transition starts next week. Key handoff moment."),
            Some("neutral"), "medium", "enriched", &ago_1,
            Some(0.90), 0, 1,
        ),
        (
            "mock-email-globex-3", "thread-globex-qbr-deck", "jamie.morrison@globex.com", "Jamie Morrison",
            "QBR Deck Review",
            "Attached the draft QBR deck for your review. Let me know if the usage metrics section needs any…",
            "medium", 0, &ago_3,
            "mock-globex-industries", "account",
            Some("Jamie Morrison sharing draft QBR deck for review. Wants feedback on usage metrics section before the presentation."),
            Some("neutral"), "low", "enriched", &ago_2,
            Some(0.30), 1, 3,
        ),
        (
            "mock-email-globex-4", "thread-globex-teamb", "casey.lee@globex.com", "Casey Lee",
            "Re: Team B Engagement — Concerns",
            "I've been reviewing Team B's numbers and I'm not convinced the tool fits their workflow. We need…",
            "high", 1, &ago_0,
            "mock-globex-industries", "account",
            Some("Casey Lee is questioning Team B's ROI and whether the tool fits their workflow. This could become a churn argument during renewal."),
            Some("negative"), "high", "enriched", &ago_0,
            Some(0.94), 0, 4,
        ),
        (
            "mock-email-globex-5", "thread-globex-reschedule", "jamie.morrison@globex.com", "Jamie Morrison",
            "Can we reschedule Thursday?",
            "Something came up on my end — any chance we can push Thursday's review to next week? I want to…",
            "medium", 1, &ago_1,
            "mock-globex-industries", "account",
            Some("Jamie Morrison asking to reschedule Thursday's review to next week. Wants to ensure he has the latest numbers before meeting."),
            Some("neutral"), "low", "enriched", &ago_0,
            Some(0.45), 0, 2,
        ),
        // ── Initech (3 emails) ──
        (
            "mock-email-initech-1", "thread-initech-onboarding", "dana.patel@initech.com", "Dana Patel",
            "Onboarding Kickoff Follow-up",
            "Thanks for a great kickoff session! A few follow-up items: access requests for the three new team…",
            "medium", 0, &ago_2,
            "mock-initech", "account",
            Some("Dana Patel following up after onboarding kickoff. Three team members need access provisioned. Positive engagement signal from new account."),
            Some("positive"), "low", "enriched", &ago_1,
            Some(0.40), 1, 2,
        ),
        (
            "mock-email-initech-2", "thread-initech-access", "dana.patel@initech.com", "Dana Patel",
            "Team Access Requests",
            "Can you process the access requests I sent over last week? The team is eager to start but blocked…",
            "low", 1, &ago_4,
            "mock-initech", "account",
            Some("Dana Patel requesting access provisioning for team members. Sent last week and still pending — team blocked on onboarding."),
            Some("neutral"), "low", "enriched", &ago_3,
            Some(0.20), 0, 2,
        ),
        (
            "mock-email-initech-3", "thread-initech-digest", "admin@initech.com", "admin@initech.com",
            "Weekly Digest",
            "Here is your weekly activity digest for Initech. 14 logins, 3 reports generated, 2 new users…",
            "low", 0, &ago_1,
            "mock-initech", "account",
            None,
            None, "low", "pending", &ago_1,
            None, 0, 1,
        ),
        // ── Unlinked (2 emails) ──
        (
            "mock-email-unknown-1", "thread-unknown-recruiter", "recruiter@somecompany.com", "recruiter@somecompany.com",
            "Exciting Opportunity at TechCo",
            "Hi — I came across your profile and wanted to reach out about an exciting opportunity at TechCo…",
            "low", 1, &ago_2,
            "", "",
            None,
            None, "low", "pending", &ago_2,
            None, 0, 1,
        ),
        (
            "mock-email-unknown-2", "thread-unknown-startup", "hello@newstartup.io", "hello@newstartup.io",
            "Interested in DailyOS for our team",
            "Hi there — we're a 50-person startup and came across DailyOS. Would love to chat about how it…",
            "medium", 1, &ago_1,
            "", "",
            None,
            None, "low", "pending", &ago_1,
            None, 0, 1,
        ),
        // ── Internal (1 email) ──
        (
            "mock-email-internal-1", "thread-internal-sprint", "mike.chen@dailyos.test", "Mike Chen",
            "Sprint Review Prep",
            "Here's what I'm planning to cover in tomorrow's sprint review. Let me know if you want to add…",
            "low", 0, &ago_1,
            "", "",
            Some("Mike Chen sharing sprint review agenda for tomorrow. Standard internal coordination."),
            Some("positive"), "low", "enriched", &ago_0,
            Some(0.10), 1, 1,
        ),
    ];

    for (
        email_id,
        thread_id,
        sender_email,
        sender_name,
        subject,
        snippet,
        priority,
        is_unread,
        received_at,
        entity_id,
        entity_type,
        summary,
        sentiment,
        urgency,
        enrichment_state,
        last_seen_at,
        relevance_score,
        user_is_last_sender,
        message_count,
    ) in &email_rows
    {
        conn.execute(
            "INSERT OR REPLACE INTO emails (email_id, thread_id, sender_email, sender_name, subject, snippet, priority, is_unread, received_at, entity_id, entity_type, contextual_summary, sentiment, urgency, enrichment_state, last_seen_at, relevance_score, user_is_last_sender, message_count, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
            rusqlite::params![email_id, thread_id, sender_email, sender_name, subject, snippet, priority, is_unread, received_at,
                if entity_id.is_empty() { &None::<&str> as &dyn rusqlite::types::ToSql } else { entity_id as &dyn rusqlite::types::ToSql },
                if entity_type.is_empty() { &None::<&str> as &dyn rusqlite::types::ToSql } else { entity_type as &dyn rusqlite::types::ToSql },
                summary, sentiment, urgency, enrichment_state, last_seen_at, relevance_score, user_is_last_sender, message_count, &today, &today],
        ).map_err(|e| format!("Email {}: {}", email_id, e))?;
    }

    // ── Pinned emails ──
    conn.execute(
        &format!(
            "UPDATE emails SET pinned_at = '{}' WHERE email_id = 'mock-email-acme-4'",
            &ago_0
        ),
        [],
    )
    .map_err(|e| format!("Pin acme-4: {}", e))?;
    conn.execute(
        &format!(
            "UPDATE emails SET pinned_at = '{}' WHERE email_id = 'mock-email-globex-3'",
            &ago_2
        ),
        [],
    )
    .map_err(|e| format!("Pin globex-3: {}", e))?;

    // ── Commitments ──
    conn.execute(
        "UPDATE emails SET commitments = '[\"Will confirm Phase 2 budget by Friday\"]' WHERE email_id = 'mock-email-acme-4'",
        [],
    ).map_err(|e| format!("Commitments acme-4: {}", e))?;
    conn.execute(
        "UPDATE emails SET commitments = '[\"Will send updated contract terms by EOW\"]' WHERE email_id = 'mock-email-globex-1'",
        [],
    ).map_err(|e| format!("Commitments globex-1: {}", e))?;

    // ── Questions ──
    conn.execute(
        "UPDATE emails SET questions = '[\"What is the timeline for the analytics module?\",\"How will resource allocation change for Q2?\"]' WHERE email_id = 'mock-email-acme-2'",
        [],
    ).map_err(|e| format!("Questions acme-2: {}", e))?;
    conn.execute(
        "UPDATE emails SET questions = '[\"Can we move Thursday review to next week?\"]' WHERE email_id = 'mock-email-globex-5'",
        [],
    ).map_err(|e| format!("Questions globex-5: {}", e))?;

    // ── Entity email cadence ──
    conn.execute(
        "INSERT OR REPLACE INTO entity_email_cadence (entity_id, entity_type, period, message_count, rolling_avg, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params!["mock-acme-corp", "account", "weekly", 4, 3.5, &today],
    ).map_err(|e| format!("Cadence acme: {}", e))?;
    conn.execute(
        "INSERT OR REPLACE INTO entity_email_cadence (entity_id, entity_type, period, message_count, rolling_avg, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params!["mock-globex-industries", "account", "weekly", 0, 2.0, &today],
    ).map_err(|e| format!("Cadence globex: {}", e))?;
    conn.execute(
        "INSERT OR REPLACE INTO entity_email_cadence (entity_id, entity_type, period, message_count, rolling_avg, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params!["mock-initech", "account", "weekly", 1, 0.8, &today],
    ).map_err(|e| format!("Cadence initech: {}", e))?;

    // =========================================================================
    // Success Plans: objectives, milestones, action links, expanded lifecycle events
    // =========================================================================

    // --- Account Objectives ---
    let objective_rows: Vec<(&str, &str, &str, Option<&str>, &str, &str, Option<&str>, &str, i32)> = vec![
        // Acme: 2 active objectives
        (
            "mock-objective-acme-ttv",
            "mock-acme-corp",
            "Reduce time-to-value by 40%",
            Some("Streamline onboarding and deployment processes to cut time-to-value from 90 days to 54 days across all new team rollouts."),
            "active",
            "2026-06-15",
            None,
            "user",
            0,
        ),
        (
            "mock-objective-acme-eng-expand",
            "mock-acme-corp",
            "Expand to engineering team",
            Some("Roll out platform adoption to the 40-person engineering organization beyond the current DevOps team."),
            "active",
            "2026-09-01",
            None,
            "ai_suggested",
            1,
        ),
        // Globex: 1 active objective (overdue)
        (
            "mock-objective-globex-pipeline",
            "mock-globex-industries",
            "Stabilize deployment pipeline",
            Some("Resolve recurring CI/CD failures and bring deployment success rate above 95%."),
            "active",
            "2026-03-01",
            None,
            "user",
            0,
        ),
        // Initech: 1 completed + 1 template-sourced
        (
            "mock-objective-initech-onboarding",
            "mock-initech",
            "Complete onboarding",
            Some("Ensure all Phase 1 users are trained, credentialed, and actively using the platform."),
            "completed",
            "2026-02-28",
            Some("2026-02-15"),
            "user",
            0,
        ),
        (
            "mock-objective-initech-tech-setup",
            "mock-initech",
            "Technical setup & integration",
            Some("Establish SSO, API integrations, and data pipeline connections for production use."),
            "active",
            "2026-05-01",
            None,
            "template",
            1,
        ),
    ];

    for (
        id,
        account_id,
        title,
        description,
        status,
        target_date,
        completed_at,
        source,
        sort_order,
    ) in &objective_rows
    {
        conn.execute(
            "INSERT OR REPLACE INTO account_objectives (id, account_id, title, description, status, target_date, completed_at, source, sort_order, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![id, account_id, title, description, status, target_date, completed_at, source, sort_order, &today, &today],
        ).map_err(|e| format!("Objective {}: {}", id, e))?;
    }

    // --- Account Milestones ---
    let milestone_rows: Vec<(
        &str,
        &str,
        &str,
        &str,
        &str,
        Option<&str>,
        Option<&str>,
        Option<&str>,
        i32,
    )> = vec![
        // Acme TTV objective: 2 completed, 1 pending
        (
            "mock-milestone-acme-ttv-1",
            "mock-objective-acme-ttv",
            "mock-acme-corp",
            "Baseline measurement complete",
            "completed",
            Some("2026-02-01"),
            Some("2026-01-28"),
            None,
            0,
        ),
        (
            "mock-milestone-acme-ttv-2",
            "mock-objective-acme-ttv",
            "mock-acme-corp",
            "Automated provisioning deployed",
            "completed",
            Some("2026-03-15"),
            Some("2026-03-10"),
            Some("go_live"),
            1,
        ),
        (
            "mock-milestone-acme-ttv-3",
            "mock-objective-acme-ttv",
            "mock-acme-corp",
            "40% reduction validated with 3 new teams",
            "pending",
            Some("2026-06-15"),
            None,
            Some("onboarding_complete"),
            2,
        ),
        // Acme eng expand objective: 0 completed, 3 pending
        (
            "mock-milestone-acme-eng-1",
            "mock-objective-acme-eng-expand",
            "mock-acme-corp",
            "Engineering champion identified",
            "pending",
            Some("2026-04-15"),
            None,
            None,
            0,
        ),
        (
            "mock-milestone-acme-eng-2",
            "mock-objective-acme-eng-expand",
            "mock-acme-corp",
            "Pilot team onboarded (10 engineers)",
            "pending",
            Some("2026-06-01"),
            None,
            Some("onboarding_complete"),
            1,
        ),
        (
            "mock-milestone-acme-eng-3",
            "mock-objective-acme-eng-expand",
            "mock-acme-corp",
            "Full engineering rollout (40 engineers)",
            "pending",
            Some("2026-09-01"),
            None,
            None,
            2,
        ),
        // Globex pipeline objective: 1 completed, 2 pending (at-risk — target date passed)
        (
            "mock-milestone-globex-pipe-1",
            "mock-objective-globex-pipeline",
            "mock-globex-industries",
            "Root cause analysis documented",
            "completed",
            Some("2026-01-15"),
            Some("2026-01-20"),
            None,
            0,
        ),
        (
            "mock-milestone-globex-pipe-2",
            "mock-objective-globex-pipeline",
            "mock-globex-industries",
            "Pipeline reliability above 90%",
            "pending",
            Some("2026-02-15"),
            None,
            None,
            1,
        ),
        (
            "mock-milestone-globex-pipe-3",
            "mock-objective-globex-pipeline",
            "mock-globex-industries",
            "95% success rate sustained for 30 days",
            "pending",
            Some("2026-03-01"),
            None,
            Some("go_live"),
            2,
        ),
        // Initech onboarding objective: all 3 completed
        (
            "mock-milestone-initech-onb-1",
            "mock-objective-initech-onboarding",
            "mock-initech",
            "Admin users trained",
            "completed",
            Some("2026-01-15"),
            Some("2026-01-12"),
            None,
            0,
        ),
        (
            "mock-milestone-initech-onb-2",
            "mock-objective-initech-onboarding",
            "mock-initech",
            "All Phase 1 users credentialed",
            "completed",
            Some("2026-02-01"),
            Some("2026-01-30"),
            Some("onboarding_complete"),
            1,
        ),
        (
            "mock-milestone-initech-onb-3",
            "mock-objective-initech-onboarding",
            "mock-initech",
            "Weekly active usage above 80%",
            "completed",
            Some("2026-02-15"),
            Some("2026-02-15"),
            Some("ebr_completed"),
            2,
        ),
        // Initech tech setup objective: 1 completed (auto_detect_signal='go_live'), 2 pending
        (
            "mock-milestone-initech-tech-1",
            "mock-objective-initech-tech-setup",
            "mock-initech",
            "SSO integration live",
            "completed",
            Some("2026-02-15"),
            Some("2026-02-10"),
            Some("go_live"),
            0,
        ),
        (
            "mock-milestone-initech-tech-2",
            "mock-objective-initech-tech-setup",
            "mock-initech",
            "API integration validated",
            "pending",
            Some("2026-03-30"),
            None,
            None,
            1,
        ),
        (
            "mock-milestone-initech-tech-3",
            "mock-objective-initech-tech-setup",
            "mock-initech",
            "Data pipeline in production",
            "pending",
            Some("2026-05-01"),
            None,
            Some("go_live"),
            2,
        ),
    ];

    for (
        id,
        objective_id,
        account_id,
        title,
        status,
        target_date,
        completed_at,
        auto_detect_signal,
        sort_order,
    ) in &milestone_rows
    {
        conn.execute(
            "INSERT OR REPLACE INTO account_milestones (id, objective_id, account_id, title, status, target_date, completed_at, auto_detect_signal, sort_order, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![id, objective_id, account_id, title, status, target_date, completed_at, auto_detect_signal, sort_order, &today, &today],
        ).map_err(|e| format!("Milestone {}: {}", id, e))?;
    }

    // --- Action-Objective Links ---
    // Link existing mock actions to objectives
    let action_links: Vec<(&str, &str)> = vec![
        ("mock-act-nps-acme", "mock-objective-acme-ttv"),
        (
            "mock-act-transcript-phase2-scope",
            "mock-objective-acme-eng-expand",
        ),
        ("mock-act-qbr-deck-globex", "mock-objective-globex-pipeline"),
    ];

    for (action_id, objective_id) in &action_links {
        conn.execute(
            "INSERT OR IGNORE INTO action_objective_links (action_id, objective_id, created_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![action_id, objective_id, &today],
        ).map_err(|e| format!("Action-objective link {}/{}: {}", action_id, objective_id, e))?;
    }

    // --- Expanded lifecycle events (new event types from migration 069) ---
    let expanded_event_rows: Vec<(&str, &str, String, Option<f64>, &str)> = vec![
        ("mock-acme-corp", "go_live", "2026-01-15".to_string(), None, "Phase 1 production deployment completed successfully"),
        ("mock-acme-corp", "ebr_completed", "2026-02-28".to_string(), None, "Q1 EBR reviewed Phase 1 outcomes and Phase 2 roadmap"),
        ("mock-globex-industries", "escalation", "2026-02-01".to_string(), None, "Team B deployment failures causing customer-facing outages. VP Engineering escalated."),
        ("mock-globex-industries", "champion_change", "2026-02-20".to_string(), None, "Pat Reynolds (VP Product) confirmed Q2 departure. Jamie Morrison stepping into champion role."),
        ("mock-initech", "kickoff", "2026-01-10".to_string(), None, "Phase 1 kickoff with Dana Patel and Priya Sharma. 60-day implementation timeline agreed."),
        ("mock-initech", "go_live", "2026-02-01".to_string(), None, "Phase 1 go-live for core team. 25 users provisioned."),
    ];

    for (account_id, event_type, event_date, arr_impact, notes) in &expanded_event_rows {
        conn.execute(
            "INSERT INTO account_events (account_id, event_type, event_date, arr_impact, notes) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![account_id, event_type, event_date, arr_impact, notes],
        ).map_err(|e| format!("Expanded account event {}/{}: {}", account_id, event_type, e))?;
    }

    // ─── I555: Enriched captures with metadata ──────────────────────────────

    // Enriched captures: RED risks
    let enriched_captures: Vec<(
        &str,
        &str,
        &str,
        Option<&str>,
        &str,
        &str,
        Option<&str>,
        Option<&str>,
        Option<&str>,
    )> = vec![
        // (id, meeting_id, meeting_title, account_id, capture_type, content, sub_type, urgency, evidence_quote)
        (
            "mock-cap-enr-red-1",
            "mock-mh-acme-7d",
            "Acme Corp Weekly Sync",
            Some("mock-acme-corp"),
            "risk",
            "Champion Sarah Chen considering departure",
            None,
            Some("red"),
            Some("I'm exploring other opportunities"),
        ),
        (
            "mock-cap-enr-red-2",
            "mock-mh-globex-3d",
            "Globex Check-in",
            Some("mock-globex-industries"),
            "risk",
            "Active competitor evaluation with Salesforce",
            Some("displacement"),
            Some("red"),
            Some("We've been piloting Salesforce for the last two weeks"),
        ),
        // YELLOW risks
        (
            "mock-cap-enr-yellow-1",
            "mock-mh-acme-7d",
            "Acme Corp Weekly Sync",
            Some("mock-acme-corp"),
            "risk",
            "Declining feature adoption in Q1",
            Some("adoption_decline"),
            Some("yellow"),
            Some("Usage dropped 20% month-over-month"),
        ),
        (
            "mock-cap-enr-yellow-2",
            "mock-mh-globex-3d",
            "Globex Check-in",
            Some("mock-globex-industries"),
            "risk",
            "Budget review scheduled for next quarter",
            None,
            Some("yellow"),
            None,
        ),
        // GREEN_WATCH risk
        (
            "mock-cap-enr-green-1",
            "mock-mh-acme-2d",
            "Acme Corp Status Call",
            Some("mock-acme-corp"),
            "risk",
            "Minor integration frustration mentioned",
            None,
            Some("green_watch"),
            Some("The API latency has been a bit annoying"),
        ),
        // Sub-typed wins
        (
            "mock-cap-enr-win-adoption",
            "mock-mh-acme-2d",
            "Acme Corp Status Call",
            Some("mock-acme-corp"),
            "win",
            "Engineering team onboarded 15 new users",
            Some("ADOPTION"),
            None,
            Some("We just hit 100 active users this week"),
        ),
        (
            "mock-cap-enr-win-expansion",
            "mock-mh-globex-3d",
            "Globex Check-in",
            Some("mock-globex-industries"),
            "win",
            "Evaluating enterprise tier for APAC region",
            Some("EXPANSION"),
            None,
            Some("Singapore team wants to start a pilot next month"),
        ),
        (
            "mock-cap-enr-win-value",
            "mock-mh-acme-7d",
            "Acme Corp Weekly Sync",
            Some("mock-acme-corp"),
            "win",
            "Reported 40% reduction in deployment time",
            Some("VALUE_REALIZED"),
            None,
            Some("Our deployment cycle went from 2 hours to 72 minutes"),
        ),
        // Commitment captures (dual-write to captures table)
        (
            "mock-cap-enr-commit-1",
            "mock-mh-acme-2d",
            "Acme Corp Status Call",
            Some("mock-acme-corp"),
            "commitment",
            "Achieve 50% adoption across engineering by Q3",
            None,
            None,
            Some("We need at least half the team using it daily"),
        ),
        (
            "mock-cap-enr-commit-2",
            "mock-mh-globex-3d",
            "Globex Check-in",
            Some("mock-globex-industries"),
            "commitment",
            "Deliver integration performance report by end of month",
            None,
            None,
            None,
        ),
    ];

    for (
        id,
        meeting_id,
        meeting_title,
        account_id,
        ctype,
        content,
        sub_type,
        urgency,
        evidence_quote,
    ) in &enriched_captures
    {
        conn.execute(
            "INSERT OR REPLACE INTO captures (id, meeting_id, meeting_title, account_id, capture_type, content, sub_type, urgency, evidence_quote, captured_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![id, meeting_id, meeting_title, account_id, ctype, content, sub_type, urgency, evidence_quote, &today],
        ).map_err(|e| format!("Enriched capture {}: {}", id, e))?;
    }

    // ─── I555: Interaction dynamics ──────────────────────────────────────────

    let dynamics_rows: Vec<(&str, i32, i32, &str, &str, &str, i32)> = vec![
        // (meeting_id, customer_pct, internal_pct, question_density, decision_maker_active, forward_looking, monologue_risk)
        ("mock-mh-acme-2d", 60, 40, "high", "yes", "high", 0),
        ("mock-mh-globex-3d", 30, 70, "low", "no", "low", 1),
        ("mock-mh-acme-7d", 50, 50, "moderate", "yes", "moderate", 0),
    ];

    for (meeting_id, cust_pct, int_pct, qd, dma, fl, mono) in &dynamics_rows {
        conn.execute(
            "INSERT OR REPLACE INTO meeting_interaction_dynamics
             (meeting_id, talk_balance_customer_pct, talk_balance_internal_pct,
              speaker_sentiments_json, question_density, decision_maker_active,
              forward_looking, monologue_risk, competitor_mentions_json, escalation_language_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                meeting_id, cust_pct, int_pct,
                r#"[{"name":"Sarah Chen","sentiment":"positive","evidence":"Proactively shared roadmap feedback"},{"name":"Alex Torres","sentiment":"neutral","evidence":"Mostly listening, asked clarifying questions"}]"#,
                qd, dma, fl, mono,
                "[]",
                "[]",
            ],
        ).map_err(|e| format!("Interaction dynamics {}: {}", meeting_id, e))?;
    }

    // ─── I555: Champion health assessments ───────────────────────────────────

    let champion_rows: Vec<(&str, &str, &str, &str, Option<&str>)> = vec![
        // (meeting_id, champion_name, status, evidence, risk)
        (
            "mock-mh-acme-2d",
            "Sarah Chen",
            "strong",
            "Proactively shared roadmap feedback and secured Phase 2 budget approval",
            None,
        ),
        (
            "mock-mh-acme-7d",
            "Sarah Chen",
            "weak",
            "Delegated to junior team member, didn't attend last 20 minutes",
            Some("Champion may be losing interest; schedule 1:1 to re-engage"),
        ),
    ];

    for (meeting_id, name, status, evidence, risk) in &champion_rows {
        conn.execute(
            "INSERT OR REPLACE INTO meeting_champion_health
             (meeting_id, champion_name, champion_status, champion_evidence, champion_risk)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![meeting_id, name, status, evidence, risk],
        )
        .map_err(|e| format!("Champion health {}: {}", meeting_id, e))?;
    }

    // ─── I555: Role changes ─────────────────────────────────────────────────

    conn.execute(
        "INSERT OR REPLACE INTO meeting_role_changes
         (id, meeting_id, person_name, old_status, new_status, evidence_quote)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            "mock-role-change-1",
            "mock-mh-globex-14d",
            "Mike Johnson",
            "VP Engineering",
            "CTO",
            "Promoted last week per LinkedIn",
        ],
    )
    .map_err(|e| format!("Role change: {}", e))?;

    // ─── I555: Captured commitments ─────────────────────────────────────────

    let commitment_rows: Vec<(
        &str,
        &str,
        Option<&str>,
        &str,
        Option<&str>,
        Option<&str>,
        &str,
        i32,
    )> = vec![
        // (id, account_id, meeting_id, title, owner, target_date, source, consumed)
        (
            "mock-commitment-1",
            "mock-acme-corp",
            Some("mock-mh-acme-2d"),
            "Achieve 50% adoption across engineering by Q3",
            Some("joint"),
            Some("2026-09-30"),
            "transcript:Acme Corp Status Call",
            0,
        ),
        (
            "mock-commitment-2",
            "mock-acme-corp",
            Some("mock-mh-acme-7d"),
            "Deliver integration performance report",
            Some("us"),
            Some("2026-03-31"),
            "transcript:Acme Corp Weekly Sync",
            0,
        ),
        (
            "mock-commitment-3",
            "mock-globex-industries",
            Some("mock-mh-globex-3d"),
            "Provide API access for custom dashboards",
            Some("us"),
            None,
            "transcript:Globex Check-in",
            1,
        ),
    ];

    for (id, account_id, meeting_id, title, owner, target_date, source, consumed) in
        &commitment_rows
    {
        conn.execute(
            "INSERT OR REPLACE INTO captured_commitments
             (id, account_id, meeting_id, title, owner, target_date, confidence, source, consumed, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'medium', ?7, ?8, ?9)",
            rusqlite::params![id, account_id, meeting_id, title, owner, target_date, source, consumed, &today],
        ).map_err(|e| format!("Captured commitment {}: {}", id, e))?;
    }

    // ── I555: Seed success_plan_signals_json on mock accounts ──
    let signals_json = r#"{"statedObjectives":[{"objective":"Reduce time-to-value by 40% through platform optimization","source":"Acme Weekly Sync, Mar 5","owner":"Sarah Chen","targetDate":"2026-06-15","confidence":"high"},{"objective":"Expand platform adoption to APAC engineering teams","source":"QBR, Feb 28","owner":"Joint","targetDate":"2026-09-01","confidence":"medium"}],"mutualSuccessCriteria":[{"criterion":"Platform adoption exceeds 85% across all teams","ownedBy":"joint","status":"in_progress"},{"criterion":"Deployment velocity sustained above 3x baseline","ownedBy":"them","status":"achieved"}],"milestoneCandidates":[{"milestone":"APAC team onboarding complete","expectedBy":"2026-06-01","detectedFrom":"QBR discussion","autoDetectEvent":"onboarding_complete"},{"milestone":"Executive business review with ROI data","expectedBy":"2026-04-15","detectedFrom":"Commitment from Sarah Chen","autoDetectEvent":"ebr_completed"}]}"#;
    conn.execute(
        "UPDATE entity_assessment SET success_plan_signals_json = ?1 WHERE entity_id = 'mock-acme-corp'",
        rusqlite::params![signals_json],
    ).ok();

    // ── DOS-15: Seed health_outlook_signals_json on mock accounts ──
    // Leading signals that complement base enrichment — champion risk, usage
    // trend, channel sentiment, commercial signals, quote wall. Keeps dev-mode
    // Health & Outlook tab realistic without touching production data.
    let health_outlook_signals = r#"{"championRisk":{"championName":"Sarah Chen","atRisk":false,"riskLevel":"low","riskEvidence":["Response time steady under 6 hours","Attended last 4 QBRs"],"tenureSignal":"2.5 years in role","recentRoleChange":null,"emailSentimentTrend30d":"stable_positive","emailResponseTimeTrend":"steady","backupChampionCandidates":[{"name":"Jordan Park","role":"Director of Engineering","why":"Shadows Sarah on technical decisions","engagementLevel":"medium"}]},"productUsageTrend":{"trajectory":"growing","evidenceSummary":"Seat activation up 18% quarter-over-quarter","weeklyActiveUsersTrend":"rising","featureAdoptionHighlights":["Workflow automation now used by 3 teams","API ingestion live since Feb"]},"channelSentiment":{"divergenceDetected":false,"supportTicketTone":"cordial","meetingTone":"collaborative","emailTone":"friendly","summary":"All channels aligned — low-risk"},"transcriptExtraction":{"churnAdjacentQuestions":[],"expansionAdjacentQuestions":[{"question":"Can we extend this to our APAC engineering org?","askedBy":"Sarah Chen","meetingDate":"2026-02-28","meetingTitle":"Q1 QBR","sentiment":"positive","whyItMatters":"Live expansion signal — APAC onboarding already on the milestone list"}]},"commercialSignals":{"arrDirection":"growing","paymentBehavior":"on_time","discountStacking":"minimal","budgetStatus":"confirmed_fy27","procurementFriction":"low"},"advocacyTrack":{"referenceReady":true,"caseStudyWillingness":"expressed_interest","speakingSlotsOffered":[],"referralsMade":1},"quoteWall":[{"speaker":"Sarah Chen","quote":"The workflow automation has genuinely changed how we ship.","meetingDate":"2026-02-28","meetingTitle":"Q1 QBR","topicTags":["adoption","value"],"sentiment":"positive","publicSafeConfidence":"high"}]}"#;
    conn.execute(
        "UPDATE entity_assessment SET health_outlook_signals_json = ?1 WHERE entity_id = 'mock-acme-corp'",
        rusqlite::params![health_outlook_signals],
    ).ok();

    Ok(())
}

/// Layer intelligence-specific data on top of `seed_database()` output.
///
/// Seeds rich intelligence via `IntelligenceJson` structs (all 6 dimensions),
/// signal events, intelligence feedback, and signal weights.
fn seed_intelligence_data(db: &ActionDb) -> Result<(), String> {
    assert_dev_db_connection(db)?;

    let now = chrono::Utc::now();
    let today = now.to_rfc3339();
    let days_ago_rfc = |n: i64| -> String { (now - chrono::Duration::days(n)).to_rfc3339() };
    let date_only = |n: i64| -> String {
        (chrono::Local::now() + chrono::Duration::days(n))
            .format("%Y-%m-%d")
            .to_string()
    };

    let conn = db.conn_ref();

    // --- Decision-flagged actions ---
    conn.execute(
        "UPDATE actions SET needs_decision = 1 WHERE id IN ('mock-act-sow-acme', 'mock-act-qbr-deck-globex')",
        [],
    ).map_err(|e| format!("Flag decisions: {}", e))?;

    // --- Portfolio alerts: renewal ---
    conn.execute(
        "UPDATE accounts SET contract_end = ?1 WHERE id = 'mock-globex-industries'",
        rusqlite::params![date_only(45)],
    )
    .map_err(|e| format!("Set Globex contract_end: {}", e))?;

    // --- Portfolio alerts: stale account ---
    conn.execute(
        "UPDATE accounts SET updated_at = ?1 WHERE id = 'mock-initech'",
        rusqlite::params![days_ago_rfc(35)],
    )
    .map_err(|e| format!("Set Initech stale: {}", e))?;

    // =========================================================================
    // Rich Intelligence via IntelligenceJson structs (6 dimensions)
    // =========================================================================

    // --- Acme Corp: Strong account, Phase 1 success ---
    let acme_intel = IntelligenceJson {
        entity_id: "mock-acme-corp".into(),
        entity_type: "account".into(),
        enriched_at: today.clone(),
        source_file_count: 4,
        executive_assessment: Some(
            "Acme Corp is our strongest enterprise account. Phase 1 completed ahead of schedule \
             with 15% above-benchmark performance. Executive sponsorship for Phase 2 is secured \
             via Sarah Chen. Primary risk is Alex Torres' departure creating a knowledge gap \
             during the critical Phase 2 scoping window. NPS trending down with 3 detractors \
             in engineering requires attention before QBR.".into()
        ),
        pull_quote: Some("Acme is expanding — new department rollout signals 40% ARR growth opportunity if we land the technical win.".into()),
        risks: vec![
            IntelRisk { text: "Alex Torres departing March — critical knowledge transfer gap".into(), source: Some("meeting notes".into()), urgency: "act_now".into(), item_source: Some(ItemSource { source: "transcript".into(), confidence: 0.8, sourced_at: days_ago_rfc(5), reference: Some("meeting Mar 10".into()) }), discrepancy: None },
            IntelRisk { text: "NPS trending down: 3 detractors in engineering team".into(), source: Some("NPS survey".into()), urgency: "watch".into(), item_source: Some(ItemSource { source: "glean_crm".into(), confidence: 0.9, sourced_at: days_ago_rfc(7), reference: Some("Salesforce".into()) }), discrepancy: None },
            IntelRisk { text: "Legal review of MSA amendment stalled for 10 days".into(), source: Some("email signal".into()), urgency: "act_now".into(), item_source: Some(ItemSource { source: "user_correction".into(), confidence: 1.0, sourced_at: days_ago_rfc(2), reference: Some("you edited this".into()) }), discrepancy: None },
        ],
        recent_wins: vec![
            IntelWin { text: "Phase 1 migration completed ahead of schedule".into(), source: Some("project tracker".into()), impact: Some("High — demonstrates execution capability for Phase 2".into()), item_source: Some(ItemSource { source: "transcript".into(), confidence: 0.8, sourced_at: days_ago_rfc(14), reference: Some("meeting Mar 1".into()) }), discrepancy: None },
            IntelWin { text: "Performance benchmarks exceeded targets by 15%".into(), source: Some("analytics".into()), impact: Some("Strong ROI narrative for expansion".into()), item_source: Some(ItemSource { source: "glean_zendesk".into(), confidence: 0.85, sourced_at: days_ago_rfc(10), reference: Some("Zendesk ticket #4821".into()) }), discrepancy: None },
        ],
        current_state: Some(CurrentState {
            working: vec!["Executive sponsorship strong — Sarah Chen fully bought in".into(), "Phase 1 delivered on time and above benchmark".into(), "Platform adoption across engineering team is solid".into()],
            not_working: vec!["NPS trending down with 3 detractors".into(), "Legal review bottleneck on MSA amendment".into(), "Knowledge transfer plan not started despite Alex's March departure".into()],
            unknowns: vec!["APAC expansion viability — Singapore pilot not yet scoped".into(), "Replacement for Alex Torres not yet identified".into()],
        }),
        stakeholder_insights: vec![
            StakeholderInsight { name: "Sarah Chen".into(), role: Some("VP Engineering".into()), assessment: Some("Strong champion. Secured Phase 2 budget independently.".into()), engagement: Some("active".into()), source: None, person_id: Some("mock-sarah-chen".into()), suggested_person_id: None, item_source: Some(ItemSource { source: "glean_chat".into(), confidence: 0.7, sourced_at: days_ago_rfc(3), reference: Some("Glean AI synthesis".into()) }), discrepancy: None },
            StakeholderInsight { name: "Alex Torres".into(), role: Some("Tech Lead".into()), assessment: Some("Technical backbone of Phase 1. Departing March — urgency around KT.".into()), engagement: Some("transitioning".into()), source: None, person_id: Some("mock-alex-torres".into()), suggested_person_id: None, item_source: None, discrepancy: None },
            StakeholderInsight { name: "Pat Kim".into(), role: Some("CTO".into()), assessment: Some("Strategic decision maker. Focused on APAC and cost consolidation.".into()), engagement: Some("periodic".into()), source: None, person_id: Some("mock-pat-kim".into()), suggested_person_id: None, item_source: Some(ItemSource { source: "glean_chat".into(), confidence: 0.7, sourced_at: days_ago_rfc(5), reference: Some("Glean AI synthesis".into()) }), discrepancy: None },
        ],
        value_delivered: vec![
            ValueItem { date: Some(days_ago_rfc(90)), statement: "Phase 1 deployment drove $200K ARR expansion".into(), source: Some("contract".into()), impact: Some("High".into()), item_source: Some(ItemSource { source: "glean_crm".into(), confidence: 0.9, sourced_at: days_ago_rfc(90), reference: Some("Salesforce".into()) }), discrepancy: None },
            ValueItem { date: Some(days_ago_rfc(60)), statement: "Performance benchmarks exceeded targets by 15%".into(), source: Some("analytics".into()), impact: Some("Strong ROI narrative".into()), item_source: None, discrepancy: None },
        ],
        company_context: Some(CompanyContext {
            description: Some("Enterprise SaaS company serving mid-market and enterprise customers".into()),
            industry: Some("Enterprise SaaS".into()),
            size: Some("500-1000 employees".into()),
            headquarters: Some("San Francisco, CA".into()),
            additional_context: None,
        }),
        health: Some(AccountHealth {
            score: 78.0,
            band: "green".into(),
            source: HealthSource::Computed,
            confidence: 0.85,
            sufficient_data: true,
            trend: HealthTrend {
                direction: "stable".into(),
                rationale: Some("Strong Phase 1 execution offset by NPS concerns and Alex Torres departure".into()),
                timeframe: "90d".into(),
                confidence: 0.8,
            },
            dimensions: RelationshipDimensions {
                meeting_cadence: DimensionScore { score: 85.0, weight: 0.15, evidence: vec!["4 meetings/month, consistent cadence".into()], trend: "stable".into() },
                email_engagement: DimensionScore { score: 80.0, weight: 0.10, evidence: vec!["Same-day replies from Sarah Chen".into(), "Active thread on Phase 2 scoping".into()], trend: "stable".into() },
                stakeholder_coverage: DimensionScore { score: 70.0, weight: 0.20, evidence: vec!["3 of 4 key roles covered".into(), "Technical lead gap after Alex departs".into()], trend: "declining".into() },
                champion_health: DimensionScore { score: 90.0, weight: 0.25, evidence: vec!["Sarah Chen secured Phase 2 budget independently".into()], trend: "stable".into() },
                financial_proximity: DimensionScore { score: 75.0, weight: 0.15, evidence: vec!["Phase 2 SOW in legal review".into(), "ARR expansion on track".into()], trend: "stable".into() },
                signal_momentum: DimensionScore { score: 72.0, weight: 0.15, evidence: vec!["NPS detractors offsetting positive delivery signals".into()], trend: "declining".into() },
            },
            divergence: None,
            narrative: Some("Strong account with solid execution track record. Phase 2 expansion is on track but needs attention on NPS detractors and knowledge transfer before Alex Torres' departure.".into()),
            recommended_actions: vec!["Address NPS detractors before QBR".into(), "Accelerate Alex Torres knowledge transfer".into(), "Unblock legal review on MSA amendment".into()],
        }),
        success_metrics: Some(vec![
            SuccessMetric { name: "Time to Value".into(), target: Some("<60 days".into()), current: Some("45 days".into()), status: Some("on_track".into()), owner: None },
            SuccessMetric { name: "NPS Score".into(), target: Some("50+".into()), current: Some("42".into()), status: Some("at_risk".into()), owner: None },
            SuccessMetric { name: "Platform Adoption".into(), target: Some("80%+".into()), current: Some("85%".into()), status: Some("on_track".into()), owner: None },
        ]),
        open_commitments: Some(vec![
            OpenCommitment { description: "Finalize Phase 2 SOW with legal".into(), owner: Some("Legal / us".into()), due_date: Some(date_only(7)), source: Some("meeting".into()), status: Some("blocked".into()), item_source: None, discrepancy: None },
            OpenCommitment { description: "Complete Alex Torres knowledge transfer".into(), owner: Some("Alex Torres + team".into()), due_date: Some(date_only(14)), source: Some("meeting".into()), status: Some("not_started".into()), item_source: None, discrepancy: None },
            OpenCommitment { description: "Address NPS detractor concerns".into(), owner: Some("CS team".into()), due_date: Some(date_only(21)), source: Some("NPS survey".into()), status: Some("in_progress".into()), item_source: None, discrepancy: None },
        ]),
        relationship_depth: Some(RelationshipDepth {
            champion_strength: Some("strong".into()),
            executive_access: Some("direct".into()),
            stakeholder_coverage: Some("good".into()),
            coverage_gaps: Some(vec!["technical_lead (post-Alex)".into()]),
        }),
        // Dimension 1: Strategic Assessment
        competitive_context: vec![
            CompetitiveInsight { competitor: "Contoso Platform".into(), threat_level: Some("mentioned".into()), context: Some("Pat Kim mentioned evaluating Contoso for APAC deployment".into()), source: Some("meeting".into()), detected_at: Some(days_ago_rfc(14)), item_source: None, discrepancy: None },
        ],
        strategic_priorities: vec![
            StrategicPriority { priority: "Phase 2 Expansion".into(), status: Some("active".into()), owner: Some("Sarah Chen".into()), source: Some("meeting".into()), timeline: Some("Q2 2026".into()) },
            StrategicPriority { priority: "APAC Pilot (Singapore)".into(), status: Some("paused".into()), owner: Some("Pat Kim".into()), source: Some("meeting".into()), timeline: Some("H2 2026".into()) },
        ],
        // Dimension 2: Relationship Health
        coverage_assessment: Some(CoverageAssessment {
            role_fill_rate: Some(0.75),
            gaps: vec!["technical_lead".into()],
            covered: vec!["executive_sponsor".into(), "champion".into(), "decision_maker".into()],
            level: Some("adequate".into()),
        }),
        organizational_changes: vec![
            OrgChange { change_type: "departure".into(), person: "Alex Torres".into(), from: Some("Tech Lead".into()), to: None, detected_at: Some(days_ago_rfc(14)), source: Some("meeting".into()), item_source: None, discrepancy: None },
        ],
        internal_team: vec![
            InternalTeamMember { person_id: Some("mock-mike-chen".into()), name: "Mike Chen".into(), role: "Account Manager".into(), source: Some("user".into()) },
        ],
        // Dimension 3: Engagement Cadence
        meeting_cadence: Some(CadenceAssessment {
            meetings_per_month: Some(4.0),
            trend: Some("stable".into()),
            days_since_last: Some(2),
            assessment: Some("healthy".into()),
            evidence: vec!["Weekly sync maintained for 3 months".into()],
        }),
        email_responsiveness: Some(ResponsivenessAssessment {
            trend: Some("stable".into()),
            volume_trend: Some("stable".into()),
            assessment: Some("responsive".into()),
            evidence: vec!["Same-day replies from Sarah Chen".into(), "Active Phase 2 scoping thread".into()],
        }),
        // Dimension 4: Value & Outcomes (blockers)
        blockers: vec![],
        // Dimension 5: Commercial Context
        contract_context: Some(ContractContext {
            contract_type: Some("annual".into()),
            auto_renew: Some(true),
            contract_start: Some("2025-03-01".into()),
            renewal_date: Some("2026-03-01".into()),
            current_arr: Some(1_200_000.0),
            multi_year_remaining: None,
            previous_renewal_outcome: Some("expanded".into()),
            procurement_notes: Some("Standard PO process, 30-day legal review".into()),
            customer_fiscal_year_start: Some(1),
        }),
        expansion_signals: vec![
            ExpansionSignal { opportunity: "Phase 2 platform expansion".into(), arr_impact: Some(200_000.0), source: Some("meeting".into()), stage: Some("evaluating".into()), strength: Some("strong".into()), item_source: None, discrepancy: None },
            ExpansionSignal { opportunity: "APAC Singapore pilot".into(), arr_impact: Some(150_000.0), source: Some("meeting".into()), stage: Some("exploring".into()), strength: Some("moderate".into()), item_source: None, discrepancy: None },
        ],
        renewal_outlook: Some(RenewalOutlook {
            confidence: Some("high".into()),
            risk_factors: vec!["NPS detractors could surface in QBR".into()],
            expansion_potential: Some("Phase 2 + APAC = $350K potential".into()),
            recommended_start: Some(date_only(-30)),
            negotiation_leverage: vec!["Phase 1 exceeded benchmarks".into(), "Strong champion in Sarah Chen".into()],
            negotiation_risk: vec!["NPS trending down".into(), "Alex Torres departure creates uncertainty".into()],
        }),
        // Dimension 6: External Health Signals
        support_health: Some(SupportHealth {
            open_tickets: Some(3),
            critical_tickets: Some(0),
            avg_resolution_time: Some("4 hours".into()),
            trend: Some("improving".into()),
            csat: Some(88.0),
            source: Some("glean_zendesk".into()),
        }),
        product_adoption: Some(AdoptionSignals {
            adoption_rate: Some(0.85),
            trend: Some("growing".into()),
            feature_adoption: vec!["Core platform: 95%".into(), "Advanced analytics: 70%".into(), "API integration: 85%".into()],
            last_active: Some(days_ago_rfc(1)),
            source: Some("product_data".into()),
        }),
        nps_csat: Some(SatisfactionData {
            nps: Some(42),
            csat: Some(88.0),
            survey_date: Some(days_ago_rfc(7)),
            verbatim: Some("Great platform but onboarding for new modules could be smoother".into()),
            source: Some("survey_tool".into()),
        }),
        gong_call_summaries: vec![
            GongCallSummary { title: "Acme Technical Review".into(), date: days_ago_rfc(7), participants: vec!["John Smith".into(), "Jane Doe".into(), "Sarah Chen".into()], key_topics: "Migration timeline, API integration, Phase 2 scoping".into(), sentiment: "positive".into() },
            GongCallSummary { title: "Acme Phase 2 Planning".into(), date: days_ago_rfc(14), participants: vec!["Sarah Chen".into(), "Pat Kim".into(), "Mike Chen".into()], key_topics: "Budget approval, APAC expansion, resource allocation".into(), sentiment: "positive".into() },
            GongCallSummary { title: "Acme NPS Debrief".into(), date: days_ago_rfc(21), participants: vec!["Alex Torres".into(), "Engineering Team".into()], key_topics: "NPS detractor root cause, onboarding friction, module complexity".into(), sentiment: "neutral".into() },
        ],
        recommended_actions: vec![
            RecommendedAction { title: "Schedule executive review with Sarah Chen".into(), rationale: "Phase 2 budget is approved but scoping hasn't started. Sarah is the sponsor — get alignment before Alex departs.".into(), priority: 2, suggested_due: Some(days_ago_rfc(-3)) },
            RecommendedAction { title: "Build adoption metrics dashboard for QBR".into(), rationale: "NPS is trending down with 3 detractors. A concrete adoption dashboard gives Sarah data to address concerns internally.".into(), priority: 3, suggested_due: Some(days_ago_rfc(-7)) },
        ],
        ..Default::default()
    };

    db.upsert_entity_intelligence(&acme_intel)
        .map_err(|e| format!("Acme intelligence: {}", e))?;

    // Acme products (from AdoptionSignals feature_adoption)
    db.upsert_account_product(
        "mock-acme-corp",
        "Core platform",
        None,
        "active",
        None,
        "product_data",
        0.85,
        None,
    )
    .map_err(|e| format!("Acme product Core platform: {}", e))?;
    db.upsert_account_product(
        "mock-acme-corp",
        "Advanced analytics",
        None,
        "active",
        None,
        "product_data",
        0.70,
        None,
    )
    .map_err(|e| format!("Acme product Advanced analytics: {}", e))?;
    db.upsert_account_product(
        "mock-acme-corp",
        "API integration",
        None,
        "active",
        None,
        "product_data",
        0.85,
        None,
    )
    .map_err(|e| format!("Acme product API integration: {}", e))?;

    // --- Globex Industries: At-risk account, declining engagement ---
    let globex_intel = IntelligenceJson {
        entity_id: "mock-globex-industries".into(),
        entity_type: "account".into(),
        enriched_at: today.clone(),
        source_file_count: 5,
        executive_assessment: Some(
            "Globex presents a mixed picture. Strong expansion momentum — 3 new teams, 40% usage \
             growth in Team A, CSAT improving. However, Team B usage is declining 20% MoM and \
             Pat Reynolds (executive sponsor) is departing Q2. Competitor Contoso is actively \
             pitching. The upcoming QBR is the pivotal moment.".into()
        ),
        pull_quote: Some("Globex is at risk — champion departed, no executive sponsor identified, and renewal is 90 days out.".into()),
        risks: vec![
            IntelRisk { text: "Pat Reynolds (executive sponsor) departing Q2 — successor unknown".into(), source: Some("direct communication".into()), urgency: "act_now".into(), item_source: Some(ItemSource { source: "transcript".into(), confidence: 0.8, sourced_at: days_ago_rfc(10), reference: Some("meeting Mar 5".into()) }), discrepancy: Some(true) },
            IntelRisk { text: "Team B usage declining 20% month-over-month".into(), source: Some("usage analytics".into()), urgency: "act_now".into(), item_source: Some(ItemSource { source: "glean_crm".into(), confidence: 0.9, sourced_at: days_ago_rfc(3), reference: Some("Salesforce".into()) }), discrepancy: None },
            IntelRisk { text: "Contoso actively pitching to Globex leadership".into(), source: Some("email intel from Jamie Morrison".into()), urgency: "watch".into(), item_source: Some(ItemSource { source: "user_correction".into(), confidence: 1.0, sourced_at: days_ago_rfc(1), reference: Some("you edited this".into()) }), discrepancy: None },
        ],
        recent_wins: vec![
            IntelWin { text: "Expanded to 3 new teams this quarter".into(), source: Some("deployment tracker".into()), impact: Some("Demonstrates platform value at scale".into()), item_source: Some(ItemSource { source: "transcript".into(), confidence: 0.8, sourced_at: days_ago_rfc(7), reference: Some("QBR prep call".into()) }), discrepancy: None },
            IntelWin { text: "Team A usage up 40% since January".into(), source: Some("usage analytics".into()), impact: Some("Strong adoption proof point".into()), item_source: Some(ItemSource { source: "glean_zendesk".into(), confidence: 0.85, sourced_at: days_ago_rfc(5), reference: Some("Zendesk ticket #7032".into()) }), discrepancy: None },
            IntelWin { text: "CSAT improved from 7.2 to 8.1".into(), source: Some("survey results".into()), impact: Some("Customer satisfaction trending positive".into()), item_source: None, discrepancy: None },
        ],
        current_state: Some(CurrentState {
            working: vec!["Team A adoption excellent — 40% growth".into(), "CSAT improving across active teams".into(), "Jamie Morrison is a strong internal champion".into()],
            not_working: vec!["Team B engagement declining 20% MoM".into(), "Executive sponsor departing with no named successor".into(), "Competitive threat from Contoso gaining traction".into()],
            unknowns: vec!["Root cause of Team B decline".into(), "Who will replace Pat Reynolds".into(), "Impact of Contoso pitch on renewal decision".into()],
        }),
        stakeholder_insights: vec![
            StakeholderInsight { name: "Pat Reynolds".into(), role: Some("VP Product".into()), assessment: Some("Departing Q2 but still engaged. Will influence successor choice.".into()), engagement: Some("transitioning".into()), source: None, person_id: Some("mock-pat-reynolds".into()), suggested_person_id: None, item_source: Some(ItemSource { source: "glean_chat".into(), confidence: 0.7, sourced_at: days_ago_rfc(10), reference: Some("Glean AI synthesis".into()) }), discrepancy: None },
            StakeholderInsight { name: "Jamie Morrison".into(), role: Some("Eng Director".into()), assessment: Some("Strongest champion. Could be elevated to executive sponsor.".into()), engagement: Some("active".into()), source: None, person_id: Some("mock-jamie-morrison".into()), suggested_person_id: None, item_source: None, discrepancy: None },
            StakeholderInsight { name: "Casey Lee".into(), role: Some("Head of Ops".into()), assessment: Some("Skeptical about Team B ROI. Evaluating Contoso.".into()), engagement: Some("at_risk".into()), source: None, person_id: Some("mock-casey-lee".into()), suggested_person_id: None, item_source: Some(ItemSource { source: "glean_chat".into(), confidence: 0.7, sourced_at: days_ago_rfc(4), reference: Some("Glean AI synthesis".into()) }), discrepancy: None },
        ],
        value_delivered: vec![
            ValueItem { date: Some(days_ago_rfc(30)), statement: "3 new team deployments in Q1".into(), source: Some("deployment tracker".into()), impact: Some("Scale validation".into()), item_source: None, discrepancy: None },
            ValueItem { date: Some(days_ago_rfc(60)), statement: "CSAT improvement: 7.2 → 8.1".into(), source: Some("survey".into()), impact: Some("Positive trend".into()), item_source: None, discrepancy: None },
        ],
        company_context: Some(CompanyContext {
            description: Some("Manufacturing technology company with global operations".into()),
            industry: Some("Manufacturing Technology".into()),
            size: Some("1000-5000 employees".into()),
            headquarters: Some("Chicago, IL".into()),
            additional_context: None,
        }),
        health: Some(AccountHealth {
            score: 42.0,
            band: "red".into(),
            source: HealthSource::Computed,
            confidence: 0.78,
            sufficient_data: true,
            trend: HealthTrend {
                direction: "declining".into(),
                rationale: Some("Team B decline and executive sponsor departure offsetting expansion wins".into()),
                timeframe: "90d".into(),
                confidence: 0.75,
            },
            dimensions: RelationshipDimensions {
                meeting_cadence: DimensionScore { score: 60.0, weight: 0.15, evidence: vec!["2 meetings/month, declining from 3".into()], trend: "declining".into() },
                email_engagement: DimensionScore { score: 45.0, weight: 0.10, evidence: vec!["Casey Lee response times increasing".into()], trend: "declining".into() },
                stakeholder_coverage: DimensionScore { score: 40.0, weight: 0.20, evidence: vec!["Exec sponsor departing".into(), "No successor identified".into()], trend: "declining".into() },
                champion_health: DimensionScore { score: 65.0, weight: 0.25, evidence: vec!["Jamie strong but can't compensate for Casey's skepticism".into()], trend: "stable".into() },
                financial_proximity: DimensionScore { score: 35.0, weight: 0.15, evidence: vec!["Renewal in 45 days, no commitment signal".into()], trend: "declining".into() },
                signal_momentum: DimensionScore { score: 30.0, weight: 0.15, evidence: vec!["Competitive threat + usage decline = negative momentum".into()], trend: "declining".into() },
            },
            divergence: None,
            narrative: Some("At-risk account requiring immediate intervention. Team B usage decline and executive sponsor departure create a dangerous combination heading into renewal. The QBR is the last opportunity to control the narrative.".into()),
            recommended_actions: vec!["Prepare Team B recovery plan for QBR".into(), "Identify and cultivate Pat Reynolds' successor".into(), "Counter Contoso narrative with adoption data".into()],
        }),
        success_metrics: Some(vec![
            SuccessMetric { name: "Adoption Rate".into(), target: Some("80%".into()), current: Some("72%".into()), status: Some("at_risk".into()), owner: None },
            SuccessMetric { name: "Team B Usage".into(), target: Some("stable".into()), current: Some("-20% MoM".into()), status: Some("critical".into()), owner: None },
        ]),
        open_commitments: Some(vec![
            OpenCommitment { description: "Address Team B usage decline before QBR".into(), owner: Some("CS + Product".into()), due_date: Some(date_only(5)), source: Some("meeting".into()), status: Some("in_progress".into()), item_source: None, discrepancy: None },
            OpenCommitment { description: "Secure renewal commitment".into(), owner: Some("Account team".into()), due_date: Some(date_only(45)), source: Some("renewal".into()), status: Some("not_started".into()), item_source: None, discrepancy: None },
            OpenCommitment { description: "Identify Pat Reynolds' successor".into(), owner: Some("Account team".into()), due_date: Some(date_only(30)), source: Some("meeting".into()), status: Some("not_started".into()), item_source: None, discrepancy: None },
        ]),
        relationship_depth: Some(RelationshipDepth {
            champion_strength: Some("moderate".into()),
            executive_access: Some("transitioning".into()),
            stakeholder_coverage: Some("thin".into()),
            coverage_gaps: Some(vec!["executive_sponsor".into(), "ops_decision_maker".into()]),
        }),
        competitive_context: vec![
            CompetitiveInsight { competitor: "Contoso Platform".into(), threat_level: Some("evaluation".into()), context: Some("Casey Lee actively evaluating Contoso for Team B replacement".into()), source: Some("email".into()), detected_at: Some(days_ago_rfc(7)), item_source: None, discrepancy: None },
        ],
        strategic_priorities: vec![
            StrategicPriority { priority: "Renewal Commitment".into(), status: Some("at_risk".into()), owner: Some("Account team".into()), source: Some("renewal cycle".into()), timeline: Some(date_only(45)) },
            StrategicPriority { priority: "Team B Recovery".into(), status: Some("active".into()), owner: Some("CS team".into()), source: Some("usage data".into()), timeline: Some("Before QBR".into()) },
        ],
        coverage_assessment: Some(CoverageAssessment {
            role_fill_rate: Some(0.5),
            gaps: vec!["executive_sponsor".into(), "ops_decision_maker".into()],
            covered: vec!["champion".into(), "technical_contact".into()],
            level: Some("thin".into()),
        }),
        organizational_changes: vec![
            OrgChange { change_type: "departure".into(), person: "Pat Reynolds".into(), from: Some("VP Product".into()), to: None, detected_at: Some(days_ago_rfc(10)), source: Some("direct".into()), item_source: None, discrepancy: None },
        ],
        internal_team: vec![
            InternalTeamMember { person_id: Some("mock-mike-chen".into()), name: "Mike Chen".into(), role: "Account Manager".into(), source: Some("user".into()) },
            InternalTeamMember { person_id: Some("mock-taylor-nguyen".into()), name: "Taylor Nguyen".into(), role: "Solutions Architect".into(), source: Some("user".into()) },
        ],
        meeting_cadence: Some(CadenceAssessment {
            meetings_per_month: Some(2.0),
            trend: Some("declining".into()),
            days_since_last: Some(3),
            assessment: Some("sparse".into()),
            evidence: vec!["Down from 3/month to 2/month".into(), "Casey Lee skipped last two syncs".into()],
        }),
        email_responsiveness: Some(ResponsivenessAssessment {
            trend: Some("slowing".into()),
            volume_trend: Some("decreasing".into()),
            assessment: Some("slow".into()),
            evidence: vec!["Casey Lee 3-day average reply time".into(), "Pat Reynolds responses more terse".into()],
        }),
        blockers: vec![
            Blocker { description: "Team B root cause analysis not started".into(), owner: Some("CS team".into()), since: Some(days_ago_rfc(14)), impact: Some("high".into()), source: Some("meeting".into()) },
        ],
        contract_context: Some(ContractContext {
            contract_type: Some("annual".into()),
            auto_renew: Some(false),
            contract_start: Some("2025-01-15".into()),
            renewal_date: Some(date_only(45)),
            current_arr: Some(850_000.0),
            multi_year_remaining: None,
            previous_renewal_outcome: Some("flat".into()),
            procurement_notes: Some("Requires VP sign-off and 45-day notice period".into()),
            customer_fiscal_year_start: Some(1),
        }),
        expansion_signals: vec![
            ExpansionSignal { opportunity: "3 new team deployments".into(), arr_impact: Some(120_000.0), source: Some("deployment tracker".into()), stage: Some("committed".into()), strength: Some("strong".into()), item_source: None, discrepancy: None },
        ],
        renewal_outlook: Some(RenewalOutlook {
            confidence: Some("low".into()),
            risk_factors: vec!["Executive sponsor departing".into(), "Team B decline".into(), "Active competitive evaluation".into()],
            expansion_potential: Some("New team deployments could offset Team B if stabilized".into()),
            recommended_start: Some(days_ago_rfc(0)),
            negotiation_leverage: vec!["Team A 40% growth".into(), "3 new team deployments".into(), "CSAT improvement".into()],
            negotiation_risk: vec!["Team B narrative".into(), "Contoso alternative".into(), "Sponsor departure".into()],
        }),
        support_health: Some(SupportHealth {
            open_tickets: Some(7),
            critical_tickets: Some(2),
            avg_resolution_time: Some("18 hours".into()),
            trend: Some("degrading".into()),
            csat: Some(72.0),
            source: Some("glean_zendesk".into()),
        }),
        product_adoption: Some(AdoptionSignals {
            adoption_rate: Some(0.72),
            trend: Some("declining".into()),
            feature_adoption: vec!["Core: 80%".into(), "Analytics: 45%".into(), "Team B: 30% and falling".into()],
            last_active: Some(days_ago_rfc(1)),
            source: Some("product_data".into()),
        }),
        nps_csat: Some(SatisfactionData {
            nps: Some(28),
            csat: Some(72.0),
            survey_date: Some(days_ago_rfc(14)),
            verbatim: Some("Team A loves it but Team B feels unsupported".into()),
            source: Some("survey_tool".into()),
        }),
        recommended_actions: vec![
            RecommendedAction { title: "Schedule 1:1 with Jamie Morrison".into(), rationale: "Jamie is the strongest remaining relationship at Globex. With Pat Reynolds departing, Jamie's buy-in is critical for renewal.".into(), priority: 1, suggested_due: Some(days_ago_rfc(-2)) },
            RecommendedAction { title: "Prepare renewal brief with Team A success data".into(), rationale: "Team B usage decline is dominating the narrative. A brief showing Team A's strong adoption counters the Contoso comparison.".into(), priority: 2, suggested_due: Some(days_ago_rfc(-5)) },
        ],
        ..Default::default()
    };

    db.upsert_entity_intelligence(&globex_intel)
        .map_err(|e| format!("Globex intelligence: {}", e))?;

    // Globex products (from AdoptionSignals feature_adoption)
    db.upsert_account_product(
        "mock-globex-industries",
        "Core",
        None,
        "active",
        None,
        "product_data",
        0.72,
        None,
    )
    .map_err(|e| format!("Globex product Core: {}", e))?;
    db.upsert_account_product(
        "mock-globex-industries",
        "Analytics",
        None,
        "active",
        None,
        "product_data",
        0.45,
        None,
    )
    .map_err(|e| format!("Globex product Analytics: {}", e))?;
    db.upsert_account_product(
        "mock-globex-industries",
        "Team collaboration",
        None,
        "active",
        None,
        "product_data",
        0.30,
        None,
    )
    .map_err(|e| format!("Globex product Team collaboration: {}", e))?;

    // --- Initech: Onboarding account, sparse but clean ---
    let initech_intel = IntelligenceJson {
        entity_id: "mock-initech".into(),
        entity_type: "account".into(),
        enriched_at: today.clone(),
        source_file_count: 2,
        executive_assessment: Some(
            "Initech is a promising expansion opportunity built on a solid Phase 1 foundation. \
             Delivered on time and under budget, providing strong ROI data for the Phase 2 \
             business case. The primary blocker is budget approval from a conservative finance \
             team.".into()
        ),
        pull_quote: Some("Initech is stable but autopilot — usage is flat, engagement is minimal, and we have no expansion signals.".into()),
        risks: vec![
            IntelRisk { text: "Phase 2 budget approval pending from finance — 7 days with no response".into(), source: Some("email from Dana Patel".into()), urgency: "watch".into(), item_source: Some(ItemSource { source: "transcript".into(), confidence: 0.8, sourced_at: days_ago_rfc(7), reference: Some("meeting Mar 8".into()) }), discrepancy: None },
            IntelRisk { text: "Team bandwidth constraints for Q2 — Priya Sharma flagged".into(), source: Some("meeting notes".into()), urgency: "watch".into(), item_source: Some(ItemSource { source: "glean_crm".into(), confidence: 0.9, sourced_at: days_ago_rfc(5), reference: Some("Salesforce".into()) }), discrepancy: None },
        ],
        recent_wins: vec![
            IntelWin { text: "Phase 1 delivered on time and under budget".into(), source: Some("project tracker".into()), impact: Some("Strong proof point for Phase 2 business case".into()), item_source: Some(ItemSource { source: "transcript".into(), confidence: 0.8, sourced_at: days_ago_rfc(10), reference: Some("kickoff meeting".into()) }), discrepancy: None },
        ],
        current_state: Some(CurrentState {
            working: vec!["Phase 1 execution was flawless — strong credibility".into(), "Dana Patel is championing Phase 2 internally".into(), "Technical integration is stable and performant".into()],
            not_working: vec!["Finance team slow to approve expansion budget".into(), "Team bandwidth concerns for Q2".into()],
            unknowns: vec!["When finance will approve Phase 2 budget".into(), "Exact scope of Phase 2".into()],
        }),
        stakeholder_insights: vec![
            StakeholderInsight { name: "Dana Patel".into(), role: Some("CTO".into()), assessment: Some("Data-driven decision maker. Phase 1 ROI is the key argument.".into()), engagement: Some("active".into()), source: None, person_id: Some("mock-dana-patel".into()), suggested_person_id: None, item_source: Some(ItemSource { source: "glean_chat".into(), confidence: 0.7, sourced_at: days_ago_rfc(6), reference: Some("Glean AI synthesis".into()) }), discrepancy: None },
            StakeholderInsight { name: "Priya Sharma".into(), role: Some("VP Product".into()), assessment: Some("Concerned about Q2 capacity. Needs phased rollout plan.".into()), engagement: Some("active".into()), source: None, person_id: Some("mock-priya-sharma".into()), suggested_person_id: None, item_source: None, discrepancy: None },
        ],
        value_delivered: vec![
            ValueItem { date: Some(days_ago_rfc(10)), statement: "Phase 1 delivered on time and under budget".into(), source: Some("project tracker".into()), impact: Some("Strong ROI proof".into()), item_source: None, discrepancy: None },
        ],
        company_context: Some(CompanyContext {
            description: Some("Financial technology company focused on enterprise automation".into()),
            industry: Some("Financial Technology".into()),
            size: Some("200-500 employees".into()),
            headquarters: Some("Boston, MA".into()),
            additional_context: None,
        }),
        health: Some(AccountHealth {
            score: 55.0,
            band: "yellow".into(),
            source: HealthSource::Computed,
            confidence: 0.65,
            sufficient_data: true,
            trend: HealthTrend {
                direction: "stable".into(),
                rationale: Some("Phase 1 success builds credibility but limited engagement history".into()),
                timeframe: "60d".into(),
                confidence: 0.6,
            },
            dimensions: RelationshipDimensions {
                meeting_cadence: DimensionScore { score: 50.0, weight: 0.15, evidence: vec!["1 meeting/month, new relationship".into()], trend: "stable".into() },
                email_engagement: DimensionScore { score: 55.0, weight: 0.10, evidence: vec!["Responsive but infrequent".into()], trend: "stable".into() },
                stakeholder_coverage: DimensionScore { score: 60.0, weight: 0.20, evidence: vec!["2 key contacts identified".into()], trend: "stable".into() },
                champion_health: DimensionScore { score: 65.0, weight: 0.25, evidence: vec!["Dana Patel supportive but constrained by finance".into()], trend: "stable".into() },
                financial_proximity: DimensionScore { score: 40.0, weight: 0.15, evidence: vec!["Budget approval pending".into()], trend: "stable".into() },
                signal_momentum: DimensionScore { score: 50.0, weight: 0.15, evidence: vec!["Limited signal history, Phase 1 success is primary data point".into()], trend: "stable".into() },
            },
            divergence: None,
            narrative: Some("Early-stage account with strong Phase 1 foundation. Limited engagement history makes scoring uncertain. Budget approval is the key gate.".into()),
            recommended_actions: vec!["Follow up on budget approval".into(), "Prepare phased rollout plan for Priya".into()],
        }),
        success_metrics: Some(vec![
            SuccessMetric { name: "Time to Value".into(), target: Some("<60 days".into()), current: Some("52 days".into()), status: Some("on_track".into()), owner: None },
            SuccessMetric { name: "Phase 1 Completion".into(), target: Some("100%".into()), current: Some("100%".into()), status: Some("on_track".into()), owner: None },
        ]),
        open_commitments: Some(vec![
            OpenCommitment { description: "Get Phase 2 budget approved".into(), owner: Some("Dana Patel / Finance".into()), due_date: Some(date_only(14)), source: Some("meeting".into()), status: Some("blocked".into()), item_source: None, discrepancy: None },
            OpenCommitment { description: "Schedule Phase 2 kickoff".into(), owner: Some("Account team".into()), due_date: None, source: Some("meeting".into()), status: Some("waiting".into()), item_source: None, discrepancy: None },
        ]),
        relationship_depth: Some(RelationshipDepth {
            champion_strength: Some("developing".into()),
            executive_access: Some("through champion".into()),
            stakeholder_coverage: Some("adequate".into()),
            coverage_gaps: Some(vec!["finance_contact".into()]),
        }),
        competitive_context: vec![],
        strategic_priorities: vec![
            StrategicPriority { priority: "Phase 2 Expansion".into(), status: Some("paused".into()), owner: Some("Dana Patel".into()), source: Some("meeting".into()), timeline: Some("Pending budget".into()) },
        ],
        coverage_assessment: Some(CoverageAssessment {
            role_fill_rate: Some(0.5),
            gaps: vec!["finance_contact".into(), "technical_lead".into()],
            covered: vec!["champion".into(), "product_owner".into()],
            level: Some("adequate".into()),
        }),
        organizational_changes: vec![],
        internal_team: vec![],
        meeting_cadence: Some(CadenceAssessment {
            meetings_per_month: Some(1.0),
            trend: Some("stable".into()),
            days_since_last: Some(10),
            assessment: Some("sparse".into()),
            evidence: vec!["Monthly check-ins only".into()],
        }),
        email_responsiveness: Some(ResponsivenessAssessment {
            trend: Some("stable".into()),
            volume_trend: Some("stable".into()),
            assessment: Some("normal".into()),
            evidence: vec!["1-2 day reply time from Dana".into()],
        }),
        blockers: vec![
            Blocker { description: "Budget approval from finance".into(), owner: Some("Finance team".into()), since: Some(days_ago_rfc(7)), impact: Some("high".into()), source: Some("email".into()) },
        ],
        contract_context: Some(ContractContext {
            contract_type: Some("annual".into()),
            auto_renew: Some(true),
            contract_start: Some("2025-10-01".into()),
            renewal_date: Some("2026-10-01".into()),
            current_arr: Some(350_000.0),
            multi_year_remaining: None,
            previous_renewal_outcome: Some("first_term".into()),
            procurement_notes: None,
            customer_fiscal_year_start: Some(10),
        }),
        expansion_signals: vec![
            ExpansionSignal { opportunity: "Phase 2 platform expansion".into(), arr_impact: Some(150_000.0), source: Some("meeting".into()), stage: Some("exploring".into()), strength: Some("early".into()), item_source: None, discrepancy: None },
        ],
        renewal_outlook: Some(RenewalOutlook {
            confidence: Some("moderate".into()),
            risk_factors: vec!["Budget approval uncertainty".into()],
            expansion_potential: Some("Phase 2 adds $150K if approved".into()),
            recommended_start: None,
            negotiation_leverage: vec!["Phase 1 on time and under budget".into()],
            negotiation_risk: vec!["Conservative finance team".into()],
        }),
        support_health: Some(SupportHealth {
            open_tickets: Some(1),
            critical_tickets: Some(0),
            avg_resolution_time: Some("2 hours".into()),
            trend: Some("stable".into()),
            csat: None,
            source: None,
        }),
        product_adoption: Some(AdoptionSignals {
            adoption_rate: Some(0.65),
            trend: Some("growing".into()),
            feature_adoption: vec!["Core: 90%".into(), "Analytics: 40%".into()],
            last_active: Some(days_ago_rfc(2)),
            source: Some("product_data".into()),
        }),
        nps_csat: None,
        dismissed_items: vec![
            DismissedItem { field: "risks".into(), content: "outdated risk about budget cuts".into(), dismissed_at: days_ago_rfc(3) },
        ],
        ..Default::default()
    };

    db.upsert_entity_intelligence(&initech_intel)
        .map_err(|e| format!("Initech intelligence: {}", e))?;

    // Initech products (from AdoptionSignals feature_adoption: "Core: 90%", "Analytics: 40%")
    db.upsert_account_product(
        "mock-initech",
        "Core",
        None,
        "active",
        None,
        "product_data",
        0.65,
        None,
    )
    .map_err(|e| format!("Initech product Core: {}", e))?;
    db.upsert_account_product(
        "mock-initech",
        "Analytics",
        None,
        "active",
        None,
        "product_data",
        0.40,
        None,
    )
    .map_err(|e| format!("Initech product Analytics: {}", e))?;

    // --- Person intelligence: Sarah Chen ---
    let sarah_intel = IntelligenceJson {
        entity_id: "mock-sarah-chen".into(),
        entity_type: "person".into(),
        enriched_at: today.clone(),
        source_file_count: 3,
        executive_assessment: Some(
            "Sarah Chen is our strongest executive sponsor across the portfolio. She independently \
             secured budget approval for Phase 2 and proactively advocates for our platform within \
             Acme's leadership team.".into()
        ),
        risks: vec![
            IntelRisk { text: "May face internal pressure if NPS detractors aren't addressed".into(), source: Some("inferred from NPS trend".into()), urgency: "watch".into(), item_source: None, discrepancy: None },
        ],
        recent_wins: vec![
            IntelWin { text: "Secured Phase 2 budget approval independently".into(), source: Some("meeting notes".into()), impact: Some("Removed the biggest Phase 2 blocker".into()), item_source: None, discrepancy: None },
            IntelWin { text: "Confirmed executive sponsorship for expansion".into(), source: Some("direct communication".into()), impact: Some("Strategic alignment at VP level".into()), item_source: None, discrepancy: None },
        ],
        current_state: Some(CurrentState {
            working: vec!["Strong internal advocacy".into(), "Proactive communication on status and blockers".into(), "Budget approval secured for Phase 2".into()],
            not_working: vec!["NPS detractors in her engineering team need her attention".into()],
            unknowns: vec!["Her stance on APAC expansion timeline".into()],
        }),
        stakeholder_insights: vec![
            StakeholderInsight { name: "Sarah Chen".into(), role: Some("VP Engineering".into()), assessment: Some("Strongest champion in the portfolio. Data-driven, decisive.".into()), engagement: Some("active".into()), source: None, person_id: Some("mock-sarah-chen".into()), suggested_person_id: None, item_source: None, discrepancy: None },
        ],
        relationship_depth: Some(RelationshipDepth {
            champion_strength: Some("strong".into()),
            executive_access: Some("direct".into()),
            stakeholder_coverage: None,
            coverage_gaps: None,
        }),
        network: Some(NetworkIntelligence {
            health: "strong".into(),
            key_relationships: vec![
                NetworkKeyRelationship { person_id: "mock-alex-torres".into(), name: "Alex Torres".into(), relationship_type: "works_with".into(), confidence: 0.95, signal_summary: Some("Weekly sync, shared projects".into()) },
                NetworkKeyRelationship { person_id: "mock-pat-kim".into(), name: "Pat Kim".into(), relationship_type: "reports_to".into(), confidence: 0.85, signal_summary: Some("CTO oversight".into()) },
            ],
            risks: vec!["Alex Torres departure may reduce her effectiveness".into()],
            opportunities: vec!["Could introduce us to APAC leadership".into()],
            influence_radius: 3,
            cluster_summary: Some("Core Acme executive cluster".into()),
        }),
        ..Default::default()
    };

    db.upsert_entity_intelligence(&sarah_intel)
        .map_err(|e| format!("Sarah Chen intelligence: {}", e))?;

    // --- Person intelligence: Jamie Morrison ---
    let jamie_intel = IntelligenceJson {
        entity_id: "mock-jamie-morrison".into(),
        entity_type: "person".into(),
        enriched_at: today.clone(),
        source_file_count: 2,
        executive_assessment: Some(
            "Jamie Morrison is our most enthusiastic champion at Globex. Natural successor to Pat \
             Reynolds as executive sponsor. Drives adoption in Team A and proactively advocates."
                .into(),
        ),
        risks: vec![IntelRisk {
            text: "May lose influence if Team B decline isn't addressed — it's in his org".into(),
            source: Some("inferred".into()),
            urgency: "watch".into(),
            item_source: None,
            discrepancy: None,
        }],
        recent_wins: vec![
            IntelWin {
                text: "Drove 40% usage growth in Team A".into(),
                source: Some("usage analytics".into()),
                impact: Some("Strongest adoption success story at Globex".into()),
                item_source: None,
                discrepancy: None,
            },
            IntelWin {
                text: "Offered to present at QBR — proactive champion behavior".into(),
                source: Some("email".into()),
                impact: Some("Internal advocacy momentum".into()),
                item_source: None,
                discrepancy: None,
            },
        ],
        current_state: Some(CurrentState {
            working: vec![
                "Active champion behavior".into(),
                "Team A adoption strong".into(),
            ],
            not_working: vec!["Team B decline partly in his org".into()],
            unknowns: vec!["Whether he'd accept executive sponsor role formally".into()],
        }),
        stakeholder_insights: vec![StakeholderInsight {
            name: "Jamie Morrison".into(),
            role: Some("Eng Director".into()),
            assessment: Some("Best exec sponsor successor candidate.".into()),
            engagement: Some("active".into()),
            source: None,
            person_id: Some("mock-jamie-morrison".into()),
            suggested_person_id: None,
            item_source: None,
            discrepancy: None,
        }],
        relationship_depth: Some(RelationshipDepth {
            champion_strength: Some("strong".into()),
            executive_access: Some("indirect".into()),
            stakeholder_coverage: None,
            coverage_gaps: None,
        }),
        network: Some(NetworkIntelligence {
            health: "moderate".into(),
            key_relationships: vec![
                NetworkKeyRelationship {
                    person_id: "mock-pat-reynolds".into(),
                    name: "Pat Reynolds".into(),
                    relationship_type: "reports_to".into(),
                    confidence: 0.90,
                    signal_summary: Some("Direct report, strong relationship".into()),
                },
                NetworkKeyRelationship {
                    person_id: "mock-casey-lee".into(),
                    name: "Casey Lee".into(),
                    relationship_type: "works_with".into(),
                    confidence: 0.80,
                    signal_summary: Some("Cross-functional collaboration".into()),
                },
            ],
            risks: vec!["Pat Reynolds departure may leave him isolated at VP level".into()],
            opportunities: vec!["Could be elevated to executive sponsor role".into()],
            influence_radius: 2,
            cluster_summary: Some("Globex product/engineering cluster".into()),
        }),
        ..Default::default()
    };

    db.upsert_entity_intelligence(&jamie_intel)
        .map_err(|e| format!("Jamie Morrison intelligence: {}", e))?;

    // --- Person intelligence: Dana Patel ---
    let dana_intel = IntelligenceJson {
        entity_id: "mock-dana-patel".into(),
        entity_type: "person".into(),
        enriched_at: today.clone(),
        source_file_count: 2,
        executive_assessment: Some(
            "Dana Patel is a data-driven CTO who values quantitative outcomes. Phase 1 delivered \
             strong ROI data that supports her Phase 2 business case. She's actively championing \
             the expansion internally but the finance team is the bottleneck."
                .into(),
        ),
        risks: vec![IntelRisk {
            text: "Finance approval delay could cool her enthusiasm if it drags past 2 weeks"
                .into(),
            source: Some("inferred".into()),
            urgency: "watch".into(),
            item_source: None,
            discrepancy: None,
        }],
        recent_wins: vec![IntelWin {
            text: "Phase 1 success validates her technology bet".into(),
            source: Some("project outcomes".into()),
            impact: Some("Strengthens her credibility with finance and board".into()),
            item_source: None,
            discrepancy: None,
        }],
        current_state: Some(CurrentState {
            working: vec![
                "Actively championing Phase 2 internally".into(),
                "Escalated budget request to CFO".into(),
            ],
            not_working: vec!["Finance team hasn't responded to budget request".into()],
            unknowns: vec!["CFO's appetite for the expansion".into()],
        }),
        stakeholder_insights: vec![StakeholderInsight {
            name: "Dana Patel".into(),
            role: Some("CTO".into()),
            assessment: Some("Data-driven, direct, values speed.".into()),
            engagement: Some("active".into()),
            source: None,
            person_id: Some("mock-dana-patel".into()),
            suggested_person_id: None,
            item_source: None,
            discrepancy: None,
        }],
        relationship_depth: Some(RelationshipDepth {
            champion_strength: Some("developing".into()),
            executive_access: Some("direct".into()),
            stakeholder_coverage: None,
            coverage_gaps: None,
        }),
        network: Some(NetworkIntelligence {
            health: "moderate".into(),
            key_relationships: vec![NetworkKeyRelationship {
                person_id: "mock-priya-sharma".into(),
                name: "Priya Sharma".into(),
                relationship_type: "works_with".into(),
                confidence: 0.85,
                signal_summary: Some("CTO/VP Product partnership".into()),
            }],
            risks: vec![],
            opportunities: vec!["Could provide introduction to CFO for budget conversation".into()],
            influence_radius: 2,
            cluster_summary: Some("Initech executive team".into()),
        }),
        ..Default::default()
    };

    db.upsert_entity_intelligence(&dana_intel)
        .map_err(|e| format!("Dana Patel intelligence: {}", e))?;

    // =========================================================================
    // Phase 5: Signal Events (20+ rows)
    // =========================================================================

    let signal_rows: Vec<(&str, &str, &str, &str, &str, f64, Option<f64>, String)> = vec![
        // (entity_type, entity_id, signal_type, source, value, confidence, decay_half_life_days, created_at)
        // Meeting completed signals
        (
            "account",
            "mock-acme-corp",
            "meeting_completed",
            "calendar",
            "Weekly sync completed",
            0.9,
            Some(14.0),
            days_ago_rfc(2),
        ),
        (
            "account",
            "mock-acme-corp",
            "meeting_completed",
            "calendar",
            "Phase 2 scoping session",
            0.9,
            Some(14.0),
            days_ago_rfc(7),
        ),
        (
            "account",
            "mock-globex-industries",
            "meeting_completed",
            "calendar",
            "Check-in with Jamie",
            0.8,
            Some(14.0),
            days_ago_rfc(3),
        ),
        (
            "account",
            "mock-initech",
            "meeting_completed",
            "calendar",
            "Phase 1 wrap meeting",
            0.8,
            Some(14.0),
            days_ago_rfc(10),
        ),
        // Email received signals
        (
            "account",
            "mock-acme-corp",
            "email_received",
            "gmail",
            "Phase 2 SOW discussion",
            0.7,
            Some(7.0),
            days_ago_rfc(1),
        ),
        (
            "account",
            "mock-globex-industries",
            "email_received",
            "gmail",
            "Team B concerns from Casey",
            0.6,
            Some(7.0),
            days_ago_rfc(2),
        ),
        (
            "account",
            "mock-initech",
            "email_received",
            "gmail",
            "Budget update from Dana",
            0.7,
            Some(7.0),
            days_ago_rfc(5),
        ),
        // Entity updated signals (enrichment cycles)
        (
            "account",
            "mock-acme-corp",
            "entity_updated",
            "ai_enrichment",
            "Intelligence refreshed",
            0.8,
            Some(30.0),
            days_ago_rfc(1),
        ),
        (
            "account",
            "mock-globex-industries",
            "entity_updated",
            "ai_enrichment",
            "Intelligence refreshed",
            0.8,
            Some(30.0),
            days_ago_rfc(1),
        ),
        (
            "account",
            "mock-initech",
            "entity_updated",
            "ai_enrichment",
            "Intelligence refreshed",
            0.7,
            Some(30.0),
            days_ago_rfc(3),
        ),
        // Intelligence curated (user deleted items)
        (
            "account",
            "mock-acme-corp",
            "intelligence_curated",
            "user_correction",
            "Removed outdated risk about budget",
            0.3,
            Some(90.0),
            days_ago_rfc(5),
        ),
        (
            "account",
            "mock-globex-industries",
            "intelligence_curated",
            "user_correction",
            "Dismissed stale win about Q3 pilot",
            0.3,
            Some(90.0),
            days_ago_rfc(8),
        ),
        // User correction signals
        (
            "account",
            "mock-acme-corp",
            "user_correction",
            "user_correction",
            "Updated NPS score from 38 to 42",
            0.3,
            Some(180.0),
            days_ago_rfc(3),
        ),
        (
            "person",
            "mock-sarah-chen",
            "user_correction",
            "user_correction",
            "Corrected role to VP Engineering",
            0.3,
            Some(180.0),
            days_ago_rfc(12),
        ),
        // Person profile updated
        (
            "person",
            "mock-sarah-chen",
            "person_profile_updated",
            "ai_enrichment",
            "Profile enriched via Clay",
            0.8,
            Some(60.0),
            days_ago_rfc(7),
        ),
        (
            "person",
            "mock-jamie-morrison",
            "person_profile_updated",
            "ai_enrichment",
            "Profile enriched via Gravatar",
            0.6,
            Some(60.0),
            days_ago_rfc(14),
        ),
        // Enrichment stale
        (
            "account",
            "mock-initech",
            "enrichment_stale",
            "system",
            "Last enrichment 30+ days ago",
            0.5,
            Some(7.0),
            days_ago_rfc(2),
        ),
        (
            "person",
            "mock-dana-patel",
            "enrichment_stale",
            "system",
            "Profile needs refresh",
            0.4,
            Some(7.0),
            days_ago_rfc(1),
        ),
        // Glean signals
        (
            "person",
            "mock-pat-reynolds",
            "glean_contact_discovered",
            "glean",
            "Discovered via Glean directory",
            0.7,
            Some(90.0),
            days_ago_rfc(20),
        ),
        (
            "person",
            "mock-casey-lee",
            "profile_enriched",
            "glean",
            "Enriched from Glean profile",
            0.7,
            Some(90.0),
            days_ago_rfc(15),
        ),
        // Co-attendance signal
        (
            "account",
            "mock-acme-corp",
            "co_attendance",
            "calendar",
            "Sarah Chen + Pat Kim at strategy meeting",
            0.8,
            Some(30.0),
            days_ago_rfc(4),
        ),
    ];

    for (entity_type, entity_id, signal_type, source, value, confidence, decay, created_at) in
        &signal_rows
    {
        let sig_id = format!(
            "mock-sig-{}-{}-{}",
            entity_id.replace("mock-", ""),
            signal_type.replace('_', "-"),
            &created_at[..10]
        );
        conn.execute(
            "INSERT OR REPLACE INTO signal_events (id, entity_type, entity_id, signal_type, source, value, confidence, decay_half_life_days, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![sig_id, entity_type, entity_id, signal_type, source, value, confidence, decay, created_at],
        ).map_err(|e| format!("Signal event: {}", e))?;
    }

    // =========================================================================
    // Pre-Meeting Context Signals (email-meeting linkage)
    // =========================================================================
    // These need dynamic meeting IDs (include today's date), so they're separate
    // from the static signal_rows vec above. entity_type = "meeting" and
    // entity_id = meeting ID, matching the JOIN in load_pre_meeting_links().
    let today_date = date_only(0);

    let pre_meeting_signals: Vec<(&str, String, serde_json::Value, f64)> = vec![
        // Sarah Chen's migration email → Acme Weekly Sync
        (
            "mock-acme-corp",
            format!("mock-mtg-acme-weekly-{}", today_date),
            serde_json::json!({
                "meeting_id": format!("mock-mtg-acme-weekly-{}", today_date),
                "meeting_title": "Acme Corp Weekly Sync",
                "email_signal_id": "mock-email-acme-1",
                "sender_email": "sarah.chen@acme.com",
                "signal_text": "Sarah Chen following up on migration timeline ahead of today's Acme Weekly Sync"
            }),
            0.85,
        ),
        // Jamie Morrison's renewal email → Globex QBR
        (
            "mock-globex-industries",
            format!("mock-mtg-globex-qbr-{}", today_date),
            serde_json::json!({
                "meeting_id": format!("mock-mtg-globex-qbr-{}", today_date),
                "meeting_title": "Globex Industries QBR",
                "email_signal_id": "mock-email-globex-1",
                "sender_email": "jamie.morrison@globex.com",
                "signal_text": "Jamie Morrison revisiting renewal terms before the Globex QBR"
            }),
            0.85,
        ),
        // Casey Lee's Team B email → Globex QBR (second attendee link)
        (
            "mock-globex-industries",
            format!("mock-mtg-globex-qbr-{}", today_date),
            serde_json::json!({
                "meeting_id": format!("mock-mtg-globex-qbr-{}", today_date),
                "meeting_title": "Globex Industries QBR",
                "email_signal_id": "mock-email-globex-4",
                "sender_email": "casey.lee@globex.com",
                "signal_text": "Casey Lee raised Team B engagement concerns before QBR"
            }),
            0.85,
        ),
    ];

    for (_account_id, meeting_id, value_json, confidence) in &pre_meeting_signals {
        let sig_id = format!(
            "mock-sig-premc-{}-{}",
            meeting_id.replace("mock-mtg-", ""),
            value_json
                .get("sender_email")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .split('@')
                .next()
                .unwrap_or("unknown")
                .replace('.', "-"),
        );
        let value_str = value_json.to_string();
        conn.execute(
            "INSERT OR REPLACE INTO signal_events (id, entity_type, entity_id, signal_type, source, value, confidence, decay_half_life_days, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                sig_id,
                "meeting",
                meeting_id,
                "pre_meeting_context",
                "email_thread",
                value_str,
                confidence,
                7.0,
                days_ago_rfc(0)
            ],
        )
        .map_err(|e| format!("Pre-meeting context signal: {}", e))?;
    }

    // =========================================================================
    // Intelligence Feedback (6 rows)
    // =========================================================================

    db.insert_intelligence_feedback(&crate::db::intelligence_feedback::FeedbackInput {
        id: "mock-fb-acme-risks-pos",
        entity_id: "mock-acme-corp",
        entity_type: "account",
        field: "risks",
        feedback_type: "positive",
        previous_value: None,
        context: Some("User confirmed risk assessment accuracy"),
    })?;
    db.insert_intelligence_feedback(&crate::db::intelligence_feedback::FeedbackInput {
        id: "mock-fb-globex-stakeholder-pos",
        entity_id: "mock-globex-industries",
        entity_type: "account",
        field: "stakeholder_insights",
        feedback_type: "positive",
        previous_value: None,
        context: Some("User confirmed Jamie Morrison assessment"),
    })?;
    db.insert_intelligence_feedback(&crate::db::intelligence_feedback::FeedbackInput {
        id: "mock-fb-initech-exec-neg",
        entity_id: "mock-initech",
        entity_type: "account",
        field: "executive_assessment",
        feedback_type: "negative",
        previous_value: None,
        context: Some("User found assessment too optimistic"),
    })?;
    db.insert_intelligence_feedback(&crate::db::intelligence_feedback::FeedbackInput {
        id: "mock-fb-globex-renewal-neg",
        entity_id: "mock-globex-industries",
        entity_type: "account",
        field: "renewal_outlook",
        feedback_type: "negative",
        previous_value: None,
        context: Some("User disagrees with renewal confidence"),
    })?;
    db.insert_intelligence_feedback(&crate::db::intelligence_feedback::FeedbackInput {
        id: "mock-fb-acme-health-replaced",
        entity_id: "mock-acme-corp",
        entity_type: "account",
        field: "health.score",
        feedback_type: "replaced",
        previous_value: Some("85"),
        context: Some("User corrected health score from 85 to 78"),
    })?;
    db.insert_intelligence_feedback(&crate::db::intelligence_feedback::FeedbackInput {
        id: "mock-fb-globex-exec-replaced",
        entity_id: "mock-globex-industries",
        entity_type: "account",
        field: "executive_assessment",
        feedback_type: "replaced",
        previous_value: Some("Globex is in good shape overall"),
        context: Some("User rewrote assessment to reflect risk"),
    })?;

    // =========================================================================
    // Signal Weights (4 rows)
    // =========================================================================

    let weight_rows: Vec<(&str, &str, &str, f64, f64, i32)> = vec![
        // (source, entity_type, signal_type, alpha, beta, update_count)
        ("glean", "account", "entity_updated", 8.0, 2.0, 10),
        ("ai_enrichment", "account", "entity_updated", 12.0, 5.0, 17),
        ("gmail", "account", "email_received", 6.0, 1.0, 7),
        ("user_correction", "account", "user_correction", 3.0, 0.0, 3),
    ];

    for (source, entity_type, signal_type, alpha, beta, update_count) in &weight_rows {
        conn.execute(
            "INSERT OR REPLACE INTO signal_weights (source, entity_type, signal_type, alpha, beta, update_count, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![source, entity_type, signal_type, alpha, beta, update_count, &today],
        ).map_err(|e| format!("Signal weight: {}", e))?;
    }

    // I645: entity_feedback_events and suppression_tombstones are populated
    // by user actions (thumbs/dismiss/accept). No mock seeds — start empty.

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
    let make_event = |id: &str,
                      title: &str,
                      start_h: u32,
                      start_m: u32,
                      end_h: u32,
                      end_m: u32,
                      mtype: MeetingType,
                      account: Option<&str>,
                      attendees: Vec<&str>|
     -> CalendarEvent {
        CalendarEvent {
            id: id.to_string(),
            title: title.to_string(),
            start: Utc.from_utc_datetime(&today.and_hms_opt(start_h, start_m, 0).unwrap()),
            end: Utc.from_utc_datetime(&today.and_hms_opt(end_h, end_m, 0).unwrap()),
            meeting_type: mtype,
            account: account.map(|s| s.to_string()),
            attendees: attendees.into_iter().map(String::from).collect(),
            is_all_day: false,
            linked_entities: None,
            classified_entities: None,
        }
    };

    // Use the same calendar event IDs as schedule.json.tmpl (after {{DATE}} patching).
    // Attendee emails match the people seeded in seed_database().
    let events = vec![
        // #1: Acme Weekly (past, 8:00 AM) — key Acme stakeholders
        make_event(
            &format!("mock-cal-acme-weekly-{}", today_str),
            "Acme Corp Weekly Sync",
            13,
            0,
            13,
            45, // 8:00-8:45 AM ET = 13:00-13:45 UTC
            MeetingType::Customer,
            Some("Acme Corp"),
            vec![
                "sarah.chen@acme.com",
                "alex.torres@acme.com",
                "mike.chen@dailyos.test",
            ],
        ),
        // #2: Eng Standup (past, 9:30 AM) — internal team
        make_event(
            &format!("mock-cal-eng-standup-{}", today_str),
            "Engineering Standup",
            14,
            30,
            14,
            45, // 9:30-9:45 AM ET
            MeetingType::TeamSync,
            None,
            vec![
                "mike.chen@dailyos.test",
                "lisa.park@dailyos.test",
                "taylor.nguyen@contractor.io",
            ],
        ),
        // #3: Initech Kickoff OMITTED — will become "cancelled"
        // #4: 1:1 with Sarah (11:00 AM) — manager
        make_event(
            &format!("mock-cal-1on1-sarah-{}", today_str),
            "1:1 with Sarah (Manager)",
            16,
            0,
            16,
            30, // 11:00-11:30 AM ET
            MeetingType::OneOnOne,
            None,
            vec!["lisa.park@dailyos.test"],
        ),
        // #5: Globex QBR (1:00 PM) — all Globex stakeholders + contractor
        make_event(
            &format!("mock-cal-globex-qbr-{}", today_str),
            "Globex Industries QBR",
            18,
            0,
            19,
            0, // 1:00-2:00 PM ET
            MeetingType::Qbr,
            Some("Globex Industries"),
            vec![
                "pat.reynolds@globex.com",
                "jamie.morrison@globex.com",
                "casey.lee@globex.com",
                "taylor.nguyen@contractor.io",
            ],
        ),
        // #6: Sprint Review (2:30 PM) — internal team
        make_event(
            &format!("mock-cal-sprint-review-{}", today_str),
            "Product Team Sprint Review",
            19,
            30,
            20,
            15, // 2:30-3:15 PM ET
            MeetingType::Internal,
            None,
            vec!["mike.chen@dailyos.test", "lisa.park@dailyos.test"],
        ),
        // #7: Initech Onboarding — NOT in briefing → "new"
        make_event(
            &format!("mock-cal-initech-onboarding-{}", today_str),
            "Initech Onboarding Call",
            20,
            30,
            21,
            30, // 3:30-4:30 PM ET
            MeetingType::Training,
            Some("Initech"),
            vec!["dana.patel@initech.com", "priya.sharma@initech.com"],
        ),
        // #8: All Hands (4:30 PM) — no individual attendees (50+ people)
        make_event(
            &format!("mock-cal-all-hands-{}", today_str),
            "Company All Hands",
            21,
            30,
            22,
            30, // 4:30-5:30 PM ET
            MeetingType::AllHands,
            None,
            vec![],
        ),
    ];

    *state.calendar.events.write() = events;

    Ok(())
}

/// Write directive JSON fixtures for pipeline testing (bypass Phase 1).
fn write_directive_fixtures(workspace: &Path) -> Result<(), String> {
    let data_dir = workspace.join("_today").join("data");
    std::fs::create_dir_all(&data_dir).map_err(|e| format!("Failed to create data dir: {}", e))?;

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
    )
    .map_err(|e| format!("Failed to write Acme prep: {}", e))?;

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
    )
    .map_err(|e| format!("Failed to write Globex QBR prep: {}", e))?;

    Ok(())
}

/// Write project workspace files (dashboard.json + dashboard.md) for 3 mock projects.
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
    )
    .map_err(|e| format!("Failed to write Phase 2 dashboard.json: {}", e))?;

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
        "\u{1f7e2}",
        date_only(30),
        "\u{1f7e2}",
        date_only(10),
        "\u{26aa}",
        date_only(30),
        "\u{26aa}",
        date_only(90),
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
    )
    .map_err(|e| format!("Failed to write Team B dashboard.json: {}", e))?;

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
        "\u{1f7e2}",
        date_only(14),
        "\u{1f7e2}",
        date_only(7),
        "\u{26aa}",
        date_only(14),
        "\u{26aa}",
        date_only(45),
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
    )
    .map_err(|e| format!("Failed to write Migration dashboard.json: {}", e))?;

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
        "\u{1f7e1}",
        date_only(60),
        "\u{1f7e1}",
        date_only(21),
        "\u{26aa}",
        date_only(45),
        "\u{26aa}",
        date_only(60),
    );
    std::fs::write(migration_dir.join("dashboard.md"), migration_md)
        .map_err(|e| format!("Failed to write Migration dashboard.md: {}", e))?;

    Ok(())
}

// write_mock_google_token removed (I298 fix) — mock scenarios must NEVER
// persist tokens to Keychain. Use in-memory GoogleAuthStatus::Authenticated instead.

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
