use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::OnceLock;

use chrono::TimeZone;
use regex::Regex;
use tauri::{Emitter, State};

use crate::executor::request_workflow_execution;
use crate::hygiene::{build_intelligence_hygiene_status, HygieneStatusView};
use crate::json_loader::{
    check_data_freshness, load_actions_json, load_emails_json, load_emails_json_with_sync,
    load_prep_json, load_schedule_json, DataFreshness,
};
use crate::parser::{count_inbox, list_inbox_files};
use crate::scheduler::get_next_run_time as scheduler_get_next_run_time;
use crate::state::{reload_config, AppState, DbTryRead};
use crate::types::{
    Action, CalendarEvent, CapturedOutcome, Config, DailyFocus, DashboardData, DayStats,
    EmailBriefingData, EmailBriefingStats, EmailSignal, EmailSyncStatus, EnrichedEmail,
    EntityEmailThread, ExecutionRecord, FullMeetingPrep, GoogleAuthStatus, InboxFile,
    LiveProactiveSuggestion, MeetingIntelligence, MeetingType, OverlayStatus,
    PostMeetingCaptureConfig, Priority, SourceReference, WeekOverview, WorkflowId, WorkflowStatus,
};
use crate::SchedulerSender;

/// Result type for dashboard data loading
#[derive(Debug, serde::Serialize)]
#[allow(clippy::large_enum_variant)]
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
    Error {
        message: String,
    },
}

/// p95 latency budgets for hot read commands.
const READ_CMD_LATENCY_BUDGET_MS: u128 = 100;
const DASHBOARD_LATENCY_BUDGET_MS: u128 = 300;
const CLAUDE_STATUS_CACHE_TTL_SECS: u64 = 300;
// TODO(I197 follow-up): migrate remaining command DB call sites to AppState DB
// helpers in passes, prioritizing frequent reads before one-off write paths.

fn normalize_match_key(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect()
}

fn attendee_domains(attendees: &[String]) -> HashSet<String> {
    attendees
        .iter()
        .filter_map(|a| a.split('@').nth(1))
        .map(|d| d.to_lowercase())
        .collect()
}

fn build_live_event_domain_map(events: &[CalendarEvent]) -> HashMap<String, HashSet<String>> {
    let mut map = HashMap::new();
    for event in events {
        map.insert(event.id.clone(), attendee_domains(&event.attendees));
    }
    map
}

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
pub fn get_config(state: State<Arc<AppState>>) -> Result<Config, String> {
    let guard = state.config.read().map_err(|_| "Lock poisoned")?;
    guard
        .clone()
        .ok_or_else(|| "No configuration loaded. Create ~/.dailyos/config.json".to_string())
}

/// Reload configuration from disk
#[tauri::command]
pub fn reload_configuration(state: State<Arc<AppState>>) -> Result<Config, String> {
    reload_config(&state)
}

/// Get dashboard data from workspace _today/data/ JSON files
#[tauri::command]
pub fn get_dashboard_data(state: State<Arc<AppState>>) -> DashboardResult {
    let started = std::time::Instant::now();
    let mut db_busy = false;

    let result = (|| {
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
        let mut meetings =
            crate::calendar_merge::merge_meetings(briefing_meetings, &live_events, &tz);

        // Consolidate all dashboard DB reads into a single lock acquisition (I235).
        // This reduces lock contention and improves dashboard load latency.
        let meeting_ids: Vec<String> = meetings.iter().map(|m| m.id.clone()).collect();
        struct DashboardDbSnapshot {
            reviewed: Option<HashMap<String, String>>,
            entity_map: Option<HashMap<String, Vec<crate::types::LinkedEntity>>>,
            accounts_with_domains: Option<Vec<(crate::db::DbAccount, Vec<String>)>>,
            non_briefing_actions: Option<Vec<crate::db::DbAction>>,
            focus_candidates: Option<Vec<crate::db::DbAction>>,
        }

        let db_snapshot = match state.with_db_try_read(|db| {
            DashboardDbSnapshot {
                reviewed: db.get_reviewed_preps().ok(),
                entity_map: db.get_meeting_entity_map(&meeting_ids).ok(),
                accounts_with_domains: db.get_all_accounts_with_domains(true).ok(),
                non_briefing_actions: db.get_non_briefing_pending_actions().ok(),
                focus_candidates: db.get_focus_candidate_actions(7).ok(),
            }
        }) {
            DbTryRead::Ok(snap) => Some(snap),
            DbTryRead::Busy => {
                db_busy = true;
                None
            }
            DbTryRead::Unavailable | DbTryRead::Poisoned => None,
        };

        // Apply DB data to meetings (outside the lock)
        if let Some(ref snap) = db_snapshot {
            // Annotate meetings with prep-reviewed state (ADR-0033)
            if let Some(ref reviewed) = snap.reviewed {
                for m in &mut meetings {
                    if reviewed.contains_key(&m.id) {
                        m.prep_reviewed = Some(true);
                    }
                }
            }

            // Annotate meetings with linked entities (I52)
            if let Some(ref entity_map) = snap.entity_map {
                for m in &mut meetings {
                    if let Some(entities) = entity_map.get(&m.id) {
                        let entities_vec: Vec<crate::types::LinkedEntity> = entities.clone();
                        m.linked_entities = Some(entities_vec);
                        if let Some(acct) = entities.iter().find(|e| e.entity_type == "account") {
                            m.account_id = Some(acct.id.clone());
                            m.account = Some(acct.name.clone());
                        }
                    }
                }
            }

            // Flag meetings matching archived accounts for unarchive suggestion (I161)
            if let Some(ref accounts_with_domains) = snap.accounts_with_domains {
                let mut archived = Vec::new();
                let mut domains_by_account: HashMap<String, HashSet<String>> = HashMap::new();
                for (account, domains) in accounts_with_domains {
                    if account.archived {
                        let domain_set: HashSet<String> =
                            domains.iter().map(|d| d.to_lowercase()).collect();
                        domains_by_account.insert(account.id.clone(), domain_set);
                        archived.push(account);
                    }
                }

                let archived_ids: HashSet<String> =
                    archived.iter().map(|a| a.id.to_lowercase()).collect();
                let live_domains = build_live_event_domain_map(&live_events);
                for m in &mut meetings {
                    if let Some(ref account_id) = m.account_id {
                        if !archived_ids.contains(&account_id.to_lowercase()) {
                            continue;
                        }
                    }

                    let account_hint_key = m
                        .account
                        .as_deref()
                        .map(normalize_match_key)
                        .unwrap_or_default();
                    let account_id_key = m
                        .account_id
                        .as_deref()
                        .map(normalize_match_key)
                        .unwrap_or_default();
                    let meeting_domains = m
                        .calendar_event_id
                        .as_ref()
                        .and_then(|id| live_domains.get(id))
                        .or_else(|| live_domains.get(&m.id));

                    let mut best: Option<(i32, String, String)> = None;
                    for archived_account in &archived {
                        let mut score = 0i32;
                        let archived_id_key = normalize_match_key(&archived_account.id);
                        let archived_name_key = normalize_match_key(&archived_account.name);

                        if !account_id_key.is_empty() && account_id_key == archived_id_key {
                            score = score.max(100);
                        }
                        if !account_hint_key.is_empty()
                            && (account_hint_key == archived_id_key
                                || account_hint_key == archived_name_key)
                        {
                            score = score.max(90);
                        }
                        if let Some(domains) = meeting_domains {
                            if let Some(account_domains) =
                                domains_by_account.get(&archived_account.id)
                            {
                                if !account_domains.is_empty()
                                    && domains.iter().any(|d| account_domains.contains(d))
                                {
                                    score = score.max(80);
                                }
                            }
                        }

                        if score == 0 {
                            continue;
                        }
                        let tie = archived_account.id.to_lowercase();
                        let candidate = (score, tie.clone(), archived_account.id.clone());
                        if best
                            .as_ref()
                            .map(|(s, t, _)| score > *s || (score == *s && tie < *t))
                            .unwrap_or(true)
                        {
                            best = Some(candidate);
                        }
                    }

                    if let Some((score, _, account_id)) = best {
                        if score >= 80 {
                            m.suggested_unarchive_account_id = Some(account_id);
                        }
                    }
                }
            }
        }

        let mut actions = load_actions_json(&today_dir).unwrap_or_default();

        // Merge non-briefing actions from SQLite (post-meeting capture, inbox) — I17
        if let Some(ref snap) = db_snapshot {
            if let Some(ref db_actions) = snap.non_briefing_actions {
                let json_titles: HashSet<String> = actions
                    .iter()
                    .map(|a| a.title.to_lowercase().trim().to_string())
                    .collect();
                for dba in db_actions {
                    if !json_titles.contains(dba.title.to_lowercase().trim()) {
                        let priority = match dba.priority.as_str() {
                            "P1" => Priority::P1,
                            "P3" => Priority::P3,
                            _ => Priority::P2,
                        };
                        actions.push(Action {
                            id: dba.id.clone(),
                            title: dba.title.clone(),
                            account: dba.account_id.clone(),
                            due_date: dba.due_date.clone(),
                            priority,
                            status: crate::types::ActionStatus::Pending,
                            is_overdue: None,
                            context: dba.context.clone(),
                            source: dba.source_label.clone(),
                            days_overdue: None,
                        });
                    }
                }
            }
        }

        let (emails, email_sync): (Option<Vec<crate::types::Email>>, Option<EmailSyncStatus>) =
            match load_emails_json_with_sync(&today_dir) {
                Ok(payload) => {
                    let emails = if payload.emails.is_empty() {
                        None
                    } else {
                        Some(payload.emails)
                    };
                    (emails, payload.sync)
                }
                Err(_) => (
                    load_emails_json(&today_dir).ok().filter(|e| !e.is_empty()),
                    None,
                ),
            };

        // Compute capacity-aware focus priorities (live, not a briefing artifact)
        let focus: Option<DailyFocus> = (|| {
            let today_date = chrono::Local::now().date_naive();
            let capacity = crate::focus_capacity::compute_focus_capacity(
                crate::focus_capacity::FocusCapacityInput {
                    meetings: meetings.clone(),
                    source: if live_events.is_empty() {
                        crate::focus_capacity::FocusCapacitySource::BriefingFallback
                    } else {
                        crate::focus_capacity::FocusCapacitySource::Live
                    },
                    timezone: tz,
                    work_hours_start: config.google.work_hours_start,
                    work_hours_end: config.google.work_hours_end,
                    day_date: today_date,
                },
            );
            let candidates = match db_snapshot.as_ref().and_then(|s| s.focus_candidates.clone()) {
                Some(c) => c,
                None => return None,
            };
            let (prioritized, top_three, implications) =
                crate::focus_prioritization::prioritize_actions(
                    candidates,
                    capacity.available_minutes,
                );
            Some(DailyFocus {
                available_minutes: capacity.available_minutes,
                deep_work_minutes: capacity.deep_work_minutes,
                meeting_minutes: capacity.meeting_minutes,
                meeting_count: capacity.meeting_count,
                prioritized_actions: prioritized,
                top_three,
                implications,
                available_blocks: capacity.available_blocks,
            })
        })();

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
                email_sync,
                focus,
            },
            freshness,
            google_auth,
        }
    })();

    let elapsed_ms = started.elapsed().as_millis();
    crate::latency::record_latency(
        "get_dashboard_data",
        elapsed_ms,
        DASHBOARD_LATENCY_BUDGET_MS,
    );
    if db_busy {
        crate::latency::increment_degraded("get_dashboard_data");
    }
    if elapsed_ms > DASHBOARD_LATENCY_BUDGET_MS {
        log::warn!(
            "get_dashboard_data exceeded latency budget: {}ms > {}ms (db_busy={})",
            elapsed_ms,
            DASHBOARD_LATENCY_BUDGET_MS,
            db_busy
        );
    } else {
        log::debug!(
            "get_dashboard_data completed in {}ms (db_busy={})",
            elapsed_ms,
            db_busy
        );
    }

    result
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
    state: State<Arc<AppState>>,
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
    state: State<Arc<AppState>>,
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
    state: State<Arc<AppState>>,
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
    let Some(json) = value else { return UserAgendaLayer::default() };
    // Try rich format first
    if let Ok(layer) = serde_json::from_str::<UserAgendaLayer>(json) {
        return layer;
    }
    // Fall back to legacy Vec<String>
    if let Ok(items) = serde_json::from_str::<Vec<String>>(json) {
        return UserAgendaLayer { items, ..Default::default() };
    }
    UserAgendaLayer::default()
}

fn parse_user_agenda_json(value: Option<&str>) -> Option<Vec<String>> {
    let layer = parse_user_agenda_layer(value);
    if layer.items.is_empty() { None } else { Some(layer.items) }
}

fn load_meeting_prep_from_sources(
    today_dir: &Path,
    meeting: &crate::db::DbMeeting,
) -> Option<FullMeetingPrep> {
    if let Ok(prep) = load_prep_json(today_dir, &meeting.id) {
        return Some(prep);
    }
    if let Some(ref frozen) = meeting.prep_frozen_json {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(frozen) {
            if let Some(prep_val) = value.get("prep") {
                if let Ok(prep) = serde_json::from_value::<FullMeetingPrep>(prep_val.clone()) {
                    return Some(prep);
                }
            }
            if let Ok(prep) = serde_json::from_value::<FullMeetingPrep>(value) {
                return Some(prep);
            }
        }
    }
    if let Some(ref prep_json) = meeting.prep_context_json {
        if let Ok(prep) = serde_json::from_str::<FullMeetingPrep>(prep_json) {
            return Some(prep);
        }
    }
    None
}

fn collect_meeting_outcomes_from_db(
    db: &crate::db::ActionDb,
    meeting: &crate::db::DbMeeting,
) -> Option<crate::types::MeetingOutcomeData> {
    let captures = db.get_captures_for_meeting(&meeting.id).ok()?;
    let actions = db.get_actions_for_meeting(&meeting.id).ok()?;

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

    if meeting.summary.is_none()
        && meeting.transcript_path.is_none()
        && meeting.transcript_processed_at.is_none()
        && wins.is_empty()
        && risks.is_empty()
        && decisions.is_empty()
        && actions.is_empty()
    {
        return None;
    }

    Some(crate::types::MeetingOutcomeData {
        meeting_id: meeting.id.clone(),
        summary: meeting.summary.clone(),
        wins,
        risks,
        decisions,
        actions,
        transcript_path: meeting.transcript_path.clone(),
        processed_at: meeting.transcript_processed_at.clone(),
    })
}

