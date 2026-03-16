//! I428: Connectivity tracking and sync freshness.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

/// Freshness status for a sync source.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncFreshness {
    pub source: String,
    pub status: FreshnessStatus,
    pub last_success_at: Option<String>,
    pub last_attempt_at: Option<String>,
    pub last_error: Option<String>,
    pub consecutive_failures: i32,
    pub age_description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FreshnessStatus {
    Green,
    Amber,
    Red,
    Unknown,
}

/// Record a successful sync for the given source.
pub fn record_sync_success(db: &Connection, source: &str) -> Result<(), String> {
    let now = Utc::now().to_rfc3339();
    db.execute(
        "INSERT INTO sync_metadata (source, last_success_at, last_attempt_at, last_error, consecutive_failures)
         VALUES (?1, ?2, ?2, NULL, 0)
         ON CONFLICT(source) DO UPDATE SET
            last_success_at = ?2,
            last_attempt_at = ?2,
            last_error = NULL,
            consecutive_failures = 0",
        params![source, now],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Record a sync failure for the given source.
pub fn record_sync_failure(db: &Connection, source: &str, error: &str) -> Result<(), String> {
    let now = Utc::now().to_rfc3339();
    db.execute(
        "INSERT INTO sync_metadata (source, last_attempt_at, last_error, consecutive_failures)
         VALUES (?1, ?2, ?3, 1)
         ON CONFLICT(source) DO UPDATE SET
            last_attempt_at = ?2,
            last_error = ?3,
            consecutive_failures = consecutive_failures + 1",
        params![source, now, error],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Get freshness status for all tracked sync sources.
pub fn get_sync_freshness(db: &Connection) -> Result<Vec<SyncFreshness>, String> {
    let mut stmt = db
        .prepare(
            "SELECT source, last_success_at, last_attempt_at, last_error, consecutive_failures
             FROM sync_metadata
             ORDER BY source",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            let source: String = row.get(0)?;
            let last_success_at: Option<String> = row.get(1)?;
            let last_attempt_at: Option<String> = row.get(2)?;
            let last_error: Option<String> = row.get(3)?;
            let consecutive_failures: i32 = row.get(4)?;

            let (status, age_description) =
                compute_freshness(&last_success_at, consecutive_failures);

            Ok(SyncFreshness {
                source,
                status,
                last_success_at,
                last_attempt_at,
                last_error,
                consecutive_failures,
                age_description,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row.map_err(|e| e.to_string())?);
    }
    Ok(results)
}

fn compute_freshness(
    last_success: &Option<String>,
    consecutive_failures: i32,
) -> (FreshnessStatus, String) {
    // Require 2+ consecutive failures before showing degraded
    if consecutive_failures >= 2 {
        if let Some(ref ts) = last_success {
            if let Ok(dt) = ts.parse::<DateTime<Utc>>() {
                let age = Utc::now() - dt;
                return (FreshnessStatus::Red, format_age(age));
            }
        }
        return (FreshnessStatus::Red, "Disconnected".to_string());
    }

    match last_success {
        None => (FreshnessStatus::Unknown, "Never synced".to_string()),
        Some(ts) => match ts.parse::<DateTime<Utc>>() {
            Ok(dt) => {
                let age = Utc::now() - dt;
                let minutes = age.num_minutes();
                let status = if minutes < 30 {
                    FreshnessStatus::Green
                } else if minutes < 240 {
                    // 30min - 4h
                    FreshnessStatus::Amber
                } else {
                    FreshnessStatus::Red
                };
                (status, format_age(age))
            }
            Err(_) => (FreshnessStatus::Unknown, "Unknown".to_string()),
        },
    }
}

fn format_age(age: chrono::Duration) -> String {
    let minutes = age.num_minutes();
    if minutes < 1 {
        "Just now".to_string()
    } else if minutes < 60 {
        format!("{}m ago", minutes)
    } else if minutes < 1440 {
        let hours = minutes / 60;
        format!("{}h ago", hours)
    } else {
        let days = minutes / 1440;
        format!("{}d ago", days)
    }
}
