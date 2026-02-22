//! Intelligence lifecycle management (ADR-0081).
//!
//! Independent, idempotent functions for assessing and generating meeting
//! intelligence. These can be called from any context: daily orchestrator,
//! weekly run, calendar polling, or user-triggered refresh.

use std::path::PathBuf;

use chrono::Utc;
use serde_json::json;

use crate::db::ActionDb;
use crate::error::ExecutionError;
use crate::pty::{ModelTier, PtyManager};
use crate::state::AppState;
use crate::types::{IntelligenceQuality, QualityLevel, Staleness};

/// Compute staleness from an optional `last_enriched_at` timestamp.
fn compute_staleness(last_enriched_at: Option<&str>) -> Staleness {
    match last_enriched_at {
        None => Staleness::Stale,
        Some(ts) => {
            let enriched = chrono::DateTime::parse_from_rfc3339(ts)
                .or_else(|_| {
                    chrono::NaiveDateTime::parse_from_str(ts, "%Y-%m-%d %H:%M:%S")
                        .map(|naive| naive.and_utc().fixed_offset())
                })
                .ok();
            match enriched {
                None => Staleness::Stale,
                Some(dt) => {
                    let hours = (Utc::now() - dt.with_timezone(&Utc)).num_hours();
                    if hours < 12 {
                        Staleness::Current
                    } else if hours < 48 {
                        Staleness::Aging
                    } else {
                        Staleness::Stale
                    }
                }
            }
        }
    }
}

/// Assess meeting intelligence quality from database alone (no AI call).
///
/// A meeting can reach `Developing` quality purely from DB queries.
/// Returns an `IntelligenceQuality` with the computed level, signal count,
/// and context flags.
pub fn assess_intelligence_quality(
    db: &ActionDb,
    meeting_id: &str,
) -> IntelligenceQuality {
    let conn = db.conn_ref();

    // 1. Load the meeting row
    let meeting = db.get_meeting_by_id(meeting_id).ok().flatten();
    let meeting = match meeting {
        Some(m) => m,
        None => {
            return IntelligenceQuality {
                level: QualityLevel::Sparse,
                signal_count: 0,
                last_enriched: None,
                has_entity_context: false,
                has_attendee_history: false,
                has_recent_signals: false,
                staleness: Staleness::Stale,
                has_new_signals: false,
            };
        }
    };

    // 2. Check if meeting has linked entities
    let entity_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM meeting_entities WHERE meeting_id = ?1",
            rusqlite::params![meeting_id],
            |row| row.get(0),
        )
        .unwrap_or(0);
    let has_entity_context = entity_count > 0;

    // 3. Check if attendees exist (non-empty JSON array)
    let _has_attendees = meeting
        .attendees
        .as_deref()
        .map(|a| {
            let trimmed = a.trim();
            !trimmed.is_empty() && trimmed != "[]" && trimmed != "null"
        })
        .unwrap_or(false);

    // 4. Check for past meetings with same entity (entity overlap)
    let has_attendee_history = if has_entity_context {
        let past_meeting_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM meetings_history m
                 JOIN meeting_entities me ON me.meeting_id = m.id
                 WHERE me.entity_id IN (
                     SELECT entity_id FROM meeting_entities WHERE meeting_id = ?1
                 )
                 AND m.id != ?1
                 AND m.start_time < ?2",
                rusqlite::params![meeting_id, meeting.start_time],
                |row| row.get(0),
            )
            .unwrap_or(0);
        past_meeting_count > 0
    } else {
        false
    };

    // 5. Check for open actions linked to the entity
    let _has_open_actions: bool = if has_entity_context {
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM actions a
                 WHERE a.status IN ('pending', 'waiting')
                 AND (
                     a.account_id IN (SELECT entity_id FROM meeting_entities WHERE meeting_id = ?1)
                     OR a.project_id IN (SELECT entity_id FROM meeting_entities WHERE meeting_id = ?1)
                 )",
                rusqlite::params![meeting_id],
                |row| row.get(0),
            )
            .unwrap_or(0);
        count > 0
    } else {
        false
    };

    // 6. Count signals from signal_events for the entity
    let signal_count: i64 = if has_entity_context {
        conn.query_row(
            "SELECT COUNT(*) FROM signal_events se
             WHERE se.superseded_by IS NULL
             AND (se.entity_type, se.entity_id) IN (
                 SELECT me.entity_type, me.entity_id
                 FROM meeting_entities me
                 WHERE me.meeting_id = ?1
             )",
            rusqlite::params![meeting_id],
            |row| row.get(0),
        )
        .unwrap_or(0)
    } else {
        0
    };

    // 7. Compute staleness
    let staleness = compute_staleness(meeting.last_enriched_at.as_deref());

    // 8. Compute quality level
    let has_recent_signals = signal_count >= 3;
    let level = if has_entity_context && has_attendee_history && has_recent_signals {
        if staleness == Staleness::Current {
            QualityLevel::Fresh
        } else {
            QualityLevel::Ready
        }
    } else if has_entity_context || has_attendee_history {
        QualityLevel::Developing
    } else {
        QualityLevel::Sparse
    };

    let has_new_signals_flag = meeting.has_new_signals.unwrap_or(0) != 0;

    IntelligenceQuality {
        level,
        signal_count: signal_count as u32,
        last_enriched: meeting.last_enriched_at.clone(),
        has_entity_context,
        has_attendee_history,
        has_recent_signals,
        staleness,
        has_new_signals: has_new_signals_flag,
    }
}

