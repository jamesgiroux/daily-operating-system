use super::ActionDb;
use rusqlite::params;

impl ActionDb {
    pub fn insert_pipeline_failure(
        &self,
        pipeline: &str,
        entity_id: Option<&str>,
        entity_type: Option<&str>,
        error_type: &str,
        error_message: Option<&str>,
        attempt: i32,
    ) -> Result<String, String> {
        let id = uuid::Uuid::new_v4().to_string();
        self.conn_ref()
            .execute(
                "INSERT INTO pipeline_failures
                 (id, pipeline, entity_id, entity_type, error_type, error_message, attempt)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    id,
                    pipeline,
                    entity_id,
                    entity_type,
                    error_type,
                    error_message,
                    attempt,
                ],
            )
            .map_err(|e| format!("Insert pipeline failure: {e}"))?;
        Ok(id)
    }

    pub fn resolve_pipeline_failures(
        &self,
        pipeline: &str,
        entity_id: Option<&str>,
        entity_type: Option<&str>,
    ) -> Result<usize, String> {
        let changed = self
            .conn_ref()
            .execute(
                "UPDATE pipeline_failures
                 SET resolved = 1,
                     resolved_at = datetime('now')
                 WHERE pipeline = ?1
                   AND resolved = 0
                   AND (?2 IS NULL OR entity_id = ?2)
                   AND (?3 IS NULL OR entity_type = ?3)",
                params![pipeline, entity_id, entity_type],
            )
            .map_err(|e| format!("Resolve pipeline failures: {e}"))?;
        Ok(changed)
    }

    pub fn count_unresolved_pipeline_failures(
        &self,
        pipeline: Option<&str>,
    ) -> Result<i64, String> {
        self.conn_ref()
            .query_row(
                "SELECT COUNT(*)
                 FROM pipeline_failures
                 WHERE resolved = 0
                   AND (?1 IS NULL OR pipeline = ?1)",
                params![pipeline],
                |row| row.get(0),
            )
            .map_err(|e| format!("Count unresolved pipeline failures: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use crate::db::test_utils::test_db;

    #[test]
    fn insert_and_count_pipeline_failures() {
        let db = test_db();

        db.insert_pipeline_failure(
            "meeting_prep",
            Some("mtg-1"),
            Some("meeting"),
            "db_write",
            Some("write failed"),
            2,
        )
        .expect("insert failure");

        let count = db
            .count_unresolved_pipeline_failures(Some("meeting_prep"))
            .expect("count unresolved");
        assert_eq!(count, 1);
    }

    #[test]
    fn resolve_pipeline_failures_marks_rows_resolved() {
        let db = test_db();

        db.insert_pipeline_failure(
            "meeting_prep",
            Some("mtg-1"),
            Some("meeting"),
            "db_write",
            Some("write failed"),
            1,
        )
        .expect("insert failure");

        let resolved = db
            .resolve_pipeline_failures("meeting_prep", Some("mtg-1"), Some("meeting"))
            .expect("resolve failure");
        assert_eq!(resolved, 1);

        let count = db
            .count_unresolved_pipeline_failures(Some("meeting_prep"))
            .expect("count unresolved");
        assert_eq!(count, 0);
    }
}
