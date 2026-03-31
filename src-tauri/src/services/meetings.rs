// Meetings service — extracted from commands.rs
// Business logic for meeting intelligence assembly and entity operations.

use chrono::TimeZone;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use tauri::Emitter;

use crate::commands::{MeetingHistoryDetail, MeetingSearchResult, PrepContext};
use crate::db::ActionDb;
use crate::state::AppState;
use crate::types::{CapturedOutcome, IntelligenceQuality, MeetingIntelligence};

pub fn upsert_meeting_for_reconcile(
    db: &ActionDb,
    meeting: &crate::db::DbMeeting,
) -> Result<(), String> {
    db.with_transaction(|tx| {
        tx.upsert_meeting(meeting).map_err(|e| e.to_string())?;
        crate::services::signals::emit(
            tx,
            "meeting",
            &meeting.id,
            "meeting_upserted",
            "reconcile",
            None,
            0.7,
        )
        .map_err(|e| format!("signal emit failed: {e}"))?;
        Ok(())
    })
}

pub fn set_meeting_prep_context(
    db: &ActionDb,
    meeting_id: &str,
    updated_json: &str,
) -> Result<(), String> {
    db.update_meeting_prep_context(meeting_id, updated_json)
        .map_err(|e| e.to_string())
}

pub fn update_capture_content(
    db: &ActionDb,
    capture_id: &str,
    content: &str,
) -> Result<(), String> {
    db.with_transaction(|tx| {
        tx.update_capture(capture_id, content)
            .map_err(|e| e.to_string())?;
        crate::services::signals::emit(
            tx,
            "capture",
            capture_id,
            "capture_updated",
            "user_edit",
            None,
            0.8,
        )
        .map_err(|e| format!("signal emit failed: {e}"))?;
        Ok(())
    })
}

pub fn clear_meeting_prep_frozen(db: &ActionDb, meeting_id: &str) -> Result<(), String> {
    db.with_transaction(|tx| {
        tx.conn_ref()
            .execute(
                "UPDATE meeting_prep SET prep_frozen_json = NULL, prep_frozen_at = NULL WHERE meeting_id = ?1",
                rusqlite::params![meeting_id],
            )
            .map_err(|e| e.to_string())?;
        crate::services::signals::emit(
            tx,
            "meeting",
            meeting_id,
            "prep_invalidated",
            "ai_enrichment",
            None,
            0.7,
        )
        .map_err(|e| format!("signal emit failed: {e}"))?;
        Ok(())
    })
}

/// Hydrate attendee context by matching calendar attendee emails to person entities.
///
/// For external meetings: scoped to non-internal attendees (customers, prospects, etc.).
/// For internal meetings (team_sync, internal, one_on_one): includes all attendees,
/// since the room IS internal colleagues (I401).
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

    // I401: Internal meetings show internal attendees — the room IS your team.
    // External meetings filter out internal colleagues to focus on the customer.
    let is_internal_meeting = matches!(
        meeting.meeting_type.as_str(),
        "team_sync" | "internal" | "one_on_one"
    );

    if is_internal_meeting {
        contexts
    } else {
        contexts
            .into_iter()
            .filter(|ctx| ctx.relationship.as_deref() != Some("internal"))
            .collect()
    }
}