// =============================================================================
// AI Enrichment (I326 Phase 2)
// =============================================================================

/// Context gathered from DB for AI enrichment prompt construction.
struct MeetingEnrichmentContext {
    title: String,
    meeting_type: String,
    start_time: String,
    attendees: String,
    description: String,
    entity_summaries: Vec<String>,
    signal_summaries: Vec<String>,
    prior_intelligence: Option<String>,
}

/// Gather meeting context from DB for AI enrichment (Phase 1 — brief DB lock).
fn gather_meeting_context(
    db: &ActionDb,
    meeting_id: &str,
    meeting: &crate::db::DbMeeting,
) -> MeetingEnrichmentContext {
    let conn = db.conn_ref();

    // Linked entities
    let mut entity_summaries = Vec::new();
    if let Ok(entity_map) = db.get_meeting_entity_map(&[meeting_id.to_string()]) {
        if let Some(entities) = entity_map.get(meeting_id) {
            for entity in entities {
                let name = &entity.name;
                let etype = entity.entity_type.as_str();

                // Try to load a brief intelligence summary for the entity
                let summary: Option<String> = conn
                    .query_row(
                        "SELECT intelligence_quality FROM meetings_history m
                         JOIN meeting_entities me ON me.meeting_id = m.id
                         WHERE me.entity_id = ?1
                         ORDER BY m.start_time DESC LIMIT 1",
                        rusqlite::params![entity.id],
                        |row| row.get(0),
                    )
                    .ok();

                let line = match summary {
                    Some(ref s) if !s.is_empty() => {
                        format!("- {} ({}): quality={}", name, etype, s)
                    }
                    _ => format!("- {} ({})", name, etype),
                };
                entity_summaries.push(line);
            }
        }
    }

    // Recent signals for linked entities
    let mut signal_summaries = Vec::new();
    let signal_query = "SELECT se.signal_type, se.source, se.value, se.entity_id
         FROM signal_events se
         WHERE se.superseded_by IS NULL
         AND (se.entity_type, se.entity_id) IN (
             SELECT me.entity_type, me.entity_id
             FROM meeting_entities me
             WHERE me.meeting_id = ?1
         )
         ORDER BY se.created_at DESC
         LIMIT 20";
    if let Ok(mut stmt) = conn.prepare(signal_query) {
        if let Ok(rows) = stmt.query_map(rusqlite::params![meeting_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
            ))
        }) {
            for row in rows.flatten() {
                let (signal_type, source, value, _entity_id) = row;
                let line = match value {
                    Some(v) if !v.is_empty() => {
                        format!("- {} ({}): {}", signal_type, source, v)
                    }
                    _ => format!("- {} ({})", signal_type, source),
                };
                signal_summaries.push(line);
            }
        }
    }

    // Attendees (raw JSON string from DB)
    let attendees = meeting
        .attendees
        .clone()
        .unwrap_or_else(|| "[]".to_string());

    // Description
    let description = meeting
        .description
        .clone()
        .unwrap_or_default();

    // Prior intelligence for incremental refresh
    let prior_intelligence = meeting.prep_context_json.clone();

    MeetingEnrichmentContext {
        title: meeting.title.clone(),
        meeting_type: meeting.meeting_type.clone(),
        start_time: meeting.start_time.clone(),
        attendees,
        description,
        entity_summaries,
        signal_summaries,
        prior_intelligence,
    }
}

