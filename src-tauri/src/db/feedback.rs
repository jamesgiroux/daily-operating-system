//! Entity feedback events and suppression tombstones.

use super::types::{DbError, FeedbackEvent, SuppressionTombstone};
use super::ActionDb;
use rusqlite::params;

/// Correction actions a user can take on an AI-surfaced intelligence field.
///
/// Persisted in `entity_feedback_events.feedback_type` as a snake_case string.
/// Each action has distinct downstream semantics in `services::feedback`:
///
/// - `Confirmed` → positive signal, rewards the source via Bayesian alpha++
/// - `Rejected` → negative feedback without suppressing the content; used by
///   legacy thumbs-down surfaces that should penalize a source but leave the
///   item visible.
/// - `Annotated` → user note stored in `reason`, threaded into next intel prompt
/// - `Corrected` → `previous_value` + `corrected_value` captured; penalizes source
///   via Bayesian beta++; triggers health recalc when field is health-affecting.
/// - `Dismissed` → negative feedback + suppression tombstone for a claim the
///   user marked wrong; the current surface should hide it immediately.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorrectionAction {
    Confirmed,
    Rejected,
    Annotated,
    Corrected,
    Dismissed,
}

impl CorrectionAction {
    /// String form persisted in `entity_feedback_events.feedback_type`.
    pub fn as_str(&self) -> &'static str {
        match self {
            CorrectionAction::Confirmed => "confirmed",
            CorrectionAction::Rejected => "rejected",
            CorrectionAction::Annotated => "annotated",
            CorrectionAction::Corrected => "corrected",
            CorrectionAction::Dismissed => "dismissed",
        }
    }

    /// Parse the wire-format action string coming from the Tauri command.
    pub fn parse(raw: &str) -> Result<Self, String> {
        match raw {
            "confirmed" => Ok(CorrectionAction::Confirmed),
            "rejected" => Ok(CorrectionAction::Rejected),
            "annotated" => Ok(CorrectionAction::Annotated),
            "corrected" => Ok(CorrectionAction::Corrected),
            "dismissed" => Ok(CorrectionAction::Dismissed),
            other => Err(format!(
                "invalid correction action '{}' (expected confirmed|rejected|annotated|corrected|dismissed)",
                other
            )),
        }
    }
}

#[derive(Debug)]
pub struct FeedbackEventInput<'a> {
    pub entity_id: &'a str,
    pub entity_type: &'a str,
    pub field_key: &'a str,
    pub item_key: Option<&'a str>,
    pub feedback_type: &'a str,
    pub source_system: Option<&'a str>,
    pub source_kind: Option<&'a str>,
    pub previous_value: Option<&'a str>,
    pub corrected_value: Option<&'a str>,
    pub reason: Option<&'a str>,
}

impl ActionDb {
    /// Record a feedback event (dismiss, accept, reject, thumbs-up, thumbs-down, etc.).
    #[must_use = "feedback events must be propagated; silent discard hides ghost-resurrection bugs"]
    pub fn record_feedback_event(&self, input: &FeedbackEventInput<'_>) -> Result<i64, DbError> {
        self.conn_ref().execute(
            "INSERT INTO entity_feedback_events \
             (entity_id, entity_type, field_key, item_key, feedback_type, \
              source_system, source_kind, previous_value, corrected_value, reason) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                input.entity_id,
                input.entity_type,
                input.field_key,
                input.item_key,
                input.feedback_type,
                input.source_system,
                input.source_kind,
                input.previous_value,
                input.corrected_value,
                input.reason,
            ],
        )?;
        Ok(self.conn_ref().last_insert_rowid())
    }

    /// Create a suppression tombstone that prevents re-surfacing a dismissed item.
    #[must_use = "tombstones must be propagated; silent discard re-surfaces dismissed items"]
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

    #[must_use = "audit row should be propagated; silent discard hides operator-actionable malformed records"]
    pub fn record_malformed_suppression(
        &self,
        record_id: &str,
        reason: &str,
        entity_id: &str,
        field_key: &str,
        caller_context: Option<&str>,
    ) -> Result<i64, DbError> {
        self.conn_ref().execute(
            "INSERT INTO suppression_malformed_log \
             (record_id, reason, entity_id, field_key, caller_context) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![record_id, reason, entity_id, field_key, caller_context],
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
