//! Historical filesystem backfill adapter. Used by `backfill_meetings.rs` to
//! reconstruct meeting rows from notes files / prep snapshots discovered on
//! disk.
//!
//! Default `intelligence_state = 'archived'` since the source material is
//! already settled (not part of the live calendar/prep pipeline).

use crate::db::{ActionDb, DbMeeting};

use super::invariants;
use super::types::{MeetingSource, WriteError, WriteOutcome, WriteRequest};

pub fn write(db: &ActionDb, req: &WriteRequest) -> Result<WriteOutcome, WriteError> {
    debug_assert_eq!(req.source, MeetingSource::Backfill);
    invariants::validate(req)?;

    let existed = db.get_meeting_by_id(&req.id).map_err(WriteError::from)?.is_some();
    let meeting = build_db_meeting(req);
    db.upsert_meeting(&meeting)?;

    // upsert_meeting seeds meeting_transcripts with intelligence_state's
    // default value ('detected'). Backfilled rows come from settled source
    // material (notes / prep files on disk) so their lifecycle position is
    // 'archived' — overwrite explicitly unless the caller passed a value.
    let intelligence_state = req.intelligence_state.as_deref().unwrap_or("archived");
    db.conn_ref()
        .execute(
            "UPDATE meeting_transcripts SET intelligence_state = ?1 WHERE meeting_id = ?2",
            rusqlite::params![intelligence_state, &req.id],
        )
        .map_err(|e| WriteError::Database(e.to_string()))?;

    Ok(if existed {
        WriteOutcome::Updated
    } else {
        WriteOutcome::Created
    })
}

fn build_db_meeting(req: &WriteRequest) -> DbMeeting {
    let intelligence_state = req
        .intelligence_state
        .clone()
        .or_else(|| Some("archived".to_string()));
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
        intelligence_state,
        intelligence_quality: None,
        last_enriched_at: None,
        signal_count: None,
        has_new_signals: None,
        last_viewed_at: None,
    }
}
