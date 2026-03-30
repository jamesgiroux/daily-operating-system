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

impl ActionDb {
    /// Insert or replace an intelligence feedback record.
    /// Uses ON CONFLICT on the UNIQUE(entity_id, entity_type, field) constraint
    /// so changing a vote replaces the previous one (AC16).
    #[allow(clippy::too_many_arguments)]
    pub fn insert_intelligence_feedback(
        &self,
        id: &str,
        entity_id: &str,
        entity_type: &str,
        field: &str,
        feedback_type: &str,
        previous_value: Option<&str>,
        context: Option<&str>,
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
                    id,
                    entity_id,
                    entity_type,
                    field,
                    feedback_type,
                    previous_value,
                    context,
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
                 FROM intelligence_feedback \
                 WHERE entity_id = ?1 AND entity_type = ?2 \
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
