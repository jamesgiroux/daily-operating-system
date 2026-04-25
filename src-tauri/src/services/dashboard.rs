// Dashboard service — extracted from commands.rs
// Business logic for dashboard data loading (daily briefing, live fallback).

use std::collections::{HashMap, HashSet};
use std::path::Path;

use chrono::{Datelike, Utc};

use crate::json_loader::DataFreshness;
use crate::parser::count_inbox;
use crate::state::AppState;
use crate::types::{
    Action, CalendarEvent, DailyFocus, DashboardBriefingCallout, DashboardData,
    DashboardLifecycleUpdate, DayOverview, DayStats, EmailSyncStage, EmailSyncState,
    EmailSyncStatus, GoogleAuthStatus, Meeting, MeetingPrep, MeetingType, OverlayStatus, Priority,
    WeekOverview,
};

/// Extract a dashboard-facing `MeetingPrep` from the `prep_context_json` DB column.
///
/// Handles two storage formats:
/// 1. `FullMeetingPrep` JSON (camelCase, from pipeline reconcile)
/// 2. AI-schema JSON with `ai_intelligence` key (from meeting_prep_queue)
fn extract_dashboard_prep(prep_json: &str) -> Option<MeetingPrep> {
    // Format 1: direct FullMeetingPrep deserialization
    if let Ok(full) = serde_json::from_str::<crate::types::FullMeetingPrep>(prep_json) {
        let wins = full.recent_wins;
        let actions = dedupe_prep_items_against(full.talking_points, wins.as_ref());
        let prep = MeetingPrep {
            context: full.meeting_context.or(full.intelligence_summary),
            risks: full.risks,
            wins,
            actions,
            stakeholders: full.attendees,
            questions: full.questions,
            ..Default::default()
        };
        let has_content = prep.context.is_some()
            || prep.risks.as_ref().is_some_and(|v| !v.is_empty())
            || prep.wins.as_ref().is_some_and(|v| !v.is_empty())
            || prep.actions.as_ref().is_some_and(|v| !v.is_empty())
            || prep.stakeholders.as_ref().is_some_and(|v| !v.is_empty())
            || prep.questions.as_ref().is_some_and(|v| !v.is_empty());
        if has_content {
            return Some(prep);
        }
    }

    // Format 2: AI-schema with top-level ai_intelligence key
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(prep_json) {
        if let Some(ai) = val.get("ai_intelligence") {
            let context = ai
                .get("narrative")
                .or_else(|| ai.get("meetingContext"))
                .or_else(|| ai.get("meeting_context"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let risks = ai.get("risks").and_then(|v| v.as_array()).map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>()
            });
            let actions = ai
                .get("talkingPoints")
                .or_else(|| ai.get("talking_points"))
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect::<Vec<_>>()
                });
            let has_content = context.is_some()
                || risks.as_ref().is_some_and(|v| !v.is_empty())
                || actions.as_ref().is_some_and(|v| !v.is_empty());
            if has_content {
                return Some(MeetingPrep {
                    context,
                    risks,
                    actions,
                    ..Default::default()
                });
            }
        }
    }

    None
}

fn dedupe_prep_items_against(
    items: Option<Vec<String>>,
    existing: Option<&Vec<String>>,
) -> Option<Vec<String>> {
    let mut seen: HashSet<String> = existing
        .into_iter()
        .flat_map(|items| items.iter())
        .map(|item| normalize_prep_card_item(item))
        .collect();

    let filtered: Vec<String> = items
        .unwrap_or_default()
        .into_iter()
        .filter(|item| {
            let key = normalize_prep_card_item(item);
            !key.is_empty() && seen.insert(key)
        })
        .collect();

    if filtered.is_empty() {
        None
    } else {
        Some(filtered)
    }
}

fn normalize_prep_card_item(item: &str) -> String {
    let mut value = item.trim().to_lowercase();
    for suffix in [" — high", " — medium", " — low", " - high", " - medium", " - low"] {
        if value.ends_with(suffix) {
            value.truncate(value.len() - suffix.len());
            break;
        }
    }
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

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

/// p95 latency budgets for dashboard.
const DASHBOARD_LATENCY_BUDGET_MS: u128 = 300;
const READ_CMD_LATENCY_BUDGET_MS: u128 = 100;

/// Result type for week data
#[derive(Debug, serde::Serialize)]
#[allow(clippy::large_enum_variant)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum WeekResult {
    Success { data: WeekOverview },
    NotFound { message: String },
    Error { message: String },
}

fn build_live_event_domain_map(events: &[CalendarEvent]) -> HashMap<String, HashSet<String>> {
    let mut map = HashMap::new();
    for event in events {
        map.insert(
            event.id.clone(),
            crate::services::entities::attendee_domains(&event.attendees),
        );
    }
    map
}

fn normalize_match_key(value: &str) -> String {
    crate::services::entities::normalize_match_key(value)
}

fn include_dashboard_meeting(intelligence_state: Option<&str>) -> bool {
    !matches!(intelligence_state, Some("archived"))
}

fn load_dashboard_lifecycle_updates(
    db: &crate::db::ActionDb,
    limit: usize,
) -> Vec<DashboardLifecycleUpdate> {
    db.get_recent_lifecycle_changes(limit)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|change| {
            let account = db.get_account(&change.account_id).ok().flatten()?;
            Some(DashboardLifecycleUpdate {
                change_id: change.id,
                account_id: change.account_id,
                account_name: account.name,
                previous_lifecycle: change.previous_lifecycle,
                new_lifecycle: change.new_lifecycle,
                renewal_stage: change.new_stage,
                source: change.source,
                confidence: change.confidence,
                evidence: change.evidence,
                health_score_before: change.health_score_before,
                health_score_after: change.health_score_after,
                action_state: change.user_response,
                created_at: change.created_at,
            })
        })
        .take(limit)
        .collect()
}

