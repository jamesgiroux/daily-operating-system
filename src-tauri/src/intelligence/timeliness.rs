//! Relevance-window filtering for intelligence items.
//!
//! Class-specific windows define how long each type of intelligence item
//! remains relevant for active display. Stale items are filtered out of
//! account detail views and meeting prep context.

use chrono::{NaiveDateTime, Utc};

/// Class-specific relevance windows (days).
pub const RELEVANCE_WINDOWS: &[(&str, i64)] = &[
    ("support_incident", 45),
    ("active_blocker", 90),
    ("call_theme", 90),
    ("stakeholder_engagement", 120),
    ("compliance_concern", 180),
    ("renewal_opportunity_stage", 14),
];

/// Default window for unclassified items.
pub const DEFAULT_RELEVANCE_DAYS: i64 = 90;

/// Check if an item is still within its relevance window.
///
/// Returns `true` if the item's observation timestamp falls within the
/// class-specific window (or default 90 days). Unparseable timestamps
/// are treated as current (safe default — don't hide items we can't date).
pub fn is_within_relevance_window(evidence_class: &str, observed_at: &str) -> bool {
    let window_days = RELEVANCE_WINDOWS
        .iter()
        .find(|(class, _)| *class == evidence_class)
        .map(|(_, days)| *days)
        .unwrap_or(DEFAULT_RELEVANCE_DAYS);

    let observed = NaiveDateTime::parse_from_str(observed_at, "%Y-%m-%dT%H:%M:%S")
        .or_else(|_| {
            chrono::DateTime::parse_from_rfc3339(observed_at).map(|dt| dt.naive_utc())
        })
        .ok();

    match observed {
        Some(dt) => {
            let cutoff = Utc::now().naive_utc() - chrono::Duration::days(window_days);
            dt > cutoff
        }
        None => true, // Can't parse date -> assume current
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recent_item_is_within_window() {
        let now = Utc::now().to_rfc3339();
        assert!(is_within_relevance_window("active_blocker", &now));
    }

    #[test]
    fn test_stale_item_outside_window() {
        let old = (Utc::now() - chrono::Duration::days(100)).to_rfc3339();
        assert!(!is_within_relevance_window("active_blocker", &old)); // 90-day window
    }

    #[test]
    fn test_support_incident_shorter_window() {
        let recent = (Utc::now() - chrono::Duration::days(30)).to_rfc3339();
        let stale = (Utc::now() - chrono::Duration::days(50)).to_rfc3339();
        assert!(is_within_relevance_window("support_incident", &recent)); // 45-day window
        assert!(!is_within_relevance_window("support_incident", &stale));
    }

    #[test]
    fn test_compliance_longer_window() {
        let within = (Utc::now() - chrono::Duration::days(150)).to_rfc3339();
        let stale = (Utc::now() - chrono::Duration::days(200)).to_rfc3339();
        assert!(is_within_relevance_window("compliance_concern", &within)); // 180-day window
        assert!(!is_within_relevance_window("compliance_concern", &stale));
    }

    #[test]
    fn test_unparseable_timestamp_treated_as_current() {
        assert!(is_within_relevance_window("active_blocker", "not-a-date"));
    }

    #[test]
    fn test_default_window_for_unknown_class() {
        let within = (Utc::now() - chrono::Duration::days(80)).to_rfc3339();
        let stale = (Utc::now() - chrono::Duration::days(100)).to_rfc3339();
        assert!(is_within_relevance_window("unknown_class", &within)); // 90-day default
        assert!(!is_within_relevance_window("unknown_class", &stale));
    }

    #[test]
    fn test_renewal_short_window() {
        let recent = (Utc::now() - chrono::Duration::days(10)).to_rfc3339();
        let stale = (Utc::now() - chrono::Duration::days(20)).to_rfc3339();
        assert!(is_within_relevance_window(
            "renewal_opportunity_stage",
            &recent
        )); // 14-day window
        assert!(!is_within_relevance_window(
            "renewal_opportunity_stage",
            &stale
        ));
    }
}
