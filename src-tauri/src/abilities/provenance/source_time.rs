//! Source-time parsing for `SourceAttribution.source_asof`.
//!
//! Three timestamps coexist on every claim and must not be conflated:
//!
//! - `source_asof` — when the evidence we cite was *itself* true /
//!   produced. The renewal email was sent on 2025-09-12. The
//!   support ticket closed at 2025-11-03T14:00Z. This is what
//!   freshness math wants.
//! - `observed_at` — when DailyOS *ingested* the evidence. The
//!   email arrived in our store at 2025-09-12T15:01Z; we crawled
//!   it ten seconds later. This is mostly an audit timestamp.
//! - `created_at` — when the *claim row* was written. Synthesis
//!   happens after observation; this is wall-clock at insert.
//!
//! The Trust Compiler's freshness factor reads `source_asof` when
//! it's known and trusted. This module produces that boundary:
//! parse strictly, classify implausible vs malformed, and let the
//! caller decide whether to lift the value into provenance or
//! emit a `SourceTimestampUnknown` warning.
//!
//! Bounds rationale:
//! - Malformed (drops the value entirely):
//!   - unparseable string
//!   - missing timezone (we can't compare without one)
//!   - earlier than 2015-01-01 (DailyOS predates nothing in our
//!     workspace; values this old are almost always synthesizer
//!     hallucinations or epoch-zero leaks)
//!   - more than five years in the future (clock skew or LLM
//!     fabrication)
//! - Implausible (lifts the value for traceability but the
//!   freshness factor must NOT trust it):
//!   - before the entity's first observation (a "fact" about
//!     2018 on an account we first saw in 2024 is suspicious)
//!   - more than 30 days in the future (mild skew is OK; a
//!     month is not)

use chrono::{DateTime, NaiveDate, TimeZone, Utc};

