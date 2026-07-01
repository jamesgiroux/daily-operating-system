//! Granola MCP client for OAuth-backed transcript sync.
//!
//! The client adapts Granola's MCP tools into the existing `GranolaDocument`
//! shape consumed by the matcher and transcript pipeline.

use std::time::Duration;

use chrono::{FixedOffset, NaiveDateTime, SecondsFormat, TimeZone, Utc};
use regex::Regex;
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::Mutex;

use super::cache::{
    EventTime, GoogleCalendarEvent, GranolaAttendee, GranolaContentType, GranolaDocument,
};

const GRANOLA_CALL_TIMEOUT: Duration = Duration::from_secs(30);
const MCP_PROTOCOL_VERSION: &str = "2025-06-18";
const MAX_MEETINGS_PER_SCAN: usize = 500;

#[derive(Debug, Clone)]
pub struct GranolaAccountInfo {
    pub email: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GranolaMeeting {
    pub id: String,
    pub title: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub google_calendar_event: Option<GoogleCalendarEvent>,
    pub attendee_emails: Vec<String>,
}

impl GranolaMeeting {
    pub fn as_match_document(&self) -> GranolaDocument {
        GranolaDocument {
            id: self.id.clone(),
            title: self.title.clone(),
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
            content: String::new(),
            content_type: GranolaContentType::Notes,
            google_calendar_event: self.google_calendar_event.clone(),
            attendee_emails: self.attendee_emails.clone(),
        }
    }

    fn into_document(self, content: String, content_type: GranolaContentType) -> GranolaDocument {
        GranolaDocument {
            id: self.id,
            title: self.title,
            created_at: self.created_at,
            updated_at: self.updated_at,
            content,
            content_type,
            google_calendar_event: self.google_calendar_event,
            attendee_emails: self.attendee_emails,
        }
    }
}

pub struct GranolaMcpClient {
    endpoint: String,
    client: reqwest::Client,
    initialized: Mutex<bool>,
}

impl GranolaMcpClient {
    pub fn new(endpoint: &str) -> Self {
        let endpoint = if endpoint.trim().is_empty() {
            crate::granola_oauth::DEFAULT_GRANOLA_MCP_ENDPOINT
        } else {
            endpoint.trim()
        };

        let client = reqwest::Client::builder()
            .timeout(GRANOLA_CALL_TIMEOUT)
            .default_headers({
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert(
                    reqwest::header::CONTENT_TYPE,
                    reqwest::header::HeaderValue::from_static("application/json"),
                );
                headers.insert(
                    reqwest::header::ACCEPT,
                    reqwest::header::HeaderValue::from_static(
                        "application/json, text/event-stream",
                    ),
                );
                headers.insert(
                    reqwest::header::HeaderName::from_static("mcp-protocol-version"),
                    reqwest::header::HeaderValue::from_static(MCP_PROTOCOL_VERSION),
                );
                headers
            })
            .build()
            .unwrap_or_default();

        Self {
            endpoint: endpoint.to_string(),
            client,
            initialized: Mutex::new(false),
        }
    }

    pub async fn list_meetings(&self, days_back: i32) -> Result<Vec<GranolaMeeting>, String> {
        let end = Utc::now();
        let start = end - chrono::Duration::days(days_back.max(1) as i64);
        let payload = self
            .call_tool(
                "list_meetings",
                serde_json::json!({
                    "custom_start": start.to_rfc3339_opts(SecondsFormat::Secs, true),
                    "custom_end": end.to_rfc3339_opts(SecondsFormat::Secs, true),
                }),
            )
            .await?;
        // Granola's list_meetings returns an XML-ish `<meetings_data>` text blob,
        // not JSON — parse that shape first, falling back to JSON for safety.
        let mut meetings = if let Some(text) = payload.as_str() {
            parse_meetings_xml(text)
        } else {
            meetings_from_value(&payload)
        };
        meetings.truncate(MAX_MEETINGS_PER_SCAN);
        Ok(meetings)
    }

    pub async fn get_meeting_transcript(&self, meeting_id: &str) -> Result<Option<String>, String> {
        let payload = self
            .call_tool(
                "get_meeting_transcript",
                serde_json::json!({ "meeting_id": meeting_id }),
            )
            .await?;
        Ok(extract_transcript_text(&payload))
    }

