//! Manual override / add adapter. Used by user-driven entity-link mutations
//! in `services::meetings::mutate_meeting_entities` (Replace + Add branches).
//!
//! These writes never carry `calendar_event_id` — the user is annotating a
//! stub history row, not authoring against a calendar event.

use crate::db::{ActionDb, EnsureMeetingHistoryInput, MeetingSyncOutcome};

use super::invariants;
use super::types::{MeetingSource, WriteError, WriteOutcome, WriteRequest};

pub fn write(db: &ActionDb, req: &WriteRequest) -> Result<WriteOutcome, WriteError> {
    debug_assert_eq!(req.source, MeetingSource::Manual);
    invariants::validate(req)?;

    let input = EnsureMeetingHistoryInput {
        id: &req.id,
        title: &req.title,
        meeting_type: &req.meeting_type,
        start_time: &req.start_time,
        end_time: req.end_time.as_deref(),
        calendar_event_id: None,
        attendees: req.attendees.as_deref(),
        description: req.description.as_deref(),
    };

    match db.ensure_meeting_in_history(input)? {
        MeetingSyncOutcome::New => Ok(WriteOutcome::Created),
        MeetingSyncOutcome::Changed => Ok(WriteOutcome::Updated),
        MeetingSyncOutcome::Unchanged => Ok(WriteOutcome::Unchanged),
    }
}
