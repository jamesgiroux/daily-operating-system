//! Report staleness management.
//!
//! Called from intel_queue.rs after write_enrichment_results to mark
//! cached reports as stale when entity intelligence is refreshed.

use crate::db::ActionDb;

/// Mark all reports for an entity as stale.
/// Called after entity intelligence is refreshed.
pub fn mark_reports_stale(db: &ActionDb, entity_id: &str) -> Result<(), String> {
    db.conn_ref()
        .execute(
            "UPDATE reports SET is_stale = 1, updated_at = datetime('now') WHERE entity_id = ?1",
            rusqlite::params![entity_id],
        )
        .map_err(|e| format!("Failed to mark reports stale: {}", e))?;

    log::debug!("reports: marked stale for entity {}", entity_id);
    Ok(())
}