/// Build the AI enrichment prompt from gathered context.
fn build_enrichment_prompt(ctx: &MeetingEnrichmentContext) -> String {
    let mut prompt = String::with_capacity(4096);

    prompt.push_str(
        "You are a meeting intelligence analyst for an executive's daily operating system. \
         Analyze this meeting and provide strategic intelligence to help the user prepare.\n\n",
    );

    prompt.push_str("<meeting>\n");
    prompt.push_str(&format!("Title: <user_data>{}</user_data>\n", ctx.title));
    prompt.push_str(&format!("Type: {}\n", ctx.meeting_type));
    prompt.push_str(&format!("Start: {}\n", ctx.start_time));
    prompt.push_str(&format!(
        "Attendees: <user_data>{}</user_data>\n",
        ctx.attendees
    ));
    if !ctx.description.is_empty() {
        prompt.push_str(&format!(
            "Description: <user_data>{}</user_data>\n",
            ctx.description
        ));
    }
    prompt.push_str("</meeting>\n\n");

    if !ctx.entity_summaries.is_empty() {
        prompt.push_str("<entity_context>\n");
        for line in &ctx.entity_summaries {
            prompt.push_str(line);
            prompt.push('\n');
        }
        prompt.push_str("</entity_context>\n\n");
    }

    if !ctx.signal_summaries.is_empty() {
        prompt.push_str("<active_signals>\n");
        for line in &ctx.signal_summaries {
            prompt.push_str(line);
            prompt.push('\n');
        }
        prompt.push_str("</active_signals>\n\n");
    }

    if let Some(ref prior) = ctx.prior_intelligence {
        if !prior.is_empty() {
            prompt.push_str("<prior_intelligence>\n");
            prompt.push_str(prior);
            prompt.push_str("\n</prior_intelligence>\n\n");
            prompt.push_str(
                "The above is prior intelligence. Update and refine it with any new context.\n\n",
            );
        }
    }

    prompt.push_str(
        "Respond with ONLY a valid JSON object (no markdown fences, no explanation):\n\
         {\n\
         \x20 \"narrative\": \"2-3 sentence strategic briefing about this meeting\",\n\
         \x20 \"risks\": [\"risk or concern 1\", ...],\n\
         \x20 \"talking_points\": [\"key point to raise 1\", ...],\n\
         \x20 \"stakeholder_notes\": [\"context about key attendee 1\", ...],\n\
         \x20 \"agenda_suggestions\": [\"suggested agenda item 1\", ...]\n\
         }\n\n\
         Constraints:\n\
         - narrative: exactly 2-3 sentences, strategic framing\n\
         - risks: max 20 items, most important first\n\
         - talking_points: max 10 items\n\
         - stakeholder_notes: max 10 items\n\
         - agenda_suggestions: max 10 items\n\
         - If insufficient context for a field, use an empty array []\n\
         - Be specific and actionable, not generic",
    );

    prompt
}

/// Truncate JSON arrays to cap sizes per I296 pattern.
fn cap_enrichment_arrays(val: &mut serde_json::Value) {
    if let Some(obj) = val.as_object_mut() {
        let caps = [
            ("risks", 20),
            ("talking_points", 10),
            ("stakeholder_notes", 10),
            ("agenda_suggestions", 10),
        ];
        for (key, max) in caps {
            if let Some(arr) = obj.get_mut(key).and_then(|v| v.as_array_mut()) {
                arr.truncate(max);
            }
        }
    }
}

