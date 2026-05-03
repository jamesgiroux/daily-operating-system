//! Enrichment trigger function.
//!
//! Replaces the hardcoded 14-day staleness threshold with a continuous
//! priority score based on meeting imminence, staleness, importance,
//! and signal activity.

use crate::db::ActionDb;

/// Compute a continuous enrichment trigger score for an entity.
///
/// Score = imminence × 0.35 + staleness × 0.25 + quality_deficit × 0.20
///       + importance × 0.10 + signal_delta × 0.10
///
/// `quality_deficit` = 1.0 − quality_score, so low-quality entities (many user
/// corrections) rank higher than merely stale ones.
///
/// Returns a value in [0.0, 1.0]. Higher = more urgent.
pub fn compute_enrichment_trigger_score(db: &ActionDb, entity_id: &str, _entity_type: &str) -> f64 {
    let imminence = meeting_imminence_score(db, entity_id);
    let staleness = staleness_score(db, entity_id);
    let quality_deficit = quality_deficit_score(db, entity_id);
    let importance = entity_importance_score(db, entity_id);
    let signal_delta = signal_delta_score(db, entity_id);

    imminence * 0.35
        + staleness * 0.25
        + quality_deficit * 0.20
        + importance * 0.10
        + signal_delta * 0.10
}

/// 1.0 if next meeting <24h, 0.5 if <7d, 0.1 if >7d, 0.0 if none.
fn meeting_imminence_score(db: &ActionDb, entity_id: &str) -> f64 {
    let hours: Option<f64> = db
        .conn_ref()
        .query_row(
            "SELECT MIN((julianday(mh.start_time) - julianday('now')) * 24.0)
             FROM meetings mh
             INNER JOIN meeting_entities me ON me.meeting_id = mh.id
             WHERE me.entity_id = ?1 AND mh.start_time > datetime('now')",
            rusqlite::params![entity_id],
            |row| row.get(0),
        )
        .ok()
        .flatten();

    match hours {
        Some(h) if h < 24.0 => 1.0,
        Some(h) if h < 168.0 => 0.5,
        Some(_) => 0.1,
        None => 0.0,
    }
}

/// Days since last enrichment / 14.0, capped at 1.0. NULL enriched_at = 1.0.
fn staleness_score(db: &ActionDb, entity_id: &str) -> f64 {
    let days: Option<f64> = db
        .conn_ref()
        .query_row(
            "SELECT julianday('now') - julianday(enriched_at)
             FROM entity_assessment WHERE entity_id = ?1 AND enriched_at IS NOT NULL",
            rusqlite::params![entity_id],
            |row| row.get(0),
        )
        .ok()
        .flatten();

    match days {
        Some(d) => (d / 14.0).clamp(0.0, 1.0),
        None => 1.0, // Never enriched
    }
}

/// 1.0 − quality_score. Low quality = high deficit = high urgency.
/// Defaults to 0.5 if no quality row exists (Beta(1,1) prior).
fn quality_deficit_score(db: &ActionDb, entity_id: &str) -> f64 {
    let score: f64 = db
        .conn_ref()
        .query_row(
            "SELECT quality_score FROM entity_quality WHERE entity_id = ?1",
            rusqlite::params![entity_id],
            |row| row.get(0),
        )
        .unwrap_or(0.5);

    (1.0 - score).clamp(0.0, 1.0)
}

/// Meeting count in last 90 days / 10.0, capped at 1.0.
fn entity_importance_score(db: &ActionDb, entity_id: &str) -> f64 {
    let count: i64 = db
        .conn_ref()
        .query_row(
            "SELECT COUNT(*) FROM meetings mh
             INNER JOIN meeting_entities me ON me.meeting_id = mh.id
             WHERE me.entity_id = ?1 AND mh.start_time > datetime('now', '-90 days')",
            rusqlite::params![entity_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    (count as f64 / 10.0).min(1.0)
}

/// Signal count since last enrichment / 10.0, capped at 1.0.
fn signal_delta_score(db: &ActionDb, entity_id: &str) -> f64 {
    let count: i64 = db
        .conn_ref()
        .query_row(
            "SELECT COUNT(*) FROM signal_events se
             WHERE se.entity_id = ?1
               AND se.created_at > COALESCE(
                   (SELECT enriched_at FROM entity_assessment WHERE entity_id = ?1),
                   '2000-01-01'
               )",
            rusqlite::params![entity_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    (count as f64 / 10.0).min(1.0)
}

/// Prioritize all entities for enrichment based on trigger scores.
/// Returns (entity_id, entity_type, score) sorted descending, filtered >= 0.25.
pub fn prioritize_enrichment_queue(db: &ActionDb) -> Vec<(String, String, f64)> {
    let mut stmt = match db
        .conn_ref()
        .prepare("SELECT entity_id, entity_type FROM entity_quality WHERE coherence_blocked = 0")
    {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let entities: Vec<(String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();

    let mut scored: Vec<(String, String, f64)> = entities
        .into_iter()
        .map(|(id, etype)| {
            let score = compute_enrichment_trigger_score(db, &id, &etype);
            (id, etype, score)
        })
        .filter(|(_, _, score)| *score >= 0.25)
        .collect();

    scored.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    scored
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::ActionDb;

    fn test_db() -> ActionDb {
        crate::db::test_utils::test_db()
    }

    #[test]
    fn test_staleness_score_never_enriched() {
        let db = test_db();
        // No entity_assessment row → should return 1.0
        let score = staleness_score(&db, "nonexistent");
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_meeting_imminence_no_meetings() {
        let db = test_db();
        let score = meeting_imminence_score(&db, "nonexistent");
        assert!((score - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_trigger_score_is_bounded() {
        let db = test_db();
        let score = compute_enrichment_trigger_score(&db, "nonexistent", "account");
        // With no data: imminence=0, staleness=1, quality_deficit=0.5, importance=0, signals=0
        // Score = 0*0.35 + 1*0.25 + 0.5*0.20 + 0*0.10 + 0*0.10 = 0.35
        assert!((0.0..=1.0).contains(&score));
        assert!((score - 0.35).abs() < 0.01);
    }

    #[test]
    fn test_prioritize_empty_quality_table() {
        let db = test_db();
        let results = prioritize_enrichment_queue(&db);
        assert!(results.is_empty());
    }

    #[test]
    fn test_prioritize_with_entities() {
        let db = test_db();

        // Insert accounts + quality rows
        db.conn_ref()
            .execute(
                "INSERT INTO accounts (id, name, updated_at, archived) VALUES ('a1', 'Acme', '2025-01-01', 0)",
                [],
            )
            .unwrap();
        db.conn_ref()
            .execute(
                "INSERT INTO entity_quality (entity_id, entity_type) VALUES ('a1', 'account')",
                [],
            )
            .unwrap();

        let results = prioritize_enrichment_queue(&db);
        // a1 should appear: staleness=1.0 (never enriched) → score ≥ 0.25
        assert!(!results.is_empty());
        assert_eq!(results[0].0, "a1");
    }
}
