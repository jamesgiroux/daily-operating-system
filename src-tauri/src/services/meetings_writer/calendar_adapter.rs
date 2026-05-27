//! Calendar source adapter. Used by:
//!
//! - Google Calendar attendance-batch ingestion (`services::people`).
//! - Prepare/timeline calendar fallback (`commands::integrations`,
//!   `prepare::orchestrate`).
//!
//! Maps `WriteRequest` to `ensure_meeting_in_history` which preserves the
//! "INSERT new / UPDATE if attributes changed" semantics the polling path
//! depends on.

use crate::db::{ActionDb, EnsureMeetingHistoryInput, MeetingSyncOutcome};

use super::invariants;
use super::types::{MeetingSource, WriteError, WriteOutcome, WriteRequest};

pub fn write(db: &ActionDb, req: &WriteRequest) -> Result<WriteOutcome, WriteError> {
    debug_assert_eq!(req.source, MeetingSource::Calendar);
    invariants::validate(req)?;

    let input = EnsureMeetingHistoryInput {
        id: &req.id,
        title: &req.title,
        meeting_type: &req.meeting_type,
        start_time: &req.start_time,
        end_time: req.end_time.as_deref(),
        calendar_event_id: req.calendar_event_id.as_deref(),
        attendees: req.attendees.as_deref(),
        description: req.description.as_deref(),
    };

    match db.ensure_meeting_in_history(input)? {
        MeetingSyncOutcome::New => Ok(WriteOutcome::Created),
        MeetingSyncOutcome::Changed => Ok(WriteOutcome::Updated),
        MeetingSyncOutcome::Unchanged => Ok(WriteOutcome::Unchanged),
    }
}
