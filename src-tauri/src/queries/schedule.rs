use chrono::{DateTime, Duration, NaiveDate, Timelike, Utc};
use serde_json::json;

use crate::types::{CalendarEvent, Meeting, TimeBlock};

fn map_gaps_to_blocks(gaps: Vec<serde_json::Value>) -> Vec<TimeBlock> {
    gaps.into_iter()
        .filter_map(|gap| {
            let start = gap.get("start")?.as_str()?.to_string();
            let end = gap.get("end")?.as_str()?.to_string();
            let duration_minutes = gap.get("duration_minutes")?.as_u64()? as u32;
            let hour = DateTime::parse_from_rfc3339(&start)
                .ok()
                .map(|dt| dt.hour())
                .unwrap_or(12);
            let suggested_use = if hour < 12 {
                Some("Deep Work".to_string())
            } else {
                Some("Admin / Follow-up".to_string())
            };
            Some(TimeBlock {
                day: String::new(),
                start,
                end,
                duration_minutes,
                suggested_use,
                action_id: None,
                meeting_id: None,
            })
        })
        .collect()
}

/// Compute available focus blocks from live calendar state.
pub fn available_blocks_from_live(events: &[CalendarEvent], day_date: NaiveDate) -> Vec<TimeBlock> {
    let meeting_events: Vec<serde_json::Value> = events
        .iter()
        .filter(|e| !e.is_all_day)
        .filter(|e| e.start.date_naive() == day_date)
        .map(|e| {
            json!({
                "start": e.start.to_rfc3339(),
                "end": e.end.to_rfc3339(),
            })
        })
        .collect();
    map_gaps_to_blocks(crate::prepare::gaps::compute_gaps(
        &meeting_events,
        day_date,
        None, // Live events are already in UTC; gap computation uses NaiveDateTime within work hours
    ))
}

/// Fallback when live events are unavailable: use schedule startIso values only.
///
/// End times are approximated to 60 minutes from start because schedule artifacts
/// may not carry machine-parseable end timestamps.
pub fn available_blocks_from_schedule_start_iso(
    meetings: &[Meeting],
    day_date: NaiveDate,
) -> Vec<TimeBlock> {
    let meeting_events: Vec<serde_json::Value> = meetings
        .iter()
        .filter_map(|m| m.start_iso.as_deref())
        .filter_map(|start_iso| DateTime::parse_from_rfc3339(start_iso).ok())
        .map(|start| {
            let start_utc = start.with_timezone(&Utc);
            let end_utc = start_utc + Duration::minutes(60);
            json!({
                "start": start_utc.to_rfc3339(),
                "end": end_utc.to_rfc3339(),
            })
        })
        .collect();
    map_gaps_to_blocks(crate::prepare::gaps::compute_gaps(
        &meeting_events,
        day_date,
        None, // Schedule fallback uses startIso which is already UTC-converted
    ))
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;
    use crate::types::{Meeting, MeetingType};

    #[test]
    fn test_available_blocks_from_live_uses_calendar_events() {
        let day = chrono::NaiveDate::from_ymd_opt(2026, 2, 12).expect("valid date");
        let events = vec![CalendarEvent {
            id: "evt-1".to_string(),
            title: "Sync".to_string(),
            start: Utc.with_ymd_and_hms(2026, 2, 12, 15, 0, 0).unwrap(),
            end: Utc.with_ymd_and_hms(2026, 2, 12, 16, 0, 0).unwrap(),
            meeting_type: MeetingType::Internal,
            account: None,
            attendees: vec![],
            is_all_day: false,
            linked_entities: None,
        }];
        let blocks = available_blocks_from_live(&events, day);
        assert!(!blocks.is_empty());
    }

    #[test]
    fn test_available_blocks_from_schedule_fallback_uses_start_iso() {
        let day = chrono::NaiveDate::from_ymd_opt(2026, 2, 12).expect("valid date");
        let meetings = vec![Meeting {
            id: "m-1".to_string(),
            calendar_event_id: None,
            time: "9:00 AM".to_string(),
            end_time: Some("10:00 AM".to_string()),
            start_iso: Some("2026-02-12T09:00:00Z".to_string()),
            title: "Fallback".to_string(),
            meeting_type: MeetingType::Internal,
            prep: None,
            is_current: None,
            prep_file: None,
            has_prep: false,
            overlay_status: None,
            prep_reviewed: None,
            linked_entities: None,
            suggested_unarchive_account_id: None,
            intelligence_quality: None,
            calendar_attendees: None,
            calendar_description: None,
        }];
        let blocks = available_blocks_from_schedule_start_iso(&meetings, day);
        assert!(!blocks.is_empty());
    }

    #[test]
    fn test_available_blocks_update_when_midday_event_added() {
        let day = chrono::NaiveDate::from_ymd_opt(2026, 2, 12).expect("valid date");
        let mut events = vec![CalendarEvent {
            id: "evt-am".to_string(),
            title: "Morning Sync".to_string(),
            start: Utc.with_ymd_and_hms(2026, 2, 12, 10, 0, 0).unwrap(),
            end: Utc.with_ymd_and_hms(2026, 2, 12, 11, 0, 0).unwrap(),
            meeting_type: MeetingType::Internal,
            account: None,
            attendees: vec![],
            is_all_day: false,
            linked_entities: None,
        }];

        let before = available_blocks_from_live(&events, day);
        let before_total: u32 = before.iter().map(|b| b.duration_minutes).sum();

        events.push(CalendarEvent {
            id: "evt-midday".to_string(),
            title: "Midday Insert".to_string(),
            start: Utc.with_ymd_and_hms(2026, 2, 12, 12, 0, 0).unwrap(),
            end: Utc.with_ymd_and_hms(2026, 2, 12, 13, 0, 0).unwrap(),
            meeting_type: MeetingType::Internal,
            account: None,
            attendees: vec![],
            is_all_day: false,
            linked_entities: None,
        });
        let after = available_blocks_from_live(&events, day);
        let after_total: u32 = after.iter().map(|b| b.duration_minutes).sum();

        assert!(after_total < before_total);
    }
}