/// AI enrichment for a single meeting (I326 Phase 2).
///
/// Split-lock pattern: gather context with DB lock, release, run PTY, return result.
/// Caller writes results to DB.
async fn enrich_meeting_with_ai(
    state: &AppState,
    meeting_id: &str,
    meeting: &crate::db::DbMeeting,
) -> Result<serde_json::Value, String> {
    // Phase 1: Gather context (brief DB lock)
    let ctx = {
        let guard = state
            .db
            .lock()
            .map_err(|_| "DB lock poisoned".to_string())?;
        let db = guard
            .as_ref()
            .ok_or_else(|| "Database not initialized".to_string())?;
        gather_meeting_context(db, meeting_id, meeting)
    };
    // DB lock released here

    // Phase 2: Build prompt
    let prompt = build_enrichment_prompt(&ctx);

    // Get workspace and AI config (read locks, not mutex)
    let workspace = {
        let config_guard = state
            .config
            .read()
            .map_err(|_| "Config lock poisoned".to_string())?;
        let config = config_guard
            .as_ref()
            .ok_or_else(|| "No config loaded".to_string())?;
        PathBuf::from(&config.workspace_path)
    };

    let ai_config = {
        let config_guard = state
            .config
            .read()
            .map_err(|_| "Config lock poisoned".to_string())?;
        let config = config_guard
            .as_ref()
            .ok_or_else(|| "No config loaded".to_string())?;
        config.ai_models.clone()
    };

    // Phase 3: Run PTY (no DB lock held — spawn_blocking for sync PTY call)
    let pty_result = tokio::task::spawn_blocking(move || {
        let pty = PtyManager::for_tier(ModelTier::Synthesis, &ai_config)
            .with_timeout(120)
            .with_nice_priority(10);
        pty.spawn_claude(&workspace, &prompt)
    })
    .await
    .map_err(|e| format!("PTY task join error: {}", e))?;

    let output = pty_result.map_err(|e| format!("PTY error: {}", e))?;

    // Phase 4: Parse response
    let json_str = crate::intelligence::extract_json_from_response(&output.stdout)
        .ok_or_else(|| "No JSON found in AI response".to_string())?;

    let mut parsed: serde_json::Value =
        serde_json::from_str(json_str).map_err(|e| format!("JSON parse error: {}", e))?;

    // Cap array sizes
    cap_enrichment_arrays(&mut parsed);

    Ok(parsed)
}

