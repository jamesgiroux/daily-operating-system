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

/// Generate or refresh intelligence for a single meeting.
///
/// Idempotent: calling twice does incremental update, not duplicate work.
/// For this initial implementation, only mechanical assessment is performed
/// (no AI enrichment). AI enrichment will be added in Phase 2.
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

    // 4. If quality >= Developing, write quality assessment to prep_context_json
    if quality.level >= QualityLevel::Developing {
        let quality_json = serde_json::to_string(&quality).unwrap_or_default();
        let guard = state.db.lock().map_err(|_| {
            ExecutionError::ConfigurationError("DB lock poisoned".to_string())
        })?;
        let db = guard.as_ref().ok_or_else(|| {
            ExecutionError::ConfigurationError("Database not initialized".to_string())
        })?;
        let _ = db.conn_ref().execute(
            "UPDATE meetings_history SET prep_context_json = ?1 WHERE id = ?2
             AND (prep_context_json IS NULL OR prep_context_json = '')",
            rusqlite::params![quality_json, meeting_id],
        );
    }

    // 5. Update DB: state = enriched, quality level, signal_count, has_new_signals = 0
    {
        let guard = state.db.lock().map_err(|_| {
            ExecutionError::ConfigurationError("DB lock poisoned".to_string())
        })?;
        let db = guard.as_ref().ok_or_else(|| {
            ExecutionError::ConfigurationError("Database not initialized".to_string())
        })?;
        db.update_intelligence_state(
            meeting_id,
            "enriched",
            Some(&quality.level.to_string()),
            Some(quality.signal_count as i32),
        )
        .map_err(|e| ExecutionError::ConfigurationError(e.to_string()))?;

        let _ = db.clear_meeting_new_signals(meeting_id);
    }

    Ok(quality)
}
