//! I319 / I581: Entity-level email cadence monitoring + silence surfacing.
//!
//! Keeps `entity_email_cadence` fresh from recent email signals, then emits
//! `email_cadence_drop` once per threshold crossing when an account has gone
//! quiet relative to its historical cadence.

use chrono::{DateTime, Utc};

use super::propagation::PropagationEngine;
use crate::db::ActionDb;

/// A cadence drop for a single entity.
#[derive(Debug, Clone)]
pub struct CadenceAnomaly {
    pub entity_id: String,
    pub entity_type: String,
    pub anomaly_type: String,
    pub days_since_last_email: i64,
    pub normal_interval_days: f64,
    pub confidence: f64,
}

fn parse_datetime(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .or_else(|_| DateTime::parse_from_rfc2822(value).map(|dt| dt.with_timezone(&Utc)))
        .or_else(|_| {
            chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S")
                .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))
        })
        .or_else(|_| {
            chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S")
                .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))
        })
        .ok()
}

fn sync_weekly_cadence_rows(db: &ActionDb) {
    let conn = db.conn_ref();
    let current_week = Utc::now().format("%G-W%V").to_string();

    let mut stmt = match conn.prepare(
        "SELECT entity_id, entity_type, COUNT(*) as cnt
         FROM email_signals
         WHERE detected_at >= datetime('now', '-7 days')
           AND entity_id IS NOT NULL
           AND entity_type IS NOT NULL
         GROUP BY entity_id, entity_type
         HAVING cnt >= 1",
    ) {
        Ok(stmt) => stmt,
        Err(err) => {
            log::warn!("cadence: failed to query email_signals: {}", err);
            return;
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
        .filter_map(|row| row.ok())
        .collect();

    for (entity_id, entity_type, current_count) in weekly_counts {
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
    }
}

/// Compute cadence anomalies and optionally propagate via the signal engine.
pub fn compute_and_emit_cadence_anomalies(db: &ActionDb) -> Vec<CadenceAnomaly> {
    compute_and_emit_cadence_anomalies_with_engine(db, None)
}

/// Compute cadence anomalies and optionally propagate via the signal engine.
pub fn compute_and_emit_cadence_anomalies_with_engine(
    db: &ActionDb,
    engine: Option<&PropagationEngine>,
) -> Vec<CadenceAnomaly> {
    sync_weekly_cadence_rows(db);

    let quiet_accounts = crate::services::emails::detect_gone_quiet_accounts(db).unwrap_or_default();
    let mut anomalies = Vec::new();

    for account in quiet_accounts {
        let last_email_at = account
            .last_email_date
            .as_deref()
            .and_then(parse_datetime);

        let active_signal_at = db
            .conn_ref()
            .query_row(
                "SELECT created_at
                 FROM signal_events
                 WHERE entity_type = 'account'
                   AND entity_id = ?1
                   AND signal_type = 'email_cadence_drop'
                   AND superseded_by IS NULL
                 ORDER BY created_at DESC
                 LIMIT 1",
                rusqlite::params![account.entity_id],
                |row| row.get::<_, String>(0),
            )
            .ok()
            .and_then(|created_at| parse_datetime(&created_at));

        if let (Some(active_signal_at), Some(last_email_at)) = (active_signal_at, last_email_at) {
            if active_signal_at >= last_email_at {
                continue;
            }
        }

        anomalies.push(CadenceAnomaly {
            entity_id: account.entity_id,
            entity_type: account.entity_type,
            anomaly_type: "gone_quiet".to_string(),
            days_since_last_email: account.days_since_last_email,
            normal_interval_days: account.normal_interval_days,
            confidence: 0.6,
        });
    }

    for anomaly in &anomalies {
        let value = format!(
            r#"{{"days_since_last_email":{},"normal_interval_days":{:.2}}}"#,
            anomaly.days_since_last_email, anomaly.normal_interval_days
        );
        if let Some(engine) = engine {
            let _ = super::bus::emit_signal_and_propagate(
                db,
                engine,
                &anomaly.entity_type,
                &anomaly.entity_id,
                "email_cadence_drop",
                "email_cadence",
                Some(&value),
                anomaly.confidence,
            );
        } else {
            let _ = super::bus::emit_signal(
                db,
                &anomaly.entity_type,
                &anomaly.entity_id,
                "email_cadence_drop",
                "email_cadence",
                Some(&value),
                anomaly.confidence,
            );
        }
        log::info!(
            "cadence: {} for {} {} (days_since_last_email={}, normal_interval_days={:.1})",
            anomaly.anomaly_type,
            anomaly.entity_type,
            anomaly.entity_id,
            anomaly.days_since_last_email,
            anomaly.normal_interval_days,
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
        db.conn_ref()
            .execute(
                "INSERT INTO email_signals (email_id, entity_id, entity_type, signal_type, signal_text)
                 VALUES ('e1', 'acct1', 'account', 'inbound', 'test email')",
                [],
            )
            .unwrap();

        let anomalies = compute_and_emit_cadence_anomalies(&db);
        assert!(anomalies.is_empty());
    }

    #[test]
    fn test_cadence_gone_quiet_detection() {
        let db = test_db();
        let conn = db.conn_ref();

        conn.execute(
            "INSERT INTO accounts (id, name, account_type, updated_at)
             VALUES ('acct1', 'Acme', 'customer', datetime('now'))",
            [],
        )
        .unwrap();

        for week_offset in 1..=3 {
            let period = format!("2025-W{:02}", 50 - week_offset);
            conn.execute(
                "INSERT INTO entity_email_cadence (entity_id, entity_type, period, message_count, rolling_avg, updated_at)
                 VALUES ('acct1', 'account', ?1, 2, 2.0, datetime('now', ?2))",
                rusqlite::params![period, format!("-{} days", week_offset * 7)],
            )
            .unwrap();
        }

        conn.execute(
            "INSERT INTO emails (email_id, thread_id, sender_email, sender_name, subject, snippet, priority, is_unread, received_at, entity_id, entity_type, created_at, updated_at)
             VALUES ('e1', 'thread-1', 'owner@acme.test', 'Owner', 'Check-in', '', 'high', 1, datetime('now', '-18 days'), 'acct1', 'account', datetime('now', '-18 days'), datetime('now', '-18 days'))",
            [],
        )
        .unwrap();

        let anomalies = compute_and_emit_cadence_anomalies(&db);
        assert_eq!(anomalies.len(), 1);
        assert_eq!(anomalies[0].anomaly_type, "gone_quiet");
        assert_eq!(anomalies[0].entity_id, "acct1");
        assert!(anomalies[0].days_since_last_email >= 18);
    }
}
