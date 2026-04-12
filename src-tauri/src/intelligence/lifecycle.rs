//! Intelligence lifecycle management (ADR-0081).
//!
//! Independent, idempotent functions for assessing and generating meeting
//! intelligence. These can be called from any context: daily orchestrator,
//! weekly run, calendar polling, or user-triggered refresh.

use chrono::Utc;

use crate::db::ActionDb;
use crate::error::ExecutionError;
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
pub fn assess_intelligence_quality(db: &ActionDb, meeting_id: &str) -> IntelligenceQuality {
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
                "SELECT COUNT(*) FROM meetings m
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
                 WHERE a.status IN ('backlog', 'unstarted', 'started')
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

// Meeting-level AI enrichment removed per ADR-0086 (I376).
// Entity intelligence is enriched via intel_queue; meeting prep is assembled
// mechanically by MeetingPrepQueue from pre-computed intelligence.json files.

/// Generate or refresh intelligence for a single meeting (ADR-0086).
///
/// Idempotent: calling twice does incremental update, not duplicate work.
/// `force_full=true` delegates to the single-service full briefing refresh
/// (`services::meetings::refresh_meeting_briefing_full`).
pub async fn generate_meeting_intelligence(
    state: &AppState,
    meeting_id: &str,
    force_full: bool,
) -> Result<IntelligenceQuality, ExecutionError> {
    // 1. Load meeting from DB
    let meeting_id_owned = meeting_id.to_string();
    let (meeting_state, has_new) = state
        .db_read(move |db| {
            let meeting = db
                .get_meeting_by_id(&meeting_id_owned)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("Meeting not found: {}", meeting_id_owned))?;
            let intel_state = meeting.intelligence_state.clone();
            let has_new = meeting.has_new_signals.unwrap_or(0);
            Ok((intel_state, has_new))
        })
        .await
        .map_err(ExecutionError::ConfigurationError)?;

    if force_full {
        let refreshed =
            crate::services::meetings::refresh_meeting_briefing_full(state, meeting_id, None)
                .await
                .map_err(ExecutionError::ConfigurationError)?;
        return Ok(refreshed.quality);
    }

    // 2. Decide whether work is needed
    if meeting_state.as_deref() == Some("enriched") {
        if has_new == 0 {
            // No new signals: only skip if intelligence is still current.
            // Stale/aging meetings need a full rebuild to refresh temporal framing.
            let mid = meeting_id.to_string();
            let quality = state
                .db_read(move |db| Ok(assess_intelligence_quality(db, &mid)))
                .await
                .map_err(ExecutionError::ConfigurationError)?;
            if quality.staleness == Staleness::Current {
                return Ok(quality);
            }
            log::info!(
                "generate_meeting_intelligence: {} stale without new signals; forcing full refresh",
                meeting_id
            );
            let refreshed =
                crate::services::meetings::refresh_meeting_briefing_full(state, meeting_id, None)
                    .await
                    .map_err(ExecutionError::ConfigurationError)?;
            return Ok(refreshed.quality);
        }
        // Has new signals: set state to "refreshing"
        let mid = meeting_id.to_string();
        let _ = state
            .db_write(move |db| {
                let _ = db.update_intelligence_state(&mid, "refreshing", None, None);
                Ok(())
            })
            .await;
    } else if meeting_state.as_deref() != Some("enriched") {
        // No intelligence exists (detected): set state to "enriching"
        let mid = meeting_id.to_string();
        let _ = state
            .db_write(move |db| {
                let _ = db.update_intelligence_state(&mid, "enriching", None, None);
                Ok(())
            })
            .await;
    }

    // 3. Run mechanical quality assessment
    let mid = meeting_id.to_string();
    let quality = state
        .db_read(move |db| Ok(assess_intelligence_quality(db, &mid)))
        .await
        .map_err(ExecutionError::ConfigurationError)?;

    // 4. Enqueue meeting prep regeneration.
    state
        .meeting_prep_queue
        .enqueue(crate::meeting_prep_queue::PrepRequest::new(
            meeting_id.to_string(),
            crate::meeting_prep_queue::PrepPriority::Manual,
        ));
    state.integrations.prep_queue_wake.notify_one();

    log::info!(
        "generate_meeting_intelligence: processed {} (force={}, quality={:?})",
        meeting_id,
        force_full,
        quality.level,
    );

    // 5. Update DB: mark as "enriched" — intelligence comes from entity level,
    // meeting prep is mechanical assembly.
    let mid = meeting_id.to_string();
    let quality_level = quality.level.to_string();
    state
        .db_write(move |db| {
            db.update_intelligence_state(
                &mid,
                "enriched",
                Some(&quality_level),
                Some(quality.signal_count as i32),
            )
            .map_err(|e| e.to_string())?;
            let _ = db.clear_meeting_new_signals(&mid);
            Ok(())
        })
        .await
        .map_err(ExecutionError::ConfigurationError)?;

    Ok(quality)
}
