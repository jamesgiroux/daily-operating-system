//! End-to-end adapter tests against a fresh in-memory database.
//!
//! Each test seeds a `WriteRequest`, routes it through the public `write`
//! dispatcher, and asserts the meeting + paired child rows materialize.

#![cfg(test)]

use rusqlite::params;

use super::types::{MeetingSource, WriteError, WriteOutcome, WriteRequest};
use crate::db::ActionDb;

fn fresh_db() -> ActionDb {
    let tempdir = tempfile::tempdir().expect("tempdir");
    ActionDb::open_at_unencrypted(tempdir.path().join("writer-test.db")).expect("open db")
}

fn calendar_request(id: &str) -> WriteRequest {
    WriteRequest {
        source: MeetingSource::Calendar,
        id: id.to_string(),
        title: "Customer sync".to_string(),
        meeting_type: "customer".to_string(),
        start_time: "2026-05-27T10:00:00+00:00".to_string(),
        end_time: Some("2026-05-27T10:30:00+00:00".to_string()),
        calendar_event_id: Some(format!("evt-{id}")),
        attendees: Some("[]".to_string()),
        description: None,
        notes_path: None,
        transcript_path: None,
        prep_context_json: None,
        user_agenda_json: None,
        user_notes: None,
        intelligence_state: None,
    }
}

#[test]
fn calendar_adapter_persists_meeting_and_child_rows() {
    let db = fresh_db();
    let req = calendar_request("meet-cal-1");
    let outcome = super::write(&db, &req).expect("calendar write");
    assert_eq!(outcome, WriteOutcome::Created);

    let count: i64 = db
        .conn_ref()
        .query_row(
            "SELECT COUNT(*) FROM meetings WHERE id = ?1",
            params!["meet-cal-1"],
            |row| row.get(0),
        )
        .expect("count meetings");
    assert_eq!(count, 1);

    let prep_count: i64 = db
        .conn_ref()
        .query_row(
            "SELECT COUNT(*) FROM meeting_prep WHERE meeting_id = ?1",
            params!["meet-cal-1"],
            |row| row.get(0),
        )
        .expect("count meeting_prep");
    assert_eq!(prep_count, 1);

    let transcript_count: i64 = db
        .conn_ref()
        .query_row(
            "SELECT COUNT(*) FROM meeting_transcripts WHERE meeting_id = ?1",
            params!["meet-cal-1"],
            |row| row.get(0),
        )
        .expect("count meeting_transcripts");
    assert_eq!(transcript_count, 1);
}

#[test]
fn calendar_adapter_rejects_personal_with_missing_calendar_event_id() {
    let db = fresh_db();
    let mut req = calendar_request("meet-cal-2");
    req.calendar_event_id = None;
    let err = super::write(&db, &req).expect_err("must fail");
    assert!(matches!(err, WriteError::MissingRequiredField { .. }));

    let count: i64 = db
        .conn_ref()
        .query_row(
            "SELECT COUNT(*) FROM meetings WHERE id = ?1",
            params!["meet-cal-2"],
            |row| row.get(0),
        )
        .expect("count meetings");
    assert_eq!(count, 0, "rejected write must not persist any row");
}

#[test]
fn manual_adapter_persists_without_calendar_event_id() {
    let db = fresh_db();
    let req = WriteRequest {
        source: MeetingSource::Manual,
        id: "meet-manual-1".to_string(),
        title: "1:1 with Alex".to_string(),
        meeting_type: "one_on_one".to_string(),
        start_time: "2026-05-27T14:00:00+00:00".to_string(),
        end_time: None,
        calendar_event_id: None,
        attendees: None,
        description: None,
        notes_path: None,
        transcript_path: None,
        prep_context_json: None,
        user_agenda_json: None,
        user_notes: None,
        intelligence_state: None,
    };
    let outcome = super::write(&db, &req).expect("manual write");
    assert_eq!(outcome, WriteOutcome::Created);

    let count: i64 = db
        .conn_ref()
        .query_row(
            "SELECT COUNT(*) FROM meetings WHERE id = ?1",
            params!["meet-manual-1"],
            |row| row.get(0),
        )
        .expect("count meetings");
    assert_eq!(count, 1);
}

#[test]
fn reconcile_adapter_accepts_legacy_local_format_start_time() {
    let db = fresh_db();
    let req = WriteRequest {
        source: MeetingSource::Reconcile,
        id: "meet-reconcile-1".to_string(),
        title: "Briefing-derived".to_string(),
        meeting_type: "internal".to_string(),
        start_time: "2026-05-27 10:00 AM".to_string(),
        end_time: None,
        calendar_event_id: None,
        attendees: None,
        description: None,
        notes_path: Some("meetings/2026-05-27.md".to_string()),
        transcript_path: None,
        prep_context_json: None,
        user_agenda_json: None,
        user_notes: None,
        intelligence_state: None,
    };
    let outcome = super::write(&db, &req).expect("reconcile write");
    assert_eq!(outcome, WriteOutcome::Created);
}

#[test]
fn backfill_adapter_defaults_intelligence_state_to_archived() {
    let db = fresh_db();
    let req = WriteRequest {
        source: MeetingSource::Backfill,
        id: "meet-backfill-1".to_string(),
        title: "Historical note".to_string(),
        meeting_type: "customer".to_string(),
        start_time: "2024-01-15T12:00:00+00:00".to_string(),
        end_time: None,
        calendar_event_id: None,
        attendees: None,
        description: None,
        notes_path: Some("/notes/2024-01-15.md".to_string()),
        transcript_path: None,
        prep_context_json: None,
        user_agenda_json: None,
        user_notes: None,
        intelligence_state: None,
    };
    super::write(&db, &req).expect("backfill write");

    let state: Option<String> = db
        .conn_ref()
        .query_row(
            "SELECT intelligence_state FROM meeting_transcripts WHERE meeting_id = ?1",
            params!["meet-backfill-1"],
            |row| row.get(0),
        )
        .expect("read intelligence_state");
    assert_eq!(state.as_deref(), Some("archived"));
}

#[test]
fn calendar_adapter_unchanged_outcome_when_rewriting_same_row() {
    let db = fresh_db();
    let req = calendar_request("meet-cal-unchanged");
    super::write(&db, &req).expect("first write");
    let outcome = super::write(&db, &req).expect("second write");
    assert_eq!(outcome, WriteOutcome::Unchanged);
}
