//! Feedback closure for user corrections and enrichment success (I409).
//!
//! Wires user corrections back to source reliability via the signal bus
//! and quality scoring system.

use super::quality;
use crate::db::ActionDb;

/// Record that a user corrected an entity's intelligence.
///
/// Increments beta (lowers quality score), bumps correction count,
/// and penalizes the source in the Thompson Sampling weight system.
pub fn record_enrichment_correction(
    db: &ActionDb,
    entity_id: &str,
    entity_type: &str,
    source: &str,
) {
    quality::ensure_quality_row(db, entity_id, entity_type);
    quality::increment_beta(db, entity_id);

    let _ = db.conn_ref().execute(
        "UPDATE entity_quality SET correction_count = correction_count + 1 WHERE entity_id = ?1",
        rusqlite::params![entity_id],
    );

    // Penalize source in Thompson Sampling weights (beta_delta = 1.0)
    let _ = db.upsert_signal_weight(source, entity_type, "enrichment_quality", 0.0, 1.0);
}

/// Record successful enrichment for an entity.
///
/// Increments alpha (raises quality score) and updates last_enrichment_at.
pub fn record_enrichment_success(db: &ActionDb, entity_id: &str) {
    // Ensure the row exists (entities may not have been initialized yet)
    let _ = db.conn_ref().execute(
        "INSERT OR IGNORE INTO entity_quality (entity_id, entity_type)
         SELECT entity_id, entity_type FROM entity_assessment WHERE entity_id = ?1
         UNION ALL
         SELECT ?1, 'unknown' WHERE NOT EXISTS (SELECT 1 FROM entity_assessment WHERE entity_id = ?1)
         LIMIT 1",
        rusqlite::params![entity_id],
    );

    quality::increment_alpha(db, entity_id);

    let _ = db.conn_ref().execute(
        "UPDATE entity_quality SET last_enrichment_at = datetime('now') WHERE entity_id = ?1",
        rusqlite::params![entity_id],
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::ActionDb;

    fn test_db() -> ActionDb {
        crate::db::test_utils::test_db()
    }

    #[test]
    fn test_record_correction_lowers_quality() {
        let db = test_db();
        quality::ensure_quality_row(&db, "acme", "account");

        let before = quality::get_quality(&db, "acme").unwrap();
        record_enrichment_correction(&db, "acme", "account", "ai_enrichment");
        let after = quality::get_quality(&db, "acme").unwrap();

        assert!(after.quality_score < before.quality_score);
        assert_eq!(after.correction_count, 1);
    }

    #[test]
    fn test_record_success_raises_quality() {
        let db = test_db();
        quality::ensure_quality_row(&db, "acme", "account");

        let before = quality::get_quality(&db, "acme").unwrap();
        record_enrichment_success(&db, "acme");
        let after = quality::get_quality(&db, "acme").unwrap();

        assert!(after.quality_score > before.quality_score);
        assert!(after.last_enrichment_at.is_some());
    }

    #[test]
    fn test_correction_increments_count() {
        let db = test_db();
        quality::ensure_quality_row(&db, "acme", "account");

        record_enrichment_correction(&db, "acme", "account", "ai_enrichment");
        record_enrichment_correction(&db, "acme", "account", "ai_enrichment");

        let q = quality::get_quality(&db, "acme").unwrap();
        assert_eq!(q.correction_count, 2);
    }
}
