//! Workspace reconcile adapter. Used by `workflow::reconcile::persist_meetings`
//! which materializes meetings from briefing markdown + prep snapshots into
//! the meetings/meeting_prep/meeting_transcripts tables.
//!
//! Routes to `db::upsert_meeting` because reconcile owns full UPSERT semantics
//! (overwrites all attribute columns), unlike the calendar / manual path which
//! only touch attributes when changes are detected.

use crate::db::{ActionDb, DbMeeting};

use super::invariants;
use super::types::{MeetingSource, WriteError, WriteOutcome, WriteRequest};

pub fn write(db: &ActionDb, req: &WriteRequest) -> Result<WriteOutcome, WriteError> {
    debug_assert_eq!(req.source, MeetingSource::Reconcile);
    invariants::validate(req)?;

    let existed = db.get_meeting_by_id(&req.id).map_err(WriteError::from)?.is_some();
    let meeting = build_db_meeting(req);
    db.upsert_meeting(&meeting)?;

    Ok(if existed {
        WriteOutcome::Updated
    } else {
        WriteOutcome::Created
    })
}

fn build_db_meeting(req: &WriteRequest) -> DbMeeting {
    DbMeeting {
        id: req.id.clone(),
        title: req.title.clone(),
        meeting_type: req.meeting_type.clone(),
        start_time: req.start_time.clone(),
        end_time: req.end_time.clone(),
        attendees: req.attendees.clone(),
        notes_path: req.notes_path.clone(),
        summary: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        calendar_event_id: req.calendar_event_id.clone(),
        description: req.description.clone(),
        prep_context_json: req.prep_context_json.clone(),
        user_agenda_json: req.user_agenda_json.clone(),
        user_notes: req.user_notes.clone(),
        prep_frozen_json: None,
        prep_frozen_at: None,
        prep_snapshot_path: None,
        prep_snapshot_hash: None,
        transcript_path: req.transcript_path.clone(),
        transcript_processed_at: None,
        intelligence_state: req.intelligence_state.clone(),
        intelligence_quality: None,
        last_enriched_at: None,
        signal_count: None,
        has_new_signals: None,
        last_viewed_at: None,
    }
}