    pub async fn get_meetings(&self, meeting_ids: &[String]) -> Result<Vec<Value>, String> {
        if meeting_ids.is_empty() {
            return Ok(Vec::new());
        }
        let payload = self
            .call_tool(
                "get_meetings",
                serde_json::json!({ "meeting_ids": meeting_ids }),
            )
            .await?;
        Ok(values_from_payload(&payload))
    }

    pub async fn get_account_info(&self) -> Result<GranolaAccountInfo, String> {
        let payload = self
            .call_tool("get_account_info", serde_json::json!({}))
            .await?;
        Ok(GranolaAccountInfo {
            email: find_first_string_by_key(&payload, &["email", "email_address", "emailAddress"]),
            name: find_first_string_by_key(&payload, &["name", "display_name", "displayName"]),
        })
    }

    pub async fn fetch_document(
        &self,
        meeting: &GranolaMeeting,
    ) -> Result<GranolaDocument, String> {
        if let Some(transcript) = self.get_meeting_transcript(&meeting.id).await? {
            if !transcript.trim().is_empty() {
                return Ok(meeting
                    .clone()
                    .into_document(transcript, GranolaContentType::Transcript));
            }
        }

        let details = self.get_meetings(std::slice::from_ref(&meeting.id)).await?;
        let notes = details.iter().find_map(extract_notes_text).ok_or_else(|| {
            "Granola meeting has no transcript, notes, or summary content".to_string()
        })?;
        Ok(meeting
            .clone()
            .into_document(notes, GranolaContentType::Notes))
    }

    pub async fn list_recent_documents(
        &self,
        days_back: i32,
    ) -> Result<Vec<GranolaDocument>, String> {
        let meetings = self.list_meetings(days_back).await?;
        let mut documents = Vec::new();
        for meeting in &meetings {
            match self.fetch_document(meeting).await {
                Ok(document) => documents.push(document),
                Err(error) => {
                    log::warn!(
                        "Granola MCP: failed to fetch meeting content: meeting_ref={}, error_ref={}",
                        crate::processor::transcript::digest_token(&meeting.title),
                        crate::processor::transcript::digest_token(&error)
                    );
                }
            }
        }
        Ok(documents)
    }

    async fn ensure_initialized(&self) -> Result<(), String> {
        let mut initialized = self.initialized.lock().await;
        if *initialized {
            return Ok(());
        }

        let initialize = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": MCP_PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {
                    "name": "DailyOS",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }
        });
        self.send_json_rpc(initialize).await?;

        let initialized_notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        });
        self.send_notification(initialized_notification).await?;

        *initialized = true;
        Ok(())
    }

    async fn call_tool(&self, name: &str, arguments: Value) -> Result<Value, String> {
        self.ensure_initialized().await?;
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": name,
                "arguments": arguments
            }
        });
        let response = self.send_json_rpc(body).await?;
        tool_payload_from_response(&response)
    }

    async fn send_json_rpc(&self, body: Value) -> Result<Value, String> {
        let token = crate::granola_oauth::get_valid_access_token()
            .await
            .map_err(|e| format!("Granola token error: {e}"))?;
        let response = self
            .client
            .post(&self.endpoint)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    format!("Granola MCP request timed out: {e}")
                } else {
                    format!("Granola MCP request failed: {e}")
                }
            })?;

        let status = response.status();
        let body_text = response.text().await.unwrap_or_default();
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            return Err(format!("Granola MCP returned {status}"));
        }
        if !status.is_success() {
            return Err(format!(
                "Granola MCP returned {status}: {}",
                truncate_for_log(&body_text)
            ));
        }

        let json_text = extract_json_from_response(&body_text);
        serde_json::from_str(&json_text).map_err(|e| {
            format!(
                "Failed to parse Granola MCP response: {} (body starts with: {})",
                e,
                truncate_for_log(&json_text)
            )
        })
    }

    async fn send_notification(&self, body: Value) -> Result<(), String> {
        let token = crate::granola_oauth::get_valid_access_token()
            .await
            .map_err(|e| format!("Granola token error: {e}"))?;
        let response = self
            .client
            .post(&self.endpoint)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Granola MCP initialized notification failed: {e}"))?;

        let status = response.status();
        if status.is_success() {
            Ok(())
        } else {
            let body_text = response.text().await.unwrap_or_default();
            Err(format!(
                "Granola MCP initialized notification returned {status}: {}",
                truncate_for_log(&body_text)
            ))
        }
    }
}

