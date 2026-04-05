use super::*;

#[tauri::command]
pub fn get_config(state: State<'_, Arc<AppState>>) -> Result<Config, String> {
    state
        .config
        .read()
        .clone()
        .ok_or_else(|| "No configuration loaded. Create ~/.dailyos/config.json".to_string())
}

/// Reload configuration from disk
#[tauri::command]
pub fn reload_configuration(state: State<'_, Arc<AppState>>) -> Result<Config, String> {
    reload_config(&state)
}

/// Get dashboard data (DB-first, workspace JSON for today-specific data)
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

pub(crate) fn parse_meeting_datetime(value: &str) -> Option<chrono::DateTime<chrono::Utc>> {
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

pub(crate) fn parse_user_agenda_json(value: Option<&str>) -> Option<Vec<String>> {
    let layer = parse_user_agenda_layer(value);
    if layer.items.is_empty() {
        None
    } else {
        Some(layer.items)
    }
}

pub(crate) fn load_meeting_prep_from_sources(
    today_dir: &Path,
    meeting: &crate::db::DbMeeting,
) -> Option<FullMeetingPrep> {
    crate::services::meetings::load_meeting_prep_from_sources(today_dir, meeting)
}

pub(crate) fn collect_meeting_outcomes_from_db(
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

pub(crate) fn backfill_prep_semantics_value(prep: &mut serde_json::Value) -> bool {
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
pub(crate) struct BackfillCounts {
    pub(crate) candidate: usize,
    pub(crate) transformed: usize,
    pub(crate) skipped: usize,
    pub(crate) parse_errors: usize,
}

pub(crate) fn backfill_prep_files_in_dir(
    preps_dir: &Path,
    dry_run: bool,
) -> Result<BackfillCounts, String> {
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

pub(crate) fn backfill_db_prep_contexts(
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
        .clone()
        .ok_or("No configuration loaded")?;

    // Use a dedicated DB connection so this async command never holds AppState DB lock
    // across Google API awaits.
    let db = crate::db::ActionDb::open().map_err(|e| e.to_string())?;
    let (entity_hints, actions) = crate::queries::proactive::load_live_suggestion_inputs(&db)?;

    // Check cache unless force refresh requested
    if !force_refresh.unwrap_or(false) {
        {
            let guard = state.calendar.week_cache.read();
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

    *state.calendar.week_cache.write() = Some((events.clone(), std::time::Instant::now()));

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
