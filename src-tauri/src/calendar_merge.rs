//! Calendar hybrid overlay merge (ADR-0032)
//!
//! Merges briefing meetings (from schedule.json) with live calendar events
//! (from Google Calendar polling). Live calendar is source of truth for
//! *which meetings exist*; briefing enrichment is overlaid by matching
//! on `calendarEventId`.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use chrono_tz::Tz;

use crate::types::{CalendarEvent, Meeting, MeetingType, OverlayStatus};

/// Format a UTC datetime to a display string like "9:00 AM" in the given timezone.
fn format_time_display(dt: DateTime<Utc>, tz: &Tz) -> String {
    dt.with_timezone(tz).format("%-I:%M %p").to_string()
}

/// Merge briefing meetings with live calendar events.
///
/// Returns a unified `Vec<Meeting>` with `overlay_status` set on each entry.
pub fn merge_meetings(briefing: Vec<Meeting>, live: &[CalendarEvent], tz: &Tz) -> Vec<Meeting> {
    // If no live data, return briefing as-is with BriefingOnly status
    if live.is_empty() {
        return briefing
            .into_iter()
            .map(|mut m| {
                m.overlay_status = Some(OverlayStatus::BriefingOnly);
                m
            })
            .collect();
    }

    // Index briefing meetings by calendar_event_id
    let mut briefing_by_id: HashMap<String, Meeting> = HashMap::new();
    let mut briefing_no_id: Vec<Meeting> = Vec::new();

    for m in briefing {
        if let Some(ref eid) = m.calendar_event_id {
            briefing_by_id.insert(eid.clone(), m);
        } else {
            briefing_no_id.push(m);
        }
    }

    let mut result: Vec<Meeting> = Vec::new();

    // Today's date in the given timezone — only merge events for today.
    // live events now span ±7 days (I386); without this gate every future
    // meeting would appear on the daily briefing as OverlayStatus::New.
    let today_date = chrono::Utc::now().with_timezone(tz).date_naive();

    // Pass 1: Walk live events
    for event in live {
        // Skip all-day and personal events
        if event.is_all_day || matches!(event.meeting_type, MeetingType::Personal) {
            continue;
        }

        // Skip events that aren't today in the user's timezone
        if event.start.with_timezone(tz).date_naive() != today_date {
            continue;
        }

        if let Some(mut briefing_meeting) = briefing_by_id.remove(&event.id) {
            // Enriched: live timing + briefing enrichment
            briefing_meeting.time = format_time_display(event.start, tz);
            briefing_meeting.end_time = Some(format_time_display(event.end, tz));
            briefing_meeting.start_iso = Some(event.start.to_rfc3339());
            briefing_meeting.title = event.title.clone();
            briefing_meeting.overlay_status = Some(OverlayStatus::Enriched);
            result.push(briefing_meeting);
        } else {
            // New: in live but not in briefing
            result.push(Meeting {
                id: event.id.clone(),
                calendar_event_id: Some(event.id.clone()),
                time: format_time_display(event.start, tz),
                end_time: Some(format_time_display(event.end, tz)),
                start_iso: Some(event.start.to_rfc3339()),
                title: event.title.clone(),
                meeting_type: event.meeting_type.clone(),
                prep: None,
                is_current: None,
                prep_file: None,
                has_prep: false,
                overlay_status: Some(OverlayStatus::New),
                prep_reviewed: None,
                linked_entities: None,
                suggested_unarchive_account_id: None,
                intelligence_quality: None,
                calendar_attendees: None,
                calendar_description: None,
            });
        }
    }

    // Pass 2: Unmatched briefing meetings with calendar_event_id → Cancelled
    for (_, mut m) in briefing_by_id {
        m.overlay_status = Some(OverlayStatus::Cancelled);
        result.push(m);
    }

    // Pass 2b: Briefing meetings without calendar_event_id → BriefingOnly
    for mut m in briefing_no_id {
        m.overlay_status = Some(OverlayStatus::BriefingOnly);
        result.push(m);
    }

    // Sort by time string (lexicographic on "H:MM AM/PM" works for display order)
    result.sort_by(|a, b| sort_time_key(&a.time).cmp(&sort_time_key(&b.time)));

    result
}

