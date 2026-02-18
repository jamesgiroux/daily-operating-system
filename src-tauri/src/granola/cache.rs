//! Granola cache file reader.
//!
//! The cache file at `~/Library/Application Support/Granola/cache-v3.json`
//! contains a double-JSON-encoded structure: the top-level `cache` field is
//! a JSON string that must be parsed again to get the actual data.

use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// Top-level cache file structure (double-encoded).
#[derive(Debug, Deserialize)]
struct CacheFile {
    cache: String,
}

/// Inner cache state after second JSON parse.
#[derive(Debug, Deserialize)]
struct CacheState {
    state: InnerState,
}

#[derive(Debug, Deserialize)]
struct InnerState {
    #[serde(default)]
    documents: HashMap<String, GranolaDocumentRaw>,
    #[serde(default)]
    transcripts: HashMap<String, serde_json::Value>,
}

/// Raw document from the cache (before filtering).
#[derive(Debug, Deserialize)]
struct GranolaDocumentRaw {
    id: Option<String>,
    title: Option<String>,
    created_at: Option<String>,
    updated_at: Option<String>,
    #[serde(default)]
    notes_markdown: Option<String>,
    #[serde(rename = "type")]
    doc_type: Option<String>,
    #[serde(default)]
    valid_meeting: Option<bool>,
    google_calendar_event: Option<GoogleCalendarEvent>,
    people: Option<GranolaPeople>,
}