/// Convert a DbPerson into an AttendeeContext with computed temperature.
pub fn person_to_attendee_context(person: &crate::db::DbPerson) -> crate::types::AttendeeContext {
    let temperature = person.last_seen.as_deref().map(|ls| {
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

/// Build outcomes from already-fetched captures + actions (avoids duplicate DB queries).
fn build_outcomes_from_data(
    meeting: &crate::db::DbMeeting,
    captures: &[crate::db::DbCapture],
    actions: &[crate::db::DbAction],
) -> Option<crate::types::MeetingOutcomeData> {
    let mut wins = Vec::new();
    let mut risks = Vec::new();
    let mut decisions = Vec::new();
    for cap in captures {
        match cap.capture_type.as_str() {
            "win" => wins.push(cap.content.clone()),
            "risk" => risks.push(cap.content.clone()),
            "decision" => decisions.push(cap.content.clone()),
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
        actions: actions.to_vec(),
        transcript_path: meeting.transcript_path.clone(),
        processed_at: meeting.transcript_processed_at.clone(),
    })
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

/// Load meeting prep from DB sources (mechanical assembly).
pub fn load_meeting_prep_from_sources(
    _today_dir: &Path,
    meeting: &crate::db::DbMeeting,
) -> Option<crate::types::FullMeetingPrep> {
    // Source 1: prep_frozen_json — mechanical assembly from entity intelligence (ADR-0086)
    if let Some(ref frozen) = meeting.prep_frozen_json {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(frozen) {
            if let Some(prep_val) = value.get("prep") {
                if let Ok(prep) =
                    serde_json::from_value::<crate::types::FullMeetingPrep>(prep_val.clone())
                {
                    return Some(prep);
                }
            }
            if let Ok(prep) = serde_json::from_value::<crate::types::FullMeetingPrep>(value) {
                return Some(prep);
            }
        }
    }
    let rebuild_in_progress = meeting.prep_frozen_json.is_none()
        && matches!(
            meeting.intelligence_state.as_deref(),
            Some("refreshing") | Some("enriching")
        );
    if rebuild_in_progress {
        return None;
    }
    // Source 2: prep_context_json from DB (I513 — no disk prep file fallback)
    if let Some(ref prep_json) = meeting.prep_context_json {
        // Try direct deserialization first
        if let Ok(prep) = serde_json::from_str::<crate::types::FullMeetingPrep>(prep_json) {
            return Some(prep);
        }
        // Fallback: if prep_context_json was overwritten with AI-schema
        // ({"quality": ..., "ai_intelligence": ...}), extract what we can
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(prep_json) {
            if let Some(ai) = value.get("ai_intelligence") {
                let mut prep = crate::types::FullMeetingPrep {
                    file_path: String::new(),
                    calendar_event_id: meeting.calendar_event_id.clone(),
                    title: meeting.title.clone(),
                    time_range: meeting.start_time.clone(),
                    meeting_context: None,
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
                    proposed_agenda: None,
                    intelligence_summary: None,
                    entity_risks: None,
                    entity_readiness: None,
                    stakeholder_insights: None,
                    recent_email_signals: None,
                    email_digest: None,
                    consistency_status: None,
                    consistency_findings: Vec::new(),
                };
                // Extract AI narrative into intelligenceSummary
                if let Some(narrative) = ai.get("narrative").and_then(|v| v.as_str()) {
                    prep.intelligence_summary = Some(narrative.to_string());
                }
                if let Some(ctx) = ai
                    .get("meetingContext")
                    .or_else(|| ai.get("meeting_context"))
                    .and_then(|v| v.as_str())
                {
                    prep.meeting_context = Some(ctx.to_string());
                }
                if let Some(risks) = ai.get("risks").and_then(|v| v.as_array()) {
                    prep.risks = Some(
                        risks
                            .iter()
                            .filter_map(|r| r.as_str().map(|s| s.to_string()))
                            .collect(),
                    );
                }
                if let Some(tp) = ai
                    .get("talkingPoints")
                    .or_else(|| ai.get("talking_points"))
                    .and_then(|v| v.as_array())
                {
                    prep.talking_points = Some(
                        tp.iter()
                            .filter_map(|r| r.as_str().map(|s| s.to_string()))
                            .collect(),
                    );
                }
                return Some(prep);
            }
        }
    }

    None
}

#[derive(Debug, Clone)]
enum MeetingEntityMutation {
    Replace {
        entity_id: Option<String>,
        entity_type: String,
        meeting_title: String,
        start_time: String,
        meeting_type: String,
    },
    Add {
        entity_id: String,
        entity_type: String,
        meeting_title: String,
        start_time: String,
        meeting_type: String,
    },
    Remove {
        entity_id: String,
        entity_type: String,
    },
}

#[derive(Debug, Default)]
struct MeetingEntityMutationOutcome {
    old_entity_ids: Vec<(String, String)>,
    entities_to_refresh: Vec<(String, String)>,
    correction_target: Option<(String, String)>,
    keyword_target: Option<(String, String, String)>,
}

fn cascade_targets<'a>(
    entity_id: Option<&'a str>,
    entity_type: &str,
) -> (Option<&'a str>, Option<&'a str>) {
    match entity_type {
        "account" => (entity_id, None),
        "project" => (None, entity_id),
        _ => (entity_id, None),
    }
}

/// Single orchestration path for meeting-entity mutations.
///
/// Performs mutation, prep invalidation, immediate mechanical rebuild, and
/// async entity intelligence refresh queuing. Falls back to prep queue when
/// immediate rebuild fails. Emits `prep-ready` event on successful rebuild
/// so the frontend auto-refreshes (I477).
async fn mutate_meeting_entities_and_refresh_briefing(
    state: &AppState,
    meeting_id: &str,
    mutation: MeetingEntityMutation,
    app_handle: Option<&tauri::AppHandle>,
) -> Result<(), String> {
    let meeting_id_s = meeting_id.to_string();

    let mutation_result = state
        .db_write({
            let meeting_id = meeting_id_s.clone();
            let mutation = mutation.clone();
            move |db| {
                let mut result = MeetingEntityMutationOutcome {
                    old_entity_ids: db
                        .get_meeting_entities(&meeting_id)
                        .map_err(|e| e.to_string())?
                        .into_iter()
                        .map(|e| (e.id, e.entity_type.as_str().to_string()))
                        .collect(),
                    ..Default::default()
                };

                match mutation {
                    MeetingEntityMutation::Replace {
                        entity_id,
                        entity_type,
                        meeting_title,
                        start_time,
                        meeting_type,
                    } => {
                        db.ensure_meeting_in_history(crate::db::EnsureMeetingHistoryInput {
                            id: &meeting_id,
                            title: &meeting_title,
                            meeting_type: &meeting_type,
                            start_time: &start_time,
                            end_time: None,
                            calendar_event_id: None,
                            attendees: None,
                            description: None,
                        })
                        .map_err(|e| e.to_string())?;

                        db.clear_meeting_entities(&meeting_id)
                            .map_err(|e| e.to_string())?;

                        if let Some(ref eid) = entity_id {
                            db.link_meeting_entity(&meeting_id, eid, &entity_type)
                                .map_err(|e| e.to_string())?;
                        }

                        let (cascade_account, cascade_project) =
                            cascade_targets(entity_id.as_deref(), &entity_type);
                        db.cascade_meeting_entity_to_actions(
                            &meeting_id,
                            cascade_account,
                            cascade_project,
                        )
                        .map_err(|e| e.to_string())?;
                        db.cascade_meeting_entity_to_captures(
                            &meeting_id,
                            cascade_account,
                            cascade_project,
                        )
                        .map_err(|e| e.to_string())?;
                        db.cascade_meeting_entity_to_people(
                            &meeting_id,
                            cascade_account,
                            cascade_project,
                        )
                        .map_err(|e| e.to_string())?;

                        result
                            .entities_to_refresh
                            .extend(result.old_entity_ids.clone());

                        if let Some(ref eid) = entity_id {
                            result
                                .entities_to_refresh
                                .push((eid.clone(), entity_type.clone()));
                            result.correction_target = Some((eid.clone(), entity_type.clone()));
                            if entity_type == "account" || entity_type == "project" {
                                result.keyword_target =
                                    Some((eid.clone(), entity_type, meeting_title));
                            }
                        }
                    }
                    MeetingEntityMutation::Add {
                        entity_id,
                        entity_type,
                        meeting_title,
                        start_time,
                        meeting_type,
                    } => {
                        db.ensure_meeting_in_history(crate::db::EnsureMeetingHistoryInput {
                            id: &meeting_id,
                            title: &meeting_title,
                            meeting_type: &meeting_type,
                            start_time: &start_time,
                            end_time: None,
                            calendar_event_id: None,
                            attendees: None,
                            description: None,
                        })
                        .map_err(|e| e.to_string())?;

                        db.link_meeting_entity(&meeting_id, &entity_id, &entity_type)
                            .map_err(|e| e.to_string())?;

                        let (cascade_account, cascade_project) =
                            cascade_targets(Some(entity_id.as_str()), &entity_type);
                        db.cascade_meeting_entity_to_people(
                            &meeting_id,
                            cascade_account,
                            cascade_project,
                        )
                        .map_err(|e| e.to_string())?;

                        result.entities_to_refresh.push((entity_id, entity_type));
                    }
                    MeetingEntityMutation::Remove {
                        entity_id,
                        entity_type,
                    } => {
                        let _ = crate::signals::feedback::record_removal(
                            db,
                            &meeting_id,
                            &entity_id,
                            &entity_type,
                        );
                        db.unlink_meeting_entity(&meeting_id, &entity_id)
                            .map_err(|e| e.to_string())?;
                        result.entities_to_refresh.push((entity_id, entity_type));
                    }
                }

                if let Ok(Some(old_path)) = db.invalidate_meeting_prep(&meeting_id) {
                    let _ = std::fs::remove_file(&old_path);
                }
                let _ = db.update_intelligence_state(&meeting_id, "refreshing", None, None);

                Ok::<MeetingEntityMutationOutcome, String>(result)
            }
        })
        .await?;

    if mutation_result.correction_target.is_some() || mutation_result.keyword_target.is_some() {
        let meeting_id = meeting_id_s.clone();
        let old_ids = mutation_result.old_entity_ids.clone();
        let correction_target = mutation_result.correction_target.clone();
        let keyword_target = mutation_result.keyword_target.clone();
        let _ = state
            .db_write(move |db| {
                if let Some((new_id, entity_type)) = correction_target {
                    if !old_ids.is_empty() && old_ids.iter().all(|(id, _)| id != &new_id) {
                        let _ = crate::signals::feedback::record_correction(
                            db,
                            &meeting_id,
                            &old_ids,
                            &new_id,
                            &entity_type,
                        );
                    }
                }

                if let Some((entity_id, entity_type, meeting_title)) = keyword_target {
                    if entity_type == "account" || entity_type == "project" {
                        let _ = crate::services::entities::auto_extract_title_keywords(
                            db,
                            &entity_id,
                            &entity_type,
                            &meeting_title,
                        );
                    }
                }
                Ok::<(), String>(())
            })
            .await;
    }

    let mut entities_to_refresh = mutation_result.entities_to_refresh;
    entities_to_refresh.sort();
    entities_to_refresh.dedup();
    if !entities_to_refresh.is_empty() {
        for (entity_id, entity_type) in entities_to_refresh {
            state
                .intel_queue
                .enqueue(crate::intel_queue::IntelRequest::new(
                    entity_id,
                    entity_type,
                    crate::intel_queue::IntelPriority::CalendarChange,
                ));
        }
        state.integrations.intel_queue_wake.notify_one();
    }

    let prep_rebuilt_sync = match crate::meeting_prep_queue::meeting_prep_blocking_inputs(state) {
        Ok((workspace, embedding_model)) => match tauri::async_runtime::spawn_blocking({
            let meeting_id = meeting_id_s.clone();
            move || {
                crate::meeting_prep_queue::generate_mechanical_prep_now_blocking(
                    workspace,
                    embedding_model,
                    meeting_id,
                )
            }
        })
        .await
        {
            Ok(Ok(())) => true,
            Ok(Err(err)) => {
                log::warn!(
                    "mutate_meeting_entities_and_refresh_briefing: immediate prep rebuild failed for {}: {}",
                    meeting_id_s,
                    err
                );
                false
            }
            Err(err) => {
                log::warn!(
                    "mutate_meeting_entities_and_refresh_briefing: prep rebuild task panicked for {}: {}",
                    meeting_id_s,
                    err
                );
                false
            }
        },
        Err(err) => {
            log::warn!(
                "mutate_meeting_entities_and_refresh_briefing: prep rebuild inputs failed for {}: {}",
                meeting_id_s,
                err
            );
            false
        }
    };

    if prep_rebuilt_sync {
        let meeting_id = meeting_id_s.clone();
        let _ = state
            .db_write(move |db| {
                let quality = crate::intelligence::assess_intelligence_quality(db, &meeting_id);
                db.update_intelligence_state(
                    &meeting_id,
                    "enriched",
                    Some(&quality.level.to_string()),
                    Some(quality.signal_count as i32),
                )
                .map_err(|e| e.to_string())?;
                Ok::<(), String>(())
            })
            .await;

        // Emit prep-ready so MeetingDetailPage auto-refreshes (I477).
        if let Some(app) = app_handle {
            let _ = app.emit(
                "prep-ready",
                crate::meeting_prep_queue::PrepReadyPayload {
                    meeting_id: meeting_id_s,
                },
            );
        }
    } else {
        // Fallback: enqueue for background rebuild. The background processor
        // will emit prep-ready when it completes.
        state
            .meeting_prep_queue
            .enqueue(crate::meeting_prep_queue::PrepRequest::new(
                meeting_id_s,
                crate::meeting_prep_queue::PrepPriority::Manual,
            ));
        state.integrations.prep_queue_wake.notify_one();
    }

    Ok(())
}

/// Context for meeting-entity mutation commands (I477).
///
/// Groups common meeting metadata fields to keep function signatures
/// within clippy's 7-argument limit.
pub struct MeetingMutationCtx<'a> {
    pub state: &'a AppState,
    pub meeting_id: &'a str,
    pub app_handle: Option<&'a tauri::AppHandle>,
}

/// Update a meeting entity with full cascade: clear existing links, set new one,
/// cascade to actions/captures/people, invalidate prep, then rebuild prep.
pub async fn update_meeting_entity(
    ctx: MeetingMutationCtx<'_>,
    entity_id: Option<&str>,
    entity_type: &str,
    meeting_title: &str,
    start_time: &str,
    meeting_type_str: &str,
) -> Result<(), String> {
    mutate_meeting_entities_and_refresh_briefing(
        ctx.state,
        ctx.meeting_id,
        MeetingEntityMutation::Replace {
            entity_id: entity_id.map(|s| s.to_string()),
            entity_type: entity_type.to_string(),
            meeting_title: meeting_title.to_string(),
            start_time: start_time.to_string(),
            meeting_type: meeting_type_str.to_string(),
        },
        ctx.app_handle,
    )
    .await
}

/// Add an entity link to a meeting with full cascade (people, intelligence).
/// Unlike `update_meeting_entity` which clears-and-replaces, this is additive.
pub async fn add_meeting_entity(
    ctx: MeetingMutationCtx<'_>,
    entity_id: &str,
    entity_type: &str,
    meeting_title: &str,
    start_time: &str,
    meeting_type_str: &str,
) -> Result<(), String> {
    mutate_meeting_entities_and_refresh_briefing(
        ctx.state,
        ctx.meeting_id,
        MeetingEntityMutation::Add {
            entity_id: entity_id.to_string(),
            entity_type: entity_type.to_string(),
            meeting_title: meeting_title.to_string(),
            start_time: start_time.to_string(),
            meeting_type: meeting_type_str.to_string(),
        },
        ctx.app_handle,
    )
    .await
}

/// Remove an entity link from a meeting with cleanup (legacy account_id, intelligence).
pub async fn remove_meeting_entity(
    ctx: MeetingMutationCtx<'_>,
    entity_id: &str,
    entity_type: &str,
) -> Result<(), String> {
    mutate_meeting_entities_and_refresh_briefing(
        ctx.state,
        ctx.meeting_id,
        MeetingEntityMutation::Remove {
            entity_id: entity_id.to_string(),
            entity_type: entity_type.to_string(),
        },
        ctx.app_handle,
    )
    .await
}

/// Get full detail for a single past meeting by ID.
///
/// Assembles the meeting row, its captures, actions, and resolves the account name.
pub async fn get_meeting_history_detail(
    meeting_id: &str,
    state: &AppState,
) -> Result<MeetingHistoryDetail, String> {
    let meeting_id = meeting_id.to_string();
    state
        .db_read(move |db| {
            let meeting_id = meeting_id.as_str();

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
        })
        .await
}

/// Search meetings by title, summary, or prep context (I183).
pub async fn search_meetings(
    query: &str,
    state: &AppState,
) -> Result<Vec<MeetingSearchResult>, String> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }

    let query = query.to_string();
    state
        .db_read(move |db| {
            let pattern = format!("%{}%", query.as_str().trim());
            let mut stmt = db
                .conn_ref()
                .prepare(
                    "SELECT m.id, m.title, m.meeting_type, m.start_time,
                    mt.summary, mp.prep_context_json
             FROM meetings m
             LEFT JOIN meeting_transcripts mt ON mt.meeting_id = m.id
             LEFT JOIN meeting_prep mp ON mp.meeting_id = m.id
             WHERE m.title LIKE ?1
                OR mt.summary LIKE ?1
                OR mp.prep_context_json LIKE ?1
             ORDER BY m.start_time DESC
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
                    ))
                })
                .map_err(|e| e.to_string())?;

            let mut results = Vec::new();
            for row in rows {
                let (id, title, meeting_type, start_time, summary, prep_json) =
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

                // Resolve account name from meeting_entities junction table
                let account_name = db
                    .get_meeting_entities(&id)
                    .ok()
                    .and_then(|ents| {
                        ents.into_iter()
                            .find(|e| e.entity_type == crate::entity::EntityType::Account)
                    })
                    .map(|e| e.name);

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
        })
        .await
}