/// Result of parsing a candidate `source_asof` string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceTimestampStatus {
    /// Parsed cleanly and within bounds. Lift into
    /// `SourceAttribution.source_asof`.
    Accepted(DateTime<Utc>),
    /// Parsed cleanly but outside the plausibility window. Lift
    /// the parsed value for audit/traceability, but downstream
    /// trust math MUST treat it as unknown for freshness purposes.
    Implausible {
        parsed: DateTime<Utc>,
        reason: SourceTimestampImplausibleReason,
    },
    /// Could not be safely interpreted. Drop the value entirely
    /// and emit a `SourceTimestampUnknown` warning. Backfill
    /// callers route this to the quarantine table.
    Malformed(SourceTimestampMalformedReason),
    /// Caller didn't supply a candidate string at all.
    Missing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceTimestampImplausibleReason {
    /// Parsed timestamp is earlier than the subject entity's
    /// first observation in DailyOS.
    BeforeEntityOrigin,
    /// Parsed timestamp is more than 30 days in the future
    /// relative to `now`.
    NearFuture,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceTimestampMalformedReason {
    /// Parser couldn't extract any timestamp from the input.
    Unparseable,
    /// Parsed but the input had no timezone — comparison would
    /// be ambiguous so we refuse it.
    MissingTimezone,
    /// Earlier than 2015-01-01; almost certainly an epoch leak
    /// or hallucination, not real evidence.
    BeforeMinimumPlausibleDate,
    /// More than 5 years in the future; clock skew or fabrication.
    FarFuture,
}

/// Lower bound: any source timestamp before this date is treated
/// as malformed. 2015-01-01 predates DailyOS by years; nothing
/// older has any chance of being legitimate evidence about a
/// workspace entity.
fn minimum_plausible_date() -> DateTime<Utc> {
    let date = NaiveDate::from_ymd_opt(2015, 1, 1).unwrap();
    Utc.from_utc_datetime(&date.and_hms_opt(0, 0, 0).unwrap())
}

/// Upper bound for malformed: 5 years past `now`. Beyond this is
/// almost certainly clock skew or LLM fabrication.
fn far_future_after(now: DateTime<Utc>) -> DateTime<Utc> {
    now + chrono::Duration::days(365 * 5)
}

/// Upper bound for implausible (vs malformed): 30 days past `now`.
/// Mild clock skew is fine; a month out is not.
fn near_future_after(now: DateTime<Utc>) -> DateTime<Utc> {
    now + chrono::Duration::days(30)
}

/// Parse a candidate `source_asof` string and classify it.
///
/// `now` is supplied (not pulled from `Utc::now()`) so backfill
/// jobs and tests get deterministic bounds. `entity_origin` is
/// the earliest observation timestamp for the claim's subject; if
/// `None`, the BeforeEntityOrigin check is skipped (e.g. backfill
/// for entities whose origin we can't reconstruct).
pub fn parse_source_timestamp(
    input: Option<&str>,
    now: DateTime<Utc>,
    entity_origin: Option<DateTime<Utc>>,
) -> SourceTimestampStatus {
    let Some(raw) = input else {
        return SourceTimestampStatus::Missing;
    };
    let raw = raw.trim();
    if raw.is_empty() {
        return SourceTimestampStatus::Missing;
    }

    // Reject inputs without a timezone before the strict parser
    // happens to accept them. RFC3339 timestamps end in `Z` or a
    // `+HH:MM` / `-HH:MM` offset; anything else is ambiguous.
    if !has_timezone_marker(raw) {
        return SourceTimestampStatus::Malformed(
            SourceTimestampMalformedReason::MissingTimezone,
        );
    }

    let parsed = match DateTime::parse_from_rfc3339(raw) {
        Ok(dt) => dt.with_timezone(&Utc),
        Err(_) => {
            return SourceTimestampStatus::Malformed(
                SourceTimestampMalformedReason::Unparseable,
            )
        }
    };

    if parsed < minimum_plausible_date() {
        return SourceTimestampStatus::Malformed(
            SourceTimestampMalformedReason::BeforeMinimumPlausibleDate,
        );
    }
    if parsed > far_future_after(now) {
        return SourceTimestampStatus::Malformed(SourceTimestampMalformedReason::FarFuture);
    }

    if parsed > near_future_after(now) {
        return SourceTimestampStatus::Implausible {
            parsed,
            reason: SourceTimestampImplausibleReason::NearFuture,
        };
    }
    if let Some(origin) = entity_origin {
        if parsed < origin {
            return SourceTimestampStatus::Implausible {
                parsed,
                reason: SourceTimestampImplausibleReason::BeforeEntityOrigin,
            };
        }
    }

    SourceTimestampStatus::Accepted(parsed)
}

/// True when the trailing characters of an RFC3339 timestamp carry
/// timezone information. Catches inputs like `"2025-09-12T15:01:00"`
/// (no offset) before the parser gives them an arbitrary local
/// interpretation.
fn has_timezone_marker(s: &str) -> bool {
    if s.ends_with('Z') || s.ends_with('z') {
        return true;
    }
    // Look for a +HH:MM or -HH:MM offset in the last six chars.
    // This is conservative; it doesn't try to validate the digits.
    let tail: Vec<char> = s.chars().rev().take(6).collect();
    if tail.len() == 6 {
        // Reversed, so the offset reads backwards: MM:HH±
        let last = tail[5];
        let colon_pos = tail[2];
        if (last == '+' || last == '-') && colon_pos == ':' {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now_2026_05_01() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 5, 1, 12, 0, 0).unwrap()
    }

    #[test]
    fn parse_source_timestamp_accepts_rfc3339_with_timezone() {
        let now = now_2026_05_01();
        let res = parse_source_timestamp(Some("2025-09-12T15:01:00Z"), now, None);
        match res {
            SourceTimestampStatus::Accepted(dt) => {
                assert_eq!(dt.to_rfc3339(), "2025-09-12T15:01:00+00:00");
            }
            other => panic!("expected Accepted, got {other:?}"),
        }
    }

    #[test]
    fn parse_source_timestamp_accepts_offset_form() {
        let now = now_2026_05_01();
        let res = parse_source_timestamp(Some("2025-09-12T15:01:00-04:00"), now, None);
        assert!(matches!(res, SourceTimestampStatus::Accepted(_)));
    }

    #[test]
    fn parse_source_timestamp_rejects_timezone_less() {
        let now = now_2026_05_01();
        let res = parse_source_timestamp(Some("2025-09-12T15:01:00"), now, None);
        assert_eq!(
            res,
            SourceTimestampStatus::Malformed(SourceTimestampMalformedReason::MissingTimezone)
        );
    }

    #[test]
    fn parse_source_timestamp_rejects_unparseable() {
        let now = now_2026_05_01();
        let res = parse_source_timestamp(Some("yesterday"), now, None);
        assert!(matches!(
            res,
            SourceTimestampStatus::Malformed(SourceTimestampMalformedReason::MissingTimezone)
                | SourceTimestampStatus::Malformed(SourceTimestampMalformedReason::Unparseable)
        ));
        // `garbleZ` ends in Z so passes timezone check; should
        // still be unparseable.
        let res = parse_source_timestamp(Some("garbleZ"), now, None);
        assert_eq!(
            res,
            SourceTimestampStatus::Malformed(SourceTimestampMalformedReason::Unparseable)
        );
    }

    #[test]
    fn parse_source_timestamp_rejects_before_2015() {
        let now = now_2026_05_01();
        let res = parse_source_timestamp(Some("2014-12-31T23:59:59Z"), now, None);
        assert_eq!(
            res,
            SourceTimestampStatus::Malformed(
                SourceTimestampMalformedReason::BeforeMinimumPlausibleDate
            )
        );
        // Epoch zero leak.
        let res = parse_source_timestamp(Some("1970-01-01T00:00:00Z"), now, None);
        assert_eq!(
            res,
            SourceTimestampStatus::Malformed(
                SourceTimestampMalformedReason::BeforeMinimumPlausibleDate
            )
        );
    }

    #[test]
    fn parse_source_timestamp_rejects_far_future() {
        let now = now_2026_05_01();
        // 6 years out — past the 5-year cutoff.
        let res = parse_source_timestamp(Some("2032-05-02T00:00:00Z"), now, None);
        assert_eq!(
            res,
            SourceTimestampStatus::Malformed(SourceTimestampMalformedReason::FarFuture)
        );
    }

    #[test]
    fn parse_source_timestamp_marks_near_future_implausible() {
        let now = now_2026_05_01();
        // 60 days out — past the 30-day plausibility window but
        // well within the 5-year malformed cutoff.
        let res = parse_source_timestamp(Some("2026-07-01T00:00:00Z"), now, None);
        match res {
            SourceTimestampStatus::Implausible { reason, .. } => {
                assert_eq!(reason, SourceTimestampImplausibleReason::NearFuture);
            }
            other => panic!("expected Implausible NearFuture, got {other:?}"),
        }
    }

    #[test]
    fn parse_source_timestamp_marks_before_entity_origin_implausible() {
        let now = now_2026_05_01();
        let entity_origin = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        // 2018 is well after 2015 (so not malformed) but before
        // entity origin in 2024 — so implausible.
        let res = parse_source_timestamp(
            Some("2018-06-15T12:00:00Z"),
            now,
            Some(entity_origin),
        );
        match res {
            SourceTimestampStatus::Implausible { parsed, reason } => {
                assert_eq!(reason, SourceTimestampImplausibleReason::BeforeEntityOrigin);
                // Parsed value lifted for traceability.
                assert_eq!(parsed.to_rfc3339(), "2018-06-15T12:00:00+00:00");
            }
            other => panic!("expected Implausible BeforeEntityOrigin, got {other:?}"),
        }
    }

    #[test]
    fn parse_source_timestamp_skips_origin_check_when_no_origin_supplied() {
        let now = now_2026_05_01();
        // Same 2018 timestamp; without an entity_origin to compare
        // against, the only checks are min/max bounds, so this
        // accepts cleanly.
        let res = parse_source_timestamp(Some("2018-06-15T12:00:00Z"), now, None);
        assert!(matches!(res, SourceTimestampStatus::Accepted(_)));
    }

    #[test]
    fn parse_source_timestamp_handles_missing_input() {
        let now = now_2026_05_01();
        assert_eq!(
            parse_source_timestamp(None, now, None),
            SourceTimestampStatus::Missing
        );
        assert_eq!(
            parse_source_timestamp(Some(""), now, None),
            SourceTimestampStatus::Missing
        );
        assert_eq!(
            parse_source_timestamp(Some("   "), now, None),
            SourceTimestampStatus::Missing
        );
    }

    #[test]
    fn parse_source_timestamp_implausible_lifts_parsed_value_for_audit() {
        let now = now_2026_05_01();
        // Implausible result must carry the parsed value so the
        // backfill audit row records what the raw string actually
        // meant — even though trust math will ignore it.
        let res = parse_source_timestamp(Some("2026-07-01T00:00:00Z"), now, None);
        if let SourceTimestampStatus::Implausible { parsed, .. } = res {
            assert_eq!(parsed.to_rfc3339(), "2026-07-01T00:00:00+00:00");
        } else {
            panic!("expected Implausible variant");
        }
    }

    #[test]
    fn parse_source_timestamp_boundary_at_5_year_cutoff() {
        let now = now_2026_05_01();
        // Exactly 5 years past now. At-or-just-before is acceptable;
        // beyond is FarFuture. The boundary test catches off-by-one
        // drift in the bounds helper.
        let exactly_5y = "2031-04-30T12:00:00Z"; // ~5y minus a day
        let res = parse_source_timestamp(Some(exactly_5y), now, None);
        assert!(
            matches!(
                res,
                SourceTimestampStatus::Accepted(_) | SourceTimestampStatus::Implausible { .. }
            ),
            "5y minus one day must not be FarFuture, got {res:?}"
        );
    }
}
