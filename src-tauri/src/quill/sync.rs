//! Transcript sync — manages quill_sync_state rows and meeting eligibility.
//!
//! State machine: pending → polling → fetching → processing → completed
//!                          ↘ retry (backoff) ↗          ↘ failed → retry?
//!                          ↘ abandoned (after max attempts)

use std::path::Path;

use chrono::Utc;

use crate::db::ActionDb;
use crate::types::{CalendarEvent, MeetingType};

/// Meeting types eligible for Quill transcript sync.
/// Mirrors PREP_ELIGIBLE_TYPES from google.rs — we only sync transcripts
/// for meetings that would benefit from intelligence enrichment.
const SYNC_ELIGIBLE_TYPES: &[MeetingType] = &[
    MeetingType::Customer,
    MeetingType::Qbr,
    MeetingType::Partnership,
    MeetingType::Internal,
    MeetingType::TeamSync,
    MeetingType::OneOnOne,
    MeetingType::External,
];

/// Minimum meeting duration (in minutes) to be eligible for transcript sync.
const MIN_DURATION_MINUTES: i64 = 10;

/// Determine whether a calendar event should have a Quill sync row created.
///
/// Filters out:
/// - All-day events
/// - Personal meetings
/// - Meetings shorter than 10 minutes
/// - Meeting types not in the eligible list
pub fn should_sync_meeting(event: &CalendarEvent) -> bool {
    if event.is_all_day {
        return false;
    }

    let duration = (event.end - event.start).num_minutes();
    if duration < MIN_DURATION_MINUTES {
        return false;
    }

    SYNC_ELIGIBLE_TYPES.contains(&event.meeting_type)
}

/// Create a quill_sync_state row for a meeting, if one doesn't already exist.
///
/// Returns the sync row ID on success, or an error message.
/// Uses INSERT OR IGNORE so it's safe to call multiple times for the same meeting.
pub fn create_sync_for_meeting(db: &ActionDb, meeting_id: &str) -> Result<String, String> {
    // Check if a sync row already exists
    match db.get_quill_sync_state(meeting_id) {
        Ok(Some(existing)) => return Ok(existing.id),
        Ok(None) => {}
        Err(e) => return Err(format!("Failed to check sync state: {}", e)),
    }

    db.insert_quill_sync_state(meeting_id)
        .map_err(|e| format!("Failed to create sync row: {}", e))
}

/// Transition a sync row to a new state with optional metadata.
pub fn transition_state(
    db: &ActionDb,
    sync_id: &str,
    new_state: &str,
    quill_meeting_id: Option<&str>,
    match_confidence: Option<f64>,
    transcript_path: Option<&str>,
    error_message: Option<&str>,
) -> Result<(), String> {
    db.update_quill_sync_state(
        sync_id,
        new_state,
        quill_meeting_id,
        match_confidence,
        error_message,
        transcript_path,
    )
    .map_err(|e| format!("Failed to transition state: {}", e))
}

/// Advance the attempt counter for a sync row.
///
/// Returns `Ok(true)` if the row can still be retried, `Ok(false)` if it has been
/// marked as abandoned (max attempts reached).
pub fn advance_attempt(db: &ActionDb, sync_id: &str) -> Result<bool, String> {
    db.advance_quill_sync_attempt(sync_id)
        .map_err(|e| format!("Failed to advance attempt: {}", e))
}

/// Check if a meeting has ended (end time is in the past).
pub fn has_meeting_ended(event: &CalendarEvent) -> bool {
    event.end < Utc::now()
}

/// Process a fetched transcript through the existing AI pipeline.
///
/// Writes the transcript to a temp file, runs `process_transcript`,
/// and updates the sync state row with results.
pub fn process_fetched_transcript(
    db: &ActionDb,
    sync_id: &str,
    meeting: &CalendarEvent,
    transcript_text: &str,
    workspace: &Path,
    profile: &str,
    ai_config: Option<&crate::types::AiModelConfig>,
) -> Result<String, String> {
    // Write transcript to temp file
    let temp_dir = workspace.join("_temp");
    let _ = std::fs::create_dir_all(&temp_dir);
    let temp_path = temp_dir.join(format!("quill-transcript-{}.md", sync_id));

    std::fs::write(&temp_path, transcript_text)
        .map_err(|e| format!("Failed to write temp transcript: {}", e))?;

    let temp_path_str = temp_path.display().to_string();

    // Transition to processing state
    let _ = transition_state(db, sync_id, "processing", None, None, None, None);

    // Run through existing transcript pipeline
    let result = crate::processor::transcript::process_transcript(
        workspace,
        &temp_path_str,
        meeting,
        Some(db),
        profile,
        ai_config,
    );

    // Clean up temp file
    let _ = std::fs::remove_file(&temp_path);

    if result.status == "success" {
        let dest = result.destination.clone().unwrap_or_default();

        // Update meeting record with transcript path
        let _ = db.update_meeting_transcript_metadata(
            &meeting.id,
            &dest,
            &Utc::now().to_rfc3339(),
            result.summary.as_deref(),
        );

        // Mark sync as completed
        let _ = transition_state(db, sync_id, "completed", None, None, Some(&dest), None);

        log::info!(
            "Quill sync: transcript processed for '{}' → {}",
            meeting.title,
            dest
        );
        Ok(dest)
    } else {
        let error = result
            .message
            .unwrap_or_else(|| "Transcript processing failed".to_string());
        let _ = transition_state(db, sync_id, "failed", None, None, None, Some(&error));
        Err(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn make_event(
        meeting_type: MeetingType,
        duration_minutes: i64,
        is_all_day: bool,
    ) -> CalendarEvent {
        let start = Utc.with_ymd_and_hms(2026, 2, 17, 14, 0, 0).unwrap();
        CalendarEvent {
            id: "test-event".to_string(),
            title: "Test Meeting".to_string(),
            start,
            end: start + chrono::Duration::minutes(duration_minutes),
            meeting_type,
            account: None,
            attendees: vec![],
            is_all_day,
        }
    }

    #[test]
    fn test_customer_meeting_eligible() {
        let event = make_event(MeetingType::Customer, 30, false);
        assert!(should_sync_meeting(&event));
    }

    #[test]
    fn test_personal_meeting_ineligible() {
        let event = make_event(MeetingType::Personal, 30, false);
        assert!(!should_sync_meeting(&event));
    }

    #[test]
    fn test_all_day_event_ineligible() {
        let event = make_event(MeetingType::Customer, 480, true);
        assert!(!should_sync_meeting(&event));
    }

    #[test]
    fn test_short_meeting_ineligible() {
        let event = make_event(MeetingType::Customer, 5, false);
        assert!(!should_sync_meeting(&event));
    }

    #[test]
    fn test_10_minute_meeting_eligible() {
        let event = make_event(MeetingType::Internal, 10, false);
        assert!(should_sync_meeting(&event));
    }

    #[test]
    fn test_external_meeting_eligible() {
        let event = make_event(MeetingType::External, 30, false);
        assert!(should_sync_meeting(&event));
    }

    #[test]
    fn test_all_hands_ineligible() {
        let event = make_event(MeetingType::AllHands, 60, false);
        assert!(!should_sync_meeting(&event));
    }
}