/// Capture meeting outcomes (actions, wins, risks) from post-meeting capture UI.
pub async fn capture_meeting_outcome(
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
    if let Ok(mut guard) = state.capture.captured.lock() {
        guard.insert(outcome.meeting_id.clone());
    }

    // Persist actions and captures to SQLite
    let outcome_clone = outcome.clone();
    let _ = state
        .db_write(move |db| {
            // Resolve stable entity IDs so post-meeting actions/captures attach even when
            // the UI payload only has display names.
            let linked_entities = db
                .get_meeting_entities(&outcome_clone.meeting_id)
                .unwrap_or_default();

            let mut resolved_account_id = linked_entities
                .iter()
                .find(|e| e.entity_type == crate::entity::EntityType::Account)
                .map(|e| e.id.clone());
            let mut resolved_project_id = linked_entities
                .iter()
                .find(|e| e.entity_type == crate::entity::EntityType::Project)
                .map(|e| e.id.clone());

            if let Some(raw_entity) = outcome_clone.account.as_deref() {
                if resolved_account_id.is_none() {
                    resolved_account_id = db
                        .get_account(raw_entity)
                        .ok()
                        .flatten()
                        .map(|a| a.id)
                        .or_else(|| {
                            db.get_account_by_name(raw_entity)
                                .ok()
                                .flatten()
                                .map(|a| a.id)
                        });
                }

                if resolved_project_id.is_none() {
                    resolved_project_id = db
                        .get_project(raw_entity)
                        .ok()
                        .flatten()
                        .map(|p| p.id)
                        .or_else(|| {
                            db.get_project_by_name(raw_entity)
                                .ok()
                                .flatten()
                                .map(|p| p.id)
                        });
                }
            }

            for action in &outcome_clone.actions {
                let now = chrono::Utc::now().to_rfc3339();
                let db_action = crate::db::DbAction {
                    id: uuid::Uuid::new_v4().to_string(),
                    title: action.title.clone(),
                    priority: "P2".to_string(),
                    status: "suggested".to_string(),
                    created_at: now.clone(),
                    due_date: action.due_date.clone(),
                    completed_at: None,
                    account_id: resolved_account_id.clone(),
                    project_id: resolved_project_id.clone(),
                    source_type: Some("post_meeting".to_string()),
                    source_id: Some(outcome_clone.meeting_id.clone()),
                    source_label: Some(outcome_clone.meeting_title.clone()),
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

            for win in &outcome_clone.wins {
                let _ = db.insert_capture_with_project(
                    &outcome_clone.meeting_id,
                    &outcome_clone.meeting_title,
                    resolved_account_id.as_deref(),
                    resolved_project_id.as_deref(),
                    "win",
                    win,
                );
            }
            for risk in &outcome_clone.risks {
                let _ = db.insert_capture_with_project(
                    &outcome_clone.meeting_id,
                    &outcome_clone.meeting_title,
                    resolved_account_id.as_deref(),
                    resolved_project_id.as_deref(),
                    "risk",
                    risk,
                );
            }
            Ok(())
        })
        .await;

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
pub async fn get_meeting_timeline(
    state: &AppState,
    days_before: Option<i64>,
    days_after: Option<i64>,
) -> Result<Vec<crate::types::TimelineMeeting>, String> {
    let days_before = days_before.unwrap_or(7);
    let days_after = days_after.unwrap_or(7);
    log::info!(
        "get_meeting_timeline: +/-{}/{} days",
        days_before,
        days_after
    );

    let today = chrono::Local::now().date_naive();
    let range_start = today - chrono::Duration::days(days_before);
    let range_end = today + chrono::Duration::days(days_after);
    let start_str = range_start.format("%Y-%m-%d").to_string();
    let end_str = format!("{}T23:59:59", range_end.format("%Y-%m-%d"));

    state
        .db_read(move |db| {
            let conn = db.conn_ref();

            // Query meetings in the date range
            let mut stmt = conn
                .prepare(
                    "SELECT m.id, m.title, m.meeting_type, m.start_time, m.end_time,
                    mt.summary, mt.transcript_processed_at, mt.has_new_signals,
                    (mp.prep_frozen_json IS NOT NULL) as has_frozen_prep
             FROM meetings m
             LEFT JOIN meeting_transcripts mt ON mt.meeting_id = m.id
             LEFT JOIN meeting_prep mp ON mp.meeting_id = m.id
             WHERE m.start_time >= ?1 AND m.start_time <= ?2
               AND (mt.intelligence_state IS NULL OR mt.intelligence_state != 'archived')
               AND m.meeting_type NOT IN ('personal', 'focus', 'blocked')
             ORDER BY m.start_time ASC",
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

            log::info!(
                "get_meeting_timeline: {} raw meetings found",
                raw_meetings.len()
            );
            if raw_meetings.is_empty() {
                return Ok(Vec::new());
            }

            // Batch fetch linked entities for all meetings
            let meeting_ids: Vec<String> = raw_meetings.iter().map(|m| m.id.clone()).collect();
            let entity_map = db.get_meeting_entity_map(&meeting_ids).unwrap_or_default();

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
            let mut result: Vec<crate::types::TimelineMeeting> =
                Vec::with_capacity(raw_meetings.len());
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
                let has_outcomes = capture_count > 0 || m.transcript_processed_at.is_some();

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
                    has_prep: m.has_frozen_prep
                        || quality.as_ref().is_some_and(|q| {
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
        })
        .await
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
        "SELECT DISTINCT m.id FROM meetings m
         INNER JOIN meeting_entities me ON me.meeting_id = m.id
         WHERE me.entity_id IN ({})
           AND m.start_time < ?1
           AND m.id != ?2
         ORDER BY m.start_time DESC
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

/// Parse a meeting datetime string into a UTC DateTime.
pub fn parse_meeting_datetime(value: &str) -> Option<chrono::DateTime<chrono::Utc>> {
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

/// Parsed user agenda layer — supports both legacy `["item"]` and rich format.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserAgendaLayer {
    #[serde(default)]
    pub items: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dismissed_topics: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hidden_attendees: Vec<String>,
}

/// Parse user agenda JSON (legacy vec or rich layer).
pub fn parse_user_agenda_layer(value: Option<&str>) -> UserAgendaLayer {
    let Some(json) = value else {
        return UserAgendaLayer::default();
    };
    if let Ok(layer) = serde_json::from_str::<UserAgendaLayer>(json) {
        return layer;
    }
    if let Ok(items) = serde_json::from_str::<Vec<String>>(json) {
        return UserAgendaLayer {
            items,
            ..Default::default()
        };
    }
    UserAgendaLayer::default()
}

/// Check if a meeting's user layer fields are read-only.
pub fn is_meeting_user_layer_read_only(meeting: &crate::db::DbMeeting) -> bool {
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
    end_dt.is_some_and(|e| e < now)
}

/// Resolve the on-disk path for a meeting's prep JSON file.
pub fn resolve_prep_path(meeting_id: &str, state: &AppState) -> Result<std::path::PathBuf, String> {
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

    if !path.starts_with(&preps_dir) {
        return Err("Invalid meeting ID".to_string());
    }

    if path.exists() {
        Ok(path)
    } else {
        Err(format!("Prep file not found: {}", path.display()))
    }
}

/// Get full meeting intelligence for the detail page.
///
/// Uses db_read for the heavy lifting (queries + prep loading), then a
/// lightweight db_write only for the two trivial UPDATEs (mark_prep_reviewed,
/// clear_meeting_new_signals). Disk I/O for prep files happens inside the
/// read closure to avoid a second round-trip, but doesn't block the writer.
pub async fn get_meeting_intelligence(
    state: &AppState,
    meeting_id: &str,
) -> Result<MeetingIntelligence, String> {
    let config = state
        .config
        .read()
        .map_err(|_| "Lock poisoned")?
        .clone()
        .ok_or("No configuration loaded")?;

    let meeting_id_owned = meeting_id.to_string();

    // Pre-check: if meeting doesn't exist in DB, try to persist it from the
    // live calendar cache. This covers the race where calendar_merge shows a
    // "New" event on the briefing before the poller has written it to SQLite.
    let mid_check = meeting_id_owned.clone();
    let exists = state
        .db_read(move |db| {
            let found = db
                .get_meeting_by_id(&mid_check)
                .map_err(|e| e.to_string())?
                .is_some();
            if !found {
                let raw = mid_check.replace("_at_", "@");
                Ok(db
                    .get_meeting_by_calendar_event_id(&raw)
                    .map_err(|e| e.to_string())?
                    .is_some())
            } else {
                Ok(true)
            }
        })
        .await
        .unwrap_or(false);

    if !exists {
        // Look for matching event in the live calendar cache
        let live_event = state.calendar.events.read().ok().and_then(|events| {
            let raw_id = meeting_id.replace("_at_", "@");
            events
                .iter()
                .find(|e| e.id == raw_id || e.id == meeting_id)
                .cloned()
        });

        if let Some(event) = live_event {
            let primary_id = crate::workflow::deliver::meeting_primary_id(
                Some(&event.id),
                &event.title,
                &event.start.to_rfc3339(),
                event.meeting_type.as_str(),
            );
            let attendees_json = serde_json::to_string(&event.attendees).unwrap_or_default();
            let start_rfc = event.start.to_rfc3339();
            let end_rfc = event.end.to_rfc3339();
            let mtype = event.meeting_type.as_str().to_string();
            let title = event.title.clone();
            let eid = event.id.clone();
            let _ = state
                .db_write(move |db| {
                    db.ensure_meeting_in_history(crate::db::EnsureMeetingHistoryInput {
                        id: &primary_id,
                        title: &title,
                        meeting_type: &mtype,
                        start_time: &start_rfc,
                        end_time: Some(&end_rfc),
                        calendar_event_id: Some(&eid),
                        attendees: Some(&attendees_json),
                        description: None,
                    })
                    .map_err(|e| e.to_string())?;
                    Ok(())
                })
                .await;
            log::info!(
                "Auto-persisted meeting from live calendar cache: {}",
                meeting_id
            );
        }
    }

    // Phase 1: Read-only — all queries, prep loading, quality assessment
    let intel = state
        .db_read(move |db| {
            let workspace = Path::new(&config.workspace_path);
            let today_dir = workspace.join("_today");
            let meeting_id = meeting_id_owned.as_str();

            let meeting = if let Some(row) = db
                .get_meeting_intelligence_row(meeting_id)
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
            let user_agenda = if agenda_layer.items.is_empty() {
                None
            } else {
                Some(agenda_layer.items.clone())
            };
            let dismissed_topics = agenda_layer.dismissed_topics.clone();
            let hidden_attendees = agenda_layer.hidden_attendees.clone();
            let user_notes = meeting.user_notes.clone();
            let mut prep = load_meeting_prep_from_sources(&today_dir, &meeting);

            if let Some(ref mut prep_data) = prep {
                prep_data.user_agenda = user_agenda.clone();
                prep_data.user_notes = user_notes.clone();

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

            // Single query for captures, then split by type (avoids duplicate in outcomes)
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

            // Build outcomes from already-fetched captures + actions (no duplicate queries)
            let outcomes = build_outcomes_from_data(&meeting, &captures, &actions);

            let prep_snapshot_path = meeting.prep_snapshot_path.clone();
            let prep_frozen_at = meeting.prep_frozen_at.clone();
            let transcript_path = meeting.transcript_path.clone();
            let transcript_processed_at = meeting.transcript_processed_at.clone();

            let intelligence_quality = Some(crate::intelligence::assess_intelligence_quality(
                db, meeting_id,
            ));

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
                intelligence_quality,
            })
        })
        .await?;

    // Step 2: Lightweight writes — mark reviewed + clear new-signal flag
    let write_meeting_id = intel.meeting.id.clone();
    let write_prep_event_id = intel
        .prep
        .as_ref()
        .and_then(|p| p.calendar_event_id.clone());
    let write_prep_title = intel
        .prep
        .as_ref()
        .map(|p| p.title.clone())
        .unwrap_or_default();
    let _ = state
        .db_write(move |db| {
            let _ = db.mark_prep_reviewed(
                &write_meeting_id,
                write_prep_event_id.as_deref(),
                &write_prep_title,
            );
            let _ = db.clear_meeting_new_signals(&write_meeting_id);
            Ok::<(), String>(())
        })
        .await;

    Ok(intel)
}

/// Link meeting entity: DB link, clear prep, enqueue re-assembly.
pub async fn link_meeting_entity_with_prep_queue(
    state: &AppState,
    meeting_id: &str,
    entity_id: &str,
    entity_type: &str,
) -> Result<(), String> {
    let meeting_id_s = meeting_id.to_string();
    let entity_id_s = entity_id.to_string();
    let entity_type_s = entity_type.to_string();
    state
        .db_write(move |db| {
            db.link_meeting_entity(&meeting_id_s, &entity_id_s, &entity_type_s)
                .map_err(|e| e.to_string())?;
            let _ = db.conn_ref().execute(
                "UPDATE meeting_prep SET prep_frozen_json = NULL WHERE meeting_id = ?1",
                rusqlite::params![meeting_id_s],
            );
            Ok(())
        })
        .await?;
    state
        .meeting_prep_queue
        .enqueue(crate::meeting_prep_queue::PrepRequest::new(
            meeting_id.to_string(),
            crate::meeting_prep_queue::PrepPriority::Manual,
        ));
    state.integrations.prep_queue_wake.notify_one();
    log::info!(
        "link_meeting_entity: relinked {} to {} ({}), enqueued prep re-assembly",
        meeting_id,
        entity_id,
        entity_type,
    );
    Ok(())
}

/// Unlink meeting entity: DB unlink, clear prep, enqueue re-assembly.
pub async fn unlink_meeting_entity_with_prep_queue(
    state: &AppState,
    meeting_id: &str,
    entity_id: &str,
) -> Result<(), String> {
    let meeting_id_s = meeting_id.to_string();
    let entity_id_s = entity_id.to_string();
    state
        .db_write(move |db| {
            db.unlink_meeting_entity(&meeting_id_s, &entity_id_s)
                .map_err(|e| e.to_string())?;
            let _ = db.conn_ref().execute(
                "UPDATE meeting_prep SET prep_frozen_json = NULL WHERE meeting_id = ?1",
                rusqlite::params![meeting_id_s],
            );
            Ok(())
        })
        .await?;
    state
        .meeting_prep_queue
        .enqueue(crate::meeting_prep_queue::PrepRequest::new(
            meeting_id.to_string(),
            crate::meeting_prep_queue::PrepPriority::Manual,
        ));
    state.integrations.prep_queue_wake.notify_one();
    log::info!(
        "unlink_meeting_entity: unlinked {} from {}, enqueued prep re-assembly",
        meeting_id,
        entity_id,
    );
    Ok(())
}

/// Persist classification-time entity links (I653).
///
/// Called at meeting creation time (calendar poll, prepare_today, prepare_week).
/// At this point, prep does not exist yet, so no invalidation is needed.
/// Additive: INSERT OR IGNORE preserves existing manual user links.
/// No signal emission — classification is detection, not user mutation.
pub fn persist_classification_entities(
    db: &crate::db::ActionDb,
    meeting_id: &str,
    entities: &[(String, String)], // (entity_id, entity_type)
) -> Result<usize, String> {
    let mut linked = 0usize;
    for (entity_id, entity_type) in entities {
        match db.link_meeting_entity(meeting_id, entity_id, entity_type) {
            Ok(_) => linked += 1,
            Err(e) => {
                // INSERT OR IGNORE — duplicates are expected, real errors are not
                let msg = e.to_string();
                if !msg.contains("UNIQUE constraint") {
                    log::warn!(
                        "persist_classification_entities: failed to link {} → {} ({}): {}",
                        meeting_id,
                        entity_id,
                        entity_type,
                        msg,
                    );
                }
            }
        }
    }
    if linked > 0 {
        log::info!(
            "persist_classification_entities: linked {} entities to meeting {}",
            linked,
            meeting_id,
        );
    }
    Ok(linked)
}

/// Persist entity links from background auto-resolution, invalidating stale prep (I653).
///
/// Called by the background entity resolution trigger (event_trigger.rs) which runs
/// minutes to hours after meeting creation. Prep may already exist and be stale.
/// If prep exists: clears prep_frozen_json and enqueues re-generation.
pub async fn persist_and_invalidate_entity_links(
    state: &AppState,
    meeting_id: &str,
    entities: &[(String, String)], // (entity_id, entity_type)
) -> Result<usize, String> {
    if entities.is_empty() {
        return Ok(0);
    }

    let meeting_id_s = meeting_id.to_string();
    let entities_owned: Vec<(String, String)> = entities.to_vec();

    let (linked, had_prep) = state
        .db_write(move |db| {
            let mut count = 0usize;
            for (entity_id, entity_type) in &entities_owned {
                match db.link_meeting_entity_if_absent(&meeting_id_s, entity_id, entity_type) {
                    Ok(true) => count += 1,
                    Ok(false) => {} // already linked
                    Err(e) => {
                        log::warn!(
                            "persist_and_invalidate: failed to link {} → {}: {}",
                            meeting_id_s,
                            entity_id,
                            e,
                        );
                    }
                }
            }

            // Check if stale prep exists that needs invalidation
            let prep_exists: bool = db
                .conn_ref()
                .query_row(
                    "SELECT prep_frozen_json IS NOT NULL FROM meeting_prep WHERE meeting_id = ?1",
                    rusqlite::params![meeting_id_s],
                    |row| row.get(0),
                )
                .unwrap_or(false);

            if prep_exists && count > 0 {
                let _ = db.conn_ref().execute(
                    "UPDATE meeting_prep SET prep_frozen_json = NULL WHERE meeting_id = ?1",
                    rusqlite::params![meeting_id_s],
                );
            }

            Ok::<(usize, bool), String>((count, prep_exists && count > 0))
        })
        .await?;

    if had_prep {
        state
            .meeting_prep_queue
            .enqueue(crate::meeting_prep_queue::PrepRequest::new(
                meeting_id.to_string(),
                crate::meeting_prep_queue::PrepPriority::Background,
            ));
        state.integrations.prep_queue_wake.notify_one();
        log::info!(
            "persist_and_invalidate: linked {} entities to {}, invalidated stale prep",
            linked,
            meeting_id,
        );
    } else if linked > 0 {
        log::info!(
            "persist_and_invalidate: linked {} entities to {} (no prep to invalidate)",
            linked,
            meeting_id,
        );
    }

    Ok(linked)
}

/// List available meeting prep files from the workspace.
pub fn list_meeting_preps(state: &AppState) -> Result<Vec<String>, String> {
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

/// Update user-authored agenda items on a meeting.
pub fn update_meeting_user_agenda(
    db: &ActionDb,
    state: &AppState,
    meeting_id: &str,
    agenda: Option<Vec<String>>,
    dismissed_topics: Option<Vec<String>>,
    hidden_attendees: Option<Vec<String>>,
) -> Result<(), String> {
    let meeting = db
        .get_meeting_intelligence_row(meeting_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Meeting not found: {}", meeting_id))?;

    if is_meeting_user_layer_read_only(&meeting) {
        return Err("Meeting user fields are read-only after freeze/past state".to_string());
    }

    let existing = parse_user_agenda_layer(meeting.user_agenda_json.as_deref());

    let truncate_strings = |v: Vec<String>, max_items: usize, max_chars: usize| -> Vec<String> {
        v.into_iter()
            .take(max_items)
            .map(|s| {
                if s.len() <= max_chars {
                    s
                } else {
                    let mut end = max_chars;
                    while !s.is_char_boundary(end) && end > 0 {
                        end -= 1;
                    }
                    s[..end].to_string()
                }
            })
            .collect()
    };

    let layer = UserAgendaLayer {
        items: truncate_strings(agenda.unwrap_or(existing.items), 50, 500),
        dismissed_topics: truncate_strings(
            dismissed_topics.unwrap_or(existing.dismissed_topics),
            50,
            500,
        ),
        hidden_attendees: truncate_strings(
            hidden_attendees.unwrap_or(existing.hidden_attendees),
            50,
            500,
        ),
    };

    let agenda_json = if layer.items.is_empty()
        && layer.dismissed_topics.is_empty()
        && layer.hidden_attendees.is_empty()
    {
        None
    } else {
        Some(serde_json::to_string(&layer).map_err(|e| format!("Serialize error: {}", e))?)
    };
    db.update_meeting_user_layer(
        meeting_id,
        agenda_json.as_deref(),
        meeting.user_notes.as_deref(),
    )
    .map_err(|e| e.to_string())?;

    // Optional mirror write to active prep file for same-session coherence.
    if let Ok(prep_path) = resolve_prep_path(meeting_id, state) {
        if let Ok(content) = std::fs::read_to_string(&prep_path) {
            if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&content) {
                if layer.items.is_empty() {
                    json.as_object_mut().map(|o| o.remove("userAgenda"));
                } else {
                    json["userAgenda"] = serde_json::json!(layer.items);
                }
                if let Ok(updated) = serde_json::to_string_pretty(&json) {
                    let _ = std::fs::write(&prep_path, updated);
                }
            }
        }
    }

    // Emit prep quality feedback signal
    let edit_count =
        layer.items.len() + layer.dismissed_topics.len() + layer.hidden_attendees.len();
    if edit_count > 0 {
        let entity_info = db
            .get_meeting_entities(meeting_id)
            .ok()
            .and_then(|entities| {
                entities.into_iter().find(|e| {
                    e.entity_type == crate::entity::EntityType::Account
                        || e.entity_type == crate::entity::EntityType::Project
                })
            });
        let (etype, eid) = entity_info
            .map(|e| (e.entity_type.as_str().to_string(), e.id))
            .unwrap_or_else(|| ("meeting".to_string(), meeting_id.to_string()));
        let _ = crate::services::signals::emit_and_propagate(
            db, &state.signals.engine,
            &etype,
            &eid,
            "prep_edited",
            "user_edit",
            Some(&format!(
                "{{\"meeting_id\":\"{}\",\"agenda_items\":{},\"dismissed\":{},\"hidden_attendees\":{}}}",
                meeting_id,
                layer.items.len(),
                layer.dismissed_topics.len(),
                layer.hidden_attendees.len()
            )),
            0.6,
        );
    }

    Ok(())
}

/// Update user-authored notes on a meeting.
pub fn update_meeting_user_notes(
    db: &ActionDb,
    state: &AppState,
    meeting_id: &str,
    notes: &str,
) -> Result<(), String> {
    let meeting = db
        .get_meeting_intelligence_row(meeting_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Meeting not found: {}", meeting_id))?;

    if is_meeting_user_layer_read_only(&meeting) {
        return Err("Meeting user fields are read-only after freeze/past state".to_string());
    }

    let notes_opt = if notes.trim().is_empty() {
        None
    } else {
        Some(notes)
    };
    db.update_meeting_user_layer(meeting_id, meeting.user_agenda_json.as_deref(), notes_opt)
        .map_err(|e| e.to_string())?;

    // Optional mirror write to active prep file for same-session coherence.
    if let Ok(prep_path) = resolve_prep_path(meeting_id, state) {
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

/// Update a single field in a meeting's frozen prep JSON with signal emission.
///
/// Supports field paths like:
/// - `"meetingContext"` (simple key)
/// - `"risks[0]"` (array index)
/// - `"attendeeContext[2].assessment"` (nested array + key)
pub fn update_meeting_prep_field(
    db: &ActionDb,
    state: &AppState,
    meeting_id: &str,
    field_path: &str,
    value: &str,
    target_person_id: Option<&str>,
) -> Result<(), String> {
    let meeting = db
        .get_meeting_intelligence_row(meeting_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Meeting not found: {}", meeting_id))?;

    let frozen = meeting
        .prep_frozen_json
        .as_deref()
        .ok_or_else(|| "No frozen prep JSON to update".to_string())?;

    let mut json: serde_json::Value =
        serde_json::from_str(frozen).map_err(|e| format!("Invalid prep JSON: {}", e))?;

    // Parse field_path and apply update
    apply_field_path_update(&mut json, field_path, value)?;

    let updated = serde_json::to_string(&json).map_err(|e| format!("Serialize error: {}", e))?;

    // Distinguish curation (delete/clear) from correction (edit).
    let is_curation = value.trim().is_empty() || value == "[]" || value == "null";

    // Get linked entities for signal emission
    let entities = db
        .get_meeting_entities(meeting_id)
        .map_err(|e| e.to_string())?;

    db.with_transaction(|tx| {
        tx.update_prep_frozen_json(meeting_id, &updated)
            .map_err(|e| e.to_string())?;

        let (signal_type, source, confidence) = if is_curation {
            ("intelligence_curated", "user_curation", 0.5)
        } else {
            ("user_correction", "user_edit", 1.0)
        };

        // Emit signal for each linked entity
        for entity in &entities {
            let entity_type_str = match entity.entity_type {
                crate::entity::EntityType::Account => "account",
                crate::entity::EntityType::Project => "project",
                crate::entity::EntityType::Person => "person",
                _ => continue,
            };
            crate::services::signals::emit_and_propagate(
                tx,
                &state.signals.engine,
                entity_type_str,
                &entity.id,
                signal_type,
                source,
                Some(&format!(
                    "{{\"field\":\"{}\",\"meeting_id\":\"{}\"}}",
                    field_path, meeting_id
                )),
                confidence,
            )
            .map_err(|e| format!("signal emit failed: {e}"))?;
        }

        // Attendee assessment edits: also target the specific person entity
        if let Some(person_id) = target_person_id {
            crate::services::signals::emit_and_propagate(
                tx,
                &state.signals.engine,
                "person",
                person_id,
                signal_type,
                source,
                Some(&format!(
                    "{{\"field\":\"{}\",\"meeting_id\":\"{}\"}}",
                    field_path, meeting_id
                )),
                confidence,
            )
            .map_err(|e| format!("signal emit failed: {e}"))?;
        }

        // Also emit a signal for the meeting itself
        crate::services::signals::emit(
            tx,
            "meeting",
            meeting_id,
            signal_type,
            source,
            Some(&format!("{{\"field\":\"{}\"}}", field_path)),
            confidence,
        )
        .map_err(|e| format!("signal emit failed: {e}"))?;

        Ok(())
    })?;

    Ok(())
}

/// Parse a field path and apply an update to a JSON value.
///
/// Supported path formats:
/// - `"meetingContext"` → `json["meetingContext"]`
/// - `"risks[0]"` → `json["risks"][0]`
/// - `"attendeeContext[2].assessment"` → `json["attendeeContext"][2]["assessment"]`
fn apply_field_path_update(
    json: &mut serde_json::Value,
    field_path: &str,
    value: &str,
) -> Result<(), String> {
    let segments = parse_field_path(field_path)?;

    if segments.is_empty() {
        return Err("Empty field path".to_string());
    }

    // Navigate to the parent, then set the final segment
    let mut current = json as &mut serde_json::Value;
    for segment in &segments[..segments.len() - 1] {
        current = navigate_segment(current, segment)?;
    }

    let new_value = if value.trim().is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::Value::String(value.to_string())
    };

    match &segments[segments.len() - 1] {
        PathSegment::Key(key) => {
            current[key.as_str()] = new_value;
        }
        PathSegment::Index(idx) => {
            let arr = current
                .as_array_mut()
                .ok_or_else(|| format!("Expected array for index [{}]", idx))?;
            if *idx >= arr.len() {
                return Err(format!(
                    "Index {} out of bounds (array length {})",
                    idx,
                    arr.len()
                ));
            }
            arr[*idx] = new_value;
        }
    }

    Ok(())
}

#[derive(Debug)]
enum PathSegment {
    Key(String),
    Index(usize),
}

/// Parse a field path into segments.
///
/// Examples:
/// - `"meetingContext"` → `[Key("meetingContext")]`
/// - `"risks[0]"` → `[Key("risks"), Index(0)]`
/// - `"attendeeContext[2].assessment"` → `[Key("attendeeContext"), Index(2), Key("assessment")]`
fn parse_field_path(path: &str) -> Result<Vec<PathSegment>, String> {
    let mut segments = Vec::new();
    let mut remaining = path;

    while !remaining.is_empty() {
        // Strip leading dot separator
        if remaining.starts_with('.') {
            remaining = &remaining[1..];
        }

        if remaining.starts_with('[') {
            // Parse index: [N]
            let end = remaining
                .find(']')
                .ok_or_else(|| format!("Unclosed bracket in path: {}", path))?;
            let idx_str = &remaining[1..end];
            let idx: usize = idx_str
                .parse()
                .map_err(|_| format!("Invalid array index: {}", idx_str))?;
            segments.push(PathSegment::Index(idx));
            remaining = &remaining[end + 1..];
        } else {
            // Parse key: up to next '[' or '.' or end
            let end = remaining.find(['[', '.']).unwrap_or(remaining.len());
            if end == 0 {
                return Err(format!("Empty key segment in path: {}", path));
            }
            segments.push(PathSegment::Key(remaining[..end].to_string()));
            remaining = &remaining[end..];
        }
    }

    Ok(segments)
}

fn navigate_segment<'a>(
    current: &'a mut serde_json::Value,
    segment: &PathSegment,
) -> Result<&'a mut serde_json::Value, String> {
    match segment {
        PathSegment::Key(key) => {
            if current.get(key.as_str()).is_none() {
                return Err(format!("Key '{}' not found in JSON", key));
            }
            Ok(&mut current[key.as_str()])
        }
        PathSegment::Index(idx) => {
            let arr = current
                .as_array_mut()
                .ok_or_else(|| format!("Expected array for index [{}]", idx))?;
            if *idx >= arr.len() {
                return Err(format!(
                    "Index {} out of bounds (array length {})",
                    idx,
                    arr.len()
                ));
            }
            Ok(&mut arr[*idx])
        }
    }
}

// ── I453: Meeting handlers extracted from commands.rs ──────────

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingBriefingRefreshProgress {
    pub meeting_id: String,
    pub stage: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u32>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingBriefingRefreshResult {
    pub meeting_id: String,
    pub refreshed_entities: u32,
    pub failed_entities: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub failed_entity_ids: Vec<String>,
    pub prep_rebuilt_sync: bool,
    pub prep_queued: bool,
    pub quality: IntelligenceQuality,
}

#[derive(Debug, Clone)]
struct MeetingBriefingSnapshot {
    prep_frozen_json: Option<String>,
    prep_frozen_at: Option<String>,
    intelligence_state: Option<String>,
    intelligence_quality: Option<String>,
    signal_count: Option<i32>,
    last_enriched_at: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RefreshCompletionPlan {
    restore_snapshot: bool,
    queue_rebuild: bool,
    return_error: bool,
}

fn plan_refresh_completion(
    had_snapshot: bool,
    refreshed_entities: u32,
    prep_rebuilt_sync: bool,
) -> RefreshCompletionPlan {
    if prep_rebuilt_sync {
        return RefreshCompletionPlan {
            restore_snapshot: false,
            queue_rebuild: false,
            return_error: false,
        };
    }

    if had_snapshot && refreshed_entities == 0 {
        return RefreshCompletionPlan {
            restore_snapshot: true,
            queue_rebuild: true,
            return_error: true,
        };
    }

    RefreshCompletionPlan {
        restore_snapshot: false,
        queue_rebuild: true,
        return_error: false,
    }
}

fn restore_meeting_briefing_snapshot(
    db: &crate::db::ActionDb,
    meeting_id: &str,
    snapshot: &MeetingBriefingSnapshot,
) -> Result<(), String> {
    db.conn_ref()
        .execute(
            "UPDATE meeting_prep
             SET prep_frozen_json = ?1, prep_frozen_at = ?2
             WHERE meeting_id = ?3",
            rusqlite::params![
                snapshot.prep_frozen_json.as_deref(),
                snapshot.prep_frozen_at.as_deref(),
                meeting_id
            ],
        )
        .map_err(|e| format!("Failed to restore previous briefing: {}", e))?;

    db.conn_ref()
        .execute(
            "UPDATE meeting_transcripts
             SET intelligence_state = ?1,
                 intelligence_quality = ?2,
                 signal_count = ?3,
                 last_enriched_at = ?4
             WHERE meeting_id = ?5",
            rusqlite::params![
                snapshot.intelligence_state.as_deref(),
                snapshot.intelligence_quality.as_deref(),
                snapshot.signal_count,
                snapshot.last_enriched_at.as_deref(),
                meeting_id
            ],
        )
        .map_err(|e| format!("Failed to restore previous meeting state: {}", e))?;

    Ok(())
}

fn emit_briefing_refresh_progress(
    app_handle: Option<&tauri::AppHandle>,
    payload: MeetingBriefingRefreshProgress,
) {
    if let Some(app) = app_handle {
        let _ = app.emit("meeting-briefing-refresh-progress", &payload);
    }
}

/// Single-service full briefing refresh for one meeting.
///
/// This is the deterministic manual refresh path:
/// 1) snapshot current prep,
/// 2) refresh linked entity intelligence,
/// 3) rebuild prep,
/// 4) swap only when the replacement is ready.
pub async fn refresh_meeting_briefing_full(
    state: &AppState,
    meeting_id: &str,
    app_handle: Option<&tauri::AppHandle>,
) -> Result<MeetingBriefingRefreshResult, String> {
    let meeting_id_owned = meeting_id.to_string();

    emit_briefing_refresh_progress(
        app_handle,
        MeetingBriefingRefreshProgress {
            meeting_id: meeting_id_owned.clone(),
            stage: "started".to_string(),
            message: "Starting full briefing refresh".to_string(),
            entity_id: None,
            entity_type: None,
            entity_name: None,
            current: None,
            total: None,
        },
    );

    // Phase 1: snapshot current prep + collect linked entities.
    emit_briefing_refresh_progress(
        app_handle,
        MeetingBriefingRefreshProgress {
            meeting_id: meeting_id_owned.clone(),
            stage: "snapshotting_prep".to_string(),
            message: "Preserving current briefing while refresh runs".to_string(),
            entity_id: None,
            entity_type: None,
            entity_name: None,
            current: None,
            total: None,
        },
    );

    let meeting_id_for_phase1 = meeting_id_owned.clone();
    let (linked_entities, previous_snapshot) = state
        .db_write(move |db| {
            let meeting = db
                .get_meeting_by_id(&meeting_id_for_phase1)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("Meeting not found: {}", meeting_id_for_phase1))?;

            let snapshot = MeetingBriefingSnapshot {
                prep_frozen_json: meeting.prep_frozen_json.clone(),
                prep_frozen_at: meeting.prep_frozen_at.clone(),
                intelligence_state: meeting.intelligence_state.clone(),
                intelligence_quality: meeting.intelligence_quality.clone(),
                signal_count: meeting.signal_count,
                last_enriched_at: meeting.last_enriched_at.clone(),
            };

            let _ = db.update_intelligence_state(&meeting_id_for_phase1, "enriching", None, None);

            let linked_entities = db
                .get_meeting_entities(&meeting_id_for_phase1)
                .map_err(|e| format!("Failed to load linked entities: {}", e))?;

            Ok::<(Vec<crate::entity::DbEntity>, MeetingBriefingSnapshot), String>((
                linked_entities,
                snapshot,
            ))
        })
        .await?;

    let had_snapshot = previous_snapshot.prep_frozen_json.is_some();

    let total_entities = linked_entities.len() as u32;
    if total_entities > 0 {
        emit_briefing_refresh_progress(
            app_handle,
            MeetingBriefingRefreshProgress {
                meeting_id: meeting_id_owned.clone(),
                stage: "refreshing_entities".to_string(),
                message: format!("Refreshing linked intelligence ({})", total_entities),
                entity_id: None,
                entity_type: None,
                entity_name: None,
                current: Some(0),
                total: Some(total_entities),
            },
        );
    }

    // Step 2: refresh linked entity intelligence synchronously.
    let mut refreshed_entities = 0u32;
    let mut failed_entities: Vec<(String, String)> = Vec::new();

    for (idx, entity) in linked_entities.iter().enumerate() {
        let current = (idx as u32) + 1;
        let entity_id = entity.id.clone();
        let entity_type = entity.entity_type.as_str().to_string();
        let entity_name = entity.name.clone();

        emit_briefing_refresh_progress(
            app_handle,
            MeetingBriefingRefreshProgress {
                meeting_id: meeting_id_owned.clone(),
                stage: "refreshing_entities".to_string(),
                message: format!(
                    "Refreshing {} intelligence ({}/{})",
                    entity_name, current, total_entities
                ),
                entity_id: Some(entity_id.clone()),
                entity_type: Some(entity_type.clone()),
                entity_name: Some(entity_name.clone()),
                current: Some(current),
                total: Some(total_entities),
            },
        );

        match crate::services::intelligence::enrich_entity(
            entity_id.clone(),
            entity_type.clone(),
            state,
            app_handle,
        )
        .await
        {
            Ok(_) => {
                refreshed_entities += 1;
                crate::intel_queue::invalidate_and_requeue_meeting_preps(state, &entity_id);
                emit_briefing_refresh_progress(
                    app_handle,
                    MeetingBriefingRefreshProgress {
                        meeting_id: meeting_id_owned.clone(),
                        stage: "entity_refreshed".to_string(),
                        message: format!("Updated {} intelligence", entity_name),
                        entity_id: Some(entity_id),
                        entity_type: Some(entity_type),
                        entity_name: Some(entity_name),
                        current: Some(current),
                        total: Some(total_entities),
                    },
                );
            }
            Err(err) => {
                log::warn!(
                    "refresh_meeting_briefing_full: sync entity refresh failed for {} ({}): {}",
                    entity.id,
                    entity.entity_type.as_str(),
                    err
                );
                failed_entities.push((entity.id.clone(), entity.entity_type.as_str().to_string()));
                emit_briefing_refresh_progress(
                    app_handle,
                    MeetingBriefingRefreshProgress {
                        meeting_id: meeting_id_owned.clone(),
                        stage: "entity_failed".to_string(),
                        message: format!("Queued retry for {} intelligence", entity.name),
                        entity_id: Some(entity.id.clone()),
                        entity_type: Some(entity.entity_type.as_str().to_string()),
                        entity_name: Some(entity.name.clone()),
                        current: Some(current),
                        total: Some(total_entities),
                    },
                );
            }
        }
    }

    // Failed entity refreshes are queued for retry.
    if !failed_entities.is_empty() {
        for (entity_id, entity_type) in &failed_entities {
            state
                .intel_queue
                .enqueue(crate::intel_queue::IntelRequest::new(
                    entity_id.clone(),
                    entity_type.clone(),
                    crate::intel_queue::IntelPriority::Manual,
                ));
        }
        state.integrations.intel_queue_wake.notify_one();
    }

    // Phase 3: rebuild mechanical prep now; fallback to queue only when no
    // previous snapshot exists. Snapshot-backed refreshes keep the prior prep
    // visible instead of queueing a no-op rebuild behind the existing snapshot.
    emit_briefing_refresh_progress(
        app_handle,
        MeetingBriefingRefreshProgress {
            meeting_id: meeting_id_owned.clone(),
            stage: "rebuilding_prep".to_string(),
            message: "Rebuilding meeting briefing".to_string(),
            entity_id: None,
            entity_type: None,
            entity_name: None,
            current: None,
            total: None,
        },
    );

    let prep_rebuilt_sync = match crate::meeting_prep_queue::meeting_prep_blocking_inputs(state) {
        Ok((workspace, embedding_model)) => match tauri::async_runtime::spawn_blocking({
            let meeting_id = meeting_id_owned.clone();
            move || {
                crate::meeting_prep_queue::regenerate_mechanical_prep_now_blocking(
                    workspace,
                    embedding_model,
                    meeting_id,
                )
            }
        })
        .await
        {
            Ok(Ok(wrote)) => wrote,
            Ok(Err(err)) => {
                log::warn!(
                    "refresh_meeting_briefing_full: immediate prep rebuild failed for {}: {}",
                    meeting_id_owned,
                    err
                );
                false
            }
            Err(err) => {
                log::warn!(
                    "refresh_meeting_briefing_full: prep rebuild task panicked for {}: {}",
                    meeting_id_owned,
                    err
                );
                false
            }
        },
        Err(err) => {
            log::warn!(
                "refresh_meeting_briefing_full: prep rebuild inputs failed for {}: {}",
                meeting_id_owned,
                err
            );
            false
        }
    };

    let completion_plan =
        plan_refresh_completion(had_snapshot, refreshed_entities, prep_rebuilt_sync);

    if completion_plan.queue_rebuild {
        state.meeting_prep_queue.enqueue(
            crate::meeting_prep_queue::PrepRequest::overwrite_existing(
                meeting_id_owned.clone(),
                crate::meeting_prep_queue::PrepPriority::Manual,
            ),
        );
        state.integrations.prep_queue_wake.notify_one();
    }

    if completion_plan.restore_snapshot {
        let meeting_id_for_restore = meeting_id_owned.clone();
        let snapshot_for_restore = previous_snapshot.clone();
        state
            .db_write(move |db| {
                restore_meeting_briefing_snapshot(
                    db,
                    &meeting_id_for_restore,
                    &snapshot_for_restore,
                )
            })
            .await?;

        emit_briefing_refresh_progress(
            app_handle,
            MeetingBriefingRefreshProgress {
                meeting_id: meeting_id_owned.clone(),
                stage: "failed".to_string(),
                message: "Update failed - showing previous briefing".to_string(),
                entity_id: None,
                entity_type: None,
                entity_name: None,
                current: Some(refreshed_entities),
                total: Some(total_entities),
            },
        );

        if completion_plan.return_error {
            return Err("Update failed - showing previous briefing".to_string());
        }
    }

    // Phase 4: finalize meeting intelligence metadata only when a new prep was written.
    let quality = if prep_rebuilt_sync {
        let meeting_id_for_phase4 = meeting_id_owned.clone();
        state
            .db_write(move |db| {
                let quality =
                    crate::intelligence::assess_intelligence_quality(db, &meeting_id_for_phase4);
                db.update_intelligence_state(
                    &meeting_id_for_phase4,
                    "enriched",
                    Some(&quality.level.to_string()),
                    Some(quality.signal_count as i32),
                )
                .map_err(|e| e.to_string())?;
                let _ = db.clear_meeting_new_signals(&meeting_id_for_phase4);
                Ok::<IntelligenceQuality, String>(quality)
            })
            .await?
    } else {
        let meeting_id_for_quality = meeting_id_owned.clone();
        state
            .db_read(move |db| {
                Ok(crate::intelligence::assess_intelligence_quality(
                    db,
                    &meeting_id_for_quality,
                ))
            })
            .await?
    };

    let prep_queued = completion_plan.queue_rebuild;
    let result = MeetingBriefingRefreshResult {
        meeting_id: meeting_id_owned.clone(),
        refreshed_entities,
        failed_entities: failed_entities.len() as u32,
        failed_entity_ids: failed_entities
            .iter()
            .map(|(entity_id, _)| entity_id.clone())
            .collect(),
        prep_rebuilt_sync,
        prep_queued,
        quality,
    };

    let completed_msg = if result.failed_entities > 0 {
        format!(
            "Briefing refreshed ({} entity retries queued)",
            result.failed_entities
        )
    } else {
        "Briefing refreshed".to_string()
    };
    emit_briefing_refresh_progress(
        app_handle,
        MeetingBriefingRefreshProgress {
            meeting_id: meeting_id_owned,
            stage: "completed".to_string(),
            message: completed_msg,
            entity_id: None,
            entity_type: None,
            entity_name: None,
            current: Some(refreshed_entities),
            total: Some(total_entities),
        },
    );

    Ok(result)
}

/// Refresh all future meeting preps: clear frozen JSON and re-enqueue.
pub async fn refresh_meeting_preps(state: &AppState) -> Result<String, String> {
    let meeting_ids: Vec<String> = state.db_write(|db| {
        let now = chrono::Utc::now().to_rfc3339();

        let meeting_ids: Vec<String> = db
            .conn_ref()
            .prepare(
                "SELECT m.id FROM meetings m
                 LEFT JOIN meeting_transcripts mt ON mt.meeting_id = m.id
                 WHERE m.start_time > ?1
                   AND m.meeting_type NOT IN ('personal', 'focus', 'blocked')
                   AND (mt.intelligence_state IS NULL OR mt.intelligence_state != 'archived')",
            )
            .and_then(|mut stmt| {
                let rows = stmt.query_map(rusqlite::params![now], |row| {
                    row.get::<_, String>(0)
                })?;
                Ok(rows.filter_map(|r| r.ok()).collect())
            })
            .map_err(|e| format!("Failed to query future meetings: {}", e))?;

        for mid in &meeting_ids {
            let _ = db.conn_ref().execute(
                "UPDATE meeting_prep SET prep_frozen_json = NULL, prep_frozen_at = NULL WHERE meeting_id = ?1",
                rusqlite::params![mid],
            );
        }

        Ok(meeting_ids)
    }).await?;

    if meeting_ids.is_empty() {
        return Ok("No future meetings to refresh".to_string());
    }

    for mid in &meeting_ids {
        state
            .meeting_prep_queue
            .enqueue(crate::meeting_prep_queue::PrepRequest::new(
                mid.clone(),
                crate::meeting_prep_queue::PrepPriority::Manual,
            ));
    }

    let count = meeting_ids.len();
    log::info!(
        "refresh_meeting_preps: cleared and requeued {} future meetings",
        count
    );
    Ok(format!("Refreshing {} meeting preps", count))
}

/// Reprocess an already-attached transcript: clear all extraction data, remove
/// the TOCTOU guard, then re-run the full pipeline as a fresh extraction.
pub async fn reprocess_meeting_transcript(
    meeting_id: &str,
    state: &std::sync::Arc<AppState>,
    app_handle: tauri::AppHandle,
) -> Result<crate::types::TranscriptResult, String> {
    // Look up the meeting and its transcript path
    let mid = meeting_id.to_string();
    let meeting_data = state
        .db_read(move |db| {
            let meeting = db
                .get_meeting_intelligence_row(&mid)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("Meeting not found: {}", mid))?;
            let transcript_path = meeting
                .transcript_path
                .clone()
                .ok_or_else(|| format!("No transcript attached to meeting {}", mid))?;
            Ok((meeting, transcript_path))
        })
        .await?;

    let (meeting_row, transcript_path) = meeting_data;

    // Clear all existing extraction data
    let clear_mid = meeting_id.to_string();
    let cleared = state
        .db_write(move |db| {
            db.clear_meeting_extraction_data(&clear_mid)
                .map_err(|e| e.to_string())
        })
        .await?;
    log::info!(
        "Reprocess: cleared {} extraction records for meeting {}",
        cleared,
        meeting_id
    );

    // Remove the TOCTOU guard so attach_meeting_transcript doesn't reject
    if let Ok(mut guard) = state.capture.transcript_processed.lock() {
        guard.remove(meeting_id);
    }

    // Build a CalendarEvent from the DB row for the pipeline
    let cal_event = crate::types::CalendarEvent {
        id: meeting_row.id.clone(),
        title: meeting_row.title.clone(),
        start: chrono::DateTime::parse_from_rfc3339(&meeting_row.start_time)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now()),
        end: meeting_row
            .end_time
            .as_deref()
            .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(chrono::Utc::now),
        meeting_type: crate::parser::parse_meeting_type(&meeting_row.meeting_type),
        attendees: Vec::new(),
        is_all_day: false,
        account: None, // Resolved by transcript pipeline from meeting_entities
        linked_entities: None,
        classified_entities: None,
    };

    // Re-run the full pipeline via the existing attach path
    attach_meeting_transcript(transcript_path, cal_event, state, app_handle).await
}

/// Attach a meeting transcript with TOCTOU guard, async processing, and event emission.
pub async fn attach_meeting_transcript(
    file_path: String,
    meeting: crate::types::CalendarEvent,
    state: &std::sync::Arc<AppState>,
    app_handle: tauri::AppHandle,
) -> Result<crate::types::TranscriptResult, String> {
    {
        let mut guard = state
            .capture
            .transcript_processed
            .lock()
            .map_err(|_| "Lock poisoned")?;
        if guard.contains_key(&meeting.id) {
            return Err(format!(
                "Meeting '{}' already has a processed transcript",
                meeting.title
            ));
        }
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

    let workspace_path = config.workspace_path.clone();
    let profile = config.profile.clone();
    let ai_config = config.ai_models.clone();
    let meeting_id = meeting.id.clone();
    let meeting_clone = meeting.clone();
    let file_path_for_record = file_path.clone();
    let progress_handle = app_handle.clone();

    let result = match tauri::async_runtime::spawn_blocking(move || {
        let workspace = std::path::Path::new(&workspace_path);
        // Open a dedicated connection instead of holding the shared mutex
        // for the entire transcript processing duration (30-120s with PTY).
        let db = crate::db::ActionDb::open().ok();
        crate::processor::transcript::process_transcript(
            workspace,
            &file_path,
            &meeting_clone,
            Some(&progress_handle),
            db.as_ref(),
            &profile,
            Some(&ai_config),
        )
    })
    .await
    {
        Ok(r) => r,
        Err(e) => {
            if let Ok(mut guard) = state.capture.transcript_processed.lock() {
                guard.remove(&meeting_id);
            }
            return Err(format!("Transcript processing task failed: {}", e));
        }
    };

    let has_outcomes = result.status == "success"
        && (result.summary.as_ref().is_some_and(|s| !s.is_empty())
            || !result.wins.is_empty()
            || !result.risks.is_empty()
            || !result.decisions.is_empty()
            || !result.actions.is_empty());

    if result.status == "success" {
        let processed_at = chrono::Utc::now().to_rfc3339();
        let transcript_destination = result.destination.clone().unwrap_or_default();

        // Always persist transcript metadata so the meeting is marked as having a
        // transcript even when AI extraction produced no outcomes (e.g. AI timeout,
        // empty response). Without this, reloading the page after a failed extraction
        // shows no transcript at all — the attachment effectively vanishes.
        {
            let mid = meeting_id.clone();
            let dest = transcript_destination.clone();
            let at = processed_at.clone();
            let summary = result.summary.clone();
            let _ = state
                .db_write(move |db| {
                    if let Err(e) =
                        db.update_meeting_transcript_metadata(&mid, &dest, &at, summary.as_deref())
                    {
                        log::warn!("Failed to persist transcript metadata: {}", e);
                    }
                    Ok(())
                })
                .await;
        }

        if has_outcomes {
            let record = crate::types::TranscriptRecord {
                meeting_id: meeting_id.clone(),
                file_path: file_path_for_record,
                destination: transcript_destination.clone(),
                summary: result.summary.clone(),
                processed_at: processed_at.clone(),
            };

            if let Ok(mut guard) = state.capture.transcript_processed.lock() {
                guard.insert(meeting_id.clone(), record);
                let _ = crate::state::save_transcript_records(&guard);
            }

            if let Ok(mut guard) = state.capture.captured.lock() {
                guard.insert(meeting_id.clone());
            }

            let outcome_data = crate::commands::build_outcome_data(&meeting_id, &result, state);
            let _ = app_handle.emit("transcript-processed", &outcome_data);

            // Emit transcript signals via the main DB connection so the
            // propagation engine can invalidate linked meeting preps.
            {
                let mid = meeting_id.clone();
                let wins = result.wins.len();
                let risks = result.risks.len();
                let decisions = result.decisions.len();
                let engine = std::sync::Arc::clone(&state.signals.engine);
                let sentiment_json = result
                    .sentiment
                    .as_ref()
                    .and_then(|s| serde_json::to_string(s).ok());
                let competitor_mentions: Vec<String> = result
                    .sentiment
                    .as_ref()
                    .map(|s| s.competitor_mentions.clone())
                    .unwrap_or_default();
                let escalation_signals: Vec<String> = result
                    .interaction_dynamics
                    .as_ref()
                    .map(|d| {
                        d.escalation_signals
                            .iter()
                            .map(|e| e.quote.clone())
                            .collect()
                    })
                    .unwrap_or_default();
                let decision_maker_inactive = result
                    .interaction_dynamics
                    .as_ref()
                    .and_then(|d| d.engagement_signals.as_ref())
                    .and_then(|e| e.decision_maker_active.as_deref())
                    .map(|v| v == "no")
                    .unwrap_or(false);
                // I555: Capture champion health, role changes, and risks for signal emissions
                let champion_health = result.champion_health.clone();
                let role_changes_data: Vec<(String, Option<String>, Option<String>)> = result
                    .role_changes
                    .iter()
                    .map(|rc| {
                        (
                            rc.person_name.clone(),
                            rc.old_status.clone(),
                            rc.new_status.clone(),
                        )
                    })
                    .collect();
                let risk_strings: Vec<String> = result.risks.clone();
                let meeting_title = meeting.title.clone();
                let _ = state
                    .db_write(move |db| {
                        let account_id: Option<String> =
                            db.get_meeting_entities(&mid).ok().and_then(|ents| {
                                ents.into_iter()
                                    .find(|e| e.entity_type == crate::entity::EntityType::Account)
                                    .map(|e| e.id)
                            });
                        if let Some(ref aid) = account_id {
                            let value = format!(
                                "{{\"meeting_id\":\"{}\",\"wins\":{},\"risks\":{},\"decisions\":{}}}",
                                mid, wins, risks, decisions
                            );
                            let _ = crate::signals::bus::emit_signal_and_propagate(
                                db,
                                &engine,
                                "account",
                                aid,
                                "transcript_outcomes",
                                "transcript",
                                Some(&value),
                                0.75,
                            );

                            // I509: Emit sentiment-derived signals
                            if let Some(ref sj) = sentiment_json {
                                let _ = crate::signals::bus::emit_signal(
                                    db, "account", aid,
                                    "transcript_sentiment", "transcript",
                                    Some(sj), 0.8,
                                );
                            }
                            for competitor in &competitor_mentions {
                                let _ = crate::signals::bus::emit_signal(
                                    db, "account", aid,
                                    "competitor_mentioned", "transcript",
                                    Some(competitor), 0.7,
                                );
                            }
                            for escalation in &escalation_signals {
                                let _ = crate::signals::bus::emit_signal(
                                    db, "account", aid,
                                    "escalation_detected", "transcript",
                                    Some(escalation), 0.8,
                                );
                            }
                            if decision_maker_inactive {
                                let _ = crate::signals::bus::emit_signal(
                                    db, "account", aid,
                                    "stakeholder_disengagement", "transcript",
                                    Some("decision_maker_inactive"), 0.6,
                                );
                            }

                            // I555: Champion health → person-level signal
                            if let Some(ref ch) = champion_health {
                                if let Ok(Some(champion_pid)) = db.get_champion_person_id(aid) {
                                    match ch.champion_status.as_str() {
                                        "weak" | "lost" => {
                                            let confidence = if ch.champion_status == "lost" { 0.9 } else { 0.7 };
                                            let value = serde_json::json!({
                                                "champion_status": ch.champion_status,
                                                "evidence": ch.champion_evidence,
                                            }).to_string();
                                            let _ = crate::signals::bus::emit_signal_and_propagate(
                                                db, &engine,
                                                "person", &champion_pid,
                                                "negative_sentiment", "transcript",
                                                Some(&value), confidence,
                                            );
                                        }
                                        "strong" => {
                                            let value = serde_json::json!({
                                                "champion_status": "strong",
                                                "evidence": ch.champion_evidence,
                                            }).to_string();
                                            let _ = crate::signals::bus::emit_signal(
                                                db, "person", &champion_pid,
                                                "champion_engagement_confirmed", "transcript",
                                                Some(&value), 0.8,
                                            );
                                        }
                                        _ => {}
                                    }
                                }
                            }

                            // I555: Meeting frequency signal → triggers rule_meeting_frequency_drop
                            let current_count = db.count_account_meetings_in_days(aid, 30).unwrap_or(0);
                            let previous_count = db.count_account_meetings_in_period(aid, 30, 30).unwrap_or(0);
                            let freq_value = serde_json::json!({
                                "meeting_count_30d": current_count,
                                "current_count": current_count,
                                "previous_count": previous_count,
                            }).to_string();
                            let _ = crate::signals::bus::emit_signal_and_propagate(
                                db, &engine,
                                "account", aid,
                                "meeting_frequency", "transcript",
                                Some(&freq_value), 0.9,
                            );

                            // I555: Risk signals with urgency-graduated confidence
                            for risk in &risk_strings {
                                let (urgency, confidence) = if risk.starts_with("[RED]") {
                                    ("red", 0.9)
                                } else if risk.starts_with("[YELLOW]") {
                                    ("yellow", 0.6)
                                } else if risk.starts_with("[GREEN_WATCH]") {
                                    ("green_watch", 0.3)
                                } else {
                                    ("unknown", 0.5)
                                };
                                let risk_value = serde_json::json!({
                                    "urgency": urgency,
                                    "content": risk,
                                }).to_string();
                                let _ = crate::signals::bus::emit_signal_and_propagate(
                                    db, &engine,
                                    "account", aid,
                                    "risk_detected", "transcript",
                                    Some(&risk_value), confidence,
                                );
                            }

                            // I555: Role changes → stakeholder_change signal
                            for (person_name, old_status, new_status) in &role_changes_data {
                                let rc_value = serde_json::json!({
                                    "person": person_name,
                                    "old": old_status,
                                    "new": new_status,
                                }).to_string();
                                let _ = crate::signals::bus::emit_signal_and_propagate(
                                    db, &engine,
                                    "account", aid,
                                    "stakeholder_change", "transcript",
                                    Some(&rc_value), 0.8,
                                );
                            }
                        }

                        // I509: Store sentiment as DB capture
                        if let Some(ref sj) = sentiment_json {
                            let _ = db.insert_capture(
                                &mid,
                                &meeting_title,
                                account_id.as_deref(),
                                "sentiment",
                                sj,
                            );
                        }

                        Ok(())
                    })
                    .await;
            }
        } else {
            // No outcomes extracted — remove from guard so the user can retry.
            if let Ok(mut guard) = state.capture.transcript_processed.lock() {
                guard.remove(&meeting_id);
                let _ = crate::state::save_transcript_records(&guard);
            }
        }
    } else if let Ok(mut guard) = state.capture.transcript_processed.lock() {
        guard.remove(&meeting_id);
        let _ = crate::state::save_transcript_records(&guard);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::plan_refresh_completion;

    #[test]
    fn refresh_completion_restores_snapshot_on_full_failure() {
        let plan = plan_refresh_completion(true, 0, false);

        assert!(plan.restore_snapshot);
        assert!(plan.queue_rebuild);
        assert!(plan.return_error);
    }

    #[test]
    fn refresh_completion_keeps_successful_swap() {
        let plan = plan_refresh_completion(true, 1, true);

        assert!(!plan.restore_snapshot);
        assert!(!plan.queue_rebuild);
        assert!(!plan.return_error);
    }

    #[test]
    fn refresh_completion_queues_when_no_snapshot_exists() {
        let plan = plan_refresh_completion(false, 0, false);

        assert!(!plan.restore_snapshot);
        assert!(plan.queue_rebuild);
        assert!(!plan.return_error);
    }

    #[test]
    fn refresh_completion_queues_retry_for_partial_success() {
        let plan = plan_refresh_completion(true, 1, false);

        assert!(!plan.restore_snapshot);
        assert!(plan.queue_rebuild);
        assert!(!plan.return_error);
    }
}
