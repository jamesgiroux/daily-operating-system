//! I319: Entity-level email cadence monitoring + anomaly surfacing.
//!
//! Aggregates email_signals by entity per week, computes a 30-day rolling
//! average, and flags anomalies (gone_quiet <50%, activity_spike >200%).
//! Runs cheaply via SQL aggregation during hygiene or after email fetch.

use crate::db::ActionDb;
use super::propagation::PropagationEngine;

/// A cadence anomaly for a single entity.
#[derive(Debug, Clone)]
pub struct CadenceAnomaly {
    pub entity_id: String,
    pub entity_type: String,
    pub anomaly_type: String, // "gone_quiet" or "activity_spike"
    pub current_count: i64,
    pub rolling_avg: f64,
    pub confidence: f64,
}

/// Compute entity email cadence and detect anomalies.
///
/// 1. Aggregate email_signals by (entity_id, entity_type) for the current week.
/// 2. Compute 30-day rolling average from entity_email_cadence history.
/// 3. Upsert current week's count into entity_email_cadence.
/// 4. Flag anomalies: <50% of avg = gone_quiet, >200% of avg = activity_spike.
/// 5. Emit cadence_anomaly signals for flagged entities.
pub fn compute_and_emit_cadence_anomalies(db: &ActionDb) -> Vec<CadenceAnomaly> {
    compute_and_emit_cadence_anomalies_with_engine(db, None)
}

