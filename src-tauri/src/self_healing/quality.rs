//! Beta distribution quality scoring for entity intelligence.
//!
//! Each entity has a Beta(alpha, beta) distribution where:
//! - alpha increments on enrichment success
//! - beta increments on user correction
//! - quality_score = alpha / (alpha + beta) (mean of the Beta distribution)
//!
//! New entities start at Beta(1,1) = uniform prior (score 0.5).

use crate::db::ActionDb;

/// Row from the entity_quality table.
#[derive(Debug, Clone)]
pub struct EntityQuality {
    pub entity_id: String,
    pub entity_type: String,
    pub quality_alpha: f64,
    pub quality_beta: f64,
    pub quality_score: f64,
    pub last_enrichment_at: Option<String>,
    pub correction_count: i64,
    pub coherence_retry_count: i64,
    pub coherence_window_start: Option<String>,
    pub coherence_blocked: bool,
}

/// Ensure a quality row exists for the given entity. INSERT OR IGNORE with Beta(1,1) defaults.
pub fn ensure_quality_row(db: &ActionDb, entity_id: &str, entity_type: &str) {
    if let Err(e) = db.conn_ref().execute(
        "INSERT OR IGNORE INTO entity_quality (entity_id, entity_type) VALUES (?1, ?2)",
        rusqlite::params![entity_id, entity_type],
    ) {
        log::warn!("ensure entity quality row failed for {entity_type}:{entity_id}: {e}");
    }
}

/// Get the full quality row for an entity.
pub fn get_quality(db: &ActionDb, entity_id: &str) -> Option<EntityQuality> {
    db.conn_ref()
        .query_row(
            "SELECT entity_id, entity_type, quality_alpha, quality_beta, quality_score,
                    last_enrichment_at, correction_count, coherence_retry_count,
                    coherence_window_start, coherence_blocked
             FROM entity_quality WHERE entity_id = ?1",
            rusqlite::params![entity_id],
            |row| {
                Ok(EntityQuality {
                    entity_id: row.get(0)?,
                    entity_type: row.get(1)?,
                    quality_alpha: row.get(2)?,
                    quality_beta: row.get(3)?,
                    quality_score: row.get(4)?,
                    last_enrichment_at: row.get(5)?,
                    correction_count: row.get(6)?,
                    coherence_retry_count: row.get(7)?,
                    coherence_window_start: row.get(8)?,
                    coherence_blocked: row.get::<_, i64>(9)? != 0,
                })
            },
        )
        .ok()
}

/// Record enrichment success: alpha += 1.0, recompute score.
pub fn increment_alpha(db: &ActionDb, entity_id: &str) {
    if let Err(e) = db.conn_ref().execute(
        "UPDATE entity_quality
         SET quality_alpha = quality_alpha + 1.0,
             quality_score = (quality_alpha + 1.0) / (quality_alpha + 1.0 + quality_beta),
             updated_at = datetime('now')
         WHERE entity_id = ?1",
        rusqlite::params![entity_id],
    ) {
        log::warn!("increment entity quality alpha failed for {entity_id}: {e}");
    }
}

/// Record user correction: beta += 1.0, recompute score.
pub fn increment_beta(db: &ActionDb, entity_id: &str) {
    if let Err(e) = db.conn_ref().execute(
        "UPDATE entity_quality
         SET quality_beta = quality_beta + 1.0,
             quality_score = quality_alpha / (quality_alpha + quality_beta + 1.0),
             updated_at = datetime('now')
         WHERE entity_id = ?1",
        rusqlite::params![entity_id],
    ) {
        log::warn!("increment entity quality beta failed for {entity_id}: {e}");
    }
}

/// Get entities below a quality threshold, returning (entity_id, entity_type, score).
pub fn get_entities_below_quality_threshold(
    db: &ActionDb,
    threshold: f64,
) -> Vec<(String, String, f64)> {
    let mut stmt = match db.conn_ref().prepare(
        "SELECT entity_id, entity_type, quality_score FROM entity_quality
         WHERE quality_score < ?1
         ORDER BY quality_score ASC",
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    stmt.query_map(rusqlite::params![threshold], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
    })
    .map(|rows| rows.filter_map(|r| r.ok()).collect())
    .unwrap_or_default()
}

