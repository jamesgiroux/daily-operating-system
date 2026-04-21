//! Intelligence feedback persistence (I529/I536).

use super::ActionDb;

/// A row from the `intelligence_feedback` table.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedbackRow {
    pub id: String,
    pub entity_id: String,
    pub entity_type: String,
    pub field: String,
    pub feedback_type: String,
    pub previous_value: Option<String>,
    pub context: Option<String>,
    pub created_at: String,
}

/// Parameters for inserting intelligence feedback.
#[derive(Debug)]
pub struct FeedbackInput<'a> {
    pub id: &'a str,
    pub entity_id: &'a str,
    pub entity_type: &'a str,
    pub field: &'a str,
    pub feedback_type: &'a str,
    pub previous_value: Option<&'a str>,
    pub context: Option<&'a str>,
}

impl ActionDb {
    /// Insert or replace an intelligence feedback record.
    /// Uses ON CONFLICT on the UNIQUE(entity_id, entity_type, field) constraint
    /// so changing a vote replaces the previous one (AC16).
    pub fn insert_intelligence_feedback(
        &self,
        input: &FeedbackInput<'_>,
    ) -> Result<(), String> {
        self.conn_ref()
            .execute(
                "INSERT INTO intelligence_feedback \
                 (id, entity_id, entity_type, field, feedback_type, previous_value, context) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) \
                 ON CONFLICT(entity_id, entity_type, field) DO UPDATE SET \
                 id = excluded.id, \
                 feedback_type = excluded.feedback_type, \
                 previous_value = excluded.previous_value, \
                 context = excluded.context, \
                 created_at = datetime('now')",
                rusqlite::params![
                    input.id,
                    input.entity_id,
                    input.entity_type,
                    input.field,
                    input.feedback_type,
                    input.previous_value,
                    input.context,
                ],
            )
            .map_err(|e| format!("Insert intelligence feedback: {e}"))?;
        Ok(())
    }

    /// I645: Check if an intelligence item is suppressed by a tombstone.
    ///
    /// Returns `true` if a matching tombstone exists for the given entity/field/item
    /// AND the item does NOT have newer evidence (sourced_at > dismissed_at).
    /// If `sourced_at` is provided and is newer than the tombstone's `dismissed_at`,
    /// the item is NOT suppressed (new evidence supersedes the dismissal).
    pub fn is_suppressed(
        &self,
        entity_id: &str,
        field_key: &str,
        item_key: Option<&str>,
        sourced_at: Option<&str>,
    ) -> Result<bool, String> {
        let conn = self.conn_ref();
        // Find matching tombstone that hasn't expired
        let result: Option<String> = conn
            .query_row(
                "SELECT dismissed_at FROM suppression_tombstones \
                 WHERE entity_id = ?1 AND field_key = ?2 AND item_key = ?3 \
                 AND (expires_at IS NULL OR expires_at > datetime('now'))",
                rusqlite::params![entity_id, field_key, item_key],
                |row| row.get(0),
            )
            .ok();

        match result {
            None => Ok(false), // No tombstone found
            Some(dismissed_at) => {
                // If the item has newer evidence, it should pass through
                if let Some(src_at) = sourced_at {
                    if src_at > dismissed_at.as_str() {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
        }
    }

    /// Get all feedback for an entity, newest first.
    ///
    /// Compatibility read during the feedback-pipeline collapse:
    /// - legacy `intelligence_feedback` votes remain visible
    /// - new `entity_feedback_events` rows are mapped back into the old
    ///   positive/negative/replaced shape for existing thumbs surfaces
    pub fn get_entity_feedback(
        &self,
        entity_id: &str,
        entity_type: &str,
    ) -> Result<Vec<FeedbackRow>, String> {
        let conn = self.conn_ref();
        let mut stmt = conn
            .prepare(
                "SELECT id, entity_id, entity_type, field, feedback_type, \
                 previous_value, context, created_at \
                 FROM (
                    SELECT id, entity_id, entity_type, field, feedback_type, \
                           previous_value, context, created_at \
                    FROM intelligence_feedback \
                    WHERE entity_id = ?1 AND entity_type = ?2

                    UNION ALL

                    SELECT CAST(id AS TEXT) AS id,
                           entity_id,
                           entity_type,
                           CASE
                               WHEN source_kind = 'field_conflict'
                                   THEN 'account_field_conflict:' || field_key || ':' || COALESCE(corrected_value, '')
                               ELSE field_key
                           END AS field,
                           CASE
                               WHEN feedback_type IN ('confirmed', 'accept') THEN 'positive'
                               WHEN feedback_type IN ('rejected', 'dismissed', 'reject', 'dismiss') THEN 'negative'
                               WHEN feedback_type = 'corrected' THEN 'replaced'
                               ELSE feedback_type
                           END AS feedback_type,
                           previous_value,
                           reason AS context,
                           created_at
                    FROM entity_feedback_events \
                    WHERE entity_id = ?1 AND entity_type = ?2 \
                      AND feedback_type IN ('confirmed', 'rejected', 'corrected', 'dismissed', 'accept', 'reject', 'dismiss')
                 ) \
                 ORDER BY created_at DESC",
            )
            .map_err(|e| format!("Prepare get_entity_feedback: {e}"))?;
        let rows = stmt
            .query_map(rusqlite::params![entity_id, entity_type], |row| {
                Ok(FeedbackRow {
                    id: row.get(0)?,
                    entity_id: row.get(1)?,
                    entity_type: row.get(2)?,
                    field: row.get(3)?,
                    feedback_type: row.get(4)?,
                    previous_value: row.get(5)?,
                    context: row.get(6)?,
                    created_at: row.get(7)?,
                })
            })
            .map_err(|e| format!("Query get_entity_feedback: {e}"))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Collect get_entity_feedback: {e}"))
    }
}
