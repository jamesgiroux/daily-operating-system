//! Calendar gap computation and focus block suggestions.
//!
//! Ported from ops/gap_analysis.py per ADR-0049.

use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use chrono_tz::Tz;
use serde_json::{json, Value};

use super::constants::{DAY_NAMES, MIN_GAP_MINUTES, WORK_DAY_END_HOUR, WORK_DAY_START_HOUR};

/// Parse an ISO datetime string to a NaiveDateTime, converting to the user's
/// timezone when the input contains timezone info (UTC 'Z' or offset).
///
/// When `user_tz` is Some, UTC/offset timestamps are converted to local time
/// before stripping the timezone — so "2026-02-09T22:00:00Z" with
/// America/New_York becomes 17:00, not 22:00.
///
/// When `user_tz` is None, falls back to stripping timezone info naively
/// (original behavior).
fn parse_event_dt(time_str: &str, user_tz: Option<Tz>) -> Option<NaiveDateTime> {
    if time_str.is_empty() {
        return None;
    }
    if time_str.contains('T') {
        // Try to parse as a full timezone-aware datetime first
        if let Some(tz) = user_tz {
            if let Ok(dt) = DateTime::parse_from_rfc3339(time_str) {
                // Convert to user timezone and extract naive local time
                let local = dt.with_timezone(&tz);
                return Some(local.naive_local());
            }
            // Also try ISO with offset but no fractional seconds
            if let Ok(dt) = DateTime::<FixedOffset>::parse_from_str(time_str, "%Y-%m-%dT%H:%M:%S%:z") {
                let local = dt.with_timezone(&tz);
                return Some(local.naive_local());
            }
        }

        // Fallback: strip timezone info naively (for bare datetimes without offset)
        let cleaned = time_str
            .trim_end_matches('Z')
            .split('+')
            .next()
            .unwrap_or(time_str);
        // Handle negative offset: find the last '-' after 'T'
        let cleaned = if let Some(t_pos) = cleaned.find('T') {
            if let Some(dash_pos) = cleaned[t_pos + 1..].rfind('-') {
                &cleaned[..t_pos + 1 + dash_pos]
            } else {
                cleaned
            }
        } else {
            cleaned
        };
        NaiveDateTime::parse_from_str(cleaned, "%Y-%m-%dT%H:%M:%S")
            .or_else(|_| NaiveDateTime::parse_from_str(cleaned, "%Y-%m-%dT%H:%M"))
            .ok()
    } else {
        // Date-only (all-day event)
        NaiveDate::parse_from_str(time_str, "%Y-%m-%d")
            .ok()
            .map(|d| d.and_hms_opt(0, 0, 0).unwrap())
    }
}

/// Find free time blocks >= MIN_GAP_MINUTES between meetings on a day.
///
/// Operates within work hours (WORK_DAY_START_HOUR to WORK_DAY_END_HOUR).
/// When `user_tz` is provided, UTC timestamps are converted to local time.
pub fn compute_gaps(events: &[Value], day_date: NaiveDate, user_tz: Option<Tz>) -> Vec<Value> {
    let day_start = NaiveDateTime::new(
        day_date,
        NaiveTime::from_hms_opt(WORK_DAY_START_HOUR, 0, 0).unwrap(),
    );
    let day_end = NaiveDateTime::new(
        day_date,
        NaiveTime::from_hms_opt(WORK_DAY_END_HOUR, 0, 0).unwrap(),
    );

    // Parse and sort event intervals, skipping all-day events (>= 24h duration)
    let mut intervals: Vec<(NaiveDateTime, NaiveDateTime)> = Vec::new();
    for ev in events {
        let start_str = ev.get("start").and_then(|v| v.as_str()).unwrap_or_default();
        let end_str = ev.get("end").and_then(|v| v.as_str()).unwrap_or_default();
        if let (Some(s), Some(e)) = (parse_event_dt(start_str, user_tz), parse_event_dt(end_str, user_tz)) {
            // Skip all-day events: duration >= 24 hours or midnight-to-midnight
            let duration_hours = (e - s).num_hours();
            if duration_hours >= 24 {
                continue;
            }
            // Also skip events starting at midnight (common all-day pattern)
            if s.time() == NaiveTime::from_hms_opt(0, 0, 0).unwrap()
                && e.time() == NaiveTime::from_hms_opt(0, 0, 0).unwrap()
            {
                continue;
            }
            intervals.push((s, e));
        }
    }
    intervals.sort_by_key(|i| i.0);

    let mut gaps = Vec::new();
    let mut cursor = day_start;

    for (start, end) in &intervals {
        let start = (*start).max(day_start);
        let end = (*end).min(day_end);

        if start > cursor {
            let duration = (start - cursor).num_minutes();
            if duration >= MIN_GAP_MINUTES {
                gaps.push(json!({
                    "start": cursor.format("%Y-%m-%dT%H:%M:%S").to_string(),
                    "end": start.format("%Y-%m-%dT%H:%M:%S").to_string(),
                    "duration_minutes": duration,
                }));
            }
        }
        cursor = cursor.max(end);
    }

    // Gap after last meeting
    if cursor < day_end {
        let duration = (day_end - cursor).num_minutes();
        if duration >= MIN_GAP_MINUTES {
            gaps.push(json!({
                "start": cursor.format("%Y-%m-%dT%H:%M:%S").to_string(),
                "end": day_end.format("%Y-%m-%dT%H:%M:%S").to_string(),
                "duration_minutes": duration,
            }));
        }
    }

    gaps
}

