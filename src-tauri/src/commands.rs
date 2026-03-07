use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::OnceLock;

/// Dev-only override for Claude status checks.
/// 0 = real check, 1 = installed+authenticated, 2 = not installed, 3 = installed but not authenticated
pub(crate) static DEV_CLAUDE_OVERRIDE: AtomicU8 = AtomicU8::new(0);

/// Dev-only override for Google auth status.
/// 0 = real check, 1 = authenticated, 2 = not configured, 3 = token expired
pub(crate) static DEV_GOOGLE_OVERRIDE: AtomicU8 = AtomicU8::new(0);

use chrono::TimeZone;
use regex::Regex;
use tauri::{Emitter, Manager, State};

use crate::executor::request_workflow_execution;
use crate::hygiene::{build_intelligence_hygiene_status, HygieneStatusView};
use crate::json_loader::load_emails_json;
use crate::parser::list_inbox_files;
use crate::scheduler::get_next_run_time as scheduler_get_next_run_time;
use crate::state::{reload_config, AppState};
use crate::types::{
    CalendarEvent, CapturedOutcome, Config, EmailBriefingData, ExecutionRecord, FullMeetingPrep,
    GoogleAuthStatus, InboxFile, LiveProactiveSuggestion, MeetingIntelligence,
    PostMeetingCaptureConfig, SourceReference, WorkflowId, WorkflowStatus,
};
use crate::SchedulerSender;

// Result types now in services
pub use crate::services::actions::ActionsResult;
pub use crate::services::dashboard::DashboardResult;
pub use crate::services::dashboard::WeekResult;

/// p95 latency budgets for hot read commands.
const READ_CMD_LATENCY_BUDGET_MS: u128 = 100;
const CLAUDE_STATUS_CACHE_TTL_SECS: u64 = 300;
// TODO(I197 follow-up): migrate remaining command DB call sites to AppState DB
// helpers in passes, prioritizing frequent reads before one-off write paths.

fn log_command_latency(command: &str, started: std::time::Instant, budget_ms: u128) {
    let elapsed_ms = started.elapsed().as_millis();
    crate::latency::record_latency(command, elapsed_ms, budget_ms);
    if elapsed_ms > budget_ms {
        log::warn!(
            "{} exceeded latency budget: {}ms > {}ms",
            command,
            elapsed_ms,
            budget_ms
        );
    } else {
        log::debug!("{} completed in {}ms", command, elapsed_ms);
    }
}

/// Get current configuration
#[tauri::command]
pub fn get_config(state: State<'_, Arc<AppState>>) -> Result<Config, String> {
    let guard = state.config.read().map_err(|_| "Lock poisoned")?;
    guard
        .clone()
        .ok_or_else(|| "No configuration loaded. Create ~/.dailyos/config.json".to_string())
}

/// Reload configuration from disk
#[tauri::command]
pub fn reload_configuration(state: State<'_, Arc<AppState>>) -> Result<Config, String> {
    reload_config(&state)
}

/// Get dashboard data from workspace _today/data/ JSON files
#[tauri::command]
pub async fn get_dashboard_data(
    state: State<'_, Arc<AppState>>,
) -> Result<DashboardResult, String> {
    Ok(crate::services::dashboard::get_dashboard_data(&state).await)
}

/// Trigger a workflow execution
#[tauri::command]
pub fn run_workflow(workflow: String, sender: State<SchedulerSender>) -> Result<String, String> {
    let workflow_id: WorkflowId = workflow.parse().map_err(|e: String| e)?;

    request_workflow_execution(&sender.0, workflow_id)?;

    Ok(format!("Workflow '{}' queued for execution", workflow))
}

/// Get the current status of a workflow
#[tauri::command]
pub fn get_workflow_status(
    workflow: String,
    state: State<'_, Arc<AppState>>,
) -> Result<WorkflowStatus, String> {
    let started = std::time::Instant::now();
    let result = (|| {
        let workflow_id: WorkflowId = workflow.parse()?;
        Ok(state.get_workflow_status(workflow_id))
    })();
    log_command_latency("get_workflow_status", started, READ_CMD_LATENCY_BUDGET_MS);
    result
}

/// Get execution history
#[tauri::command]
pub fn get_execution_history(
    limit: Option<usize>,
    state: State<'_, Arc<AppState>>,
) -> Vec<ExecutionRecord> {
    let started = std::time::Instant::now();
    let result = state.get_execution_history(limit.unwrap_or(10));
    log_command_latency("get_execution_history", started, READ_CMD_LATENCY_BUDGET_MS);
    result
}

/// Get the next scheduled run time for a workflow
#[tauri::command]
pub fn get_next_run_time(
    workflow: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Option<String>, String> {
    let started = std::time::Instant::now();
    let result = (|| {
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
    })();
    log_command_latency("get_next_run_time", started, READ_CMD_LATENCY_BUDGET_MS);
    result
}

// =============================================================================
// Meeting Prep Command
// =============================================================================

/// Result type for meeting prep
#[derive(Debug, serde::Serialize)]
#[allow(clippy::large_enum_variant)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum MeetingPrepResult {
    Success { data: FullMeetingPrep },
    NotFound { message: String },
    Error { message: String },
}

fn parse_meeting_datetime(value: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    if value.trim().is_empty() {
        return None;
    }
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(value) {
        return Some(dt.with_timezone(&chrono::Utc));
    }
    for fmt in ["%Y-%m-%d %H:%M:%S", "%Y-%m-%d %H:%M", "%Y-%m-%d %I:%M %p"] {
        if let Ok(ndt) = chrono::NaiveDateTime::parse_from_str(value, fmt) {
            if let Some(local_dt) = chrono::Local.from_local_datetime(&ndt).single() {
                return Some(local_dt.with_timezone(&chrono::Utc));
            }
            return Some(chrono::Utc.from_utc_datetime(&ndt));
        }
    }
    None
}

/// Parsed user agenda layer — supports both legacy `["item"]` and rich `{ items, dismissedTopics, hiddenAttendees }`.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserAgendaLayer {
    #[serde(default)]
    items: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    dismissed_topics: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    hidden_attendees: Vec<String>,
}

fn parse_user_agenda_layer(value: Option<&str>) -> UserAgendaLayer {
    let Some(json) = value else {
        return UserAgendaLayer::default();
    };
    // Try rich format first
    if let Ok(layer) = serde_json::from_str::<UserAgendaLayer>(json) {
        return layer;
    }
    // Fall back to legacy Vec<String>
    if let Ok(items) = serde_json::from_str::<Vec<String>>(json) {
        return UserAgendaLayer {
            items,
            ..Default::default()
        };
    }
    UserAgendaLayer::default()
}

fn parse_user_agenda_json(value: Option<&str>) -> Option<Vec<String>> {
    let layer = parse_user_agenda_layer(value);
    if layer.items.is_empty() {
        None
    } else {
        Some(layer.items)
    }
}

fn load_meeting_prep_from_sources(
    today_dir: &Path,
    meeting: &crate::db::DbMeeting,
) -> Option<FullMeetingPrep> {
    crate::services::meetings::load_meeting_prep_from_sources(today_dir, meeting)
}

fn collect_meeting_outcomes_from_db(
    db: &crate::db::ActionDb,
    meeting: &crate::db::DbMeeting,
) -> Option<crate::types::MeetingOutcomeData> {
    crate::services::meetings::collect_meeting_outcomes_from_db(db, meeting)
}

/// Unified meeting detail payload for current + historical meetings.
#[tauri::command]
pub async fn get_meeting_intelligence(
    meeting_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<MeetingIntelligence, String> {
    crate::services::meetings::get_meeting_intelligence(&state, &meeting_id).await
}

/// Single-service full refresh for a meeting briefing.
///
/// Clears existing briefing, refreshes linked entity intelligence, and rebuilds
/// meeting prep. Emits `meeting-briefing-refresh-progress` events as it runs.
#[tauri::command]
pub async fn refresh_meeting_briefing(
    state: State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
    meeting_id: String,
) -> Result<crate::services::meetings::MeetingBriefingRefreshResult, String> {
    let result = crate::services::meetings::refresh_meeting_briefing_full(
        &state,
        &meeting_id,
        Some(&app_handle),
    )
    .await?;
    let _ = app_handle.emit("entity-updated", ());
    Ok(result)
}

/// Generate or refresh intelligence for a single meeting (ADR-0081).
/// Pass `force: true` to clear existing intelligence and regenerate from scratch.
#[tauri::command]
pub async fn generate_meeting_intelligence(
    state: State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
    meeting_id: String,
    force: Option<bool>,
) -> Result<crate::types::IntelligenceQuality, String> {
    let force_full = force.unwrap_or(false);
    if force_full {
        let result = crate::services::meetings::refresh_meeting_briefing_full(
            &state,
            &meeting_id,
            Some(&app_handle),
        )
        .await?;
        let _ = app_handle.emit("entity-updated", ());
        return Ok(result.quality);
    }

    let result =
        crate::intelligence::generate_meeting_intelligence(&state, &meeting_id, force_full)
            .await
            .map_err(|e| e.to_string())?;
    let _ = app_handle.emit("entity-updated", ());
    Ok(result)
}

/// Trigger background enrichment for a single meeting without blocking the caller.
/// Returns immediately; enrichment runs asynchronously in a spawned task.
/// Emits `intelligence-updated` with the meeting_id on completion.
#[tauri::command]
pub async fn enrich_meeting_background(
    meeting_id: String,
    state: State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let state = Arc::clone(&state);
    tokio::spawn(async move {
        match crate::intelligence::generate_meeting_intelligence(&state, &meeting_id, false).await {
            Ok(_) => {
                let _ = app_handle.emit("intelligence-updated", &meeting_id);
            }
            Err(e) => {
                log::warn!("enrich_meeting_background failed for {}: {}", meeting_id, e);
            }
        }
    });
    Ok(())
}

/// Compatibility wrapper while frontend migrates to get_meeting_intelligence.
#[tauri::command]
pub async fn get_meeting_prep(
    meeting_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<MeetingPrepResult, String> {
    Ok(match get_meeting_intelligence(meeting_id, state).await {
        Ok(intel) => match intel.prep {
            Some(data) => MeetingPrepResult::Success { data },
            None => MeetingPrepResult::NotFound {
                message: "Meeting found but has no prep data".to_string(),
            },
        },
        Err(message) => MeetingPrepResult::NotFound { message },
    })
}

#[derive(Debug, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackfillReport {
    pub dry_run: bool,
    pub candidate_file_count: usize,
    pub candidate_db_row_count: usize,
    pub transformed_file_count: usize,
    pub transformed_db_row_count: usize,
    pub skipped_file_count: usize,
    pub skipped_db_row_count: usize,
    pub parse_error_file_count: usize,
    pub parse_error_db_row_count: usize,
}

fn backfill_source_tail_regex() -> &'static Regex {
    static SOURCE_TAIL_RE: OnceLock<Regex> = OnceLock::new();
    SOURCE_TAIL_RE.get_or_init(|| {
        Regex::new(r"(?i)(?:^|\s)[_*]*\(?\s*source:\s*([^)]+?)\s*\)?[_*\s]*$")
            .expect("source tail regex should compile")
    })
}

fn backfill_recent_win_prefix_regex() -> &'static Regex {
    static RECENT_WIN_PREFIX_RE: OnceLock<Regex> = OnceLock::new();
    RECENT_WIN_PREFIX_RE.get_or_init(|| {
        Regex::new(r"(?i)^(recent\s+win|win)\s*:\s*")
            .expect("recent win prefix regex should compile")
    })
}

fn sanitize_backfill_text(value: &str) -> String {
    value
        .replace("**", "")
        .replace("__", "")
        .replace(['`', '*'], "")
        .replace('_', " ")
        .replace(['[', ']', '(', ')'], "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn split_backfill_inline_source(value: &str) -> (String, Option<String>) {
    let raw = value.trim();
    if let Some(caps) = backfill_source_tail_regex().captures(raw) {
        if let Some(full_match) = caps.get(0) {
            let cleaned = raw[..full_match.start()].trim().to_string();
            let source = caps
                .get(1)
                .map(|m| sanitize_backfill_text(m.as_str()))
                .and_then(|s| if s.is_empty() { None } else { Some(s) });
            return (cleaned, source);
        }
    }
    (raw.to_string(), None)
}

fn clean_recent_win_for_backfill(value: &str) -> Option<String> {
    let (without_source, _) = split_backfill_inline_source(value);
    let cleaned = backfill_recent_win_prefix_regex()
        .replace(&without_source, "")
        .to_string();
    let cleaned = sanitize_backfill_text(&cleaned);
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

fn clean_generic_prep_line(value: &str) -> Option<String> {
    let (without_source, _) = split_backfill_inline_source(value);
    let cleaned = sanitize_backfill_text(&without_source);
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

fn source_reference_from_raw(source: &str) -> Option<SourceReference> {
    let cleaned = sanitize_backfill_text(source);
    if cleaned.is_empty() {
        return None;
    }
    let label = cleaned
        .split(['/', '\\'])
        .rfind(|part| !part.trim().is_empty())
        .unwrap_or(cleaned.as_str())
        .to_string();
    Some(SourceReference {
        label,
        path: Some(cleaned),
        last_updated: None,
    })
}

fn normalized_source_key(source: &SourceReference) -> String {
    source
        .path
        .as_deref()
        .unwrap_or(&source.label)
        .to_lowercase()
}

fn parse_source_reference_value(value: &serde_json::Value) -> Option<SourceReference> {
    let obj = value.as_object()?;
    let label = obj
        .get("label")
        .and_then(|v| v.as_str())
        .map(sanitize_backfill_text)
        .unwrap_or_default();
    let path = obj
        .get("path")
        .and_then(|v| v.as_str())
        .map(sanitize_backfill_text)
        .filter(|s| !s.is_empty());
    let resolved_label = if label.is_empty() {
        path.as_deref()
            .and_then(|p| p.split(['/', '\\']).rfind(|s| !s.is_empty()))
            .unwrap_or("")
            .to_string()
    } else {
        label
    };
    if resolved_label.is_empty() {
        return None;
    }
    Some(SourceReference {
        label: resolved_label,
        path,
        last_updated: None,
    })
}

fn backfill_prep_semantics_value(prep: &mut serde_json::Value) -> bool {
    let Some(obj) = prep.as_object_mut() else {
        return false;
    };

    let mut changed = false;
    let mut win_keys: HashSet<String> = HashSet::new();
    let mut source_keys: HashSet<String> = HashSet::new();
    let mut normalized_wins: Vec<String> = Vec::new();
    let mut normalized_sources: Vec<SourceReference> = Vec::new();

    if let Some(existing_sources) = obj.get("recentWinSources").and_then(|v| v.as_array()) {
        for source in existing_sources {
            if let Some(src) = parse_source_reference_value(source) {
                let key = normalized_source_key(&src);
                if !source_keys.contains(&key) {
                    source_keys.insert(key);
                    normalized_sources.push(src);
                }
            }
        }
    }

    if let Some(existing_wins) = obj.get("recentWins").and_then(|v| v.as_array()) {
        for win in existing_wins {
            let Some(raw) = win.as_str() else { continue };
            let (without_source, extracted_source) = split_backfill_inline_source(raw);
            if let Some(cleaned) = clean_recent_win_for_backfill(&without_source) {
                let key = cleaned.to_lowercase();
                if !win_keys.contains(&key) {
                    win_keys.insert(key);
                    normalized_wins.push(cleaned);
                }
            }
            if let Some(source) = extracted_source {
                if let Some(src_ref) = source_reference_from_raw(&source) {
                    let key = normalized_source_key(&src_ref);
                    if !source_keys.contains(&key) {
                        source_keys.insert(key);
                        normalized_sources.push(src_ref);
                    }
                }
            }
        }
    }

    let talking_points_original: Vec<String> = obj
        .get("talkingPoints")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(ToString::to_string))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if !talking_points_original.is_empty() {
        let mut cleaned_points: Vec<String> = Vec::new();
        let mut talking_point_seen: HashSet<String> = HashSet::new();
        let derive_wins_from_talking_points = normalized_wins.is_empty();

        for point in &talking_points_original {
            let (without_source, extracted_source) = split_backfill_inline_source(point);

            if let Some(source) = extracted_source {
                if let Some(src_ref) = source_reference_from_raw(&source) {
                    let key = normalized_source_key(&src_ref);
                    if !source_keys.contains(&key) {
                        source_keys.insert(key);
                        normalized_sources.push(src_ref);
                    }
                }
            }

            if let Some(cleaned_point) = clean_generic_prep_line(&without_source) {
                let key = cleaned_point.to_lowercase();
                if !talking_point_seen.contains(&key) {
                    talking_point_seen.insert(key);
                    cleaned_points.push(cleaned_point);
                }
            }

            if derive_wins_from_talking_points {
                if let Some(cleaned_win) = clean_recent_win_for_backfill(&without_source) {
                    let win_key = cleaned_win.to_lowercase();
                    if !win_keys.contains(&win_key) {
                        win_keys.insert(win_key);
                        normalized_wins.push(cleaned_win);
                    }
                }
            }
        }

        if cleaned_points != talking_points_original {
            obj.insert(
                "talkingPoints".to_string(),
                serde_json::json!(cleaned_points),
            );
            changed = true;
        }
    }

    if !normalized_wins.is_empty() {
        let wins_value = serde_json::json!(normalized_wins);
        if obj.get("recentWins") != Some(&wins_value) {
            obj.insert("recentWins".to_string(), wins_value);
            changed = true;
        }
    }

    if !normalized_sources.is_empty() {
        let sources_value =
            serde_json::to_value(&normalized_sources).unwrap_or(serde_json::json!([]));
        if obj.get("recentWinSources") != Some(&sources_value) {
            obj.insert("recentWinSources".to_string(), sources_value);
            changed = true;
        }
    }

    changed
}

fn write_json_atomic(path: &Path, value: &serde_json::Value) -> Result<(), String> {
    let temp_path = path.with_extension("json.tmp");
    let payload = serde_json::to_string_pretty(value)
        .map_err(|e| format!("Failed to serialize JSON for {}: {}", path.display(), e))?;
    fs::write(&temp_path, payload)
        .map_err(|e| format!("Failed to write temp file {}: {}", temp_path.display(), e))?;
    fs::rename(&temp_path, path).map_err(|e| format!("Failed to replace {}: {}", path.display(), e))
}

#[derive(Debug, Default, Clone, Copy)]
struct BackfillCounts {
    candidate: usize,
    transformed: usize,
    skipped: usize,
    parse_errors: usize,
}

fn backfill_prep_files_in_dir(preps_dir: &Path, dry_run: bool) -> Result<BackfillCounts, String> {
    let mut counts = BackfillCounts::default();
    if !preps_dir.exists() {
        return Ok(counts);
    }

    let entries = fs::read_dir(preps_dir).map_err(|e| {
        format!(
            "Failed to read preps directory {}: {}",
            preps_dir.display(),
            e
        )
    })?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }

        counts.candidate += 1;
        let raw = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(_) => {
                counts.parse_errors += 1;
                continue;
            }
        };
        let mut prep: serde_json::Value = match serde_json::from_str(&raw) {
            Ok(value) => value,
            Err(_) => {
                counts.parse_errors += 1;
                continue;
            }
        };

        if backfill_prep_semantics_value(&mut prep) {
            counts.transformed += 1;
            if !dry_run {
                write_json_atomic(&path, &prep)?;
            }
        } else {
            counts.skipped += 1;
        }
    }

    Ok(counts)
}

fn backfill_db_prep_contexts(
    db: &crate::db::ActionDb,
    dry_run: bool,
) -> Result<BackfillCounts, String> {
    let mut counts = BackfillCounts::default();
    let rows = db
        .list_meeting_prep_contexts()
        .map_err(|e| format!("Failed to query prep context rows: {}", e))?;
    counts.candidate = rows.len();

    for (meeting_id, prep_json) in rows {
        let mut prep: serde_json::Value = match serde_json::from_str(&prep_json) {
            Ok(value) => value,
            Err(_) => {
                counts.parse_errors += 1;
                continue;
            }
        };
        if backfill_prep_semantics_value(&mut prep) {
            counts.transformed += 1;
            if !dry_run {
                let updated_json = serde_json::to_string(&prep)
                    .map_err(|e| format!("Failed to serialize backfilled prep context: {}", e))?;
                crate::services::mutations::set_meeting_prep_context(
                    db,
                    &meeting_id,
                    &updated_json,
                )
                .map_err(|e| format!("Failed to update prep context for {}: {}", meeting_id, e))?;
            }
        } else {
            counts.skipped += 1;
        }
    }

    Ok(counts)
}

/// One-time semantic backfill for prep payloads (I196).
///
/// Targets:
/// - `_today/data/preps/*.json`
/// - `meeting_prep.prep_context_json`
#[tauri::command]
pub async fn backfill_prep_semantics(
    dry_run: bool,
    state: State<'_, Arc<AppState>>,
) -> Result<BackfillReport, String> {
    let config = state
        .config
        .read()
        .map_err(|_| "Lock poisoned")?
        .clone()
        .ok_or("No configuration loaded")?;

    let workspace = Path::new(&config.workspace_path);
    let preps_dir = workspace.join("_today").join("data").join("preps");

    let mut report = BackfillReport {
        dry_run,
        ..Default::default()
    };

    let file_counts = backfill_prep_files_in_dir(&preps_dir, dry_run)?;
    report.candidate_file_count = file_counts.candidate;
    report.transformed_file_count = file_counts.transformed;
    report.skipped_file_count = file_counts.skipped;
    report.parse_error_file_count = file_counts.parse_errors;

    let db_counts = state
        .db_write(move |db| backfill_db_prep_contexts(db, dry_run))
        .await?;
    report.candidate_db_row_count = db_counts.candidate;
    report.transformed_db_row_count = db_counts.transformed;
    report.skipped_db_row_count = db_counts.skipped;
    report.parse_error_db_row_count = db_counts.parse_errors;

    Ok(report)
}

// =============================================================================
// Week Overview Command
// =============================================================================

/// Get week overview data
#[tauri::command]
pub fn get_week_data(state: State<'_, Arc<AppState>>) -> WeekResult {
    crate::services::dashboard::get_week_data(&state)
}

/// TTL thresholds for week calendar cache (W6).
const WEEK_CACHE_FRESH_SECS: u64 = 120; // 2 min: serve immediately
const WEEK_CACHE_STALE_SECS: u64 = 300; // 5 min: serve stale + background refresh

/// Live proactive suggestions computed from current calendar + SQLite action state.
///
/// Uses a TTL cache to avoid hitting Google Calendar API on every call (W6).
/// Fresh (<2 min): return cached. Stale (2-5 min): return cached + refresh in background.
/// Expired (>5 min) or empty: wait for fresh fetch.
#[tauri::command]
pub async fn get_live_proactive_suggestions(
    state: State<'_, Arc<AppState>>,
    force_refresh: Option<bool>,
) -> Result<Vec<LiveProactiveSuggestion>, String> {
    let config = state
        .config
        .read()
        .map_err(|_| "Lock poisoned")?
        .clone()
        .ok_or("No configuration loaded")?;

    // Use a dedicated DB connection so this async command never holds AppState DB lock
    // across Google API awaits.
    let db = crate::db::ActionDb::open().map_err(|e| e.to_string())?;
    let (entity_hints, actions) = crate::queries::proactive::load_live_suggestion_inputs(&db)?;

    // Check cache unless force refresh requested
    if !force_refresh.unwrap_or(false) {
        if let Ok(guard) = state.calendar.week_cache.read() {
            if let Some((ref events, fetched_at)) = *guard {
                let age = fetched_at.elapsed().as_secs();
                if age < WEEK_CACHE_FRESH_SECS {
                    // Fresh: compute from cached events directly
                    return crate::queries::proactive::compute_suggestions_from_events(
                        &config, events, &actions,
                    );
                }
                if age < WEEK_CACHE_STALE_SECS {
                    // Stale but usable: compute now, trigger background refresh
                    let result = crate::queries::proactive::compute_suggestions_from_events(
                        &config, events, &actions,
                    );
                    let bg_state = state.inner().clone();
                    let bg_config = config.clone();
                    let bg_hints = entity_hints.clone();
                    tokio::spawn(async move {
                        let _ = refresh_week_calendar_cache(&bg_state, &bg_config, bg_hints).await;
                    });
                    return result;
                }
            }
        }
    }

    // Cache miss or expired: fetch, cache, and compute
    let events = refresh_week_calendar_cache(&state, &config, entity_hints).await?;
    crate::queries::proactive::compute_suggestions_from_events(&config, &events, &actions)
}

/// Fetch week calendar events from Google API and update the AppState cache.
async fn refresh_week_calendar_cache(
    state: &AppState,
    config: &crate::types::Config,
    entity_hints: Vec<crate::google_api::classify::EntityHint>,
) -> Result<Vec<CalendarEvent>, String> {
    let events = crate::queries::proactive::fetch_week_events(config, &entity_hints).await?;

    if let Ok(mut guard) = state.calendar.week_cache.write() {
        *guard = Some((events.clone(), std::time::Instant::now()));
    }

    Ok(events)
}

/// Force-refresh meeting preps for all future meetings.
///
/// Clears existing prep_frozen_json and enqueues all future meetings into the
/// MeetingPrepQueue at Manual priority. Used by the WeekPage refresh button.
#[tauri::command]
pub async fn refresh_meeting_preps(state: State<'_, Arc<AppState>>) -> Result<String, String> {
    crate::services::meetings::refresh_meeting_preps(&state).await
}

// =============================================================================
// Focus Data Command
// =============================================================================

// =============================================================================
// Actions Command
// =============================================================================

/// Get all actions with full context
#[tauri::command]
pub async fn get_all_actions(state: State<'_, Arc<AppState>>) -> Result<ActionsResult, String> {
    Ok(crate::services::actions::get_all_actions(&state).await)
}

// =============================================================================
// Inbox Command
// =============================================================================

/// Result type for inbox files
#[derive(Debug, serde::Serialize)]
#[allow(clippy::large_enum_variant)]
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
pub async fn get_inbox_files(state: State<'_, Arc<AppState>>) -> Result<InboxResult, String> {
    let config = match state.config.read() {
        Ok(guard) => match guard.clone() {
            Some(c) => c,
            None => {
                return Ok(InboxResult::Error {
                    message: "No configuration loaded".to_string(),
                    files: Vec::new(),
                    count: 0,
                })
            }
        },
        Err(_) => {
            return Ok(InboxResult::Error {
                message: "Internal error: config lock poisoned".to_string(),
                files: Vec::new(),
                count: 0,
            })
        }
    };

    let workspace = Path::new(&config.workspace_path);
    let mut files = list_inbox_files(workspace);
    let count = files.len();

    // Enrich files with persistent processing status from DB
    if let Ok(status_map) = state
        .db_read(|db| db.get_latest_processing_status().map_err(|e| e.to_string()))
        .await
    {
        for file in &mut files {
            if let Some((status, error)) = status_map.get(&file.filename) {
                file.processing_status = Some(status.clone());
                // For needs_entity, error_message stores the suggested name
                if status == "needs_entity" {
                    file.suggested_entity_name = error.clone();
                } else {
                    file.processing_error = error.clone();
                }
            }
        }
    }

    if files.is_empty() {
        Ok(InboxResult::Empty {
            message: "Inbox is clear".to_string(),
            files,
            count,
        })
    } else {
        Ok(InboxResult::Success { files, count })
    }
}

