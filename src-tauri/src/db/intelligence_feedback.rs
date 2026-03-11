//! Intelligence feedback persistence (I529/I536).

use super::ActionDb;

impl ActionDb {
    /// Insert an intelligence feedback record.
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
                "INSERT OR REPLACE INTO intelligence_feedback \
                 (id, entity_id, entity_type, field, feedback_type, previous_value, context) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
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
}
