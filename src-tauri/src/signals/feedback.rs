//! Correction recording and weight updates (I307 / ADR-0080 Phase 3).
//!
//! When a user corrects a meeting-entity assignment, this module:
//! 1. Records the correction in `entity_resolution_feedback`
//! 2. Identifies which signal source was wrong
//! 3. Penalizes the wrong source (increment beta)
//! 4. Rewards sources that pointed to the correct entity (increment alpha)

use rusqlite::params;
use uuid::Uuid;

use crate::db::{ActionDb, DbError};
use crate::signals::bus::SignalEvent;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Record a user correction and update signal weights accordingly.
///
/// `old_entities` contains (entity_id, entity_type) pairs that were previously linked.
/// `new_entity_id` is the entity the user corrected to (None for removals).
pub fn record_correction(
    db: &ActionDb,
    meeting_id: &str,
    old_entities: &[(String, String)],
    new_entity_id: &str,
    new_entity_type: &str,
) -> Result<(), DbError> {
    // Find which signal source led to the wrong resolution
    let resolution_signals = db.get_resolution_signals_for_meeting(meeting_id)?;

    for (old_id, old_type) in old_entities {
        // Find the signal source that recommended the old (wrong) entity
        let wrong_source = resolution_signals
            .iter()
            .find(|s| s.entity_id == *old_id && s.entity_type == *old_type)
            .map(|s| s.source.clone());

        let source_str = wrong_source.as_deref();

        // Record feedback row
        db.insert_resolution_feedback(
            meeting_id,
            Some(old_id),
            Some(old_type),
            Some(new_entity_id),
            Some(new_entity_type),
            source_str,
        )?;

        // Update weights if we know which source was wrong
        if let Some(ref wrong) = wrong_source {
            update_weights_from_correction(db, wrong, old_type, "entity_resolution")?;
        }
    }

    // Reward sources that pointed to the correct entity
    for signal in &resolution_signals {
        if signal.entity_id == new_entity_id
            && signal.entity_type == new_entity_type
        {
            // This source was correct — increment alpha
            db.upsert_signal_weight(
                &signal.source,
                &signal.entity_type,
                "entity_resolution",
                1.0, // alpha_delta
                0.0, // beta_delta
            )?;
        }
    }

    Ok(())
}

/// Record a removal correction (user removed an entity link entirely).
pub fn record_removal(
    db: &ActionDb,
    meeting_id: &str,
    removed_entity_id: &str,
    removed_entity_type: &str,
) -> Result<(), DbError> {
    let resolution_signals = db.get_resolution_signals_for_meeting(meeting_id)?;

    let wrong_source = resolution_signals
        .iter()
        .find(|s| s.entity_id == removed_entity_id && s.entity_type == removed_entity_type)
        .map(|s| s.source.clone());

    let source_str = wrong_source.as_deref();

    db.insert_resolution_feedback(
        meeting_id,
        Some(removed_entity_id),
        Some(removed_entity_type),
        None,
        None,
        source_str,
    )?;

    if let Some(ref wrong) = wrong_source {
        update_weights_from_correction(db, wrong, removed_entity_type, "entity_resolution")?;
    }

    Ok(())
}

/// Penalize a signal source that produced a wrong resolution.
///
/// Increments beta (failure count) for the wrong source.
fn update_weights_from_correction(
    db: &ActionDb,
    wrong_source: &str,
    entity_type: &str,
    signal_type: &str,
) -> Result<(), DbError> {
    // Penalize wrong source: increment beta
    db.upsert_signal_weight(wrong_source, entity_type, signal_type, 0.0, 1.0)
}

// ---------------------------------------------------------------------------
// ActionDb methods
// ---------------------------------------------------------------------------

