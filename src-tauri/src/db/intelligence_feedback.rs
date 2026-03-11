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
