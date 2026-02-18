//! Temporal decay for signal weighting (pure math, no DB).

use chrono::{DateTime, Utc};

/// Compute the decayed weight of a signal using exponential half-life decay.
///
/// `base * 2^(-age_days / half_life_days)`
pub fn decayed_weight(base_weight: f64, age_days: f64, half_life_days: f64) -> f64 {
    if half_life_days <= 0.0 || age_days < 0.0 {
        return base_weight;
    }
    base_weight * (2.0_f64).powf(-age_days / half_life_days)
}

/// Parse an RFC3339/ISO-8601 timestamp and compute fractional days since now.
pub fn age_days_from_now(created_at_iso: &str) -> f64 {
    let parsed = match DateTime::parse_from_rfc3339(created_at_iso) {
        Ok(dt) => dt.with_timezone(&Utc),
        Err(_) => {
            // Try SQLite datetime format (no timezone)
            match chrono::NaiveDateTime::parse_from_str(created_at_iso, "%Y-%m-%d %H:%M:%S") {
                Ok(naive) => naive.and_utc(),
                Err(_) => return 0.0,
            }
        }
    };
    let duration = Utc::now() - parsed;
    let secs = duration.num_seconds() as f64;
    (secs / 86400.0).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_half_life_gives_half_weight() {
        let result = decayed_weight(1.0, 30.0, 30.0);
        assert!(
            (result - 0.5).abs() < 0.001,
            "expected ~0.5, got {}",
            result
        );
    }

    #[test]
    fn test_zero_age_full_weight() {
        let result = decayed_weight(0.8, 0.0, 90.0);
        assert!((result - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_double_half_life_quarter_weight() {
        let result = decayed_weight(1.0, 60.0, 30.0);
        assert!(
            (result - 0.25).abs() < 0.001,
            "expected ~0.25, got {}",
            result
        );
    }

    #[test]
    fn test_negative_age_returns_base() {
        assert_eq!(decayed_weight(0.9, -5.0, 30.0), 0.9);
    }

    #[test]
    fn test_zero_half_life_returns_base() {
        assert_eq!(decayed_weight(0.9, 10.0, 0.0), 0.9);
    }

    #[test]
    fn test_age_days_from_recent() {
        let now = Utc::now().to_rfc3339();
        let age = age_days_from_now(&now);
        assert!(age < 0.01, "just-created should be ~0 days old, got {}", age);
    }

    #[test]
    fn test_age_days_sqlite_format() {
        // SQLite datetime('now') format
        let ts = "2020-01-01 00:00:00";
        let age = age_days_from_now(ts);
        assert!(age > 365.0, "2020 timestamp should be >365 days old, got {}", age);
    }
}
