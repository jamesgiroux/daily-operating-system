// Dashboard service — extracted from commands.rs
// Business logic for dashboard data loading (daily briefing, live fallback).

use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::json_loader::{
    check_data_freshness, load_actions_json, load_directive, load_emails_json,
    load_emails_json_with_sync, load_schedule_json, DataFreshness,
};
use crate::parser::count_inbox;
use crate::state::{AppState, DbTryRead};
use crate::types::{
    Action, CalendarEvent, DailyFocus, DashboardData, DayOverview, DayStats,
    EmailSyncStage, EmailSyncState, EmailSyncStatus, GoogleAuthStatus, Meeting, MeetingType, OverlayStatus, Priority,
    WeekOverview,
};

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

/// Build dashboard data from live SQLite when schedule.json is missing.
///
/// Returns `None` if no meetings exist for today or DB is unavailable.
pub fn build_live_dashboard_data(state: &AppState) -> Option<DashboardData> {
    // Gather all data under a single try-read lock.
    struct LiveSnapshot {
        meetings: Vec<crate::db::DbMeeting>,
        actions: Vec<crate::db::DbAction>,
        focus_candidates: Vec<crate::db::DbAction>,
        entity_map: HashMap<String, Vec<crate::types::LinkedEntity>>,
        intelligence_qualities: HashMap<String, crate::types::IntelligenceQuality>,
    }

    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let tomorrow = (chrono::Local::now() + chrono::Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();

    let snap = match state.with_db_try_read(|db| {
        let conn = db.conn_ref();

        // 1. Query today's meetings from meetings_history
        let mut stmt = conn
            .prepare(
                "SELECT id, title, meeting_type, start_time, end_time, attendees,
                        notes_path, summary, created_at, calendar_event_id, description,
                        prep_context_json, intelligence_state
                 FROM meetings_history
                 WHERE start_time >= ?1 AND start_time < ?2
                 ORDER BY start_time ASC",
            )
            .ok()?;
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
            .ok()?;
        let meetings: Vec<crate::db::DbMeeting> =
            meeting_rows.filter_map(|r| r.ok()).collect();

        if meetings.is_empty() {
            return None;
        }

        // 2. Get actions
        let actions = db.get_non_briefing_pending_actions().unwrap_or_default();
        let focus_candidates = db.get_focus_candidate_actions(7).unwrap_or_default();

        // 3. Get entity map and intelligence qualities
        let meeting_ids: Vec<String> = meetings.iter().map(|m| m.id.clone()).collect();
        let entity_map = db.get_meeting_entity_map(&meeting_ids).unwrap_or_default();
        let mut iq_map = HashMap::new();
        for mid in &meeting_ids {
            let q = crate::intelligence::assess_intelligence_quality(db, mid);
            iq_map.insert(mid.clone(), q);
        }

        Some(LiveSnapshot {
            meetings,
            actions,
            focus_candidates,
            entity_map,
            intelligence_qualities: iq_map,
        })
    }) {
        DbTryRead::Ok(Some(snap)) => snap,
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
            let time = chrono::NaiveDateTime::parse_from_str(&dbm.start_time, "%Y-%m-%dT%H:%M:%S")
                .or_else(|_| chrono::NaiveDateTime::parse_from_str(&dbm.start_time, "%Y-%m-%d %H:%M:%S"))
                .map(|dt| dt.format("%-I:%M %p").to_string())
                .unwrap_or_else(|_| dbm.start_time.clone());

            let end_time = dbm.end_time.as_ref().and_then(|et| {
                chrono::NaiveDateTime::parse_from_str(et, "%Y-%m-%dT%H:%M:%S")
                    .or_else(|_| chrono::NaiveDateTime::parse_from_str(et, "%Y-%m-%d %H:%M:%S"))
                    .map(|dt| dt.format("%-I:%M %p").to_string())
                    .ok()
            });

            let linked_entities = snap.entity_map.get(&dbm.id).cloned();
            let intelligence_quality = snap.intelligence_qualities.get(&dbm.id).cloned();

            Meeting {
                id: dbm.id,
                calendar_event_id: dbm.calendar_event_id,
                time,
                end_time,
                start_iso: Some(dbm.start_time),
                title: dbm.title,
                meeting_type,
                prep: None,
                is_current: None,
                prep_file: None,
                has_prep,
                overlay_status: None,
                prep_reviewed: None,
                linked_entities,
                suggested_unarchive_account_id: None,
                intelligence_quality,
                calendar_attendees: None,
                calendar_description: None,
            }
        })
        .collect();

    // Build actions
    let actions: Vec<Action> = snap
        .actions
        .into_iter()
        .map(|dba| {
            let priority = match dba.priority.as_str() {
                "P1" => Priority::P1,
                "P3" => Priority::P3,
                _ => Priority::P2,
            };
            Action {
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
    let meeting_count = meetings.len();
    let overview = DayOverview {
        greeting: greeting.to_string(),
        date: chrono::Local::now().format("%A, %B %e").to_string(),
        summary: format!(
            "You have {} meeting{} today",
            meeting_count,
            if meeting_count == 1 { "" } else { "s" }
        ),
        focus: None,
    };

    // Compute focus capacity
    let config_guard = state.config.read().ok()?;
    let config = config_guard.as_ref()?;
    let tz: chrono_tz::Tz = config
        .schedules
        .today
        .timezone
        .parse()
        .unwrap_or(chrono_tz::America::New_York);
    let today_date = chrono::Local::now().date_naive();
    let capacity = crate::focus_capacity::compute_focus_capacity(
        crate::focus_capacity::FocusCapacityInput {
            meetings: meetings.clone(),
            source: crate::focus_capacity::FocusCapacitySource::Live,
            timezone: tz,
            work_hours_start: config.google.work_hours_start,
            work_hours_end: config.google.work_hours_end,
            day_date: today_date,
        },
    );
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
        emails: None,
        email_sync: None,
        focus,
        email_narrative: None,
        replies_needed: Vec::new(),
        user_domains: crate::state::load_config()
            .ok()
            .map(|c| c.resolved_user_domains())
            .filter(|d| !d.is_empty()),
    })
}

/// Get dashboard data from workspace _today/data/ JSON files.
///
/// This is the main business logic for the `get_dashboard_data` command.
/// Returns the full DashboardResult including latency tracking.
pub fn get_dashboard_data(state: &AppState) -> DashboardResult {
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
        let today_dir_exists = today_dir.exists();
        let data_dir = today_dir.join("data");
        let data_dir_exists = today_dir_exists && data_dir.exists();

        // Load from JSON (happy path)
        let schedule_result = if data_dir_exists {
            load_schedule_json(&today_dir).ok()
        } else {
            None
        };

        // If schedule.json is unavailable, try building from live SQLite data
        let (overview, briefing_meetings) = match schedule_result {
            Some(data) => data,
            None => {
                // Fallback: build dashboard from SQLite meetings_history
                if let Some(live_data) = build_live_dashboard_data(state) {
                    if !live_data.meetings.is_empty() {
                        log::info!(
                            "schedule.json unavailable — serving {} meetings from SQLite",
                            live_data.meetings.len()
                        );
                        return DashboardResult::Success {
                            data: live_data,
                            freshness: DataFreshness::Unknown,
                            google_auth,
                        };
                    }
                }
                return DashboardResult::Empty {
                    message: "Your daily briefing will appear here once generated.".to_string(),
                    google_auth,
                };
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
            intelligence_qualities: HashMap<String, crate::types::IntelligenceQuality>,
        }

        let db_snapshot = match state.with_db_try_read(|db| {
            let mut iq_map = HashMap::new();
            for mid in &meeting_ids {
                let q = crate::intelligence::assess_intelligence_quality(db, mid);
                iq_map.insert(mid.clone(), q);
            }
            DashboardDbSnapshot {
                reviewed: db.get_reviewed_preps().ok(),
                entity_map: db.get_meeting_entity_map(&meeting_ids).ok(),
                accounts_with_domains: db.get_all_accounts_with_domains(true).ok(),
                non_briefing_actions: db.get_non_briefing_pending_actions().ok(),
                focus_candidates: db.get_focus_candidate_actions(7).ok(),
                intelligence_qualities: iq_map,
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
                    let linked_account_id = m.linked_entities.as_ref()
                        .and_then(|ents| ents.iter().find(|e| e.entity_type == "account"))
                        .map(|e| e.id.clone());
                    let linked_account_name = m.linked_entities.as_ref()
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

        // Annotate meetings with intelligence quality from lifecycle assessment (I329)
        if let Some(ref snap) = db_snapshot {
            for m in &mut meetings {
                if let Some(q) = snap.intelligence_qualities.get(&m.id) {
                    m.intelligence_quality = Some(q.clone());
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

        // I368: Try DB first for enriched emails, fall back to JSON
        let (emails, email_sync): (Option<Vec<crate::types::Email>>, Option<EmailSyncStatus>) = {
            let mut db_emails: Vec<crate::types::Email> =
                match state.with_db_try_read(|db| {
                    let rows = db.get_all_active_emails()?;
                    // Batch-resolve entity names (same approach as emails service)
                    let entity_ids: std::collections::HashSet<String> = rows
                        .iter()
                        .filter_map(|e| e.entity_id.clone())
                        .collect();
                    let mut entity_names: std::collections::HashMap<String, String> = std::collections::HashMap::new();
                    for eid in &entity_ids {
                        if let Ok(Some(a)) = db.get_account(eid) {
                            entity_names.insert(eid.clone(), a.name);
                        } else if let Ok(Some(p)) = db.get_person(eid) {
                            // Find the most relevant linked account using email context
                            let email_context: String = rows.iter()
                                .filter(|e| e.entity_id.as_deref() == Some(eid.as_str()))
                                .filter_map(|e| e.contextual_summary.as_deref()
                                    .or(e.subject.as_deref()))
                                .collect::<Vec<_>>()
                                .join(" ")
                                .to_lowercase();
                            let display = crate::services::emails::best_account_for_person(db, eid, &email_context)
                                .unwrap_or(p.name);
                            entity_names.insert(eid.clone(), display);
                        } else if let Ok(Some(p)) = db.get_project(eid) {
                            entity_names.insert(eid.clone(), p.name);
                        }
                    }
                    Ok::<_, String>((rows, entity_names))
                }) {
                    DbTryRead::Ok(Ok((rows, entity_names))) if !rows.is_empty() => rows
                        .iter()
                        .map(|dbe| {
                            let entity_name = dbe.entity_id.as_ref()
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
                            commitments: Vec::new(),
                            questions: Vec::new(),
                            sentiment: dbe.sentiment.clone(),
                            urgency: dbe.urgency.clone(),
                            entity_id: dbe.entity_id.clone(),
                            entity_type: dbe.entity_type.clone(),
                            entity_name,
                            relevance_score: dbe.relevance_score,
                            score_reason: dbe.score_reason.clone(),
                        }})
                        .collect(),
                    _ => Vec::new(),
                };

            // I395: Sort by relevance score for briefing
            db_emails.sort_by(|a, b| {
                let sa = a.relevance_score.unwrap_or(-1.0);
                let sb = b.relevance_score.unwrap_or(-1.0);
                sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
            });

            if !db_emails.is_empty() {
                (Some(db_emails), None)
            } else {
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
                }
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

        // Load email narrative + replies_needed from directive (I355)
        let (email_narrative, replies_needed) = load_directive(&today_dir)
            .map(|d| (d.emails.narrative, d.emails.replies_needed))
            .unwrap_or_default();

        DashboardResult::Success {
            data: DashboardData {
                overview,
                stats,
                meetings,
                actions,
                emails,
                email_sync: email_sync.or_else(|| {
                    // Fall back to DB enrichment stats when JSON sync status absent (I373)
                    if let crate::state::DbTryRead::Ok(Ok(stats)) =
                        state.with_db_try_read(|db| db.get_email_sync_stats())
                    {
                        let last = stats.last_fetch_at?;
                        Some(EmailSyncStatus {
                            state: if stats.failed > 0 { EmailSyncState::Warning } else { EmailSyncState::Ok },
                            stage: EmailSyncStage::Enrich,
                            code: None,
                            message: Some(format!("{}/{} ready", stats.enriched, stats.total)),
                            using_last_known_good: None,
                            can_retry: if stats.failed > 0 { Some(true) } else { None },
                            last_attempt_at: Some(last.clone()),
                            last_success_at: Some(last),
                            enrichment_pending: Some(stats.pending as i64),
                            enrichment_enriched: Some(stats.enriched as i64),
                            enrichment_failed: Some(stats.failed as i64),
                            total_active: Some(stats.total as i64),
                        })
                    } else {
                        None
                    }
                }),
                focus,
                email_narrative,
                replies_needed,
                user_domains: {
                    let domains = config.resolved_user_domains();
                    if domains.is_empty() { None } else { Some(domains) }
                },
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

/// Get week overview data from workspace _today/data/ JSON files.
pub fn get_week_data(state: &AppState) -> WeekResult {
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

    let workspace = std::path::Path::new(&config.workspace_path);
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

    fn log_latency(command: &str, started: std::time::Instant, budget_ms: u128) {
        let elapsed_ms = started.elapsed().as_millis();
        crate::latency::record_latency(command, elapsed_ms, budget_ms);
        if elapsed_ms > budget_ms {
            log::warn!("{} exceeded latency budget: {}ms > {}ms", command, elapsed_ms, budget_ms);
        } else {
            log::debug!("{} completed in {}ms", command, elapsed_ms);
        }
    }
    log_latency("get_week_data", started, READ_CMD_LATENCY_BUDGET_MS);
    WeekResult::Success { data: week }
}
