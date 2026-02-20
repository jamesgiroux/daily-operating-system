//! Signal event CRUD and source tier weights (ADR-0080).

use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::{ActionDb, DbError};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A row from the `signal_events` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignalEvent {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub signal_type: String,
    pub source: String,
    pub value: Option<String>,
    pub confidence: f64,
    pub decay_half_life_days: i32,
    pub created_at: String,
    pub superseded_by: Option<String>,
    /// Context tag for the signal source (e.g. "inbound_email", "outbound_email").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_context: Option<String>,
}

// ---------------------------------------------------------------------------
// Source tier weights (ADR-0080)
// ---------------------------------------------------------------------------

/// Base reliability weight for a signal source.
///
/// Tier 1 (highest): user corrections, transcript evidence
/// Tier 2: attendee patterns, email threads
/// Tier 3: third-party enrichment (Clay, Gravatar)
/// Tier 4 (lowest): keyword heuristics, AI inference
pub fn source_base_weight(source: &str) -> f64 {
    match source {
        "user_correction" | "explicit" => 1.0,
        "transcript" | "notes" => 0.9,
        "attendee" | "attendee_vote" | "email_thread" | "junction" => 0.8,
        "group_pattern" => 0.75,
        "proactive" => 0.7,
        "clay" | "gravatar" => 0.6,
        "keyword" | "keyword_fuzzy" | "heuristic" | "embedding" => 0.4,
        _ => 0.5,
    }
}

/// Default half-life in days for a signal source.
pub fn default_half_life(source: &str) -> i32 {
    match source {
        "user_correction" | "explicit" => 365,
        "transcript" | "notes" => 60,
        "attendee" | "attendee_vote" | "junction" => 30,
        "group_pattern" => 60,
        "proactive" => 3,
        "clay" | "gravatar" => 90,
        "keyword" | "keyword_fuzzy" | "heuristic" | "embedding" => 7,
        _ => 30,
    }
}

// ---------------------------------------------------------------------------
// Signal event operations
// ---------------------------------------------------------------------------

/// Emit a new signal event. Returns the generated signal ID.
pub fn emit_signal(
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    signal_type: &str,
    source: &str,
    value: Option<&str>,
    confidence: f64,
) -> Result<String, DbError> {
    let id = format!("sig-{}", Uuid::new_v4());
    let half_life = default_half_life(source);
    db.insert_signal_event(&id, entity_type, entity_id, signal_type, source, value, confidence, half_life)?;
    Ok(id)
}

/// Emit a new signal event with context tagging. Returns the generated signal ID.
#[allow(clippy::too_many_arguments)]
pub fn emit_signal_with_context(
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    signal_type: &str,
    source: &str,
    value: Option<&str>,
    confidence: f64,
    source_context: Option<&str>,
) -> Result<String, DbError> {
    let id = format!("sig-{}", Uuid::new_v4());
    let half_life = default_half_life(source);
    db.insert_signal_event_with_context(
        &id, entity_type, entity_id, signal_type, source, value,
        confidence, half_life, source_context,
    )?;
    Ok(id)
}

/// Emit a signal and run propagation rules, returning the original signal ID
/// and any derived signal IDs.
#[allow(clippy::too_many_arguments)]
pub fn emit_signal_and_propagate(
    db: &ActionDb,
    engine: &super::propagation::PropagationEngine,
    entity_type: &str,
    entity_id: &str,
    signal_type: &str,
    source: &str,
    value: Option<&str>,
    confidence: f64,
) -> Result<(String, Vec<String>), DbError> {
    let id = emit_signal(db, entity_type, entity_id, signal_type, source, value, confidence)?;

    // Read back the signal for propagation
    let signal = SignalEvent {
        id: id.clone(),
        entity_type: entity_type.to_string(),
        entity_id: entity_id.to_string(),
        signal_type: signal_type.to_string(),
        source: source.to_string(),
        value: value.map(|s| s.to_string()),
        confidence,
        decay_half_life_days: default_half_life(source),
        created_at: Utc::now().to_rfc3339(),
        superseded_by: None,
        source_context: None,
    };

    let derived_ids = engine.propagate(db, &signal)?;

    // Propagate signal to linked meetings (ADR-0081 Phase 4A)
    if let Err(e) = propagate_signal_to_meetings(db, entity_id) {
        log::warn!("Failed to propagate signal to meetings: {}", e);
    }

    Ok((id, derived_ids))
}

