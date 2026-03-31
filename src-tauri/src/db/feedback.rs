//! Entity feedback events and suppression tombstones (I645).

use super::types::{DbError, FeedbackEvent, SuppressionTombstone};
use super::ActionDb;
use rusqlite::params;

impl ActionDb {
    /// Record a feedback event (dismiss, accept, reject, thumbs-up, thumbs-down, etc.).
    #[allow(clippy::too_many_arguments)]
    pub fn record_feedback_event(
        &self,
        entity_id: &str,
        entity_type: &str,
        field_key: &str,
        item_key: Option<&str>,
        feedback_type: &str,
        source_system: Option<&str>,
        source_kind: Option<&str>,
        previous_value: Option<&str>,
        corrected_value: Option<&str>,
        reason: Option<&str>,
    ) -> Result<i64, DbError> {
        self.conn_ref().execute(
            "INSERT INTO entity_feedback_events \
             (entity_id, entity_type, field_key, item_key, feedback_type, \
              source_system, source_kind, previous_value, corrected_value, reason) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                entity_id,
                entity_type,
                field_key,
                item_key,
                feedback_type,
                source_system,
                source_kind,
                previous_value,
                corrected_value,
                reason,
            ],
        )?;
        Ok(self.conn_ref().last_insert_rowid())
    }

    /// Create a suppression tombstone that prevents re-surfacing a dismissed item.
    pub fn create_suppression_tombstone(
        &self,
        entity_id: &str,
        field_key: &str,
        item_key: Option<&str>,
        item_hash: Option<&str>,
        source_scope: Option<&str>,
        expires_at: Option<&str>,
    ) -> Result<i64, DbError> {
        self.conn_ref().execute(
            "INSERT INTO suppression_tombstones \
             (entity_id, field_key, item_key, item_hash, source_scope, expires_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![entity_id, field_key, item_key, item_hash, source_scope, expires_at],
        )?;
        Ok(self.conn_ref().last_insert_rowid())
    }

    /// Get feedback events for an entity, newest first.
    pub fn get_feedback_events(
        &self,
        entity_id: &str,
        limit: usize,
    ) -> Result<Vec<FeedbackEvent>, DbError> {
        let mut stmt = self.conn_ref().prepare(
            "SELECT id, entity_id, entity_type, field_key, item_key, feedback_type, \
             source_system, source_kind, previous_value, corrected_value, reason, created_at \
             FROM entity_feedback_events \
             WHERE entity_id = ?1 \
             ORDER BY created_at DESC \
             LIMIT ?2",
        )?;

        let rows = stmt.query_map(params![entity_id, limit as i64], |row| {
            Ok(FeedbackEvent {
                id: row.get(0)?,
                entity_id: row.get(1)?,
                entity_type: row.get(2)?,
                field_key: row.get(3)?,
                item_key: row.get(4)?,
                feedback_type: row.get(5)?,
                source_system: row.get(6)?,
                source_kind: row.get(7)?,
                previous_value: row.get(8)?,
                corrected_value: row.get(9)?,
                reason: row.get(10)?,
                created_at: row.get(11)?,
            })
        })?;

        let mut events = Vec::new();
        for row in rows {
            events.push(row?);
        }
        Ok(events)
    }

    /// Get all active (non-expired) suppression tombstones for an entity.
    pub fn get_active_suppressions(
        &self,
        entity_id: &str,
    ) -> Result<Vec<SuppressionTombstone>, DbError> {
        let mut stmt = self.conn_ref().prepare(
            "SELECT id, entity_id, field_key, item_key, item_hash, dismissed_at, \
             source_scope, expires_at, superseded_by_evidence_after \
             FROM suppression_tombstones \
             WHERE entity_id = ?1 \
             AND (expires_at IS NULL OR expires_at > datetime('now')) \
             ORDER BY dismissed_at DESC",
        )?;

        let rows = stmt.query_map(params![entity_id], |row| {
            Ok(SuppressionTombstone {
                id: row.get(0)?,
                entity_id: row.get(1)?,
                field_key: row.get(2)?,
                item_key: row.get(3)?,
                item_hash: row.get(4)?,
                dismissed_at: row.get(5)?,
                source_scope: row.get(6)?,
                expires_at: row.get(7)?,
                superseded_by_evidence_after: row.get(8)?,
            })
        })?;

        let mut tombstones = Vec::new();
        for row in rows {
            tombstones.push(row?);
        }
        Ok(tombstones)
    }
}
