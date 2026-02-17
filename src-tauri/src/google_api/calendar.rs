//! Google Calendar API v3 — event fetching.
//!
//! Replaces calendar_poll.py and ops/calendar_fetch.py.

use chrono::{DateTime, NaiveDate, Offset, Utc};
use serde::Deserialize;

use super::{send_with_retry, GoogleApiError, RetryPolicy};

// ============================================================================
// API response types (deserialized from Google Calendar JSON)
// ============================================================================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CalendarListResponse {
    #[serde(default)]
    items: Vec<GoogleEventRaw>,
    next_page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GoogleEventRaw {
    #[serde(default)]
    id: String,
    #[serde(default)]
    summary: Option<String>,
    start: Option<EventDateTime>,
    end: Option<EventDateTime>,
    #[serde(default)]
    attendees: Vec<Attendee>,
    organizer: Option<Organizer>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    location: Option<String>,
    #[serde(default)]
    recurring_event_id: Option<String>,
    #[serde(default)]
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EventDateTime {
    date_time: Option<String>,
    date: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Attendee {
    #[serde(default)]
    email: String,
    #[serde(default)]
    response_status: Option<String>,
    #[serde(default)]
    resource: Option<bool>,
    #[serde(rename = "self", default)]
    is_self: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Organizer {
    #[serde(default)]
    email: String,
}

// ============================================================================
// Public types
// ============================================================================

/// A normalized calendar event from Google Calendar.
#[derive(Debug, Clone, serde::Serialize)]
pub struct GoogleCalendarEvent {
    pub id: String,
    pub summary: String,
    pub start: String,
    pub end: String,
    pub attendees: Vec<String>,
    /// Per-attendee RSVP status from Google Calendar (accepted/tentative/declined/needsAction).
    /// Key is lowercase email.
    #[serde(default)]
    pub attendee_rsvp: std::collections::HashMap<String, String>,
    pub organizer: String,
    pub description: String,
    pub location: String,
    pub is_recurring: bool,
    pub is_all_day: bool,
}

// ============================================================================
// Calendar API
// ============================================================================

/// Fetch events from Google Calendar for a date range.
///
/// Handles pagination (maxResults=250, pageToken). Filters out cancelled
/// events, declined events, and resource rooms.
pub async fn fetch_events(
    access_token: &str,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<Vec<GoogleCalendarEvent>, GoogleApiError> {
    let client = reqwest::Client::new();

    // Use local-midnight bounds, not UTC midnight.
    // On Sunday 8pm EST, UTC midnight is already Monday — using Z would fetch wrong day.
    let local_offset = chrono::Local::now().offset().fix();
    let offset_secs = local_offset.local_minus_utc();
    let offset_str = format!(
        "{:+03}:{:02}",
        offset_secs / 3600,
        (offset_secs.unsigned_abs() % 3600) / 60
    );
    let time_min = format!("{}T00:00:00{}", start_date, offset_str);
    let time_max = format!(
        "{}T00:00:00{}",
        end_date + chrono::Duration::days(1),
        offset_str
    );

    let mut all_events = Vec::new();
    let mut page_token: Option<String> = None;

    loop {
        let mut request = client
            .get("https://www.googleapis.com/calendar/v3/calendars/primary/events")
            .bearer_auth(access_token)
            .query(&[
                ("timeMin", time_min.as_str()),
                ("timeMax", time_max.as_str()),
                ("singleEvents", "true"),
                ("orderBy", "startTime"),
                ("maxResults", "250"),
            ]);

        if let Some(ref token) = page_token {
            request = request.query(&[("pageToken", token.as_str())]);
        }

        let resp = send_with_retry(request, &RetryPolicy::default()).await?;

        let status = resp.status();
        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(GoogleApiError::AuthExpired);
        }
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(GoogleApiError::ApiError {
                status: status.as_u16(),
                message: body,
            });
        }

        let body: CalendarListResponse = resp.json().await?;

        for item in body.items {
            // Skip cancelled events
            if item.status.as_deref() == Some("cancelled") {
                continue;
            }

            // Skip declined events (self declined)
            let self_declined = item.attendees.iter().any(|a| {
                a.is_self == Some(true) && a.response_status.as_deref() == Some("declined")
            });
            if self_declined {
                continue;
            }

            // Filter out resource rooms from attendee list
            let attendees: Vec<String> = item
                .attendees
                .iter()
                .filter(|a| a.resource != Some(true))
                .map(|a| a.email.clone())
                .filter(|e| !e.is_empty())
                .collect();

            let start_str = item
                .start
                .as_ref()
                .and_then(|s| s.date_time.as_deref().or(s.date.as_deref()))
                .unwrap_or("")
                .to_string();
            let end_str = item
                .end
                .as_ref()
                .and_then(|s| s.date_time.as_deref().or(s.date.as_deref()))
                .unwrap_or("")
                .to_string();

            let is_all_day = item
                .start
                .as_ref()
                .map(|s| s.date_time.is_none() && s.date.is_some())
                .unwrap_or(false);

            all_events.push(GoogleCalendarEvent {
                id: item.id,
                summary: item.summary.unwrap_or_else(|| "(No title)".to_string()),
                start: start_str,
                end: end_str,
                attendees,
                organizer: item.organizer.map(|o| o.email).unwrap_or_default(),
                description: item.description.unwrap_or_default(),
                location: item.location.unwrap_or_default(),
                is_recurring: item.recurring_event_id.is_some(),
                is_all_day,
            });
        }

        page_token = body.next_page_token;
        if page_token.is_none() {
            break;
        }
    }

    Ok(all_events)
}

/// Fetch the owner's domain from the primary calendar metadata.
pub async fn get_owner_domain(access_token: &str) -> Result<Option<String>, GoogleApiError> {
    let client = reqwest::Client::new();
    let resp = send_with_retry(
        client
            .get("https://www.googleapis.com/calendar/v3/calendars/primary")
            .bearer_auth(access_token),
        &RetryPolicy::default(),
    )
    .await?;

    if !resp.status().is_success() {
        return Ok(None);
    }

    let body: serde_json::Value = resp.json().await?;
    let id = body["id"].as_str().unwrap_or("");

    if id.contains('@') {
        Ok(Some(id.split('@').nth(1).unwrap_or("").to_string()))
    } else {
        Ok(None)
    }
}

/// Parse an ISO datetime string to a chrono DateTime<Utc>.
pub fn parse_event_datetime(s: &str) -> Option<DateTime<Utc>> {
    if s.is_empty() {
        return None;
    }
    if s.contains('T') {
        // Full datetime
        DateTime::parse_from_rfc3339(&s.replace('Z', "+00:00"))
            .or_else(|_| DateTime::parse_from_rfc3339(s))
            .ok()
            .map(|dt| dt.with_timezone(&Utc))
    } else {
        // Date-only (all-day event) — treat as midnight UTC
        chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .ok()
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_event_datetime_rfc3339() {
        let dt = parse_event_datetime("2026-02-08T09:00:00-05:00").unwrap();
        assert_eq!(dt.hour(), 14); // 9 AM EST = 14:00 UTC
    }

    #[test]
    fn test_parse_event_datetime_z_suffix() {
        let dt = parse_event_datetime("2026-02-08T14:00:00Z").unwrap();
        assert_eq!(dt.hour(), 14);
    }

    #[test]
    fn test_parse_event_datetime_date_only() {
        let dt = parse_event_datetime("2026-02-08").unwrap();
        assert_eq!(dt.hour(), 0);
        assert_eq!(
            dt.date_naive(),
            chrono::NaiveDate::from_ymd_opt(2026, 2, 8).unwrap()
        );
    }

    #[test]
    fn test_parse_event_datetime_empty() {
        assert!(parse_event_datetime("").is_none());
    }

    #[test]
    fn test_google_event_deserialization() {
        let json = r#"{
            "items": [
                {
                    "id": "event123",
                    "summary": "Team Standup",
                    "start": {"dateTime": "2026-02-08T09:00:00-05:00"},
                    "end": {"dateTime": "2026-02-08T09:30:00-05:00"},
                    "attendees": [
                        {"email": "alice@company.com", "responseStatus": "accepted"},
                        {"email": "bob@company.com", "responseStatus": "accepted"},
                        {"email": "room@resource.calendar.google.com", "resource": true}
                    ],
                    "organizer": {"email": "alice@company.com"},
                    "recurringEventId": "abc123"
                }
            ]
        }"#;

        let resp: CalendarListResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.items.len(), 1);
        assert_eq!(resp.items[0].summary.as_deref(), Some("Team Standup"));
        assert_eq!(resp.items[0].attendees.len(), 3);
        assert!(resp.items[0].recurring_event_id.is_some());
    }

    #[test]
    fn test_all_day_event_detection() {
        let json = r#"{
            "items": [{
                "id": "allday1",
                "summary": "Holiday",
                "start": {"date": "2026-02-08"},
                "end": {"date": "2026-02-09"},
                "attendees": [],
                "status": "confirmed"
            }]
        }"#;

        let resp: CalendarListResponse = serde_json::from_str(json).unwrap();
        let item = &resp.items[0];
        let is_all_day = item
            .start
            .as_ref()
            .map(|s| s.date_time.is_none() && s.date.is_some())
            .unwrap_or(false);
        assert!(is_all_day);
    }

    #[test]
    fn test_declined_event_detection() {
        let json = r#"{
            "items": [{
                "id": "declined1",
                "summary": "Meeting I declined",
                "start": {"dateTime": "2026-02-08T10:00:00Z"},
                "end": {"dateTime": "2026-02-08T11:00:00Z"},
                "attendees": [
                    {"email": "me@company.com", "self": true, "responseStatus": "declined"},
                    {"email": "other@company.com", "responseStatus": "accepted"}
                ]
            }]
        }"#;

        let resp: CalendarListResponse = serde_json::from_str(json).unwrap();
        let self_declined = resp.items[0]
            .attendees
            .iter()
            .any(|a| a.is_self == Some(true) && a.response_status.as_deref() == Some("declined"));
        assert!(self_declined);
    }

    use chrono::Timelike;
}