/// Generate or refresh intelligence for a single meeting.
///
/// Idempotent: calling twice does incremental update, not duplicate work.
/// Performs mechanical quality assessment, then AI enrichment if quality >= Developing.
pub async fn generate_meeting_intelligence(
    state: &AppState,
    meeting_id: &str,
    force_full: bool,
) -> Result<IntelligenceQuality, ExecutionError> {
    // 1. Load meeting from DB
    let (meeting_state, has_new) = {
        let guard = state.db.lock().map_err(|_| {
            ExecutionError::ConfigurationError("DB lock poisoned".to_string())
        })?;
        let db = guard.as_ref().ok_or_else(|| {
            ExecutionError::ConfigurationError("Database not initialized".to_string())
        })?;

        let meeting = db
            .get_meeting_by_id(meeting_id)
            .map_err(|e| ExecutionError::ConfigurationError(e.to_string()))?
            .ok_or_else(|| {
                ExecutionError::ConfigurationError(format!(
                    "Meeting not found: {}",
                    meeting_id
                ))
            })?;

        let intel_state = meeting.intelligence_state.clone();
        let has_new = meeting.has_new_signals.unwrap_or(0);
        (intel_state, has_new)
    };

    // 2. Decide whether work is needed
    if meeting_state.as_deref() == Some("enriched") && !force_full {
        if has_new == 0 {
            // No new signals — return current quality without extra work
            let quality = {
                let guard = state.db.lock().map_err(|_| {
                    ExecutionError::ConfigurationError("DB lock poisoned".to_string())
                })?;
                let db = guard.as_ref().ok_or_else(|| {
                    ExecutionError::ConfigurationError("Database not initialized".to_string())
                })?;
                assess_intelligence_quality(db, meeting_id)
            };
            return Ok(quality);
        }
        // Has new signals: set state to "refreshing"
        let guard = state.db.lock().map_err(|_| {
            ExecutionError::ConfigurationError("DB lock poisoned".to_string())
        })?;
        let db = guard.as_ref().ok_or_else(|| {
            ExecutionError::ConfigurationError("Database not initialized".to_string())
        })?;
        let _ = db.update_intelligence_state(meeting_id, "refreshing", None, None);
    } else if meeting_state.as_deref() != Some("enriched") || force_full {
        // No intelligence exists (detected) or force_full: set state to "enriching"
        let guard = state.db.lock().map_err(|_| {
            ExecutionError::ConfigurationError("DB lock poisoned".to_string())
        })?;
        let db = guard.as_ref().ok_or_else(|| {
            ExecutionError::ConfigurationError("Database not initialized".to_string())
        })?;
        let _ = db.update_intelligence_state(meeting_id, "enriching", None, None);
    }

    // 3. Run mechanical quality assessment
    let quality = {
        let guard = state.db.lock().map_err(|_| {
            ExecutionError::ConfigurationError("DB lock poisoned".to_string())
        })?;
        let db = guard.as_ref().ok_or_else(|| {
            ExecutionError::ConfigurationError("Database not initialized".to_string())
        })?;
        assess_intelligence_quality(db, meeting_id)
    };

    // 4. Quality assessment is stored via update_intelligence_state in step 5.
    // Do NOT write bare quality JSON to prep_context_json — that column is
    // reserved for FullMeetingPrep data. Writing quality there corrupts
    // deserialization and shows empty briefings.

    // 4.5 AI enrichment if quality >= Developing (I326 Phase 2)
    // Sparse meetings get mechanical prep from MeetingPrepQueue instead.
    let mut ai_enrichment_succeeded = false;
    if quality.level >= QualityLevel::Developing {
        let meeting_for_ai = {
            let guard = state.db.lock().map_err(|_| {
                ExecutionError::ConfigurationError("DB lock poisoned".to_string())
            })?;
            let db = guard.as_ref().ok_or_else(|| {
                ExecutionError::ConfigurationError("Database not initialized".to_string())
            })?;
            db.get_meeting_by_id(meeting_id)
                .map_err(|e| ExecutionError::ConfigurationError(e.to_string()))?
                .ok_or_else(|| {
                    ExecutionError::ConfigurationError(format!(
                        "Meeting not found: {}",
                        meeting_id
                    ))
                })?
        };

        match enrich_meeting_with_ai(state, meeting_id, &meeting_for_ai).await {
            Ok(ai_json) => {
                let now = Utc::now().to_rfc3339();

                let guard = state.db.lock().map_err(|_| {
                    ExecutionError::ConfigurationError("DB lock poisoned".to_string())
                })?;
                let db = guard.as_ref().ok_or_else(|| {
                    ExecutionError::ConfigurationError("Database not initialized".to_string())
                })?;

                // Read existing prep_context_json and merge AI fields into it
                // instead of replacing — preserves FullMeetingPrep structure
                let existing: Option<String> = db.conn_ref()
                    .query_row(
                        "SELECT prep_context_json FROM meetings_history WHERE id = ?1",
                        rusqlite::params![meeting_id],
                        |row| row.get(0),
                    )
                    .ok();

                let merged_str = if let Some(ref existing_str) = existing {
                    if let Ok(mut existing_json) = serde_json::from_str::<serde_json::Value>(existing_str) {
                        // Overlay AI fields onto existing prep data
                        if let Some(obj) = existing_json.as_object_mut() {
                            obj.insert("quality".to_string(), json!(&quality));
                            obj.insert("ai_intelligence".to_string(), ai_json.clone());
                        }
                        serde_json::to_string(&existing_json).unwrap_or_default()
                    } else {
                        // Existing data is unparseable — write AI-only
                        serde_json::to_string(&json!({
                            "quality": &quality,
                            "ai_intelligence": ai_json,
                        })).unwrap_or_default()
                    }
                } else {
                    // No existing data — write AI-only
                    serde_json::to_string(&json!({
                        "quality": &quality,
                        "ai_intelligence": ai_json,
                    })).unwrap_or_default()
                };

                let _ = db.conn_ref().execute(
                    "UPDATE meetings_history SET prep_context_json = ?1, last_enriched_at = ?2 WHERE id = ?3",
                    rusqlite::params![merged_str, now, meeting_id],
                );
                ai_enrichment_succeeded = true;
                log::info!(
                    "AI enrichment complete for meeting {}",
                    meeting_id
                );
            }
            Err(e) => {
                log::warn!(
                    "AI enrichment failed for meeting {}: {} — will retry on next refresh",
                    meeting_id,
                    e
                );
            }
        }
    }

    // 5. Update DB: state depends on whether AI enrichment succeeded
    {
        let new_state = if ai_enrichment_succeeded { "enriched" } else { "detected" };
        let guard = state.db.lock().map_err(|_| {
            ExecutionError::ConfigurationError("DB lock poisoned".to_string())
        })?;
        let db = guard.as_ref().ok_or_else(|| {
            ExecutionError::ConfigurationError("Database not initialized".to_string())
        })?;
        db.update_intelligence_state(
            meeting_id,
            new_state,
            Some(&quality.level.to_string()),
            Some(quality.signal_count as i32),
        )
        .map_err(|e| ExecutionError::ConfigurationError(e.to_string()))?;

        if ai_enrichment_succeeded {
            let _ = db.clear_meeting_new_signals(meeting_id);
        }
    }

    Ok(quality)
}
