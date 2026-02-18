//! Granola → DailyOS meeting matching.
//!
//! Primary: match Google Calendar event ID from Granola document to
//! meetings_history.id (both originate from Google Calendar).
//! Fallback: reuse the Quill matcher's title + time correlation algorithm.

use chrono::{DateTime, Utc};

use super::cache::GranolaDocument;

/// Result of matching a Granola document to a DailyOS meeting.
#[derive(Debug, Clone)]
pub struct GranolaMatchResult {
    pub meeting_id: String,
    pub method: MatchMethod,
}

/// How the match was made.
#[derive(Debug, Clone, PartialEq)]
pub enum MatchMethod {
    /// Exact Google Calendar event ID match.
    CalendarId,
    /// Fuzzy title + time window match (fallback).
    TitleTime,
}

/// Attempt to match a Granola document to a meeting_id in the database.
///
/// Strategy:
/// 1. If the Granola document has a `google_calendar_event.id`, look it up
///    directly in meetings_history (the `id` column stores the calendar event ID).
/// 2. Fallback: match by title + time proximity using the Quill matcher algorithm.
pub fn match_to_meeting(
    doc: &GranolaDocument,
    meeting_ids: &[(String, String, String)], // (id, title, start_time)
) -> Option<GranolaMatchResult> {
    // Strategy 1: Direct calendar event ID match
    if let Some(ref cal_event) = doc.google_calendar_event {
        if let Some(ref cal_id) = cal_event.id {
            if !cal_id.is_empty() {
                for (mid, _title, _start) in meeting_ids {
                    if mid == cal_id {
                        return Some(GranolaMatchResult {
                            meeting_id: mid.clone(),
                            method: MatchMethod::CalendarId,
                        });
                    }
                }
            }
        }
    }

    // Strategy 2: Title + time proximity fallback
    let doc_start = doc
        .google_calendar_event
        .as_ref()
        .and_then(|e| e.start.as_ref())
        .and_then(|s| s.date_time.as_deref())
        .and_then(|t| t.parse::<DateTime<Utc>>().ok());

    let doc_start = doc_start?;

    let mut best: Option<(GranolaMatchResult, u32)> = None;

    for (mid, title, start_time) in meeting_ids {
        let meeting_start = match start_time.parse::<DateTime<Utc>>() {
            Ok(t) => t,
            Err(_) => continue,
        };

        let t_score = title_score(&doc.title, title);
        let tm_score = time_proximity_score(&doc_start, &meeting_start);
        let total = t_score + tm_score;

        if total >= 100 && best.as_ref().is_none_or(|(_, s)| total > *s) {
            best = Some((
                GranolaMatchResult {
                    meeting_id: mid.clone(),
                    method: MatchMethod::TitleTime,
                },
                total,
            ));
        }
    }

    best.map(|(result, _)| result)
}

/// Score title similarity (same algorithm as quill::matcher).
fn title_score(a: &str, b: &str) -> u32 {
    let a_lower = a.to_lowercase();
    let b_lower = b.to_lowercase();

    if a_lower == b_lower {
        return 100;
    }
    if a_lower.contains(&b_lower) || b_lower.contains(&a_lower) {
        return 70;
    }

    // Token overlap
    let tokens_a: std::collections::HashSet<&str> = a_lower.split_whitespace().collect();
    let tokens_b: std::collections::HashSet<&str> = b_lower.split_whitespace().collect();
    let union = tokens_a.union(&tokens_b).count();
    if union > 0 {
        let intersection = tokens_a.intersection(&tokens_b).count();
        let overlap = intersection as f64 / union as f64;
        if overlap > 0.5 {
            return 50;
        }
    }

    0
}

/// Score time proximity between two timestamps.
fn time_proximity_score(a: &DateTime<Utc>, b: &DateTime<Utc>) -> u32 {
    let diff_minutes = (*a - *b).num_minutes().unsigned_abs();
    if diff_minutes <= 5 {
        80
    } else if diff_minutes <= 15 {
        60
    } else if diff_minutes <= 30 {
        30
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::granola::cache::{EventTime, GoogleCalendarEvent};

    fn make_doc(id: &str, title: &str, cal_id: Option<&str>, start: Option<&str>) -> GranolaDocument {
        GranolaDocument {
            id: id.to_string(),
            title: title.to_string(),
            created_at: None,
            updated_at: None,
            content: "Test content".to_string(),
            google_calendar_event: Some(GoogleCalendarEvent {
                id: cal_id.map(String::from),
                summary: Some(title.to_string()),
                start: start.map(|s| EventTime {
                    date_time: Some(s.to_string()),
                }),
                end: None,
                status: None,
                attendees: vec![],
            }),
            attendee_emails: vec![],
        }
    }

    #[test]
    fn test_calendar_id_match() {
        let doc = make_doc("g1", "Weekly Sync", Some("cal-123"), Some("2026-02-17T14:00:00Z"));
        let meetings = vec![
            ("cal-123".to_string(), "Weekly Sync".to_string(), "2026-02-17T14:00:00Z".to_string()),
        ];

        let result = match_to_meeting(&doc, &meetings);
        assert!(result.is_some());
        let m = result.unwrap();
        assert_eq!(m.meeting_id, "cal-123");
        assert_eq!(m.method, MatchMethod::CalendarId);
    }

    #[test]
    fn test_title_time_fallback() {
        let doc = make_doc("g1", "Weekly Sync", Some("cal-999"), Some("2026-02-17T14:00:00Z"));
        let meetings = vec![
            ("meeting-1".to_string(), "Weekly Sync".to_string(), "2026-02-17T14:00:00Z".to_string()),
        ];

        let result = match_to_meeting(&doc, &meetings);
        assert!(result.is_some());
        let m = result.unwrap();
        assert_eq!(m.meeting_id, "meeting-1");
        assert_eq!(m.method, MatchMethod::TitleTime);
    }

    #[test]
    fn test_no_match() {
        let doc = make_doc("g1", "Completely Different", Some("cal-999"), Some("2026-02-17T14:00:00Z"));
        let meetings = vec![
            ("meeting-1".to_string(), "Unrelated Meeting".to_string(), "2026-02-17T20:00:00Z".to_string()),
        ];

        let result = match_to_meeting(&doc, &meetings);
        assert!(result.is_none());
    }

    #[test]
    fn test_calendar_id_preferred_over_title() {
        let doc = make_doc("g1", "Wrong Title", Some("cal-123"), Some("2026-02-17T14:00:00Z"));
        let meetings = vec![
            ("cal-123".to_string(), "Correct Title".to_string(), "2026-02-17T14:00:00Z".to_string()),
            ("cal-456".to_string(), "Wrong Title".to_string(), "2026-02-17T14:00:00Z".to_string()),
        ];

        let result = match_to_meeting(&doc, &meetings);
        assert!(result.is_some());
        let m = result.unwrap();
        assert_eq!(m.meeting_id, "cal-123");
        assert_eq!(m.method, MatchMethod::CalendarId);
    }
}