/// Unified meeting detail payload for current + historical meetings.
#[tauri::command]
pub fn get_meeting_intelligence(
    meeting_id: String,
    state: State<Arc<AppState>>,
) -> Result<MeetingIntelligence, String> {
    let config = state
        .config
        .read()
        .map_err(|_| "Lock poisoned")?
        .clone()
        .ok_or("No configuration loaded")?;
    let workspace = Path::new(&config.workspace_path);
    let today_dir = workspace.join("_today");

    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let meeting = if let Some(row) = db
        .get_meeting_intelligence_row(&meeting_id)
        .map_err(|e| e.to_string())?
    {
        row
    } else {
        let raw_calendar_id = meeting_id.replace("_at_", "@");
        db.get_meeting_by_calendar_event_id(&raw_calendar_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Meeting not found: {}", meeting_id))?
    };

    let agenda_layer = parse_user_agenda_layer(meeting.user_agenda_json.as_deref());
    let user_agenda = if agenda_layer.items.is_empty() { None } else { Some(agenda_layer.items.clone()) };
    let dismissed_topics = agenda_layer.dismissed_topics.clone();
    let hidden_attendees = agenda_layer.hidden_attendees.clone();
    let user_notes = meeting.user_notes.clone();
    let mut prep = load_meeting_prep_from_sources(&today_dir, &meeting);

    if let Some(ref mut prep_data) = prep {
        prep_data.user_agenda = user_agenda.clone();
        prep_data.user_notes = user_notes.clone();
        let _ = db.mark_prep_reviewed(
            &meeting.id,
            prep_data.calendar_event_id.as_deref(),
            &prep_data.title,
        );

        // Hydrate attendee_context from people DB (I51)
        if prep_data.attendee_context.is_none() {
            let attendee_context = hydrate_attendee_context(db, &meeting);
            if !attendee_context.is_empty() {
                prep_data.attendee_context = Some(attendee_context);
            }
        }
    }

    let now = chrono::Utc::now();
    let start_dt = parse_meeting_datetime(&meeting.start_time);
    let end_dt = meeting
        .end_time
        .as_deref()
        .and_then(parse_meeting_datetime)
        .or(start_dt.map(|s| s + chrono::Duration::hours(1)));
    let is_current = start_dt
        .zip(end_dt)
        .is_some_and(|(s, e)| s <= now && now <= e);
    let is_past = end_dt.is_some_and(|e| e < now);
    let is_frozen = meeting.prep_frozen_at.is_some();
    let can_edit_user_layer = !(is_past || is_frozen);

    let captures = db
        .get_captures_for_meeting(&meeting.id)
        .map_err(|e| e.to_string())?;
    let actions = db
        .get_actions_for_meeting(&meeting.id)
        .map_err(|e| e.to_string())?;
    let linked_entities = db
        .get_meeting_entities(&meeting.id)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|e| crate::types::LinkedEntity {
            id: e.id,
            name: e.name,
            entity_type: e.entity_type.as_str().to_string(),
        })
        .collect::<Vec<_>>();

    let outcomes = collect_meeting_outcomes_from_db(db, &meeting);
    let prep_snapshot_path = meeting.prep_snapshot_path.clone();
    let prep_frozen_at = meeting.prep_frozen_at.clone();
    let transcript_path = meeting.transcript_path.clone();
    let transcript_processed_at = meeting.transcript_processed_at.clone();

    Ok(MeetingIntelligence {
        meeting,
        prep,
        is_past,
        is_current,
        is_frozen,
        can_edit_user_layer,
        user_agenda,
        user_notes,
        dismissed_topics,
        hidden_attendees,
        outcomes,
        captures,
        actions,
        linked_entities,
        prep_snapshot_path,
        prep_frozen_at,
        transcript_path,
        transcript_processed_at,
    })
}

/// Build AttendeeContext by matching calendar attendee emails to person entities.
/// Scoped to external (non-internal) attendees who are in the people database.
fn hydrate_attendee_context(
    db: &crate::db::ActionDb,
    meeting: &crate::db::DbMeeting,
) -> Vec<crate::types::AttendeeContext> {
    use std::collections::HashSet;

    let mut seen_emails = HashSet::new();
    let mut contexts = Vec::new();

    // Strategy 1: Get people already linked via meeting_attendees junction table
    if let Ok(linked_people) = db.get_meeting_attendees(&meeting.id) {
        for person in &linked_people {
            let email_lower = person.email.to_lowercase();
            if seen_emails.contains(&email_lower) {
                continue;
            }
            seen_emails.insert(email_lower);
            contexts.push(person_to_attendee_context(person));
        }
    }

    // Strategy 2: Parse emails from meeting.attendees field and look up each
    if let Some(ref attendees_str) = meeting.attendees {
        let emails: Vec<String> = attendees_str
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| s.contains('@'))
            .collect();

        for email in &emails {
            if seen_emails.contains(email) {
                continue;
            }
            if let Ok(Some(person)) = db.get_person_by_email(email) {
                seen_emails.insert(email.clone());
                contexts.push(person_to_attendee_context(&person));
            }
        }
    }

    // Filter to non-internal, non-archived people
    contexts
        .into_iter()
        .filter(|ctx| {
            // Keep external and unknown relationships; exclude internal
            ctx.relationship.as_deref() != Some("internal")
        })
        .collect()
}

/// Convert a DbPerson into an AttendeeContext with computed temperature.
fn person_to_attendee_context(person: &crate::db::DbPerson) -> crate::types::AttendeeContext {
    let temperature = person
        .last_seen
        .as_deref()
        .map(|ls| {
            let days = crate::db::days_since_iso(ls);
            match days {
                Some(d) if d < 7 => "hot".to_string(),
                Some(d) if d < 30 => "warm".to_string(),
                Some(d) if d < 60 => "cool".to_string(),
                _ => "cold".to_string(),
            }
        });

    crate::types::AttendeeContext {
        name: person.name.clone(),
        email: Some(person.email.clone()),
        role: person.role.clone(),
        organization: person.organization.clone(),
        relationship: Some(person.relationship.clone()),
        meeting_count: Some(person.meeting_count),
        last_seen: person.last_seen.clone(),
        temperature,
        notes: person.notes.clone(),
        person_id: Some(person.id.clone()),
    }
}

/// Compatibility wrapper while frontend migrates to get_meeting_intelligence.
#[tauri::command]
pub fn get_meeting_prep(meeting_id: String, state: State<Arc<AppState>>) -> MeetingPrepResult {
    match get_meeting_intelligence(meeting_id, state) {
        Ok(intel) => match intel.prep {
            Some(data) => MeetingPrepResult::Success { data },
            None => MeetingPrepResult::NotFound {
                message: "Meeting found but has no prep data".to_string(),
            },
        },
        Err(message) => MeetingPrepResult::NotFound { message },
    }
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
                db.update_meeting_prep_context(&meeting_id, &updated_json)
                    .map_err(|e| {
                        format!("Failed to update prep context for {}: {}", meeting_id, e)
                    })?;
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
/// - `meetings_history.prep_context_json`
#[tauri::command]
pub fn backfill_prep_semantics(
    dry_run: bool,
    state: State<Arc<AppState>>,
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

    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let db_counts = backfill_db_prep_contexts(db, dry_run)?;
    report.candidate_db_row_count = db_counts.candidate;
    report.transformed_db_row_count = db_counts.transformed;
    report.skipped_db_row_count = db_counts.skipped;
    report.parse_error_db_row_count = db_counts.parse_errors;

    Ok(report)
}

// =============================================================================
// Week Overview Command
// =============================================================================

/// Result type for week data
#[derive(Debug, serde::Serialize)]
#[allow(clippy::large_enum_variant)]
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

    let started = std::time::Instant::now();

    let mut week = match crate::json_loader::load_week_json(&today_dir) {
        Ok(w) => w,
        Err(e) => {
            return WeekResult::NotFound {
                message: format!("No week data: {}", e),
            }
        }
    };

    // Enrich dayShapes with live per-day action priorities (I279)
    if let Some(ref mut shapes) = week.day_shapes {
        if let Ok(db) = crate::db::ActionDb::open() {
            if let Ok(candidates) = db.get_focus_candidate_actions(7) {
                for shape in shapes.iter_mut() {
                    let available_minutes: u32 =
                        shape.available_blocks.iter().map(|b| b.duration_minutes).sum();

                    let (prioritized, _top_three, implications) =
                        crate::focus_prioritization::prioritize_actions(
                            candidates.clone(),
                            available_minutes,
                        );

                    shape.prioritized_actions = Some(prioritized);
                    shape.focus_implications = Some(implications);
                }
            }
        }
    }

    log_command_latency("get_week_data", started, READ_CMD_LATENCY_BUDGET_MS);
    WeekResult::Success { data: week }
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
    let (account_hints, actions) = crate::queries::proactive::load_live_suggestion_inputs(&db)?;

    // Check cache unless force refresh requested
    if !force_refresh.unwrap_or(false) {
        if let Ok(guard) = state.week_calendar_cache.read() {
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
                    let bg_hints = account_hints.clone();
                    tokio::spawn(async move {
                        let _ = refresh_week_calendar_cache(&bg_state, &bg_config, bg_hints).await;
                    });
                    return result;
                }
            }
        }
    }

    // Cache miss or expired: fetch, cache, and compute
    let events = refresh_week_calendar_cache(&state, &config, account_hints).await?;
    crate::queries::proactive::compute_suggestions_from_events(&config, &events, &actions)
}

/// Fetch week calendar events from Google API and update the AppState cache.
async fn refresh_week_calendar_cache(
    state: &AppState,
    config: &crate::types::Config,
    account_hints: std::collections::HashSet<String>,
) -> Result<Vec<CalendarEvent>, String> {
    let events = crate::queries::proactive::fetch_week_events(config, &account_hints).await?;

    if let Ok(mut guard) = state.week_calendar_cache.write() {
        *guard = Some((events.clone(), std::time::Instant::now()));
    }

    Ok(events)
}

/// Retry only week AI enrichment without rerunning full week prepare/deliver.
#[tauri::command]
pub async fn retry_week_enrichment(state: State<'_, Arc<AppState>>) -> Result<String, String> {
    let config = state
        .config
        .read()
        .map_err(|_| "Lock poisoned")?
        .clone()
        .ok_or("No configuration loaded")?;

    let workspace_path = config.workspace_path.clone();
    let user_ctx = crate::types::UserContext::from_config(&config);
    let ai_models = config.ai_models.clone();
    let state = state.inner().clone();

    let task = tauri::async_runtime::spawn_blocking(move || {
        let workspace = std::path::Path::new(&workspace_path);
        let data_dir = workspace.join("_today").join("data");
        let week_path = data_dir.join("week-overview.json");
        if !week_path.exists() {
            return Err("No weekly overview found. Run the weekly workflow first.".to_string());
        }

        let pty = crate::pty::PtyManager::for_tier(crate::pty::ModelTier::Synthesis, &ai_models);
        crate::workflow::deliver::enrich_week(&data_dir, &pty, workspace, &user_ctx, &state)
    });

    match task.await {
        Ok(result) => result?,
        Err(e) => return Err(format!("Week enrichment task panicked: {}", e)),
    }

    Ok("Week enrichment retried".to_string())
}

// =============================================================================
// Focus Data Command
// =============================================================================

// =============================================================================
// Actions Command
// =============================================================================