impl ActionDb {
    /// Insert a row into entity_resolution_feedback.
    pub fn insert_resolution_feedback(
        &self,
        meeting_id: &str,
        old_entity_id: Option<&str>,
        old_entity_type: Option<&str>,
        new_entity_id: Option<&str>,
        new_entity_type: Option<&str>,
        signal_source: Option<&str>,
    ) -> Result<(), DbError> {
        let id = format!("fb-{}", Uuid::new_v4());
        self.conn_ref().execute(
            "INSERT INTO entity_resolution_feedback
                (id, meeting_id, old_entity_id, old_entity_type, new_entity_id, new_entity_type, signal_source)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, meeting_id, old_entity_id, old_entity_type, new_entity_id, new_entity_type, signal_source],
        )?;
        Ok(())
    }

    /// Get entity_resolution signals for a meeting (to identify which source was wrong).
    pub fn get_resolution_signals_for_meeting(
        &self,
        meeting_id: &str,
    ) -> Result<Vec<SignalEvent>, DbError> {
        let pattern = format!("%\"event_id\":\"{}\"%", meeting_id);
        let mut stmt = self.conn_ref().prepare(
            "SELECT id, entity_type, entity_id, signal_type, source, value,
                    confidence, decay_half_life_days, created_at, superseded_by,
                    source_context
             FROM signal_events
             WHERE signal_type = 'entity_resolution'
               AND superseded_by IS NULL
               AND value LIKE ?1
             ORDER BY created_at DESC",
        )?;

        let rows = stmt.query_map(params![pattern], Self::map_signal_event_row)?;

        let mut events = Vec::new();
        for row in rows {
            events.push(row?);
        }
        Ok(events)
    }

    /// Upsert signal_weights: increment alpha/beta by the given deltas.
    ///
    /// Uses INSERT ... ON CONFLICT to create with prior (1+delta) or update existing.
    pub fn upsert_signal_weight(
        &self,
        source: &str,
        entity_type: &str,
        signal_type: &str,
        alpha_delta: f64,
        beta_delta: f64,
    ) -> Result<(), DbError> {
        self.conn_ref().execute(
            "INSERT INTO signal_weights (source, entity_type, signal_type, alpha, beta, update_count)
             VALUES (?1, ?2, ?3, 1.0 + ?4, 1.0 + ?5, 1)
             ON CONFLICT (source, entity_type, signal_type) DO UPDATE SET
                alpha = alpha + ?4,
                beta = beta + ?5,
                update_count = update_count + 1",
            params![source, entity_type, signal_type, alpha_delta, beta_delta],
        )?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::test_db;

    #[test]
    fn test_insert_resolution_feedback() {
        let db = test_db();
        db.insert_resolution_feedback("m1", Some("a1"), Some("account"), Some("a2"), Some("account"), Some("keyword"))
            .expect("insert feedback");

        let count: i32 = db.conn_ref().query_row(
            "SELECT COUNT(*) FROM entity_resolution_feedback WHERE meeting_id = 'm1'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_upsert_signal_weight_creates_new() {
        let db = test_db();
        db.upsert_signal_weight("keyword", "account", "entity_resolution", 0.0, 1.0)
            .expect("upsert");

        // Should create with alpha=1.0, beta=2.0 (1.0 + 1.0 delta)
        let (alpha, beta, count): (f64, f64, i32) = db.conn_ref().query_row(
            "SELECT alpha, beta, update_count FROM signal_weights
             WHERE source = 'keyword' AND entity_type = 'account' AND signal_type = 'entity_resolution'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).unwrap();
        assert!((alpha - 1.0).abs() < 0.01);
        assert!((beta - 2.0).abs() < 0.01);
        assert_eq!(count, 1);
    }

    #[test]
    fn test_upsert_signal_weight_increments_existing() {
        let db = test_db();
        // First insert
        db.upsert_signal_weight("keyword", "account", "entity_resolution", 0.0, 1.0)
            .expect("first upsert");
        // Second increment
        db.upsert_signal_weight("keyword", "account", "entity_resolution", 0.0, 1.0)
            .expect("second upsert");

        let (alpha, beta, count): (f64, f64, i32) = db.conn_ref().query_row(
            "SELECT alpha, beta, update_count FROM signal_weights
             WHERE source = 'keyword' AND entity_type = 'account' AND signal_type = 'entity_resolution'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).unwrap();
        assert!((alpha - 1.0).abs() < 0.01);
        assert!((beta - 3.0).abs() < 0.01); // 2.0 + 1.0
        assert_eq!(count, 2);
    }

    #[test]
    fn test_record_correction_updates_weights() {
        let db = test_db();

        // Emit a resolution signal that pointed to the wrong entity
        crate::signals::bus::emit_signal(
            &db, "account", "wrong-acme", "entity_resolution", "keyword",
            Some(&format!("{{\"event_id\":\"m1\",\"source\":\"keyword\",\"outcome\":\"resolved\"}}")),
            0.8,
        ).expect("emit");

        // Also emit one that pointed to the correct entity
        crate::signals::bus::emit_signal(
            &db, "account", "correct-acme", "entity_resolution", "attendee_vote",
            Some(&format!("{{\"event_id\":\"m1\",\"source\":\"attendee_vote\",\"outcome\":\"resolved\"}}")),
            0.7,
        ).expect("emit correct");

        // Record correction: wrong-acme was wrong, correct-acme is right
        let old_entities = vec![("wrong-acme".to_string(), "account".to_string())];
        record_correction(&db, "m1", &old_entities, "correct-acme", "account")
            .expect("record correction");

        // Verify feedback was recorded
        let count: i32 = db.conn_ref().query_row(
            "SELECT COUNT(*) FROM entity_resolution_feedback WHERE meeting_id = 'm1'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(count, 1);

        // Verify keyword source was penalized (beta incremented)
        let (alpha, beta, _): (f64, f64, i32) = db.conn_ref().query_row(
            "SELECT alpha, beta, update_count FROM signal_weights
             WHERE source = 'keyword' AND entity_type = 'account'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).unwrap();
        assert!((alpha - 1.0).abs() < 0.01, "keyword alpha should stay at prior: {}", alpha);
        assert!(beta > 1.5, "keyword beta should be incremented: {}", beta);

        // Verify attendee_vote source was rewarded (alpha incremented)
        let (alpha2, beta2, _): (f64, f64, i32) = db.conn_ref().query_row(
            "SELECT alpha, beta, update_count FROM signal_weights
             WHERE source = 'attendee_vote' AND entity_type = 'account'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).unwrap();
        assert!(alpha2 > 1.5, "attendee_vote alpha should be incremented: {}", alpha2);
        assert!((beta2 - 1.0).abs() < 0.01, "attendee_vote beta should stay at prior: {}", beta2);
    }
}
