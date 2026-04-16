//! Signal-driven prep invalidation (I308 — ADR-0080 Phase 4).
//!
//! When a signal arrives for an entity, check upcoming meetings (48h) linked
//! to that entity. If the meeting has stale prep, push its ID to the existing
//! `prep_invalidation_queue` for regeneration.

use parking_lot::Mutex;

use crate::db::ActionDb;

use super::bus::SignalEvent;

/// Minimum confidence for a signal to trigger prep invalidation.
const MIN_CONFIDENCE: f64 = 0.70;

/// Check if a newly-emitted signal should invalidate any upcoming meeting preps.
///
/// For the signal's entity, queries upcoming meetings (48h) via `meeting_entities`.
/// If the signal confidence ≥ 0.7 and the meeting exists, pushes the meeting ID
/// to the prep invalidation queue.
pub fn check_and_invalidate_preps(
    db: &ActionDb,
    signal: &SignalEvent,
    prep_queue: &Mutex<Vec<String>>,
) {
    if signal.confidence < MIN_CONFIDENCE {
        return;
    }

    // Skip signals that don't affect meeting prep content
    let invalidating_types = [
        "stakeholder_change",
        "champion_risk",
        "renewal_risk_escalation",
        "engagement_warning",
        "project_health_warning",
        "title_change",
        "company_change", // person_departed removed — no emitter exists
        "pre_meeting_context",
        "stakeholders_updated",
        "team_member_added",
        "team_member_removed",
        "relationship_graph_changed",
        "relationship_reclassified",
        "transcript_outcomes", // manually attached transcript — invalidate linked future meeting preps
        "field_updated",      // DOS-110: account field changes (including sentiment) invalidate prep
    ];

    if !invalidating_types.contains(&signal.signal_type.as_str()) {
        return;
    }

    let meeting_ids =
        match db.get_upcoming_meetings_for_entity(&signal.entity_type, &signal.entity_id, 48) {
            Ok(ids) => ids,
            Err(e) => {
                log::warn!("Prep invalidation: failed to query meetings: {}", e);
                return;
            }
        };

    if meeting_ids.is_empty() {
        return;
    }

    {
        let mut queue = prep_queue.lock();
        for mid in &meeting_ids {
            if !queue.contains(mid) {
                queue.push(mid.clone());
                log::info!(
                    "Prep invalidation: queued meeting {} due to {} signal on {}/{}",
                    mid,
                    signal.signal_type,
                    signal.entity_type,
                    signal.entity_id,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ActionDb methods
// ---------------------------------------------------------------------------

impl ActionDb {
    /// Get upcoming meetings (within N hours) linked to an entity.
    pub fn get_upcoming_meetings_for_entity(
        &self,
        entity_type: &str,
        entity_id: &str,
        hours: i32,
    ) -> Result<Vec<String>, crate::db::DbError> {
        let hours_param = format!("+{} hours", hours);
        let mut stmt = self.conn_ref().prepare(
            "SELECT DISTINCT me.meeting_id
             FROM meeting_entities me
             JOIN meetings mh ON mh.id = me.meeting_id
             WHERE me.entity_id = ?1 AND me.entity_type = ?2
               AND mh.start_time >= datetime('now')
               AND mh.start_time <= datetime('now', ?3)",
        )?;

        let rows = stmt.query_map(
            rusqlite::params![entity_id, entity_type, hours_param],
            |row| row.get::<_, String>(0),
        )?;

        let mut ids = Vec::new();
        for row in rows {
            ids.push(row?);
        }
        Ok(ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::test_db;
    use parking_lot::Mutex;

    fn make_signal(signal_type: &str, confidence: f64) -> SignalEvent {
        SignalEvent {
            id: "sig-test".to_string(),
            entity_type: "account".to_string(),
            entity_id: "a1".to_string(),
            signal_type: signal_type.to_string(),
            source: "propagation".to_string(),
            value: None,
            confidence,
            decay_half_life_days: 30,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            superseded_by: None,
            source_context: None,
        }
    }

    #[test]
    fn test_low_confidence_skipped() {
        let db = test_db();
        let queue = Mutex::new(Vec::<String>::new());
        let signal = make_signal("stakeholder_change", 0.50);

        check_and_invalidate_preps(&db, &signal, &queue);

        let q = queue.lock();
        assert!(q.is_empty(), "low-confidence signal should not invalidate");
    }

    #[test]
    fn test_irrelevant_type_skipped() {
        let db = test_db();
        let queue = Mutex::new(Vec::<String>::new());
        let signal = make_signal("entity_resolution", 0.95);

        check_and_invalidate_preps(&db, &signal, &queue);

        let q = queue.lock();
        assert!(q.is_empty(), "entity_resolution should not invalidate prep");
    }

    #[test]
    fn test_invalidation_with_upcoming_meeting() {
        let db = test_db();
        let conn = db.conn_ref();

        // Create account and meeting linked to it
        conn.execute(
            "INSERT INTO accounts (id, name, updated_at) VALUES ('a1', 'Acme', '2026-01-01')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO meetings (id, title, meeting_type, start_time, created_at)
             VALUES ('m1', 'QBR', 'customer', datetime('now', '+2 hours'), datetime('now'))",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO meeting_prep (meeting_id) VALUES ('m1')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO meeting_transcripts (meeting_id) VALUES ('m1')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO meeting_entities (meeting_id, entity_id, entity_type)
             VALUES ('m1', 'a1', 'account')",
            [],
        )
        .unwrap();

        let queue = Mutex::new(Vec::<String>::new());
        let signal = make_signal("stakeholder_change", 0.85);

        check_and_invalidate_preps(&db, &signal, &queue);

        let q = queue.lock();
        assert_eq!(q.len(), 1);
        assert_eq!(q[0], "m1");
    }

    #[test]
    fn test_relationship_graph_change_invalidates_person_prep() {
        let db = test_db();
        let conn = db.conn_ref();

        conn.execute(
            "INSERT INTO people (id, email, name, relationship, meeting_count, updated_at)
             VALUES ('p1', 'person@example.com', 'Person One', 'external', 0, '2026-01-01')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO meetings (id, title, meeting_type, start_time, created_at)
             VALUES ('m-person', 'Intro', 'customer', datetime('now', '+4 hours'), datetime('now'))",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO meeting_prep (meeting_id) VALUES ('m-person')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO meeting_transcripts (meeting_id) VALUES ('m-person')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO meeting_entities (meeting_id, entity_id, entity_type)
             VALUES ('m-person', 'p1', 'person')",
            [],
        )
        .unwrap();

        let queue = Mutex::new(Vec::<String>::new());
        let signal = SignalEvent {
            id: "sig-relationship".to_string(),
            entity_type: "person".to_string(),
            entity_id: "p1".to_string(),
            signal_type: "relationship_graph_changed".to_string(),
            source: "user_action".to_string(),
            value: None,
            confidence: 0.9,
            decay_half_life_days: 30,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            superseded_by: None,
            source_context: None,
        };

        check_and_invalidate_preps(&db, &signal, &queue);

        let q = queue.lock();
        assert_eq!(q.len(), 1);
        assert_eq!(q[0], "m-person");
    }
}