/// Result type for all actions
#[derive(Debug, serde::Serialize)]
#[allow(clippy::large_enum_variant)]
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
                    if !json_titles.contains(dba.title.to_lowercase().trim()) {
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
    let mut files = list_inbox_files(workspace);
    let count = files.len();

    // Enrich files with persistent processing status from DB
    if let Ok(db_guard) = state.db.lock() {
        if let Some(db) = db_guard.as_ref() {
            if let Ok(status_map) = db.get_latest_processing_status() {
                for file in &mut files {
                    if let Some((status, error)) = status_map.get(&file.filename) {
                        file.processing_status = Some(status.clone());
                        file.processing_error = error.clone();
                    }
                }
            }
        }
    }

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
    entity_id: Option<String>,
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
    let entity_id = entity_id.clone();

    // Validate filename before processing (I60: path traversal guard)
    let workspace = Path::new(&workspace_path);
    crate::util::validate_inbox_path(workspace, &filename)?;

    tauri::async_runtime::spawn_blocking(move || {
        let workspace = Path::new(&workspace_path);
        let db_guard = state.db.lock().ok();
        let db_ref = db_guard.as_ref().and_then(|g| g.as_ref());
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
pub fn copy_to_inbox(paths: Vec<String>, state: State<Arc<AppState>>) -> Result<usize, String> {
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

/// Get emails enriched with entity signals from SQLite.
///
/// Loads emails from emails.json, then batch-queries the email_signals table
/// for all email IDs to build enriched emails and entity thread summaries.
#[tauri::command]
pub fn get_emails_enriched(state: State<Arc<AppState>>) -> Result<EmailBriefingData, String> {
    let config = state
        .config
        .read()
        .map_err(|_| "Config lock poisoned".to_string())?
        .clone()
        .ok_or_else(|| "No configuration loaded".to_string())?;

    let workspace = Path::new(&config.workspace_path);
    let today_dir = workspace.join("_today");

    let emails = load_emails_json(&today_dir).unwrap_or_default();

    // Collect email IDs for batch signal lookup
    let email_ids: Vec<String> = emails.iter().map(|e| e.id.clone()).collect();

    // Batch-query signals from DB
    let db_signals = match state.with_db_try_read(|db| db.list_email_signals_by_email_ids(&email_ids)) {
        crate::state::DbTryRead::Ok(Ok(sigs)) => sigs,
        _ => Vec::new(),
    };

    let has_enrichment = !db_signals.is_empty()
        || emails.iter().any(|e| e.summary.is_some());

    // Index signals by email_id
    let mut signals_by_email: HashMap<String, Vec<EmailSignal>> = HashMap::new();
    for sig in &db_signals {
        signals_by_email
            .entry(sig.email_id.clone())
            .or_default()
            .push(EmailSignal {
                signal_type: sig.signal_type.clone(),
                signal_text: sig.signal_text.clone(),
                confidence: sig.confidence,
                sentiment: sig.sentiment.clone(),
                urgency: sig.urgency.clone(),
                detected_at: Some(sig.detected_at.clone()),
            });
    }

    // Build enriched emails by priority
    let mut high = Vec::new();
    let mut medium = Vec::new();
    let mut low = Vec::new();
    let mut needs_action = 0usize;

    for email in emails {
        let sigs = signals_by_email.remove(&email.id).unwrap_or_default();
        if email.recommended_action.is_some() {
            needs_action += 1;
        }
        let enriched = EnrichedEmail {
            email: email.clone(),
            signals: sigs,
        };
        match email.priority {
            crate::types::EmailPriority::High => high.push(enriched),
            crate::types::EmailPriority::Medium => medium.push(enriched),
            crate::types::EmailPriority::Low => low.push(enriched),
        }
    }

    // Build entity threads from signals
    let mut entity_map: HashMap<String, (String, Vec<EmailSignal>, HashSet<String>)> =
        HashMap::new();
    for sig in &db_signals {
        let entry = entity_map
            .entry(sig.entity_id.clone())
            .or_insert_with(|| (sig.entity_type.clone(), Vec::new(), HashSet::new()));
        entry.1.push(EmailSignal {
            signal_type: sig.signal_type.clone(),
            signal_text: sig.signal_text.clone(),
            confidence: sig.confidence,
            sentiment: sig.sentiment.clone(),
            urgency: sig.urgency.clone(),
            detected_at: Some(sig.detected_at.clone()),
        });
        entry.2.insert(sig.email_id.clone());
    }

    // Resolve entity names from DB
    let entity_threads: Vec<EntityEmailThread> = entity_map
        .into_iter()
        .map(|(entity_id, (entity_type, signals, email_set))| {
            let entity_name: String = {
                let eid = entity_id.clone();
                let etype = entity_type.clone();
                match state.with_db_try_read(|db| -> Result<String, crate::db::DbError> {
                    if &etype == "account" {
                        Ok(db.get_account(&eid)?.map(|a| a.name).unwrap_or_else(|| eid.clone()))
                    } else {
                        Ok(db.get_project(&eid)?.map(|p| p.name).unwrap_or_else(|| eid.clone()))
                    }
                }) {
                    crate::state::DbTryRead::Ok(Ok(name)) => name,
                    _ => entity_id.clone(),
                }
            };

            // Build signal summary like "2 risks, 1 expansion signal"
            let mut type_counts: HashMap<String, usize> = HashMap::new();
            for s in &signals {
                *type_counts.entry(s.signal_type.clone()).or_insert(0) += 1;
            }
            let summary_parts: Vec<String> = type_counts
                .iter()
                .map(|(t, c)| format!("{} {}", c, t))
                .collect();
            let signal_summary = summary_parts.join(", ");

            EntityEmailThread {
                entity_id,
                entity_name,
                entity_type,
                email_count: email_set.len(),
                signal_summary,
                signals,
            }
        })
        .collect();

    let total = high.len() + medium.len() + low.len();
    Ok(EmailBriefingData {
        stats: EmailBriefingStats {
            total,
            high_count: high.len(),
            medium_count: medium.len(),
            low_count: low.len(),
            needs_action,
        },
        high_priority: high,
        medium_priority: medium,
        low_priority: low,
        entity_threads,
        has_enrichment,
    })
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

/// Archive low-priority emails in Gmail and remove them from local data (I144).
///
/// Reads emails.json, collects IDs of low-priority emails, calls Gmail
/// batchModify to remove the INBOX label, then rewrites emails.json
/// without the archived entries.
#[tauri::command]
pub async fn archive_low_priority_emails(state: State<'_, Arc<AppState>>) -> Result<usize, String> {
    let config = state
        .config
        .read()
        .map_err(|_| "Lock poisoned")?
        .clone()
        .ok_or("No configuration loaded")?;

    let workspace = Path::new(&config.workspace_path);
    let emails_path = workspace.join("_today").join("data").join("emails.json");

    // Read current emails.json
    let content = std::fs::read_to_string(&emails_path)
        .map_err(|e| format!("Failed to read emails.json: {}", e))?;
    let mut data: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse emails.json: {}", e))?;

    // Collect low-priority email IDs
    let low_emails = data["lowPriority"].as_array().cloned().unwrap_or_default();

    let ids: Vec<String> = low_emails
        .iter()
        .filter_map(|e| e["id"].as_str().map(String::from))
        .collect();

    if ids.is_empty() {
        return Ok(0);
    }

    // Archive in Gmail
    let access_token = crate::google_api::get_valid_access_token()
        .await
        .map_err(|e| format!("Gmail auth failed: {}", e))?;

    let archived = crate::google_api::gmail::archive_emails(&access_token, &ids)
        .await
        .map_err(|e| format!("Gmail archive failed: {}", e))?;

    // Remove low-priority from local JSON and update stats
    data["lowPriority"] = serde_json::json!([]);
    if let Some(stats) = data.get_mut("stats") {
        let high = stats["highCount"].as_u64().unwrap_or(0);
        let medium = stats["mediumCount"].as_u64().unwrap_or(0);
        stats["lowCount"] = serde_json::json!(0);
        stats["total"] = serde_json::json!(high + medium);
    }

    crate::util::atomic_write_str(
        &emails_path,
        &serde_json::to_string_pretty(&data)
            .map_err(|e| format!("Failed to serialize emails: {}", e))?,
    )
    .map_err(|e| format!("Failed to write emails.json: {}", e))?;

    log::info!("Archived {} low-priority emails in Gmail", archived);
    Ok(archived)
}

/// Set user profile (customer-success or general)
#[tauri::command]
pub fn set_profile(profile: String, state: State<Arc<AppState>>) -> Result<Config, String> {
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
pub fn set_entity_mode(mode: String, state: State<Arc<AppState>>) -> Result<Config, String> {
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
pub fn set_workspace_path(path: String, state: State<Arc<AppState>>) -> Result<Config, String> {
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
                &config.resolved_user_domains(),
            );
            let _ = crate::accounts::sync_accounts_from_workspace(workspace, db);
            let _ = crate::projects::sync_projects_from_workspace(workspace, db);
        }
    }

    Ok(config)
}

/// Toggle developer mode (shows/hides devtools panel)
#[tauri::command]
pub fn set_developer_mode(enabled: bool, state: State<Arc<AppState>>) -> Result<Config, String> {
    crate::state::create_or_update_config(&state, |config| {
        config.developer_mode = enabled;
    })
}

/// Set UI personality tone (professional, friendly, playful)
#[tauri::command]
pub fn set_personality(personality: String, state: State<Arc<AppState>>) -> Result<Config, String> {
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
    state: State<Arc<AppState>>,
) -> Result<Config, String> {
    // Validate tier
    let valid_tiers = ["synthesis", "extraction", "mechanical"];
    if !valid_tiers.contains(&tier.as_str()) {
        return Err(format!(
            "Invalid tier '{}'. Must be one of: {}",
            tier,
            valid_tiers.join(", ")
        ));
    }

    // Validate model
    let valid_models = ["opus", "sonnet", "haiku"];
    if !valid_models.contains(&model.as_str()) {
        return Err(format!(
            "Invalid model '{}'. Must be one of: {}",
            model,
            valid_models.join(", ")
        ));
    }

    crate::state::create_or_update_config(&state, |config| {
        match tier.as_str() {
            "synthesis" => config.ai_models.synthesis = model.clone(),
            "extraction" => config.ai_models.extraction = model.clone(),
            "mechanical" => config.ai_models.mechanical = model.clone(),
            _ => {} // unreachable after validation
        }
    })
}

/// Set hygiene configuration (I271)
#[tauri::command]
pub fn set_hygiene_config(
    scan_interval_hours: Option<u32>,
    ai_budget: Option<u32>,
    pre_meeting_hours: Option<u32>,
    state: State<Arc<AppState>>,
) -> Result<Config, String> {
    // Validate values
    if let Some(v) = scan_interval_hours {
        if ![1, 2, 4, 8].contains(&v) {
            return Err(format!("Invalid scan interval: {}. Must be 1, 2, 4, or 8.", v));
        }
    }
    if let Some(v) = ai_budget {
        if ![5, 10, 20, 50].contains(&v) {
            return Err(format!("Invalid AI budget: {}. Must be 5, 10, 20, or 50.", v));
        }
    }
    if let Some(v) = pre_meeting_hours {
        if ![2, 4, 12, 24].contains(&v) {
            return Err(format!("Invalid pre-meeting window: {}. Must be 2, 4, 12, or 24.", v));
        }
    }

    crate::state::create_or_update_config(&state, |config| {
        if let Some(v) = scan_interval_hours {
            config.hygiene_scan_interval_hours = v;
        }
        if let Some(v) = ai_budget {
            config.hygiene_ai_budget = v;
        }
        if let Some(v) = pre_meeting_hours {
            config.hygiene_pre_meeting_hours = v;
        }
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

/// Save user profile fields (name, company, title, focus, domains)
#[tauri::command]
pub fn set_user_profile(
    name: Option<String>,
    company: Option<String>,
    title: Option<String>,
    focus: Option<String>,
    domain: Option<String>,
    domains: Option<Vec<String>>,
    state: State<Arc<AppState>>,
) -> Result<String, String> {
    crate::state::create_or_update_config(&state, |config| {
        // Helper: trim, convert empty to None
        fn clean(val: Option<String>) -> Option<String> {
            val.and_then(|s| {
                let trimmed = s.trim().to_string();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed)
                }
            })
        }

        config.user_name = clean(name);
        config.user_company = clean(company);
        config.user_title = clean(title);
        config.user_focus = clean(focus);

        // Prefer multi-domain list; fall back to legacy single domain
        if let Some(d) = domains {
            let cleaned: Vec<String> = d
                .into_iter()
                .map(|s| s.trim().to_lowercase())
                .filter(|s| !s.is_empty())
                .collect();
            if cleaned.is_empty() {
                config.user_domains = None;
                // Also clear legacy field
                config.user_domain = None;
            } else {
                // Set first as legacy field for backwards compat
                config.user_domain = Some(cleaned[0].clone());
                config.user_domains = Some(cleaned);
            }
        } else if let Some(d) = domain {
            let trimmed = d.trim().to_lowercase();
            config.user_domain = if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            };
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
    let completed = db.get_completed_actions(48).map_err(|e| e.to_string())?;
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
pub fn complete_action(id: String, state: State<Arc<AppState>>) -> Result<(), String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    db.complete_action(&id).map_err(|e| e.to_string())
}

/// Reopen a completed action, setting it back to pending.
#[tauri::command]
pub fn reopen_action(id: String, state: State<Arc<AppState>>) -> Result<(), String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    db.reopen_action(&id).map_err(|e| e.to_string())
}

/// Accept a proposed action, moving it to pending (I256).
#[tauri::command]
pub fn accept_proposed_action(id: String, state: State<Arc<AppState>>) -> Result<(), String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    db.accept_proposed_action(&id).map_err(|e| e.to_string())
}

/// Reject a proposed action by archiving it (I256).
#[tauri::command]
pub fn reject_proposed_action(id: String, state: State<Arc<AppState>>) -> Result<(), String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    db.reject_proposed_action(&id).map_err(|e| e.to_string())
}

/// Get all proposed (AI-suggested) actions (I256).
#[tauri::command]
pub fn get_proposed_actions(
    state: State<Arc<AppState>>,
) -> Result<Vec<crate::db::DbAction>, String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    db.get_proposed_actions().map_err(|e| e.to_string())
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
        db.get_account(aid).ok().flatten().map(|a| a.name)
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

    // Parse persisted prep context (I181)
    let prep_context = meeting
        .prep_context_json
        .as_ref()
        .and_then(|json_str| serde_json::from_str::<PrepContext>(json_str).ok());

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
        prep_context,
    })
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
pub fn search_meetings(
    query: String,
    state: State<Arc<AppState>>,
) -> Result<Vec<MeetingSearchResult>, String> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }

    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let pattern = format!("%{}%", query.trim());
    let mut stmt = db
        .conn_ref()
        .prepare(
            "SELECT id, title, meeting_type, start_time, account_id, summary, prep_context_json
             FROM meetings_history
             WHERE title LIKE ?1
                OR summary LIKE ?1
                OR prep_context_json LIKE ?1
             ORDER BY start_time DESC
             LIMIT 50",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(rusqlite::params![&pattern], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    for row in rows {
        let (id, title, meeting_type, start_time, account_id, summary, prep_json) =
            row.map_err(|e| e.to_string())?;

        // Extract snippet: prefer summary, fall back to intelligence summary from prep
        let match_snippet = summary.or_else(|| {
            prep_json.and_then(|json| {
                serde_json::from_str::<serde_json::Value>(&json)
                    .ok()
                    .and_then(|v| {
                        v.get("intelligenceSummary")
                            .and_then(|s| s.as_str().map(|s| s.to_string()))
                    })
            })
        });

        // Resolve account name
        let account_name = account_id
            .as_ref()
            .and_then(|aid| db.get_account(aid).ok().flatten())
            .map(|a| a.name);

        results.push(MeetingSearchResult {
            id,
            title,
            meeting_type,
            start_time,
            account_name,
            match_snippet,
        });
    }

    Ok(results)
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
pub fn get_google_auth_status(state: State<Arc<AppState>>) -> GoogleAuthStatus {
    let started = std::time::Instant::now();
    let cached = state
        .google_auth
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or(GoogleAuthStatus::NotConfigured);

    // If cached state says not configured, re-check storage — token may have
    // been written by a script or the browser auth flow completing late.
    if matches!(cached, GoogleAuthStatus::NotConfigured) {
        let fresh = crate::state::detect_google_auth();
        if matches!(fresh, GoogleAuthStatus::Authenticated { .. }) {
            if let Ok(mut guard) = state.google_auth.lock() {
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
    state.calendar_events.read().ok().and_then(|guard| {
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
    state.calendar_events.read().ok().and_then(|guard| {
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
                outcome.account.as_deref().unwrap_or(&outcome.meeting_title),
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
pub fn set_capture_enabled(enabled: bool, state: State<Arc<AppState>>) -> Result<(), String> {
    crate::state::create_or_update_config(&state, |config| {
        config.post_meeting_capture.enabled = enabled;
    })?;
    Ok(())
}

/// Set post-meeting capture delay (minutes before prompt appears)
#[tauri::command]
pub fn set_capture_delay(delay_minutes: u32, state: State<Arc<AppState>>) -> Result<(), String> {
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
    let ai_config = config.ai_models.clone();
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
            Some(&ai_config),
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
    let has_outcomes = result.status == "success"
        && (result.summary.as_ref().is_some_and(|s| !s.is_empty())
            || !result.wins.is_empty()
            || !result.risks.is_empty()
            || !result.decisions.is_empty()
            || !result.actions.is_empty());

    if result.status == "success" && has_outcomes {
        let processed_at = chrono::Utc::now().to_rfc3339();
        let transcript_destination = result.destination.clone().unwrap_or_default();
        let record = crate::types::TranscriptRecord {
            meeting_id: meeting_id.clone(),
            file_path: file_path_for_record,
            destination: transcript_destination.clone(),
            summary: result.summary.clone(),
            processed_at: processed_at.clone(),
        };

        if let Ok(mut guard) = state.transcript_processed.lock() {
            guard.insert(meeting_id.clone(), record);
            let _ = crate::state::save_transcript_records(&guard);
        }

        if let Ok(mut guard) = state.capture_captured.lock() {
            guard.insert(meeting_id.clone());
        }

        // Persist transcript metadata in SQLite so outcomes are durable without map files.
        if let Ok(db_guard) = state.db.lock() {
            if let Some(db) = db_guard.as_ref() {
                if let Err(e) = db.update_meeting_transcript_metadata(
                    &meeting_id,
                    &transcript_destination,
                    &processed_at,
                    result.summary.as_deref(),
                ) {
                    log::warn!("Failed to persist transcript metadata: {}", e);
                }
            }
        }

        // Build and emit outcome data for live frontend updates
        let outcome_data = build_outcome_data(&meeting_id, &result, &state);
        let _ = app_handle.emit("transcript-processed", &outcome_data);
    } else {
        // Processing failed or AI extraction was empty — remove sentinel so retry is possible
        if let Ok(mut guard) = state.transcript_processed.lock() {
            guard.remove(&meeting_id);
            let _ = crate::state::save_transcript_records(&guard);
        }
    }

    Ok(result)
}

/// Get meeting outcomes (from transcript processing or manual capture).
///
/// Returns `None` only when no outcomes/transcript metadata exist in SQLite.
#[tauri::command]
pub fn get_meeting_outcomes(
    meeting_id: String,
    state: State<Arc<AppState>>,
) -> Result<Option<crate::types::MeetingOutcomeData>, String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let meeting = db
        .get_meeting_intelligence_row(&meeting_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Meeting not found: {}", meeting_id))?;
    Ok(collect_meeting_outcomes_from_db(db, &meeting))
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
        return Err(format!(
            "Invalid priority: {}. Must be P1, P2, or P3.",
            priority
        ));
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
pub fn create_action(
    request: CreateActionRequest,
    state: State<Arc<AppState>>,
) -> Result<String, String> {
    let CreateActionRequest {
        title,
        priority,
        due_date,
        account_id,
        project_id,
        person_id,
        context,
        source_label,
    } = request;

    let title = crate::util::validate_bounded_string(&title, "title", 1, 280)?;
    let priority = priority.unwrap_or_else(|| "P2".to_string());
    crate::util::validate_enum_string(priority.as_str(), "priority", &["P1", "P2", "P3"])?;
    if let Some(ref date) = due_date {
        crate::util::validate_yyyy_mm_dd(date, "due_date")?;
    }
    if let Some(ref id) = account_id {
        crate::util::validate_id_slug(id, "account_id")?;
    }
    if let Some(ref id) = project_id {
        crate::util::validate_id_slug(id, "project_id")?;
    }
    if let Some(ref id) = person_id {
        crate::util::validate_id_slug(id, "person_id")?;
    }
    if let Some(ref value) = context {
        crate::util::validate_bounded_string(value, "context", 1, 2000)?;
    }
    if let Some(ref value) = source_label {
        crate::util::validate_bounded_string(value, "source_label", 1, 200)?;
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
pub fn update_action(
    request: UpdateActionRequest,
    state: State<Arc<AppState>>,
) -> Result<(), String> {
    let UpdateActionRequest {
        id,
        title,
        due_date,
        clear_due_date,
        context,
        clear_context,
        source_label,
        clear_source_label,
        account_id,
        clear_account,
        project_id,
        clear_project,
        person_id,
        clear_person,
        priority,
    } = request;

    crate::util::validate_id_slug(&id, "id")?;
    if let Some(ref p) = priority {
        crate::util::validate_enum_string(p.as_str(), "priority", &["P1", "P2", "P3"])?;
    }
    if let Some(ref t) = title {
        crate::util::validate_bounded_string(t, "title", 1, 280)?;
    }
    if let Some(ref d) = due_date {
        crate::util::validate_yyyy_mm_dd(d, "due_date")?;
    }
    if let Some(ref c) = context {
        crate::util::validate_bounded_string(c, "context", 1, 2000)?;
    }
    if let Some(ref s) = source_label {
        crate::util::validate_bounded_string(s, "source_label", 1, 200)?;
    }
    if let Some(ref a) = account_id {
        crate::util::validate_id_slug(a, "account_id")?;
    }
    if let Some(ref p) = project_id {
        crate::util::validate_id_slug(p, "project_id")?;
    }
    if let Some(ref p) = person_id {
        crate::util::validate_id_slug(p, "person_id")?;
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
        (
            "emailTriage",
            "Email Triage",
            "Fetch and classify Gmail messages",
            false,
        ),
        (
            "postMeetingCapture",
            "Post-Meeting Capture",
            "Prompt for outcomes after meetings end",
            false,
        ),
        (
            "meetingPrep",
            "Meeting Prep",
            "Generate prep context for upcoming meetings",
            false,
        ),
        (
            "weeklyPlanning",
            "Weekly Planning",
            "Weekly overview and focus block suggestions",
            false,
        ),
        (
            "inboxProcessing",
            "Inbox Processing",
            "Classify and route files from _inbox",
            false,
        ),
        (
            "accountTracking",
            "Account Tracking",
            "Track customer accounts, health, and ARR",
            true,
        ),
        (
            "projectTracking",
            "Project Tracking",
            "Track projects, milestones, and deliverables",
            false,
        ),
        (
            "impactRollup",
            "Impact Rollup",
            "Roll up daily wins and risks to account files",
            true,
        ),
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
pub fn install_demo_data(state: State<Arc<AppState>>) -> Result<String, String> {
    let workspace_path = state
        .config
        .read()
        .map_err(|_| "Config lock failed")?
        .as_ref()
        .map(|c| c.workspace_path.clone())
        .ok_or("No workspace configured")?;

    if !crate::devtools::is_dev_workspace(&state) {
        return Err(
            "Refused: demo data can only be installed in the dev sandbox \
             (~/Documents/DailyOS-dev). Switch workspace first."
                .into(),
        );
    }

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

        // Create folder + directory template (ADR-0059, idempotent)
        let account_dir = workspace.join("Accounts").join(name);
        if let Err(e) = std::fs::create_dir_all(&account_dir) {
            log::warn!("Failed to create account dir '{}': {}", name, e);
            continue;
        }
        if let Err(e) = crate::util::bootstrap_entity_directory(&account_dir, name, "account") {
            log::warn!("Failed to bootstrap account template '{}': {}", name, e);
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
            nps: None,
            tracker_path: Some(format!("Accounts/{}", name)),
            parent_id: None,
            is_internal: false,
            updated_at: now.clone(),
            archived: false,
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
            archived: false,
        };

        // Create folder + directory template (ADR-0059, idempotent)
        let project_dir = workspace.join("Projects").join(name);
        if let Err(e) = std::fs::create_dir_all(&project_dir) {
            log::warn!("Failed to create project dir '{}': {}", name, e);
        }
        if let Err(e) = crate::util::bootstrap_entity_directory(&project_dir, name, "project") {
            log::warn!("Failed to bootstrap project template '{}': {}", name, e);
        }

        if let Ok(db_guard) = state.db.lock() {
            if let Some(db) = db_guard.as_ref() {
                if let Err(e) = db.upsert_project(&db_project) {
                    log::warn!("Failed to upsert project '{}': {}", name, e);
                }
                // Write dashboard.json + dashboard.md
                let json = crate::projects::default_project_json(&db_project);
                let _ =
                    crate::projects::write_project_json(workspace, &db_project, Some(&json), db);
                let _ = crate::projects::write_project_markdown(
                    workspace,
                    &db_project,
                    Some(&json),
                    db,
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

    let (hints, internal_root) = {
        let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
        let db = db_guard.as_ref().ok_or("Database not initialized")?;
        (
            crate::helpers::build_external_account_hints(db),
            db.get_internal_root_account().ok().flatten(),
        )
    };

    let mut cards = Vec::new();
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

        let mut suggested_entity_id = None;
        let mut suggested_entity_name = None;
        if let Ok(db_guard) = state.db.lock() {
            if let Some(db) = db_guard.as_ref() {
                if let Some(ref account_hint) = cm.account {
                    if let Ok(Some(account)) = db.get_account_by_name(account_hint) {
                        suggested_entity_id = Some(account.id.clone());
                        suggested_entity_name = Some(account.name.clone());
                    }
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
            start_time: Some(start.to_rfc3339()),
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
}

#[derive(Debug, Clone)]
struct ClaudeStatusCacheEntry {
    status: ClaudeStatus,
    checked_at: std::time::Instant,
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
    static STATUS_CACHE: OnceLock<Mutex<Option<ClaudeStatusCacheEntry>>> = OnceLock::new();
    let cache = STATUS_CACHE.get_or_init(|| Mutex::new(None));
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
        ClaudeStatus {
            installed,
            authenticated,
        }
    })
    .await
    .unwrap_or(ClaudeStatus {
        installed: false,
        authenticated: false,
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

// =============================================================================
// Onboarding: Inbox Training Sample (I78)
// =============================================================================

/// Copy a bundled sample meeting notes file into _inbox/ for onboarding training.
///
/// Returns the filename of the installed sample.
#[tauri::command]
pub fn install_inbox_sample(state: State<Arc<AppState>>) -> Result<String, String> {
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

// =============================================================================
// Dev Tools
// =============================================================================

/// Apply a dev scenario (reset, mock_full, mock_no_auth, mock_empty).
///
/// Returns an error in release builds. In debug builds, delegates to
/// `devtools::apply_scenario` which orchestrates the scenario switch.
#[tauri::command]
pub fn dev_apply_scenario(scenario: String, state: State<Arc<AppState>>) -> Result<String, String> {
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
pub fn dev_get_state(state: State<Arc<AppState>>) -> Result<crate::devtools::DevState, String> {
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
pub fn dev_run_today_mechanical(state: State<Arc<AppState>>) -> Result<String, String> {
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
pub fn dev_run_today_full(state: State<Arc<AppState>>) -> Result<String, String> {
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
pub fn dev_run_week_mechanical(state: State<Arc<AppState>>) -> Result<String, String> {
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
pub fn dev_run_week_full(state: State<Arc<AppState>>) -> Result<String, String> {
    if !cfg!(debug_assertions) {
        return Err("Dev tools not available in release builds".into());
    }
    crate::devtools::run_week_full(&state)
}

/// Restore from dev mode to live mode (I298).
///
/// Deactivates dev DB isolation, reopens the live database, and restores the
/// original workspace path. Returns a confirmation message.
#[tauri::command]
pub fn dev_restore_live(state: State<Arc<AppState>>) -> Result<String, String> {
    if !cfg!(debug_assertions) {
        return Err("Dev tools not available in release builds".into());
    }
    crate::devtools::restore_live(&state)
}

/// Purge all known mock/dev data from the current database (I298).
///
/// Removes exact mock IDs seeded by devtools scenarios. Safe for the live DB.
#[tauri::command]
pub fn dev_purge_mock_data(state: State<Arc<AppState>>) -> Result<String, String> {
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
///
/// Cross-references SQLite data + today's schedule to surface decisions due,
/// stale delegations, portfolio alerts, cancelable meetings, and skip-today items.
#[tauri::command]
pub fn get_executive_intelligence(
    state: State<Arc<AppState>>,
) -> Result<crate::intelligence::ExecutiveIntelligence, String> {
    let started = std::time::Instant::now();
    let result = (|| {
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
    })();

    log_command_latency(
        "get_executive_intelligence",
        started,
        READ_CMD_LATENCY_BUDGET_MS,
    );
    result
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
            let person_dir =
                crate::people::person_dir(Path::new(&config.workspace_path), &person.name);
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

/// Reassign a meeting's entity with full cascade to actions, captures, and intelligence.
/// Clears existing entity links, sets the new one, and cascades to related tables.
#[tauri::command]
pub fn update_meeting_entity(
    meeting_id: String,
    entity_id: Option<String>,
    entity_type: String,
    meeting_title: String,
    start_time: String,
    meeting_type_str: String,
    state: State<Arc<AppState>>,
) -> Result<(), String> {
    // Collect old entity IDs before modifying (for intelligence queue)
    let old_entity_ids: Vec<(String, String)>;

    {
        let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
        let db = db_guard.as_ref().ok_or("Database not initialized")?;

        old_entity_ids = db
            .get_meeting_entities(&meeting_id)
            .unwrap_or_default()
            .into_iter()
            .map(|e| (e.id, e.entity_type.as_str().to_string()))
            .collect();

        // Ensure meeting exists without clobbering existing metadata.
        db.ensure_meeting_in_history(crate::db::EnsureMeetingHistoryInput {
            id: &meeting_id,
            title: &meeting_title,
            meeting_type: &meeting_type_str,
            start_time: &start_time,
            end_time: None,
            account_id: None,
            calendar_event_id: None,
        })
        .map_err(|e| e.to_string())?;

        // Clear all existing entity links
        db.clear_meeting_entities(&meeting_id)
            .map_err(|e| e.to_string())?;

        // Determine account_id and project_id for cascade
        let (cascade_account, cascade_project) = match entity_type.as_str() {
            "account" => (entity_id.as_deref(), None),
            "project" => (None, entity_id.as_deref()),
            _ => (entity_id.as_deref(), None),
        };

        // Link new entity if provided
        if let Some(ref eid) = entity_id {
            db.link_meeting_entity(&meeting_id, eid, &entity_type)
                .map_err(|e| e.to_string())?;
        }

        // Update legacy account_id on meetings_history
        db.update_meeting_account(&meeting_id, cascade_account)
            .map_err(|e| e.to_string())?;

        // Cascade to actions and captures
        db.cascade_meeting_entity_to_actions(&meeting_id, cascade_account, cascade_project)
            .map_err(|e| e.to_string())?;
        db.cascade_meeting_entity_to_captures(&meeting_id, cascade_account, cascade_project)
            .map_err(|e| e.to_string())?;

        // Cascade to people: link external attendees to the entity (I184)
        db.cascade_meeting_entity_to_people(&meeting_id, cascade_account, cascade_project)
            .map_err(|e| e.to_string())?;
    }
    // DB lock released

    // Queue intelligence refresh for old and new entities
    let mut entities_to_refresh: Vec<(String, String)> = old_entity_ids;
    if let Some(ref eid) = entity_id {
        entities_to_refresh.push((eid.clone(), entity_type.clone()));
    }
    // Dedup
    entities_to_refresh.sort();
    entities_to_refresh.dedup();
    for (eid, etype) in entities_to_refresh {
        state.intel_queue.enqueue(crate::intel_queue::IntelRequest {
            entity_id: eid,
            entity_type: etype,
            priority: crate::intel_queue::IntelPriority::CalendarChange,
            requested_at: std::time::Instant::now(),
        });
    }

    Ok(())
}

// =========================================================================
// Additive Meeting-Entity Link/Unlink (I184 multi-entity)
// =========================================================================

/// Add an entity link to a meeting with full cascade (people, intelligence).
/// Unlike `update_meeting_entity` which clears-and-replaces, this is additive.
#[tauri::command]
pub fn add_meeting_entity(
    meeting_id: String,
    entity_id: String,
    entity_type: String,
    meeting_title: String,
    start_time: String,
    meeting_type_str: String,
    state: State<Arc<AppState>>,
) -> Result<(), String> {
    {
        let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
        let db = db_guard.as_ref().ok_or("Database not initialized")?;

        // Ensure meeting exists without clobbering existing metadata.
        db.ensure_meeting_in_history(crate::db::EnsureMeetingHistoryInput {
            id: &meeting_id,
            title: &meeting_title,
            meeting_type: &meeting_type_str,
            start_time: &start_time,
            end_time: None,
            account_id: None,
            calendar_event_id: None,
        })
        .map_err(|e| e.to_string())?;

        // Add entity link (idempotent)
        db.link_meeting_entity(&meeting_id, &entity_id, &entity_type)
            .map_err(|e| e.to_string())?;

        // Update legacy account_id if linking an account
        if entity_type == "account" {
            db.update_meeting_account(&meeting_id, Some(&entity_id))
                .map_err(|e| e.to_string())?;
        }

        // Cascade people to this entity
        let (cascade_account, cascade_project) = match entity_type.as_str() {
            "account" => (Some(entity_id.as_str()), None),
            "project" => (None, Some(entity_id.as_str())),
            _ => (Some(entity_id.as_str()), None),
        };
        db.cascade_meeting_entity_to_people(&meeting_id, cascade_account, cascade_project)
            .map_err(|e| e.to_string())?;
    }
    // DB lock released

    // Queue intelligence refresh
    state.intel_queue.enqueue(crate::intel_queue::IntelRequest {
        entity_id,
        entity_type,
        priority: crate::intel_queue::IntelPriority::CalendarChange,
        requested_at: std::time::Instant::now(),
    });

    Ok(())
}

/// Remove an entity link from a meeting with cleanup (legacy account_id, intelligence).
#[tauri::command]
pub fn remove_meeting_entity(
    meeting_id: String,
    entity_id: String,
    entity_type: String,
    state: State<Arc<AppState>>,
) -> Result<(), String> {
    {
        let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
        let db = db_guard.as_ref().ok_or("Database not initialized")?;

        db.unlink_meeting_entity(&meeting_id, &entity_id)
            .map_err(|e| e.to_string())?;

        // If we removed an account, update legacy account_id to next linked account (or null)
        if entity_type == "account" {
            let remaining = db.get_meeting_entities(&meeting_id).unwrap_or_default();
            let next_account = remaining
                .iter()
                .find(|e| e.entity_type == crate::entity::EntityType::Account);
            db.update_meeting_account(&meeting_id, next_account.map(|a| a.id.as_str()))
                .map_err(|e| e.to_string())?;
        }
    }
    // DB lock released

    // Queue intelligence refresh for removed entity
    state.intel_queue.enqueue(crate::intel_queue::IntelRequest {
        entity_id,
        entity_type,
        priority: crate::intel_queue::IntelPriority::CalendarChange,
        requested_at: std::time::Instant::now(),
    });

    Ok(())
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
    let email = crate::util::validate_email(&email)?;

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
        archived: false,
    };

    db.upsert_person(&person).map_err(|e| e.to_string())?;
    Ok(id)
}

/// Merge two people: transfer all references from `remove_id` to `keep_id`, then delete the removed person.
/// Also cleans up filesystem directories and regenerates the kept person's files.
#[tauri::command]
pub fn merge_people(
    keep_id: String,
    remove_id: String,
    state: State<Arc<AppState>>,
) -> Result<String, String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    // Get removed person's info before merge (for filesystem cleanup)
    let removed = db
        .get_person(&remove_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Person not found: {}", remove_id))?;

    // Perform DB merge
    db.merge_people(&keep_id, &remove_id)
        .map_err(|e| e.to_string())?;

    // Filesystem cleanup
    let config = state.config.read().map_err(|_| "Lock poisoned")?;
    if let Some(ref config) = *config {
        let workspace = Path::new(&config.workspace_path);

        // Remove the merged-away person's directory
        let remove_dir = if let Some(ref tp) = removed.tracker_path {
            workspace.join(tp)
        } else {
            crate::people::person_dir(workspace, &removed.name)
        };
        if remove_dir.exists() {
            let _ = std::fs::remove_dir_all(&remove_dir);
        }

        // Regenerate kept person's files
        if let Ok(Some(kept)) = db.get_person(&keep_id) {
            let _ = crate::people::write_person_json(workspace, &kept, db);
            let _ = crate::people::write_person_markdown(workspace, &kept, db);
        }
    }

    Ok(keep_id)
}

/// Delete a person and all their references. Also removes their filesystem directory.
#[tauri::command]
pub fn delete_person(person_id: String, state: State<Arc<AppState>>) -> Result<(), String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    // Get person info before delete (for filesystem cleanup)
    let person = db
        .get_person(&person_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Person not found: {}", person_id))?;

    // Perform DB delete
    db.delete_person(&person_id).map_err(|e| e.to_string())?;

    // Filesystem cleanup
    let config = state.config.read().map_err(|_| "Lock poisoned")?;
    if let Some(ref config) = *config {
        let workspace = Path::new(&config.workspace_path);
        let person_dir = if let Some(ref tp) = person.tracker_path {
            workspace.join(tp)
        } else {
            crate::people::person_dir(workspace, &person.name)
        };
        if person_dir.exists() {
            let _ = std::fs::remove_dir_all(&person_dir);
        }
    }

    Ok(())
}

/// Enrich a person with intelligence assessment (relationship intelligence).
/// Uses split-lock pattern (I173) — DB lock held only briefly during gather/write.
#[tauri::command]
pub async fn enrich_person(
    person_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::entity_intel::IntelligenceJson, String> {
    use crate::intel_queue::{
        gather_enrichment_input, run_enrichment, write_enrichment_results, IntelPriority,
        IntelRequest,
    };

    let request = IntelRequest {
        entity_id: person_id,
        entity_type: "person".to_string(),
        priority: IntelPriority::Manual,
        requested_at: std::time::Instant::now(),
    };

    // Phase 1: Brief DB lock — gather context
    let input = gather_enrichment_input(&state, &request)?;

    // Phase 2: No lock — PTY enrichment
    let ai_config = state
        .config
        .read()
        .ok()
        .and_then(|g| g.as_ref().map(|c| c.ai_models.clone()))
        .unwrap_or_default();
    let intel = run_enrichment(&input, &ai_config)?;

    // Phase 3: Brief DB lock — write results
    write_enrichment_results(&state, &input, &intel)?;

    Ok(intel)
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
    pub is_internal: bool,
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
    pub is_internal: bool,
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
pub fn get_accounts_list(state: State<Arc<AppState>>) -> Result<Vec<AccountListItem>, String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let accounts = db.get_top_level_accounts().map_err(|e| e.to_string())?;

    let items: Vec<AccountListItem> = accounts
        .into_iter()
        .map(|a| {
            let child_count = db.get_child_accounts(&a.id).map(|c| c.len()).unwrap_or(0);

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
    pub is_internal: bool,
}

#[tauri::command]
pub fn get_accounts_for_picker(state: State<Arc<AppState>>) -> Result<Vec<PickerAccount>, String> {
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
            let parent_name = a
                .parent_id
                .as_ref()
                .and_then(|pid| parent_names.get(pid).cloned());
            PickerAccount {
                id: a.id,
                name: a.name,
                parent_name,
                is_internal: a.is_internal,
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

    let children = db
        .get_child_accounts(&parent_id)
        .map_err(|e| e.to_string())?;

    // Look up parent name for breadcrumb context
    let parent_name = db.get_account(&parent_id).ok().flatten().map(|a| a.name);

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
                    chrono::DateTime::parse_from_rfc3339(&format!(
                        "{}+00:00",
                        lm.trim_end_matches('Z')
                    ))
                })
                .ok()
                .map(|dt| (chrono::Utc::now() - dt.with_timezone(&chrono::Utc)).num_days())
        })
    });

    let team_summary = db.get_account_team(&a.id).ok().and_then(|members| {
        if members.is_empty() {
            None
        } else {
            let labels: Vec<String> = members
                .iter()
                .take(2)
                .map(|m| format!("{} ({})", m.person_name, m.role.to_uppercase()))
                .collect();
            let suffix = if members.len() > 2 {
                format!(" +{}", members.len() - 2)
            } else {
                String::new()
            };
            Some(format!("Team: {}{}", labels.join(", "), suffix))
        }
    });

    AccountListItem {
        id: a.id.clone(),
        name: a.name.clone(),
        lifecycle: a.lifecycle.clone(),
        arr: a.arr,
        health: a.health.clone(),
        nps: a.nps,
        team_summary,
        renewal_date: a.contract_end.clone(),
        open_action_count,
        days_since_last_meeting,
        parent_id: a.parent_id.clone(),
        parent_name: None,
        child_count,
        is_parent: child_count > 0,
        is_internal: a.is_internal,
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
        let intel = crate::entity_intel::read_intelligence_json(&account_dir)
            .ok()
            .or_else(|| {
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

    let recent_meetings: Vec<MeetingPreview> = db
        .get_meetings_for_account_with_prep(&account_id, 10)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|m| {
            let prep_context = m
                .prep_context_json
                .as_ref()
                .and_then(|json_str| serde_json::from_str::<PrepContext>(json_str).ok());
            MeetingPreview {
                id: m.id,
                title: m.title,
                start_time: m.start_time,
                meeting_type: m.meeting_type,
                prep_context,
            }
        })
        .collect();

    let linked_people = db.get_people_for_entity(&account_id).unwrap_or_default();
    let account_team = db.get_account_team(&account_id).unwrap_or_default();
    let account_team_import_notes = db
        .get_account_team_import_notes(&account_id)
        .unwrap_or_default();

    let signals = db.get_stakeholder_signals(&account_id).ok();

    let recent_captures = db
        .get_captures_for_account(&account_id, 90)
        .unwrap_or_default();
    let recent_email_signals = db
        .list_recent_email_signals_for_entity(&account_id, 12)
        .unwrap_or_default();

    // I114: Resolve parent name for child accounts, children for parent accounts
    let parent_name = account
        .parent_id
        .as_ref()
        .and_then(|pid| db.get_account(pid).ok().flatten().map(|a| a.name));

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
        renewal_date: account.contract_end,
        contract_start: account.contract_start,
        company_overview: overview,
        strategic_programs: programs,
        notes,
        open_actions,
        upcoming_meetings,
        recent_meetings,
        linked_people,
        account_team,
        account_team_import_notes,
        signals,
        recent_captures,
        recent_email_signals,
        parent_id: account.parent_id,
        parent_name,
        children,
        parent_aggregate,
        is_internal: account.is_internal,
        intelligence,
    })
}

/// Get account-team members (I207).
#[tauri::command]
pub fn get_account_team(
    account_id: String,
    state: State<Arc<AppState>>,
) -> Result<Vec<crate::db::DbAccountTeamMember>, String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    db.get_account_team(&account_id).map_err(|e| e.to_string())
}

/// Add a person-role pair to an account team (I207).
#[tauri::command]
pub fn add_account_team_member(
    account_id: String,
    person_id: String,
    role: String,
    state: State<Arc<AppState>>,
) -> Result<(), String> {
    let role = role.trim().to_lowercase();
    if role.is_empty() {
        return Err("Role is required".to_string());
    }
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    db.add_account_team_member(&account_id, &person_id, &role)
        .map_err(|e| e.to_string())
}

/// Remove a person-role pair from an account team (I207).
#[tauri::command]
pub fn remove_account_team_member(
    account_id: String,
    person_id: String,
    role: String,
    state: State<Arc<AppState>>,
) -> Result<(), String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    db.remove_account_team_member(&account_id, &person_id, &role)
        .map_err(|e| e.to_string())
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
            let json_path =
                crate::accounts::resolve_account_dir(workspace, &account).join("dashboard.json");
            let existing = if json_path.exists() {
                crate::accounts::read_account_json(&json_path)
                    .ok()
                    .map(|r| r.json)
            } else {
                None
            };
            let _ = crate::accounts::write_account_json(workspace, &account, existing.as_ref(), db);
            let _ =
                crate::accounts::write_account_markdown(workspace, &account, existing.as_ref(), db);
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
    let json_path =
        crate::accounts::resolve_account_dir(workspace, &account).join("dashboard.json");
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

    let programs: Vec<crate::accounts::StrategicProgram> = serde_json::from_str(&programs_json)
        .map_err(|e| format!("Invalid programs JSON: {}", e))?;

    let config = state.config.read().map_err(|_| "Lock poisoned")?;
    let config = config.as_ref().ok_or("Config not loaded")?;
    let workspace = Path::new(&config.workspace_path);

    let json_path =
        crate::accounts::resolve_account_dir(workspace, &account).join("dashboard.json");
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
    let (id, tracker_path, is_internal) = if let Some(ref pid) = parent_id {
        let parent = db
            .get_account(pid)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Parent account not found: {}", pid))?;
        let child_id = format!("{}--{}", pid, crate::util::slugify(&name));
        let parent_dir = parent
            .tracker_path
            .unwrap_or_else(|| format!("Accounts/{}", parent.name));
        let tp = format!("{}/{}", parent_dir, name);
        (child_id, tp, parent.is_internal)
    } else {
        let id = crate::util::slugify(&name);
        (id, format!("Accounts/{}", name), false)
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
        nps: None,
        tracker_path: Some(tracker_path),
        parent_id,
        is_internal,
        updated_at: now,
        archived: false,
    };

    db.upsert_account(&account).map_err(|e| e.to_string())?;
    if let Some(ref pid) = account.parent_id {
        let _ = db.copy_account_domains(pid, &account.id);
    }

    // Create workspace files + directory template (ADR-0059)
    let config = state.config.read().map_err(|_| "Lock poisoned")?;
    if let Some(ref config) = *config {
        let workspace = Path::new(&config.workspace_path);
        let account_dir = crate::accounts::resolve_account_dir(workspace, &account);
        let _ = std::fs::create_dir_all(&account_dir);
        let _ = crate::util::bootstrap_entity_directory(&account_dir, &name, "account");
        let _ = crate::accounts::write_account_json(workspace, &account, None, db);
        let _ = crate::accounts::write_account_markdown(workspace, &account, None, db);
    }

    Ok(id)
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

// Domain normalization moved to crate::util::normalize_domains (DRY)

fn normalize_key(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect()
}

fn create_child_account_record(
    db: &crate::db::ActionDb,
    workspace: Option<&Path>,
    parent: &crate::db::DbAccount,
    name: &str,
    description: Option<&str>,
    owner_person_id: Option<&str>,
) -> Result<crate::db::DbAccount, String> {
    let children = db
        .get_child_accounts(&parent.id)
        .map_err(|e| e.to_string())?;
    if children.iter().any(|c| c.name.eq_ignore_ascii_case(name)) {
        return Err(format!(
            "A child named '{}' already exists under '{}'",
            name, parent.name
        ));
    }

    let base_slug = crate::util::slugify(name);
    let mut id = format!("{}--{}", parent.id, base_slug);
    let mut suffix = 2usize;
    while db.get_account(&id).map_err(|e| e.to_string())?.is_some() {
        id = format!("{}--{}-{}", parent.id, base_slug, suffix);
        suffix += 1;
    }

    let parent_tracker = parent.tracker_path.clone().unwrap_or_else(|| {
        if parent.is_internal {
            format!("Internal/{}", parent.name)
        } else {
            format!("Accounts/{}", parent.name)
        }
    });
    let tracker_path = format!("{}/{}", parent_tracker, name);
    let now = chrono::Utc::now().to_rfc3339();

    let account = crate::db::DbAccount {
        id,
        name: name.to_string(),
        lifecycle: None,
        arr: None,
        health: None,
        contract_start: None,
        contract_end: None,
        nps: None,
        tracker_path: Some(tracker_path),
        parent_id: Some(parent.id.clone()),
        is_internal: parent.is_internal,
        updated_at: now,
        archived: false,
    };

    db.upsert_account(&account).map_err(|e| e.to_string())?;
    db.copy_account_domains(&parent.id, &account.id)
        .map_err(|e| e.to_string())?;

    if let Some(owner_id) = owner_person_id {
        db.link_person_to_entity(owner_id, &account.id, "owner")
            .map_err(|e| e.to_string())?;
    }

    if let Some(ws) = workspace {
        let account_dir = crate::accounts::resolve_account_dir(ws, &account);
        let _ = std::fs::create_dir_all(&account_dir);
        let _ = crate::util::bootstrap_entity_directory(&account_dir, name, "account");

        let mut json = default_account_json(&account);
        if let Some(desc) = description {
            let trimmed = desc.trim();
            if !trimmed.is_empty() {
                json.notes = Some(trimmed.to_string());
            }
        }
        let _ = crate::accounts::write_account_json(ws, &account, Some(&json), db);
        let _ = crate::accounts::write_account_markdown(ws, &account, Some(&json), db);
    }

    Ok(account)
}

fn infer_internal_account_for_meeting(
    db: &crate::db::ActionDb,
    title: &str,
    attendees_csv: Option<&str>,
) -> Option<crate::db::DbAccount> {
    let internal_accounts = db.get_internal_accounts().ok()?;
    if internal_accounts.is_empty() {
        return None;
    }
    let root = internal_accounts
        .iter()
        .find(|a| a.parent_id.is_none())
        .cloned();
    let candidates: Vec<crate::db::DbAccount> = internal_accounts
        .iter()
        .filter(|a| a.parent_id.is_some())
        .cloned()
        .collect();
    if candidates.is_empty() {
        return root;
    }

    let title_key = normalize_key(title);
    let attendee_set: HashSet<String> = attendees_csv
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| s.contains('@'))
        .collect();

    let mut best: Option<(i32, crate::db::DbAccount)> = None;
    for candidate in candidates {
        let mut score = 0i32;
        let name_key = normalize_key(&candidate.name);
        if !name_key.is_empty() && title_key.contains(&name_key) {
            score += 2;
        }

        let overlaps = db
            .get_people_for_entity(&candidate.id)
            .unwrap_or_default()
            .iter()
            .filter(|p| attendee_set.contains(&p.email.to_lowercase()))
            .count() as i32;
        score += overlaps * 3;

        match &best {
            None => best = Some((score, candidate)),
            Some((best_score, best_acc)) => {
                if score > *best_score
                    || (score == *best_score
                        && candidate.name.to_lowercase() < best_acc.name.to_lowercase())
                {
                    best = Some((score, candidate));
                }
            }
        }
    }

    match best {
        Some((score, account)) if score > 0 => Some(account),
        _ => root,
    }
}

#[tauri::command]
pub fn get_internal_team_setup_status(
    state: State<Arc<AppState>>,
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

    let suggested_colleagues = {
        let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
        let db = db_guard.as_ref().ok_or("Database not initialized")?;
        db.get_people(Some("internal"))
            .map_err(|e| e.to_string())?
            .into_iter()
            .take(5)
            .map(|p| TeamColleagueInput {
                name: p.name,
                email: p.email,
                title: p.role,
            })
            .collect::<Vec<_>>()
    };

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
pub fn create_internal_organization(
    company_name: String,
    domains: Vec<String>,
    team_name: String,
    colleagues: Vec<TeamColleagueInput>,
    existing_person_ids: Option<Vec<String>>,
    state: State<Arc<AppState>>,
) -> Result<CreateInternalOrganizationResult, String> {
    // Validation (before transaction)
    let company_name = crate::util::validate_entity_name(&company_name)?.to_string();
    let team_name = crate::util::validate_entity_name(&team_name)?.to_string();
    let domains = crate::util::normalize_domains(&domains);
    let workspace_path = state
        .config
        .read()
        .map_err(|_| "Lock poisoned")?
        .as_ref()
        .map(|c| c.workspace_path.clone())
        .ok_or("Config not loaded")?;
    let workspace = Path::new(&workspace_path);

    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    // === CRITICAL SECTION: Transaction wraps all DB writes ===
    // Filesystem writes happen after commit (best-effort per ADR-0048).
    let (root_account, initial_team, created_people, updated_people) =
        db.with_transaction(|db| {
            if db
                .get_internal_root_account()
                .map_err(|e| e.to_string())?
                .is_some()
            {
                return Err("Internal organization already exists".to_string());
            }

            let mut root_id = format!("internal-{}", crate::util::slugify(&company_name));
            let mut suffix = 2usize;
            while db
                .get_account(&root_id)
                .map_err(|e| e.to_string())?
                .is_some()
            {
                root_id = format!(
                    "internal-{}-{}",
                    crate::util::slugify(&company_name),
                    suffix
                );
                suffix += 1;
            }

            let now = chrono::Utc::now().to_rfc3339();
            let root_account = crate::db::DbAccount {
                id: root_id.clone(),
                name: company_name.clone(),
                lifecycle: Some("active".to_string()),
                arr: None,
                health: Some("green".to_string()),
                contract_start: None,
                contract_end: None,
                nps: None,
                tracker_path: Some(format!("Internal/{}", company_name)),
                parent_id: None,
                is_internal: true,
                updated_at: now,
                archived: false,
            };
            db.upsert_account(&root_account)
                .map_err(|e| e.to_string())?;
            db.set_account_domains(&root_account.id, &domains)
                .map_err(|e| e.to_string())?;

            let initial_team =
                create_child_account_record(db, None, &root_account, &team_name, None, None)?;
            db.copy_account_domains(&root_account.id, &initial_team.id)
                .map_err(|e| e.to_string())?;

            let mut created_people: Vec<crate::db::DbPerson> = Vec::new();
            for colleague in &colleagues {
                let email = match crate::util::validate_email(&colleague.email) {
                    Ok(e) => e,
                    Err(_) => continue,
                };
                let person_id = crate::util::slugify(&email);
                let now = chrono::Utc::now().to_rfc3339();
                let person = crate::db::DbPerson {
                    id: person_id.clone(),
                    email: email.clone(),
                    name: colleague.name.trim().to_string(),
                    organization: Some(company_name.clone()),
                    role: colleague.title.clone(),
                    relationship: "internal".to_string(),
                    notes: None,
                    tracker_path: None,
                    last_seen: None,
                    first_seen: Some(now.clone()),
                    meeting_count: 0,
                    updated_at: now,
                    archived: false,
                };
                db.upsert_person(&person).map_err(|e| e.to_string())?;
                db.link_person_to_entity(&person_id, &root_account.id, "member")
                    .map_err(|e| e.to_string())?;
                db.link_person_to_entity(&person_id, &initial_team.id, "member")
                    .map_err(|e| e.to_string())?;
                created_people.push(person);
            }

            let mut updated_people: Vec<crate::db::DbPerson> = Vec::new();
            for person_id in existing_person_ids.unwrap_or_default() {
                if let Ok(Some(mut person)) = db.get_person(&person_id) {
                    if person.relationship != "internal" {
                        person.relationship = "internal".to_string();
                        person.organization = Some(company_name.clone());
                        db.upsert_person(&person).map_err(|e| e.to_string())?;
                        updated_people.push(person);
                    }
                    db.link_person_to_entity(&person_id, &root_account.id, "member")
                        .map_err(|e| e.to_string())?;
                    db.link_person_to_entity(&person_id, &initial_team.id, "member")
                        .map_err(|e| e.to_string())?;
                }
            }

            Ok((root_account, initial_team, created_people, updated_people))
        })?;

    // Filesystem writes (best-effort, outside transaction)
    let root_dir = crate::accounts::resolve_account_dir(workspace, &root_account);
    let _ = std::fs::create_dir_all(&root_dir);
    let _ = crate::util::bootstrap_entity_directory(&root_dir, &company_name, "account");
    let _ = crate::accounts::write_account_json(workspace, &root_account, None, db);
    let _ = crate::accounts::write_account_markdown(workspace, &root_account, None, db);

    let team_dir = crate::accounts::resolve_account_dir(workspace, &initial_team);
    let _ = std::fs::create_dir_all(&team_dir);
    let _ = crate::util::bootstrap_entity_directory(&team_dir, &team_name, "account");
    let _ = crate::accounts::write_account_json(workspace, &initial_team, None, db);
    let _ = crate::accounts::write_account_markdown(workspace, &initial_team, None, db);

    for person in &created_people {
        let _ = crate::people::write_person_json(workspace, person, db);
        let _ = crate::people::write_person_markdown(workspace, person, db);
    }
    for person in &updated_people {
        let _ = crate::people::write_person_json(workspace, person, db);
        let _ = crate::people::write_person_markdown(workspace, person, db);
    }

    drop(db_guard);

    crate::state::create_or_update_config(&state, |config| {
        config.internal_team_setup_completed = true;
        config.internal_team_setup_version = 1;
        config.internal_org_account_id = Some(root_account.id.clone());
        if config.user_company.is_none() {
            config.user_company = Some(company_name.clone());
        }
        if !domains.is_empty() {
            config.user_domain = domains.first().cloned();
            config.user_domains = Some(domains.clone());
        }
    })?;

    Ok(CreateInternalOrganizationResult {
        root_account_id: root_account.id,
        initial_team_id: initial_team.id,
    })
}

#[tauri::command]
pub fn create_child_account(
    parent_id: String,
    name: String,
    description: Option<String>,
    owner_person_id: Option<String>,
    state: State<Arc<AppState>>,
) -> Result<CreateChildAccountResult, String> {
    let name = crate::util::validate_entity_name(&name)?.to_string();
    let workspace_path = state
        .config
        .read()
        .map_err(|_| "Lock poisoned")?
        .as_ref()
        .map(|c| c.workspace_path.clone());
    let workspace = workspace_path.as_deref().map(Path::new);

    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let parent = db
        .get_account(&parent_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Parent account not found: {}", parent_id))?;
    let child = create_child_account_record(
        db,
        workspace,
        &parent,
        &name,
        description.as_deref(),
        owner_person_id.as_deref(),
    )?;
    drop(db_guard);

    state.intel_queue.enqueue(crate::intel_queue::IntelRequest {
        entity_id: child.id.clone(),
        entity_type: "account".to_string(),
        priority: crate::intel_queue::IntelPriority::ContentChange,
        requested_at: std::time::Instant::now(),
    });

    Ok(CreateChildAccountResult { id: child.id })
}

#[tauri::command]
pub fn create_team(
    name: String,
    description: Option<String>,
    owner_person_id: Option<String>,
    state: State<Arc<AppState>>,
) -> Result<CreateChildAccountResult, String> {
    let root_id = {
        let cfg = state
            .config
            .read()
            .map_err(|_| "Lock poisoned")?
            .clone()
            .ok_or("Config not loaded")?;
        if let Some(id) = cfg.internal_org_account_id {
            id
        } else {
            let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
            let db = db_guard.as_ref().ok_or("Database not initialized")?;
            db.get_internal_root_account()
                .map_err(|e| e.to_string())?
                .map(|a| a.id)
                .ok_or("No internal organization configured")?
        }
    };

    create_child_account(root_id, name, description, owner_person_id, state)
}

#[tauri::command]
pub fn backfill_internal_meeting_associations(
    state: State<Arc<AppState>>,
) -> Result<usize, String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let mut stmt = db
        .conn_ref()
        .prepare(
            "SELECT m.id, m.title, m.attendees
             FROM meetings_history m
             LEFT JOIN meeting_entities me ON me.meeting_id = m.id AND me.entity_type = 'account'
             WHERE m.meeting_type IN ('internal', 'team_sync', 'one_on_one')
               AND me.meeting_id IS NULL",
        )
        .map_err(|e| e.to_string())?;
    let meetings: Vec<(String, String, Option<String>)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut updated = 0usize;
    for (meeting_id, title, attendees) in meetings {
        let Some(account) = infer_internal_account_for_meeting(db, &title, attendees.as_deref())
        else {
            continue;
        };
        let _ = db.link_meeting_entity(&meeting_id, &account.id, "account");
        let _ = db.update_meeting_account(&meeting_id, Some(&account.id));
        let _ = db.cascade_meeting_entity_to_people(&meeting_id, Some(&account.id), None);
        updated += 1;
    }

    Ok(updated)
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
    let files = db.get_entity_files(&entity_id).map_err(|e| e.to_string())?;
    drop(db_guard);

    state
        .embedding_queue
        .enqueue(crate::processor::embeddings::EmbeddingRequest {
            entity_id: entity_id.clone(),
            entity_type: "account".to_string(),
            requested_at: std::time::Instant::now(),
        });
    state.intel_queue.enqueue(crate::intel_queue::IntelRequest {
        entity_id,
        entity_type: "account".to_string(),
        priority: crate::intel_queue::IntelPriority::ContentChange,
        requested_at: std::time::Instant::now(),
    });

    Ok(files)
}

/// Reveal a file in macOS Finder.
///
/// Path must resolve to within the workspace directory or ~/.dailyos/ (I293).
#[tauri::command]
pub fn reveal_in_finder(
    path: String,
    state: State<Arc<AppState>>,
) -> Result<(), String> {
    let canonical = std::fs::canonicalize(&path)
        .map_err(|e| format!("Invalid path: {}", e))?;
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
pub fn export_briefing_html(
    meeting_id: String,
    markdown: String,
) -> Result<(), String> {
    let tmp_dir = std::env::temp_dir().join("dailyos-export");
    std::fs::create_dir_all(&tmp_dir)
        .map_err(|e| format!("Failed to create temp dir: {}", e))?;

    let safe_id = meeting_id
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect::<String>();
    let filename = format!("briefing-{}.html", if safe_id.is_empty() { "export" } else { &safe_id });
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

    std::fs::write(&path, &html)
        .map_err(|e| format!("Failed to write HTML: {}", e))?;

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
        if trimmed.starts_with("# ") {
            if in_list { html.push_str(&format!("</{}>\n", list_type)); in_list = false; }
            html.push_str(&format!("<h1>{}</h1>\n", &trimmed[2..]));
        } else if trimmed.starts_with("## ") {
            if in_list { html.push_str(&format!("</{}>\n", list_type)); in_list = false; }
            html.push_str(&format!("<h2>{}</h2>\n", &trimmed[3..]));
        } else if trimmed.starts_with("### ") {
            if in_list { html.push_str(&format!("</{}>\n", list_type)); in_list = false; }
            html.push_str(&format!("<h3>{}</h3>\n", &trimmed[4..]));
        }
        // Unordered list
        else if trimmed.starts_with("- ") {
            if !in_list { html.push_str("<ul>\n"); in_list = true; list_type = "ul"; }
            html.push_str(&format!("<li>{}</li>\n", &trimmed[2..]));
        }
        // Ordered list
        else if trimmed.len() > 2 && trimmed.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) && trimmed.contains(". ") {
            if let Some(pos) = trimmed.find(". ") {
                if !in_list { html.push_str("<ol>\n"); in_list = true; list_type = "ol"; }
                html.push_str(&format!("<li>{}</li>\n", &trimmed[pos + 2..]));
            }
        }
        // Horizontal rule
        else if trimmed == "---" || trimmed == "***" {
            if in_list { html.push_str(&format!("</{}>\n", list_type)); in_list = false; }
            html.push_str("<hr>\n");
        }
        // Paragraph
        else {
            if in_list { html.push_str(&format!("</{}>\n", list_type)); in_list = false; }
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

use crate::types::{ChatEntityListItem, meetings_to_json};

fn ensure_open_chat_session(
    db: &crate::db::ActionDb,
    entity_id: Option<&str>,
    entity_type: Option<&str>,
) -> Result<crate::db::DbChatSession, String> {
    if let Some(existing) = db
        .get_open_chat_session(entity_id, entity_type)
        .map_err(|e| e.to_string())?
    {
        return Ok(existing);
    }

    let now = chrono::Utc::now().to_rfc3339();
    let session_id = uuid::Uuid::new_v4().to_string();
    db.create_chat_session(&session_id, entity_id, entity_type, &now, &now)
        .map_err(|e| e.to_string())
}

fn append_chat_exchange(
    db: &crate::db::ActionDb,
    session_id: &str,
    user_content: &str,
    assistant_json: &serde_json::Value,
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    let first_idx = db
        .get_next_chat_turn_index(session_id)
        .map_err(|e| e.to_string())?;

    db.append_chat_turn(
        &uuid::Uuid::new_v4().to_string(),
        session_id,
        first_idx,
        "user",
        user_content,
        &now,
    )
    .map_err(|e| e.to_string())?;

    let assistant_content =
        serde_json::to_string(assistant_json).map_err(|e| format!("serialize failed: {}", e))?;
    db.append_chat_turn(
        &uuid::Uuid::new_v4().to_string(),
        session_id,
        first_idx + 1,
        "assistant",
        &assistant_content,
        &now,
    )
    .map_err(|e| e.to_string())?;

    db.bump_chat_session_stats(session_id, 2, Some(user_content))
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn chat_search_content(
    entity_id: String,
    query: String,
    top_k: Option<usize>,
    state: State<Arc<AppState>>,
) -> Result<Vec<crate::queries::search::ContentMatch>, String> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(Vec::new());
    }

    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let matches = crate::queries::search::search_entity_content(
        db,
        Some(state.embedding_model.as_ref()),
        &entity_id,
        query,
        top_k.unwrap_or(10).clamp(1, 50),
        0.7,
        0.3,
    )?;

    let session = ensure_open_chat_session(db, Some(&entity_id), None)?;
    let response = serde_json::json!({
        "entityId": entity_id,
        "query": query,
        "matches": matches,
    });
    append_chat_exchange(db, &session.id, query, &response)?;

    Ok(matches)
}

#[tauri::command]
pub fn chat_query_entity(
    entity_id: String,
    question: String,
    state: State<Arc<AppState>>,
) -> Result<serde_json::Value, String> {
    let question = question.trim();
    if question.is_empty() {
        return Err("question is required".to_string());
    }

    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

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
        } else if let Some(project) = db.get_project(&entity_id).map_err(|e| e.to_string())? {
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
        Some(state.embedding_model.as_ref()),
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
}

#[tauri::command]
pub fn chat_get_briefing(state: State<Arc<AppState>>) -> Result<serde_json::Value, String> {
    let dashboard = get_dashboard_data(state.clone());
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let session = ensure_open_chat_session(db, None, None)?;

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
    append_chat_exchange(db, &session.id, "get briefing", &response)?;
    Ok(response)
}

#[tauri::command]
pub fn chat_list_entities(
    entity_type: Option<String>,
    state: State<Arc<AppState>>,
) -> Result<Vec<ChatEntityListItem>, String> {
    let requested = entity_type
        .as_deref()
        .map(|s| s.to_lowercase())
        .unwrap_or_else(|| "all".to_string());

    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

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
}

// ── I74/I131: Entity Intelligence Enrichment via Claude Code ────────

/// Uses split-lock pattern (I173) — DB lock held only briefly during gather/write.
#[tauri::command]
pub async fn enrich_account(
    account_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::entity_intel::IntelligenceJson, String> {
    use crate::intel_queue::{
        gather_enrichment_input, run_enrichment, write_enrichment_results, IntelPriority,
        IntelRequest,
    };

    let request = IntelRequest {
        entity_id: account_id,
        entity_type: "account".to_string(),
        priority: IntelPriority::Manual,
        requested_at: std::time::Instant::now(),
    };

    // Phase 1: Brief DB lock — gather context
    let input = gather_enrichment_input(&state, &request)?;

    // Phase 2: No lock — PTY enrichment
    let ai_config = state
        .config
        .read()
        .ok()
        .and_then(|g| g.as_ref().map(|c| c.ai_models.clone()))
        .unwrap_or_default();
    let intel = run_enrichment(&input, &ai_config)?;

    // Phase 3: Brief DB lock — write results
    write_enrichment_results(&state, &input, &intel)?;

    Ok(intel)
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
    pub recent_email_signals: Vec<crate::db::DbEmailSignal>,
    /// Entity intelligence (ADR-0057) — synthesized assessment from enrichment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intelligence: Option<crate::entity_intel::IntelligenceJson>,
}

/// Get all projects with computed summary fields for the list page.
#[tauri::command]
pub fn get_projects_list(state: State<Arc<AppState>>) -> Result<Vec<ProjectListItem>, String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let projects = db.get_all_projects().map_err(|e| e.to_string())?;

    let items: Vec<ProjectListItem> = projects
        .into_iter()
        .map(|p| {
            let open_action_count = db.get_project_actions(&p.id).map(|a| a.len()).unwrap_or(0);
            let days_since_last_meeting = db.get_project_signals(&p.id).ok().and_then(|s| {
                s.last_meeting.as_ref().and_then(|lm| {
                    chrono::DateTime::parse_from_rfc3339(lm)
                        .ok()
                        .map(|dt| (chrono::Utc::now() - dt.with_timezone(&chrono::Utc)).num_days())
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

    let linked_people = db.get_people_for_entity(&project_id).unwrap_or_default();

    let signals = db.get_project_signals(&project_id).ok();

    // Get captures linked to project meetings
    let recent_captures = db
        .get_captures_for_project(&project_id, 90)
        .unwrap_or_default();
    let recent_email_signals = db
        .list_recent_email_signals_for_entity(&project_id, 12)
        .unwrap_or_default();

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
        recent_email_signals,
        intelligence,
    })
}

/// Create a new project.
#[tauri::command]
pub fn create_project(name: String, state: State<Arc<AppState>>) -> Result<String, String> {
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
        archived: false,
    };

    db.upsert_project(&project).map_err(|e| e.to_string())?;

    // Create workspace files + directory template (ADR-0059)
    let config = state.config.read().map_err(|_| "Lock poisoned")?;
    if let Some(ref config) = *config {
        let workspace = Path::new(&config.workspace_path);
        let project_dir = crate::projects::project_dir(workspace, validated_name);
        let _ = std::fs::create_dir_all(&project_dir);
        let _ = crate::util::bootstrap_entity_directory(&project_dir, validated_name, "project");
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
            let json_path =
                crate::projects::project_dir(workspace, &project.name).join("dashboard.json");
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
        let json_path =
            crate::projects::project_dir(workspace, &project.name).join("dashboard.json");

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
/// Uses split-lock pattern (I173) — DB lock held only briefly during gather/write.
#[tauri::command]
pub async fn enrich_project(
    project_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::entity_intel::IntelligenceJson, String> {
    use crate::intel_queue::{
        gather_enrichment_input, run_enrichment, write_enrichment_results, IntelPriority,
        IntelRequest,
    };

    let request = IntelRequest {
        entity_id: project_id,
        entity_type: "project".to_string(),
        priority: IntelPriority::Manual,
        requested_at: std::time::Instant::now(),
    };

    // Phase 1: Brief DB lock — gather context
    let input = gather_enrichment_input(&state, &request)?;

    // Phase 2: No lock — PTY enrichment
    let ai_config = state
        .config
        .read()
        .ok()
        .and_then(|g| g.as_ref().map(|c| c.ai_models.clone()))
        .unwrap_or_default();
    let intel = run_enrichment(&input, &ai_config)?;

    // Phase 3: Brief DB lock — write results
    write_enrichment_results(&state, &input, &intel)?;

    Ok(intel)
}

// ── I76: Database Backup & Rebuild ──────────────────────────────────

#[tauri::command]
pub async fn backup_database(state: tauri::State<'_, Arc<AppState>>) -> Result<String, String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    crate::db_backup::backup_database(db)
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

    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    crate::db_backup::rebuild_from_filesystem(
        std::path::Path::new(&workspace_path),
        db,
        &user_domains,
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
            account_team: Vec::new(),
            csm: None,
            champion: None,
        },
        company_overview: None,
        strategic_programs: Vec::new(),
        notes: None,
        custom_sections: Vec::new(),
        parent_id: account.parent_id.clone(),
    }
}

/// Get the latest hygiene scan report
#[tauri::command]
pub fn get_hygiene_report(
    state: State<Arc<AppState>>,
) -> Result<Option<crate::hygiene::HygieneReport>, String> {
    let guard = state
        .last_hygiene_report
        .lock()
        .map_err(|_| "Lock poisoned".to_string())?;
    Ok(guard.clone())
}

/// Get a prose narrative summarizing the last hygiene scan.
#[tauri::command]
pub fn get_hygiene_narrative(
    state: State<Arc<AppState>>,
) -> Result<Option<crate::hygiene::HygieneNarrativeView>, String> {
    let report = state
        .last_hygiene_report
        .lock()
        .map_err(|_| "Lock poisoned")?;
    Ok(report
        .as_ref()
        .and_then(crate::hygiene::build_hygiene_narrative))
}

/// Get the current Intelligence Hygiene status view model.
#[tauri::command]
pub fn get_intelligence_hygiene_status(
    state: State<Arc<AppState>>,
) -> Result<HygieneStatusView, String> {
    let report = state
        .last_hygiene_report
        .lock()
        .map_err(|_| "Lock poisoned".to_string())?
        .clone();
    Ok(build_intelligence_hygiene_status(&state, report.as_ref()))
}

/// Run a hygiene scan immediately and return the updated status.
#[tauri::command]
pub fn run_hygiene_scan_now(state: State<Arc<AppState>>) -> Result<HygieneStatusView, String> {
    if state
        .hygiene_scan_running
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
            Some(&state.hygiene_budget),
            Some(&state.intel_queue),
            false,
        );

        // Prune old audit trail files (I297)
        let pruned = crate::audit::prune_audit_files(workspace);
        if pruned > 0 {
            log::info!("run_hygiene_scan_now: pruned {} old audit files", pruned);
        }

        if let Ok(mut guard) = state.last_hygiene_report.lock() {
            *guard = Some(report.clone());
        }
        if let Ok(mut guard) = state.last_hygiene_scan_at.lock() {
            *guard = Some(report.scanned_at.clone());
        }
        if let Ok(mut guard) = state.next_hygiene_scan_at.lock() {
            *guard = Some(
                (chrono::Utc::now()
                    + chrono::Duration::seconds(crate::hygiene::scan_interval_secs(Some(&config)) as i64))
                .to_rfc3339(),
            );
        }

        Ok(report)
    })();

    state
        .hygiene_scan_running
        .store(false, std::sync::atomic::Ordering::Release);

    let report = scan_result?;
    Ok(build_intelligence_hygiene_status(&state, Some(&report)))
}

/// Detect potential duplicate people (I172).
#[tauri::command]
pub fn get_duplicate_people(
    state: State<Arc<AppState>>,
) -> Result<Vec<crate::hygiene::DuplicateCandidate>, String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    crate::hygiene::detect_duplicate_people(db)
}

/// Detect potential duplicate people for a specific person (I172).
#[tauri::command]
pub fn get_duplicate_people_for_person(
    person_id: String,
    state: State<Arc<AppState>>,
) -> Result<Vec<crate::hygiene::DuplicateCandidate>, String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let dupes = crate::hygiene::detect_duplicate_people(db)?;
    Ok(dupes
        .into_iter()
        .filter(|d| d.person1_id == person_id || d.person2_id == person_id)
        .collect())
}

// =============================================================================
// I176: Archive / Unarchive Entities
// =============================================================================

/// Archive or unarchive an account. Cascades to children when archiving.
#[tauri::command]
pub fn archive_account(
    id: String,
    archived: bool,
    state: State<Arc<AppState>>,
) -> Result<(), String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    db.archive_account(&id, archived)
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Archive or unarchive a project.
#[tauri::command]
pub fn archive_project(
    id: String,
    archived: bool,
    state: State<Arc<AppState>>,
) -> Result<(), String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    db.archive_project(&id, archived)
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Archive or unarchive a person.
#[tauri::command]
pub fn archive_person(
    id: String,
    archived: bool,
    state: State<Arc<AppState>>,
) -> Result<(), String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    db.archive_person(&id, archived)
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Get archived accounts.
#[tauri::command]
pub fn get_archived_accounts(
    state: State<Arc<AppState>>,
) -> Result<Vec<crate::db::DbAccount>, String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    db.get_archived_accounts().map_err(|e| e.to_string())
}

/// Get archived projects.
#[tauri::command]
pub fn get_archived_projects(
    state: State<Arc<AppState>>,
) -> Result<Vec<crate::db::DbProject>, String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    db.get_archived_projects().map_err(|e| e.to_string())
}

/// Get archived people with signals.
#[tauri::command]
pub fn get_archived_people(
    state: State<Arc<AppState>>,
) -> Result<Vec<crate::db::PersonListItem>, String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    db.get_archived_people_with_signals()
        .map_err(|e| e.to_string())
}

// =============================================================================
// I171: Multi-Domain Config
// =============================================================================

/// Set multiple user domains for multi-org meeting classification.
/// After saving, reclassifies existing people and meetings to reflect the new domains.
#[tauri::command]
pub fn set_user_domains(domains: String, state: State<Arc<AppState>>) -> Result<Config, String> {
    let parsed: Vec<String> = domains
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();

    let config = crate::state::create_or_update_config(&state, |config| {
        // Update legacy single-domain field for backward compat
        config.user_domain = parsed.first().cloned();
        config.user_domains = if parsed.is_empty() {
            None
        } else {
            Some(parsed.clone())
        };
    })?;

    // Reclassify existing people and meetings for the new domains (I184)
    if !parsed.is_empty() {
        if let Ok(db_guard) = state.db.lock() {
            if let Some(db) = db_guard.as_ref() {
                match db.reclassify_people_for_domains(&parsed) {
                    Ok(n) if n > 0 => {
                        log::info!("Reclassified {} people after domain change", n);
                        // Now fix meeting types based on updated relationships
                        match db.reclassify_meeting_types_from_attendees() {
                            Ok(m) if m > 0 => {
                                log::info!("Reclassified {} meetings after domain change", m);
                            }
                            Ok(_) => {}
                            Err(e) => log::warn!("Meeting reclassification failed: {}", e),
                        }
                    }
                    Ok(_) => {}
                    Err(e) => log::warn!("People reclassification failed: {}", e),
                }
            }
        }
    }

    Ok(config)
}

// =============================================================================
// I162: Bulk Entity Creation
// =============================================================================

/// Bulk-create accounts from a list of names. Returns created account IDs.
#[tauri::command]
pub fn bulk_create_accounts(
    names: Vec<String>,
    state: State<Arc<AppState>>,
) -> Result<Vec<String>, String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let config = state.config.read().map_err(|_| "Lock poisoned")?;
    let workspace_path = config
        .as_ref()
        .ok_or("Config not loaded")?
        .workspace_path
        .clone();
    let workspace = Path::new(&workspace_path);

    let mut created_ids = Vec::with_capacity(names.len());

    for raw_name in &names {
        let name = crate::util::validate_entity_name(raw_name)?;
        let id = crate::util::slugify(name);

        // Skip duplicates
        if let Ok(Some(_)) = db.get_account(&id) {
            continue;
        }

        let now = chrono::Utc::now().to_rfc3339();
        let account = crate::db::DbAccount {
            id: id.clone(),
            name: name.to_string(),
            lifecycle: None,
            arr: None,
            health: None,
            contract_start: None,
            contract_end: None,
            nps: None,
            tracker_path: Some(format!("Accounts/{}", name)),
            parent_id: None,
            is_internal: false,
            updated_at: now,
            archived: false,
        };

        db.upsert_account(&account).map_err(|e| e.to_string())?;

        // Create workspace files + directory template (ADR-0059)
        let account_dir = crate::accounts::resolve_account_dir(workspace, &account);
        let _ = std::fs::create_dir_all(&account_dir);
        let _ = crate::util::bootstrap_entity_directory(&account_dir, name, "account");
        let _ = crate::accounts::write_account_json(workspace, &account, None, db);
        let _ = crate::accounts::write_account_markdown(workspace, &account, None, db);

        created_ids.push(id);
    }

    Ok(created_ids)
}

/// Bulk-create projects from a list of names. Returns created project IDs.
#[tauri::command]
pub fn bulk_create_projects(
    names: Vec<String>,
    state: State<Arc<AppState>>,
) -> Result<Vec<String>, String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let config = state.config.read().map_err(|_| "Lock poisoned")?;
    let workspace_path = config
        .as_ref()
        .ok_or("Config not loaded")?
        .workspace_path
        .clone();
    let workspace = Path::new(&workspace_path);

    let mut created_ids = Vec::with_capacity(names.len());

    for raw_name in &names {
        let name = crate::util::validate_entity_name(raw_name)?;
        let id = crate::util::slugify(name);

        // Skip duplicates
        if let Ok(Some(_)) = db.get_project(&id) {
            continue;
        }

        let now = chrono::Utc::now().to_rfc3339();
        let project = crate::db::DbProject {
            id: id.clone(),
            name: name.to_string(),
            status: "active".to_string(),
            milestone: None,
            owner: None,
            target_date: None,
            tracker_path: Some(format!("Projects/{}", name)),
            updated_at: now,
            archived: false,
        };

        db.upsert_project(&project).map_err(|e| e.to_string())?;

        // Create workspace files + directory template (ADR-0059)
        let project_dir = crate::projects::project_dir(workspace, name);
        let _ = std::fs::create_dir_all(&project_dir);
        let _ = crate::util::bootstrap_entity_directory(&project_dir, name, "project");
        let _ = crate::projects::write_project_json(workspace, &project, None, db);
        let _ = crate::projects::write_project_markdown(workspace, &project, None, db);

        created_ids.push(id);
    }

    Ok(created_ids)
}

// =============================================================================
// I143: Account Events
// =============================================================================

/// Record an account lifecycle event (expansion, contraction, churn, etc.)
#[tauri::command]
pub fn record_account_event(
    account_id: String,
    event_type: String,
    event_date: String,
    arr_impact: Option<f64>,
    notes: Option<String>,
    state: State<Arc<AppState>>,
) -> Result<i64, String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    db.record_account_event(
        &account_id,
        &event_type,
        &event_date,
        arr_impact,
        notes.as_deref(),
    )
    .map_err(|e| e.to_string())
}

/// Get account events for a given account.
#[tauri::command]
pub fn get_account_events(
    account_id: String,
    state: State<Arc<AppState>>,
) -> Result<Vec<crate::db::DbAccountEvent>, String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    db.get_account_events(&account_id)
        .map_err(|e| e.to_string())
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
    db.update_meeting_user_layer(meeting_id, agenda_json.as_deref(), merged_notes.as_deref())
        .map_err(|e| e.to_string())?;

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
                agenda_items.extend(
                    proposed
                        .iter()
                        .map(|item| item.topic.trim().to_string())
                        .filter(|item| !item.is_empty()),
                );
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
pub fn apply_meeting_prep_prefill(
    meeting_id: String,
    agenda_items: Vec<String>,
    notes_append: String,
    state: State<Arc<AppState>>,
) -> Result<ApplyPrepPrefillResult, String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let result = apply_meeting_prep_prefill_inner(db, &meeting_id, &agenda_items, &notes_append)?;

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
pub fn generate_meeting_agenda_message_draft(
    meeting_id: String,
    context_hint: Option<String>,
    state: State<Arc<AppState>>,
) -> Result<AgendaDraftResult, String> {
    let config = state
        .config
        .read()
        .map_err(|_| "Lock poisoned")?
        .clone()
        .ok_or("No configuration loaded")?;
    let workspace = Path::new(&config.workspace_path);
    let today_dir = workspace.join("_today");

    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
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
}

/// Update user-authored agenda items on a meeting prep file.
#[tauri::command]
pub fn update_meeting_user_agenda(
    meeting_id: String,
    agenda: Vec<String>,
    dismissed_topics: Option<Vec<String>>,
    hidden_attendees: Option<Vec<String>>,
    state: State<Arc<AppState>>,
) -> Result<(), String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let meeting = db
        .get_meeting_intelligence_row(&meeting_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Meeting not found: {}", meeting_id))?;

    if is_meeting_user_layer_read_only(&meeting) {
        return Err("Meeting user fields are read-only after freeze/past state".to_string());
    }

    // Merge with existing layer to preserve fields not being updated
    let existing = parse_user_agenda_layer(meeting.user_agenda_json.as_deref());
    let layer = UserAgendaLayer {
        items: agenda.clone(),
        dismissed_topics: dismissed_topics.unwrap_or(existing.dismissed_topics),
        hidden_attendees: hidden_attendees.unwrap_or(existing.hidden_attendees),
    };

    let agenda_json = if layer.items.is_empty() && layer.dismissed_topics.is_empty() && layer.hidden_attendees.is_empty() {
        None
    } else {
        Some(serde_json::to_string(&layer).map_err(|e| format!("Serialize error: {}", e))?)
    };
    db.update_meeting_user_layer(
        &meeting_id,
        agenda_json.as_deref(),
        meeting.user_notes.as_deref(),
    )
    .map_err(|e| e.to_string())?;

    // Optional mirror write to active prep file for same-session coherence.
    if let Ok(prep_path) = resolve_prep_path(&meeting_id, &state) {
        if let Ok(content) = std::fs::read_to_string(&prep_path) {
            if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&content) {
                if agenda.is_empty() {
                    json.as_object_mut().map(|o| o.remove("userAgenda"));
                } else {
                    json["userAgenda"] = serde_json::json!(agenda);
                }
                if let Ok(updated) = serde_json::to_string_pretty(&json) {
                    let _ = std::fs::write(&prep_path, updated);
                }
            }
        }
    }

    Ok(())
}

/// Update user-authored notes on a meeting prep file.
#[tauri::command]
pub fn update_meeting_user_notes(
    meeting_id: String,
    notes: String,
    state: State<Arc<AppState>>,
) -> Result<(), String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let meeting = db
        .get_meeting_intelligence_row(&meeting_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Meeting not found: {}", meeting_id))?;

    if is_meeting_user_layer_read_only(&meeting) {
        return Err("Meeting user fields are read-only after freeze/past state".to_string());
    }

    let notes_opt = if notes.trim().is_empty() {
        None
    } else {
        Some(notes.as_str())
    };
    db.update_meeting_user_layer(&meeting_id, meeting.user_agenda_json.as_deref(), notes_opt)
        .map_err(|e| e.to_string())?;

    // Optional mirror write to active prep file for same-session coherence.
    if let Ok(prep_path) = resolve_prep_path(&meeting_id, &state) {
        if let Ok(content) = std::fs::read_to_string(&prep_path) {
            if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&content) {
                if notes.is_empty() {
                    json.as_object_mut().map(|o| o.remove("userNotes"));
                } else {
                    json["userNotes"] = serde_json::json!(notes);
                }
                if let Ok(updated) = serde_json::to_string_pretty(&json) {
                    let _ = std::fs::write(&prep_path, updated);
                }
            }
        }
    }

    Ok(())
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
        let db = ActionDb::open_at(db_path).expect("open db");

        let meeting = DbMeeting {
            id: "mtg-1".to_string(),
            title: "Test Meeting".to_string(),
            meeting_type: "customer".to_string(),
            start_time: Utc::now().to_rfc3339(),
            end_time: None,
            account_id: None,
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
        let db = ActionDb::open_at(db_path).expect("open db");

        let meeting = DbMeeting {
            id: "mtg-prefill".to_string(),
            title: "Prefill Test".to_string(),
            meeting_type: "customer".to_string(),
            start_time: (Utc::now() + chrono::Duration::hours(2)).to_rfc3339(),
            end_time: Some((Utc::now() + chrono::Duration::hours(3)).to_rfc3339()),
            account_id: None,
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
        };
        db.upsert_meeting(&meeting).expect("upsert meeting");

        let first = apply_meeting_prep_prefill_inner(
            &db,
            "mtg-prefill",
            &["Confirm blockers".to_string(), "Agree owners".to_string()],
            "Bring latest renewal risk updates.",
        )
        .expect("first prefill");
        assert_eq!(first.added_agenda_items, 2);
        assert!(first.notes_appended);

        let second = apply_meeting_prep_prefill_inner(
            &db,
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
        let db = ActionDb::open_at(db_path).expect("open db");

        let past = DbMeeting {
            id: "mtg-past".to_string(),
            title: "Past Meeting".to_string(),
            meeting_type: "customer".to_string(),
            start_time: (Utc::now() - chrono::Duration::hours(4)).to_rfc3339(),
            end_time: Some((Utc::now() - chrono::Duration::hours(3)).to_rfc3339()),
            account_id: None,
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
        };
        db.upsert_meeting(&past).expect("upsert past meeting");

        let err = apply_meeting_prep_prefill_inner(&db, "mtg-past", &["Item".to_string()], "notes")
            .expect_err("past meeting should be read-only");
        assert!(err.contains("read-only"));

        let frozen = DbMeeting {
            id: "mtg-frozen".to_string(),
            title: "Frozen Meeting".to_string(),
            meeting_type: "customer".to_string(),
            start_time: (Utc::now() + chrono::Duration::hours(2)).to_rfc3339(),
            end_time: Some((Utc::now() + chrono::Duration::hours(3)).to_rfc3339()),
            account_id: None,
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
        };
        db.upsert_meeting(&frozen).expect("upsert frozen meeting");

        let frozen_err =
            apply_meeting_prep_prefill_inner(&db, "mtg-frozen", &["Item".to_string()], "notes")
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
            account_id: None,
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
pub fn backfill_historical_meetings(
    state: State<Arc<AppState>>,
) -> Result<(usize, usize, Vec<String>), String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let config_guard = state.config.read().map_err(|_| "Config lock poisoned")?;
    let config = config_guard.as_ref().ok_or("Config not initialized")?;

    crate::backfill_meetings::backfill_historical_meetings(db, config)
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
    let app_state = state.inner().clone();

    let task = tauri::async_runtime::spawn_blocking(move || {
        // Phase 1: Brief DB lock — gather context + build prompt
        let input = {
            let db_guard = app_state
                .db
                .lock()
                .map_err(|_| "Lock poisoned".to_string())?;
            let db = db_guard
                .as_ref()
                .ok_or_else(|| "Database not initialized".to_string())?;

            let config_guard = app_state
                .config
                .read()
                .map_err(|_| "Config lock poisoned".to_string())?;
            let config = config_guard
                .as_ref()
                .ok_or_else(|| "Config not initialized".to_string())?;

            let workspace = std::path::Path::new(&config.workspace_path);
            crate::risk_briefing::gather_risk_input(
                workspace,
                db,
                &account_id,
                config.user_name.clone(),
                config.ai_models.clone(),
            )?
        }; // locks dropped here

        // Phase 2: No lock — PTY enrichment (long-running)
        crate::risk_briefing::run_risk_enrichment(&input)
    });

    match task.await {
        Ok(result) => result,
        Err(e) => Err(format!("Risk briefing task panicked: {}", e)),
    }
}

/// Read a cached risk briefing for an account (fast, no AI).
#[tauri::command]
pub fn get_risk_briefing(
    state: State<Arc<AppState>>,
    account_id: String,
) -> Result<crate::types::RiskBriefing, String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let config_guard = state.config.read().map_err(|_| "Config lock poisoned")?;
    let config = config_guard.as_ref().ok_or("Config not initialized")?;

    let account = db
        .get_account(&account_id)
        .map_err(|e| format!("DB error: {}", e))?
        .ok_or_else(|| format!("Account not found: {}", account_id))?;

    let workspace = std::path::Path::new(&config.workspace_path);
    let account_dir = crate::accounts::resolve_account_dir(workspace, &account);
    crate::risk_briefing::read_risk_briefing(&account_dir)
}

/// Save an edited risk briefing back to disk (user corrections).
#[tauri::command]
pub fn save_risk_briefing(
    state: State<Arc<AppState>>,
    account_id: String,
    briefing: crate::types::RiskBriefing,
) -> Result<(), String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let config_guard = state.config.read().map_err(|_| "Config lock poisoned")?;
    let config = config_guard.as_ref().ok_or("Config not initialized")?;

    let account = db
        .get_account(&account_id)
        .map_err(|e| format!("DB error: {}", e))?
        .ok_or_else(|| format!("Account not found: {}", account_id))?;

    let workspace = std::path::Path::new(&config.workspace_path);
    let account_dir = crate::accounts::resolve_account_dir(workspace, &account);
    crate::risk_briefing::write_risk_briefing(&account_dir, &briefing)
}

// =============================================================================
// MCP: Claude Desktop Configuration (ADR-0075)
// =============================================================================

/// Result of Claude Desktop MCP configuration.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeDesktopConfigResult {
    pub success: bool,
    pub message: String,
    pub config_path: Option<String>,
    pub binary_path: Option<String>,
}

/// Configure Claude Desktop to use the DailyOS MCP server.
///
/// Reads (or creates) `~/Library/Application Support/Claude/claude_desktop_config.json`
/// and adds/updates the `mcpServers.dailyos` entry pointing to the bundled binary.
#[tauri::command]
pub fn configure_claude_desktop() -> ClaudeDesktopConfigResult {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => {
            return ClaudeDesktopConfigResult {
                success: false,
                message: "Could not find home directory".to_string(),
                config_path: None,
                binary_path: None,
            }
        }
    };

    // Resolve MCP binary path: check common locations
    let binary_name = "dailyos-mcp";
    let binary_path = resolve_mcp_binary_path(&home, binary_name);

    let binary_path_str = match &binary_path {
        Some(p) => p.to_string_lossy().to_string(),
        None => {
            return ClaudeDesktopConfigResult {
                success: false,
                message: format!(
                    "The {binary_name} component is missing from this installation. \
                     Please reinstall DailyOS from the latest release at https://daily-os.com"
                ),
                config_path: None,
                binary_path: None,
            }
        }
    };

    // Claude Desktop config path
    let config_path = home
        .join("Library")
        .join("Application Support")
        .join("Claude")
        .join("claude_desktop_config.json");

    // Read existing config or start fresh
    let mut config: serde_json::Value = if config_path.exists() {
        match std::fs::read_to_string(&config_path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_else(|_| {
                serde_json::json!({})
            }),
            Err(_) => serde_json::json!({}),
        }
    } else {
        serde_json::json!({})
    };

    // Ensure mcpServers object exists
    if config.get("mcpServers").is_none() {
        config["mcpServers"] = serde_json::json!({});
    }

    // Set the dailyos entry
    config["mcpServers"]["dailyos"] = serde_json::json!({
        "command": binary_path_str,
        "args": [],
        "env": {}
    });

    // Ensure parent directory exists
    if let Some(parent) = config_path.parent() {
        if !parent.exists() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return ClaudeDesktopConfigResult {
                    success: false,
                    message: format!("Failed to create config directory: {e}"),
                    config_path: None,
                    binary_path: Some(binary_path_str),
                };
            }
        }
    }

    // Write config
    let formatted = match serde_json::to_string_pretty(&config) {
        Ok(s) => s,
        Err(e) => {
            return ClaudeDesktopConfigResult {
                success: false,
                message: format!("Failed to serialize config: {e}"),
                config_path: Some(config_path.to_string_lossy().to_string()),
                binary_path: Some(binary_path_str),
            }
        }
    };

    match std::fs::write(&config_path, formatted) {
        Ok(()) => ClaudeDesktopConfigResult {
            success: true,
            message: "Claude Desktop configured. Restart Claude Desktop to connect.".to_string(),
            config_path: Some(config_path.to_string_lossy().to_string()),
            binary_path: Some(binary_path_str),
        },
        Err(e) => ClaudeDesktopConfigResult {
            success: false,
            message: format!("Failed to write config: {e}"),
            config_path: Some(config_path.to_string_lossy().to_string()),
            binary_path: Some(binary_path_str),
        },
    }
}

/// Resolve the MCP binary path by checking common locations.
fn resolve_mcp_binary_path(
    home: &std::path::Path,
    binary_name: &str,
) -> Option<std::path::PathBuf> {
    // 1. Check if in PATH (cargo install location)
    let cargo_bin = home.join(".cargo").join("bin").join(binary_name);
    if cargo_bin.exists() {
        return Some(cargo_bin);
    }

    // 2. Check alongside the running executable (app bundle)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let sibling = exe_dir.join(binary_name);
            if sibling.exists() {
                return Some(sibling);
            }
            // macOS .app bundle: Contents/MacOS/
            let macos_sibling = exe_dir.join(binary_name);
            if macos_sibling.exists() {
                return Some(macos_sibling);
            }
        }
    }

    // 3. Check dev build location (target/debug)
    let cwd = std::env::current_dir().ok()?;
    let dev_paths = [
        cwd.join("target/debug").join(binary_name),
        cwd.join("src-tauri/target/debug").join(binary_name),
        cwd.join("target/release").join(binary_name),
        cwd.join("src-tauri/target/release").join(binary_name),
    ];
    for path in &dev_paths {
        if path.exists() {
            return Some(path.clone());
        }
    }

    None
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
pub fn update_intelligence_field(
    entity_id: String,
    entity_type: String,
    field_path: String,
    value: String,
    state: State<Arc<AppState>>,
) -> Result<(), String> {
    let config = state.config.read().map_err(|_| "Lock poisoned")?;
    let config = config.as_ref().ok_or("No configuration loaded")?;
    let workspace = Path::new(&config.workspace_path);

    // Look up entity to resolve directory
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let account = if entity_type == "account" {
        db.get_account(&entity_id).map_err(|e| e.to_string())?
    } else {
        None
    };

    let entity_name = match entity_type.as_str() {
        "account" => account.as_ref().map(|a| a.name.clone()),
        "project" => db
            .get_project(&entity_id)
            .map_err(|e| e.to_string())?
            .map(|p| p.name),
        "person" => db
            .get_person(&entity_id)
            .map_err(|e| e.to_string())?
            .map(|p| p.name),
        _ => return Err(format!("Unsupported entity type: {}", entity_type)),
    }
    .ok_or_else(|| format!("{} '{}' not found", entity_type, entity_id))?;

    let dir = crate::entity_intel::resolve_entity_dir(
        workspace,
        &entity_type,
        &entity_name,
        account.as_ref(),
    )?;

    let intel = crate::entity_intel::apply_intelligence_field_update(&dir, &field_path, &value)?;

    // Update SQLite cache
    let _ = db.upsert_entity_intelligence(&intel);

    Ok(())
}

/// Bulk-replace the stakeholder list in an entity's intelligence.json.
///
/// Used for add/remove stakeholder operations. Replaces the entire
/// stakeholderInsights array and marks it as user-edited.
#[tauri::command]
pub fn update_stakeholders(
    entity_id: String,
    entity_type: String,
    stakeholders_json: String,
    state: State<Arc<AppState>>,
) -> Result<(), String> {
    let stakeholders: Vec<crate::entity_intel::StakeholderInsight> =
        serde_json::from_str(&stakeholders_json)
            .map_err(|e| format!("Invalid stakeholders JSON: {}", e))?;

    let config = state.config.read().map_err(|_| "Lock poisoned")?;
    let config = config.as_ref().ok_or("No configuration loaded")?;
    let workspace = Path::new(&config.workspace_path);

    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let account = if entity_type == "account" {
        db.get_account(&entity_id).map_err(|e| e.to_string())?
    } else {
        None
    };

    let entity_name = match entity_type.as_str() {
        "account" => account.as_ref().map(|a| a.name.clone()),
        "project" => db
            .get_project(&entity_id)
            .map_err(|e| e.to_string())?
            .map(|p| p.name),
        "person" => db
            .get_person(&entity_id)
            .map_err(|e| e.to_string())?
            .map(|p| p.name),
        _ => return Err(format!("Unsupported entity type: {}", entity_type)),
    }
    .ok_or_else(|| format!("{} '{}' not found", entity_type, entity_id))?;

    let dir = crate::entity_intel::resolve_entity_dir(
        workspace,
        &entity_type,
        &entity_name,
        account.as_ref(),
    )?;

    let intel = crate::entity_intel::apply_stakeholders_update(&dir, stakeholders)?;

    // Update SQLite cache
    let _ = db.upsert_entity_intelligence(&intel);

    Ok(())
}

/// Create a person entity from a stakeholder name (no email required).
///
/// Used when a stakeholder card references someone who doesn't yet exist as
/// a person entity. Creates with empty email, links to the parent entity.
#[tauri::command]
pub fn create_person_from_stakeholder(
    entity_id: String,
    entity_type: String,
    name: String,
    role: Option<String>,
    state: State<Arc<AppState>>,
) -> Result<String, String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("Name is required".to_string());
    }

    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    // Generate ID from name (no email available)
    let id = crate::util::slugify(&name);
    let now = chrono::Utc::now().to_rfc3339();

    let person = crate::db::DbPerson {
        id: id.clone(),
        email: String::new(), // Empty email — no address known
        name: name.clone(),
        organization: None,
        role,
        relationship: "external".to_string(),
        notes: None,
        tracker_path: None,
        last_seen: None,
        first_seen: Some(now.clone()),
        meeting_count: 0,
        updated_at: now,
        archived: false,
    };

    db.upsert_person(&person).map_err(|e| e.to_string())?;

    // Link to the parent entity
    db.link_person_to_entity(&id, &entity_id, &entity_type)
        .map_err(|e| e.to_string())?;

    // Write person files to workspace
    let config = state.config.read().map_err(|_| "Lock poisoned")?;
    if let Some(ref config) = *config {
        let workspace = Path::new(&config.workspace_path);
        let _ = crate::people::write_person_json(workspace, &person, db);
        let _ = crate::people::write_person_markdown(workspace, &person, db);
    }

    log::info!(
        "Created person '{}' (id={}) from stakeholder, linked to {} '{}'",
        name,
        id,
        entity_type,
        entity_id,
    );

    Ok(id)
}