/// Load recent, undismissed briefing callouts (last 7 days) for the dashboard.
fn load_briefing_callouts(db: &crate::db::ActionDb, limit: usize) -> Vec<DashboardBriefingCallout> {
    let sql =
        "SELECT id, entity_id, entity_type, entity_name, severity, headline, detail, created_at
               FROM briefing_callouts
               WHERE dismissed_at IS NULL
                 AND created_at >= datetime('now', '-7 days')
               ORDER BY
                 CASE severity WHEN 'critical' THEN 0 WHEN 'warning' THEN 1 ELSE 2 END,
                 created_at DESC
               LIMIT ?1";
    let conn = db.conn_ref();
    let mut stmt = match conn.prepare(sql) {
        Ok(s) => s,
        Err(e) => {
            log::warn!("load_briefing_callouts: {}", e);
            return Vec::new();
        }
    };
    let rows = match stmt.query_map(rusqlite::params![limit as i64], |row| {
        Ok(DashboardBriefingCallout {
            id: row.get(0)?,
            entity_id: row.get(1)?,
            entity_type: row.get(2)?,
            entity_name: row.get(3)?,
            severity: row.get(4)?,
            headline: row.get(5)?,
            detail: row.get(6)?,
            callout_type: row.get::<_, String>(4)?.clone(), // severity doubles as callout_type bucket
            created_at: row.get(7)?,
        })
    }) {
        Ok(r) => r,
        Err(e) => {
            log::warn!("load_briefing_callouts query: {}", e);
            return Vec::new();
        }
    };
    rows.filter_map(|r| r.ok()).collect()
}

