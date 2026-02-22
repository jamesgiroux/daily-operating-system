// Meetings service — extracted from commands.rs
// Business logic for meeting intelligence assembly and entity operations.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::commands::{MeetingHistoryDetail, MeetingSearchResult, PrepContext};
use crate::db::ActionDb;
use crate::state::AppState;
use crate::types::CapturedOutcome;

/// Hydrate attendee context by matching calendar attendee emails to person entities.
/// Scoped to external (non-internal) attendees who are in the people database.
pub fn hydrate_attendee_context(
    db: &ActionDb,
    meeting: &crate::db::DbMeeting,
) -> Vec<crate::types::AttendeeContext> {
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
            if let Ok(Some(person)) = db.get_person_by_email_or_alias(email) {
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
pub fn person_to_attendee_context(person: &crate::db::DbPerson) -> crate::types::AttendeeContext {
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

/// Collect meeting outcomes (captures + actions) from DB for a meeting.
pub fn collect_meeting_outcomes_from_db(
    db: &ActionDb,
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

/// Load meeting prep from multiple sources: JSON file, frozen payload, or DB context.
pub fn load_meeting_prep_from_sources(
    today_dir: &Path,
    meeting: &crate::db::DbMeeting,
) -> Option<crate::types::FullMeetingPrep> {
    if let Ok(prep) = crate::json_loader::load_prep_json(today_dir, &meeting.id) {
        return Some(prep);
    }
    if let Some(ref frozen) = meeting.prep_frozen_json {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(frozen) {
            if let Some(prep_val) = value.get("prep") {
                if let Ok(prep) = serde_json::from_value::<crate::types::FullMeetingPrep>(prep_val.clone()) {
                    return Some(prep);
                }
            }
            if let Ok(prep) = serde_json::from_value::<crate::types::FullMeetingPrep>(value) {
                return Some(prep);
            }
        }
    }
    if let Some(ref prep_json) = meeting.prep_context_json {
        if let Ok(prep) = serde_json::from_str::<crate::types::FullMeetingPrep>(prep_json) {
            return Some(prep);
        }
    }
    None
}

/// Update a meeting entity with full cascade: clear existing links, set new one,
/// cascade to actions/captures/people, invalidate prep, queue intelligence refresh.
pub fn update_meeting_entity(
    state: &AppState,
    meeting_id: &str,
    entity_id: Option<&str>,
    entity_type: &str,
    meeting_title: &str,
    start_time: &str,
    meeting_type_str: &str,
) -> Result<(), String> {
    // Collect old entity IDs before modifying (for intelligence queue)
    let old_entity_ids: Vec<(String, String)>;

    {
        let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
        let db = db_guard.as_ref().ok_or("Database not initialized")?;

        old_entity_ids = db
            .get_meeting_entities(meeting_id)
            .unwrap_or_default()
            .into_iter()
            .map(|e| (e.id, e.entity_type.as_str().to_string()))
            .collect();

        // Ensure meeting exists without clobbering existing metadata.
        db.ensure_meeting_in_history(crate::db::EnsureMeetingHistoryInput {
            id: meeting_id,
            title: meeting_title,
            meeting_type: meeting_type_str,
            start_time,
            end_time: None,
            calendar_event_id: None,
            attendees: None,
            description: None,
        })
        .map_err(|e| e.to_string())?;

        // Clear all existing entity links
        db.clear_meeting_entities(meeting_id)
            .map_err(|e| e.to_string())?;

        // Determine account_id and project_id for cascade
        let (cascade_account, cascade_project) = match entity_type {
            "account" => (entity_id, None),
            "project" => (None, entity_id),
            _ => (entity_id, None),
        };

        // Link new entity if provided
        if let Some(eid) = entity_id {
            db.link_meeting_entity(meeting_id, eid, entity_type)
                .map_err(|e| e.to_string())?;
        }

        // Cascade to actions and captures
        db.cascade_meeting_entity_to_actions(meeting_id, cascade_account, cascade_project)
            .map_err(|e| e.to_string())?;
        db.cascade_meeting_entity_to_captures(meeting_id, cascade_account, cascade_project)
            .map_err(|e| e.to_string())?;

        // Cascade to people: link external attendees to the entity (I184)
        db.cascade_meeting_entity_to_people(meeting_id, cascade_account, cascade_project)
            .map_err(|e| e.to_string())?;

        // I305: Invalidate meeting prep so it regenerates with new entity intelligence
        if let Ok(Some(old_path)) = db.invalidate_meeting_prep(meeting_id) {
            let _ = std::fs::remove_file(&old_path);
        }
    }
    // DB lock released

    // I307: Record correction for learning when user changes entity assignment
    if !old_entity_ids.is_empty() {
        if let Some(new_id) = entity_id {
            let differs = old_entity_ids.iter().all(|(id, _)| id != new_id);
            if differs {
                if let Ok(db_guard) = state.db.lock() {
                    if let Some(db) = db_guard.as_ref() {
                        let _ = crate::signals::feedback::record_correction(
                            db, meeting_id, &old_entity_ids, new_id, entity_type,
                        );
                    }
                }
            }
        }
    }

    // I307: Auto-extract title keywords for the corrected entity.
    if let Some(new_id) = entity_id {
        if entity_type == "account" || entity_type == "project" {
            if let Ok(db_guard) = state.db.lock() {
                if let Some(db) = db_guard.as_ref() {
                    let _ = crate::services::entities::auto_extract_title_keywords(
                        db, new_id, entity_type, meeting_title,
                    );
                }
            }
        }
    }

    // I305: Queue prep regeneration
    if let Ok(mut queue) = state.prep_invalidation_queue.lock() {
        queue.push(meeting_id.to_string());
    }

    // Queue intelligence refresh for old and new entities
    let mut entities_to_refresh: Vec<(String, String)> = old_entity_ids;
    if let Some(eid) = entity_id {
        entities_to_refresh.push((eid.to_string(), entity_type.to_string()));
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

/// Add an entity link to a meeting with full cascade (people, intelligence).
/// Unlike `update_meeting_entity` which clears-and-replaces, this is additive.
pub fn add_meeting_entity(
    state: &AppState,
    meeting_id: &str,
    entity_id: &str,
    entity_type: &str,
    meeting_title: &str,
    start_time: &str,
    meeting_type_str: &str,
) -> Result<(), String> {
    {
        let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
        let db = db_guard.as_ref().ok_or("Database not initialized")?;

        // Ensure meeting exists without clobbering existing metadata.
        db.ensure_meeting_in_history(crate::db::EnsureMeetingHistoryInput {
            id: meeting_id,
            title: meeting_title,
            meeting_type: meeting_type_str,
            start_time,
            end_time: None,
            calendar_event_id: None,
            attendees: None,
            description: None,
        })
        .map_err(|e| e.to_string())?;

        // Add entity link (idempotent)
        db.link_meeting_entity(meeting_id, entity_id, entity_type)
            .map_err(|e| e.to_string())?;

        // Cascade people to this entity
        let (cascade_account, cascade_project) = match entity_type {
            "account" => (Some(entity_id), None),
            "project" => (None, Some(entity_id)),
            _ => (Some(entity_id), None),
        };
        db.cascade_meeting_entity_to_people(meeting_id, cascade_account, cascade_project)
            .map_err(|e| e.to_string())?;

        // I305: Invalidate meeting prep so it regenerates with new entity intelligence
        if let Ok(Some(old_path)) = db.invalidate_meeting_prep(meeting_id) {
            let _ = std::fs::remove_file(&old_path);
        }
    }
    // DB lock released

    // I305: Queue prep regeneration
    if let Ok(mut queue) = state.prep_invalidation_queue.lock() {
        queue.push(meeting_id.to_string());
    }

    // Queue intelligence refresh
    state.intel_queue.enqueue(crate::intel_queue::IntelRequest {
        entity_id: entity_id.to_string(),
        entity_type: entity_type.to_string(),
        priority: crate::intel_queue::IntelPriority::CalendarChange,
        requested_at: std::time::Instant::now(),
    });

    Ok(())
}

/// Remove an entity link from a meeting with cleanup (legacy account_id, intelligence).
pub fn remove_meeting_entity(
    state: &AppState,
    meeting_id: &str,
    entity_id: &str,
    entity_type: &str,
) -> Result<(), String> {
    {
        let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
        let db = db_guard.as_ref().ok_or("Database not initialized")?;

        // I307: Record removal as correction for learning
        let _ = crate::signals::feedback::record_removal(
            db, meeting_id, entity_id, entity_type,
        );

        db.unlink_meeting_entity(meeting_id, entity_id)
            .map_err(|e| e.to_string())?;

        // I305: Invalidate meeting prep so it regenerates with new entity intelligence
        if let Ok(Some(old_path)) = db.invalidate_meeting_prep(meeting_id) {
            let _ = std::fs::remove_file(&old_path);
        }
    }
    // DB lock released

    // I305: Queue prep regeneration
    if let Ok(mut queue) = state.prep_invalidation_queue.lock() {
        queue.push(meeting_id.to_string());
    }

    // Queue intelligence refresh for removed entity
    state.intel_queue.enqueue(crate::intel_queue::IntelRequest {
        entity_id: entity_id.to_string(),
        entity_type: entity_type.to_string(),
        priority: crate::intel_queue::IntelPriority::CalendarChange,
        requested_at: std::time::Instant::now(),
    });

    Ok(())
}

/// Get full detail for a single past meeting by ID.
///
/// Assembles the meeting row, its captures, actions, and resolves the account name.
pub fn get_meeting_history_detail(
    meeting_id: &str,
    state: &AppState,
) -> Result<MeetingHistoryDetail, String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let meeting = db
        .get_meeting_by_id(meeting_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Meeting not found: {meeting_id}"))?;

    let captures = db
        .get_captures_for_meeting(meeting_id)
        .map_err(|e| e.to_string())?;

    let actions = db
        .get_actions_for_meeting(meeting_id)
        .map_err(|e| e.to_string())?;

    // Resolve account name from junction table
    let (linked_account_id, account_name) = db
        .get_meeting_entities(meeting_id)
        .ok()
        .and_then(|ents| {
            ents.into_iter()
                .find(|e| e.entity_type == crate::entity::EntityType::Account)
        })
        .map(|e| (Some(e.id), Some(e.name)))
        .unwrap_or((None, None));

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
        account_id: linked_account_id,
        account_name,
        summary: meeting.summary,
        attendees,
        captures,
        actions,
        prep_context,
    })
}

/// Search meetings by title, summary, or prep context (I183).
pub fn search_meetings(
    query: &str,
    state: &AppState,
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

/// Capture meeting outcomes (actions, wins, risks) from post-meeting capture UI.
pub fn capture_meeting_outcome(
    outcome: &CapturedOutcome,
    state: &AppState,
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
                account_name: None,
                next_meeting_title: None,
                next_meeting_start: None,
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

/// Get meeting timeline for the week view (past + upcoming meetings with intelligence quality).
pub fn get_meeting_timeline(
    state: &AppState,
    days_before: Option<i64>,
    days_after: Option<i64>,
) -> Result<Vec<crate::types::TimelineMeeting>, String> {
    let days_before = days_before.unwrap_or(7);
    let days_after = days_after.unwrap_or(7);
    log::info!("get_meeting_timeline: +/-{}/{} days", days_before, days_after);

    let today = chrono::Local::now().date_naive();
    let range_start = today - chrono::Duration::days(days_before);
    let range_end = today + chrono::Duration::days(days_after);
    let start_str = range_start.format("%Y-%m-%d").to_string();
    let end_str = format!("{}T23:59:59", range_end.format("%Y-%m-%d"));

    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let conn = db.conn_ref();

    // Query meetings in the date range
    let mut stmt = conn
        .prepare(
            "SELECT id, title, meeting_type, start_time, end_time, summary,
                    transcript_processed_at, has_new_signals,
                    (prep_frozen_json IS NOT NULL) as has_frozen_prep
             FROM meetings_history
             WHERE start_time >= ?1 AND start_time <= ?2
               AND (intelligence_state IS NULL OR intelligence_state != 'archived')
               AND meeting_type NOT IN ('personal', 'focus', 'blocked')
             ORDER BY start_time ASC",
        )
        .map_err(|e| format!("Failed to prepare timeline query: {}", e))?;

    struct RawMeeting {
        id: String,
        title: String,
        meeting_type: String,
        start_time: String,
        end_time: Option<String>,
        summary: Option<String>,
        transcript_processed_at: Option<String>,
        has_new_signals: Option<i32>,
        has_frozen_prep: bool,
    }

    let raw_meetings: Vec<RawMeeting> = stmt
        .query_map(rusqlite::params![start_str, end_str], |row| {
            Ok(RawMeeting {
                id: row.get(0)?,
                title: row.get(1)?,
                meeting_type: row.get(2)?,
                start_time: row.get(3)?,
                end_time: row.get(4)?,
                summary: row.get(5)?,
                transcript_processed_at: row.get(6)?,
                has_new_signals: row.get(7)?,
                has_frozen_prep: row.get::<_, i32>(8).unwrap_or(0) != 0,
            })
        })
        .map_err(|e| format!("Failed to query timeline: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    log::info!("get_meeting_timeline: {} raw meetings found", raw_meetings.len());
    if raw_meetings.is_empty() {
        return Ok(Vec::new());
    }

    // Batch fetch linked entities for all meetings
    let meeting_ids: Vec<String> = raw_meetings.iter().map(|m| m.id.clone()).collect();
    let entity_map = db
        .get_meeting_entity_map(&meeting_ids)
        .unwrap_or_default();

    // Check for captures per meeting (batch)
    let capture_placeholders: Vec<String> = (0..meeting_ids.len())
        .map(|i| format!("?{}", i + 1))
        .collect();
    let capture_sql = format!(
        "SELECT meeting_id, COUNT(*) FROM captures WHERE meeting_id IN ({}) GROUP BY meeting_id",
        capture_placeholders.join(", ")
    );
    let mut capture_stmt = conn
        .prepare(&capture_sql)
        .map_err(|e| format!("Failed to prepare captures query: {}", e))?;
    let capture_params: Vec<&dyn rusqlite::types::ToSql> = meeting_ids
        .iter()
        .map(|id| id as &dyn rusqlite::types::ToSql)
        .collect();
    let capture_counts: HashMap<String, i64> = capture_stmt
        .query_map(capture_params.as_slice(), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|e| format!("Failed to query captures: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    // Count follow-up actions per meeting (I342)
    let action_count_sql = format!(
        "SELECT source_id, COUNT(*) FROM actions WHERE source_id IN ({}) GROUP BY source_id",
        capture_placeholders.join(", ")
    );
    let mut action_count_stmt = conn
        .prepare(&action_count_sql)
        .map_err(|e| format!("Failed to prepare action count query: {}", e))?;
    let action_count_params: Vec<&dyn rusqlite::types::ToSql> = meeting_ids
        .iter()
        .map(|id| id as &dyn rusqlite::types::ToSql)
        .collect();
    let action_counts: HashMap<String, i32> = action_count_stmt
        .query_map(action_count_params.as_slice(), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?))
        })
        .map_err(|e| format!("Failed to query action counts: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    // Build timeline meetings
    let mut result: Vec<crate::types::TimelineMeeting> = Vec::with_capacity(raw_meetings.len());
    for m in &raw_meetings {
        // Intelligence quality assessment (skip on error)
        let quality = match crate::intelligence::assess_intelligence_quality(db, &m.id) {
            q if q.level == crate::types::QualityLevel::Sparse
                && q.signal_count == 0
                && !q.has_entity_context =>
            {
                // Minimal quality — still include it
                Some(q)
            }
            q => Some(q),
        };

        let capture_count = capture_counts.get(&m.id).copied().unwrap_or(0);
        let has_outcomes =
            capture_count > 0 || m.transcript_processed_at.is_some();

        let outcome_summary = if has_outcomes {
            m.summary.clone()
        } else {
            None
        };

        let entities = entity_map.get(&m.id).cloned().unwrap_or_default();

        let has_new_signals = m.has_new_signals.unwrap_or(0) != 0;

        // Find prior meeting: most recent earlier meeting sharing at least one entity
        let prior_meeting_id = if !entities.is_empty() {
            let entity_ids: Vec<&str> = entities.iter().map(|e| e.id.as_str()).collect();
            find_prior_meeting(conn, &m.id, &m.start_time, &entity_ids)
        } else {
            None
        };

        let follow_up_count = action_counts.get(&m.id).copied();

        result.push(crate::types::TimelineMeeting {
            id: m.id.clone(),
            title: m.title.clone(),
            start_time: m.start_time.clone(),
            end_time: m.end_time.clone(),
            meeting_type: m.meeting_type.clone(),
            // has_prep: true if frozen prep exists OR intelligence quality is above sparse
            has_prep: m.has_frozen_prep || quality.as_ref().is_some_and(|q| {
                !matches!(q.level, crate::types::QualityLevel::Sparse)
            }),
            intelligence_quality: quality,
            has_outcomes,
            outcome_summary,
            entities,
            has_new_signals,
            prior_meeting_id,
            follow_up_count,
        });
    }

    Ok(result)
}

/// Find the most recent past meeting that shares at least one entity with the current meeting.
fn find_prior_meeting(
    conn: &rusqlite::Connection,
    current_meeting_id: &str,
    current_start_time: &str,
    entity_ids: &[&str],
) -> Option<String> {
    if entity_ids.is_empty() {
        return None;
    }
    let placeholders: Vec<String> = (0..entity_ids.len())
        .map(|i| format!("?{}", i + 3))
        .collect();
    let sql = format!(
        "SELECT DISTINCT mh.id FROM meetings_history mh
         INNER JOIN meeting_entities me ON me.meeting_id = mh.id
         WHERE me.entity_id IN ({})
           AND mh.start_time < ?1
           AND mh.id != ?2
         ORDER BY mh.start_time DESC
         LIMIT 1",
        placeholders.join(", ")
    );
    let mut stmt = conn.prepare(&sql).ok()?;
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    params.push(Box::new(current_start_time.to_string()));
    params.push(Box::new(current_meeting_id.to_string()));
    for eid in entity_ids {
        params.push(Box::new(eid.to_string()));
    }
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    stmt.query_row(param_refs.as_slice(), |row| row.get::<_, String>(0))
        .ok()
}
