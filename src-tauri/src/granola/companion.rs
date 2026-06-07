//! Client for Granola's local companion IPC bridge.
//!
//! Granola's current desktop app can expose notes through a local socket plus
//! metadata file. This keeps DailyOS on Granola's supported local integration
//! path and avoids treating stale plaintext cache files as authoritative.

use chrono::{DateTime, SecondsFormat, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::cache::{
    EventTime, GoogleCalendarEvent, GranolaAttendee, GranolaContentType, GranolaDocument,
};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const NOTE_PAGE_SIZE: usize = 100;
const MAX_NOTES_PER_SCAN: usize = 500;

#[derive(Debug, thiserror::Error)]
pub enum CompanionError {
    #[error("Granola companion access is not enabled")]
    NotConfigured,
    #[error("Granola companion metadata is invalid")]
    MetadataInvalid(String),
    #[error("Granola companion socket is not available")]
    SocketMissing,
    #[error("Granola companion connection failed")]
    ConnectionFailed(String),
    #[error("Granola companion request failed")]
    RequestFailed { code: String, message: String },
    #[error("Granola companion response was invalid")]
    InvalidResponse(String),
    #[error("Granola note has no transcript or notes content")]
    NoContent,
}

impl CompanionError {
    pub fn is_unavailable(&self) -> bool {
        matches!(self, Self::NotConfigured | Self::SocketMissing)
    }
}

#[derive(Debug, Clone)]
pub struct CompanionStatus {
    pub available: bool,
    pub metadata_path: PathBuf,
    pub socket_path: Option<PathBuf>,
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CompanionNote {
    pub id: String,
    pub title: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub google_calendar_event: Option<GoogleCalendarEvent>,
    pub attendee_emails: Vec<String>,
}

impl CompanionNote {
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
}

#[derive(Debug, Clone)]
pub struct CompanionClient {
    metadata_path: PathBuf,
}

impl CompanionClient {
    pub fn new() -> Result<Self, CompanionError> {
        let metadata_path = metadata_path();
        let metadata = read_metadata(&metadata_path)?;
        ensure_socket_available(&metadata)?;
        Ok(Self { metadata_path })
    }

    pub fn status() -> CompanionStatus {
        let metadata_path = metadata_path();
        match read_metadata(&metadata_path) {
            Ok(metadata) => {
                let socket_path = PathBuf::from(&metadata.socket_path);
                if socket_path.exists() {
                    CompanionStatus {
                        available: true,
                        metadata_path,
                        socket_path: Some(socket_path),
                        message: None,
                    }
                } else {
                    CompanionStatus {
                        available: false,
                        metadata_path,
                        socket_path: Some(socket_path),
                        message: Some(
                            "Granola companion bridge is not running. Enable Granola companion access and keep Granola open.".to_string(),
                        ),
                    }
                }
            }
            Err(CompanionError::NotConfigured) => CompanionStatus {
                available: false,
                metadata_path,
                socket_path: None,
                message: Some(
                    "Granola companion bridge is not enabled. Enable Granola companion access in Granola.".to_string(),
                ),
            },
            Err(error) => CompanionStatus {
                available: false,
                metadata_path,
                socket_path: None,
                message: Some(error.to_string()),
            },
        }
    }

    pub fn list_recent_notes(&self, days_back: i32) -> Result<Vec<CompanionNote>, CompanionError> {
        let after = (Utc::now() - chrono::Duration::days(days_back.max(1) as i64))
            .to_rfc3339_opts(SecondsFormat::Secs, true);
        self.list_notes(Some(after), None, MAX_NOTES_PER_SCAN)
    }

    pub fn list_notes_near(
        &self,
        start_time: &str,
        end_time: Option<&str>,
    ) -> Result<Vec<CompanionNote>, CompanionError> {
        let start = parse_time(start_time).unwrap_or_else(Utc::now);
        let end = end_time
            .and_then(parse_time)
            .unwrap_or_else(|| start + chrono::Duration::hours(3));
        let after =
            (start - chrono::Duration::hours(12)).to_rfc3339_opts(SecondsFormat::Secs, true);
        let before = (end + chrono::Duration::days(2)).to_rfc3339_opts(SecondsFormat::Secs, true);
        self.list_notes(Some(after), Some(before), MAX_NOTES_PER_SCAN)
    }

    pub fn fetch_document(&self, note: &CompanionNote) -> Result<GranolaDocument, CompanionError> {
        if let Ok(text) = self.fetch_transcript_text(&note.id) {
            if !text.trim().is_empty() {
                return Ok(self.document_from_note(note, text, GranolaContentType::Transcript));
            }
        }

        let notes = self.fetch_notes_text(&note.id)?;
        if notes.trim().is_empty() {
            return Err(CompanionError::NoContent);
        }

        Ok(self.document_from_note(note, notes, GranolaContentType::Notes))
    }

    fn list_notes(
        &self,
        created_after: Option<String>,
        created_before: Option<String>,
        max_notes: usize,
    ) -> Result<Vec<CompanionNote>, CompanionError> {
        let mut notes = Vec::new();
        let mut offset = 0usize;

        loop {
            let mut params = json!({
                "limit": NOTE_PAGE_SIZE,
                "offset": offset,
            });
            if let Some(after) = created_after.as_ref() {
                params["created_after"] = json!(after);
            }
            if let Some(before) = created_before.as_ref() {
                params["created_before"] = json!(before);
            }

            let result = self.call("notes.list", params)?;
            let page: NotesListResult = serde_json::from_value(result)
                .map_err(|e| CompanionError::InvalidResponse(e.to_string()))?;

            notes.extend(
                page.notes
                    .into_iter()
                    .filter(|note| note.note_type == "meeting")
                    .filter_map(CompanionNote::try_from_raw),
            );

            if !page.has_more || notes.len() >= max_notes {
                notes.truncate(max_notes);
                return Ok(notes);
            }

            offset = page.next_offset.unwrap_or(offset + NOTE_PAGE_SIZE);
        }
    }

    fn fetch_transcript_text(&self, note_id: &str) -> Result<String, CompanionError> {
        let result = self.call("notes.transcript.get", json!({ "id": note_id }))?;
        let transcript: TranscriptResult = serde_json::from_value(result)
            .map_err(|e| CompanionError::InvalidResponse(e.to_string()))?;
        Ok(transcript
            .transcript
            .into_iter()
            .map(|chunk| chunk.text)
            .filter(|text| !text.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n"))
    }

    fn fetch_notes_text(&self, note_id: &str) -> Result<String, CompanionError> {
        let result = self.call("notes.get", json!({ "ids": [note_id] }))?;
        let notes: NotesGetResult = serde_json::from_value(result)
            .map_err(|e| CompanionError::InvalidResponse(e.to_string()))?;
        let note = notes
            .notes
            .into_iter()
            .next()
            .ok_or(CompanionError::NoContent)?;

        [
            note.notes_markdown,
            note.notes_plain,
            note.summary_markdown,
            note.summary_text,
        ]
        .into_iter()
        .flatten()
        .find(|text| !text.trim().is_empty())
        .ok_or(CompanionError::NoContent)
    }

    fn document_from_note(
        &self,
        note: &CompanionNote,
        content: String,
        content_type: GranolaContentType,
    ) -> GranolaDocument {
        GranolaDocument {
            id: note.id.clone(),
            title: note.title.clone(),
            created_at: note.created_at.clone(),
            updated_at: note.updated_at.clone(),
            content,
            content_type,
            google_calendar_event: note.google_calendar_event.clone(),
            attendee_emails: note.attendee_emails.clone(),
        }
    }

    fn call(&self, method: &str, params: Value) -> Result<Value, CompanionError> {
        let metadata = read_metadata(&self.metadata_path)?;
        ensure_socket_available(&metadata)?;
        call_companion(&metadata, method, params)
    }
}

fn metadata_path() -> PathBuf {
    super::granola_dir()
        .join("companion-cli")
        .join("companion-cli.json")
}

fn parse_time(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .ok()
}

#[derive(Debug, Deserialize)]
struct CompanionMetadata {
    protocol_version: u32,
    socket_path: String,
    token: String,
}

fn read_metadata(path: &Path) -> Result<CompanionMetadata, CompanionError> {
    let raw = std::fs::read_to_string(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            CompanionError::NotConfigured
        } else {
            CompanionError::MetadataInvalid(e.to_string())
        }
    })?;
    let metadata: CompanionMetadata =
        serde_json::from_str(&raw).map_err(|e| CompanionError::MetadataInvalid(e.to_string()))?;
    if metadata.protocol_version != 1
        || metadata.socket_path.is_empty()
        || metadata.token.is_empty()
    {
        return Err(CompanionError::MetadataInvalid(
            "unsupported metadata shape".to_string(),
        ));
    }
    Ok(metadata)
}

fn ensure_socket_available(metadata: &CompanionMetadata) -> Result<(), CompanionError> {
    if Path::new(&metadata.socket_path).exists() {
        Ok(())
    } else {
        Err(CompanionError::SocketMissing)
    }
}

#[cfg(unix)]
fn call_companion(
    metadata: &CompanionMetadata,
    method: &str,
    params: Value,
) -> Result<Value, CompanionError> {
    use std::os::unix::net::UnixStream;

    let mut stream = UnixStream::connect(&metadata.socket_path)
        .map_err(|e| CompanionError::ConnectionFailed(e.to_string()))?;
    stream
        .set_read_timeout(Some(REQUEST_TIMEOUT))
        .map_err(|e| CompanionError::ConnectionFailed(e.to_string()))?;
    stream
        .set_write_timeout(Some(REQUEST_TIMEOUT))
        .map_err(|e| CompanionError::ConnectionFailed(e.to_string()))?;

    let reader_stream = stream
        .try_clone()
        .map_err(|e| CompanionError::ConnectionFailed(e.to_string()))?;
    let mut reader = BufReader::new(reader_stream);

    write_json_line(
        &mut stream,
        &json!({ "type": "auth", "token": metadata.token }),
    )?;
    let auth: AuthResponse = read_json_line(&mut reader)?;
    if !auth.ok {
        return Err(CompanionError::RequestFailed {
            code: "UNAUTHENTICATED".to_string(),
            message: auth
                .error
                .unwrap_or_else(|| "Granola companion authentication failed".to_string()),
        });
    }

    let id = uuid::Uuid::new_v4().to_string();
    write_json_line(
        &mut stream,
        &json!({
            "type": "request",
            "id": id,
            "method": method,
            "params": params,
        }),
    )?;

    let response: RpcResponse = read_json_line(&mut reader)?;
    if response.ok {
        response
            .result
            .ok_or_else(|| CompanionError::InvalidResponse("missing result".to_string()))
    } else {
        let error = response.error.unwrap_or(RpcError {
            code: "UNKNOWN".to_string(),
            message: "Granola companion request failed".to_string(),
        });
        Err(CompanionError::RequestFailed {
            code: error.code,
            message: error.message,
        })
    }
}

#[cfg(not(unix))]
fn call_companion(
    _metadata: &CompanionMetadata,
    _method: &str,
    _params: Value,
) -> Result<Value, CompanionError> {
    Err(CompanionError::ConnectionFailed(
        "Granola companion IPC is only implemented for Unix sockets".to_string(),
    ))
}

fn write_json_line(stream: &mut impl Write, value: &Value) -> Result<(), CompanionError> {
    serde_json::to_writer(&mut *stream, value)
        .map_err(|e| CompanionError::InvalidResponse(e.to_string()))?;
    stream
        .write_all(b"\n")
        .map_err(|e| CompanionError::ConnectionFailed(e.to_string()))?;
    stream
        .flush()
        .map_err(|e| CompanionError::ConnectionFailed(e.to_string()))
}

fn read_json_line<T: for<'de> Deserialize<'de>>(
    reader: &mut impl BufRead,
) -> Result<T, CompanionError> {
    let mut line = String::new();
    let bytes = reader
        .read_line(&mut line)
        .map_err(|e| CompanionError::ConnectionFailed(e.to_string()))?;
    if bytes == 0 {
        return Err(CompanionError::ConnectionFailed(
            "Granola companion closed the connection".to_string(),
        ));
    }
    serde_json::from_str(line.trim_end())
        .map_err(|e| CompanionError::InvalidResponse(e.to_string()))
}

impl CompanionNote {
    fn try_from_raw(raw: CompanionNoteRaw) -> Option<Self> {
        let title = raw.title.filter(|title| !title.trim().is_empty())?;
        let google_calendar_event = raw
            .calendar_event
            .as_ref()
            .map(|event| GoogleCalendarEvent {
                id: event.calendar_event_id.clone(),
                summary: event.event_title.clone(),
                start: event
                    .scheduled_start_time
                    .as_ref()
                    .map(|date_time| EventTime {
                        date_time: Some(date_time.clone()),
                    }),
                end: event
                    .scheduled_end_time
                    .as_ref()
                    .map(|date_time| EventTime {
                        date_time: Some(date_time.clone()),
                    }),
                status: raw.status.clone(),
                attendees: event
                    .invitees
                    .iter()
                    .map(|invitee| GranolaAttendee {
                        email: Some(invitee.email.to_lowercase()),
                        response_status: None,
                        is_self: None,
                    })
                    .collect(),
            });
        let attendee_emails = raw
            .calendar_event
            .as_ref()
            .map(|event| {
                event
                    .invitees
                    .iter()
                    .map(|invitee| invitee.email.to_lowercase())
                    .collect()
            })
            .unwrap_or_default();

        Some(Self {
            id: raw.id,
            title,
            created_at: Some(raw.created_at),
            updated_at: Some(raw.updated_at),
            google_calendar_event,
            attendee_emails,
        })
    }
}

#[derive(Debug, Deserialize)]
struct AuthResponse {
    ok: bool,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RpcResponse {
    ok: bool,
    result: Option<Value>,
    error: Option<RpcError>,
}

#[derive(Debug, Deserialize)]
struct RpcError {
    code: String,
    message: String,
}

#[derive(Debug, Deserialize)]
struct NotesListResult {
    notes: Vec<CompanionNoteRaw>,
    has_more: bool,
    next_offset: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct CompanionNoteRaw {
    id: String,
    title: Option<String>,
    created_at: String,
    updated_at: String,
    #[serde(rename = "type")]
    note_type: String,
    status: Option<String>,
    calendar_event: Option<CompanionCalendarEvent>,
}

#[derive(Debug, Deserialize)]
struct CompanionCalendarEvent {
    event_title: Option<String>,
    invitees: Vec<CompanionInvitee>,
    calendar_event_id: Option<String>,
    scheduled_start_time: Option<String>,
    scheduled_end_time: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CompanionInvitee {
    email: String,
}

#[derive(Debug, Deserialize)]
struct TranscriptResult {
    transcript: Vec<TranscriptChunk>,
}

#[derive(Debug, Deserialize)]
struct TranscriptChunk {
    text: String,
}

#[derive(Debug, Deserialize)]
struct NotesGetResult {
    notes: Vec<NoteDetail>,
}

#[derive(Debug, Deserialize)]
struct NoteDetail {
    notes_plain: Option<String>,
    notes_markdown: Option<String>,
    summary_text: Option<String>,
    summary_markdown: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn companion_note_maps_calendar_fields_for_matching() {
        let raw = CompanionNoteRaw {
            id: "11111111-1111-1111-1111-111111111111".to_string(),
            title: Some("Customer check-in".to_string()),
            created_at: "2026-05-26T14:00:00Z".to_string(),
            updated_at: "2026-05-26T15:00:00Z".to_string(),
            note_type: "meeting".to_string(),
            status: Some("done".to_string()),
            calendar_event: Some(CompanionCalendarEvent {
                event_title: Some("Customer check-in".to_string()),
                invitees: vec![CompanionInvitee {
                    email: "user@example.com".to_string(),
                }],
                calendar_event_id: Some("cal-123@example.com".to_string()),
                scheduled_start_time: Some("2026-05-26T14:00:00Z".to_string()),
                scheduled_end_time: Some("2026-05-26T15:00:00Z".to_string()),
            }),
        };

        let note = CompanionNote::try_from_raw(raw).expect("valid note maps");
        let doc = note.as_match_document();

        assert_eq!(doc.title, "Customer check-in");
        assert_eq!(
            doc.google_calendar_event
                .as_ref()
                .and_then(|event| event.id.as_deref()),
            Some("cal-123@example.com")
        );
        assert_eq!(doc.attendee_emails, vec!["user@example.com"]);
    }
}