/// Build dashboard data from live SQLite when schedule.json is missing.
///
/// Returns `None` if no meetings exist for today or DB is unavailable.
pub async fn build_live_dashboard_data(state: &AppState) -> Option<DashboardData> {
    // Gather all data under a single read lock.
    struct LiveSnapshot {
        meetings: Vec<crate::db::DbMeeting>,
        actions: Vec<crate::db::DbAction>,
        focus_candidates: Vec<crate::db::DbAction>,
        entity_map: HashMap<String, Vec<crate::types::LinkedEntity>>,
        intelligence_qualities: HashMap<String, crate::types::IntelligenceQuality>,
        lifecycle_updates: Vec<DashboardLifecycleUpdate>,
        briefing_callouts: Vec<DashboardBriefingCallout>,
        aging_action_count: Option<i64>,
    }

    let engine = std::sync::Arc::clone(&state.signals.engine);
    let _ = state
        .db_write(move |db| {
            crate::services::accounts::refresh_lifecycle_states_for_dashboard(db, &engine)
        })
        .await;

    let tz_for_live: chrono_tz::Tz = state
        .config
        .read()
        .as_ref()
        .map(|c| c.schedules.today.timezone.clone())
        .and_then(|t| t.parse().ok())
        .unwrap_or(chrono_tz::America::New_York);
    let tf_live = crate::helpers::today_meeting_filter(&tz_for_live);
    let today = tf_live.date;
    let tomorrow = tf_live.next_date;

    let snap = match state
        .db_read(move |db| {
            let conn = db.conn_ref();

            // 1. Query today's meetings from meetings + LEFT JOINs for prep/transcript columns
            let mut stmt = conn
                .prepare(
                    "SELECT m.id, m.title, m.meeting_type, m.start_time, m.end_time, m.attendees,
                        m.notes_path, mt.summary, m.created_at, m.calendar_event_id, m.description,
                        mp.prep_context_json, mt.intelligence_state
                 FROM meetings m
                 LEFT JOIN meeting_prep mp ON mp.meeting_id = m.id
                 LEFT JOIN meeting_transcripts mt ON mt.meeting_id = m.id
                 WHERE m.start_time >= ?1 AND m.start_time < ?2
                 ORDER BY m.start_time ASC",
                )
                .map_err(|e| e.to_string())?;
            let meeting_rows = stmt
                .query_map(rusqlite::params![today, tomorrow], |row| {
                    Ok(crate::db::DbMeeting {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        meeting_type: row.get(2)?,
                        start_time: row.get(3)?,
                        end_time: row.get(4)?,
                        attendees: row.get(5)?,
                        notes_path: row.get(6)?,
                        summary: row.get(7)?,
                        created_at: row.get(8)?,
                        calendar_event_id: row.get(9)?,
                        description: row.get(10)?,
                        prep_context_json: row.get(11)?,
                        user_agenda_json: None,
                        user_notes: None,
                        prep_frozen_json: None,
                        prep_frozen_at: None,
                        prep_snapshot_path: None,
                        prep_snapshot_hash: None,
                        transcript_path: None,
                        transcript_processed_at: None,
                        intelligence_state: row.get(12)?,
                        intelligence_quality: None,
                        last_enriched_at: None,
                        signal_count: None,
                        has_new_signals: None,
                        last_viewed_at: None,
                    })
                })
                .map_err(|e| e.to_string())?;
            let meetings: Vec<crate::db::DbMeeting> = meeting_rows
                .filter_map(|r| r.ok())
                .filter(|m| include_dashboard_meeting(m.intelligence_state.as_deref()))
                .collect();

            if meetings.is_empty() {
                return Ok(None);
            }

            // 2. Get actions — use get_due_actions(90) to match Actions page counts
            let actions = db.get_due_actions(90).unwrap_or_default();
            let focus_candidates = db.get_focus_candidate_actions(7).unwrap_or_default();

            // 3. Get entity map and intelligence qualities
            let meeting_ids: Vec<String> = meetings.iter().map(|m| m.id.clone()).collect();
            // DOS-258: read from the linked_entities view rather than the legacy
            // meeting_entities junction table so dashboard prep chips match the
            // meeting detail page.
            let entity_map = db
                .get_linked_entities_map_for_meetings(&meeting_ids)
                .unwrap_or_default();
            let mut iq_map = HashMap::new();
            for mid in &meeting_ids {
                let q = crate::intelligence::assess_intelligence_quality(db, mid);
                iq_map.insert(mid.clone(), q);
            }
            let lifecycle_updates = load_dashboard_lifecycle_updates(db, 3);
            let briefing_callouts = load_briefing_callouts(db, 10);
            let aging_count = crate::services::actions::get_aging_action_count(db).ok();

            Ok(Some(LiveSnapshot {
                meetings,
                actions,
                focus_candidates,
                entity_map,
                intelligence_qualities: iq_map,
                lifecycle_updates,
                briefing_callouts,
                aging_action_count: aging_count.filter(|&c| c > 0),
            }))
        })
        .await
    {
        Ok(Some(snap)) => snap,
        _ => return None,
    };

    // Convert DbMeetings to frontend Meeting structs (outside lock).
    let meetings: Vec<Meeting> = snap
        .meetings
        .into_iter()
        .map(|dbm| {
            let meeting_type = crate::parser::parse_meeting_type(&dbm.meeting_type);
            let has_prep = dbm.prep_context_json.is_some();

            // Format time as "h:mm AM" from ISO start_time
            // Handles NaiveDateTime (%Y-%m-%dT%H:%M:%S), space-separated, and
            // RFC3339 with timezone offset (e.g. 2026-02-28T09:30:00-05:00).
            let time = chrono::NaiveDateTime::parse_from_str(&dbm.start_time, "%Y-%m-%dT%H:%M:%S")
                .map(|dt| dt.format("%-I:%M %p").to_string())
                .or_else(|_| {
                    chrono::NaiveDateTime::parse_from_str(&dbm.start_time, "%Y-%m-%d %H:%M:%S")
                        .map(|dt| dt.format("%-I:%M %p").to_string())
                })
                .or_else(|_| {
                    chrono::DateTime::parse_from_rfc3339(&dbm.start_time)
                        .map(|dt| dt.format("%-I:%M %p").to_string())
                })
                .unwrap_or_else(|_| dbm.start_time.clone());

            let end_time = dbm.end_time.as_ref().and_then(|et| {
                chrono::NaiveDateTime::parse_from_str(et, "%Y-%m-%dT%H:%M:%S")
                    .map(|dt| dt.format("%-I:%M %p").to_string())
                    .or_else(|_| {
                        chrono::NaiveDateTime::parse_from_str(et, "%Y-%m-%d %H:%M:%S")
                            .map(|dt| dt.format("%-I:%M %p").to_string())
                    })
                    .or_else(|_| {
                        chrono::DateTime::parse_from_rfc3339(et)
                            .map(|dt| dt.format("%-I:%M %p").to_string())
                    })
                    .ok()
            });

            let linked_entities = snap.entity_map.get(&dbm.id).cloned();
            let intelligence_quality = snap.intelligence_qualities.get(&dbm.id).cloned();

            let prep = dbm
                .prep_context_json
                .as_deref()
                .and_then(extract_dashboard_prep);
            let calendar_description = dbm.description.clone();

            Meeting {
                id: dbm.id,
                calendar_event_id: dbm.calendar_event_id,
                time,
                end_time,
                start_iso: Some(dbm.start_time),
                title: dbm.title,
                meeting_type,
                prep,
                is_current: None,
                prep_file: None,
                has_prep,
                overlay_status: None,
                prep_reviewed: None,
                linked_entities,
                suggested_unarchive_account_id: None,
                intelligence_quality,
                calendar_attendees: None,
                calendar_description,
            }
        })
        .collect();

    // Build actions
    let actions: Vec<Action> = snap
        .actions
        .into_iter()
        .map(|dba| {
            let priority = Priority::from_i32(dba.priority);
            Action {
                id: dba.id,
                title: dba.title,
                account: dba.account_id,
                due_date: dba.due_date,
                priority,
                status: crate::types::ActionStatus::Unstarted,
                is_overdue: None,
                context: dba.context,
                source: dba.source_label,
                days_overdue: None,
            }
        })
        .collect();

    // Build overview
    let hour = chrono::Timelike::hour(&chrono::Local::now());
    let greeting = if hour < 12 {
        "Good morning"
    } else if hour < 17 {
        "Good afternoon"
    } else {
        "Good evening"
    };
    // Count only active (non-cancelled) and non-personal meetings for display
    let active_meetings_count = meetings
        .iter()
        .filter(|m| {
            m.overlay_status != Some(OverlayStatus::Cancelled)
                && m.meeting_type != MeetingType::Personal
        })
        .count();
    let overview = DayOverview {
        greeting: greeting.to_string(),
        date: chrono::Local::now().format("%A, %B %e").to_string(),
        summary: format!(
            "You have {} meeting{} today",
            active_meetings_count,
            if active_meetings_count == 1 { "" } else { "s" }
        ),
        focus: None,
    };

    // Compute focus capacity
    let config_guard = state.config.read();
    let config = config_guard.as_ref()?;
    let tz: chrono_tz::Tz = config
        .schedules
        .today
        .timezone
        .parse()
        .unwrap_or(chrono_tz::America::New_York);
    let today_date = chrono::Local::now().date_naive();
    let capacity =
        crate::focus_capacity::compute_focus_capacity(crate::focus_capacity::FocusCapacityInput {
            meetings: meetings.clone(),
            source: crate::focus_capacity::FocusCapacitySource::Live,
            timezone: tz,
            work_hours_start: config.google.work_hours_start,
            work_hours_end: config.google.work_hours_end,
            day_date: today_date,
        });
    let focus = if snap.focus_candidates.is_empty() {
        None
    } else {
        let (prioritized, top_three, implications) =
            crate::focus_prioritization::prioritize_actions(
                snap.focus_candidates,
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
    };

    // Stats
    let workspace = Path::new(&config.workspace_path);
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

    Some(DashboardData {
        overview,
        stats,
        meetings,
        actions,
        lifecycle_updates: if snap.lifecycle_updates.is_empty() {
            None
        } else {
            Some(snap.lifecycle_updates)
        },
        emails: None,
        email_sync: None,
        focus,
        email_narrative: None,
        replies_needed: Vec::new(),
        user_domains: crate::state::load_config()
            .ok()
            .map(|c| c.resolved_user_domains())
            .filter(|d| !d.is_empty()),
        briefing_callouts: snap.briefing_callouts,
        aging_action_count: snap.aging_action_count,
    })
}

/// Get dashboard data from workspace _today/data/ JSON files.
///
/// This is the main business logic for the `get_dashboard_data` command.
/// Returns the full DashboardResult including latency tracking.
pub async fn get_dashboard_data(state: &AppState) -> DashboardResult {
    let engine = std::sync::Arc::clone(&state.signals.engine);
    let _ = state
        .db_write(move |db| {
            crate::services::accounts::refresh_lifecycle_states_for_dashboard(db, &engine)
        })
        .await;

    let started = std::time::Instant::now();
    let mut db_busy = false;

    let result = get_dashboard_data_inner(state, &mut db_busy).await;

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

async fn get_dashboard_data_inner(state: &AppState, db_busy: &mut bool) -> DashboardResult {
    // Get Google auth status for frontend
    let google_auth = state
        .calendar
        .google_auth
        .lock()
        .clone();
    // Get config
    let config = match state.config.read().clone() {
            Some(c) => c,
            None => {
                return DashboardResult::Error {
                    message: "No configuration. Create ~/.dailyos/config.json with { \"workspacePath\": \"/path/to/workspace\" }".to_string(),
                }
            }
        };

    let workspace = Path::new(&config.workspace_path);

    // Build meetings from SQLite (DB-first, I513)
    let live_events = state
        .calendar
        .events
        .read()
        .clone();
    let tz: chrono_tz::Tz = config
        .schedules
        .today
        .timezone
        .parse()
        .unwrap_or(chrono_tz::America::New_York);

    // Query today's meetings from DB.
    // start_time is stored in two formats (RFC3339 UTC from calendar poller,
    // local "YYYY-MM-DD HH:MM AM" from pipeline). Bare date comparison works
    // for both. Evening UTC edge cases are caught by live calendar merge below.
    let tf = crate::helpers::today_meeting_filter(&tz);
    let today_clone = tf.date.clone();
    let tomorrow_clone = tf.next_date.clone();
    let db_meetings: Vec<crate::db::DbMeeting> = match state
        .db_read(move |db| {
            let conn = db.conn_ref();
            let mut stmt = conn
                .prepare(
                    "SELECT m.id, m.title, m.meeting_type, m.start_time, m.end_time, m.attendees,
                        m.notes_path, mt.summary, m.created_at, m.calendar_event_id, m.description,
                        mp.prep_context_json, mt.intelligence_state
                     FROM meetings m
                     LEFT JOIN meeting_prep mp ON mp.meeting_id = m.id
                     LEFT JOIN meeting_transcripts mt ON mt.meeting_id = m.id
                     WHERE m.start_time >= ?1 AND m.start_time < ?2
                     ORDER BY m.start_time ASC",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(rusqlite::params![today_clone, tomorrow_clone], |row| {
                    Ok(crate::db::DbMeeting {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        meeting_type: row.get(2)?,
                        start_time: row.get(3)?,
                        end_time: row.get(4)?,
                        attendees: row.get(5)?,
                        notes_path: row.get(6)?,
                        summary: row.get(7)?,
                        created_at: row.get(8)?,
                        calendar_event_id: row.get(9)?,
                        description: row.get(10)?,
                        prep_context_json: row.get(11)?,
                        user_agenda_json: None,
                        user_notes: None,
                        prep_frozen_json: None,
                        prep_frozen_at: None,
                        prep_snapshot_path: None,
                        prep_snapshot_hash: None,
                        transcript_path: None,
                        transcript_processed_at: None,
                        intelligence_state: row.get(12)?,
                        intelligence_quality: None,
                        last_enriched_at: None,
                        signal_count: None,
                        has_new_signals: None,
                        last_viewed_at: None,
                    })
                })
                .map_err(|e| e.to_string())?;
            Ok(rows
                .filter_map(|r| r.ok())
                .filter(|m| include_dashboard_meeting(m.intelligence_state.as_deref()))
                .collect())
        })
        .await
    {
        Ok(meetings) => meetings,
        Err(e) => {
            log::warn!("Dashboard DB query failed: {e}");
            *db_busy = true;
            Vec::new()
        }
    };

    // NOTE: We deliberately do NOT return Empty when meetings are empty.
    // The briefing should render with whatever data exists (emails, actions,
    // lifecycle updates, callouts) even on meeting-free days or before
    // calendar syncs. The frontend handles empty meeting sections gracefully.

    // Convert DB meetings to frontend Meeting structs
    let briefing_meetings: Vec<Meeting> = db_meetings
        .into_iter()
        .map(|dbm| {
            let meeting_type = crate::parser::parse_meeting_type(&dbm.meeting_type);
            let has_prep = dbm.prep_context_json.is_some();

            // Helper to format time in user's timezone
            let format_meeting_time = |time_str: &str| -> String {
                // Try RFC3339 first (from Google Calendar API, has timezone info)
                if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(time_str) {
                    let utc_dt = dt.with_timezone(&Utc);
                    return utc_dt.with_timezone(&tz).format("%-I:%M %p").to_string();
                }
                // Fall back to local format (from pipeline, assume in user's timezone)
                chrono::NaiveDateTime::parse_from_str(time_str, "%Y-%m-%dT%H:%M:%S")
                    .or_else(|_| {
                        chrono::NaiveDateTime::parse_from_str(time_str, "%Y-%m-%d %H:%M:%S")
                    })
                    .map(|dt| dt.format("%-I:%M %p").to_string())
                    .unwrap_or_else(|_| time_str.to_string())
            };

            let time = format_meeting_time(&dbm.start_time);
            let end_time = dbm.end_time.as_ref().map(|et| format_meeting_time(et));

            let prep = dbm
                .prep_context_json
                .as_deref()
                .and_then(extract_dashboard_prep);
            let calendar_description = dbm.description.clone();

            Meeting {
                id: dbm.id,
                calendar_event_id: dbm.calendar_event_id,
                time,
                end_time,
                start_iso: Some(dbm.start_time),
                title: dbm.title,
                meeting_type,
                prep,
                is_current: None,
                prep_file: None,
                has_prep,
                overlay_status: None,
                prep_reviewed: None,
                linked_entities: None,
                suggested_unarchive_account_id: None,
                intelligence_quality: None,
                calendar_attendees: None,
                calendar_description,
            }
        })
        .collect();

    // Merge DB meetings with live calendar events (ADR-0032)
    let mut meetings = crate::calendar_merge::merge_meetings(briefing_meetings, &live_events, &tz);

    // Filter out personal meetings — they're tracked but not displayed in briefing (ADR-0032)
    meetings.retain(|m| m.meeting_type != MeetingType::Personal);

    // Consolidate all dashboard DB reads into a single lock acquisition (I235).
    // This reduces lock contention and improves dashboard load latency.
    let meeting_ids: Vec<String> = meetings.iter().map(|m| m.id.clone()).collect();
    struct DashboardDbSnapshot {
        reviewed: Option<HashMap<String, String>>,
        entity_map: Option<HashMap<String, Vec<crate::types::LinkedEntity>>>,
        accounts_with_domains: Option<Vec<(crate::db::DbAccount, Vec<String>)>>,
        non_briefing_actions: Option<Vec<crate::db::DbAction>>,
        focus_candidates: Option<Vec<crate::db::DbAction>>,
        intelligence_qualities: HashMap<String, crate::types::IntelligenceQuality>,
    }

    let meeting_ids_clone = meeting_ids.clone();
    let db_snapshot = match state
        .db_read(move |db| {
            let mut iq_map = HashMap::new();
            for mid in &meeting_ids_clone {
                let q = crate::intelligence::assess_intelligence_quality(db, mid);
                iq_map.insert(mid.clone(), q);
            }
            Ok(DashboardDbSnapshot {
                reviewed: db.get_reviewed_preps().ok(),
                // DOS-258: read from the linked_entities view rather than the legacy
                // meeting_entities junction table so dashboard snapshot chips match
                // the meeting detail page.
                entity_map: db
                    .get_linked_entities_map_for_meetings(&meeting_ids_clone)
                    .ok(),
                accounts_with_domains: db.get_all_accounts_with_domains(true).ok(),
                non_briefing_actions: db.get_due_actions(90).ok(),
                focus_candidates: db.get_focus_candidate_actions(7).ok(),
                intelligence_qualities: iq_map,
            })
        })
        .await
    {
        Ok(snap) => Some(snap),
        Err(_) => {
            *db_busy = true;
            None
        }
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
                let linked_account_id = m
                    .linked_entities
                    .as_ref()
                    .and_then(|ents| ents.iter().find(|e| e.entity_type == "account"))
                    .map(|e| e.id.clone());
                let linked_account_name = m
                    .linked_entities
                    .as_ref()
                    .and_then(|ents| ents.iter().find(|e| e.entity_type == "account"))
                    .map(|e| e.name.clone());

                if let Some(ref account_id) = linked_account_id {
                    if !archived_ids.contains(&account_id.to_lowercase()) {
                        continue;
                    }
                }

                let account_hint_key = linked_account_name
                    .as_deref()
                    .map(normalize_match_key)
                    .unwrap_or_default();
                let account_id_key = linked_account_id
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
                        if let Some(account_domains) = domains_by_account.get(&archived_account.id)
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

    // Annotate meetings with intelligence quality from lifecycle assessment (I329)
    if let Some(ref snap) = db_snapshot {
        for m in &mut meetings {
            if let Some(q) = snap.intelligence_qualities.get(&m.id) {
                m.intelligence_quality = Some(q.clone());
            }
        }
    }

    // Load actions from DB (I513 — DB is sole source)
    let actions: Vec<Action> = db_snapshot
        .as_ref()
        .and_then(|snap| snap.non_briefing_actions.as_ref())
        .map(|db_actions| {
            db_actions
                .iter()
                .map(|dba| {
                    let priority = Priority::from_i32(dba.priority);
                    Action {
                        id: dba.id.clone(),
                        title: dba.title.clone(),
                        account: dba.account_id.clone(),
                        due_date: dba.due_date.clone(),
                        priority,
                        status: crate::types::ActionStatus::Unstarted,
                        is_overdue: None,
                        context: dba.context.clone(),
                        source: dba.source_label.clone(),
                        days_overdue: None,
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    // I368: Try DB first for enriched emails, fall back to JSON
    let (emails, email_sync): (Option<Vec<crate::types::Email>>, Option<EmailSyncStatus>) = {
        let mut db_emails: Vec<crate::types::Email> = state
            .db_read(|db| {
                let all_rows = db.get_all_active_emails().map_err(|e| e.to_string())?;
                if all_rows.is_empty() {
                    return Ok(Vec::new());
                }
                // Collapse to latest email per thread — matches EmailsPage behavior
                let rows = crate::services::emails::collapse_to_latest_thread_emails(&all_rows);
                // Batch-resolve entity names (same approach as emails service)
                let entity_ids: std::collections::HashSet<String> =
                    rows.iter().filter_map(|e| e.entity_id.clone()).collect();
                let mut entity_names: std::collections::HashMap<String, String> =
                    std::collections::HashMap::new();
                for eid in &entity_ids {
                    if let Ok(Some(a)) = db.get_account(eid) {
                        entity_names.insert(eid.clone(), a.name);
                    } else if let Ok(Some(p)) = db.get_person(eid) {
                        // Find the most relevant linked account using email context
                        let email_context: String = rows
                            .iter()
                            .filter(|e| e.entity_id.as_deref() == Some(eid.as_str()))
                            .filter_map(|e| {
                                e.contextual_summary.as_deref().or(e.subject.as_deref())
                            })
                            .collect::<Vec<_>>()
                            .join(" ")
                            .to_lowercase();
                        let display = crate::services::emails::best_account_for_person(
                            db,
                            eid,
                            &email_context,
                        )
                        .unwrap_or(p.name);
                        entity_names.insert(eid.clone(), display);
                    } else if let Ok(Some(p)) = db.get_project(eid) {
                        entity_names.insert(eid.clone(), p.name);
                    }
                }
                Ok(rows
                    .iter()
                    .map(|dbe| {
                        let entity_name = dbe
                            .entity_id
                            .as_ref()
                            .and_then(|eid| entity_names.get(eid).cloned());
                        crate::types::Email {
                            id: dbe.email_id.clone(),
                            sender: dbe.sender_name.clone().unwrap_or_default(),
                            sender_email: dbe.sender_email.clone().unwrap_or_default(),
                            subject: dbe.subject.clone().unwrap_or_default(),
                            snippet: dbe.snippet.clone(),
                            priority: match dbe.priority.as_deref() {
                                Some("high") => crate::types::EmailPriority::High,
                                Some("low") => crate::types::EmailPriority::Low,
                                _ => crate::types::EmailPriority::Medium,
                            },
                            avatar_url: None,
                            summary: dbe.contextual_summary.clone(),
                            recommended_action: None,
                            conversation_arc: None,
                            email_type: None,
                            commitments: dbe
                                .commitments
                                .as_ref()
                                .and_then(|c| serde_json::from_str::<Vec<String>>(c).ok())
                                .unwrap_or_default(),
                            questions: dbe
                                .questions
                                .as_ref()
                                .and_then(|q| serde_json::from_str::<Vec<String>>(q).ok())
                                .unwrap_or_default(),
                            sentiment: dbe.sentiment.clone(),
                            urgency: dbe.urgency.clone(),
                            entity_id: dbe.entity_id.clone(),
                            entity_type: dbe.entity_type.clone(),
                            entity_name,
                            relevance_score: dbe.relevance_score,
                            score_reason: dbe.score_reason.clone(),
                            is_unread: dbe.is_unread,
                            pinned_at: dbe.pinned_at.clone(),
                            tracked_commitments: Vec::new(),
                            meeting_linked: None,
                        }
                    })
                    .collect())
            })
            .await
            .unwrap_or_default();

        db_emails.sort_by(crate::services::emails::compare_email_rank);

        // I448: DB is source of truth — no JSON fallback (archived emails
        // have resolved_at set and are correctly filtered by DB queries)
        if !db_emails.is_empty() {
            (Some(db_emails), None)
        } else {
            (None, None)
        }
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
        let candidates = match db_snapshot
            .as_ref()
            .and_then(|s| s.focus_candidates.clone())
        {
            Some(c) => c,
            None => return None,
        };
        let (prioritized, top_three, implications) =
            crate::focus_prioritization::prioritize_actions(candidates, capacity.available_minutes);
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
    let total_meetings = active_meetings.len();
    let customer_meetings = active_meetings
        .iter()
        .filter(|m| matches!(m.meeting_type, MeetingType::Customer | MeetingType::Qbr))
        .count();
    let stats = DayStats {
        total_meetings,
        customer_meetings,
        actions_due: actions.len(),
        inbox_count,
    };

    // Build overview from live state (I513 — no JSON overview source)
    let hour = chrono::Timelike::hour(&chrono::Local::now());
    let greeting = if hour < 12 {
        "Good morning"
    } else if hour < 17 {
        "Good afternoon"
    } else {
        "Good evening"
    };
    let mut overview = DayOverview {
        greeting: greeting.to_string(),
        date: chrono::Local::now().format("%A, %B %e").to_string(),
        summary: String::new(),
        focus: None,
    };
    let mut parts = vec![format!(
        "You have {} meeting{} today",
        total_meetings,
        if total_meetings == 1 { "" } else { "s" }
    )];
    if customer_meetings > 0 {
        parts.push(format!(
            "{} customer call{}",
            customer_meetings,
            if customer_meetings != 1 { "s" } else { "" }
        ));
    }
    overview.summary = parts.join(" with ");

    // DB-based freshness: check app_state_kv for briefing timestamp, fall back to meeting existence
    let freshness_today = tf.date.clone();
    let freshness_tomorrow = tf.next_date.clone();
    let freshness = match state
        .db_read(move |db| {
            let conn = db.conn_ref();
            // Try app_state_kv briefing_freshness key first
            let kv_result: Result<Option<String>, _> = conn.query_row(
                "SELECT value_json FROM app_state_kv WHERE key = 'briefing_freshness'",
                [],
                |row| row.get(0),
            );
            if let Ok(Some(json_str)) = kv_result {
                if let Ok(manifest) = serde_json::from_str::<serde_json::Value>(&json_str) {
                    let date = manifest.get("date").and_then(|v| v.as_str()).unwrap_or("");
                    let generated_at = manifest
                        .get("generatedAt")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let today_str = chrono::Local::now().format("%Y-%m-%d").to_string();
                    if date == today_str {
                        return Ok(DataFreshness::Fresh { generated_at });
                    } else {
                        return Ok(DataFreshness::Stale {
                            data_date: date.to_string(),
                            generated_at,
                        });
                    }
                }
            }
            // Fallback: if meetings exist for today, consider it fresh
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM meetings WHERE start_time >= ?1 AND start_time < ?2",
                    rusqlite::params![freshness_today, freshness_tomorrow],
                    |row| row.get(0),
                )
                .unwrap_or(0);
            if count > 0 {
                Ok(DataFreshness::Fresh {
                    generated_at: chrono::Utc::now().to_rfc3339(),
                })
            } else {
                Ok(DataFreshness::Unknown)
            }
        })
        .await
    {
        Ok(f) => f,
        Err(_) => DataFreshness::Unknown,
    };

    // I513: Build replies_needed from DB instead of directive file.
    let replies_needed: Vec<crate::json_loader::DirectiveReplyNeeded> = state
        .db_read(|db| {
            let now = chrono::Utc::now();
            Ok(db
                .get_threads_awaiting_reply()
                .unwrap_or_default()
                .into_iter()
                .map(|(thread_id, subject, from, date)| {
                    let wait_duration =
                        crate::prepare::orchestrate::compute_wait_duration_public(&date, &now);
                    crate::json_loader::DirectiveReplyNeeded {
                        thread_id,
                        subject,
                        from,
                        date: Some(date),
                        wait_duration: Some(wait_duration),
                    }
                })
                .collect())
        })
        .await
        .unwrap_or_default();

    let email_narrative: Option<String> = {
        let email_count = emails.as_ref().map(|v| v.len()).unwrap_or(0);
        if email_count == 0 {
            None
        } else {
            // Count entities with meetings today
            let entity_ids: Vec<String> = emails
                .as_ref()
                .map(|v| {
                    v.iter()
                        .filter_map(|e| e.entity_id.clone())
                        .collect::<std::collections::HashSet<_>>()
                        .into_iter()
                        .collect()
                })
                .unwrap_or_default();
            let meeting_linked = if entity_ids.is_empty() {
                0usize
            } else {
                let email_today = tf.date.clone();
                let email_tomorrow = tf.next_date.clone();
                state.db_read(move |db| {
                        let count = entity_ids.iter().filter(|eid| {
                            db.conn_ref()
                                .query_row(
                                    "SELECT COUNT(*) FROM meeting_entities me
                                     JOIN meetings m ON me.meeting_id = m.id
                                     WHERE me.entity_id = ?1 AND m.start_time >= ?2 AND m.start_time < ?3",
                                    rusqlite::params![eid, email_today, email_tomorrow],
                                    |row| row.get::<_, i64>(0),
                                )
                                .unwrap_or(0) > 0
                        }).count();
                        Ok::<usize, String>(count)
                    }).await.unwrap_or(0)
            };
            if meeting_linked > 0 {
                Some(format!(
                    "{} threads in your inbox, {} linked to today's meetings.",
                    email_count, meeting_linked
                ))
            } else {
                Some(format!("{} threads in your inbox.", email_count))
            }
        }
    };

    // Fall back to DB enrichment stats when JSON sync status absent (I373)
    let email_sync_fallback: Option<EmailSyncStatus> = if email_sync.is_none() {
        match state
            .db_read(|db| db.get_email_sync_stats().map_err(|e| e.to_string()))
            .await
        {
            Ok(stats) => stats.last_fetch_at.as_ref().map(|last| EmailSyncStatus {
                state: if stats.failed > 0 {
                    EmailSyncState::Warning
                } else {
                    EmailSyncState::Ok
                },
                stage: EmailSyncStage::Enrich,
                code: None,
                message: Some(format!("{}/{} ready", stats.enriched, stats.total)),
                using_last_known_good: None,
                can_retry: if stats.failed > 0 { Some(true) } else { None },
                last_attempt_at: Some(last.clone()),
                last_success_at: Some(last.clone()),
                enrichment_pending: Some(stats.pending as i64),
                enrichment_enriched: Some(stats.enriched as i64),
                enrichment_failed: Some(stats.failed as i64),
                total_active: Some(stats.total as i64),
            }),
            Err(_) => None,
        }
    } else {
        None
    };

    let (lifecycle_updates, briefing_callouts, aging_action_count) = state
        .db_read(|db| {
            let lu = load_dashboard_lifecycle_updates(db, 3);
            let bc = load_briefing_callouts(db, 10);
            let ac = crate::services::actions::get_aging_action_count(db).ok();
            Ok::<(Vec<DashboardLifecycleUpdate>, Vec<DashboardBriefingCallout>, Option<i64>), String>((lu, bc, ac))
        })
        .await
        .unwrap_or_default();

    DashboardResult::Success {
        data: DashboardData {
            overview,
            stats,
            meetings,
            actions,
            lifecycle_updates: if lifecycle_updates.is_empty() {
                None
            } else {
                Some(lifecycle_updates)
            },
            emails,
            email_sync: email_sync.or(email_sync_fallback),
            focus,
            email_narrative,
            replies_needed,
            user_domains: {
                let domains = config.resolved_user_domains();
                if domains.is_empty() {
                    None
                } else {
                    Some(domains)
                }
            },
            briefing_callouts,
            aging_action_count: aging_action_count.filter(|&c| c > 0),
        },
        freshness,
        google_auth,
    }
}

/// Get week overview data from DB (I513: no JSON file reads).
pub fn get_week_data(_state: &AppState) -> WeekResult {
    let started = std::time::Instant::now();

    let db = match crate::db::ActionDb::open() {
        Ok(db) => db,
        Err(e) => {
            return WeekResult::Error {
                message: format!("Failed to open DB: {}", e),
            }
        }
    };

    // Compute Monday..Sunday of current week
    let today = chrono::Local::now().date_naive();
    let weekday = today.weekday().num_days_from_monday(); // Mon=0 .. Sun=6
    let monday = today - chrono::Duration::days(weekday as i64);
    let next_monday = monday + chrono::Duration::days(7);

    let week_number = format!("W{:02}", today.iso_week().week());
    let friday = monday + chrono::Duration::days(4);
    let date_range = format!(
        "{} – {}",
        monday.format("%b %d"),
        friday.format("%b %d, %Y")
    );

    // Query all meetings Mon–Sun
    let meetings_raw = db
        .get_meetings_in_range(&monday.to_string(), &next_monday.to_string())
        .unwrap_or_default();

    // Group by day
    static DAY_NAMES: [&str; 7] = [
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
        "Sunday",
    ];
    let mut days = Vec::new();
    for i in 0..7 {
        let day_date = monday + chrono::Duration::days(i);
        let day_str = day_date.to_string();
        let day_meetings: Vec<crate::types::WeekMeeting> = meetings_raw
            .iter()
            .filter(|(_, _, _, start, _, _)| start.starts_with(&day_str))
            .filter_map(|(id, title, mtype, start, _, has_prep)| {
                // Skip personal meetings
                if mtype == "personal" {
                    return None;
                }
                let time = chrono::NaiveDateTime::parse_from_str(start, "%Y-%m-%dT%H:%M:%S")
                    .or_else(|_| chrono::NaiveDateTime::parse_from_str(start, "%Y-%m-%d %H:%M:%S"))
                    .map(|dt| dt.format("%l:%M %p").to_string().trim().to_string())
                    .unwrap_or_else(|_| {
                        // Try parsing with timezone info
                        chrono::DateTime::parse_from_rfc3339(start)
                            .map(|dt| {
                                dt.with_timezone(&chrono::Local)
                                    .format("%l:%M %p")
                                    .to_string()
                                    .trim()
                                    .to_string()
                            })
                            .unwrap_or_default()
                    });
                let meeting_type = match mtype.as_str() {
                    "customer" => MeetingType::Customer,
                    "qbr" => MeetingType::Qbr,
                    "training" => MeetingType::Training,
                    "team_sync" => MeetingType::TeamSync,
                    "one_on_one" => MeetingType::OneOnOne,
                    "partnership" => MeetingType::Partnership,
                    "all_hands" => MeetingType::AllHands,
                    "external" => MeetingType::External,
                    "personal" => MeetingType::Personal,
                    _ => MeetingType::Internal,
                };
                let prep_status = if *has_prep {
                    crate::types::PrepStatus::PrepReady
                } else {
                    crate::types::PrepStatus::PrepNeeded
                };
                Some(crate::types::WeekMeeting {
                    time,
                    title: title.clone(),
                    meeting_id: Some(id.clone()),
                    meeting_type,
                    prep_status,
                    linked_entities: None,
                })
            })
            .collect();
        days.push(crate::types::WeekDay {
            date: day_str,
            day_name: DAY_NAMES[i as usize].to_string(),
            meetings: day_meetings,
        });
    }

    // Action summary from DB
    let action_summary = db
        .get_pending_action_counts()
        .ok()
        .map(
            |(total, _p1, _p2, overdue)| crate::types::WeekActionSummary {
                overdue_count: overdue as usize,
                due_this_week: total as usize,
                critical_items: Vec::new(),
                overdue: None,
                due_this_week_items: None,
            },
        );

    let mut week = WeekOverview {
        week_number,
        date_range,
        days,
        action_summary,
        hygiene_alerts: None,
        focus_areas: None,
        available_time_blocks: None,
        week_narrative: None,
        top_priority: None,
        readiness_checks: None,
        day_shapes: None,
    };

    // Enrich dayShapes with live per-day action priorities (I279)
    if let Some(ref mut shapes) = week.day_shapes {
        if let Ok(candidates) = db.get_focus_candidate_actions(7) {
            for shape in shapes.iter_mut() {
                let available_minutes: u32 = shape
                    .available_blocks
                    .iter()
                    .map(|b| b.duration_minutes)
                    .sum();

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

    fn log_latency(command: &str, started: std::time::Instant, budget_ms: u128) {
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
    log_latency("get_week_data", started, READ_CMD_LATENCY_BUDGET_MS);
    WeekResult::Success { data: week }
}

#[cfg(test)]
mod tests {
    use super::include_dashboard_meeting;

    #[test]
    fn archived_meetings_are_excluded_from_dashboard() {
        assert!(!include_dashboard_meeting(Some("archived")));
    }

    #[test]
    fn active_meetings_remain_visible_on_dashboard() {
        assert!(include_dashboard_meeting(None));
        assert!(include_dashboard_meeting(Some("detected")));
        assert!(include_dashboard_meeting(Some("enriched")));
    }
}