/// Process a single inbox file (classify, route, log).
///
/// Runs on a background thread to avoid blocking the main thread.
#[tauri::command]
pub async fn process_inbox_file(
    filename: String,
    entity_id: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<crate::processor::ProcessingResult, String> {
    let config = state
        .config
        .read()
        .map_err(|_| "Internal error")?
        .clone()
        .ok_or("No configuration loaded")?;

    let workspace_path = config.workspace_path.clone();
    let profile = config.profile.clone();
    let entity_id = entity_id.clone();

    // Validate filename before processing (I60: path traversal guard)
    let workspace = Path::new(&workspace_path);
    crate::util::validate_inbox_path(workspace, &filename)?;

    tauri::async_runtime::spawn_blocking(move || {
        let workspace = Path::new(&workspace_path);
        // Open a dedicated connection instead of holding the shared mutex
        // for the entire duration of process_file (which can take seconds).
        let db = crate::db::ActionDb::open().ok();
        let db_ref = db.as_ref();
        let entity_tracker_path = entity_id.as_deref().and_then(|eid| {
            db_ref
                .and_then(|db| db.get_entity(eid).ok().flatten())
                .and_then(|e| e.tracker_path)
        });
        crate::processor::process_file(
            workspace,
            &filename,
            db_ref,
            &profile,
            entity_tracker_path.as_deref(),
        )
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

    let workspace_path = config.workspace_path.clone();
    let profile = config.profile.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let workspace = Path::new(&workspace_path);
        // Open a dedicated connection instead of holding the shared mutex
        // for the entire batch processing duration.
        let db = crate::db::ActionDb::open().ok();
        crate::processor::process_all(workspace, db.as_ref(), &profile)
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
    entity_id: Option<String>,
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
    let entity_id = entity_id.clone();

    // Validate filename before enriching (I60: path traversal guard)
    let workspace = Path::new(&workspace_path);
    crate::util::validate_inbox_path(workspace, &filename)?;

    let user_ctx = crate::types::UserContext::from_config(&config);
    let ai_config = config.ai_models.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let workspace = Path::new(&workspace_path);
        let entity_tracker_path = state
            .db
            .lock()
            .ok()
            .and_then(|g| {
                g.as_ref().and_then(|db| {
                    entity_id
                        .as_deref()
                        .and_then(|eid| db.get_entity(eid).ok().flatten())
                })
            })
            .and_then(|e| e.tracker_path);
        crate::processor::enrich::enrich_file(
            workspace,
            &filename,
            Some(&state),
            &profile,
            Some(&user_ctx),
            Some(&ai_config),
            entity_tracker_path.as_deref(),
        )
    })
    .await
    .map_err(|e| format!("AI processing task failed: {}", e))
}

/// Get the content of a specific inbox file for preview
#[tauri::command]
pub fn get_inbox_file_content(
    filename: String,
    state: State<'_, Arc<AppState>>,
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
        let size = std::fs::metadata(&file_path).map(|m| m.len()).unwrap_or(0);
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
            let size = std::fs::metadata(&file_path).map(|m| m.len()).unwrap_or(0);
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
pub fn copy_to_inbox(paths: Vec<String>, state: State<'_, Arc<AppState>>) -> Result<usize, String> {
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

    // Build allowlist of source directories
    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    let allowed_source_dirs: Vec<std::path::PathBuf> = vec![
        home.join("Documents"),
        home.join("Desktop"),
        home.join("Downloads"),
    ];

    let mut copied = 0;

    for path_str in &paths {
        let source = Path::new(path_str);

        // Skip directories
        if !source.is_file() {
            continue;
        }

        // Validate source path is within allowed directories
        let canonical_source = std::fs::canonicalize(source)
            .map_err(|e| format!("Invalid source path '{}': {}", path_str, e))?;
        let source_str = canonical_source.to_string_lossy();
        let source_allowed = allowed_source_dirs.iter().any(|dir| {
            std::fs::canonicalize(dir)
                .map(|cd| source_str.starts_with(&*cd.to_string_lossy()))
                .unwrap_or(false)
        });
        if !source_allowed {
            log::warn!(
                "Rejected copy_to_inbox source outside allowed directories: {}",
                path_str
            );
            continue;
        }

        let filename = match source.file_name() {
            Some(name) => name.to_owned(),
            None => continue,
        };

        let mut dest = inbox_dir.join(&filename);

        // Handle duplicates: append (1), (2), etc.
        if dest.exists() {
            let stem = dest
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("file")
                .to_string();
            let ext = dest
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_string();
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
pub fn get_all_emails(state: State<'_, Arc<AppState>>) -> EmailsResult {
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

/// Get emails enriched with entity signals from SQLite.
///
/// Get enriched email briefing data with signals and entity threads.
#[tauri::command]
pub async fn get_emails_enriched(
    state: State<'_, Arc<AppState>>,
) -> Result<EmailBriefingData, String> {
    crate::services::emails::get_emails_enriched(&state).await
}

/// Update the entity assignment for an email (I395 — user correction).
/// Cascades to email_signals and emits a signal bus event for relevance learning.
#[tauri::command]
pub async fn update_email_entity(
    state: State<'_, Arc<AppState>>,
    email_id: String,
    entity_id: Option<String>,
    entity_type: Option<String>,
) -> Result<(), String> {
    state
        .db_write(move |db| {
            crate::services::emails::update_email_entity(
                db,
                &email_id,
                entity_id.as_deref(),
                entity_type.as_deref(),
            )
        })
        .await
}

/// Dismiss a single email signal by ID. Sets `deactivated_at` to now.
/// Emits a signal bus event for relevance learning.
#[tauri::command]
pub async fn dismiss_email_signal(
    state: State<'_, Arc<AppState>>,
    signal_id: i64,
) -> Result<(), String> {
    state
        .db_write(move |db| crate::services::emails::dismiss_email_signal(db, signal_id))
        .await
}

/// Get email sync status: last fetch time, enrichment progress, failure count (I373).
#[tauri::command]
pub async fn get_email_sync_status(
    state: State<'_, Arc<AppState>>,
) -> Result<crate::db::EmailSyncStats, String> {
    state.db_read(|db| db.get_email_sync_stats()).await
}

/// Get emails linked to a specific entity for entity detail pages (I368 AC5).
#[tauri::command]
pub async fn get_entity_emails(
    state: State<'_, Arc<AppState>>,
    entity_id: String,
    entity_type: String,
) -> Result<Vec<crate::db::DbEmail>, String> {
    state
        .db_read(move |db| crate::services::emails::get_entity_emails(db, &entity_id, &entity_type))
        .await
}

/// Refresh emails independently without re-running the full /today pipeline (I20).
#[tauri::command]
pub async fn refresh_emails(
    state: State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    crate::services::emails::refresh_emails(state.inner(), app_handle).await
}

/// Reconcile local inbox presence with Gmail inbox in lightweight mode.
/// Marks archived/removed emails resolved without running full enrichment.
#[tauri::command]
pub async fn sync_email_inbox_presence(
    state: State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<bool, String> {
    crate::services::emails::sync_email_inbox_presence(state.inner(), app_handle).await
}

/// Archive low-priority emails in Gmail and remove them from local data (I144).
#[tauri::command]
pub async fn archive_low_priority_emails(state: State<'_, Arc<AppState>>) -> Result<usize, String> {
    crate::services::emails::archive_low_priority_emails(&state).await
}

/// Set user profile (customer-success or general)
#[tauri::command]
pub fn set_profile(profile: String, state: State<'_, Arc<AppState>>) -> Result<Config, String> {
    // Validate profile value
    if profile != "customer-success" && profile != "general" {
        return Err(format!(
            "Invalid profile: {}. Must be 'customer-success' or 'general'.",
            profile
        ));
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
    state: State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<Config, String> {
    let config = crate::services::settings::set_entity_mode(&mode, &state)?;
    let _ = app_handle.emit("config-updated", ());
    Ok(config)
}

/// Set workspace path and scaffold directory structure
#[tauri::command]
pub async fn set_workspace_path(
    path: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Config, String> {
    let result = crate::services::settings::set_workspace_path(&path, &state).await;
    if result.is_ok() {
        if let Ok(mut audit) = state.audit_log.lock() {
            let category = {
                let home = dirs::home_dir().unwrap_or_default();
                let documents = home.join("Documents");
                let p = std::path::Path::new(&path);
                if p.starts_with(&documents) {
                    "documents"
                } else if p.starts_with(&home) {
                    "home"
                } else {
                    "custom"
                }
            };
            let _ = audit.append(
                "config",
                "workspace_path_changed",
                serde_json::json!({"category": category}),
            );
        }
    }
    result
}

/// Toggle developer mode with full isolation.
///
/// When enabled: switches to isolated dev database, workspace, and auth.
/// When disabled: returns to live database, workspace, and real auth.
#[tauri::command]
pub async fn set_developer_mode(
    enabled: bool,
    state: State<'_, Arc<AppState>>,
) -> Result<Config, String> {
    if enabled {
        crate::devtools::enter_dev_mode(&state)?;
    } else {
        crate::devtools::exit_dev_mode(&state)?;
    }

    // Reinitialize the async DB connection pool at the new path
    if let Err(e) = state.reinit_db_service().await {
        log::warn!("Failed to reinit db_service after dev mode toggle: {}", e);
    }

    // Return the current config (which is now either dev or live)
    let guard = state.config.read().map_err(|_| "Lock poisoned")?;
    guard
        .clone()
        .ok_or_else(|| "No configuration loaded".to_string())
}

/// Check if workspace is under iCloud sync and warning hasn't been dismissed (I464).
#[tauri::command]
pub fn check_icloud_warning(state: State<'_, Arc<AppState>>) -> Result<Option<String>, String> {
    let guard = state.config.read().map_err(|_| "Lock poisoned")?;
    let config = guard
        .clone()
        .ok_or_else(|| "No configuration loaded".to_string())?;

    if config.icloud_warning_dismissed.unwrap_or(false) {
        return Ok(None);
    }

    let workspace_path = &config.workspace_path;
    if crate::util::is_under_icloud_scope(workspace_path) {
        Ok(Some(workspace_path.clone()))
    } else {
        Ok(None)
    }
}

/// Dismiss the iCloud workspace warning permanently (I464).
#[tauri::command]
pub fn dismiss_icloud_warning(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    crate::state::create_or_update_config(&state, |config| {
        config.icloud_warning_dismissed = Some(true);
    })?;
    Ok(())
}

// =============================================================================
// App Lock (I465)
// =============================================================================

/// Get whether the app is currently locked.
#[tauri::command]
pub fn get_lock_status(state: State<'_, Arc<AppState>>) -> bool {
    state.is_locked.load(std::sync::atomic::Ordering::Relaxed)
}

/// Check if the encryption key is missing (I462 recovery screen).
#[tauri::command]
pub fn get_encryption_key_status(state: State<'_, Arc<AppState>>) -> bool {
    state
        .encryption_key_missing
        .load(std::sync::atomic::Ordering::Relaxed)
}

/// Lock the app immediately.
#[tauri::command]
pub async fn lock_app(
    state: State<'_, Arc<AppState>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    state
        .is_locked
        .store(true, std::sync::atomic::Ordering::Relaxed);
    let _ = app.emit("app-locked", ());
    Ok(())
}

/// Attempt to unlock the app via system authentication (Touch ID / password).
#[tauri::command]
pub async fn unlock_app(
    state: State<'_, Arc<AppState>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    use std::sync::atomic::Ordering;

    // Check cooldown: 30s after 3 consecutive failures
    let failed_count = state.failed_unlock_count.load(Ordering::Relaxed);
    if failed_count >= 3 {
        if let Ok(guard) = state.last_failed_unlock.lock() {
            if let Some(last) = *guard {
                if last.elapsed().as_secs() < 30 {
                    let remaining = 30 - last.elapsed().as_secs();
                    return Err(format!(
                        "Too many failed attempts. Try again in {} seconds.",
                        remaining
                    ));
                }
            }
        }
        // Cooldown expired, reset counter
        state.failed_unlock_count.store(0, Ordering::Relaxed);
    }

    // Attempt system authentication (Touch ID / password)
    match attempt_system_auth().await {
        Ok(true) => {
            state.is_locked.store(false, Ordering::Relaxed);
            state.failed_unlock_count.store(0, Ordering::Relaxed);
            // Reset activity timer so the user gets a fresh idle window
            if let Ok(mut guard) = state.last_activity.lock() {
                *guard = std::time::Instant::now();
            }
            if let Ok(mut audit) = state.audit_log.lock() {
                let _ = audit.append("security", "app_unlock_succeeded", serde_json::json!({}));
            }
            let _ = app.emit("app-unlocked", ());
            Ok(())
        }
        Ok(false) => {
            let new_count = state.failed_unlock_count.fetch_add(1, Ordering::Relaxed) + 1;
            if let Ok(mut audit) = state.audit_log.lock() {
                let _ = audit.append(
                    "security",
                    "app_unlock_failed",
                    serde_json::json!({"consecutive_failures": new_count}),
                );
            }
            if let Ok(mut guard) = state.last_failed_unlock.lock() {
                *guard = Some(std::time::Instant::now());
            }
            if new_count >= 3 {
                Err(
                    "Authentication failed. Too many attempts — please wait 30 seconds."
                        .to_string(),
                )
            } else {
                Err("Authentication failed.".to_string())
            }
        }
        Err(e) => Err(format!("Authentication error: {}", e)),
    }
}

/// Set the app lock idle timeout in minutes (None = disabled).
#[tauri::command]
pub fn set_lock_timeout(
    minutes: Option<u32>,
    state: State<'_, Arc<AppState>>,
) -> Result<Config, String> {
    if let Some(v) = minutes {
        if ![5, 15, 30].contains(&v) {
            return Err(format!(
                "Invalid lock timeout: {}. Must be 5, 15, or 30.",
                v
            ));
        }
    }
    crate::state::create_or_update_config(&state, |config| {
        config.app_lock_timeout_minutes = minutes;
    })
}

/// Signal user activity (click/keypress) to reset the idle lock timer.
#[tauri::command]
pub fn signal_user_activity(state: State<'_, Arc<AppState>>) {
    if let Ok(mut guard) = state.last_activity.lock() {
        *guard = std::time::Instant::now();
    }
}

/// Signal window focus change to reset the idle lock timer.
#[tauri::command]
pub fn signal_window_focus(focused: bool, state: State<'_, Arc<AppState>>) {
    if focused {
        if let Ok(mut guard) = state.last_activity.lock() {
            *guard = std::time::Instant::now();
        }
    }
}

/// Attempt system-level authentication using macOS LocalAuthentication framework.
/// Calls LAContext.evaluatePolicy directly via objc2 FFI so the Touch ID dialog
/// shows "DailyOS" as the requesting app (not "osascript").
#[cfg(target_os = "macos")]
async fn attempt_system_auth() -> Result<bool, String> {
    let (tx, rx) = tokio::sync::oneshot::channel::<Result<bool, String>>();

    std::thread::spawn(move || {
        use block2::RcBlock;
        use objc2::runtime::Bool;
        use objc2_foundation::{NSComparisonResult, NSDate, NSRunLoop, NSString};
        use objc2_local_authentication::{LAContext, LAPolicy};

        let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

        unsafe {
            let context = LAContext::new();
            let policy = LAPolicy(1); // deviceOwnerAuthentication (biometrics + passcode fallback)

            // Check if any authentication method is available
            if let Err(e) = context.canEvaluatePolicy_error(policy) {
                log::warn!("Biometric authentication unavailable: {e}, auto-unlocking");
                let _ = tx.send(Ok(true));
                return;
            }

            let reason = NSString::from_str("DailyOS requires authentication to unlock.");
            let done_clone = done.clone();
            let tx = std::sync::Mutex::new(Some(tx));
            let block = RcBlock::new(move |success: Bool, err: *mut objc2_foundation::NSError| {
                let result = if success.as_bool() {
                    Ok(true)
                } else if !err.is_null() {
                    let err_ref = &*err;
                    let code = err_ref.code();
                    // LAError.userCancel = -2, LAError.systemCancel = -4
                    if code == -2 || code == -4 {
                        log::info!("Touch ID cancelled (code {code})");
                        Ok(false)
                    } else {
                        let desc = err_ref.localizedDescription();
                        log::warn!("Touch ID error (code {code}): {desc}");
                        Ok(false)
                    }
                } else {
                    Ok(false)
                };
                if let Some(tx) = tx.lock().unwrap().take() {
                    let _ = tx.send(result);
                }
                done_clone.store(true, std::sync::atomic::Ordering::Release);
            });

            context.evaluatePolicy_localizedReason_reply(policy, &reason, &block);

            // Pump the run loop until the callback fires or 30s deadline
            let deadline = NSDate::dateWithTimeIntervalSinceNow(30.0);
            while !done.load(std::sync::atomic::Ordering::Acquire) {
                let step = NSDate::dateWithTimeIntervalSinceNow(0.1);
                NSRunLoop::currentRunLoop().runUntilDate(&step);
                if NSDate::date().compare(&deadline) != NSComparisonResult::Ascending {
                    log::warn!("Touch ID run loop deadline exceeded");
                    break;
                }
            }
        }
    });

    // 35s outer timeout — the thread has its own 30s deadline,
    // but if something hangs we don't want the frontend stuck forever.
    match tokio::time::timeout(std::time::Duration::from_secs(35), rx).await {
        Ok(Ok(result)) => result,
        Ok(Err(_)) => {
            log::warn!("Touch ID channel closed without result");
            Ok(false)
        }
        Err(_) => Err("Authentication timed out".to_string()),
    }
}

/// Non-macOS fallback: no biometric available, auto-unlock.
#[cfg(not(target_os = "macos"))]
async fn attempt_system_auth() -> Result<bool, String> {
    Ok(true)
}

/// Set UI personality tone (professional, friendly, playful)
#[tauri::command]
pub fn set_personality(
    personality: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Config, String> {
    let normalized = personality.to_lowercase();
    crate::types::validate_personality(&normalized)?;
    crate::state::create_or_update_config(&state, |config| {
        config.personality = normalized.clone();
    })
}

/// Set AI model for a tier (synthesis, extraction, mechanical)
#[tauri::command]
pub fn set_ai_model(
    tier: String,
    model: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Config, String> {
    crate::services::settings::set_ai_model(&tier, &model, &state)
}

/// Set hygiene configuration (I271)
#[tauri::command]
pub fn set_hygiene_config(
    scan_interval_hours: Option<u32>,
    ai_budget: Option<u32>,
    pre_meeting_hours: Option<u32>,
    state: State<'_, Arc<AppState>>,
) -> Result<Config, String> {
    crate::services::settings::set_hygiene_config(
        scan_interval_hours,
        ai_budget,
        pre_meeting_hours,
        &state,
    )
}

/// Set schedule for a workflow
#[tauri::command]
pub fn set_schedule(
    workflow: String,
    hour: u32,
    minute: u32,
    timezone: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Config, String> {
    crate::services::settings::set_schedule(&workflow, hour, minute, &timezone, &state)
}

/// Save user profile fields (name, company, title, focus, domains)
#[tauri::command]
pub async fn set_user_profile(
    name: Option<String>,
    company: Option<String>,
    title: Option<String>,
    focus: Option<String>,
    domain: Option<String>,
    domains: Option<Vec<String>>,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    crate::services::settings::set_user_profile(
        name, company, title, focus, domain, domains, &state,
    )
    .await
}

// =============================================================================
// User Entity Commands (I411 — ADR-0089/0090)
// =============================================================================

/// Get the user entity (creates from config on first call).
#[tauri::command]
pub async fn get_user_entity(
    state: State<'_, Arc<AppState>>,
) -> Result<crate::types::UserEntity, String> {
    crate::services::user_entity::get_user_entity(&state).await
}

/// Update a single field on the user entity.
#[tauri::command]
pub async fn update_user_entity_field(
    field: String,
    value: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    crate::services::user_entity::update_user_entity_field(&field, &value, &state).await
}

/// Get all user context entries.
#[tauri::command]
pub async fn get_user_context_entries(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::types::UserContextEntry>, String> {
    crate::services::user_entity::get_user_context_entries(&state).await
}

/// Create a new user context entry.
#[tauri::command]
pub async fn create_user_context_entry(
    title: String,
    content: String,
    state: State<'_, Arc<AppState>>,
) -> Result<crate::types::UserContextEntry, String> {
    crate::services::user_entity::create_user_context_entry(&title, &content, &state).await
}

/// Update an existing user context entry.
#[tauri::command]
pub async fn update_user_context_entry(
    id: String,
    title: String,
    content: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    crate::services::user_entity::update_user_context_entry(&id, &title, &content, &state).await
}

/// Delete a user context entry.
#[tauri::command]
pub async fn delete_user_context_entry(
    id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    crate::services::user_entity::delete_user_context_entry(&id, &state).await
}

/// Get all entity context entries for an entity.
#[tauri::command]
pub async fn get_entity_context_entries(
    entity_type: String,
    entity_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::types::EntityContextEntry>, String> {
    crate::services::entity_context::get_entries(&entity_type, &entity_id, &state).await
}

/// Create a new entity context entry.
#[tauri::command]
pub async fn create_entity_context_entry(
    entity_type: String,
    entity_id: String,
    title: String,
    content: String,
    state: State<'_, Arc<AppState>>,
) -> Result<crate::types::EntityContextEntry, String> {
    crate::services::entity_context::create_entry(
        &entity_type,
        &entity_id,
        &title,
        &content,
        &state,
    )
    .await
}

/// Update an existing entity context entry.
#[tauri::command]
pub async fn update_entity_context_entry(
    id: String,
    title: String,
    content: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    crate::services::entity_context::update_entry(&id, &title, &content, &state).await
}

/// Delete an entity context entry.
#[tauri::command]
pub async fn delete_entity_context_entry(
    id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    crate::services::entity_context::delete_entry(&id, &state).await
}

/// Process a user attachment from the /me page dropzone.
///
/// Copies the file into _user/attachments/ (if not already there), processes it
/// through the file processor pipeline, and indexes it as user_context content.
#[tauri::command]
pub async fn process_user_attachment(
    path: String,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let config = state
        .config
        .read()
        .map_err(|_| "Internal error")?
        .clone()
        .ok_or("No configuration loaded")?;

    let workspace = std::path::Path::new(&config.workspace_path);
    let attachments_dir = workspace.join("_user").join("attachments");

    // Ensure _user/attachments/ exists
    if !attachments_dir.exists() {
        std::fs::create_dir_all(&attachments_dir)
            .map_err(|e| format!("Failed to create _user/attachments: {}", e))?;
    }

    let source = std::path::Path::new(&path);
    if !source.is_file() {
        return Err(format!("Not a file: {}", path));
    }

    // Determine final path in _user/attachments/
    let filename = source.file_name().ok_or("Invalid filename")?;
    let dest = attachments_dir.join(filename);

    // Copy if not already in _user/attachments/
    let final_path = if source.starts_with(&attachments_dir) {
        source.to_path_buf()
    } else {
        // Handle duplicates
        let final_dest = if dest.exists() {
            let stem = dest.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
            let ext = dest.extension().and_then(|e| e.to_str()).unwrap_or("");
            let mut candidate = dest.clone();
            for i in 1..1000 {
                candidate = if ext.is_empty() {
                    attachments_dir.join(format!("{}-{}", stem, i))
                } else {
                    attachments_dir.join(format!("{}-{}.{}", stem, i, ext))
                };
                if !candidate.exists() {
                    break;
                }
            }
            candidate
        } else {
            dest
        };

        std::fs::copy(source, &final_dest).map_err(|e| format!("Failed to copy file: {}", e))?;
        final_dest
    };

    // Process through the pipeline
    let state_inner = state.inner().clone();
    let workspace_owned = workspace.to_path_buf();
    let final_path_owned = final_path.clone();

    let result = tokio::task::spawn_blocking(move || {
        let db = crate::db::ActionDb::open().ok();
        let result = crate::processor::process_user_attachment(
            &workspace_owned,
            &final_path_owned,
            db.as_ref(),
        );

        // Queue embedding generation
        if matches!(result, crate::processor::ProcessingResult::Routed { .. }) {
            state_inner
                .embedding_queue
                .enqueue(crate::processor::embeddings::EmbeddingRequest {
                    entity_id: "user_context".to_string(),
                    entity_type: "user_context".to_string(),
                    requested_at: std::time::Instant::now(),
                });
        }

        result
    })
    .await
    .map_err(|e| format!("Processing failed: {}", e))?;

    match result {
        crate::processor::ProcessingResult::Routed { destination, .. } => Ok(destination),
        crate::processor::ProcessingResult::Error { message } => Err(message),
        crate::processor::ProcessingResult::NeedsEnrichment
        | crate::processor::ProcessingResult::NeedsEntity { .. } => {
            Ok(final_path.display().to_string())
        }
    }
}

/// List available meeting prep files
#[tauri::command]
pub fn list_meeting_preps(state: State<'_, Arc<AppState>>) -> Result<Vec<String>, String> {
    crate::services::meetings::list_meeting_preps(&state)
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
pub async fn get_actions_from_db(
    days_ahead: Option<i32>,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<ActionListItem>, String> {
    let days = days_ahead.unwrap_or(7);
    state
        .db_read(move |db| crate::services::actions::get_actions_from_db(db, days))
        .await
}

/// Mark an action as completed in the SQLite database.
///
/// Sets `status = 'completed'` and `completed_at` to the current UTC timestamp.
#[tauri::command]
pub async fn complete_action(id: String, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let engine = state.signals.engine.clone();
    state
        .db_write(move |db| crate::services::actions::complete_action(db, &engine, &id))
        .await
}

/// Reopen a completed action, setting it back to pending.
#[tauri::command]
pub async fn reopen_action(id: String, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let engine = state.signals.engine.clone();
    state
        .db_write(move |db| crate::services::actions::reopen_action(db, &engine, &id))
        .await
}

/// Accept a proposed action, moving it to pending (I256).
#[tauri::command]
pub async fn accept_proposed_action(
    id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let engine = state.signals.engine.clone();
    state
        .db_write(move |db| crate::services::actions::accept_proposed_action(db, &engine, &id))
        .await
}

/// Reject a proposed action by archiving it (I256).
#[tauri::command]
pub async fn reject_proposed_action(
    id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let engine = state.signals.engine.clone();
    state
        .db_write(move |db| crate::services::actions::reject_proposed_action(db, &engine, &id))
        .await
}

/// Dismiss an email-extracted item (commitment, question, reply_needed) from
/// The Correspondent. Records the dismissal in SQLite for relevance learning.
#[tauri::command]
pub async fn dismiss_email_item(
    item_type: String,
    email_id: String,
    item_text: String,
    sender_domain: Option<String>,
    email_type: Option<String>,
    entity_id: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    state
        .db_write(move |db| {
            crate::services::emails::dismiss_email_item(
                db,
                &item_type,
                &email_id,
                &item_text,
                sender_domain.as_deref(),
                email_type.as_deref(),
                entity_id.as_deref(),
            )
        })
        .await
}

/// Get all dismissed email item keys for frontend filtering.
#[tauri::command]
pub async fn list_dismissed_email_items(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<String>, String> {
    state
        .db_read(|db| {
            db.list_dismissed_email_items()
                .map(|set| set.into_iter().collect())
                .map_err(|e| e.to_string())
        })
        .await
}

/// Reset all email dismissal learning data (I374).
/// Truncates the email_dismissals table so classification starts fresh.
#[tauri::command]
pub async fn reset_email_preferences(
    services: State<'_, crate::services::ServiceLayer>,
) -> Result<String, String> {
    let state = services.state();
    state
        .db_write(|db| {
            let count = crate::services::mutations::reset_email_dismissals(db)?;
            log::info!(
                "reset_email_preferences: cleared {} dismissal records",
                count
            );
            Ok(format!("Cleared {} email dismissal records", count))
        })
        .await
}

/// Get all proposed (AI-suggested) actions (I256).
#[tauri::command]
pub async fn get_proposed_actions(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::db::DbAction>, String> {
    state
        .db_read(crate::services::actions::get_proposed_actions)
        .await
}

/// Get recent meeting history for an account from the SQLite database.
///
/// Returns meetings within `lookback_days` (default 30), limited to `limit` results (default 3).
#[tauri::command]
pub async fn get_meeting_history(
    account_id: String,
    lookback_days: Option<i32>,
    limit: Option<i32>,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::db::DbMeeting>, String> {
    let days = lookback_days.unwrap_or(30);
    let lim = limit.unwrap_or(3);
    state
        .db_read(move |db| {
            db.get_meeting_history(&account_id, days, lim)
                .map_err(|e| e.to_string())
        })
        .await
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
    /// Parsed prep context from enrichment (I181).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prep_context: Option<PrepContext>,
}

/// Enriched pre-meeting prep context persisted from daily briefing (I181).
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepContext {
    pub intelligence_summary: Option<String>,
    pub entity_risks: Option<Vec<serde_json::Value>>,
    pub entity_readiness: Option<Vec<String>>,
    pub talking_points: Option<Vec<String>>,
    pub recent_wins: Option<Vec<String>>,
    pub recent_win_sources: Option<Vec<SourceReference>>,
    pub proposed_agenda: Option<Vec<serde_json::Value>>,
    pub open_items: Option<Vec<serde_json::Value>>,
    pub questions: Option<Vec<String>>,
    pub stakeholder_insights: Option<Vec<serde_json::Value>>,
}

/// Get full detail for a single past meeting by ID.
///
/// Assembles the meeting row, its captures, actions, and resolves the account name.
#[tauri::command]
pub async fn get_meeting_history_detail(
    meeting_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<MeetingHistoryDetail, String> {
    crate::services::meetings::get_meeting_history_detail(&meeting_id, &state).await
}

// =============================================================================
// Meeting Search (I183)
// =============================================================================

/// A meeting search result with minimal metadata.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingSearchResult {
    pub id: String,
    pub title: String,
    pub meeting_type: String,
    pub start_time: String,
    pub account_name: Option<String>,
    pub match_snippet: Option<String>,
}

/// Search meetings by title, summary, or prep context (I183).
#[tauri::command]
pub async fn search_meetings(
    query: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<MeetingSearchResult>, String> {
    crate::services::meetings::search_meetings(&query, &state).await
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
pub async fn get_action_detail(
    action_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<ActionDetail, String> {
    state
        .db_read(move |db| crate::services::actions::get_action_detail(db, &action_id))
        .await
}

// =============================================================================
// Phase 3.0: Google Auth Commands
// =============================================================================

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct GoogleAuthFailedPayload {
    message: String,
}

/// Get current Google authentication status.
///
/// Re-checks persisted auth storage when cached state is NotConfigured,
/// so the UI picks up credentials written by external flows.
#[tauri::command]
pub fn get_google_auth_status(state: State<'_, Arc<AppState>>) -> GoogleAuthStatus {
    let started = std::time::Instant::now();

    // Dev override: return mocked Google auth status
    if cfg!(debug_assertions) {
        let ov = DEV_GOOGLE_OVERRIDE.load(Ordering::Relaxed);
        if ov != 0 {
            log_command_latency(
                "get_google_auth_status",
                started,
                READ_CMD_LATENCY_BUDGET_MS,
            );
            return match ov {
                1 => GoogleAuthStatus::Authenticated {
                    email: "dev@dailyos.test".to_string(),
                },
                3 => GoogleAuthStatus::TokenExpired,
                _ => GoogleAuthStatus::NotConfigured,
            };
        }
    }

    let cached = state
        .calendar
        .google_auth
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or(GoogleAuthStatus::NotConfigured);

    // If cached state says not configured, re-check storage — token may have
    // been written by a script or the browser auth flow completing late.
    if matches!(cached, GoogleAuthStatus::NotConfigured) {
        let fresh = crate::state::detect_google_auth();
        if matches!(fresh, GoogleAuthStatus::Authenticated { .. }) {
            if let Ok(mut guard) = state.calendar.google_auth.lock() {
                *guard = fresh.clone();
            }
            log_command_latency(
                "get_google_auth_status",
                started,
                READ_CMD_LATENCY_BUDGET_MS,
            );
            return fresh;
        }
    }

    log_command_latency(
        "get_google_auth_status",
        started,
        READ_CMD_LATENCY_BUDGET_MS,
    );
    cached
}

/// Start Google OAuth flow
#[tauri::command]
pub async fn start_google_auth(
    state: State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<GoogleAuthStatus, String> {
    // Dev override: skip real OAuth flow when auth is mocked
    if cfg!(debug_assertions) {
        let ov = DEV_GOOGLE_OVERRIDE.load(Ordering::Relaxed);
        match ov {
            1 => {
                let status = GoogleAuthStatus::Authenticated {
                    email: "dev@dailyos.test".to_string(),
                };
                if let Ok(mut guard) = state.calendar.google_auth.lock() {
                    *guard = status.clone();
                }
                return Ok(status);
            }
            2 => return Err("Google not configured (dev override)".into()),
            3 => return Err("Google token expired (dev override)".into()),
            _ => {} // 0 = real flow
        }
    }

    let config = state
        .config
        .read()
        .map_err(|_| "Lock poisoned")?
        .clone()
        .ok_or("No configuration loaded")?;

    let workspace_path = config.workspace_path.clone();

    // Run the native Rust OAuth flow
    let workspace = std::path::Path::new(&workspace_path);
    let email = match crate::google::start_auth(workspace).await {
        Ok(email) => email,
        Err(err) => {
            let message = err.to_string();
            let _ = app_handle.emit(
                "google-auth-failed",
                GoogleAuthFailedPayload {
                    message: message.clone(),
                },
            );
            return Err(message);
        }
    };

    let new_status = GoogleAuthStatus::Authenticated {
        email: email.clone(),
    };

    // Audit: oauth_connected
    if let Ok(mut audit) = state.audit_log.lock() {
        let _ = audit.append(
            "security",
            "oauth_connected",
            serde_json::json!({"provider": "google"}),
        );
    }

    // Update state
    if let Ok(mut guard) = state.calendar.google_auth.lock() {
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
    state: State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    crate::google::disconnect()?;

    let purge_report = state.with_db_write(|db| {
        crate::db::data_lifecycle::purge_source(db, crate::db::data_lifecycle::DataSource::Google)
            .map_err(|e| e.to_string())
    })?;

    // Audit: oauth_revoked
    if let Ok(mut audit) = state.audit_log.lock() {
        let _ = audit.append(
            "security",
            "oauth_revoked",
            serde_json::json!({"provider": "google", "purge": purge_report}),
        );
    }

    let new_status = GoogleAuthStatus::NotConfigured;

    // Update state
    if let Ok(mut guard) = state.calendar.google_auth.lock() {
        *guard = new_status.clone();
    }

    // Clear calendar events
    if let Ok(mut guard) = state.calendar.events.write() {
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
pub fn get_calendar_events(state: State<'_, Arc<AppState>>) -> Vec<CalendarEvent> {
    state
        .calendar
        .events
        .read()
        .map(|guard| guard.clone())
        .unwrap_or_default()
}

/// Get the currently active meeting (if any)
#[tauri::command]
pub fn get_current_meeting(state: State<'_, Arc<AppState>>) -> Option<CalendarEvent> {
    let now = chrono::Utc::now();
    state.calendar.events.read().ok().and_then(|guard| {
        guard
            .iter()
            .find(|e| e.start <= now && e.end > now && !e.is_all_day)
            .cloned()
    })
}

/// Get the next upcoming meeting
#[tauri::command]
pub fn get_next_meeting(state: State<'_, Arc<AppState>>) -> Option<CalendarEvent> {
    let now = chrono::Utc::now();
    state.calendar.events.read().ok().and_then(|guard| {
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
pub async fn capture_meeting_outcome(
    outcome: CapturedOutcome,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    crate::services::meetings::capture_meeting_outcome(&outcome, &state).await
}

/// Dismiss a post-meeting capture prompt (skip)
#[tauri::command]
pub fn dismiss_meeting_prompt(
    meeting_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    if let Ok(mut guard) = state.capture.dismissed.lock() {
        guard.insert(meeting_id);
    }
    Ok(())
}

/// Get post-meeting capture settings
#[tauri::command]
pub fn get_capture_settings(state: State<'_, Arc<AppState>>) -> PostMeetingCaptureConfig {
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
pub fn set_capture_enabled(enabled: bool, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    crate::state::create_or_update_config(&state, |config| {
        config.post_meeting_capture.enabled = enabled;
    })?;
    Ok(())
}

/// Set post-meeting capture delay (minutes before prompt appears)
#[tauri::command]
pub fn set_capture_delay(
    delay_minutes: u32,
    state: State<'_, Arc<AppState>>,
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
    crate::services::meetings::attach_meeting_transcript(
        file_path,
        meeting,
        state.inner(),
        app_handle,
    )
    .await
}

/// Get meeting outcomes (from transcript processing or manual capture).
///
/// Returns `None` only when no outcomes/transcript metadata exist in SQLite.
#[tauri::command]
pub async fn get_meeting_outcomes(
    meeting_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Option<crate::types::MeetingOutcomeData>, String> {
    state
        .db_read(move |db| {
            let meeting = db
                .get_meeting_intelligence_row(&meeting_id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("Meeting not found: {}", meeting_id))?;
            Ok(collect_meeting_outcomes_from_db(db, &meeting))
        })
        .await
}

/// Update the content of a capture (win/risk/decision) — I45 inline editing.
#[tauri::command]
pub async fn update_capture(
    id: String,
    content: String,
    services: State<'_, crate::services::ServiceLayer>,
) -> Result<(), String> {
    let state = services.state();
    state
        .db_write(move |db| crate::services::mutations::update_capture_content(db, &id, &content))
        .await
}

/// Cycle an action's priority (P1→P2→P3→P1) — I45 interaction.
#[tauri::command]
pub async fn update_action_priority(
    id: String,
    priority: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    // Validate priority
    if !matches!(priority.as_str(), "P1" | "P2" | "P3") {
        return Err(format!(
            "Invalid priority: {}. Must be P1, P2, or P3.",
            priority
        ));
    }
    let engine = state.signals.engine.clone();
    state
        .db_write(move |db| {
            crate::services::actions::update_action_priority(db, &engine, &id, &priority)
        })
        .await
}

// =============================================================================
// Manual Action CRUD (I127 / I128)
// =============================================================================

/// Create a new action manually (not from briefing/transcript/inbox).
///
/// Returns the new action's UUID. Priority defaults to P2 if not provided.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateActionRequest {
    pub title: String,
    pub priority: Option<String>,
    pub due_date: Option<String>,
    pub account_id: Option<String>,
    pub project_id: Option<String>,
    pub person_id: Option<String>,
    pub context: Option<String>,
    pub source_label: Option<String>,
}

#[tauri::command]
pub async fn create_action(
    request: CreateActionRequest,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    crate::services::actions::create_action(request, &state).await
}

/// Update arbitrary fields on an existing action (I128).
///
/// Only provided fields are updated; `None` means "don't touch".
/// To clear a nullable field, pass the corresponding `clear_*` flag.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateActionRequest {
    pub id: String,
    pub title: Option<String>,
    pub due_date: Option<String>,
    pub clear_due_date: Option<bool>,
    pub context: Option<String>,
    pub clear_context: Option<bool>,
    pub source_label: Option<String>,
    pub clear_source_label: Option<bool>,
    pub account_id: Option<String>,
    pub clear_account: Option<bool>,
    pub project_id: Option<String>,
    pub clear_project: Option<bool>,
    pub person_id: Option<String>,
    pub clear_person: Option<bool>,
    pub priority: Option<String>,
}

#[tauri::command]
pub async fn update_action(
    request: UpdateActionRequest,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    crate::services::actions::update_action(request, &state).await
}

// =============================================================================
// Processing History (I6)
// =============================================================================

/// Get processing history from the SQLite database.
///
/// Returns recent inbox processing log entries for the History page.
#[tauri::command]
pub async fn get_processing_history(
    limit: Option<i32>,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::db::DbProcessingLog>, String> {
    let lim = limit.unwrap_or(50);
    state
        .db_read(move |db| db.get_processing_log(lim).map_err(|e| e.to_string()))
        .await
}

// =============================================================================
// Onboarding: Demo Data
// =============================================================================

/// Install demo data for first-run experience (I56).
///
/// Seeds curated accounts, actions, meetings, and people marked `is_demo = 1`.
/// Writes fixture files if a workspace path is configured.
#[tauri::command]
pub async fn install_demo_data(state: State<'_, Arc<AppState>>) -> Result<String, String> {
    let workspace_path = state
        .config
        .read()
        .map_err(|_| "Config lock failed")?
        .as_ref()
        .and_then(|c| {
            if c.workspace_path.is_empty() {
                None
            } else {
                Some(c.workspace_path.clone())
            }
        });

    let ws = workspace_path.clone();
    state
        .db_write(move |db| crate::demo::install_demo(db, ws.as_deref().map(std::path::Path::new)))
        .await?;

    Ok("Demo data installed".into())
}

/// Clear all demo data and reset demo mode (I56).
#[tauri::command]
pub async fn clear_demo_data(state: State<'_, Arc<AppState>>) -> Result<String, String> {
    let workspace_path = state
        .config
        .read()
        .map_err(|_| "Config lock failed")?
        .as_ref()
        .and_then(|c| {
            if c.workspace_path.is_empty() {
                None
            } else {
                Some(c.workspace_path.clone())
            }
        });

    let ws = workspace_path.clone();
    state
        .db_write(move |db| crate::demo::clear_demo(db, ws.as_deref().map(std::path::Path::new)))
        .await?;

    Ok("Demo data cleared".into())
}

/// Get app-level state (demo mode, tour, wizard progress).
#[tauri::command]
pub async fn get_app_state(
    state: State<'_, Arc<AppState>>,
) -> Result<crate::demo::AppStateRow, String> {
    state.db_read(crate::demo::get_app_state).await
}

/// Mark the post-wizard tour as completed.
#[tauri::command]
pub async fn set_tour_completed(state: State<'_, Arc<AppState>>) -> Result<String, String> {
    state.db_write(crate::demo::set_tour_completed).await?;
    Ok("Tour completed".into())
}

/// Mark the wizard as completed with current timestamp.
#[tauri::command]
pub async fn set_wizard_completed(state: State<'_, Arc<AppState>>) -> Result<String, String> {
    state.db_write(crate::demo::set_wizard_completed).await?;
    Ok("Wizard completed".into())
}

/// Set wizard last step for mid-wizard resume.
#[tauri::command]
pub async fn set_wizard_step(
    step: String,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    state
        .db_write(move |db| crate::demo::set_wizard_step(db, &step))
        .await?;
    Ok("Wizard step saved".into())
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
pub async fn populate_workspace(
    accounts: Vec<String>,
    projects: Vec<String>,
    state: State<'_, Arc<AppState>>,
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

    // 3. Process accounts: filesystem first, collect valid names
    let mut valid_account_names: Vec<String> = Vec::new();
    for name in &accounts {
        let name = match crate::util::validate_entity_name(name) {
            Ok(n) => n,
            Err(e) => {
                log::warn!("Skipping invalid account name '{}': {}", name, e);
                continue;
            }
        };

        // Create folder + directory template (ADR-0059, idempotent)
        let account_dir = workspace.join("Accounts").join(name);
        if let Err(e) = std::fs::create_dir_all(&account_dir) {
            log::warn!("Failed to create account dir '{}': {}", name, e);
            continue;
        }
        if let Err(e) = crate::util::bootstrap_entity_directory(&account_dir, name, "account") {
            log::warn!("Failed to bootstrap account template '{}': {}", name, e);
        }
        valid_account_names.push(name.to_string());
    }
    let account_count = valid_account_names.len();

    // 4. Process projects: filesystem first, collect valid entries
    let mut valid_projects: Vec<crate::db::DbProject> = Vec::new();
    for name in &projects {
        let name = match crate::util::validate_entity_name(name) {
            Ok(n) => n,
            Err(e) => {
                log::warn!("Skipping invalid project name '{}': {}", name, e);
                continue;
            }
        };

        // Create folder + directory template (ADR-0059, idempotent)
        let project_dir = workspace.join("Projects").join(name);
        if let Err(e) = std::fs::create_dir_all(&project_dir) {
            log::warn!("Failed to create project dir '{}': {}", name, e);
        }
        if let Err(e) = crate::util::bootstrap_entity_directory(&project_dir, name, "project") {
            log::warn!("Failed to bootstrap project template '{}': {}", name, e);
        }

        let slug = crate::util::slugify(name);
        valid_projects.push(crate::db::DbProject {
            id: slug,
            name: name.to_string(),
            status: "active".to_string(),
            milestone: None,
            owner: None,
            target_date: None,
            tracker_path: Some(format!("Projects/{}", name)),
            parent_id: None,
            updated_at: now.clone(),
            archived: false,
            keywords: None,
            keywords_extracted_at: None,
            metadata: None,
        });
    }
    let project_count = valid_projects.len();

    // Batch DB operations
    let engine = std::sync::Arc::clone(&state.signals.engine);
    let wp = workspace_path.clone();
    let _ = state
        .db_write(move |db| {
            let workspace = std::path::Path::new(&wp);
            // Upsert accounts
            for name in &valid_account_names {
                let slug = crate::util::slugify(name);
                let existing = db.get_account(&slug).ok().flatten();
                let db_account = crate::db::DbAccount {
                    id: slug,
                    name: name.to_string(),
                    lifecycle: existing.as_ref().and_then(|e| e.lifecycle.clone()),
                    arr: existing.as_ref().and_then(|e| e.arr),
                    health: existing.as_ref().and_then(|e| e.health.clone()),
                    contract_start: existing.as_ref().and_then(|e| e.contract_start.clone()),
                    contract_end: existing.as_ref().and_then(|e| e.contract_end.clone()),
                    nps: existing.as_ref().and_then(|e| e.nps),
                    tracker_path: Some(format!("Accounts/{}", name)),
                    parent_id: existing.as_ref().and_then(|e| e.parent_id.clone()),
                    account_type: existing
                        .as_ref()
                        .map(|e| e.account_type.clone())
                        .unwrap_or(crate::db::AccountType::Customer),
                    updated_at: now.clone(),
                    archived: existing.as_ref().map(|e| e.archived).unwrap_or(false),
                    keywords: existing.as_ref().and_then(|e| e.keywords.clone()),
                    keywords_extracted_at: existing
                        .as_ref()
                        .and_then(|e| e.keywords_extracted_at.clone()),
                    metadata: existing.as_ref().and_then(|e| e.metadata.clone()),
                };
                if let Err(e) = crate::services::mutations::upsert_account(db, &engine, &db_account)
                {
                    log::warn!("Failed to upsert account '{}': {}", name, e);
                }
            }
            // Upsert projects + write dashboard files
            for db_project in &valid_projects {
                if let Err(e) = crate::services::mutations::upsert_project(db, &engine, db_project)
                {
                    log::warn!("Failed to upsert project '{}': {}", db_project.name, e);
                }
                let json = crate::projects::default_project_json(db_project);
                let _ = crate::projects::write_project_json(workspace, db_project, Some(&json), db);
                let _ =
                    crate::projects::write_project_markdown(workspace, db_project, Some(&json), db);
            }
            Ok(())
        })
        .await;

    Ok(format!(
        "Created {} accounts, {} projects",
        account_count, project_count
    ))
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingPrimingCard {
    pub id: String,
    pub title: String,
    pub start_time: Option<String>,
    pub day_label: String,
    pub suggested_entity_id: Option<String>,
    pub suggested_entity_name: Option<String>,
    pub suggested_action: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingPrimingContext {
    pub google_connected: bool,
    pub cards: Vec<OnboardingPrimingCard>,
    pub prompt: String,
}

#[tauri::command]
pub async fn get_onboarding_priming_context(
    state: State<'_, Arc<AppState>>,
) -> Result<OnboardingPrimingContext, String> {
    let config = state
        .config
        .read()
        .map_err(|_| "Lock poisoned")?
        .clone()
        .ok_or("Config not loaded")?;
    let user_domains = config.resolved_user_domains();

    let access_token = match crate::google_api::get_valid_access_token().await {
        Ok(token) => token,
        Err(_) => {
            return Ok(OnboardingPrimingContext {
                google_connected: false,
                cards: Vec::new(),
                prompt: "Connect Google Calendar to preview your first full briefing. You can still generate a first run now."
                    .to_string(),
            })
        }
    };

    let today = chrono::Local::now().date_naive();
    let tomorrow = today + chrono::Duration::days(1);
    let raw_events = crate::google_api::calendar::fetch_events(&access_token, today, tomorrow)
        .await
        .map_err(|e| format!("Calendar fetch failed: {}", e))?;

    let (hints, internal_root) = state
        .db_read(|db| {
            Ok((
                crate::helpers::build_entity_hints(db),
                db.get_internal_root_account().ok().flatten(),
            ))
        })
        .await?;

    // Pre-classify all meetings and collect account hints for batch DB lookup
    let mut classified: Vec<(
        crate::google_api::classify::ClassifiedMeeting,
        crate::types::CalendarEvent,
        String,
        Option<String>,
    )> = Vec::new();
    for raw in raw_events.iter().filter(|e| !e.is_all_day).take(8) {
        let cm = crate::google_api::classify::classify_meeting_multi(raw, &user_domains, &hints);
        let event = cm.to_calendar_event();
        let start = event.start.with_timezone(&chrono::Local);
        let day_label = if start.date_naive() == today {
            "Today".to_string()
        } else if start.date_naive() == tomorrow {
            "Tomorrow".to_string()
        } else {
            start.format("%a").to_string()
        };
        let account_hint = cm.account().map(|s| s.to_string());
        classified.push((cm, event, day_label, account_hint));
    }

    // Batch-resolve account hints in a single DB read
    let account_hints: Vec<Option<String>> =
        classified.iter().map(|(_, _, _, h)| h.clone()).collect();
    let resolved_accounts = state
        .db_read(move |db| {
            let mut results = Vec::new();
            for hint in &account_hints {
                if let Some(ref name) = hint {
                    if let Ok(Some(account)) = db.get_account_by_name(name) {
                        results.push(Some((account.id.clone(), account.name.clone())));
                    } else {
                        results.push(None);
                    }
                } else {
                    results.push(None);
                }
            }
            Ok(results)
        })
        .await?;

    let mut cards = Vec::new();
    for (i, (_cm, event, day_label, _account_hint)) in classified.into_iter().enumerate() {
        let mut suggested_entity_id = None;
        let mut suggested_entity_name = None;

        if let Some(Some((ref id, ref name))) = resolved_accounts.get(i) {
            suggested_entity_id = Some(id.clone());
            suggested_entity_name = Some(name.clone());
        } else if matches!(
            event.meeting_type,
            crate::types::MeetingType::Internal
                | crate::types::MeetingType::TeamSync
                | crate::types::MeetingType::OneOnOne
        ) {
            if let Some(ref root) = internal_root {
                suggested_entity_id = Some(root.id.clone());
                suggested_entity_name = Some(root.name.clone());
            }
        }

        let suggested_action = match event.meeting_type {
            crate::types::MeetingType::Customer
            | crate::types::MeetingType::Qbr
            | crate::types::MeetingType::Partnership => {
                "Open context and prep follow-up questions".to_string()
            }
            crate::types::MeetingType::Internal
            | crate::types::MeetingType::TeamSync
            | crate::types::MeetingType::OneOnOne => {
                "Capture decisions and owners in Inbox".to_string()
            }
            _ => "Review context before kickoff".to_string(),
        };

        cards.push(OnboardingPrimingCard {
            id: event.id,
            title: event.title,
            start_time: Some(event.start.with_timezone(&chrono::Local).to_rfc3339()),
            day_label,
            suggested_entity_id,
            suggested_entity_name,
            suggested_action,
        });
    }

    Ok(OnboardingPrimingContext {
        google_connected: true,
        cards,
        prompt:
            "Prime your first briefing by reviewing high-priority meetings and running a full 'today' workflow preview."
                .to_string(),
    })
}

// =============================================================================
// Onboarding: Claude Code Status (I79)
// =============================================================================

/// Check whether Claude Code CLI is installed and authenticated.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeStatus {
    pub installed: bool,
    pub authenticated: bool,
    pub node_installed: bool,
}

#[derive(Debug, Clone)]
struct ClaudeStatusCacheEntry {
    status: ClaudeStatus,
    checked_at: std::time::Instant,
}

static CLAUDE_STATUS_CACHE: OnceLock<Mutex<Option<ClaudeStatusCacheEntry>>> = OnceLock::new();

fn claude_status_cache() -> &'static Mutex<Option<ClaudeStatusCacheEntry>> {
    CLAUDE_STATUS_CACHE.get_or_init(|| Mutex::new(None))
}

/// Return in-memory command latency rollups for diagnostics/devtools.
#[tauri::command]
pub fn get_latency_rollups() -> crate::latency::LatencyRollupsPayload {
    crate::latency::get_rollups()
}

/// Cache Claude status checks to avoid shelling out on every focus event.
///
/// The subprocess spawn (`claude --print hello`) runs on a blocking thread
/// via `spawn_blocking` so it never ties up a Tauri IPC thread.
#[tauri::command]
pub async fn check_claude_status() -> ClaudeStatus {
    let started = std::time::Instant::now();

    // Dev override: return mocked status without spawning subprocess
    if cfg!(debug_assertions) {
        let ov = DEV_CLAUDE_OVERRIDE.load(Ordering::Relaxed);
        if ov != 0 {
            log_command_latency("check_claude_status", started, READ_CMD_LATENCY_BUDGET_MS);
            return match ov {
                1 => ClaudeStatus {
                    installed: true,
                    authenticated: true,
                    node_installed: true,
                },
                2 => ClaudeStatus {
                    installed: false,
                    authenticated: false,
                    node_installed: false,
                },
                3 => ClaudeStatus {
                    installed: true,
                    authenticated: false,
                    node_installed: true,
                },
                _ => ClaudeStatus {
                    installed: false,
                    authenticated: false,
                    node_installed: false,
                },
            };
        }
    }

    let cache = claude_status_cache();
    let ttl = std::time::Duration::from_secs(CLAUDE_STATUS_CACHE_TTL_SECS);

    // Fast path: return cached result without blocking
    if let Ok(guard) = cache.lock() {
        if let Some(entry) = guard.as_ref() {
            if entry.checked_at.elapsed() < ttl {
                log_command_latency("check_claude_status", started, READ_CMD_LATENCY_BUDGET_MS);
                return entry.status.clone();
            }
        }
    }

    // Slow path: spawn subprocess on a blocking thread so IPC stays responsive
    let status = tokio::task::spawn_blocking(|| {
        let installed = crate::pty::PtyManager::is_claude_available();
        let authenticated = if installed {
            crate::pty::PtyManager::is_claude_authenticated().unwrap_or(false)
        } else {
            false
        };
        let node_installed = crate::util::resolve_node_binary().is_some();
        ClaudeStatus {
            installed,
            authenticated,
            node_installed,
        }
    })
    .await
    .unwrap_or(ClaudeStatus {
        installed: false,
        authenticated: false,
        node_installed: false,
    });

    if let Ok(mut guard) = cache.lock() {
        *guard = Some(ClaudeStatusCacheEntry {
            status: status.clone(),
            checked_at: std::time::Instant::now(),
        });
    }

    log_command_latency("check_claude_status", started, READ_CMD_LATENCY_BUDGET_MS);
    status
}

/// Open the Claude sign-in page in the user's default browser.
///
/// Claude Code stores credentials in the macOS Keychain after OAuth completes
/// on the website. After the user signs in, clicking "Check again" will pick
/// up the new keychain entry.
///
/// Also clears the status cache so the next `check_claude_status` call
/// performs a fresh probe.
#[tauri::command]
pub fn launch_claude_login() -> Result<(), String> {
    // Clear cached status so the next check returns a fresh result.
    if let Ok(mut guard) = claude_status_cache().lock() {
        *guard = None;
    }

    open::that("https://claude.ai/login").map_err(|e| e.to_string())
}

/// Clear the Claude status TTL cache so the next `check_claude_status` call
/// performs a fresh probe. Called by the "Re-check" button in onboarding so
/// installing Node/Claude while the app is running is detected immediately.
#[tauri::command]
pub fn clear_claude_status_cache() {
    if let Ok(mut guard) = claude_status_cache().lock() {
        *guard = None;
    }
}

// =============================================================================
// Onboarding: Inbox Training Sample (I78)
// =============================================================================

/// Copy a bundled sample meeting notes file into _inbox/ for onboarding training.
///
/// Returns the filename of the installed sample.
#[tauri::command]
pub fn install_inbox_sample(state: State<'_, Arc<AppState>>) -> Result<String, String> {
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

    std::fs::write(&dest, content).map_err(|e| format!("Failed to write sample file: {}", e))?;

    Ok(filename.to_string())
}

/// Get frequent same-domain correspondents from Gmail sent mail.
#[tauri::command]
pub async fn get_frequent_correspondents(
    user_email: String,
    _state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::google_api::gmail::FrequentCorrespondent>, String> {
    let token =
        crate::google_api::load_token().map_err(|e| format!("Google not connected: {}", e))?;

    crate::google_api::gmail::fetch_frequent_correspondents(&token.token, &user_email, 10)
        .await
        .map_err(|e| format!("Failed to fetch correspondents: {}", e))
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
    state: State<'_, Arc<AppState>>,
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
pub fn dev_get_state(state: State<'_, Arc<AppState>>) -> Result<crate::devtools::DevState, String> {
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
pub fn dev_run_today_mechanical(state: State<'_, Arc<AppState>>) -> Result<String, String> {
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
pub fn dev_run_today_full(state: State<'_, Arc<AppState>>) -> Result<String, String> {
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
pub fn dev_run_week_mechanical(state: State<'_, Arc<AppState>>) -> Result<String, String> {
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
pub fn dev_run_week_full(state: State<'_, Arc<AppState>>) -> Result<String, String> {
    if !cfg!(debug_assertions) {
        return Err("Dev tools not available in release builds".into());
    }
    crate::devtools::run_week_full(&state)
}

/// Restore from dev mode to live mode (I298).
///
/// Deactivates dev DB isolation, reopens the live database, reinitializes the
/// async DB connection pool, and restores the original workspace path.
#[tauri::command]
pub async fn dev_restore_live(state: State<'_, Arc<AppState>>) -> Result<String, String> {
    if !cfg!(debug_assertions) {
        return Err("Dev tools not available in release builds".into());
    }
    let result = crate::devtools::restore_live(&state)?;

    // Reinitialize the async DB connection pool at the live path
    if let Err(e) = state.reinit_db_service().await {
        log::warn!("Failed to reinit db_service after dev_restore_live: {}", e);
    }

    Ok(result)
}

/// Purge all known mock/dev data from the current database (I298).
///
/// Removes exact mock IDs seeded by devtools scenarios. Safe for the live DB.
#[tauri::command]
pub fn dev_purge_mock_data(state: State<'_, Arc<AppState>>) -> Result<String, String> {
    if !cfg!(debug_assertions) {
        return Err("Dev tools not available in release builds".into());
    }
    crate::devtools::purge_mock_data(&state)
}

/// Delete stale dev artifact files from disk (I298).
///
/// Removes dailyos-dev.db and optionally ~/Documents/DailyOS-dev/.
#[tauri::command]
pub fn dev_clean_artifacts(include_workspace: bool) -> Result<String, String> {
    if !cfg!(debug_assertions) {
        return Err("Dev tools not available in release builds".into());
    }
    crate::devtools::clean_dev_artifacts(include_workspace)
}

/// Set dev auth overrides for Claude and Google status checks.
///
/// 0 = real check (no override), 1 = authenticated/ready,
/// 2 = not installed/not configured, 3 = installed-not-authed / token expired.
#[tauri::command]
pub fn dev_set_auth_override(claude_mode: u8, google_mode: u8) -> Result<String, String> {
    if !cfg!(debug_assertions) {
        return Err("Dev tools not available in release builds".into());
    }
    DEV_CLAUDE_OVERRIDE.store(claude_mode, Ordering::Relaxed);
    DEV_GOOGLE_OVERRIDE.store(google_mode, Ordering::Relaxed);
    Ok(format!(
        "Auth overrides set — Claude: {}, Google: {}",
        claude_mode, google_mode
    ))
}

/// Apply a named onboarding scenario: reset wizard state + set auth overrides.
///
/// Scenarios: fresh, auth_ready, no_claude, claude_unauthed, no_google, google_expired, nothing_works.
#[tauri::command]
pub fn dev_onboarding_scenario(
    scenario: String,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    if !cfg!(debug_assertions) {
        return Err("Dev tools not available in release builds".into());
    }
    crate::devtools::onboarding_scenario(&scenario, &state)
}

/// Build MeetingOutcomeData from a TranscriptResult + state lookups.
pub fn build_outcome_data(
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

    crate::types::MeetingOutcomeData {
        meeting_id: meeting_id.to_string(),
        summary: result.summary.clone(),
        wins: result.wins.clone(),
        risks: result.risks.clone(),
        decisions: result.decisions.clone(),
        actions,
        transcript_path: result.destination.clone(),
        processed_at: Some(chrono::Utc::now().to_rfc3339()),
    }
}

/// Compute executive intelligence signals (I42).
#[tauri::command]
pub async fn get_executive_intelligence(
    state: State<'_, Arc<AppState>>,
) -> Result<crate::intelligence::ExecutiveIntelligence, String> {
    let started = std::time::Instant::now();
    let result = crate::services::entities::get_executive_intelligence(&state);
    log_command_latency(
        "get_executive_intelligence",
        started,
        READ_CMD_LATENCY_BUDGET_MS,
    );
    result.await
}

// =============================================================================
// People Commands (I51)
// =============================================================================

/// Get all people with pre-computed signals, optionally filtered by relationship.
#[tauri::command]
pub async fn get_people(
    relationship: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::db::PersonListItem>, String> {
    state
        .db_read(move |db| {
            db.get_people_with_signals(relationship.as_deref())
                .map_err(|e| e.to_string())
        })
        .await
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
    pub recent_captures: Vec<crate::db::DbCapture>,
    pub recent_email_signals: Vec<crate::db::DbEmailSignal>,
    pub intelligence: Option<crate::intelligence::IntelligenceJson>,
    pub open_actions: Vec<crate::db::DbAction>,
    pub upcoming_meetings: Vec<MeetingSummary>,
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

/// Richer meeting summary with optional prep context (ADR-0063).
/// Used on account detail pages where prep preview is needed.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingPreview {
    pub id: String,
    pub title: String,
    pub start_time: String,
    pub meeting_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prep_context: Option<PrepContext>,
}

/// Get full detail for a person (person + signals + entities + recent meetings).
#[tauri::command]
pub async fn get_person_detail(
    person_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<PersonDetailResult, String> {
    crate::services::people::get_person_detail(&person_id, &state).await
}

/// Search people by name, email, or organization.
#[tauri::command]
pub async fn search_people(
    query: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::db::DbPerson>, String> {
    state
        .db_read(move |db| db.search_people(&query, 50).map_err(|e| e.to_string()))
        .await
}

/// Update a single field on a person (role, organization, notes, relationship).
/// Also updates the person's workspace files.
#[tauri::command]
pub async fn update_person(
    person_id: String,
    field: String,
    value: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let app_state = state.inner().clone();
    state
        .db_write(move |db| {
            crate::services::people::update_person_field(db, &app_state, &person_id, &field, &value)
        })
        .await
}

/// Link a person to an entity (account/project).
/// Regenerates person.json so the link persists in the filesystem (ADR-0048).
#[tauri::command]
pub async fn link_person_entity(
    person_id: String,
    entity_id: String,
    relationship_type: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let app_state = state.inner().clone();
    state
        .db_write(move |db| {
            crate::services::people::link_person_entity(
                db,
                &app_state,
                &person_id,
                &entity_id,
                &relationship_type,
            )
        })
        .await
}

/// Unlink a person from an entity.
/// Regenerates person.json so the removal persists in the filesystem (ADR-0048).
#[tauri::command]
pub async fn unlink_person_entity(
    person_id: String,
    entity_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let app_state = state.inner().clone();
    state
        .db_write(move |db| {
            crate::services::people::unlink_person_entity(db, &app_state, &person_id, &entity_id)
        })
        .await
}

/// Get people linked to an entity.
#[tauri::command]
pub async fn get_people_for_entity(
    entity_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::db::DbPerson>, String> {
    state
        .db_read(move |db| {
            db.get_people_for_entity(&entity_id)
                .map_err(|e| e.to_string())
        })
        .await
}

/// Get people who attended a specific meeting.
#[tauri::command]
pub async fn get_meeting_attendees(
    meeting_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::db::DbPerson>, String> {
    state
        .db_read(move |db| {
            db.get_meeting_attendees(&meeting_id)
                .map_err(|e| e.to_string())
        })
        .await
}

// =========================================================================
// Meeting-Entity M2M (I52)
// =========================================================================

/// Link a meeting to an entity (account/project) via the junction table.
/// ADR-0086: After relinking, clears prep_frozen_json and enqueues for
/// mechanical re-assembly from the new entity's intelligence.
#[tauri::command]
pub async fn link_meeting_entity(
    meeting_id: String,
    entity_id: String,
    entity_type: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    crate::services::meetings::link_meeting_entity_with_prep_queue(
        &state,
        &meeting_id,
        &entity_id,
        &entity_type,
    )
    .await
}

/// Remove a meeting-entity link from the junction table.
/// ADR-0086: After unlinking, clears prep_frozen_json and enqueues for
/// mechanical re-assembly without the removed entity's intelligence.
#[tauri::command]
pub async fn unlink_meeting_entity(
    meeting_id: String,
    entity_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    crate::services::meetings::unlink_meeting_entity_with_prep_queue(
        &state,
        &meeting_id,
        &entity_id,
    )
    .await
}

/// Get all entities linked to a meeting via the junction table.
#[tauri::command]
pub async fn get_meeting_entities(
    meeting_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::entity::DbEntity>, String> {
    state
        .db_read(move |db| {
            db.get_meeting_entities(&meeting_id)
                .map_err(|e| e.to_string())
        })
        .await
}

/// Reassign a meeting's entity with full cascade to actions, captures, and intelligence.
/// Clears existing entity links, sets the new one, and cascades to related tables.
/// Emits `prep-ready` event on successful rebuild (I477).
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn update_meeting_entity(
    meeting_id: String,
    entity_id: Option<String>,
    entity_type: String,
    meeting_title: String,
    start_time: String,
    meeting_type_str: String,
    state: State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let ctx = crate::services::meetings::MeetingMutationCtx {
        state: &state,
        meeting_id: &meeting_id,
        app_handle: Some(&app_handle),
    };
    crate::services::meetings::update_meeting_entity(
        ctx,
        entity_id.as_deref(),
        &entity_type,
        &meeting_title,
        &start_time,
        &meeting_type_str,
    )
    .await
}

// =========================================================================
// Additive Meeting-Entity Link/Unlink (I184 multi-entity)
// =========================================================================

/// Add an entity link to a meeting with full cascade (people, intelligence).
/// Unlike `update_meeting_entity` which clears-and-replaces, this is additive.
/// Emits `prep-ready` event on successful rebuild (I477).
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn add_meeting_entity(
    meeting_id: String,
    entity_id: String,
    entity_type: String,
    meeting_title: String,
    start_time: String,
    meeting_type_str: String,
    state: State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let ctx = crate::services::meetings::MeetingMutationCtx {
        state: &state,
        meeting_id: &meeting_id,
        app_handle: Some(&app_handle),
    };
    crate::services::meetings::add_meeting_entity(
        ctx,
        &entity_id,
        &entity_type,
        &meeting_title,
        &start_time,
        &meeting_type_str,
    )
    .await
}

/// Remove an entity link from a meeting with cleanup (legacy account_id, intelligence).
/// Emits `prep-ready` event on successful rebuild (I477).
#[tauri::command]
pub async fn remove_meeting_entity(
    meeting_id: String,
    entity_id: String,
    entity_type: String,
    state: State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let ctx = crate::services::meetings::MeetingMutationCtx {
        state: &state,
        meeting_id: &meeting_id,
        app_handle: Some(&app_handle),
    };
    crate::services::meetings::remove_meeting_entity(ctx, &entity_id, &entity_type).await
}

// =========================================================================
// Entity Keyword Management (I305)
// =========================================================================

/// Remove a keyword from a project's auto-extracted keyword list.
#[tauri::command]
pub async fn remove_project_keyword(
    project_id: String,
    keyword: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    state
        .db_write(move |db| {
            crate::services::mutations::remove_project_keyword(db, &project_id, &keyword)
        })
        .await
}

/// Remove a keyword from an account's auto-extracted keyword list.
#[tauri::command]
pub async fn remove_account_keyword(
    account_id: String,
    keyword: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    state
        .db_write(move |db| {
            crate::services::mutations::remove_account_keyword(db, &account_id, &keyword)
        })
        .await
}

// =========================================================================
// Person Creation (I129)
// =========================================================================

/// Create a new person manually. Returns the generated person ID.
#[tauri::command]
pub async fn create_person(
    email: String,
    name: String,
    organization: Option<String>,
    role: Option<String>,
    relationship: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let email = crate::util::validate_email(&email)?;
    state
        .db_write(move |db| {
            crate::services::people::create_person(
                db,
                &email,
                &name,
                organization.as_deref(),
                role.as_deref(),
                relationship.as_deref(),
            )
        })
        .await
}

/// Merge two people: transfer all references from `remove_id` to `keep_id`, then delete the removed person.
/// Also cleans up filesystem directories and regenerates the kept person's files.
#[tauri::command]
pub async fn merge_people(
    keep_id: String,
    remove_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let app_state = state.inner().clone();
    state
        .db_write(move |db| {
            crate::services::people::merge_people(db, &app_state, &keep_id, &remove_id)
        })
        .await
}

/// Delete a person and all their references. Also removes their filesystem directory.
#[tauri::command]
pub async fn delete_person(
    person_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let app_state = state.inner().clone();
    state
        .db_write(move |db| crate::services::people::delete_person(db, &app_state, &person_id))
        .await
}

/// Enrich a person with intelligence assessment (relationship intelligence).
/// Uses split-lock pattern (I173) — DB lock held only briefly during gather/write.
#[tauri::command]
pub async fn enrich_person(
    person_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::intelligence::IntelligenceJson, String> {
    crate::services::intelligence::enrich_entity(person_id, "person".to_string(), &state).await
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team_summary: Option<String>,
    pub renewal_date: Option<String>,
    pub open_action_count: usize,
    pub days_since_last_meeting: Option<i64>,
    pub parent_id: Option<String>,
    pub parent_name: Option<String>,
    pub child_count: usize,
    pub is_parent: bool,
    pub account_type: crate::db::AccountType,
    pub archived: bool,
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
    pub renewal_date: Option<String>,
    pub contract_start: Option<String>,
    pub company_overview: Option<crate::accounts::CompanyOverview>,
    pub strategic_programs: Vec<crate::accounts::StrategicProgram>,
    pub notes: Option<String>,
    pub open_actions: Vec<crate::db::DbAction>,
    pub upcoming_meetings: Vec<MeetingSummary>,
    /// ADR-0063: richer type with optional prep context for preview cards.
    pub recent_meetings: Vec<MeetingPreview>,
    pub linked_people: Vec<crate::db::DbPerson>,
    pub account_team: Vec<crate::db::DbAccountTeamMember>,
    pub account_team_import_notes: Vec<crate::db::DbAccountTeamImportNote>,
    pub signals: Option<crate::db::StakeholderSignals>,
    pub recent_captures: Vec<crate::db::DbCapture>,
    pub recent_email_signals: Vec<crate::db::DbEmailSignal>,
    pub parent_id: Option<String>,
    pub parent_name: Option<String>,
    pub children: Vec<AccountChildSummary>,
    pub parent_aggregate: Option<crate::db::ParentAggregate>,
    pub account_type: crate::db::AccountType,
    pub archived: bool,
    /// Entity intelligence (ADR-0057) — synthesized assessment from enrichment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intelligence: Option<crate::intelligence::IntelligenceJson>,
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
    pub account_type: String,
}

/// Get top-level accounts with computed summary fields for the list page (I114).
///
/// Returns only accounts where `parent_id IS NULL`. Each parent account
/// includes a `child_count` so the UI can show an expand chevron.
#[tauri::command]
pub async fn get_accounts_list(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<AccountListItem>, String> {
    state
        .db_read(crate::services::accounts::get_accounts_list)
        .await
}

/// Lightweight list of ALL accounts (parents + children) for entity pickers.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PickerAccount {
    pub id: String,
    pub name: String,
    pub parent_name: Option<String>,
    pub account_type: crate::db::AccountType,
}

#[tauri::command]
pub async fn get_accounts_for_picker(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<PickerAccount>, String> {
    state
        .db_read(crate::services::accounts::get_accounts_for_picker)
        .await
}

/// Get child accounts for a parent (I114).
#[tauri::command]
pub async fn get_child_accounts_list(
    parent_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<AccountListItem>, String> {
    state
        .db_read(move |db| crate::services::accounts::get_child_accounts_list(db, &parent_id))
        .await
}

/// I316: Get ancestor accounts for breadcrumb navigation.
#[tauri::command]
pub async fn get_account_ancestors(
    account_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::db::DbAccount>, String> {
    state
        .db_read(move |db| {
            db.get_account_ancestors(&account_id)
                .map_err(|e| e.to_string())
        })
        .await
}

/// I316: Get all descendant accounts for a given ancestor.
#[tauri::command]
pub async fn get_descendant_accounts(
    ancestor_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::db::DbAccount>, String> {
    state
        .db_read(move |db| {
            db.get_descendant_accounts(&ancestor_id)
                .map_err(|e| e.to_string())
        })
        .await
}

/// Convert a DbAccount to an AccountListItem with computed signals.
fn account_to_list_item(
    a: &crate::db::DbAccount,
    db: &crate::db::ActionDb,
    child_count: usize,
) -> AccountListItem {
    crate::services::accounts::account_to_list_item(a, db, child_count)
}

/// Get full detail for an account (DB fields + narrative JSON + computed data).
#[tauri::command]
pub async fn get_account_detail(
    account_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<AccountDetailResult, String> {
    crate::services::accounts::get_account_detail(&account_id, &state).await
}

/// Get account-team members (I207).
#[tauri::command]
pub async fn get_account_team(
    account_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::db::DbAccountTeamMember>, String> {
    state
        .db_read(move |db| db.get_account_team(&account_id).map_err(|e| e.to_string()))
        .await
}

/// Add a person-role pair to an account team (I207).
#[tauri::command]
pub async fn add_account_team_member(
    account_id: String,
    person_id: String,
    role: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let app_state = state.inner().clone();
    state
        .db_write(move |db| {
            crate::services::accounts::add_account_team_member(
                db,
                &app_state,
                &account_id,
                &person_id,
                &role,
            )
        })
        .await
}

/// Remove a person-role pair from an account team (I207).
#[tauri::command]
pub async fn remove_account_team_member(
    account_id: String,
    person_id: String,
    role: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let app_state = state.inner().clone();
    state
        .db_write(move |db| {
            crate::services::accounts::remove_account_team_member(
                db,
                &app_state,
                &account_id,
                &person_id,
                &role,
            )
        })
        .await
}

/// Update a single structured field on an account.
/// Writes to SQLite, then regenerates dashboard.json + dashboard.md.
#[tauri::command]
pub async fn update_account_field(
    account_id: String,
    field: String,
    value: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let app_state = state.inner().clone();
    state
        .db_write(move |db| {
            crate::services::accounts::update_account_field(
                db,
                &app_state,
                &account_id,
                &field,
                &value,
            )
        })
        .await
}

/// Update account notes (narrative field — JSON only, not SQLite).
/// Writes dashboard.json + regenerates dashboard.md.
#[tauri::command]
pub async fn update_account_notes(
    account_id: String,
    notes: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let app_state = state.inner().clone();
    state
        .db_write(move |db| {
            crate::services::accounts::update_account_notes(db, &app_state, &account_id, &notes)
        })
        .await
}

/// Update account strategic programs (narrative field — JSON only).
/// Writes dashboard.json + regenerates dashboard.md.
#[tauri::command]
pub async fn update_account_programs(
    account_id: String,
    programs_json: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let app_state = state.inner().clone();
    state
        .db_write(move |db| {
            crate::services::accounts::update_account_programs(
                db,
                &app_state,
                &account_id,
                &programs_json,
            )
        })
        .await
}

/// Create a new account. Creates SQLite record + workspace files.
/// If `parent_id` is provided, creates a child (BU) account under that parent.
/// If `account_type` is provided, uses that type; otherwise defaults to `customer`
/// (or inherits from parent for child accounts).
#[tauri::command]
pub async fn create_account(
    name: String,
    parent_id: Option<String>,
    account_type: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let acct_type = account_type.map(|s| crate::db::AccountType::from_db(&s));
    let app_state = state.inner().clone();
    state
        .db_write(move |db| {
            crate::services::accounts::create_account(
                db,
                &app_state,
                &name,
                parent_id.as_deref(),
                acct_type,
            )
        })
        .await
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamColleagueInput {
    pub name: String,
    pub email: String,
    pub title: Option<String>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateInternalOrganizationResult {
    pub root_account_id: String,
    pub initial_team_id: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InternalTeamSetupPrefill {
    pub company: Option<String>,
    pub domains: Vec<String>,
    pub title: Option<String>,
    pub suggested_team_name: String,
    pub suggested_colleagues: Vec<TeamColleagueInput>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InternalTeamSetupStatus {
    pub required: bool,
    pub prefill: InternalTeamSetupPrefill,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateChildAccountResult {
    pub id: String,
}

#[tauri::command]
pub async fn get_internal_team_setup_status(
    state: State<'_, Arc<AppState>>,
) -> Result<InternalTeamSetupStatus, String> {
    let config = state
        .config
        .read()
        .map_err(|_| "Lock poisoned")?
        .clone()
        .ok_or("Config not loaded")?;

    let suggested_team_name = if let Some(title) = config.user_title.as_deref() {
        if title.to_lowercase().contains("manager") || title.to_lowercase().contains("director") {
            "Leadership Team".to_string()
        } else {
            "Core Team".to_string()
        }
    } else {
        "Core Team".to_string()
    };

    let suggested_colleagues = state
        .db_read(|db| {
            db.get_people(Some("internal"))
                .map_err(|e| e.to_string())
                .map(|people| {
                    people
                        .into_iter()
                        .take(5)
                        .map(|p| TeamColleagueInput {
                            name: p.name,
                            email: p.email,
                            title: p.role,
                        })
                        .collect::<Vec<_>>()
                })
        })
        .await?;

    Ok(InternalTeamSetupStatus {
        required: !config.internal_team_setup_completed,
        prefill: InternalTeamSetupPrefill {
            company: config.user_company.clone(),
            domains: config.resolved_user_domains(),
            title: config.user_title.clone(),
            suggested_team_name,
            suggested_colleagues,
        },
    })
}

#[tauri::command]
pub async fn create_internal_organization(
    company_name: String,
    domains: Vec<String>,
    team_name: String,
    colleagues: Vec<TeamColleagueInput>,
    existing_person_ids: Option<Vec<String>>,
    state: State<'_, Arc<AppState>>,
) -> Result<CreateInternalOrganizationResult, String> {
    crate::services::accounts::create_internal_organization(
        &state,
        &company_name,
        &domains,
        &team_name,
        &colleagues,
        &existing_person_ids.unwrap_or_default(),
    )
    .await
}

#[tauri::command]
pub async fn create_child_account(
    parent_id: String,
    name: String,
    description: Option<String>,
    owner_person_id: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<CreateChildAccountResult, String> {
    crate::services::accounts::create_child_account_cmd(
        &state,
        &parent_id,
        &name,
        description.as_deref(),
        owner_person_id.as_deref(),
    )
    .await
}

#[tauri::command]
pub async fn create_team(
    name: String,
    description: Option<String>,
    owner_person_id: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<CreateChildAccountResult, String> {
    let cfg = state
        .config
        .read()
        .map_err(|_| "Lock poisoned")?
        .clone()
        .ok_or("Config not loaded")?;

    let root_id = if let Some(id) = cfg.internal_org_account_id {
        id
    } else {
        state
            .db_read(|db| {
                db.get_internal_root_account()
                    .map_err(|e| e.to_string())?
                    .map(|a| a.id)
                    .ok_or("No internal organization configured".to_string())
            })
            .await?
    };

    create_child_account(root_id, name, description, owner_person_id, state).await
}

#[tauri::command]
pub async fn backfill_internal_meeting_associations(
    state: State<'_, Arc<AppState>>,
) -> Result<usize, String> {
    state
        .db_write(crate::services::accounts::backfill_internal_meeting_associations)
        .await
}

// =============================================================================
// I124: Content Index
// =============================================================================

/// Get indexed files for an entity.
#[tauri::command]
pub async fn get_entity_files(
    entity_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::db::DbContentFile>, String> {
    state
        .db_read(move |db| db.get_entity_files(&entity_id).map_err(|e| e.to_string()))
        .await
}

/// Re-scan an entity's directory and return the updated file list.
///
/// Supports accounts, projects, and people. The `entity_type` parameter
/// determines which lookup and sync path to use.
#[tauri::command]
pub async fn index_entity_files(
    entity_type: String,
    entity_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::db::DbContentFile>, String> {
    let workspace_path = state
        .config
        .read()
        .map_err(|_| "Lock poisoned")?
        .as_ref()
        .ok_or("Config not loaded")?
        .workspace_path
        .clone();

    let et = entity_type.clone();
    let eid = entity_id.clone();
    let wp = workspace_path.clone();
    let files = state
        .db_write(move |db| {
            let workspace = Path::new(&wp);
            match et.as_str() {
                "account" => {
                    let account = db
                        .get_account(&eid)
                        .map_err(|e| e.to_string())?
                        .ok_or_else(|| format!("Account not found: {}", eid))?;
                    crate::accounts::sync_content_index_for_account(workspace, db, &account)?;
                }
                "project" => {
                    let project = db
                        .get_project(&eid)
                        .map_err(|e| e.to_string())?
                        .ok_or_else(|| format!("Project not found: {}", eid))?;
                    crate::projects::sync_content_index_for_project(workspace, db, &project)?;
                }
                "person" => {
                    let person = db
                        .get_person(&eid)
                        .map_err(|e| e.to_string())?
                        .ok_or_else(|| format!("Person not found: {}", eid))?;
                    let dir = if let Some(ref tp) = person.tracker_path {
                        workspace.join(tp)
                    } else {
                        crate::people::person_dir(workspace, &person.name)
                    };
                    crate::entity_io::sync_content_index_for_entity(
                        db, workspace, &person.id, "person", &dir,
                    )?;
                }
                _ => return Err(format!("Unknown entity type: {}", et)),
            }
            db.get_entity_files(&eid).map_err(|e| e.to_string())
        })
        .await?;

    state
        .embedding_queue
        .enqueue(crate::processor::embeddings::EmbeddingRequest {
            entity_id: entity_id.clone(),
            entity_type: entity_type.clone(),
            requested_at: std::time::Instant::now(),
        });
    state.integrations.embedding_queue_wake.notify_one();
    state.intel_queue.enqueue(crate::intel_queue::IntelRequest {
        entity_id,
        entity_type,
        priority: crate::intel_queue::IntelPriority::ContentChange,
        requested_at: std::time::Instant::now(),
        retry_count: 0,
    });
    state.integrations.intel_queue_wake.notify_one();

    Ok(files)
}

/// Reveal a file in macOS Finder.
///
/// Path must resolve to within the workspace directory or ~/.dailyos/ (I293).
#[tauri::command]
pub fn reveal_in_finder(path: String, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let canonical = std::fs::canonicalize(&path).map_err(|e| format!("Invalid path: {}", e))?;
    let canonical_str = canonical.to_string_lossy();

    // Allow workspace directory
    let workspace_ok = state
        .config
        .read()
        .ok()
        .and_then(|c| c.as_ref().map(|cfg| cfg.workspace_path.clone()))
        .map(|wp| {
            std::fs::canonicalize(&wp)
                .map(|cwp| canonical_str.starts_with(&*cwp.to_string_lossy()))
                .unwrap_or(false)
        })
        .unwrap_or(false);

    // Allow ~/.dailyos/
    let config_ok = dirs::home_dir()
        .map(|h| {
            let config_dir = h.join(".dailyos");
            std::fs::canonicalize(&config_dir)
                .map(|cd| canonical_str.starts_with(&*cd.to_string_lossy()))
                .unwrap_or(false)
        })
        .unwrap_or(false);

    if !workspace_ok && !config_ok {
        return Err("Path is outside the workspace directory".to_string());
    }

    std::process::Command::new("open")
        .arg("-R")
        .arg(&path)
        .spawn()
        .map_err(|e| format!("Failed to open Finder: {}", e))?;
    Ok(())
}

/// Export a meeting briefing as a styled HTML file and open in the default browser.
/// The user can then Print > Save as PDF from the browser.
#[tauri::command]
pub fn export_briefing_html(meeting_id: String, markdown: String) -> Result<(), String> {
    let tmp_dir = std::env::temp_dir().join("dailyos-export");
    std::fs::create_dir_all(&tmp_dir).map_err(|e| format!("Failed to create temp dir: {}", e))?;

    let safe_id = meeting_id
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect::<String>();
    let filename = format!(
        "briefing-{}.html",
        if safe_id.is_empty() {
            "export"
        } else {
            &safe_id
        }
    );
    let path = tmp_dir.join(&filename);

    // Convert markdown to simple HTML
    let body_html = markdown_to_simple_html(&markdown);

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>Intelligence Report</title>
<style>
  @import url('https://fonts.googleapis.com/css2?family=Newsreader:ital,opsz,wght@0,6..72,200..800;1,6..72,200..800&family=DM+Sans:wght@400;500&family=JetBrains+Mono:wght@400;500&display=swap');
  body {{ font-family: 'DM Sans', sans-serif; max-width: 700px; margin: 48px auto; padding: 0 24px; color: #2a2a2a; line-height: 1.65; font-size: 15px; }}
  h1 {{ font-family: 'Newsreader', serif; font-size: 36px; font-weight: 400; letter-spacing: -0.01em; margin: 0 0 8px; }}
  h2 {{ font-family: 'Newsreader', serif; font-size: 22px; font-weight: 400; margin: 48px 0 12px; border-top: 1px solid #e0ddd8; padding-top: 16px; }}
  p {{ margin: 0 0 12px; }}
  ul, ol {{ padding-left: 20px; margin: 0 0 12px; }}
  li {{ margin-bottom: 8px; }}
  code {{ font-family: 'JetBrains Mono', monospace; font-size: 13px; background: #f5f3ef; padding: 1px 4px; border-radius: 2px; }}
  blockquote {{ border-left: 3px solid #c9a227; padding-left: 20px; margin: 16px 0; font-style: italic; color: #555; }}
  hr {{ border: none; border-top: 1px solid #e0ddd8; margin: 32px 0; }}
  .meta {{ font-family: 'JetBrains Mono', monospace; font-size: 11px; color: #888; letter-spacing: 0.04em; margin-bottom: 32px; }}
  @media print {{ body {{ margin: 24px; }} }}
</style>
</head>
<body>
<p class="meta">DAILYOS INTELLIGENCE REPORT</p>
{}
</body>
</html>"#,
        body_html
    );

    std::fs::write(&path, &html).map_err(|e| format!("Failed to write HTML: {}", e))?;

    std::process::Command::new("open")
        .arg(path.to_str().unwrap_or(""))
        .spawn()
        .map_err(|e| format!("Failed to open browser: {}", e))?;

    Ok(())
}

/// Simple markdown to HTML converter for briefing export.
fn markdown_to_simple_html(md: &str) -> String {
    let mut html = String::new();
    let mut in_list = false;
    let mut list_type = "ul";

    for line in md.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            if in_list {
                html.push_str(&format!("</{}>\n", list_type));
                in_list = false;
            }
            continue;
        }

        // Headings
        if let Some(rest) = trimmed.strip_prefix("# ") {
            if in_list {
                html.push_str(&format!("</{}>\n", list_type));
                in_list = false;
            }
            html.push_str(&format!("<h1>{}</h1>\n", rest));
        } else if let Some(rest) = trimmed.strip_prefix("## ") {
            if in_list {
                html.push_str(&format!("</{}>\n", list_type));
                in_list = false;
            }
            html.push_str(&format!("<h2>{}</h2>\n", rest));
        } else if let Some(rest) = trimmed.strip_prefix("### ") {
            if in_list {
                html.push_str(&format!("</{}>\n", list_type));
                in_list = false;
            }
            html.push_str(&format!("<h3>{}</h3>\n", rest));
        }
        // Unordered list
        else if let Some(rest) = trimmed.strip_prefix("- ") {
            if !in_list {
                html.push_str("<ul>\n");
                in_list = true;
                list_type = "ul";
            }
            html.push_str(&format!("<li>{}</li>\n", rest));
        }
        // Ordered list
        else if trimmed.len() > 2
            && trimmed
                .chars()
                .next()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
            && trimmed.contains(". ")
        {
            if let Some(pos) = trimmed.find(". ") {
                if !in_list {
                    html.push_str("<ol>\n");
                    in_list = true;
                    list_type = "ol";
                }
                html.push_str(&format!("<li>{}</li>\n", &trimmed[pos + 2..]));
            }
        }
        // Horizontal rule
        else if trimmed == "---" || trimmed == "***" {
            if in_list {
                html.push_str(&format!("</{}>\n", list_type));
                in_list = false;
            }
            html.push_str("<hr>\n");
        }
        // Paragraph
        else {
            if in_list {
                html.push_str(&format!("</{}>\n", list_type));
                in_list = false;
            }
            html.push_str(&format!("<p>{}</p>\n", trimmed));
        }
    }

    if in_list {
        html.push_str(&format!("</{}>\n", list_type));
    }

    html
}

// =============================================================================
// Sprint 26: Chat Tool Commands
// =============================================================================

use crate::types::{meetings_to_json, ChatEntityListItem};

fn ensure_open_chat_session(
    db: &crate::db::ActionDb,
    entity_id: Option<&str>,
    entity_type: Option<&str>,
) -> Result<crate::db::DbChatSession, String> {
    crate::services::mutations::ensure_open_chat_session(db, entity_id, entity_type)
}

fn append_chat_exchange(
    db: &crate::db::ActionDb,
    session_id: &str,
    user_content: &str,
    assistant_json: &serde_json::Value,
) -> Result<(), String> {
    crate::services::mutations::append_chat_exchange(db, session_id, user_content, assistant_json)
}

#[tauri::command]
pub async fn chat_search_content(
    entity_id: String,
    query: String,
    top_k: Option<usize>,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::queries::search::ContentMatch>, String> {
    let query_str = query.trim().to_string();
    if query_str.is_empty() {
        return Ok(Vec::new());
    }

    let embedding_model = state.embedding_model.clone();
    let k = top_k.unwrap_or(10).clamp(1, 50);
    state
        .db_write(move |db| {
            let matches = crate::queries::search::search_entity_content(
                db,
                Some(embedding_model.as_ref()),
                &entity_id,
                &query_str,
                k,
                0.7,
                0.3,
            )?;

            let session = ensure_open_chat_session(db, Some(&entity_id), None)?;
            let response = serde_json::json!({
                "entityId": entity_id,
                "query": query_str,
                "matches": matches,
            });
            append_chat_exchange(db, &session.id, &query_str, &response)?;

            Ok(matches)
        })
        .await
}

#[tauri::command]
pub async fn chat_query_entity(
    entity_id: String,
    question: String,
    state: State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, String> {
    let question_str = question.trim().to_string();
    if question_str.is_empty() {
        return Err("question is required".to_string());
    }

    let embedding_model = state.embedding_model.clone();
    state
        .db_write(move |db| {
            let question = question_str.as_str();

            let (entity_type, entity_name, facts, open_actions, recent_meetings) =
                if let Some(account) = db.get_account(&entity_id).map_err(|e| e.to_string())? {
                    let meetings = db
                        .get_meetings_for_account(&entity_id, 10)
                        .map_err(|e| e.to_string())?;
                    let meetings_json = meetings_to_json(&meetings);
                    (
                        "account",
                        account.name.clone(),
                        serde_json::json!({
                            "health": account.health,
                            "lifecycle": account.lifecycle,
                            "arr": account.arr,
                            "renewal": account.contract_end,
                            "nps": account.nps,
                        }),
                        db.get_account_actions(&entity_id).unwrap_or_default(),
                        meetings_json,
                    )
                } else if let Some(project) =
                    db.get_project(&entity_id).map_err(|e| e.to_string())?
                {
                    let meetings = db
                        .get_meetings_for_project(&entity_id, 10)
                        .map_err(|e| e.to_string())?;
                    let meetings_json = meetings_to_json(&meetings);
                    (
                        "project",
                        project.name.clone(),
                        serde_json::json!({
                            "status": project.status,
                            "milestone": project.milestone,
                            "owner": project.owner,
                            "targetDate": project.target_date,
                        }),
                        db.get_project_actions(&entity_id).unwrap_or_default(),
                        meetings_json,
                    )
                } else if let Some(person) = db.get_person(&entity_id).map_err(|e| e.to_string())? {
                    let meetings = db
                        .get_person_meetings(&entity_id, 10)
                        .map_err(|e| e.to_string())?;
                    let meetings_json = meetings_to_json(&meetings);
                    (
                        "person",
                        person.name.clone(),
                        serde_json::json!({
                            "organization": person.organization,
                            "role": person.role,
                            "relationship": person.relationship,
                            "meetingCount": person.meeting_count,
                            "lastSeen": person.last_seen,
                        }),
                        Vec::new(),
                        meetings_json,
                    )
                } else {
                    return Err(format!("Entity not found: {}", entity_id));
                };

            let semantic_matches = crate::queries::search::search_entity_content(
                db,
                Some(embedding_model.as_ref()),
                &entity_id,
                question,
                8,
                0.7,
                0.3,
            )?;
            let intelligence = db.get_entity_intelligence(&entity_id).ok().flatten();

            let session = ensure_open_chat_session(db, Some(&entity_id), Some(entity_type))?;
            let response = serde_json::json!({
                "sessionId": session.id,
                "entityId": entity_id,
                "entityType": entity_type,
                "entityName": entity_name,
                "question": question,
                "facts": facts,
                "intelligence": intelligence,
                "openActions": open_actions,
                "recentMeetings": recent_meetings,
                "semanticMatches": semantic_matches,
            });
            append_chat_exchange(db, &session.id, question, &response)?;

            Ok(response)
        })
        .await
}

#[tauri::command]
pub async fn chat_get_briefing(
    state: State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, String> {
    let dashboard = crate::services::dashboard::get_dashboard_data(&state).await;

    let response = match dashboard {
        DashboardResult::Success {
            data, freshness, ..
        } => serde_json::json!({
            "status": "success",
            "data": data,
            "freshness": freshness,
        }),
        DashboardResult::Empty { message, .. } => serde_json::json!({
            "status": "empty",
            "message": message,
        }),
        DashboardResult::Error { message } => serde_json::json!({
            "status": "error",
            "message": message,
        }),
    };

    state
        .db_write(move |db| {
            let session = ensure_open_chat_session(db, None, None)?;
            append_chat_exchange(db, &session.id, "get briefing", &response)?;
            Ok(response)
        })
        .await
}

#[tauri::command]
pub async fn chat_list_entities(
    entity_type: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<ChatEntityListItem>, String> {
    let requested = entity_type
        .as_deref()
        .map(|s| s.to_lowercase())
        .unwrap_or_else(|| "all".to_string());

    state
        .db_write(move |db| {
            let mut items = Vec::new();
            if requested == "all" || requested == "account" || requested == "accounts" {
                let accounts = db.get_all_accounts().map_err(|e| e.to_string())?;
                for account in accounts.into_iter().filter(|a| !a.archived) {
                    let open_action_count = db
                        .get_account_actions(&account.id)
                        .map(|a| a.len())
                        .unwrap_or(0);
                    items.push(ChatEntityListItem {
                        id: account.id,
                        name: account.name,
                        entity_type: "account".to_string(),
                        status: account.lifecycle,
                        health: account.health,
                        open_action_count,
                    });
                }
            }

            if requested == "all" || requested == "project" || requested == "projects" {
                let projects = db.get_all_projects().map_err(|e| e.to_string())?;
                for project in projects.into_iter().filter(|p| !p.archived) {
                    let open_action_count = db
                        .get_project_actions(&project.id)
                        .map(|a| a.len())
                        .unwrap_or(0);
                    items.push(ChatEntityListItem {
                        id: project.id,
                        name: project.name,
                        entity_type: "project".to_string(),
                        status: Some(project.status),
                        health: None,
                        open_action_count,
                    });
                }
            }

            let session = ensure_open_chat_session(db, None, None)?;
            let response = serde_json::json!({
                "entityType": requested,
                "count": items.len(),
                "items": items,
            });
            append_chat_exchange(db, &session.id, "list entities", &response)?;

            Ok(items)
        })
        .await
}

// ── I74/I131: Entity Intelligence Enrichment via Claude Code ────────

/// Uses split-lock pattern (I173) — DB lock held only briefly during gather/write.
#[tauri::command]
pub async fn enrich_account(
    account_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::intelligence::IntelligenceJson, String> {
    crate::services::intelligence::enrich_entity(account_id, "account".to_string(), &state).await
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
    pub parent_id: Option<String>,
    pub parent_name: Option<String>,
    pub child_count: usize,
    pub is_parent: bool,
    pub archived: bool,
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
    pub recent_email_signals: Vec<crate::db::DbEmailSignal>,
    pub archived: bool,
    /// Entity intelligence (ADR-0057) — synthesized assessment from enrichment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intelligence: Option<crate::intelligence::IntelligenceJson>,
    /// I388: Parent project ID (if this is a child project).
    pub parent_id: Option<String>,
    /// I388: Parent project name (resolved from parent_id).
    pub parent_name: Option<String>,
    /// I388: Child project summaries (if this is a parent project).
    pub children: Vec<ProjectChildSummary>,
    /// I388: Aggregate stats for parent projects.
    pub parent_aggregate: Option<crate::db::ProjectParentAggregate>,
}

/// Compact child project summary for parent detail pages (I388).
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectChildSummary {
    pub id: String,
    pub name: String,
    pub status: String,
    pub milestone: Option<String>,
    pub open_action_count: usize,
}

/// Get all projects with computed summary fields for the list page.
#[tauri::command]
pub async fn get_projects_list(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<ProjectListItem>, String> {
    crate::services::projects::get_projects_list(&state).await
}

/// Get full detail for a project.
#[tauri::command]
pub async fn get_project_detail(
    project_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<ProjectDetailResult, String> {
    crate::services::projects::get_project_detail(&project_id, &state).await
}

/// Get child projects for a parent project (I388).
#[tauri::command]
pub async fn get_child_projects_list(
    parent_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<ProjectListItem>, String> {
    crate::services::projects::get_child_projects_list(&parent_id, &state).await
}

/// I388: Get ancestor projects for breadcrumb navigation.
#[tauri::command]
pub async fn get_project_ancestors(
    project_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::db::DbProject>, String> {
    state
        .db_read(move |db| {
            db.get_project_ancestors(&project_id)
                .map_err(|e| e.to_string())
        })
        .await
}

/// Create a new project.
#[tauri::command]
pub async fn create_project(
    name: String,
    parent_id: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    crate::services::projects::create_project(&name, parent_id, &state).await
}

/// Update a single structured field on a project.
#[tauri::command]
pub async fn update_project_field(
    project_id: String,
    field: String,
    value: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    crate::services::projects::update_project_field(&project_id, &field, &value, &state).await
}

/// Update the notes field on a project.
#[tauri::command]
pub async fn update_project_notes(
    project_id: String,
    notes: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    crate::services::projects::update_project_notes(&project_id, &notes, &state).await
}

/// Enrich a project via Claude Code intelligence enrichment.
#[tauri::command]
pub async fn enrich_project(
    project_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::intelligence::IntelligenceJson, String> {
    crate::services::intelligence::enrich_entity(project_id, "project".to_string(), &state).await
}

// ── I76: Database Backup & Rebuild ──────────────────────────────────

#[tauri::command]
pub async fn backup_database(state: tauri::State<'_, Arc<AppState>>) -> Result<String, String> {
    state.db_read(crate::db_backup::backup_database).await
}

#[tauri::command]
pub fn get_database_recovery_status(
    state: State<'_, Arc<AppState>>,
) -> crate::state::DatabaseRecoveryStatus {
    state.get_database_recovery_status()
}

#[tauri::command]
pub fn list_database_backups() -> Result<Vec<crate::db_backup::BackupInfo>, String> {
    crate::db_backup::list_database_backups()
}

#[tauri::command]
pub async fn restore_database_from_backup(
    backup_path: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let _permit = state
        .heavy_work_semaphore
        .acquire()
        .await
        .map_err(|_| "Failed to acquire restore lock".to_string())?;

    // Drop existing DB handles before swapping files on disk.
    {
        let mut db_service_guard = state.db_service.write().await;
        *db_service_guard = None;
        let mut db_guard = state
            .db
            .lock()
            .map_err(|_| "DB lock poisoned".to_string())?;
        *db_guard = None;
    }

    if let Err(e) = crate::db_backup::restore_database_from_backup(Path::new(&backup_path)) {
        // Best-effort recovery: re-open original DB handles if restore failed.
        if let Ok(db) = crate::db::ActionDb::open() {
            if let Ok(mut db_guard) = state.db.lock() {
                *db_guard = Some(db);
            }
            let _ = state.init_db_service().await;
        } else {
            state.set_database_recovery_required(
                "restore_failed",
                format!("Restore failed and database reopen failed: {e}"),
            );
        }
        return Err(e);
    }

    let reopened = match crate::db::ActionDb::open() {
        Ok(db) => db,
        Err(e) => {
            let detail = format!("Restore applied but database reopen failed: {e}");
            state.set_database_recovery_required("restore_reopen_failed", detail.clone());
            return Err(detail);
        }
    };
    {
        let mut db_guard = state
            .db
            .lock()
            .map_err(|_| "DB lock poisoned".to_string())?;
        *db_guard = Some(reopened);
    }

    if let Err(e) = state.init_db_service().await {
        state.set_database_recovery_required("restore_reopen_failed", e.clone());
        return Err(format!(
            "Restore succeeded but failed to reinitialize DB service: {e}"
        ));
    }

    state.clear_database_recovery_required();
    Ok(())
}

#[tauri::command]
pub async fn start_fresh_database(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    // Drop DB handles before deleting files.
    {
        let mut db_service_guard = state.db_service.write().await;
        *db_service_guard = None;
        let mut db_guard = state
            .db
            .lock()
            .map_err(|_| "DB lock poisoned".to_string())?;
        *db_guard = None;
    }
    crate::db_backup::start_fresh_database()
}

#[tauri::command]
pub async fn export_database_copy(destination: String) -> Result<(), String> {
    crate::db_backup::export_database_copy(&destination)
}

#[tauri::command]
pub fn get_database_info() -> Result<crate::db_backup::DatabaseInfo, String> {
    crate::db_backup::get_database_info()
}

#[tauri::command]
pub async fn rebuild_database(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(usize, usize, usize), String> {
    let (workspace_path, user_domains) = {
        let guard = state.config.read().map_err(|_| "Lock poisoned")?;
        let config = guard.as_ref().ok_or("Config not loaded")?;
        (
            config.workspace_path.clone(),
            config.resolved_user_domains(),
        )
    };

    state
        .db_write(move |db| {
            crate::db_backup::rebuild_from_filesystem(
                std::path::Path::new(&workspace_path),
                db,
                &user_domains,
            )
        })
        .await
}

/// Helper: create a default AccountJson from a DbAccount.
fn default_account_json(account: &crate::db::DbAccount) -> crate::accounts::AccountJson {
    crate::services::accounts::default_account_json(account)
}

/// Get the latest hygiene scan report
#[tauri::command]
pub fn get_hygiene_report(
    state: State<'_, Arc<AppState>>,
) -> Result<Option<crate::hygiene::HygieneReport>, String> {
    let guard = state
        .hygiene
        .report
        .lock()
        .map_err(|_| "Lock poisoned".to_string())?;
    Ok(guard.clone())
}

/// Get a prose narrative summarizing the last hygiene scan.
#[tauri::command]
pub fn get_hygiene_narrative(
    state: State<'_, Arc<AppState>>,
) -> Result<Option<crate::hygiene::HygieneNarrativeView>, String> {
    let report = state.hygiene.report.lock().map_err(|_| "Lock poisoned")?;
    Ok(report
        .as_ref()
        .and_then(crate::hygiene::build_hygiene_narrative))
}

/// Get the current Intelligence Hygiene status view model.
#[tauri::command]
pub fn get_intelligence_hygiene_status(
    state: State<'_, Arc<AppState>>,
) -> Result<HygieneStatusView, String> {
    let report = state
        .hygiene
        .report
        .lock()
        .map_err(|_| "Lock poisoned".to_string())?
        .clone();
    Ok(build_intelligence_hygiene_status(&state, report.as_ref()))
}

/// Run a hygiene scan immediately and return the updated status.
#[tauri::command]
pub fn run_hygiene_scan_now(state: State<'_, Arc<AppState>>) -> Result<HygieneStatusView, String> {
    if state
        .hygiene
        .scan_running
        .compare_exchange(
            false,
            true,
            std::sync::atomic::Ordering::AcqRel,
            std::sync::atomic::Ordering::Acquire,
        )
        .is_err()
    {
        return Err("A hygiene scan is already running".to_string());
    }

    let scan_result = (|| -> Result<crate::hygiene::HygieneReport, String> {
        let config = state
            .config
            .read()
            .map_err(|_| "Lock poisoned".to_string())?
            .clone()
            .ok_or("No configuration loaded".to_string())?;

        let db = crate::db::ActionDb::open().map_err(|e| e.to_string())?;
        let workspace = std::path::Path::new(&config.workspace_path);
        let report = crate::hygiene::run_hygiene_scan(
            &db,
            &config,
            workspace,
            Some(&state.hygiene.budget),
            Some(&state.intel_queue),
            false,
            Some(state.embedding_model.as_ref()),
        );

        // Prune old audit trail files (I297)
        let pruned = crate::audit::prune_audit_files(workspace);
        if pruned > 0 {
            log::info!("run_hygiene_scan_now: pruned {} old audit files", pruned);
        }

        if let Ok(mut guard) = state.hygiene.report.lock() {
            *guard = Some(report.clone());
        }
        if let Ok(mut guard) = state.hygiene.last_scan_at.lock() {
            *guard = Some(report.scanned_at.clone());
        }
        if let Ok(mut guard) = state.hygiene.next_scan_at.lock() {
            *guard = Some(
                (chrono::Utc::now()
                    + chrono::Duration::seconds(
                        crate::hygiene::scan_interval_secs(Some(&config)) as i64
                    ))
                .to_rfc3339(),
            );
        }

        Ok(report)
    })();

    state
        .hygiene
        .scan_running
        .store(false, std::sync::atomic::Ordering::Release);

    let report = scan_result?;
    Ok(build_intelligence_hygiene_status(&state, Some(&report)))
}

/// Detect potential duplicate people (I172).
#[tauri::command]
pub async fn get_duplicate_people(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::hygiene::DuplicateCandidate>, String> {
    state.db_read(crate::hygiene::detect_duplicate_people).await
}

/// Detect potential duplicate people for a specific person (I172).
#[tauri::command]
pub async fn get_duplicate_people_for_person(
    person_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::hygiene::DuplicateCandidate>, String> {
    state
        .db_read(move |db| {
            let dupes = crate::hygiene::detect_duplicate_people(db)?;
            Ok(dupes
                .into_iter()
                .filter(|d| d.person1_id == person_id || d.person2_id == person_id)
                .collect())
        })
        .await
}

// =============================================================================
// I176: Archive / Unarchive Entities
// =============================================================================

/// Archive or unarchive an account. Cascades to children when archiving.
#[tauri::command]
pub async fn archive_account(
    id: String,
    archived: bool,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let app_state = state.inner().clone();
    state
        .db_write(move |db| {
            crate::services::accounts::archive_account(db, &app_state, &id, archived)
        })
        .await
}

/// Merge source account into target account.
#[tauri::command]
pub async fn merge_accounts(
    from_id: String,
    into_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<crate::db::MergeResult, String> {
    let app_state = state.inner().clone();
    state
        .db_write(move |db| {
            crate::services::accounts::merge_accounts(db, &app_state, &from_id, &into_id)
        })
        .await
}

/// Archive or unarchive a project.
#[tauri::command]
pub async fn archive_project(
    id: String,
    archived: bool,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    state
        .db_write(move |db| crate::services::projects::archive_project(db, &id, archived))
        .await
}

/// Archive or unarchive a person.
#[tauri::command]
pub async fn archive_person(
    id: String,
    archived: bool,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let app_state = state.inner().clone();
    state
        .db_write(move |db| crate::services::people::archive_person(db, &app_state, &id, archived))
        .await
}

/// Get archived accounts.
#[tauri::command]
pub async fn get_archived_accounts(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::db::DbAccount>, String> {
    state
        .db_read(|db| db.get_archived_accounts().map_err(|e| e.to_string()))
        .await
}

/// Get archived projects.
#[tauri::command]
pub async fn get_archived_projects(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::db::DbProject>, String> {
    state
        .db_read(|db| db.get_archived_projects().map_err(|e| e.to_string()))
        .await
}

/// Get archived people with signals.
#[tauri::command]
pub async fn get_archived_people(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::db::PersonListItem>, String> {
    state
        .db_read(|db| {
            db.get_archived_people_with_signals()
                .map_err(|e| e.to_string())
        })
        .await
}

/// Restore an archived account with optional child restoration (I199).
#[tauri::command]
pub async fn restore_account(
    account_id: String,
    restore_children: bool,
    state: State<'_, Arc<AppState>>,
) -> Result<usize, String> {
    state
        .db_write(move |db| {
            crate::services::accounts::restore_account(db, &account_id, restore_children)
        })
        .await
}

// =============================================================================
// I171: Multi-Domain Config
// =============================================================================

/// Set multiple user domains for multi-org meeting classification.
#[tauri::command]
pub async fn set_user_domains(
    domains: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Config, String> {
    crate::services::settings::set_user_domains(&domains, &state).await
}

// =============================================================================
// I162: Bulk Entity Creation
// =============================================================================

/// Bulk-create accounts from a list of names. Returns created account IDs.
#[tauri::command]
pub async fn bulk_create_accounts(
    names: Vec<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<String>, String> {
    let workspace_path = state
        .config
        .read()
        .map_err(|_| "Lock poisoned")?
        .as_ref()
        .ok_or("Config not loaded")?
        .workspace_path
        .clone();
    state
        .db_write(move |db| {
            let workspace = Path::new(&workspace_path);
            crate::services::accounts::bulk_create_accounts(db, workspace, &names)
        })
        .await
}

/// Bulk-create projects from a list of names. Returns created project IDs.
#[tauri::command]
pub async fn bulk_create_projects(
    names: Vec<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<String>, String> {
    let workspace_path = state
        .config
        .read()
        .map_err(|_| "Lock poisoned")?
        .as_ref()
        .ok_or("Config not loaded")?
        .workspace_path
        .clone();
    state
        .db_write(move |db| {
            let workspace = Path::new(&workspace_path);
            crate::services::projects::bulk_create_projects(db, workspace, &names)
        })
        .await
}

// =============================================================================
// I143: Account Events
// =============================================================================

/// Record an account lifecycle event (expansion, downsell, churn, etc.)
#[tauri::command]
pub async fn record_account_event(
    account_id: String,
    event_type: String,
    event_date: String,
    arr_impact: Option<f64>,
    notes: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<i64, String> {
    let app_state = state.inner().clone();
    state
        .db_write(move |db| {
            crate::services::accounts::record_account_event(
                db,
                &app_state,
                &account_id,
                &event_type,
                &event_date,
                arr_impact,
                notes.as_deref(),
            )
        })
        .await
}

/// Get account events for a given account.
#[tauri::command]
pub async fn get_account_events(
    account_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::db::DbAccountEvent>, String> {
    state
        .db_read(move |db| {
            db.get_account_events(&account_id)
                .map_err(|e| e.to_string())
        })
        .await
}

// =============================================================================
// I194: User Agenda + Notes Editability (ADR-0065)
// =============================================================================

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyPrepPrefillResult {
    pub meeting_id: String,
    pub added_agenda_items: usize,
    pub notes_appended: bool,
    pub mode: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgendaDraftResult {
    pub meeting_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    pub body: String,
}

fn is_meeting_user_layer_read_only(meeting: &crate::db::DbMeeting) -> bool {
    if meeting.prep_frozen_at.is_some() {
        return true;
    }
    let now = chrono::Utc::now();
    let end_dt = meeting
        .end_time
        .as_deref()
        .and_then(parse_meeting_datetime)
        .or_else(|| {
            parse_meeting_datetime(&meeting.start_time).map(|s| s + chrono::Duration::hours(1))
        });
    // Default to read-only when time can't be parsed — safer than allowing edits
    // on meetings whose temporal state is unknown.
    end_dt.is_none_or(|e| e < now)
}

fn normalized_item_key(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn merge_user_agenda(existing: &[String], incoming: &[String]) -> (Vec<String>, usize) {
    let mut merged = existing.to_vec();
    let mut seen: std::collections::HashSet<String> = existing
        .iter()
        .map(|item| normalized_item_key(item))
        .filter(|k| !k.is_empty())
        .collect();
    let mut added = 0usize;

    for item in incoming {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            continue;
        }
        let key = normalized_item_key(trimmed);
        if key.is_empty() || seen.contains(&key) {
            continue;
        }
        merged.push(trimmed.to_string());
        seen.insert(key);
        added += 1;
    }

    (merged, added)
}

fn merge_user_notes(existing: Option<&str>, notes_append: &str) -> (Option<String>, bool) {
    let append = notes_append.trim();
    if append.is_empty() {
        return (existing.map(|s| s.to_string()), false);
    }

    match existing.map(str::trim).filter(|s| !s.is_empty()) {
        Some(current) if current.contains(append) => (Some(current.to_string()), false),
        Some(current) => (Some(format!("{}\n\n{}", current, append)), true),
        None => (Some(append.to_string()), true),
    }
}

fn apply_meeting_prep_prefill_inner(
    db: &crate::db::ActionDb,
    engine: &crate::signals::propagation::PropagationEngine,
    meeting_id: &str,
    agenda_items: &[String],
    notes_append: &str,
) -> Result<ApplyPrepPrefillResult, String> {
    let meeting = db
        .get_meeting_intelligence_row(meeting_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Meeting not found: {}", meeting_id))?;

    if is_meeting_user_layer_read_only(&meeting) {
        return Err("Meeting user fields are read-only after freeze/past state".to_string());
    }

    let existing_agenda =
        parse_user_agenda_json(meeting.user_agenda_json.as_deref()).unwrap_or_default();
    let (merged_agenda, added_agenda_items) = merge_user_agenda(&existing_agenda, agenda_items);
    let agenda_json = if merged_agenda.is_empty() {
        None
    } else {
        Some(serde_json::to_string(&merged_agenda).map_err(|e| format!("Serialize error: {}", e))?)
    };

    let (merged_notes, notes_appended) =
        merge_user_notes(meeting.user_notes.as_deref(), notes_append);
    crate::services::mutations::update_meeting_user_layer(
        db,
        engine,
        meeting_id,
        agenda_json.as_deref(),
        merged_notes.as_deref(),
    )?;

    Ok(ApplyPrepPrefillResult {
        meeting_id: meeting_id.to_string(),
        added_agenda_items,
        notes_appended,
        mode: "append_dedupe".to_string(),
    })
}

fn generate_agenda_draft_body(
    title: &str,
    time_range: Option<&str>,
    agenda_items: &[String],
    context_hint: Option<&str>,
    context: Option<&str>,
) -> String {
    let mut body = String::new();
    body.push_str(&format!(
        "Hi all,\n\nAhead of {}, here is a proposed agenda",
        title
    ));
    if let Some(range) = time_range.filter(|value| !value.trim().is_empty()) {
        body.push_str(&format!(" for {}.", range));
    } else {
        body.push('.');
    }
    body.push_str("\n\n");

    if agenda_items.is_empty() {
        body.push_str("1. Confirm priorities and desired outcomes\n");
        body.push_str("2. Review current risks and blockers\n");
        body.push_str("3. Align on owners and next steps\n");
    } else {
        for (idx, item) in agenda_items.iter().enumerate() {
            body.push_str(&format!("{}. {}\n", idx + 1, item));
        }
    }

    if let Some(hint) = context_hint.map(str::trim).filter(|s| !s.is_empty()) {
        body.push_str(&format!("\nAdditional context to cover: {}\n", hint));
    }

    if let Some(summary) = context
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.lines().next().unwrap_or(s).trim())
        .filter(|s| !s.is_empty())
    {
        body.push_str(&format!("\nCurrent context: {}\n", summary));
    }

    body.push_str("\nPlease reply with additions or edits.\n\nThanks");
    body
}

fn build_agenda_draft_result(
    meeting: &crate::db::DbMeeting,
    prep: Option<&FullMeetingPrep>,
    context_hint: Option<&str>,
) -> AgendaDraftResult {
    let mut agenda_items: Vec<String> = Vec::new();
    if let Some(prep) = prep {
        if let Some(ref user_agenda) = prep.user_agenda {
            agenda_items.extend(user_agenda.iter().map(|item| item.trim().to_string()));
        }
        if agenda_items.is_empty() {
            if let Some(ref proposed) = prep.proposed_agenda {
                // Filter out talking_point source items to match frontend "Your Plan" display.
                // Fall back to all items if filtering leaves nothing.
                let non_talking: Vec<String> = proposed
                    .iter()
                    .filter(|item| item.source.as_deref() != Some("talking_point"))
                    .map(|item| item.topic.trim().to_string())
                    .filter(|item| !item.is_empty())
                    .collect();
                if non_talking.is_empty() {
                    agenda_items.extend(
                        proposed
                            .iter()
                            .map(|item| item.topic.trim().to_string())
                            .filter(|item| !item.is_empty()),
                    );
                } else {
                    agenda_items.extend(non_talking);
                }
            }
        }
    }
    agenda_items.retain(|item| !item.is_empty());
    let mut seen = std::collections::HashSet::new();
    agenda_items.retain(|item| seen.insert(normalized_item_key(item)));

    let title = prep
        .map(|p| p.title.as_str())
        .filter(|t| !t.trim().is_empty())
        .unwrap_or(&meeting.title);
    let time_range = prep.map(|p| p.time_range.as_str());
    let context = prep
        .and_then(|p| p.meeting_context.as_deref())
        .or(meeting.summary.as_deref());

    AgendaDraftResult {
        meeting_id: meeting.id.clone(),
        subject: Some(format!("Agenda for {}", title)),
        body: generate_agenda_draft_body(title, time_range, &agenda_items, context_hint, context),
    }
}

/// Apply AI-suggested prep additions in append + dedupe mode.
#[tauri::command]
pub async fn apply_meeting_prep_prefill(
    meeting_id: String,
    agenda_items: Vec<String>,
    notes_append: String,
    state: State<'_, Arc<AppState>>,
) -> Result<ApplyPrepPrefillResult, String> {
    let engine = state.signals.engine.clone();
    let mid = meeting_id.clone();
    let ai = agenda_items.clone();
    let na = notes_append.clone();
    let result = state
        .db_write(move |db| apply_meeting_prep_prefill_inner(db, &engine, &mid, &ai, &na))
        .await?;

    // Mirror write to active prep JSON (best-effort) for immediate UI coherence.
    if let Ok(prep_path) = resolve_prep_path(&meeting_id, &state) {
        if let Ok(content) = std::fs::read_to_string(&prep_path) {
            if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&content) {
                let existing = json
                    .get("userAgenda")
                    .and_then(|v| v.as_array())
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(|item| item.as_str().map(|s| s.to_string()))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                let (merged_agenda, _) = merge_user_agenda(&existing, &agenda_items);
                if let Some(obj) = json.as_object_mut() {
                    if merged_agenda.is_empty() {
                        obj.remove("userAgenda");
                    } else {
                        obj.insert("userAgenda".to_string(), serde_json::json!(merged_agenda));
                    }
                    let existing_notes = obj.get("userNotes").and_then(|v| v.as_str());
                    let (merged_notes, _) = merge_user_notes(existing_notes, &notes_append);
                    if let Some(notes) = merged_notes {
                        obj.insert("userNotes".to_string(), serde_json::json!(notes));
                    }
                }
                if let Ok(updated) = serde_json::to_string_pretty(&json) {
                    let _ = std::fs::write(&prep_path, updated);
                }
            }
        }
    }

    Ok(result)
}

/// Generate a draft agenda message from current prep context.
#[tauri::command]
pub async fn generate_meeting_agenda_message_draft(
    meeting_id: String,
    context_hint: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<AgendaDraftResult, String> {
    let config = state
        .config
        .read()
        .map_err(|_| "Lock poisoned")?
        .clone()
        .ok_or("No configuration loaded")?;
    let workspace_path = config.workspace_path.clone();

    state
        .db_read(move |db| {
            let workspace = Path::new(&workspace_path);
            let today_dir = workspace.join("_today");
            let meeting = db
                .get_meeting_intelligence_row(&meeting_id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("Meeting not found: {}", meeting_id))?;
            let prep = load_meeting_prep_from_sources(&today_dir, &meeting);

            Ok(build_agenda_draft_result(
                &meeting,
                prep.as_ref(),
                context_hint.as_deref(),
            ))
        })
        .await
}

/// Update user-authored agenda items on a meeting prep file.
#[tauri::command]
pub async fn update_meeting_user_agenda(
    meeting_id: String,
    agenda: Option<Vec<String>>,
    dismissed_topics: Option<Vec<String>>,
    hidden_attendees: Option<Vec<String>>,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let app_state = state.inner().clone();
    state
        .db_write(move |db| {
            crate::services::meetings::update_meeting_user_agenda(
                db,
                &app_state,
                &meeting_id,
                agenda,
                dismissed_topics,
                hidden_attendees,
            )
        })
        .await
}

/// Update user-authored notes on a meeting prep file.
#[tauri::command]
pub async fn update_meeting_user_notes(
    meeting_id: String,
    notes: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let app_state = state.inner().clone();
    state
        .db_write(move |db| {
            crate::services::meetings::update_meeting_user_notes(
                db,
                &app_state,
                &meeting_id,
                &notes,
            )
        })
        .await
}

/// Resolve the on-disk path for a meeting's prep JSON file.
fn resolve_prep_path(meeting_id: &str, state: &AppState) -> Result<std::path::PathBuf, String> {
    let config = state
        .config
        .read()
        .map_err(|_| "Lock poisoned")?
        .clone()
        .ok_or("No configuration loaded")?;

    let workspace = Path::new(&config.workspace_path);
    let preps_dir = workspace.join("_today").join("data").join("preps");
    let clean_id = meeting_id.trim_end_matches(".json").trim_end_matches(".md");
    let path = preps_dir.join(format!("{}.json", clean_id));

    // Path containment check: prevent traversal outside preps directory
    if !path.starts_with(&preps_dir) {
        return Err("Invalid meeting ID".to_string());
    }

    if path.exists() {
        Ok(path)
    } else {
        Err(format!("Prep file not found: {}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{ActionDb, DbMeeting};
    use chrono::Utc;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn test_backfill_prep_semantics_value_derives_recent_wins_and_sources() {
        let mut prep = json!({
            "talkingPoints": [
                "Recent win: Sponsor re-engaged _(source: 2026-02-11-sync.md)_",
                "Win: Tier upgrade requested"
            ]
        });

        let changed = backfill_prep_semantics_value(&mut prep);
        assert!(changed);
        assert_eq!(prep["recentWins"][0], "Sponsor re-engaged");
        assert_eq!(prep["recentWins"][1], "Tier upgrade requested");
        assert_eq!(prep["recentWinSources"][0]["label"], "2026-02-11-sync.md");
        assert_eq!(prep["talkingPoints"][0], "Recent win: Sponsor re-engaged");
    }

    #[test]
    fn test_backfill_prep_files_in_dir_dry_run_counts() {
        let dir = tempdir().expect("tempdir");
        let preps_dir = dir.path().join("preps");
        fs::create_dir_all(&preps_dir).expect("create preps dir");

        fs::write(
            preps_dir.join("needs-backfill.json"),
            serde_json::to_string_pretty(&json!({
                "talkingPoints": ["Recent win: Sponsor re-engaged (source: notes.md)"]
            }))
            .unwrap(),
        )
        .unwrap();
        fs::write(
            preps_dir.join("already-new.json"),
            serde_json::to_string_pretty(&json!({
                "recentWins": ["Sponsor re-engaged"],
                "recentWinSources": [{"label": "notes.md", "path": "notes.md"}]
            }))
            .unwrap(),
        )
        .unwrap();
        fs::write(preps_dir.join("bad.json"), "{").unwrap();

        let counts = backfill_prep_files_in_dir(&preps_dir, true).expect("dry-run should succeed");
        assert_eq!(counts.candidate, 3);
        assert_eq!(counts.transformed, 1);
        assert_eq!(counts.skipped, 1);
        assert_eq!(counts.parse_errors, 1);

        let unchanged = fs::read_to_string(preps_dir.join("needs-backfill.json")).unwrap();
        assert!(unchanged.contains("talkingPoints"));
        assert!(!unchanged.contains("recentWins"));
    }

    #[test]
    fn test_backfill_prep_files_in_dir_apply_updates_file() {
        let dir = tempdir().expect("tempdir");
        let preps_dir = dir.path().join("preps");
        fs::create_dir_all(&preps_dir).expect("create preps dir");
        let path = preps_dir.join("meeting.json");
        fs::write(
            &path,
            serde_json::to_string_pretty(&json!({
                "talkingPoints": ["Recent win: Expansion approved (source: expansion.md)"]
            }))
            .unwrap(),
        )
        .unwrap();

        let counts = backfill_prep_files_in_dir(&preps_dir, false).expect("apply should succeed");
        assert_eq!(counts.transformed, 1);

        let updated: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(updated["recentWins"][0], "Expansion approved");
        assert_eq!(updated["recentWinSources"][0]["label"], "expansion.md");
    }

    #[test]
    fn test_backfill_db_prep_contexts_apply_updates_rows() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("test.db");
        let db = ActionDb::open_at_unencrypted(db_path).expect("open db");

        let meeting = DbMeeting {
            id: "mtg-1".to_string(),
            title: "Test Meeting".to_string(),
            meeting_type: "customer".to_string(),
            start_time: Utc::now().to_rfc3339(),
            end_time: None,
            attendees: None,
            notes_path: None,
            summary: None,
            created_at: Utc::now().to_rfc3339(),
            calendar_event_id: None,
            description: None,
            prep_context_json: Some(
                serde_json::to_string(&json!({
                    "talkingPoints": ["Recent win: Champion re-engaged (source: call.md)"]
                }))
                .unwrap(),
            ),
            user_agenda_json: None,
            user_notes: None,
            prep_frozen_json: None,
            prep_frozen_at: None,
            prep_snapshot_path: None,
            prep_snapshot_hash: None,
            transcript_path: None,
            transcript_processed_at: None,
            intelligence_state: None,
            intelligence_quality: None,
            last_enriched_at: None,
            signal_count: None,
            has_new_signals: None,
            last_viewed_at: None,
        };
        db.upsert_meeting(&meeting).expect("insert meeting");

        let dry_counts = backfill_db_prep_contexts(&db, true).expect("dry-run");
        assert_eq!(dry_counts.candidate, 1);
        assert_eq!(dry_counts.transformed, 1);

        let before = db
            .get_meeting_by_id("mtg-1")
            .expect("meeting lookup")
            .expect("meeting exists")
            .prep_context_json
            .unwrap();
        assert!(!before.contains("recentWins"));

        let apply_counts = backfill_db_prep_contexts(&db, false).expect("apply");
        assert_eq!(apply_counts.candidate, 1);
        assert_eq!(apply_counts.transformed, 1);

        let after = db
            .get_meeting_by_id("mtg-1")
            .expect("meeting lookup")
            .expect("meeting exists")
            .prep_context_json
            .unwrap();
        assert!(after.contains("recentWins"));
        assert!(after.contains("recentWinSources"));
    }

    #[test]
    fn test_apply_meeting_prep_prefill_additive_and_idempotent() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("test.db");
        let db = ActionDb::open_at_unencrypted(db_path).expect("open db");

        let meeting = DbMeeting {
            id: "mtg-prefill".to_string(),
            title: "Prefill Test".to_string(),
            meeting_type: "customer".to_string(),
            start_time: (Utc::now() + chrono::Duration::hours(2)).to_rfc3339(),
            end_time: Some((Utc::now() + chrono::Duration::hours(3)).to_rfc3339()),
            attendees: None,
            notes_path: None,
            summary: Some("Context summary".to_string()),
            created_at: Utc::now().to_rfc3339(),
            calendar_event_id: None,
            description: None,
            prep_context_json: None,
            user_agenda_json: None,
            user_notes: None,
            prep_frozen_json: None,
            prep_frozen_at: None,
            prep_snapshot_path: None,
            prep_snapshot_hash: None,
            transcript_path: None,
            transcript_processed_at: None,
            intelligence_state: None,
            intelligence_quality: None,
            last_enriched_at: None,
            signal_count: None,
            has_new_signals: None,
            last_viewed_at: None,
        };
        db.upsert_meeting(&meeting).expect("upsert meeting");
        let engine = crate::signals::propagation::PropagationEngine::new();

        let first = apply_meeting_prep_prefill_inner(
            &db,
            &engine,
            "mtg-prefill",
            &["Confirm blockers".to_string(), "Agree owners".to_string()],
            "Bring latest renewal risk updates.",
        )
        .expect("first prefill");
        assert_eq!(first.added_agenda_items, 2);
        assert!(first.notes_appended);

        let second = apply_meeting_prep_prefill_inner(
            &db,
            &engine,
            "mtg-prefill",
            &["Confirm blockers".to_string(), "Agree owners".to_string()],
            "Bring latest renewal risk updates.",
        )
        .expect("second prefill");
        assert_eq!(second.added_agenda_items, 0);
        assert!(!second.notes_appended);

        let updated = db
            .get_meeting_intelligence_row("mtg-prefill")
            .expect("load meeting")
            .expect("meeting exists");
        let agenda =
            parse_user_agenda_json(updated.user_agenda_json.as_deref()).unwrap_or_default();
        assert_eq!(agenda.len(), 2);
        assert!(updated
            .user_notes
            .unwrap_or_default()
            .contains("renewal risk updates"));
    }

    #[test]
    fn test_apply_meeting_prep_prefill_blocks_past_or_frozen() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("test.db");
        let db = ActionDb::open_at_unencrypted(db_path).expect("open db");

        let past = DbMeeting {
            id: "mtg-past".to_string(),
            title: "Past Meeting".to_string(),
            meeting_type: "customer".to_string(),
            start_time: (Utc::now() - chrono::Duration::hours(4)).to_rfc3339(),
            end_time: Some((Utc::now() - chrono::Duration::hours(3)).to_rfc3339()),
            attendees: None,
            notes_path: None,
            summary: None,
            created_at: Utc::now().to_rfc3339(),
            calendar_event_id: None,
            description: None,
            prep_context_json: None,
            user_agenda_json: None,
            user_notes: None,
            prep_frozen_json: None,
            prep_frozen_at: None,
            prep_snapshot_path: None,
            prep_snapshot_hash: None,
            transcript_path: None,
            transcript_processed_at: None,
            intelligence_state: None,
            intelligence_quality: None,
            last_enriched_at: None,
            signal_count: None,
            has_new_signals: None,
            last_viewed_at: None,
        };
        db.upsert_meeting(&past).expect("upsert past meeting");
        let engine = crate::signals::propagation::PropagationEngine::new();

        let err = apply_meeting_prep_prefill_inner(
            &db,
            &engine,
            "mtg-past",
            &["Item".to_string()],
            "notes",
        )
        .expect_err("past meeting should be read-only");
        assert!(err.contains("read-only"));

        let frozen = DbMeeting {
            id: "mtg-frozen".to_string(),
            title: "Frozen Meeting".to_string(),
            meeting_type: "customer".to_string(),
            start_time: (Utc::now() + chrono::Duration::hours(2)).to_rfc3339(),
            end_time: Some((Utc::now() + chrono::Duration::hours(3)).to_rfc3339()),
            attendees: None,
            notes_path: None,
            summary: None,
            created_at: Utc::now().to_rfc3339(),
            calendar_event_id: None,
            description: None,
            prep_context_json: None,
            user_agenda_json: None,
            user_notes: None,
            prep_frozen_json: Some("{}".to_string()),
            prep_frozen_at: Some(Utc::now().to_rfc3339()),
            prep_snapshot_path: None,
            prep_snapshot_hash: None,
            transcript_path: None,
            transcript_processed_at: None,
            intelligence_state: None,
            intelligence_quality: None,
            last_enriched_at: None,
            signal_count: None,
            has_new_signals: None,
            last_viewed_at: None,
        };
        db.upsert_meeting(&frozen).expect("upsert frozen meeting");

        let frozen_err = apply_meeting_prep_prefill_inner(
            &db,
            &engine,
            "mtg-frozen",
            &["Item".to_string()],
            "notes",
        )
        .expect_err("frozen meeting should be read-only");
        assert!(frozen_err.contains("read-only"));
    }

    #[test]
    fn test_generate_meeting_agenda_message_draft_deterministic_structure() {
        let meeting = DbMeeting {
            id: "mtg-draft".to_string(),
            title: "Acme Weekly".to_string(),
            meeting_type: "customer".to_string(),
            start_time: Utc::now().to_rfc3339(),
            end_time: None,
            attendees: None,
            notes_path: None,
            summary: Some("Renewal risk still elevated.".to_string()),
            created_at: Utc::now().to_rfc3339(),
            calendar_event_id: None,
            description: None,
            prep_context_json: None,
            user_agenda_json: None,
            user_notes: None,
            prep_frozen_json: None,
            prep_frozen_at: None,
            prep_snapshot_path: None,
            prep_snapshot_hash: None,
            transcript_path: None,
            transcript_processed_at: None,
            intelligence_state: None,
            intelligence_quality: None,
            last_enriched_at: None,
            signal_count: None,
            has_new_signals: None,
            last_viewed_at: None,
        };

        let prep = FullMeetingPrep {
            file_path: "preps/mtg-draft.json".to_string(),
            calendar_event_id: None,
            title: "Acme Weekly".to_string(),
            time_range: "Tuesday 2:00 PM".to_string(),
            meeting_context: Some("Renewal risk remains high.".to_string()),
            calendar_notes: None,
            account_snapshot: None,
            quick_context: None,
            user_agenda: None,
            user_notes: None,
            attendees: None,
            since_last: None,
            strategic_programs: None,
            current_state: None,
            open_items: None,
            risks: None,
            talking_points: None,
            recent_wins: None,
            recent_win_sources: None,
            questions: None,
            key_principles: None,
            references: None,
            raw_markdown: None,
            stakeholder_signals: None,
            attendee_context: None,
            proposed_agenda: Some(vec![
                crate::types::AgendaItem {
                    topic: "Align on renewal path".to_string(),
                    why: None,
                    source: None,
                },
                crate::types::AgendaItem {
                    topic: "Confirm owner handoffs".to_string(),
                    why: None,
                    source: None,
                },
            ]),
            intelligence_summary: None,
            entity_risks: None,
            entity_readiness: None,
            stakeholder_insights: None,
            recent_email_signals: None,
            consistency_status: None,
            consistency_findings: Vec::new(),
        };

        let draft = build_agenda_draft_result(&meeting, Some(&prep), Some("Cover timeline risks"));
        assert_eq!(draft.subject.as_deref(), Some("Agenda for Acme Weekly"));
        assert!(draft.body.contains("1. Align on renewal path"));
        assert!(draft.body.contains("2. Confirm owner handoffs"));
        assert!(draft.body.contains("Cover timeline risks"));
        assert!(draft.body.contains("Please reply with additions or edits."));
    }
}

// ==================== Backfill ====================

/// Backfill historical meetings from filesystem into database.
///
/// Scans account/project directories for meeting files (transcripts, notes, summaries)
/// and creates database records + entity links for meetings not already in the system.
///
/// Returns (meetings_created, meetings_skipped, errors).
#[tauri::command]
pub async fn backfill_historical_meetings(
    state: State<'_, Arc<AppState>>,
) -> Result<(usize, usize, Vec<String>), String> {
    let config = state
        .config
        .read()
        .map_err(|_| "Config lock poisoned")?
        .clone()
        .ok_or("Config not initialized")?;

    state
        .db_write(move |db| crate::backfill_meetings::backfill_historical_meetings(db, &config))
        .await
}

// ==================== Risk Briefing ====================

/// Generate a strategic risk briefing for an account via AI.
/// All blocking work (DB lock + file I/O + PTY) runs in spawn_blocking
/// so the async runtime stays responsive and the UI can render the
/// progress page without beachballing.
#[tauri::command]
pub async fn generate_risk_briefing(
    state: State<'_, Arc<AppState>>,
    account_id: String,
) -> Result<crate::types::RiskBriefing, String> {
    crate::services::intelligence::generate_risk_briefing(state.inner(), &account_id).await
}

/// Read a cached risk briefing for an account (fast, no AI).
#[tauri::command]
pub async fn get_risk_briefing(
    state: State<'_, Arc<AppState>>,
    account_id: String,
) -> Result<crate::types::RiskBriefing, String> {
    let app_state = state.inner().clone();
    state
        .db_read(move |db| {
            crate::services::intelligence::get_risk_briefing(db, &app_state, &account_id)
        })
        .await
}

/// Save an edited risk briefing back to disk (user corrections).
#[tauri::command]
pub async fn save_risk_briefing(
    state: State<'_, Arc<AppState>>,
    account_id: String,
    briefing: crate::types::RiskBriefing,
) -> Result<(), String> {
    let app_state = state.inner().clone();
    state
        .db_write(move |db| {
            crate::services::intelligence::save_risk_briefing(
                db,
                &app_state,
                &account_id,
                &briefing,
            )
        })
        .await
}

// =============================================================================
// Reports (v0.15.0 — I397)
// =============================================================================

/// Generate a report for an entity (async, PTY enrichment).
#[tauri::command]
pub async fn generate_report(
    state: State<'_, Arc<AppState>>,
    entity_id: String,
    entity_type: String,
    report_type: String,
) -> Result<crate::reports::ReportRow, String> {
    crate::services::reports::generate_report(state.inner(), &entity_id, &entity_type, &report_type)
        .await
}

/// Read a cached report (fast, no AI).
#[tauri::command]
pub async fn get_report(
    state: State<'_, Arc<AppState>>,
    entity_id: String,
    entity_type: String,
    report_type: String,
) -> Result<Option<crate::reports::ReportRow>, String> {
    state
        .db_read(move |db| {
            crate::services::reports::get_report_cached(db, &entity_id, &entity_type, &report_type)
        })
        .await
}

/// Save user edits to a report (persists content_json back to DB).
#[tauri::command]
pub async fn save_report(
    state: State<'_, Arc<AppState>>,
    entity_id: String,
    entity_type: String,
    report_type: String,
    content_json: String,
) -> Result<(), String> {
    state
        .db_write(move |db| {
            crate::services::reports::save_report(
                db,
                &entity_id,
                &entity_type,
                &report_type,
                &content_json,
            )
        })
        .await
}

/// Fetch all reports for an entity.
#[tauri::command]
pub async fn get_reports_for_entity(
    state: State<'_, Arc<AppState>>,
    entity_id: String,
    entity_type: String,
) -> Result<Vec<crate::reports::ReportRow>, String> {
    state
        .db_read(move |db| {
            crate::services::reports::get_all_reports_for_entity(db, &entity_id, &entity_type)
        })
        .await
}

// =============================================================================
// MCP: Claude Desktop Configuration (ADR-0075)
// =============================================================================

pub use crate::services::integrations::ClaudeDesktopConfigResult;

/// Check whether DailyOS is already registered in Claude Desktop's MCP config.
#[tauri::command]
pub fn get_claude_desktop_status() -> ClaudeDesktopConfigResult {
    crate::services::integrations::get_claude_desktop_status()
}

/// Configure Claude Desktop to use the DailyOS MCP server.
#[tauri::command]
pub fn configure_claude_desktop() -> ClaudeDesktopConfigResult {
    crate::services::integrations::configure_claude_desktop()
}

// =============================================================================
// Cowork Plugin Export
// =============================================================================

/// Result of a Cowork plugin export operation.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoworkPluginResult {
    pub success: bool,
    pub message: String,
    pub path: Option<String>,
}

/// Info about a bundled Cowork plugin.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoworkPluginInfo {
    pub name: String,
    pub description: String,
    pub filename: String,
    pub available: bool,
    pub exported: bool,
}

/// Export a bundled Cowork plugin zip to ~/Desktop.
#[tauri::command]
pub fn export_cowork_plugin(
    app_handle: tauri::AppHandle,
    plugin_name: String,
) -> CoworkPluginResult {
    let filename = match plugin_name.as_str() {
        "dailyos" => "dailyos-plugin.zip",
        "dailyos-writer" => "dailyos-writer-plugin.zip",
        _ => {
            return CoworkPluginResult {
                success: false,
                message: format!("Unknown plugin: {plugin_name}"),
                path: None,
            }
        }
    };

    let resource_path = app_handle
        .path()
        .resource_dir()
        .ok()
        .map(|d| d.join("resources/plugins").join(filename));

    // In dev mode, fall back to the source tree
    let source_path = resource_path.filter(|p| p.exists()).or_else(|| {
        let dev_path = std::env::current_dir()
            .ok()?
            .join("resources/plugins")
            .join(filename);
        dev_path.exists().then_some(dev_path)
    });

    let source = match source_path {
        Some(p) => p,
        None => {
            return CoworkPluginResult {
                success: false,
                message: format!("Bundled plugin not found: {filename}"),
                path: None,
            }
        }
    };

    let desktop = match dirs::home_dir() {
        Some(h) => h.join("Desktop").join(filename),
        None => {
            return CoworkPluginResult {
                success: false,
                message: "Could not determine home directory".to_string(),
                path: None,
            }
        }
    };

    match std::fs::copy(&source, &desktop) {
        Ok(_) => CoworkPluginResult {
            success: true,
            message: format!("Saved to Desktop/{filename}"),
            path: Some(desktop.to_string_lossy().to_string()),
        },
        Err(e) => CoworkPluginResult {
            success: false,
            message: format!("Failed to copy: {e}"),
            path: None,
        },
    }
}

/// List available bundled Cowork plugins and their export status.
#[tauri::command]
pub fn get_cowork_plugins_status(app_handle: tauri::AppHandle) -> Vec<CoworkPluginInfo> {
    let plugins = vec![
        (
            "dailyos",
            "dailyos-plugin.zip",
            "DailyOS workspace tools — briefings, accounts, meetings, actions",
        ),
        (
            "dailyos-writer",
            "dailyos-writer-plugin.zip",
            "DailyOS Writer — drafts emails, agendas, and follow-ups from your data",
        ),
    ];

    let desktop = dirs::home_dir().map(|h| h.join("Desktop"));

    let resource_dir = app_handle.path().resource_dir().ok();

    plugins
        .into_iter()
        .map(|(name, filename, description)| {
            let available = resource_dir
                .as_ref()
                .map(|d: &std::path::PathBuf| d.join("resources/plugins").join(filename).exists())
                .unwrap_or(false)
                || std::env::current_dir()
                    .ok()
                    .map(|d: std::path::PathBuf| {
                        d.join("resources/plugins").join(filename).exists()
                    })
                    .unwrap_or(false);

            let exported = desktop
                .as_ref()
                .map(|d| d.join(filename).exists())
                .unwrap_or(false);

            CoworkPluginInfo {
                name: name.to_string(),
                description: description.to_string(),
                filename: filename.to_string(),
                available,
                exported,
            }
        })
        .collect()
}

// =============================================================================
// Intelligence Field Editing (I261)
// =============================================================================

/// Update a single field in an entity's intelligence.json.
///
/// Reads the file, applies the update via JSON path navigation, records a
/// UserEdit entry (protecting the field from AI overwrite), and writes back
/// to filesystem + SQLite cache.
#[tauri::command]
pub async fn update_intelligence_field(
    entity_id: String,
    entity_type: String,
    field_path: String,
    value: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    crate::services::intelligence::update_intelligence_field(
        &entity_id,
        &entity_type,
        &field_path,
        &value,
        &state,
    )
    .await
}

/// Bulk-replace the stakeholder list in an entity's intelligence.json.
#[tauri::command]
pub async fn update_stakeholders(
    entity_id: String,
    entity_type: String,
    stakeholders_json: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let stakeholders: Vec<crate::intelligence::StakeholderInsight> =
        serde_json::from_str(&stakeholders_json)
            .map_err(|e| format!("Invalid stakeholders JSON: {}", e))?;
    crate::services::intelligence::update_stakeholders(
        &entity_id,
        &entity_type,
        stakeholders,
        &state,
    )
    .await
}

/// Create a person entity from a stakeholder name (no email required).
///
/// Used when a stakeholder card references someone who doesn't yet exist as
/// a person entity. Creates with empty email, links to the parent entity.
#[tauri::command]
pub async fn create_person_from_stakeholder(
    entity_id: String,
    entity_type: String,
    name: String,
    role: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let app_state = state.inner().clone();
    state
        .db_write(move |db| {
            crate::services::people::create_person_from_stakeholder(
                db,
                &app_state,
                &entity_id,
                &entity_type,
                &name,
                role.as_deref(),
            )
        })
        .await
}

// =============================================================================
// Quill MCP Integration
// =============================================================================

/// Quill integration status for the frontend.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuillStatus {
    pub enabled: bool,
    pub bridge_exists: bool,
    pub bridge_path: String,
    pub pending_syncs: usize,
    pub failed_syncs: usize,
    pub completed_syncs: usize,
    pub last_sync_at: Option<String>,
    pub last_error: Option<String>,
    pub last_error_at: Option<String>,
    pub abandoned_syncs: usize,
    pub poll_interval_minutes: u32,
}

/// Get the current status of the Quill integration.
#[tauri::command]
pub async fn get_quill_status(state: State<'_, Arc<AppState>>) -> Result<QuillStatus, String> {
    let config = state
        .config
        .read()
        .ok()
        .and_then(|g| g.as_ref().map(|c| c.quill.clone()));

    let quill_config = config.unwrap_or_default();
    let bridge_exists = std::path::Path::new(&quill_config.bridge_path).exists();

    // Count sync states from DB without blocking the main thread on the
    // legacy sync mutex (can beachball during wake/unlock contention).
    let (pending, failed, completed, last_sync, last_error, last_error_at, abandoned) = state
        .db_read(|db| {
            let pending = db.get_pending_quill_syncs().map(|v| v.len()).unwrap_or(0);

            // Count failed, completed, abandoned from all rows
            let (failed_count, completed_count, last, abandoned_count) = db
                .conn_ref()
                .prepare(
                    "SELECT
                        SUM(CASE WHEN state = 'failed' THEN 1 ELSE 0 END),
                        SUM(CASE WHEN state = 'completed' THEN 1 ELSE 0 END),
                        MAX(completed_at),
                        SUM(CASE WHEN state = 'abandoned' THEN 1 ELSE 0 END)
                     FROM quill_sync_state",
                )
                .and_then(|mut stmt| {
                    stmt.query_row([], |row| {
                        Ok((
                            row.get::<_, i64>(0).unwrap_or(0) as usize,
                            row.get::<_, i64>(1).unwrap_or(0) as usize,
                            row.get::<_, Option<String>>(2)?,
                            row.get::<_, i64>(3).unwrap_or(0) as usize,
                        ))
                    })
                })
                .unwrap_or((0, 0, None, 0));

            // Get last error from failed/abandoned syncs
            let (err_msg, err_at) = db
                .conn_ref()
                .prepare(
                    "SELECT error_message, updated_at FROM quill_sync_state
                     WHERE state IN ('failed', 'abandoned') AND error_message IS NOT NULL
                     ORDER BY updated_at DESC LIMIT 1",
                )
                .and_then(|mut stmt| {
                    stmt.query_row([], |row| {
                        Ok((
                            row.get::<_, Option<String>>(0)?,
                            row.get::<_, Option<String>>(1)?,
                        ))
                    })
                })
                .unwrap_or((None, None));

            Ok((
                pending,
                failed_count,
                completed_count,
                last,
                err_msg,
                err_at,
                abandoned_count,
            ))
        })
        .await
        .unwrap_or((0, 0, 0, None, None, None, 0));

    Ok(QuillStatus {
        enabled: quill_config.enabled,
        bridge_exists,
        bridge_path: quill_config.bridge_path,
        pending_syncs: pending,
        failed_syncs: failed,
        completed_syncs: completed,
        last_sync_at: last_sync,
        last_error,
        last_error_at,
        abandoned_syncs: abandoned,
        poll_interval_minutes: quill_config.poll_interval_minutes,
    })
}

/// Enable or disable Quill integration.
#[tauri::command]
pub fn set_quill_enabled(enabled: bool, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    crate::state::create_or_update_config(&state, |config| {
        config.quill.enabled = enabled;
    })?;
    Ok(())
}

/// Result of a Quill historical backfill operation.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuillBackfillResult {
    pub created: usize,
    pub eligible: usize,
}

/// Create Quill sync rows for past meetings that never had transcript sync.
#[tauri::command]
pub async fn start_quill_backfill(
    days_back: Option<u32>,
    state: State<'_, Arc<AppState>>,
) -> Result<QuillBackfillResult, String> {
    let days_back = days_back.unwrap_or(365);
    if !(1..=3650).contains(&days_back) {
        return Err("daysBack must be between 1 and 3650".to_string());
    }
    let days_back_i32 = days_back as i32;

    state
        .db_write(move |db| {
            let ids = db
                .get_backfill_eligible_meeting_ids(days_back_i32)
                .map_err(|e| e.to_string())?;
            let eligible = ids.len();
            let mut created = 0;
            for id in &ids {
                if crate::quill::sync::create_sync_for_meeting(db, id).is_ok() {
                    created += 1;
                }
            }
            Ok(QuillBackfillResult { created, eligible })
        })
        .await
}

/// Set the Quill poll interval (1–60 minutes).
#[tauri::command]
pub fn set_quill_poll_interval(
    minutes: u32,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    if !(1..=60).contains(&minutes) {
        return Err("Poll interval must be between 1 and 60 minutes".to_string());
    }
    crate::state::create_or_update_config(&state, |config| {
        config.quill.poll_interval_minutes = minutes;
    })?;
    Ok(())
}

/// Test the Quill MCP connection by spawning the bridge and verifying connectivity.
#[tauri::command]
pub async fn test_quill_connection(state: State<'_, Arc<AppState>>) -> Result<bool, String> {
    let bridge_path = state
        .config
        .read()
        .map_err(|_| "Lock poisoned".to_string())?
        .as_ref()
        .map(|c| c.quill.bridge_path.clone())
        .unwrap_or_default();

    if bridge_path.is_empty() {
        return Ok(false);
    }

    let client = crate::quill::client::QuillClient::connect(&bridge_path)
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;

    client.disconnect().await;
    Ok(true)
}

/// Trigger Quill transcript sync for a single meeting.
/// Creates a sync row if none exists, or resets a failed/stale one to pending.
#[tauri::command]
pub async fn trigger_quill_sync_for_meeting(
    meeting_id: String,
    force: Option<bool>,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let force = force.unwrap_or(false);
    state
        .db_write(move |db| {
            // Check if a sync row already exists
            match db
                .get_quill_sync_state_by_source(&meeting_id, "quill")
                .map_err(|e| e.to_string())?
            {
                Some(existing) => {
                    match existing.state.as_str() {
                        "completed" if !force => Ok("already_completed".to_string()),
                        "completed" => {
                            // Force re-sync: reset to pending so poller picks it up again.
                            // This handles the case where captures were lost due to a bug
                            // or when the user wants to re-process with updated AI.
                            crate::quill::sync::transition_state(
                                db,
                                &existing.id,
                                "pending",
                                None,
                                None,
                                None,
                                Some("Force re-sync"),
                            )
                            .map_err(|e| e.to_string())?;
                            Ok("resyncing".to_string())
                        }
                        "pending" | "polling" | "fetching" | "processing" if force => {
                            // Force-reset a stuck in-progress state back to pending.
                            // Covers the case where the app crashed or the AI pipeline
                            // failed silently mid-processing, leaving the row orphaned.
                            crate::quill::sync::transition_state(
                                db,
                                &existing.id,
                                "pending",
                                None,
                                None,
                                None,
                                Some("Force reset from stuck state"),
                            )
                            .map_err(|e| e.to_string())?;
                            Ok("resyncing".to_string())
                        }
                        "pending" | "polling" | "fetching" | "processing" => {
                            Ok("already_in_progress".to_string())
                        }
                        _ => {
                            // Failed or abandoned — reset to pending for retry
                            crate::quill::sync::transition_state(
                                db,
                                &existing.id,
                                "pending",
                                None,
                                None,
                                None,
                                Some("Manual retry"),
                            )
                            .map_err(|e| e.to_string())?;
                            Ok("retrying".to_string())
                        }
                    }
                }
                None => {
                    crate::quill::sync::create_sync_for_meeting(db, &meeting_id)
                        .map_err(|e| e.to_string())?;
                    Ok("created".to_string())
                }
            }
        })
        .await
}

/// Get Quill sync states, optionally filtered by meeting ID.
#[tauri::command]
pub async fn get_quill_sync_states(
    meeting_id: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::db::DbQuillSyncState>, String> {
    state
        .db_read(move |db| match meeting_id {
            Some(ref mid) => {
                let row = db
                    .get_quill_sync_state_by_source(mid, "quill")
                    .map_err(|e| e.to_string())?;
                Ok(row.into_iter().collect())
            }
            None => db.get_pending_quill_syncs().map_err(|e| e.to_string()),
        })
        .await
}

// =============================================================================
// Granola Integration (I226)
// =============================================================================

/// Granola integration status for the frontend.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GranolaStatus {
    pub enabled: bool,
    pub cache_exists: bool,
    pub cache_path: String,
    pub document_count: usize,
    pub pending_syncs: usize,
    pub failed_syncs: usize,
    pub completed_syncs: usize,
    pub last_sync_at: Option<String>,
    pub poll_interval_minutes: u32,
}

/// Get the current status of the Granola integration.
#[tauri::command]
pub async fn get_granola_status(state: State<'_, Arc<AppState>>) -> Result<GranolaStatus, String> {
    let config = state
        .config
        .read()
        .ok()
        .and_then(|g| g.as_ref().map(|c| c.granola.clone()));

    let granola_config = config.unwrap_or_default();
    let resolved_path = crate::granola::resolve_cache_path(&granola_config);
    let cache_exists = resolved_path.is_some();

    let document_count = match &resolved_path {
        Some(p) => crate::granola::cache::count_documents(p).unwrap_or(0),
        None => 0,
    };

    // Count sync states from DB (source='granola')
    let (pending, failed, completed, last_sync) = state
        .db_read(|db| {
            let (failed_count, completed_count, last, pending_count) = db
                .conn_ref()
                .prepare(
                    "SELECT
                    SUM(CASE WHEN state = 'failed' THEN 1 ELSE 0 END),
                    SUM(CASE WHEN state = 'completed' THEN 1 ELSE 0 END),
                    MAX(completed_at),
                    SUM(CASE WHEN state IN ('pending', 'polling', 'processing') THEN 1 ELSE 0 END)
                 FROM quill_sync_state WHERE source = 'granola'",
                )
                .and_then(|mut stmt| {
                    stmt.query_row([], |row| {
                        Ok((
                            row.get::<_, i64>(0).unwrap_or(0) as usize,
                            row.get::<_, i64>(1).unwrap_or(0) as usize,
                            row.get::<_, Option<String>>(2)?,
                            row.get::<_, i64>(3).unwrap_or(0) as usize,
                        ))
                    })
                })
                .unwrap_or((0, 0, None, 0));
            Ok((pending_count, failed_count, completed_count, last))
        })
        .await
        .unwrap_or((0, 0, 0, None));

    Ok(GranolaStatus {
        enabled: granola_config.enabled,
        cache_exists,
        cache_path: resolved_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default(),
        document_count,
        pending_syncs: pending,
        failed_syncs: failed,
        completed_syncs: completed,
        last_sync_at: last_sync,
        poll_interval_minutes: granola_config.poll_interval_minutes,
    })
}

/// Enable or disable Granola integration.
#[tauri::command]
pub fn set_granola_enabled(enabled: bool, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    crate::state::create_or_update_config(&state, |config| {
        config.granola.enabled = enabled;
    })?;
    Ok(())
}

/// Set the Granola poll interval (1–60 minutes).
#[tauri::command]
pub fn set_granola_poll_interval(
    minutes: u32,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    if !(1..=60).contains(&minutes) {
        return Err("Poll interval must be between 1 and 60 minutes".to_string());
    }
    crate::state::create_or_update_config(&state, |config| {
        config.granola.poll_interval_minutes = minutes;
    })?;
    Ok(())
}

/// Result of a Granola backfill operation.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GranolaBackfillResult {
    pub created: usize,
    pub eligible: usize,
}

/// Create Granola sync rows for past meetings found in the cache.
#[tauri::command]
pub fn start_granola_backfill(
    days_back: Option<u32>,
    state: State<'_, Arc<AppState>>,
) -> Result<GranolaBackfillResult, String> {
    let days_back = days_back.unwrap_or(365);
    if !(1..=3650).contains(&days_back) {
        return Err("daysBack must be between 1 and 3650".to_string());
    }
    let (created, eligible) =
        crate::granola::poller::run_granola_backfill(&state, days_back as i32)?;
    Ok(GranolaBackfillResult { created, eligible })
}

/// Test whether the Granola cache file exists and is valid.
#[tauri::command]
pub fn test_granola_cache(state: State<'_, Arc<AppState>>) -> Result<usize, String> {
    let granola_config = state
        .config
        .read()
        .map_err(|_| "Lock poisoned".to_string())?
        .as_ref()
        .map(|c| c.granola.clone())
        .unwrap_or_default();

    let path = crate::granola::resolve_cache_path(&granola_config)
        .ok_or("Granola cache file not found")?;

    crate::granola::cache::count_documents(&path)
}

// ═══════════════════════════════════════════════════════════════════════════
// I229: Gravatar MCP Integration
// ═══════════════════════════════════════════════════════════════════════════

/// Gravatar integration status for the settings UI.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GravatarStatus {
    pub enabled: bool,
    pub cached_count: i64,
    pub api_key_set: bool,
}

/// Get Gravatar integration status.
#[tauri::command]
pub fn get_gravatar_status(state: State<'_, Arc<AppState>>) -> GravatarStatus {
    let config = state
        .config
        .read()
        .ok()
        .and_then(|g| g.as_ref().map(|c| c.gravatar.clone()));

    let gravatar_config = config.unwrap_or_default();

    let cached_count = state
        .db
        .lock()
        .ok()
        .and_then(|g| {
            g.as_ref()
                .map(|db| crate::gravatar::cache::count_cached(db.conn_ref()))
        })
        .unwrap_or(0);

    GravatarStatus {
        enabled: gravatar_config.enabled,
        cached_count,
        api_key_set: gravatar_config.api_key.is_some(),
    }
}

/// Enable or disable Gravatar integration.
#[tauri::command]
pub fn set_gravatar_enabled(enabled: bool, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    crate::state::create_or_update_config(&state, |config| {
        config.gravatar.enabled = enabled;
    })?;
    Ok(())
}

/// Set or clear the Gravatar API key.
#[tauri::command]
pub fn set_gravatar_api_key(
    key: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    crate::state::create_or_update_config(&state, |config| {
        config.gravatar.api_key = key.filter(|k| !k.is_empty());
    })?;
    Ok(())
}

/// Fetch Gravatar data for a single person on demand.
#[tauri::command]
pub async fn fetch_gravatar(
    person_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    // Look up person's email
    let pid = person_id.clone();
    let email = state
        .db_read(move |db| {
            db.conn_ref()
            .query_row(
                "SELECT email FROM person_emails WHERE person_id = ?1 AND is_primary = 1 LIMIT 1",
                [&pid],
                |row| row.get::<_, String>(0),
            )
            .map_err(|_| format!("No email found for person {}", pid))
        })
        .await?;

    let api_key = state
        .config
        .read()
        .ok()
        .and_then(|g| g.as_ref().and_then(|c| c.gravatar.api_key.clone()));

    // Connect and fetch
    let client = crate::gravatar::client::GravatarClient::connect(api_key.as_deref())
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;

    let profile = client.get_profile(&email).await.unwrap_or_default();

    let data_dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".dailyos")
        .join("avatars");
    let _ = std::fs::create_dir_all(&data_dir);

    let avatar_path = match client.get_avatar(&email, 200).await {
        Ok(Some(bytes)) => {
            use sha2::{Digest, Sha256};
            let hash = Sha256::digest(email.as_bytes());
            let hash_hex = hex::encode(&hash[..8]);
            let path = data_dir.join(format!("{}.png", hash_hex));
            if std::fs::write(&path, &bytes).is_ok() {
                Some(path.to_string_lossy().to_string())
            } else {
                None
            }
        }
        _ => None,
    };

    let interests = client.get_interests(&email).await.unwrap_or_default();

    client.disconnect().await;

    // Cache result
    let has_gravatar = profile.display_name.is_some() || avatar_path.is_some();
    let cache_entry = crate::gravatar::cache::CachedGravatar {
        email: email.clone(),
        avatar_url: avatar_path,
        display_name: profile.display_name,
        bio: profile.bio,
        location: profile.location,
        company: profile.company,
        job_title: profile.job_title,
        interests_json: if interests.is_empty() {
            None
        } else {
            serde_json::to_string(&interests).ok()
        },
        has_gravatar,
        fetched_at: chrono::Utc::now().to_rfc3339(),
        person_id: Some(person_id),
    };

    state
        .db_write(move |db| crate::gravatar::cache::upsert_cache(db.conn_ref(), &cache_entry))
        .await?;

    Ok(())
}

/// Batch fetch Gravatar data for all people with stale or missing cache.
#[tauri::command]
pub async fn bulk_fetch_gravatars(state: State<'_, Arc<AppState>>) -> Result<usize, String> {
    let api_key = state
        .config
        .read()
        .ok()
        .and_then(|g| g.as_ref().and_then(|c| c.gravatar.api_key.clone()));

    let emails_to_fetch: Vec<(String, Option<String>)> = state
        .db_read(|db| crate::gravatar::cache::get_stale_emails(db.conn_ref(), 100))
        .await?;

    if emails_to_fetch.is_empty() {
        return Ok(0);
    }

    let client = crate::gravatar::client::GravatarClient::connect(api_key.as_deref())
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;

    let data_dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".dailyos")
        .join("avatars");
    let _ = std::fs::create_dir_all(&data_dir);

    let mut fetched = 0;
    for (email, person_id) in &emails_to_fetch {
        let profile = client.get_profile(email).await.unwrap_or_default();

        let avatar_path = match client.get_avatar(email, 200).await {
            Ok(Some(bytes)) => {
                use sha2::{Digest, Sha256};
                let hash = Sha256::digest(email.as_bytes());
                let hash_hex = hex::encode(&hash[..8]);
                let path = data_dir.join(format!("{}.png", hash_hex));
                if std::fs::write(&path, &bytes).is_ok() {
                    Some(path.to_string_lossy().to_string())
                } else {
                    None
                }
            }
            _ => None,
        };

        let interests = client.get_interests(email).await.unwrap_or_default();

        let has_gravatar = profile.display_name.is_some() || avatar_path.is_some();
        let cache_entry = crate::gravatar::cache::CachedGravatar {
            email: email.clone(),
            avatar_url: avatar_path,
            display_name: profile.display_name,
            bio: profile.bio,
            location: profile.location,
            company: profile.company,
            job_title: profile.job_title,
            interests_json: if interests.is_empty() {
                None
            } else {
                serde_json::to_string(&interests).ok()
            },
            has_gravatar,
            fetched_at: chrono::Utc::now().to_rfc3339(),
            person_id: person_id.clone(),
        };

        let _ = state
            .db_write(move |db| crate::gravatar::cache::upsert_cache(db.conn_ref(), &cache_entry))
            .await;

        fetched += 1;
        // Rate limit: 1 req/sec
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }

    client.disconnect().await;
    Ok(fetched)
}

/// Get avatar for a person as a data URL (base64-encoded PNG).
/// Returns None if no cached avatar exists.
#[tauri::command]
pub async fn get_person_avatar(
    person_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Option<String>, String> {
    let path = match state
        .db_read(move |db| {
            Ok(crate::gravatar::cache::get_avatar_url_for_person(
                db.conn_ref(),
                &person_id,
            ))
        })
        .await
    {
        Ok(Some(p)) => p,
        _ => return Ok(None),
    };
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(_) => return Ok(None),
    };
    let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes);
    Ok(Some(format!("data:image/png;base64,{}", b64)))
}

// ═══════════════════════════════════════════════════════════════════════════
// I228: Clay Contact & Company Enrichment
// ═══════════════════════════════════════════════════════════════════════════

/// Clay integration status for the settings UI.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClayStatusData {
    pub enabled: bool,
    pub api_key_set: bool,
    pub auto_enrich_on_create: bool,
    pub sweep_interval_hours: u32,
    pub enriched_count: i64,
    pub pending_count: i64,
    pub last_enrichment_at: Option<String>,
}

/// Get Clay integration status.
#[tauri::command]
pub async fn get_clay_status(state: State<'_, Arc<AppState>>) -> Result<ClayStatusData, String> {
    let config = state
        .config
        .read()
        .ok()
        .and_then(|g| g.as_ref().map(|c| c.clay.clone()));

    let clay_config = config.unwrap_or_default();

    let (enriched_count, pending_count, last_enrichment) = state
        .db_read(|db| {
            let enriched: i64 = db
                .conn_ref()
                .query_row(
                    "SELECT COUNT(*) FROM people WHERE last_enriched_at IS NOT NULL",
                    [],
                    |row| row.get(0),
                )
                .unwrap_or(0);
            let pending: i64 = db
                .conn_ref()
                .query_row(
                    "SELECT COUNT(*) FROM clay_sync_state WHERE state = 'pending'",
                    [],
                    |row| row.get(0),
                )
                .unwrap_or(0);
            let last: Option<String> = db
                .conn_ref()
                .query_row("SELECT MAX(last_enriched_at) FROM people", [], |row| {
                    row.get(0)
                })
                .unwrap_or(None);
            Ok((enriched, pending, last))
        })
        .await
        .unwrap_or((0, 0, None));

    Ok(ClayStatusData {
        enabled: clay_config.enabled,
        api_key_set: clay_config.api_key.is_some(),
        auto_enrich_on_create: clay_config.auto_enrich_on_create,
        sweep_interval_hours: clay_config.sweep_interval_hours,
        enriched_count,
        pending_count,
        last_enrichment_at: last_enrichment,
    })
}

/// Enable or disable Clay integration.
#[tauri::command]
pub fn set_clay_enabled(enabled: bool, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    crate::state::create_or_update_config(&state, |config| {
        config.clay.enabled = enabled;
    })?;
    Ok(())
}

/// Set or clear the Clay API key.
#[tauri::command]
pub fn set_clay_api_key(
    key: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    crate::state::create_or_update_config(&state, |config| {
        config.clay.api_key = key.filter(|k| !k.is_empty());
    })?;
    Ok(())
}

/// Toggle auto-enrich on person creation.
#[tauri::command]
pub fn set_clay_auto_enrich(enabled: bool, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    crate::state::create_or_update_config(&state, |config| {
        config.clay.auto_enrich_on_create = enabled;
    })?;
    Ok(())
}

/// Resolve Smithery credentials for Clay MCP: API key from keychain +
/// namespace and connection ID from config.
fn resolve_smithery_config(state: &AppState) -> Result<(String, String, String), String> {
    let api_key = crate::clay::oauth::get_smithery_api_key()
        .ok_or("No Smithery API key. Configure in Settings \u{2192} Connectors \u{2192} Clay.")?;
    let config = state
        .config
        .read()
        .ok()
        .and_then(|g| g.as_ref().map(|c| c.clay.clone()));
    let clay = config.ok_or("Config not loaded")?;
    let ns = clay
        .smithery_namespace
        .ok_or("Smithery namespace not configured")?;
    let conn = clay
        .smithery_connection_id
        .ok_or("Smithery connection ID not configured")?;
    Ok((api_key, ns, conn))
}

/// Test Clay connection by attempting to connect via Smithery.
#[tauri::command]
pub async fn test_clay_connection(state: State<'_, Arc<AppState>>) -> Result<bool, String> {
    let (api_key, ns, conn) = resolve_smithery_config(&state)?;

    let client = crate::clay::client::ClayClient::connect(&api_key, &ns, &conn)
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;

    client.disconnect().await;
    Ok(true)
}

/// Enrichment result for the frontend.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnrichmentResultData {
    pub person_id: String,
    pub fields_updated: Vec<String>,
    pub signals: Vec<String>,
}

/// Enrich a single person from Clay on demand.
#[tauri::command]
pub async fn enrich_person_from_clay(
    person_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<EnrichmentResultData, String> {
    let (api_key, ns, conn) = resolve_smithery_config(&state)?;

    let client = crate::clay::client::ClayClient::connect(&api_key, &ns, &conn)
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;

    let result =
        crate::clay::enricher::enrich_person_from_clay_with_client(&state, &person_id, &client)
            .await?;

    client.disconnect().await;

    Ok(EnrichmentResultData {
        person_id: result.person_id,
        fields_updated: result.fields_updated,
        signals: result.signals,
    })
}

/// Enrich an account's company data from Clay (via linked people).
#[tauri::command]
pub async fn enrich_account_from_clay(
    account_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<EnrichmentResultData, String> {
    // Find a linked person for this account, enrich them, company data follows
    let person_id: Option<String> = state.db.lock().ok().and_then(|g| {
        g.as_ref().and_then(|db| {
            db.conn_ref()
                .query_row(
                    "SELECT person_id FROM account_stakeholders WHERE account_id = ?1 LIMIT 1",
                    [&account_id],
                    |row| row.get(0),
                )
                .ok()
        })
    });

    let person_id = person_id.ok_or("No linked people found for this account")?;

    let (api_key, ns, conn) = resolve_smithery_config(&state)?;

    let client = crate::clay::client::ClayClient::connect(&api_key, &ns, &conn)
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;

    let result =
        crate::clay::enricher::enrich_person_from_clay_with_client(&state, &person_id, &client)
            .await?;

    client.disconnect().await;

    Ok(EnrichmentResultData {
        person_id: result.person_id,
        fields_updated: result.fields_updated,
        signals: result.signals,
    })
}

/// Bulk enrichment result.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkEnrichResult {
    pub queued: usize,
    pub total_unenriched: usize,
}

/// Start bulk Clay enrichment for all unenriched people.
#[tauri::command]
pub async fn start_clay_bulk_enrich(
    state: State<'_, Arc<AppState>>,
) -> Result<BulkEnrichResult, String> {
    let unenriched = state
        .db_read(|db| {
            let mut stmt = db
                .conn_ref()
                .prepare("SELECT id FROM people WHERE last_enriched_at IS NULL AND archived = 0")
                .map_err(|e| e.to_string())?;
            let unenriched: Vec<String> = stmt
                .query_map([], |row| row.get(0))
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .collect();

            Ok(unenriched)
        })
        .await?;
    let total = state
        .db_write(move |db| crate::services::mutations::queue_clay_sync_for_people(db, &unenriched))
        .await?;

    // Wake the enrichment processor immediately to process queued items
    state.integrations.enrichment_wake.notify_one();

    Ok(BulkEnrichResult {
        queued: total,
        total_unenriched: total,
    })
}

/// Enrichment log entry for the frontend.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnrichmentLogEntry {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub source: String,
    pub event_type: String,
    pub signal_type: Option<String>,
    pub fields_updated: Option<String>,
    pub created_at: String,
}

/// Get enrichment log entries for an entity.
#[tauri::command]
pub async fn get_enrichment_log(
    entity_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<EnrichmentLogEntry>, String> {
    state.db_read(move |db| {
        let mut stmt = db
            .conn_ref()
            .prepare(
                "SELECT id, entity_type, entity_id, source, event_type, signal_type, fields_updated, created_at
                 FROM enrichment_log
                 WHERE entity_id = ?1
                 ORDER BY created_at DESC
                 LIMIT 50",
            )
            .map_err(|e| e.to_string())?;

        let entries = stmt
            .query_map([&entity_id], |row| {
                Ok(EnrichmentLogEntry {
                    id: row.get(0)?,
                    entity_type: row.get(1)?,
                    entity_id: row.get(2)?,
                    source: row.get(3)?,
                    event_type: row.get(4)?,
                    signal_type: row.get(5)?,
                    fields_updated: row.get(6)?,
                    created_at: row.get(7)?,
                })
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        Ok(entries)
    }).await
}

// ---------------------------------------------------------------------------
// Clay — Smithery Connect (I422)
// ---------------------------------------------------------------------------

/// Auto-detect Smithery settings and Clay connection from CLI config + API.
#[tauri::command]
pub async fn detect_smithery_settings() -> Result<serde_json::Value, String> {
    let settings_path = dirs::home_dir()
        .ok_or("No home directory")?
        .join("Library/Application Support/smithery/settings.json");

    if !settings_path.exists() {
        return Err("Smithery CLI not configured. Run: npx @smithery/cli login".to_string());
    }

    let content = std::fs::read_to_string(&settings_path)
        .map_err(|e| format!("Failed to read Smithery settings: {}", e))?;

    let val: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse Smithery settings: {}", e))?;

    let api_key = val.get("apiKey").and_then(|v| v.as_str()).unwrap_or("");
    let namespace = val.get("namespace").and_then(|v| v.as_str()).unwrap_or("");

    if api_key.is_empty() || namespace.is_empty() {
        return Err("Smithery settings missing apiKey or namespace".to_string());
    }

    // List connections via Smithery API to find the Clay one
    let client = reqwest::Client::new();
    let connections_url = format!("https://api.smithery.ai/connect/{}", namespace);
    let clay_connection_id = match client
        .get(&connections_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            let body = resp.text().await.unwrap_or_default();
            let parsed: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
            // Find a connection whose name/mcpUrl contains "clay"
            parsed
                .get("connections")
                .and_then(|c| c.as_array())
                .and_then(|arr| {
                    arr.iter().find(|conn| {
                        let name = conn.get("name").and_then(|n| n.as_str()).unwrap_or("");
                        let url = conn.get("mcpUrl").and_then(|u| u.as_str()).unwrap_or("");
                        let status = conn
                            .get("status")
                            .and_then(|s| s.get("state"))
                            .and_then(|s| s.as_str())
                            .unwrap_or("");
                        (name.contains("clay") || url.contains("clay")) && status == "connected"
                    })
                })
                .and_then(|conn| conn.get("connectionId").and_then(|id| id.as_str()))
                .map(String::from)
        }
        _ => None,
    };

    Ok(serde_json::json!({
        "apiKey": api_key,
        "namespace": namespace,
        "connectionId": clay_connection_id,
    }))
}

/// Save Smithery API key to keychain.
#[tauri::command]
pub async fn save_smithery_api_key(key: String) -> Result<(), String> {
    let trimmed = key.trim().to_string();
    if trimmed.is_empty() {
        return Err("API key cannot be empty".to_string());
    }
    crate::clay::oauth::save_smithery_api_key(&trimmed)
}

/// Save Smithery connection config (namespace + connection ID).
#[tauri::command]
pub fn set_smithery_connection(
    namespace: String,
    connection_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let ns = if namespace.trim().is_empty() {
        None
    } else {
        Some(namespace)
    };
    let conn = if connection_id.trim().is_empty() {
        None
    } else {
        Some(connection_id)
    };
    crate::state::create_or_update_config(&state, |config| {
        config.clay.smithery_namespace = ns.clone();
        config.clay.smithery_connection_id = conn.clone();
    })?;
    Ok(())
}

/// Disconnect Smithery — remove keychain entry and clear config fields.
#[tauri::command]
pub fn disconnect_smithery(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    crate::clay::oauth::delete_smithery_api_key()?;
    crate::state::create_or_update_config(&state, |config| {
        config.clay.smithery_namespace = None;
        config.clay.smithery_connection_id = None;
    })?;

    let _ = state.with_db_write(|db| {
        crate::db::data_lifecycle::purge_source(db, crate::db::data_lifecycle::DataSource::Clay)
            .map_err(|e| e.to_string())
    })?;
    Ok(())
}

/// Get Smithery connection status.
#[tauri::command]
pub fn get_smithery_status(state: State<'_, Arc<AppState>>) -> serde_json::Value {
    let has_api_key = crate::clay::oauth::get_smithery_api_key().is_some();
    let (namespace, connection_id) = state
        .config
        .read()
        .ok()
        .and_then(|g| {
            g.as_ref().map(|c| {
                (
                    c.clay.smithery_namespace.clone(),
                    c.clay.smithery_connection_id.clone(),
                )
            })
        })
        .unwrap_or((None, None));

    let connected = has_api_key && namespace.is_some() && connection_id.is_some();

    serde_json::json!({
        "connected": connected,
        "hasApiKey": has_api_key,
        "namespace": namespace,
        "connectionId": connection_id,
    })
}

// =============================================================================
// I346: Linear Integration
// =============================================================================

/// Linear integration status for the frontend.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinearStatusData {
    pub enabled: bool,
    pub api_key_set: bool,
    pub poll_interval_minutes: u32,
    pub issue_count: i64,
    pub project_count: i64,
    pub last_sync_at: Option<String>,
}

/// Get Linear integration status.
#[tauri::command]
pub fn get_linear_status(state: State<'_, Arc<AppState>>) -> LinearStatusData {
    let config = state
        .config
        .read()
        .ok()
        .and_then(|g| g.as_ref().map(|c| c.linear.clone()));

    let linear_config = config.unwrap_or_default();

    let (issue_count, project_count, last_sync) = state
        .db
        .lock()
        .ok()
        .and_then(|g| {
            g.as_ref().map(|db| {
                let issues: i64 = db
                    .conn_ref()
                    .query_row("SELECT COUNT(*) FROM linear_issues", [], |row| row.get(0))
                    .unwrap_or(0);
                let projects: i64 = db
                    .conn_ref()
                    .query_row("SELECT COUNT(*) FROM linear_projects", [], |row| row.get(0))
                    .unwrap_or(0);
                let last: Option<String> = db
                    .conn_ref()
                    .query_row("SELECT MAX(synced_at) FROM linear_issues", [], |row| {
                        row.get(0)
                    })
                    .unwrap_or(None);
                (issues, projects, last)
            })
        })
        .unwrap_or((0, 0, None));

    LinearStatusData {
        enabled: linear_config.enabled,
        api_key_set: linear_config.api_key.is_some(),
        poll_interval_minutes: linear_config.poll_interval_minutes,
        issue_count,
        project_count,
        last_sync_at: last_sync,
    }
}

/// Enable or disable Linear integration.
#[tauri::command]
pub fn set_linear_enabled(enabled: bool, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    crate::state::create_or_update_config(&state, |config| {
        config.linear.enabled = enabled;
    })?;
    Ok(())
}

/// Set or clear the Linear API key.
#[tauri::command]
pub fn set_linear_api_key(
    key: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    crate::state::create_or_update_config(&state, |config| {
        config.linear.api_key = key.filter(|k| !k.is_empty());
    })?;
    Ok(())
}

/// Test Linear connection by fetching the viewer.
#[tauri::command]
pub async fn test_linear_connection(state: State<'_, Arc<AppState>>) -> Result<String, String> {
    let api_key = state
        .config
        .read()
        .ok()
        .and_then(|g| g.as_ref().and_then(|c| c.linear.api_key.clone()))
        .ok_or("No Linear API key configured")?;

    let client = crate::linear::client::LinearClient::new(&api_key);
    let viewer = client.test_connection().await?;
    Ok(viewer.name)
}

/// Trigger an immediate Linear sync.
#[tauri::command]
pub fn start_linear_sync(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.integrations.linear_poller_wake.notify_one();
    Ok(())
}

/// I425: Get the 5 most recently synced Linear issues.
#[tauri::command]
pub async fn get_linear_recent_issues(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<serde_json::Value>, String> {
    state.db_read(|db| {
        let mut stmt = db.conn_ref().prepare(
            "SELECT id, identifier, title, state_name, state_type, priority_label, due_date, synced_at
             FROM linear_issues
             WHERE state_type NOT IN ('completed', 'cancelled')
             ORDER BY priority ASC, synced_at DESC LIMIT 5"
        ).map_err(|e| e.to_string())?;
        let issues = stmt.query_map([], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "identifier": row.get::<_, String>(1)?,
                "title": row.get::<_, String>(2)?,
                "stateName": row.get::<_, Option<String>>(3)?,
                "stateType": row.get::<_, Option<String>>(4)?,
                "priorityLabel": row.get::<_, Option<String>>(5)?,
                "dueDate": row.get::<_, Option<String>>(6)?,
                "syncedAt": row.get::<_, Option<String>>(7)?,
            }))
        }).map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
        Ok(issues)
    }).await
}

/// I425: Get all Linear entity links with project and entity names.
#[tauri::command]
pub async fn get_linear_entity_links(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<serde_json::Value>, String> {
    state
        .db_read(|db| {
            let mut stmt = db
                .conn_ref()
                .prepare(
                    "SELECT lel.id, lel.linear_project_id, lp.name as project_name,
                    lel.entity_id, lel.entity_type, lel.confirmed,
                    CASE lel.entity_type
                        WHEN 'account' THEN (SELECT name FROM accounts WHERE id = lel.entity_id)
                        WHEN 'project' THEN (SELECT name FROM projects WHERE id = lel.entity_id)
                        WHEN 'person' THEN (SELECT name FROM people WHERE id = lel.entity_id)
                    END as entity_name
             FROM linear_entity_links lel
             LEFT JOIN linear_projects lp ON lp.id = lel.linear_project_id
             ORDER BY lel.created_at DESC",
                )
                .map_err(|e| e.to_string())?;
            let links = stmt
                .query_map([], |row| {
                    Ok(serde_json::json!({
                        "id": row.get::<_, String>(0)?,
                        "linearProjectId": row.get::<_, String>(1)?,
                        "projectName": row.get::<_, Option<String>>(2)?,
                        "entityId": row.get::<_, String>(3)?,
                        "entityType": row.get::<_, String>(4)?,
                        "confirmed": row.get::<_, bool>(5)?,
                        "entityName": row.get::<_, Option<String>>(6)?,
                    }))
                })
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .collect();
            Ok(links)
        })
        .await
}

/// I425: Auto-detect entity links by fuzzy-matching Linear project names to entity names.
#[tauri::command]
pub async fn run_linear_auto_link(state: State<'_, Arc<AppState>>) -> Result<usize, String> {
    state
        .db_write(|db| {
            let conn = db.conn_ref();
            let mut linked = 0usize;

            // Get all Linear projects
            let projects: Vec<(String, String)> = {
                let mut stmt = conn
                    .prepare("SELECT id, name FROM linear_projects")
                    .map_err(|e| e.to_string())?;
                let rows = stmt
                    .query_map([], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                    })
                    .map_err(|e| e.to_string())?
                    .filter_map(|r| r.ok())
                    .collect();
                rows
            };

            for (project_id, project_name) in &projects {
                let lower_name = project_name.to_lowercase();

                // Try matching against accounts (exact case-insensitive)
                let account_match: Option<String> = conn
                    .query_row(
                        "SELECT id FROM accounts WHERE LOWER(name) = ?1 AND archived = 0",
                        [&lower_name],
                        |row| row.get(0),
                    )
                    .ok();

                if let Some(account_id) = account_match {
                    crate::services::mutations::create_linear_entity_link_with_confirmed(
                        db,
                        project_id,
                        &account_id,
                        "account",
                        false,
                    )?;
                    linked += 1;
                    continue;
                }

                // Try matching against projects (exact case-insensitive)
                let project_match: Option<String> = conn
                    .query_row(
                        "SELECT id FROM projects WHERE LOWER(name) = ?1 AND archived = 0",
                        [&lower_name],
                        |row| row.get(0),
                    )
                    .ok();

                if let Some(proj_id) = project_match {
                    crate::services::mutations::create_linear_entity_link_with_confirmed(
                        db, project_id, &proj_id, "project", false,
                    )?;
                    linked += 1;
                }
            }

            Ok(linked)
        })
        .await
}

/// I425: Delete a Linear entity link.
#[tauri::command]
pub async fn delete_linear_entity_link(
    state: State<'_, Arc<AppState>>,
    link_id: String,
) -> Result<(), String> {
    state
        .db_write(move |db| crate::services::mutations::delete_linear_entity_link(db, &link_id))
        .await
}

/// List all Linear projects for the manual link picker.
#[tauri::command]
pub async fn get_linear_projects(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<serde_json::Value>, String> {
    state
        .db_read(|db| {
            let mut stmt = db
                .conn_ref()
                .prepare("SELECT id, name FROM linear_projects ORDER BY name ASC")
                .map_err(|e| e.to_string())?;
            let projects = stmt
                .query_map([], |row| {
                    Ok(serde_json::json!({
                        "id": row.get::<_, String>(0)?,
                        "name": row.get::<_, String>(1)?,
                    }))
                })
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .collect();
            Ok(projects)
        })
        .await
}

/// Manually create a Linear entity link.
#[tauri::command]
pub async fn create_linear_entity_link(
    services: State<'_, crate::services::ServiceLayer>,
    linear_project_id: String,
    entity_id: String,
    entity_type: String,
) -> Result<(), String> {
    let state = services.state();
    if !["account", "project"].contains(&entity_type.as_str()) {
        return Err("entity_type must be 'account' or 'project'".to_string());
    }
    state
        .db_write(move |db| {
            crate::services::mutations::create_linear_entity_link(
                db,
                &linear_project_id,
                &entity_id,
                &entity_type,
            )
        })
        .await
}

// =============================================================================
// I309: Role Presets
// =============================================================================

/// Set the active role preset.
#[tauri::command]
pub async fn set_role(
    role: String,
    state: State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    let preset = crate::presets::loader::load_preset(&role)?;

    crate::state::create_or_update_config(&state, |c| {
        c.role = role.clone();
        c.custom_preset_path = None;
        c.entity_mode = preset.default_entity_mode.clone();
        c.profile = crate::types::profile_for_entity_mode(&c.entity_mode);
    })?;

    if let Ok(mut guard) = state.active_preset.write() {
        *guard = Some(preset);
    }

    let _ = app_handle.emit("config-updated", ());
    Ok("ok".to_string())
}

/// Get the currently active role preset.
#[tauri::command]
pub async fn get_active_preset(
    state: State<'_, Arc<AppState>>,
) -> Result<Option<crate::presets::schema::RolePreset>, String> {
    Ok(state
        .active_preset
        .read()
        .map_err(|_| "Lock poisoned")?
        .clone())
}

/// List all available role presets.
#[tauri::command]
pub async fn get_available_presets() -> Result<Vec<(String, String, String)>, String> {
    Ok(crate::presets::loader::get_available_presets())
}

// =============================================================================
// I311: Entity Metadata
// =============================================================================

/// Update JSON metadata for an entity (account or project).
#[tauri::command]
pub async fn update_entity_metadata(
    entity_type: String,
    entity_id: String,
    metadata: String,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    serde_json::from_str::<serde_json::Value>(&metadata)
        .map_err(|e| format!("Invalid JSON metadata: {}", e))?;
    let engine = state.signals.engine.clone();
    state
        .db_write(move |db| {
            crate::services::mutations::update_entity_metadata(
                db,
                &engine,
                &entity_type,
                &entity_id,
                &metadata,
            )
        })
        .await?;
    Ok("ok".to_string())
}

/// Get JSON metadata for an entity (account or project).
#[tauri::command]
pub async fn get_entity_metadata(
    entity_type: String,
    entity_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    state
        .db_read(move |db| db.get_entity_metadata(&entity_type, &entity_id))
        .await
}

// =============================================================================
// I323: Email Disposition Correction
// =============================================================================

/// Correct an email disposition (I323).
/// Records a feedback signal for Thompson Sampling priority recalibration.
/// Does NOT un-archive the email (user can find it in Gmail "All Mail").
#[tauri::command]
pub async fn correct_email_disposition(
    email_id: String,
    corrected_priority: String,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let valid_priorities = ["high", "medium", "low"];
    if !valid_priorities.contains(&corrected_priority.as_str()) {
        return Err(format!(
            "Invalid priority: {}. Must be high, medium, or low.",
            corrected_priority
        ));
    }

    state
        .db_write(move |db| {
            crate::services::mutations::upsert_email_feedback_signal(
                db,
                &email_id,
                &corrected_priority,
            )?;

            log::info!(
                "correct_email_disposition: {} corrected to {}",
                email_id,
                corrected_priority
            );
            Ok(format!("Disposition corrected to {}", corrected_priority))
        })
        .await
}

// =============================================================================
// I330: Meeting Timeline (±7 days)
// =============================================================================

/// Return meetings for +/-N days around today with intelligence quality data.
///
/// Always-live: if no future meetings exist in `meetings`, fetches from
/// Google Calendar and upserts stubs so the timeline populates on first load
/// without waiting for scheduled workflows.
#[tauri::command]
pub async fn get_meeting_timeline(
    state: State<'_, Arc<AppState>>,
    days_before: Option<i64>,
    days_after: Option<i64>,
) -> Result<Vec<crate::types::TimelineMeeting>, String> {
    let days_after_val = days_after.unwrap_or(7);
    let result =
        crate::services::meetings::get_meeting_timeline(&state, days_before, days_after).await?;

    // Check if we have any meetings AFTER today (i.e., tomorrow or later)
    let tomorrow_str = (chrono::Local::now().date_naive() + chrono::Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();
    let has_future = result
        .iter()
        .any(|m| m.start_time.as_str() >= tomorrow_str.as_str());
    if has_future || days_after_val == 0 {
        // Enqueue future meetings that have no prep_frozen_json yet
        let ts = tomorrow_str.clone();
        let needs_prep: Vec<String> = state
            .db_read(move |db| {
                Ok(db
                    .conn_ref()
                    .prepare(
                        "SELECT m.id FROM meetings m
                     LEFT JOIN meeting_prep mp ON mp.meeting_id = m.id
                     WHERE m.start_time >= ?1
                       AND mp.prep_frozen_json IS NULL
                       AND m.meeting_type NOT IN ('personal', 'focus', 'blocked')",
                    )
                    .and_then(|mut stmt| {
                        let rows =
                            stmt.query_map(rusqlite::params![ts], |row| row.get::<_, String>(0))?;
                        Ok(rows.filter_map(|r| r.ok()).collect())
                    })
                    .unwrap_or_default())
            })
            .await
            .unwrap_or_default();
        if !needs_prep.is_empty() {
            log::info!(
                "get_meeting_timeline: enqueuing {} future meetings without prep",
                needs_prep.len()
            );
            for mid in needs_prep {
                state
                    .meeting_prep_queue
                    .enqueue(crate::meeting_prep_queue::PrepRequest {
                        meeting_id: mid,
                        priority: crate::meeting_prep_queue::PrepPriority::PageLoad,
                        requested_at: std::time::Instant::now(),
                    });
            }
            state.integrations.prep_queue_wake.notify_one();
        }
        return Ok(result);
    }

    // No future meetings in DB — try live fetch from Google Calendar
    let access_token = match crate::google_api::get_valid_access_token().await {
        Ok(t) => t,
        Err(_) => return Ok(result), // No auth — return what we have
    };

    let today = chrono::Local::now().date_naive();
    let range_end = today + chrono::Duration::days(days_after_val);
    let raw_events = match crate::google_api::calendar::fetch_events(
        &access_token,
        today + chrono::Duration::days(1), // tomorrow onward (today already covered)
        range_end,
    )
    .await
    {
        Ok(events) => events,
        Err(e) => {
            log::warn!("get_meeting_timeline: live calendar fetch failed: {}", e);
            return Ok(result);
        }
    };

    if raw_events.is_empty() {
        return Ok(result);
    }

    // Classify and upsert into meetings (same pattern as prepare_today)
    let user_domains = state
        .config
        .read()
        .ok()
        .and_then(|g| g.as_ref().map(|c| c.resolved_user_domains()))
        .unwrap_or_default();
    let entity_hints = state
        .db_read(|db| Ok(crate::helpers::build_entity_hints(db)))
        .await?;

    // Classify events first (no DB needed)
    let mut to_upsert: Vec<(
        crate::types::CalendarEvent,
        Vec<crate::google_api::classify::ResolvedMeetingEntity>,
    )> = Vec::new();
    for raw in &raw_events {
        let cm =
            crate::google_api::classify::classify_meeting_multi(raw, &user_domains, &entity_hints);
        let event = cm.to_calendar_event();

        // Skip personal (matches timeline query filter)
        if matches!(event.meeting_type, crate::types::MeetingType::Personal) {
            continue;
        }
        let resolved = cm.resolved_entities.clone();
        to_upsert.push((event, resolved));
    }

    // Batch upsert in a single DB write
    let upserted_ids = state
        .db_write(move |db| {
            let mut ids: Vec<String> = Vec::new();
            for (event, resolved_entities) in &to_upsert {
                // Only insert if not already present
                if db
                    .get_meeting_by_calendar_event_id(&event.id)
                    .ok()
                    .flatten()
                    .is_some()
                {
                    continue;
                }

                let attendees_json = if event.attendees.is_empty() {
                    None
                } else {
                    Some(serde_json::to_string(&event.attendees).unwrap_or_default())
                };

                let db_meeting = crate::db::DbMeeting {
                    id: event.id.clone(),
                    title: event.title.clone(),
                    meeting_type: event.meeting_type.as_str().to_string(),
                    start_time: event.start.to_rfc3339(),
                    end_time: Some(event.end.to_rfc3339()),
                    attendees: attendees_json,
                    notes_path: None,
                    summary: None,
                    created_at: chrono::Utc::now().to_rfc3339(),
                    calendar_event_id: Some(event.id.clone()),
                    description: None,
                    prep_context_json: None,
                    user_agenda_json: None,
                    user_notes: None,
                    prep_frozen_json: None,
                    prep_frozen_at: None,
                    prep_snapshot_path: None,
                    prep_snapshot_hash: None,
                    transcript_path: None,
                    transcript_processed_at: None,
                    intelligence_state: None,
                    intelligence_quality: None,
                    last_enriched_at: None,
                    signal_count: None,
                    has_new_signals: None,
                    last_viewed_at: None,
                };
                let links: Vec<(String, String)> = resolved_entities
                    .iter()
                    .map(|re| (re.entity_id.clone(), re.entity_type.clone()))
                    .collect();
                if let Err(e) = crate::services::mutations::upsert_timeline_meeting_with_entities(
                    db,
                    &db_meeting,
                    &links,
                ) {
                    log::warn!(
                        "get_meeting_timeline: failed to upsert '{}': {}",
                        event.title,
                        e
                    );
                    continue;
                }

                ids.push(event.id.clone());
            }
            Ok(ids)
        })
        .await?;

    let upserted = upserted_ids.len() as u32;

    if upserted > 0 {
        log::info!(
            "get_meeting_timeline: upserted {} future meetings from Google Calendar",
            upserted
        );

        // Enqueue newly upserted meetings for prep generation
        for mid in &upserted_ids {
            state
                .meeting_prep_queue
                .enqueue(crate::meeting_prep_queue::PrepRequest {
                    meeting_id: mid.clone(),
                    priority: crate::meeting_prep_queue::PrepPriority::PageLoad,
                    requested_at: std::time::Instant::now(),
                });
        }
        if !upserted_ids.is_empty() {
            state.integrations.prep_queue_wake.notify_one();
        }

        // Re-query with the newly upserted meetings
        return crate::services::meetings::get_meeting_timeline(&state, days_before, days_after)
            .await;
    }

    Ok(result)
}

// =============================================================================
// I390: Person Relationships (ADR-0088)
// =============================================================================

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationshipPayload {
    /// Pass an existing ID to update; omit for a new relationship.
    pub id: Option<String>,
    pub from_person_id: String,
    pub to_person_id: String,
    pub relationship_type: String,
    #[serde(default = "default_rel_direction")]
    pub direction: String,
    #[serde(default = "default_rel_confidence")]
    pub confidence: f64,
    pub context_entity_id: Option<String>,
    pub context_entity_type: Option<String>,
    #[serde(default = "default_rel_source")]
    pub source: String,
}

fn default_rel_direction() -> String {
    "directed".to_string()
}
fn default_rel_confidence() -> f64 {
    0.8
}
fn default_rel_source() -> String {
    "user_confirmed".to_string()
}

#[tauri::command]
pub async fn upsert_person_relationship(
    state: State<'_, Arc<AppState>>,
    payload: RelationshipPayload,
) -> Result<String, String> {
    // Validate relationship type parses
    payload
        .relationship_type
        .parse::<crate::db::person_relationships::RelationshipType>()
        .map_err(|e| format!("Invalid relationship type: {}", e))?;

    let engine = state.signals.engine.clone();
    state
        .db_write(move |db| {
            let id = payload
                .id
                .unwrap_or_else(|| format!("rel-{}", uuid::Uuid::new_v4()));
            crate::services::mutations::upsert_person_relationship(
                db,
                &engine,
                &id,
                &payload.from_person_id,
                &payload.to_person_id,
                &payload.relationship_type,
                &payload.direction,
                payload.confidence,
                payload.context_entity_id.as_deref(),
                payload.context_entity_type.as_deref(),
                &payload.source,
            )?;
            Ok(id)
        })
        .await
}

#[tauri::command]
pub async fn delete_person_relationship(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<(), String> {
    let engine = state.signals.engine.clone();
    state
        .db_write(move |db| {
            crate::services::mutations::delete_person_relationship(db, &engine, &id)
        })
        .await
}

#[tauri::command]
pub async fn get_person_relationships(
    state: State<'_, Arc<AppState>>,
    person_id: String,
) -> Result<Vec<crate::db::person_relationships::PersonRelationship>, String> {
    state
        .db_read(move |db| {
            db.get_relationships_for_person(&person_id)
                .map_err(|e| format!("Failed to get relationships: {}", e))
        })
        .await
}

// =========================================================================
// Google Drive Connector (I426)
// =========================================================================

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveStatusData {
    pub enabled: bool,
    pub connected: bool,
    pub watched_count: i64,
    pub synced_count: i64,
    pub last_sync_at: Option<String>,
    pub poll_interval_minutes: u32,
}

/// Get a valid Google OAuth access token for use with Drive API and Picker.
/// Returns the token string or an error if not authenticated.
#[tauri::command]
pub async fn get_google_access_token() -> Result<String, String> {
    crate::google_api::get_valid_access_token()
        .await
        .map_err(|e| format!("Failed to get access token: {}", e))
}

/// Get Google API Client ID for use with Google Picker API.
/// Returns the numeric project ID extracted from the full client_id.
#[tauri::command]
pub fn get_google_client_id() -> String {
    // Extract numeric project ID from client_id format: "245504828099-xxx.apps.googleusercontent.com"
    "245504828099".to_string()
}

/// Get Google Drive integration status.
#[tauri::command]
pub async fn get_google_drive_status(
    state: State<'_, Arc<AppState>>,
) -> Result<crate::commands::DriveStatusData, String> {
    let config = state
        .config
        .read()
        .ok()
        .and_then(|g| g.as_ref().map(|c| c.drive.clone()));

    let drive_config = config.unwrap_or_default();

    let connected = state
        .calendar
        .google_auth
        .lock()
        .map(|guard| matches!(*guard, crate::types::GoogleAuthStatus::Authenticated { .. }))
        .unwrap_or(false);

    let (watched_count, synced_count, last_sync) = state
        .db_read(|db| {
            let conn = db.conn_ref();
            let watched: i64 = conn
                .query_row("SELECT COUNT(*) FROM drive_watched_sources", [], |row| {
                    row.get(0)
                })
                .unwrap_or(0);
            let synced: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM drive_watched_sources WHERE last_synced_at IS NOT NULL",
                    [],
                    |row| row.get(0),
                )
                .unwrap_or(0);
            let last: Option<String> = conn
                .query_row(
                    "SELECT MAX(last_synced_at) FROM drive_watched_sources",
                    [],
                    |row| row.get(0),
                )
                .unwrap_or(None);
            Ok((watched, synced, last))
        })
        .await
        .unwrap_or((0, 0, None));

    Ok(crate::commands::DriveStatusData {
        enabled: drive_config.enabled,
        connected,
        watched_count,
        synced_count,
        last_sync_at: last_sync,
        poll_interval_minutes: drive_config.poll_interval_minutes,
    })
}

/// Enable or disable Google Drive integration.
#[tauri::command]
pub fn set_google_drive_enabled(
    enabled: bool,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    crate::state::create_or_update_config(&state, |config| {
        config.drive.enabled = enabled;
    })?;
    Ok(())
}

/// Trigger an immediate Drive sync.
#[tauri::command]
pub fn trigger_drive_sync_now(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.integrations.drive_poller_wake.notify_one();
    Ok(())
}

/// Import a file from Google Drive once (no ongoing sync).
///
/// Downloads the file, converts to markdown, and saves to the entity's
/// Documents/ folder. Does NOT create a watched source entry.
#[tauri::command]
pub async fn import_google_drive_file(
    google_id: String,
    name: String,
    entity_id: String,
    entity_type: String,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let content = crate::google_drive::client::download_file_as_markdown(&google_id).await?;

    let workspace = state
        .config
        .read()
        .ok()
        .and_then(|g| g.as_ref().map(|c| c.workspace_path.clone()))
        .ok_or("Workspace not configured")?;

    let path = crate::google_drive::poller::save_to_entity_docs(
        &workspace,
        &entity_type,
        &entity_id,
        &name,
        &content,
    )?;

    log::info!("Drive import (once): saved {} to {}", name, path.display());
    Ok(path.display().to_string())
}

/// Add a watched Drive source linked to an entity.
#[tauri::command]
pub async fn add_google_drive_watch(
    google_id: String,
    name: String,
    file_type: String,
    google_doc_url: Option<String>,
    entity_id: String,
    entity_type: String,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let watch_id = state
        .db_write(move |db| {
            crate::google_drive::sync::upsert_watched_source(
                db,
                &google_id,
                &name,
                &file_type,
                google_doc_url.as_deref(),
                &entity_id,
                &entity_type,
            )
        })
        .await?;

    // Wake the poller so it does an initial sync
    state.integrations.drive_poller_wake.notify_one();

    Ok(watch_id)
}

/// Remove a watched Drive source.
#[tauri::command]
pub async fn remove_google_drive_watch(
    watch_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    state
        .db_write(move |db| crate::google_drive::sync::remove_watched_source(db, &watch_id))
        .await
}

/// Get all watched Drive sources.
#[tauri::command]
pub async fn get_google_drive_watches(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<DriveWatchData>, String> {
    let sources = state
        .db_read(crate::google_drive::sync::get_all_watched_sources)
        .await?;
    Ok(sources
        .into_iter()
        .map(|s| DriveWatchData {
            id: s.id,
            google_id: s.google_id,
            name: s.name,
            file_type: s.file_type,
            google_doc_url: s.google_doc_url,
            entity_id: s.entity_id,
            entity_type: s.entity_type,
            last_synced_at: s.last_synced_at,
        })
        .collect())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveWatchData {
    pub id: String,
    pub google_id: String,
    pub name: String,
    pub file_type: String,
    pub google_doc_url: Option<String>,
    pub entity_id: String,
    pub entity_type: String,
    pub last_synced_at: Option<String>,
}

// =============================================================================
// I471: Audit Log Commands
// =============================================================================

/// Get recent audit log records, optionally filtered by category.
#[tauri::command]
pub fn get_audit_log_records(
    limit: Option<usize>,
    category_filter: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Vec<crate::audit_log::AuditRecord> {
    let path = if let Ok(guard) = state.audit_log.lock() {
        guard.path().to_path_buf()
    } else {
        return Vec::new();
    };

    crate::audit_log::read_records(&path, limit.unwrap_or(100), category_filter.as_deref())
}

/// Export the audit log to a user-selected path.
#[tauri::command]
pub fn export_audit_log(dest_path: String, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let src = if let Ok(guard) = state.audit_log.lock() {
        guard.path().to_path_buf()
    } else {
        return Err("Audit log unavailable".to_string());
    };

    if !src.exists() {
        return Err("No audit log file exists yet".to_string());
    }

    std::fs::copy(&src, &dest_path).map_err(|e| format!("Failed to export audit log: {e}"))?;
    Ok(())
}

/// Verify the audit log hash chain integrity.
#[tauri::command]
pub fn verify_audit_log_integrity(state: State<'_, Arc<AppState>>) -> Result<String, String> {
    let path = if let Ok(guard) = state.audit_log.lock() {
        guard.path().to_path_buf()
    } else {
        return Err("Audit log unavailable".to_string());
    };

    if !path.exists() {
        return Ok("No audit log file exists yet.".to_string());
    }

    match crate::audit_log::verify_audit_log(&path) {
        Ok(count) => Ok(format!(
            "Integrity verified: {} records, hash chain intact.",
            count
        )),
        Err((line, msg)) => Err(format!(
            "Integrity check failed at record {}: {}",
            line, msg
        )),
    }
}

// ---------------------------------------------------------------------------
// Context Mode (ADR-0095)
// ---------------------------------------------------------------------------

/// Get the current context mode (Local or Glean).
#[tauri::command]
pub fn get_context_mode(state: State<'_, Arc<AppState>>) -> Result<serde_json::Value, String> {
    let mode = state.with_db_read(|db| Ok(crate::context_provider::read_context_mode(db)))?;

    serde_json::to_value(&mode).map_err(|e| format!("Serialization error: {}", e))
}

/// Set the context mode. Requires app restart to take effect.
/// In Glean mode, Clay and Gravatar enrichment are automatically disabled.
#[tauri::command]
pub fn set_context_mode(
    mode: serde_json::Value,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let parsed: crate::context_provider::ContextMode =
        serde_json::from_value(mode).map_err(|e| format!("Invalid context mode: {}", e))?;

    // Read current mode before writing so we can log from/to
    let previous_mode = state
        .with_db_read(|db| Ok(crate::context_provider::read_context_mode(db)))
        .unwrap_or_default();

    state.with_db_write(|db| crate::context_provider::save_context_mode(db, &parsed))?;

    // Log the mode change with from/to
    let mode_name = |m: &crate::context_provider::ContextMode| -> &str {
        match m {
            crate::context_provider::ContextMode::Local => "local",
            crate::context_provider::ContextMode::Glean { .. } => "glean",
        }
    };
    if let Ok(mut audit) = state.audit_log.lock() {
        let _ = audit.append(
            "config",
            "context_mode_changed",
            serde_json::json!({
                "from": mode_name(&previous_mode),
                "to": mode_name(&parsed),
            }),
        );
    }

    // Enqueue all entities for re-enrichment at ProactiveHygiene priority.
    // A mode switch means context sources changed — existing intelligence
    // should be refreshed with the new provider on next app start.
    if let Ok(db_guard) = state.db.lock() {
        if let Some(db) = db_guard.as_ref() {
            use crate::intel_queue::{IntelPriority, IntelRequest};
            let mut count = 0u32;

            // Re-enqueue all entities that have intelligence (stale threshold = 0)
            if let Ok(entities) = db.get_stale_entity_intelligence(0) {
                for (entity_id, entity_type, _) in entities {
                    state.intel_queue.enqueue(IntelRequest {
                        entity_id,
                        entity_type,
                        priority: IntelPriority::ProactiveHygiene,
                        requested_at: std::time::Instant::now(),
                        retry_count: 0,
                    });
                    count += 1;
                }
            }
            // Also enqueue entities with no intelligence yet
            if let Ok(missing) = db.get_entities_without_intelligence() {
                for (entity_id, entity_type) in missing {
                    state.intel_queue.enqueue(IntelRequest {
                        entity_id,
                        entity_type,
                        priority: IntelPriority::ProactiveHygiene,
                        requested_at: std::time::Instant::now(),
                        retry_count: 0,
                    });
                    count += 1;
                }
            }

            if count > 0 {
                log::info!(
                    "Context mode switch: enqueued {} entities for re-enrichment",
                    count
                );
            }
        }
    }

    Ok(())
}

/// Start Glean OAuth consent flow — opens browser for SSO authentication.
///
/// Uses MCP OAuth discovery + DCR from the Glean MCP endpoint URL.
/// Returns `GleanAuthStatus::Authenticated` on success.
#[tauri::command]
pub async fn start_glean_auth(
    endpoint: String,
    state: State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<crate::glean::GleanAuthStatus, String> {
    use crate::glean;

    match glean::oauth::run_glean_consent_flow(&endpoint).await {
        Ok(result) => {
            let status = glean::GleanAuthStatus::Authenticated {
                email: result.email.unwrap_or_else(|| "connected".to_string()),
                name: result.name,
            };

            // Audit: oauth_connected
            if let Ok(mut audit) = state.audit_log.lock() {
                let _ = audit.append(
                    "security",
                    "oauth_connected",
                    serde_json::json!({"provider": "glean"}),
                );
            }

            let _ = app_handle.emit("glean-auth-changed", &status);
            Ok(status)
        }
        Err(glean::GleanAuthError::FlowCancelled) => {
            Err("Glean authorization was cancelled".to_string())
        }
        Err(e) => {
            let message = format!("{}", e);
            let _ = app_handle.emit(
                "glean-auth-failed",
                serde_json::json!({ "message": message }),
            );
            Err(message)
        }
    }
}

/// Get current Glean authentication status from Keychain.
#[tauri::command]
pub fn get_glean_auth_status() -> crate::glean::GleanAuthStatus {
    crate::glean::detect_glean_auth()
}

/// Disconnect Glean — delete OAuth token from Keychain.
#[tauri::command]
pub fn disconnect_glean(
    state: State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    crate::glean::token_store::delete_token().map_err(|e| format!("{}", e))?;

    let purge_report = state.with_db_write(|db| {
        crate::db::data_lifecycle::purge_source(db, crate::db::data_lifecycle::DataSource::Glean)
            .map_err(|e| e.to_string())
    })?;

    // Audit: oauth_revoked
    if let Ok(mut audit) = state.audit_log.lock() {
        let _ = audit.append(
            "security",
            "oauth_revoked",
            serde_json::json!({"provider": "glean", "purge": purge_report}),
        );
    }

    let status = crate::glean::GleanAuthStatus::NotConfigured;
    let _ = app_handle.emit("glean-auth-changed", &status);

    log::info!("Glean disconnected");
    Ok(())
}