/// When a signal is emitted for an entity, flag all future meetings
/// linked to that entity as having new signals.
pub fn propagate_signal_to_meetings(
    db: &ActionDb,
    entity_id: &str,
) -> Result<usize, DbError> {
    let conn = db.conn_ref();
    let mut stmt = conn.prepare(
        "SELECT me.meeting_id FROM meeting_entities me
         INNER JOIN meetings_history mh ON mh.id = me.meeting_id
         WHERE me.entity_id = ?1
         AND mh.start_time > datetime('now')
         AND mh.intelligence_state != 'archived'"
    )?;

    let meeting_ids: Vec<String> = stmt
        .query_map(params![entity_id], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();

    let count = meeting_ids.len();
    for meeting_id in &meeting_ids {
        db.mark_meeting_new_signals(meeting_id)?;
    }

    if count > 0 {
        log::info!(
            "Propagated signal for entity {} to {} future meeting(s)",
            entity_id,
            count
        );
    }

    Ok(count)
}

/// Mark an old signal as superseded by a new one.
pub fn supersede_signal(db: &ActionDb, old_id: &str, new_id: &str) -> Result<(), DbError> {
    db.update_signal_superseded(old_id, new_id)
}

/// Get all active (non-superseded) signals for an entity.
pub fn get_active_signals(
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
) -> Result<Vec<SignalEvent>, DbError> {
    db.get_signal_events(entity_type, entity_id, None)
}

/// Get active signals filtered by signal_type.
pub fn get_active_signals_by_type(
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    signal_type: &str,
) -> Result<Vec<SignalEvent>, DbError> {
    db.get_signal_events(entity_type, entity_id, Some(signal_type))
}

/// Read the learned reliability for a source from the signal_weights table.
///
/// When the system has enough data (>= 5 updates), uses Thompson Sampling
/// to explore/exploit weight learning. Otherwise returns 0.5 (uninformative prior).
pub fn get_learned_reliability(
    db: &ActionDb,
    source: &str,
    entity_type: &str,
    signal_type: &str,
) -> f64 {
    match db.get_signal_weight(source, entity_type, signal_type) {
        Ok(Some((alpha, beta, update_count))) => {
            if update_count >= 5 {
                super::sampling::sample_reliability(alpha, beta)
            } else {
                0.5
            }
        }
        _ => 0.5,
    }
}

// ---------------------------------------------------------------------------
// ActionDb methods
// ---------------------------------------------------------------------------

impl ActionDb {
    /// Insert a signal event row.
    #[allow(clippy::too_many_arguments)]
    pub fn insert_signal_event(
        &self,
        id: &str,
        entity_type: &str,
        entity_id: &str,
        signal_type: &str,
        source: &str,
        value: Option<&str>,
        confidence: f64,
        decay_half_life_days: i32,
    ) -> Result<(), DbError> {
        self.conn_ref().execute(
            "INSERT INTO signal_events
                (id, entity_type, entity_id, signal_type, source, value, confidence, decay_half_life_days)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![id, entity_type, entity_id, signal_type, source, value, confidence, decay_half_life_days],
        )?;
        Ok(())
    }

    /// Insert a signal event row with source_context.
    #[allow(clippy::too_many_arguments)]
    pub fn insert_signal_event_with_context(
        &self,
        id: &str,
        entity_type: &str,
        entity_id: &str,
        signal_type: &str,
        source: &str,
        value: Option<&str>,
        confidence: f64,
        decay_half_life_days: i32,
        source_context: Option<&str>,
    ) -> Result<(), DbError> {
        self.conn_ref().execute(
            "INSERT INTO signal_events
                (id, entity_type, entity_id, signal_type, source, value, confidence, decay_half_life_days, source_context)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![id, entity_type, entity_id, signal_type, source, value, confidence, decay_half_life_days, source_context],
        )?;
        Ok(())
    }

    /// Query non-superseded signal events for an entity, optionally filtered by signal_type.
    pub fn get_signal_events(
        &self,
        entity_type: &str,
        entity_id: &str,
        signal_type: Option<&str>,
    ) -> Result<Vec<SignalEvent>, DbError> {
        let (sql, params_vec): (&str, Vec<Box<dyn rusqlite::types::ToSql>>) = match signal_type {
            Some(st) => (
                "SELECT id, entity_type, entity_id, signal_type, source, value,
                        confidence, decay_half_life_days, created_at, superseded_by
                 FROM signal_events
                 WHERE entity_type = ?1 AND entity_id = ?2 AND signal_type = ?3
                   AND superseded_by IS NULL
                 ORDER BY created_at DESC",
                vec![
                    Box::new(entity_type.to_string()),
                    Box::new(entity_id.to_string()),
                    Box::new(st.to_string()),
                ],
            ),
            None => (
                "SELECT id, entity_type, entity_id, signal_type, source, value,
                        confidence, decay_half_life_days, created_at, superseded_by
                 FROM signal_events
                 WHERE entity_type = ?1 AND entity_id = ?2
                   AND superseded_by IS NULL
                 ORDER BY created_at DESC",
                vec![
                    Box::new(entity_type.to_string()),
                    Box::new(entity_id.to_string()),
                ],
            ),
        };

        let mut stmt = self.conn_ref().prepare(sql)?;
        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(param_refs.as_slice(), |row| {
            Ok(SignalEvent {
                id: row.get(0)?,
                entity_type: row.get(1)?,
                entity_id: row.get(2)?,
                signal_type: row.get(3)?,
                source: row.get(4)?,
                value: row.get(5)?,
                confidence: row.get(6)?,
                decay_half_life_days: row.get(7)?,
                created_at: row.get(8)?,
                superseded_by: row.get(9)?,
                source_context: None,
            })
        })?;

        let mut events = Vec::new();
        for row in rows {
            events.push(row?);
        }
        Ok(events)
    }

    /// Mark a signal as superseded.
    pub fn update_signal_superseded(&self, old_id: &str, new_id: &str) -> Result<(), DbError> {
        self.conn_ref().execute(
            "UPDATE signal_events SET superseded_by = ?2 WHERE id = ?1",
            params![old_id, new_id],
        )?;
        Ok(())
    }

    /// Read a signal_weight row. Returns (alpha, beta, update_count) or None if no row.
    pub fn get_signal_weight(
        &self,
        source: &str,
        entity_type: &str,
        signal_type: &str,
    ) -> Result<Option<(f64, f64, i32)>, DbError> {
        match self.conn_ref().query_row(
            "SELECT alpha, beta, update_count FROM signal_weights
             WHERE source = ?1 AND entity_type = ?2 AND signal_type = ?3",
            params![source, entity_type, signal_type],
            |row| Ok((row.get::<_, f64>(0)?, row.get::<_, f64>(1)?, row.get::<_, i32>(2)?)),
        ) {
            Ok(triple) => Ok(Some(triple)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DbError::Sqlite(e)),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> ActionDb {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("test.db");
        std::mem::forget(dir);
        ActionDb::open_at(path).expect("open")
    }

    #[test]
    fn test_source_base_weights() {
        assert_eq!(source_base_weight("user_correction"), 1.0);
        assert_eq!(source_base_weight("transcript"), 0.9);
        assert_eq!(source_base_weight("attendee_vote"), 0.8);
        assert_eq!(source_base_weight("clay"), 0.6);
        assert_eq!(source_base_weight("keyword"), 0.4);
        assert_eq!(source_base_weight("unknown"), 0.5);
    }

    #[test]
    fn test_default_half_lives() {
        assert_eq!(default_half_life("user_correction"), 365);
        assert_eq!(default_half_life("transcript"), 60);
        assert_eq!(default_half_life("clay"), 90);
        assert_eq!(default_half_life("heuristic"), 7);
    }

    #[test]
    fn test_emit_and_get_signals() {
        let db = test_db();
        let id = emit_signal(&db, "account", "acme-1", "entity_resolution", "keyword", Some("name match"), 0.8)
            .expect("emit");
        assert!(id.starts_with("sig-"));

        let signals = get_active_signals(&db, "account", "acme-1").expect("get");
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].signal_type, "entity_resolution");
        assert_eq!(signals[0].source, "keyword");
        assert!((signals[0].confidence - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_supersede_excludes_old() {
        let db = test_db();
        let old_id = emit_signal(&db, "person", "p1", "profile_update", "clay", None, 0.7).expect("emit old");
        let new_id = emit_signal(&db, "person", "p1", "profile_update", "clay", None, 0.85).expect("emit new");

        supersede_signal(&db, &old_id, &new_id).expect("supersede");

        let active = get_active_signals(&db, "person", "p1").expect("get");
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, new_id);
    }

    #[test]
    fn test_get_signals_by_type() {
        let db = test_db();
        emit_signal(&db, "account", "a1", "entity_resolution", "keyword", None, 0.8).expect("emit 1");
        emit_signal(&db, "account", "a1", "pre_meeting_context", "email_thread", None, 0.7).expect("emit 2");

        let resolution_only = get_active_signals_by_type(&db, "account", "a1", "entity_resolution").expect("get");
        assert_eq!(resolution_only.len(), 1);
        assert_eq!(resolution_only[0].signal_type, "entity_resolution");
    }

    #[test]
    fn test_learned_reliability_default() {
        let db = test_db();
        let reliability = get_learned_reliability(&db, "clay", "person", "profile_update");
        assert!((reliability - 0.5).abs() < 0.01, "uninformative prior should be 0.5");
    }
}