/// Compute gaps for each weekday.
/// When `user_tz` is provided, UTC timestamps are converted to local time.
pub fn compute_all_gaps(events_by_day: &Value, monday: NaiveDate, user_tz: Option<Tz>) -> Value {
    let mut result = serde_json::Map::new();
    for (i, day_name) in DAY_NAMES.iter().enumerate() {
        let day_date = monday + chrono::Duration::days(i as i64);
        let day_events = events_by_day
            .get(day_name)
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        result.insert(
            day_name.to_string(),
            Value::Array(compute_gaps(&day_events, day_date, user_tz)),
        );
    }
    Value::Object(result)
}

/// Generate focus-time suggestions from large gaps.
///
/// Prioritizes morning slots (deep work) and afternoon slots (admin).
pub fn suggest_focus_blocks(gaps_by_day: &Value) -> Vec<Value> {
    let mut suggestions = Vec::new();

    for day_name in DAY_NAMES {
        let day_gaps = match gaps_by_day.get(day_name).and_then(|v| v.as_array()) {
            Some(g) => g,
            None => continue,
        };

        for gap in day_gaps {
            let duration = gap
                .get("duration_minutes")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            if duration < MIN_GAP_MINUTES {
                continue;
            }

            let start_str = gap.get("start").and_then(|v| v.as_str()).unwrap_or("");
            let start_dt = match parse_event_dt(start_str, None) {
                Some(dt) => dt,
                None => continue,
            };

            let block_type = if start_dt.hour() < 12 {
                "Deep Work"
            } else {
                "Admin / Follow-up"
            };

            suggestions.push(json!({
                "day": day_name,
                "start": gap.get("start"),
                "end": gap.get("end"),
                "duration_minutes": duration,
                "suggested_use": block_type,
            }));
        }
    }

    suggestions
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_event(start: &str, end: &str) -> Value {
        json!({
            "start": start,
            "end": end,
        })
    }

    #[test]
    fn test_parse_event_dt_iso() {
        let dt = parse_event_dt("2026-02-08T09:00:00-05:00", None);
        assert!(dt.is_some());
        assert_eq!(dt.unwrap().hour(), 9);
    }

    #[test]
    fn test_parse_event_dt_utc() {
        let dt = parse_event_dt("2026-02-08T14:00:00Z", None);
        assert!(dt.is_some());
        assert_eq!(dt.unwrap().hour(), 14);
    }

    #[test]
    fn test_parse_event_dt_date_only() {
        let dt = parse_event_dt("2026-02-08", None);
        assert!(dt.is_some());
        assert_eq!(dt.unwrap().hour(), 0);
    }

    #[test]
    fn test_parse_event_dt_utc_with_timezone() {
        // 22:00 UTC = 17:00 EST (America/New_York, -5 in winter)
        let est: Tz = "America/New_York".parse().unwrap();
        let dt = parse_event_dt("2026-02-08T22:00:00Z", Some(est));
        assert!(dt.is_some());
        assert_eq!(dt.unwrap().hour(), 17); // 5 PM local, not 10 PM
    }

    #[test]
    fn test_parse_event_dt_offset_with_timezone() {
        // 17:00-05:00 = 17:00 EST (already in user's tz effectively)
        let est: Tz = "America/New_York".parse().unwrap();
        let dt = parse_event_dt("2026-02-08T17:00:00-05:00", Some(est));
        assert!(dt.is_some());
        assert_eq!(dt.unwrap().hour(), 17);
    }

    #[test]
    fn test_utc_event_no_phantom_gap() {
        // A 5 PM EST meeting stored as 22:00Z should NOT create gaps after 17:00 local
        let est: Tz = "America/New_York".parse().unwrap();
        let day = NaiveDate::from_ymd_opt(2026, 2, 9).unwrap();
        let events = vec![make_event("2026-02-09T22:00:00Z", "2026-02-09T23:00:00Z")];
        let gaps = compute_gaps(&events, day, Some(est));

        // 22:00Z = 17:00 EST = end of work day. No gap after it.
        // Only gap should be 9:00 AM - 5:00 PM (480 min) since meeting is at boundary.
        for gap in &gaps {
            let end_str = gap.get("end").and_then(|v| v.as_str()).unwrap_or("");
            let end_dt = parse_event_dt(end_str, None).unwrap();
            assert!(end_dt.hour() <= WORK_DAY_END_HOUR as u32);
        }
    }

    #[test]
    fn test_compute_gaps_empty_day() {
        let day = NaiveDate::from_ymd_opt(2026, 2, 9).unwrap(); // Monday
        let gaps = compute_gaps(&[], day, None);
        assert_eq!(gaps.len(), 1);
        // Full work day: 9:00-17:00 = 480 min
        assert_eq!(gaps[0]["duration_minutes"], 480);
    }

    #[test]
    fn test_compute_gaps_one_meeting() {
        let day = NaiveDate::from_ymd_opt(2026, 2, 9).unwrap();
        let events = vec![make_event("2026-02-09T10:00:00", "2026-02-09T11:00:00")];
        let gaps = compute_gaps(&events, day, None);

        // 9:00-10:00 (60 min) and 11:00-17:00 (360 min)
        assert_eq!(gaps.len(), 2);
        assert_eq!(gaps[0]["duration_minutes"], 60);
        assert_eq!(gaps[1]["duration_minutes"], 360);
    }

    #[test]
    fn test_compute_gaps_adjacent_meetings() {
        let day = NaiveDate::from_ymd_opt(2026, 2, 9).unwrap();
        let events = vec![
            make_event("2026-02-09T10:00:00", "2026-02-09T11:00:00"),
            make_event("2026-02-09T11:00:00", "2026-02-09T12:00:00"),
        ];
        let gaps = compute_gaps(&events, day, None);

        // 9:00-10:00 (60 min) and 12:00-17:00 (300 min)
        assert_eq!(gaps.len(), 2);
        assert_eq!(gaps[0]["duration_minutes"], 60);
        assert_eq!(gaps[1]["duration_minutes"], 300);
    }

    #[test]
    fn test_compute_gaps_below_threshold() {
        let day = NaiveDate::from_ymd_opt(2026, 2, 9).unwrap();
        let events = vec![
            make_event("2026-02-09T09:00:00", "2026-02-09T09:40:00"),
            make_event("2026-02-09T09:50:00", "2026-02-09T17:00:00"),
        ];
        let gaps = compute_gaps(&events, day, None);

        // 10 min gap at 9:40-9:50 — below threshold, should not appear
        assert_eq!(gaps.len(), 0);
    }

    #[test]
    fn test_suggest_focus_blocks_morning_afternoon() {
        let day = NaiveDate::from_ymd_opt(2026, 2, 9).unwrap();
        let events = vec![make_event("2026-02-09T12:00:00", "2026-02-09T13:00:00")];
        let gaps = compute_gaps(&events, day, None);

        let gaps_by_day = json!({ "Monday": gaps });
        let suggestions = suggest_focus_blocks(&gaps_by_day);

        // Should have morning (9:00-12:00 = Deep Work) and afternoon (13:00-17:00 = Admin)
        assert_eq!(suggestions.len(), 2);
        assert_eq!(suggestions[0]["suggested_use"], "Deep Work");
        assert_eq!(suggestions[1]["suggested_use"], "Admin / Follow-up");
    }

    #[test]
    fn test_compute_all_gaps() {
        let monday = NaiveDate::from_ymd_opt(2026, 2, 9).unwrap();
        let events_by_day = json!({
            "Monday": [],
            "Tuesday": [],
            "Wednesday": [],
            "Thursday": [],
            "Friday": [],
        });
        let result = compute_all_gaps(&events_by_day, monday, None);
        assert!(result.get("Monday").is_some());
        assert!(result.get("Friday").is_some());
        // Each empty day has one gap (full work day)
        assert_eq!(result["Monday"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_all_day_events_skipped() {
        // An all-day event (midnight-to-midnight) should NOT consume work hours
        let day = NaiveDate::from_ymd_opt(2026, 2, 16).unwrap(); // Monday
        let events = vec![
            make_event("2026-02-16T00:00:00Z", "2026-02-17T00:00:00Z"), // all-day "Home"
            make_event("2026-02-16T14:00:00", "2026-02-16T14:30:00"),   // 30 min meeting
        ];
        let gaps = compute_gaps(&events, day, None);

        // Should have gaps (all-day event skipped, 30-min meeting leaves gaps)
        assert!(!gaps.is_empty(), "All-day events should be skipped in gap analysis");
        let total: i64 = gaps
            .iter()
            .filter_map(|g| g.get("duration_minutes").and_then(|v| v.as_i64()))
            .sum();
        // Full day = 480 min, minus 30 min meeting = 450 min of gaps
        assert_eq!(total, 450);
    }

    #[test]
    fn test_all_day_events_with_timezone_skipped() {
        // UTC midnight-to-midnight becomes EST 7pm-to-7pm, but should still be detected
        let est: Tz = "America/New_York".parse().unwrap();
        let day = NaiveDate::from_ymd_opt(2026, 2, 16).unwrap();
        let events = vec![
            make_event("2026-02-16T00:00:00Z", "2026-02-17T00:00:00Z"), // all-day
        ];
        let gaps = compute_gaps(&events, day, Some(est));

        // Should have one gap (full work day) since the all-day event is skipped
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0]["duration_minutes"], 480);
    }
}