/// Compute cadence anomalies and optionally propagate via the signal engine.
pub fn compute_and_emit_cadence_anomalies_with_engine(
    db: &ActionDb,
    engine: Option<&PropagationEngine>,
) -> Vec<CadenceAnomaly> {
    let conn = db.conn_ref();

    // Step 1: Aggregate this week's email signals by entity
    let current_week = chrono::Utc::now().format("%G-W%V").to_string();

    let mut stmt = match conn.prepare(
        "SELECT entity_id, entity_type, COUNT(*) as cnt
         FROM email_signals
         WHERE detected_at >= datetime('now', '-7 days')
         GROUP BY entity_id, entity_type
         HAVING cnt >= 1",
    ) {
        Ok(s) => s,
        Err(e) => {
            log::warn!("cadence: failed to query email_signals: {}", e);
            return Vec::new();
        }
    };

    let weekly_counts: Vec<(String, String, i64)> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|r| r.ok())
        .collect();

    let mut anomalies = Vec::new();

    for (entity_id, entity_type, current_count) in &weekly_counts {
        // Step 2: Get rolling average from historical cadence data (last 4 weeks)
        let rolling_avg: f64 = conn
            .query_row(
                "SELECT COALESCE(AVG(message_count), 0.0)
                 FROM entity_email_cadence
                 WHERE entity_id = ?1 AND entity_type = ?2
                   AND period != ?3
                   AND updated_at >= datetime('now', '-30 days')",
                rusqlite::params![entity_id, entity_type, current_week],
                |row| row.get(0),
            )
            .unwrap_or(0.0);

        // Step 3: Upsert current week
        let _ = conn.execute(
            "INSERT INTO entity_email_cadence (entity_id, entity_type, period, message_count, rolling_avg, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))
             ON CONFLICT(entity_id, entity_type, period) DO UPDATE SET
                message_count = excluded.message_count,
                rolling_avg = excluded.rolling_avg,
                updated_at = datetime('now')",
            rusqlite::params![
                entity_id,
                entity_type,
                current_week,
                current_count,
                rolling_avg,
            ],
        );

        // Step 4: Detect anomalies (only if we have enough history)
        if rolling_avg < 2.0 {
            continue; // Not enough baseline to detect anomalies
        }

        let ratio = *current_count as f64 / rolling_avg;

        if ratio < 0.5 {
            let confidence = (1.0 - ratio).min(0.95);
            anomalies.push(CadenceAnomaly {
                entity_id: entity_id.clone(),
                entity_type: entity_type.clone(),
                anomaly_type: "gone_quiet".to_string(),
                current_count: *current_count,
                rolling_avg,
                confidence,
            });
        } else if ratio > 2.0 {
            let confidence = ((ratio - 1.0) / 3.0).min(0.95);
            anomalies.push(CadenceAnomaly {
                entity_id: entity_id.clone(),
                entity_type: entity_type.clone(),
                anomaly_type: "activity_spike".to_string(),
                current_count: *current_count,
                rolling_avg,
                confidence,
            });
        }
    }

    // Step 4b: Detect completely silent entities (had baseline activity but zero this week)
    let active_entity_ids: std::collections::HashSet<(String, String)> = weekly_counts
        .iter()
        .map(|(eid, etype, _)| (eid.clone(), etype.clone()))
        .collect();

    if let Ok(mut silent_stmt) = conn.prepare(
        "SELECT entity_id, entity_type, AVG(message_count) as avg_count
         FROM entity_email_cadence
         WHERE updated_at >= datetime('now', '-30 days')
           AND period != ?1
         GROUP BY entity_id, entity_type
         HAVING avg_count >= 3.0",
    ) {
        let silent_entities: Vec<(String, String, f64)> = silent_stmt
            .query_map(rusqlite::params![current_week], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, f64>(2)?,
                ))
            })
            .ok()
            .into_iter()
            .flatten()
            .filter_map(|r| r.ok())
            .collect();

        for (entity_id, entity_type, avg) in silent_entities {
            if active_entity_ids.contains(&(entity_id.clone(), entity_type.clone())) {
                continue; // Already processed above
            }
            anomalies.push(CadenceAnomaly {
                entity_id,
                entity_type,
                anomaly_type: "gone_quiet".to_string(),
                current_count: 0,
                rolling_avg: avg,
                confidence: 0.90,
            });
        }
    }

    // Step 5: Emit signals for anomalies (with propagation when engine available)
    for anomaly in &anomalies {
        if let Some(eng) = engine {
            let _ = super::bus::emit_signal_and_propagate(
                db,
                eng,
                &anomaly.entity_type,
                &anomaly.entity_id,
                "cadence_anomaly",
                "email_cadence",
                Some(&anomaly.anomaly_type),
                anomaly.confidence,
            );
        } else {
            let _ = super::bus::emit_signal(
                db,
                &anomaly.entity_type,
                &anomaly.entity_id,
                "cadence_anomaly",
                "email_cadence",
                Some(&anomaly.anomaly_type),
                anomaly.confidence,
            );
        }
        log::info!(
            "cadence: {} for {} {} (count={}, avg={:.1})",
            anomaly.anomaly_type,
            anomaly.entity_type,
            anomaly.entity_id,
            anomaly.current_count,
            anomaly.rolling_avg,
        );
    }

    anomalies
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::test_db;

    #[test]
    fn test_cadence_no_signals_returns_empty() {
        let db = test_db();
        let anomalies = compute_and_emit_cadence_anomalies(&db);
        assert!(anomalies.is_empty());
    }

    #[test]
    fn test_cadence_with_signals_no_baseline() {
        let db = test_db();
        // Insert some email signals
        db.conn_ref()
            .execute(
                "INSERT INTO email_signals (email_id, entity_id, entity_type, signal_type, signal_text)
                 VALUES ('e1', 'acct1', 'account', 'inbound', 'test email')",
                [],
            )
            .unwrap();

        let anomalies = compute_and_emit_cadence_anomalies(&db);
        // No baseline (rolling_avg < 2.0), so no anomalies
        assert!(anomalies.is_empty());
    }

    #[test]
    fn test_cadence_gone_quiet_detection() {
        let db = test_db();
        let conn = db.conn_ref();

        // Set up historical baseline: 10 emails/week for past weeks
        for week_offset in 1..=4 {
            let period = format!("2025-W{:02}", 50 - week_offset);
            conn.execute(
                "INSERT INTO entity_email_cadence (entity_id, entity_type, period, message_count, rolling_avg, updated_at)
                 VALUES ('acct1', 'account', ?1, 10, 10.0, datetime('now', ?2))",
                rusqlite::params![period, format!("-{} days", week_offset * 7)],
            )
            .unwrap();
        }

        // Current week: only 2 signals (< 50% of avg 10)
        for i in 0..2 {
            conn.execute(
                "INSERT INTO email_signals (email_id, entity_id, entity_type, signal_type, signal_text)
                 VALUES (?1, 'acct1', 'account', 'inbound', 'test')",
                rusqlite::params![format!("e{}", i)],
            )
            .unwrap();
        }

        let anomalies = compute_and_emit_cadence_anomalies(&db);
        assert_eq!(anomalies.len(), 1);
        assert_eq!(anomalies[0].anomaly_type, "gone_quiet");
        assert_eq!(anomalies[0].entity_id, "acct1");
    }
}