fn extract_json_from_response(body: &str) -> String {
    let trimmed = body.trim();
    if trimmed.starts_with('{') {
        return trimmed.to_string();
    }

    for line in trimmed.lines() {
        let line = line.trim();
        if let Some(data) = line.strip_prefix("data:") {
            let data = data.trim();
            if !data.is_empty() && data != "[DONE]" && data.starts_with('{') {
                return data.to_string();
            }
        }
    }

    trimmed.to_string()
}

fn tool_payload_from_response(response: &Value) -> Result<Value, String> {
    if let Some(error) = response.get("error") {
        return Err(format!("Granola MCP tool error: {error}"));
    }

    let result = response
        .get("result")
        .ok_or_else(|| "Granola MCP response missing result".to_string())?;

    if let Some(structured) = result.get("structuredContent") {
        return Ok(structured.clone());
    }

    if let Some(content) = result.get("content").and_then(|c| c.as_array()) {
        let text = content
            .iter()
            .filter_map(|item| {
                if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                    item.get("text").and_then(|t| t.as_str())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("");

        if text.trim().is_empty() {
            return Ok(Value::Null);
        }

        return Ok(serde_json::from_str(&text).unwrap_or(Value::String(text)));
    }

    Ok(result.clone())
}

fn meetings_from_value(value: &Value) -> Vec<GranolaMeeting> {
    values_from_payload(value)
        .into_iter()
        .filter_map(|item| serde_json::from_value::<RawMeeting>(item).ok())
        .filter_map(RawMeeting::into_meeting)
        .collect()
}

/// Parse Granola's `list_meetings` XML text shape into meetings.
///
/// Granola returns human-readable markup, e.g.:
/// ```text
/// <meetings_data from="Jun 1, 2026" to="Jun 30, 2026" count="43">
/// <meeting id="UUID" title="JG and Brant sync" date="Jun 30, 2026 2:30 PM EDT">
///   <known_participants><a@x.com><b@x.com></known_participants>
/// </meeting>
/// </meetings_data>
/// ```
/// Participant emails are encoded as child tag names inside `<known_participants>`.
fn parse_meetings_xml(text: &str) -> Vec<GranolaMeeting> {
    let block_re = match Regex::new(r"(?s)<meeting\b([^>]*)>(.*?)</meeting>") {
        Ok(re) => re,
        Err(_) => return Vec::new(),
    };
    let email_re = Regex::new(r"<([^<>/\s]+@[^<>/\s]+)>").ok();

    let mut meetings = Vec::new();
    for cap in block_re.captures_iter(text) {
        let attrs = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let body = cap.get(2).map(|m| m.as_str()).unwrap_or("");

        let Some(id) = xml_attr(attrs, "id").and_then(|v| non_empty(&v)) else {
            continue;
        };
        let title = xml_attr(attrs, "title")
            .and_then(|v| non_empty(&v))
            .unwrap_or_else(|| "Untitled meeting".to_string());
        let start_rfc3339 = xml_attr(attrs, "date").and_then(|d| parse_granola_date(&d));

        let attendee_emails: Vec<String> = email_re
            .as_ref()
            .map(|re| {
                re.captures_iter(body)
                    .filter_map(|c| c.get(1).map(|m| m.as_str().to_lowercase()))
                    .collect()
            })
            .unwrap_or_default();

        let google_calendar_event = build_google_calendar_event(
            &title,
            None,
            start_rfc3339.clone(),
            None,
            None,
            &attendee_emails,
        );

        meetings.push(GranolaMeeting {
            id,
            title,
            created_at: start_rfc3339,
            updated_at: None,
            google_calendar_event,
            attendee_emails,
        });
    }
    meetings
}

/// Extract a double-quoted XML attribute value by name from a tag's attribute string.
fn xml_attr(attrs: &str, name: &str) -> Option<String> {
    let re = Regex::new(&format!(r#"{}\s*=\s*"([^"]*)""#, regex::escape(name))).ok()?;
    re.captures(attrs)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

/// Parse Granola's human date, e.g. `Jun 30, 2026 2:30 PM EDT`, to an RFC 3339 UTC string.
fn parse_granola_date(value: &str) -> Option<String> {
    let value = value.trim();
    // Split a trailing alphabetic timezone abbreviation (EDT, PST, UTC, ...).
    let (datetime_part, tz_offset_hours) = match value.rsplit_once(' ') {
        Some((rest, tz)) if !tz.is_empty() && tz.chars().all(|c| c.is_ascii_alphabetic()) => {
            (rest.trim(), tz_offset_hours(tz))
        }
        _ => (value, 0),
    };

    let naive = NaiveDateTime::parse_from_str(datetime_part, "%b %d, %Y %I:%M %p").ok()?;
    let offset = FixedOffset::east_opt(tz_offset_hours * 3600)?;
    let local = offset.from_local_datetime(&naive).single()?;
    Some(local.with_timezone(&Utc).to_rfc3339())
}

/// Map common North American timezone abbreviations to a UTC offset (hours).
/// Unknown zones fall back to UTC; time-proximity matching tolerates small drift.
fn tz_offset_hours(tz: &str) -> i32 {
    match tz.to_ascii_uppercase().as_str() {
        "EDT" => -4,
        "EST" | "CDT" => -5,
        "CST" | "MDT" => -6,
        "MST" | "PDT" => -7,
        "PST" => -8,
        _ => 0,
    }
}

fn values_from_payload(value: &Value) -> Vec<Value> {
    if let Some(arr) = value.as_array() {
        return arr.clone();
    }

    for key in ["meetings", "notes", "documents", "results", "data"] {
        if let Some(arr) = value.get(key).and_then(|v| v.as_array()) {
            return arr.clone();
        }
    }

    if value.is_null() {
        Vec::new()
    } else {
        vec![value.clone()]
    }
}

fn extract_transcript_text(value: &Value) -> Option<String> {
    if let Some(text) = value.as_str() {
        return non_empty(text);
    }

    if let Some(transcript) = value.get("transcript") {
        if let Some(text) = extract_transcript_text(transcript) {
            return Some(text);
        }
    }

    if let Some(arr) = value.as_array() {
        let text = arr
            .iter()
            .filter_map(|item| {
                item.as_str()
                    .or_else(|| item.get("text").and_then(|text| text.as_str()))
            })
            .filter(|text| !text.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        return non_empty(&text);
    }

    if let Some(text) =
        first_non_empty_string(value, &["text", "transcript_text", "transcriptText"])
    {
        return Some(text);
    }

    None
}

fn extract_notes_text(value: &Value) -> Option<String> {
    first_non_empty_string(
        value,
        &[
            "notes_markdown",
            "notesMarkdown",
            "notes_plain",
            "notesPlain",
            "summary_markdown",
            "summaryMarkdown",
            "summary_text",
            "summaryText",
            "content",
            "markdown",
            "text",
        ],
    )
}

fn first_non_empty_string(value: &Value, keys: &[&str]) -> Option<String> {
    if let Some(obj) = value.as_object() {
        for key in keys {
            if let Some(text) = obj.get(*key).and_then(|v| v.as_str()).and_then(non_empty) {
                return Some(text);
            }
        }

        for child in obj.values() {
            if let Some(text) = first_non_empty_string(child, keys) {
                return Some(text);
            }
        }
    }

    if let Some(arr) = value.as_array() {
        for child in arr {
            if let Some(text) = first_non_empty_string(child, keys) {
                return Some(text);
            }
        }
    }

    None
}

fn find_first_string_by_key(value: &Value, keys: &[&str]) -> Option<String> {
    if let Some(obj) = value.as_object() {
        for key in keys {
            if let Some(text) = obj.get(*key).and_then(|v| v.as_str()).and_then(non_empty) {
                return Some(text);
            }
        }
        for child in obj.values() {
            if let Some(text) = find_first_string_by_key(child, keys) {
                return Some(text);
            }
        }
    }

    if let Some(arr) = value.as_array() {
        for child in arr {
            if let Some(text) = find_first_string_by_key(child, keys) {
                return Some(text);
            }
        }
    }

    None
}

fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn truncate_for_log(value: &str) -> &str {
    if value.len() > 200 {
        &value[..200]
    } else {
        value
    }
}

#[derive(Debug, Deserialize)]
struct RawMeeting {
    #[serde(default, alias = "meeting_id", alias = "document_id")]
    id: Option<String>,
    #[serde(default, alias = "name")]
    title: Option<String>,
    #[serde(default, alias = "createdAt", alias = "created_time")]
    created_at: Option<String>,
    #[serde(default, alias = "updatedAt", alias = "updated_time")]
    updated_at: Option<String>,
    #[serde(default, alias = "scheduled_start_time", alias = "startTime")]
    start_time: Option<String>,
    #[serde(default, alias = "scheduled_end_time", alias = "endTime")]
    end_time: Option<String>,
    #[serde(default, alias = "calendar_event_id", alias = "calendarEventId")]
    calendar_event_id: Option<String>,
    #[serde(default, alias = "calendar_event", alias = "calendarEvent")]
    calendar_event: Option<RawCalendarEvent>,
    #[serde(
        default,
        alias = "google_calendar_event",
        alias = "googleCalendarEvent"
    )]
    google_calendar_event: Option<RawCalendarEvent>,
    #[serde(default, alias = "invitees")]
    attendees: Vec<RawAttendee>,
}

impl RawMeeting {
    fn into_meeting(self) -> Option<GranolaMeeting> {
        let id = self.id.and_then(|value| non_empty(&value))?;
        let title = self
            .title
            .and_then(|value| non_empty(&value))
            .unwrap_or_else(|| "Untitled meeting".to_string());
        let raw_calendar = self.google_calendar_event.or(self.calendar_event);

        let mut attendee_emails = self
            .attendees
            .iter()
            .filter_map(RawAttendee::email)
            .collect::<Vec<_>>();
        if let Some(ref calendar) = raw_calendar {
            for email in calendar.attendee_emails() {
                if !attendee_emails.contains(&email) {
                    attendee_emails.push(email);
                }
            }
        }

        let google_calendar_event = build_google_calendar_event(
            &title,
            self.calendar_event_id,
            self.start_time,
            self.end_time,
            raw_calendar,
            &attendee_emails,
        );

        Some(GranolaMeeting {
            id,
            title,
            created_at: self.created_at,
            updated_at: self.updated_at,
            google_calendar_event,
            attendee_emails,
        })
    }
}

fn build_google_calendar_event(
    title: &str,
    calendar_event_id: Option<String>,
    start_time: Option<String>,
    end_time: Option<String>,
    raw_calendar: Option<RawCalendarEvent>,
    attendee_emails: &[String],
) -> Option<GoogleCalendarEvent> {
    let summary = raw_calendar
        .as_ref()
        .and_then(|calendar| calendar.summary.clone())
        .or_else(|| Some(title.to_string()));
    let id = calendar_event_id.or_else(|| raw_calendar.as_ref().and_then(|calendar| calendar.id()));
    let start = start_time.or_else(|| raw_calendar.as_ref().and_then(|calendar| calendar.start()));
    let end = end_time.or_else(|| raw_calendar.as_ref().and_then(|calendar| calendar.end()));

    if id.is_none() && start.is_none() && attendee_emails.is_empty() {
        return None;
    }

    Some(GoogleCalendarEvent {
        id,
        summary,
        start: start.map(|date_time| EventTime {
            date_time: Some(date_time),
        }),
        end: end.map(|date_time| EventTime {
            date_time: Some(date_time),
        }),
        status: None,
        attendees: attendee_emails
            .iter()
            .map(|email| GranolaAttendee {
                email: Some(email.clone()),
                response_status: None,
                is_self: None,
            })
            .collect(),
    })
}

#[derive(Debug, Deserialize)]
struct RawCalendarEvent {
    #[serde(default, alias = "calendar_event_id", alias = "calendarEventId")]
    id: Option<String>,
    #[serde(default, alias = "event_title", alias = "eventTitle")]
    summary: Option<String>,
    #[serde(default, alias = "scheduled_start_time", alias = "startTime")]
    scheduled_start_time: Option<String>,
    #[serde(default, alias = "scheduled_end_time", alias = "endTime")]
    scheduled_end_time: Option<String>,
    #[serde(default)]
    start: Option<RawEventTime>,
    #[serde(default)]
    end: Option<RawEventTime>,
    #[serde(default, alias = "invitees")]
    attendees: Vec<RawAttendee>,
}

impl RawCalendarEvent {
    fn id(&self) -> Option<String> {
        self.id.as_ref().and_then(|value| non_empty(value))
    }

    fn start(&self) -> Option<String> {
        self.scheduled_start_time
            .as_ref()
            .and_then(|value| non_empty(value))
            .or_else(|| self.start.as_ref().and_then(RawEventTime::date_time))
    }

    fn end(&self) -> Option<String> {
        self.scheduled_end_time
            .as_ref()
            .and_then(|value| non_empty(value))
            .or_else(|| self.end.as_ref().and_then(RawEventTime::date_time))
    }

    fn attendee_emails(&self) -> Vec<String> {
        self.attendees
            .iter()
            .filter_map(RawAttendee::email)
            .collect()
    }
}

#[derive(Debug, Deserialize)]
struct RawEventTime {
    #[serde(default, alias = "dateTime")]
    date_time: Option<String>,
}

impl RawEventTime {
    fn date_time(&self) -> Option<String> {
        self.date_time.as_ref().and_then(|value| non_empty(value))
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawAttendee {
    Email(String),
    Object {
        email: Option<String>,
        #[serde(default, alias = "emailAddress")]
        email_address: Option<String>,
    },
}

impl RawAttendee {
    fn email(&self) -> Option<String> {
        match self {
            Self::Email(value) => non_empty(value).map(|email| email.to_lowercase()),
            Self::Object {
                email,
                email_address,
            } => email
                .as_ref()
                .or(email_address.as_ref())
                .and_then(|value| non_empty(value))
                .map(|email| email.to_lowercase()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_sse_framed_json_response() {
        let body = "event: message\ndata: {\"jsonrpc\":\"2.0\",\"result\":{\"ok\":true}}\n\n";
        assert_eq!(
            extract_json_from_response(body),
            "{\"jsonrpc\":\"2.0\",\"result\":{\"ok\":true}}"
        );
    }

    #[test]
    fn maps_meeting_metadata_into_match_document() {
        let payload = serde_json::json!({
            "meetings": [{
                "id": "meeting-1",
                "title": "Customer check-in",
                "created_at": "2026-05-26T14:00:00Z",
                "calendar_event": {
                    "calendar_event_id": "cal-123@example.com",
                    "scheduled_start_time": "2026-05-26T14:00:00Z",
                    "scheduled_end_time": "2026-05-26T15:00:00Z",
                    "invitees": [{ "email": "User@Example.com" }]
                }
            }]
        });

        let meetings = meetings_from_value(&payload);
        assert_eq!(meetings.len(), 1);
        let doc = meetings[0].as_match_document();
        assert_eq!(doc.title, "Customer check-in");
        assert_eq!(
            doc.google_calendar_event
                .as_ref()
                .and_then(|event| event.id.as_deref()),
            Some("cal-123@example.com")
        );
        assert_eq!(doc.attendee_emails, vec!["user@example.com"]);
    }

    #[test]
    fn parses_granola_xml_meetings() {
        let xml = "<meetings_data from=\"Jun 1, 2026\" to=\"Jun 30, 2026\" count=\"2\">\n\
            <meeting id=\"b9f74b7b-3e04-432e-a897-d63fa7341f0f\" title=\"JG and Brant sync\" date=\"Jun 30, 2026 2:30 PM EDT\">\n\
            <known_participants><james.giroux@a8c.com><Brant.Williams@a8c.com></known_participants>\n\
            </meeting>\n\
            <meeting id=\"no-date-id\" title=\"Standup\" date=\"\">\n</meeting>\n\
            </meetings_data>";
        let meetings = parse_meetings_xml(xml);
        assert_eq!(meetings.len(), 2);

        let first = &meetings[0];
        assert_eq!(first.id, "b9f74b7b-3e04-432e-a897-d63fa7341f0f");
        assert_eq!(first.title, "JG and Brant sync");
        // EDT 2:30 PM == 18:30 UTC
        assert_eq!(first.created_at.as_deref(), Some("2026-06-30T18:30:00+00:00"));
        assert_eq!(
            first.attendee_emails,
            vec![
                "james.giroux@a8c.com".to_string(),
                "brant.williams@a8c.com".to_string()
            ]
        );
        assert_eq!(
            first
                .google_calendar_event
                .as_ref()
                .and_then(|e| e.start.as_ref())
                .and_then(|s| s.date_time.as_deref()),
            Some("2026-06-30T18:30:00+00:00")
        );

        // Meeting with an empty date still parses (id + title); no start time.
        assert_eq!(meetings[1].id, "no-date-id");
        assert!(meetings[1].created_at.is_none());
    }

    #[test]
    fn extracts_transcript_segments() {
        let payload = serde_json::json!({
            "transcript": [
                { "text": "First segment." },
                { "text": "Second segment." }
            ]
        });

        assert_eq!(
            extract_transcript_text(&payload).as_deref(),
            Some("First segment.\nSecond segment.")
        );
    }
}