/// Google Calendar event data embedded in a Granola document.
#[derive(Debug, Clone, Deserialize)]
pub struct GoogleCalendarEvent {
    pub id: Option<String>,
    pub summary: Option<String>,
    pub start: Option<EventTime>,
    pub end: Option<EventTime>,
    pub status: Option<String>,
    #[serde(default)]
    pub attendees: Vec<GranolaAttendee>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EventTime {
    #[serde(rename = "dateTime")]
    pub date_time: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GranolaAttendee {
    pub email: Option<String>,
    #[serde(rename = "responseStatus")]
    pub response_status: Option<String>,
    #[serde(rename = "self")]
    pub is_self: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct GranolaPeople {
    #[serde(default)]
    attendees: Vec<GranolaPersonAttendee>,
}

#[derive(Debug, Deserialize)]
struct GranolaPersonAttendee {
    email: Option<String>,
}

/// A parsed, validated Granola document ready for sync.
#[derive(Debug, Clone)]
pub struct GranolaDocument {
    pub id: String,
    pub title: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub content: String,
    pub google_calendar_event: Option<GoogleCalendarEvent>,
    pub attendee_emails: Vec<String>,
}

/// Read and parse the Granola cache file.
///
/// Returns valid meeting documents with content (notes_markdown or transcript).
pub fn read_cache(cache_path: &Path) -> Result<Vec<GranolaDocument>, String> {
    let raw = std::fs::read_to_string(cache_path)
        .map_err(|e| format!("Failed to read Granola cache: {}", e))?;

    let cache_file: CacheFile = serde_json::from_str(&raw)
        .map_err(|e| format!("Failed to parse Granola cache outer JSON: {}", e))?;

    let cache_state: CacheState = serde_json::from_str(&cache_file.cache)
        .map_err(|e| format!("Failed to parse Granola cache inner JSON: {}", e))?;

    let mut documents = Vec::new();

    for (key, doc) in &cache_state.state.documents {
        // Filter: valid_meeting == true, type == "meeting"
        if doc.valid_meeting != Some(true) {
            continue;
        }
        if doc.doc_type.as_deref() != Some("meeting") {
            continue;
        }

        let id = doc.id.as_deref().unwrap_or(key).to_string();
        let title = match &doc.title {
            Some(t) if !t.is_empty() => t.clone(),
            _ => continue,
        };

        // Extract content: prefer raw transcript, fall back to notes_markdown
        let content = extract_content(
            &id,
            doc.notes_markdown.as_deref(),
            &cache_state.state.transcripts,
        );

        let content = match content {
            Some(c) if !c.trim().is_empty() => c,
            _ => continue,
        };

        // Collect attendee emails
        let mut attendee_emails: Vec<String> = Vec::new();
        if let Some(ref cal_event) = doc.google_calendar_event {
            for att in &cal_event.attendees {
                if let Some(ref email) = att.email {
                    attendee_emails.push(email.to_lowercase());
                }
            }
        }
        if let Some(ref people) = doc.people {
            for att in &people.attendees {
                if let Some(ref email) = att.email {
                    let lower = email.to_lowercase();
                    if !attendee_emails.contains(&lower) {
                        attendee_emails.push(lower);
                    }
                }
            }
        }

        documents.push(GranolaDocument {
            id,
            title,
            created_at: doc.created_at.clone(),
            updated_at: doc.updated_at.clone(),
            content,
            google_calendar_event: doc.google_calendar_event.clone(),
            attendee_emails,
        });
    }

    Ok(documents)
}

/// Extract the best available content for a document.
///
/// Prefers raw transcript (paid tier), falls back to notes_markdown.
fn extract_content(
    doc_id: &str,
    notes_markdown: Option<&str>,
    transcripts: &HashMap<String, serde_json::Value>,
) -> Option<String> {
    // Check for raw transcript data (paid tier only)
    if let Some(transcript_data) = transcripts.get(doc_id) {
        if let Some(text) = extract_transcript_text(transcript_data) {
            if !text.trim().is_empty() {
                return Some(text);
            }
        }
    }

    // Fall back to notes_markdown
    notes_markdown.map(|s| s.to_string())
}

/// Extract text from a transcript value (which may be a string or structured object).
fn extract_transcript_text(value: &serde_json::Value) -> Option<String> {
    // Direct string
    if let Some(s) = value.as_str() {
        return Some(s.to_string());
    }

    // Object with a "text" or "transcript" field
    if let Some(obj) = value.as_object() {
        if let Some(text) = obj.get("text").and_then(|v| v.as_str()) {
            return Some(text.to_string());
        }
        if let Some(text) = obj.get("transcript").and_then(|v| v.as_str()) {
            return Some(text.to_string());
        }
    }

    None
}

/// Count valid meeting documents in the cache without fully parsing.
pub fn count_documents(cache_path: &Path) -> Result<usize, String> {
    read_cache(cache_path).map(|docs| docs.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_content_prefers_transcript() {
        let mut transcripts = HashMap::new();
        transcripts.insert(
            "doc1".to_string(),
            serde_json::json!("Raw transcript text here"),
        );

        let result = extract_content("doc1", Some("Notes markdown"), &transcripts);
        assert_eq!(result, Some("Raw transcript text here".to_string()));
    }

    #[test]
    fn test_extract_content_falls_back_to_notes() {
        let transcripts = HashMap::new();
        let result = extract_content("doc1", Some("Notes markdown"), &transcripts);
        assert_eq!(result, Some("Notes markdown".to_string()));
    }

    #[test]
    fn test_extract_content_none_when_empty() {
        let transcripts = HashMap::new();
        let result = extract_content("doc1", None, &transcripts);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_transcript_text_string() {
        let val = serde_json::json!("Hello world");
        assert_eq!(extract_transcript_text(&val), Some("Hello world".to_string()));
    }

    #[test]
    fn test_extract_transcript_text_object() {
        let val = serde_json::json!({"text": "Transcript text"});
        assert_eq!(
            extract_transcript_text(&val),
            Some("Transcript text".to_string())
        );
    }

    #[test]
    fn test_read_cache_valid_file() {
        let dir = tempfile::tempdir().unwrap();
        let cache_path = dir.path().join("cache-v3.json");

        let inner = serde_json::json!({
            "state": {
                "documents": {
                    "doc-1": {
                        "id": "doc-1",
                        "title": "Weekly Sync",
                        "type": "meeting",
                        "valid_meeting": true,
                        "notes_markdown": "# Meeting Notes\n\nDiscussed Q1 goals.",
                        "google_calendar_event": {
                            "id": "cal-event-123",
                            "summary": "Weekly Sync",
                            "start": { "dateTime": "2026-02-17T14:00:00Z" },
                            "end": { "dateTime": "2026-02-17T15:00:00Z" },
                            "attendees": [
                                { "email": "alice@acme.com" },
                                { "email": "bob@acme.com" }
                            ]
                        }
                    },
                    "doc-2": {
                        "id": "doc-2",
                        "title": "Invalid Meeting",
                        "type": "meeting",
                        "valid_meeting": false,
                        "notes_markdown": "Should be filtered out"
                    }
                },
                "transcripts": {},
                "events": [],
                "people": []
            }
        });

        let cache_file = serde_json::json!({
            "cache": serde_json::to_string(&inner).unwrap()
        });

        std::fs::write(&cache_path, serde_json::to_string(&cache_file).unwrap()).unwrap();

        let docs = read_cache(&cache_path).unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].id, "doc-1");
        assert_eq!(docs[0].title, "Weekly Sync");
        assert!(docs[0].content.contains("Q1 goals"));
        assert_eq!(docs[0].attendee_emails.len(), 2);
        assert!(docs[0].google_calendar_event.is_some());
        assert_eq!(
            docs[0].google_calendar_event.as_ref().unwrap().id.as_deref(),
            Some("cal-event-123")
        );
    }
}