/// Bulk-initialize quality rows for all known entities. Idempotent (INSERT OR IGNORE).
pub fn initialize_quality_scores(db: &ActionDb) {
    // Accounts
    if let Err(e) = db.conn_ref().execute_batch(
        "INSERT OR IGNORE INTO entity_quality (entity_id, entity_type)
         SELECT id, 'account' FROM accounts WHERE archived = 0;
         INSERT OR IGNORE INTO entity_quality (entity_id, entity_type)
         SELECT id, 'project' FROM projects WHERE archived = 0;
         INSERT OR IGNORE INTO entity_quality (entity_id, entity_type)
         SELECT id, 'person' FROM people;",
    ) {
        log::warn!("initialize entity quality rows failed: {e}");
    }
}

/// Count entities with quality_score below 0.45.
pub fn get_low_quality_count(db: &ActionDb) -> usize {
    db.conn_ref()
        .query_row(
            "SELECT COUNT(*) FROM entity_quality WHERE quality_score < 0.45",
            [],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0) as usize
}

/// Count entities with coherence_blocked = 1.
pub fn get_coherence_blocked_count(db: &ActionDb) -> usize {
    db.conn_ref()
        .query_row(
            "SELECT COUNT(*) FROM entity_quality WHERE coherence_blocked = 1",
            [],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0) as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::ActionDb;

    fn test_db() -> ActionDb {
        crate::db::test_utils::test_db()
    }

    #[test]
    fn test_ensure_and_get_quality() {
        let db = test_db();
        ensure_quality_row(&db, "acme", "account");

        let q = get_quality(&db, "acme").expect("should exist");
        assert_eq!(q.entity_id, "acme");
        assert_eq!(q.entity_type, "account");
        assert!((q.quality_alpha - 1.0).abs() < f64::EPSILON);
        assert!((q.quality_beta - 1.0).abs() < f64::EPSILON);
        assert!((q.quality_score - 0.5).abs() < f64::EPSILON);
        assert!(!q.coherence_blocked);
    }

    #[test]
    fn test_increment_alpha_raises_score() {
        let db = test_db();
        ensure_quality_row(&db, "acme", "account");

        increment_alpha(&db, "acme");
        let q = get_quality(&db, "acme").unwrap();
        // Beta(2,1): score = 2/3 ≈ 0.667
        assert!(q.quality_score > 0.6);
    }

    #[test]
    fn test_increment_beta_lowers_score() {
        let db = test_db();
        ensure_quality_row(&db, "acme", "account");

        increment_beta(&db, "acme");
        let q = get_quality(&db, "acme").unwrap();
        // Beta(1,2): score = 1/3 ≈ 0.333
        assert!(q.quality_score < 0.4);
    }

    #[test]
    fn test_get_entities_below_threshold() {
        let db = test_db();
        ensure_quality_row(&db, "good", "account");
        ensure_quality_row(&db, "bad", "account");

        // Make "bad" low quality
        increment_beta(&db, "bad");
        increment_beta(&db, "bad");

        let low = get_entities_below_quality_threshold(&db, 0.4);
        assert_eq!(low.len(), 1);
        assert_eq!(low[0].0, "bad");
    }

    #[test]
    fn test_initialize_quality_scores_idempotent() {
        let db = test_db();
        // Insert a test account
        db.conn_ref()
            .execute(
                "INSERT INTO accounts (id, name, updated_at, archived) VALUES ('a1', 'Acme', '2025-01-01', 0)",
                [],
            )
            .unwrap();

        initialize_quality_scores(&db);
        let q1 = get_quality(&db, "a1").expect("should exist");
        assert!((q1.quality_score - 0.5).abs() < f64::EPSILON);

        // Run again — should not error or change values
        initialize_quality_scores(&db);
        let q2 = get_quality(&db, "a1").unwrap();
        assert!((q2.quality_score - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_low_quality_and_blocked_counts() {
        let db = test_db();
        ensure_quality_row(&db, "good", "account");
        ensure_quality_row(&db, "bad", "account");

        // Lower bad's score below 0.45
        increment_beta(&db, "bad");
        increment_beta(&db, "bad");

        assert_eq!(get_low_quality_count(&db), 1);
        assert_eq!(get_coherence_blocked_count(&db), 0);

        // Block bad
        db.conn_ref()
            .execute(
                "UPDATE entity_quality SET coherence_blocked = 1 WHERE entity_id = 'bad'",
                [],
            )
            .unwrap();
        assert_eq!(get_coherence_blocked_count(&db), 1);
    }
}
