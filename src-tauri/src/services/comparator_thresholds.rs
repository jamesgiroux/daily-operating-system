use chrono::Duration;

pub const COMPARATOR_THRESHOLD_VERSION: &str = "adr-0131-thresholds:v1";
pub const HIGH_THRESHOLD: f32 = 0.85;
pub const LOW_THRESHOLD: f32 = 0.60;

pub const AMBIGUOUS_BASE_INTERVAL_DAYS: i64 = 7;
pub const AMBIGUOUS_MAX_ATTEMPTS: i64 = 5;
pub const AMBIGUOUS_BACKOFF_BASE: i64 = 2;
pub const PENDING_BACKFILL_MAX_AGE_HOURS: i64 = 24;
pub const PENDING_BACKFILL_MAX_RETRIES: i64 = 3;

pub fn ambiguous_base_interval() -> Duration {
    Duration::days(AMBIGUOUS_BASE_INTERVAL_DAYS)
}

pub fn pending_backfill_max_age() -> Duration {
    Duration::hours(PENDING_BACKFILL_MAX_AGE_HOURS)
}