/// Convert a display time like "9:00 AM" to a sortable 24h minute value.
fn sort_time_key(time: &str) -> u32 {
    let time = time.trim();
    let (time_part, period) = if let Some(pos) = time.find(['A', 'P']) {
        (&time[..pos].trim(), &time[pos..])
    } else {
        return 9999; // Unparseable → sort to end
    };

    let parts: Vec<&str> = time_part.split(':').collect();
    if parts.len() != 2 {
        return 9999;
    }

    let hours: u32 = parts[0].parse().unwrap_or(0);
    let minutes: u32 = parts[1].parse().unwrap_or(0);

    let h24 = if period.starts_with('P') && hours != 12 {
        hours + 12
    } else if period.starts_with('A') && hours == 12 {
        0
    } else {
        hours
    };

    h24 * 60 + minutes
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn make_tz() -> Tz {
        "America/New_York".parse().unwrap()
    }

    fn make_live_event(id: &str, title: &str, start_hour: u32, end_hour: u32) -> CalendarEvent {
        // Build events using today in the test timezone (America/New_York) so they
        // always pass the today-filter in merge_meetings regardless of UTC wall-clock.
        let tz = make_tz();
        let today_local = Utc::now().with_timezone(&tz).date_naive();
        let start = tz
            .from_local_datetime(&today_local.and_hms_opt(start_hour, 0, 0).unwrap())
            .unwrap()
            .to_utc();
        let end = tz
            .from_local_datetime(&today_local.and_hms_opt(end_hour, 0, 0).unwrap())
            .unwrap()
            .to_utc();
        CalendarEvent {
            id: id.to_string(),
            title: title.to_string(),
            start,
            end,
            meeting_type: MeetingType::Customer,
            account: Some("Acme".to_string()),
            attendees: vec![],
            is_all_day: false,
            linked_entities: None,
        }
    }

    fn make_briefing_meeting(id: &str, event_id: Option<&str>, title: &str) -> Meeting {
        Meeting {
            id: id.to_string(),
            calendar_event_id: event_id.map(|s| s.to_string()),
            time: "9:00 AM".to_string(),
            end_time: Some("10:00 AM".to_string()),
            start_iso: None,
            title: title.to_string(),
            meeting_type: MeetingType::Customer,
            prep: None,
            is_current: None,
            prep_file: Some("prep-acme".to_string()),
            has_prep: true,
            overlay_status: None,
            prep_reviewed: None,
            linked_entities: None,
            suggested_unarchive_account_id: None,
            intelligence_quality: None,
            calendar_attendees: None,
            calendar_description: None,
        }
    }

    #[test]
    fn test_empty_live_returns_briefing_only() {
        let briefing = vec![make_briefing_meeting("1", Some("evt1"), "Acme Sync")];
        let result = merge_meetings(briefing, &[], &make_tz());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].overlay_status, Some(OverlayStatus::BriefingOnly));
    }

    #[test]
    fn test_enriched_meeting() {
        let briefing = vec![make_briefing_meeting("1", Some("evt1"), "Acme Sync")];
        let live = vec![make_live_event("evt1", "Acme Weekly Sync", 14, 15)];
        let result = merge_meetings(briefing, &live, &make_tz());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].overlay_status, Some(OverlayStatus::Enriched));
        // Title updated from live
        assert_eq!(result[0].title, "Acme Weekly Sync");
        // Prep preserved from briefing
        assert!(result[0].has_prep);
        assert_eq!(result[0].prep_file, Some("prep-acme".to_string()));
    }

    #[test]
    fn test_cancelled_meeting() {
        let briefing = vec![make_briefing_meeting("1", Some("evt1"), "Acme Sync")];
        // Live has no events matching evt1
        let live = vec![make_live_event("evt2", "Other Meeting", 14, 15)];
        let result = merge_meetings(briefing, &live, &make_tz());
        assert_eq!(result.len(), 2);
        let cancelled = result
            .iter()
            .find(|m| m.calendar_event_id == Some("evt1".to_string()))
            .unwrap();
        assert_eq!(cancelled.overlay_status, Some(OverlayStatus::Cancelled));
    }

    #[test]
    fn test_new_meeting() {
        let briefing = vec![];
        let live = vec![make_live_event("evt1", "Surprise Meeting", 14, 15)];
        let result = merge_meetings(briefing, &live, &make_tz());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].overlay_status, Some(OverlayStatus::New));
        assert!(!result[0].has_prep);
    }

    #[test]
    fn test_briefing_no_event_id_passes_through() {
        let briefing = vec![make_briefing_meeting("1", None, "Manual Meeting")];
        let live = vec![make_live_event("evt1", "Live Meeting", 14, 15)];
        let result = merge_meetings(briefing, &live, &make_tz());
        let manual = result.iter().find(|m| m.title == "Manual Meeting").unwrap();
        assert_eq!(manual.overlay_status, Some(OverlayStatus::BriefingOnly));
    }

    #[test]
    fn test_all_day_events_skipped() {
        let briefing = vec![];
        let mut event = make_live_event("evt1", "All Day", 0, 23);
        event.is_all_day = true;
        let result = merge_meetings(briefing, &[event], &make_tz());
        assert!(result.is_empty());
    }

    #[test]
    fn test_sorted_by_time() {
        let briefing = vec![];
        let live = vec![
            make_live_event("evt2", "Afternoon", 19, 20), // 2 PM ET
            make_live_event("evt1", "Morning", 14, 15),   // 9 AM ET
        ];
        let result = merge_meetings(briefing, &live, &make_tz());
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].title, "Morning");
        assert_eq!(result[1].title, "Afternoon");
    }

    #[test]
    fn test_sort_time_key() {
        assert!(sort_time_key("9:00 AM") < sort_time_key("10:00 AM"));
        assert!(sort_time_key("12:00 PM") < sort_time_key("1:00 PM"));
        assert!(sort_time_key("11:59 AM") < sort_time_key("12:00 PM"));
    }
}
