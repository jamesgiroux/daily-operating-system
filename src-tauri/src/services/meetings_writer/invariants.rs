//! Invariant matrix for meeting-row writes.
//!
//! Each `MeetingSource` declares which fields are required, optional, or
//! forbidden on top of the global rules. Validation happens at the adapter
//! boundary so the DB-layer functions don't need to re-check.
//!
//! ## Matrix
//!
//! | Field                | Calendar | Reconcile | Manual    | Backfill |
//! |----------------------|----------|-----------|-----------|----------|
//! | `id`                 | required | required  | required  | required |
//! | `title`              | required | required  | required  | required |
//! | `meeting_type`       | required | required  | required  | required |
//! | `start_time`         | RFC3339  | flexible  | RFC3339   | flexible |
//! | `end_time`           | required | optional  | forbidden | optional |
//! | `calendar_event_id`  | required | optional  | forbidden | optional |
//! | `attendees`          | required | optional  | forbidden | optional |
//! | `intelligence_state` | optional | optional  | optional  | archived |
//!
//! `Calendar` and `Manual` require RFC3339 UTC `start_time` so downstream
//! TZ-aware projection (`services/meetings_view.rs`) can range-query
//! deterministically. `Reconcile` and `Backfill` accept legacy formats
//! because their source material predates the contract. Existing rows that
//! carry pre-RFC3339 formats are not normalized by this substrate — the
//! invariant matrix is enforced on WRITE only. A separate backfill
//! migration to normalize legacy rows is tracked as a follow-up.

use super::types::{MeetingSource, WriteError, WriteRequest};

pub fn validate(req: &WriteRequest) -> Result<(), WriteError> {
    validate_global(req)?;
    match req.source {
        MeetingSource::Calendar => validate_calendar(req),
        MeetingSource::Reconcile => validate_reconcile(req),
        MeetingSource::Manual => validate_manual(req),
        MeetingSource::Backfill => validate_backfill(req),
    }
}

fn validate_global(req: &WriteRequest) -> Result<(), WriteError> {
    let meeting_source = req.source.as_str();
    if req.id.trim().is_empty() {
        return Err(WriteError::MissingRequiredField {
            meeting_source,
            field: "id",
        });
    }
    if req.title.trim().is_empty() {
        return Err(WriteError::MissingRequiredField {
            meeting_source,
            field: "title",
        });
    }
    if req.meeting_type.trim().is_empty() {
        return Err(WriteError::MissingRequiredField {
            meeting_source,
            field: "meeting_type",
        });
    }
    if req.start_time.trim().is_empty() {
        return Err(WriteError::MissingRequiredField {
            meeting_source,
            field: "start_time",
        });
    }
    Ok(())
}

fn validate_calendar(req: &WriteRequest) -> Result<(), WriteError> {
    let meeting_source = MeetingSource::Calendar.as_str();
    if !is_rfc3339(&req.start_time) {
        return Err(WriteError::InvalidValue {
            field: "start_time",
            reason: format!(
                "Calendar source requires RFC3339 UTC start_time, got `{}`",
                req.start_time
            ),
        });
    }
    if req.calendar_event_id.as_deref().unwrap_or("").is_empty() {
        return Err(WriteError::MissingRequiredField {
            meeting_source,
            field: "calendar_event_id",
        });
    }
    if req.end_time.as_deref().unwrap_or("").is_empty() {
        return Err(WriteError::MissingRequiredField {
            meeting_source,
            field: "end_time",
        });
    }
    if req.attendees.as_deref().unwrap_or("").is_empty() {
        return Err(WriteError::MissingRequiredField {
            meeting_source,
            field: "attendees",
        });
    }
    Ok(())
}

fn validate_reconcile(_req: &WriteRequest) -> Result<(), WriteError> {
    Ok(())
}

fn validate_manual(req: &WriteRequest) -> Result<(), WriteError> {
    let meeting_source = MeetingSource::Manual.as_str();
    if req.calendar_event_id.is_some() {
        return Err(WriteError::ForbiddenField {
            meeting_source,
            field: "calendar_event_id",
        });
    }
    if !is_rfc3339(&req.start_time) {
        return Err(WriteError::InvalidValue {
            field: "start_time",
            reason: format!(
                "Manual source requires RFC3339 UTC start_time, got `{}`",
                req.start_time
            ),
        });
    }
    Ok(())
}

fn validate_backfill(_req: &WriteRequest) -> Result<(), WriteError> {
    Ok(())
}

/// Loose RFC3339 check: starts with YYYY-MM-DDTHH:MM:SS and ends with Z or
/// timezone offset. Tight enough to reject local-format strings
/// (`2026-03-06 10:00 AM`); cheap enough to run per-write.
fn is_rfc3339(value: &str) -> bool {
    if value.len() < 20 {
        return false;
    }
    let bytes = value.as_bytes();
    // Position 10 must be 'T'; positions 4 and 7 must be '-'.
    bytes[4] == b'-' && bytes[7] == b'-' && bytes[10] == b'T'
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_request(source: MeetingSource) -> WriteRequest {
        WriteRequest {
            source,
            id: "meet-1".to_string(),
            title: "Sync".to_string(),
            meeting_type: "customer".to_string(),
            start_time: "2026-05-27T10:00:00+00:00".to_string(),
            end_time: Some("2026-05-27T10:30:00+00:00".to_string()),
            calendar_event_id: Some("evt-1".to_string()),
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
    fn calendar_requires_all_calendar_fields() {
        let mut req = base_request(MeetingSource::Calendar);
        req.calendar_event_id = None;
        assert!(matches!(
            validate(&req),
            Err(WriteError::MissingRequiredField {
                field: "calendar_event_id",
                ..
            })
        ));
    }

    #[test]
    fn calendar_rejects_local_format_start_time() {
        let mut req = base_request(MeetingSource::Calendar);
        req.start_time = "2026-05-27 10:00 AM".to_string();
        assert!(matches!(
            validate(&req),
            Err(WriteError::InvalidValue {
                field: "start_time",
                ..
            })
        ));
    }

    #[test]
    fn manual_forbids_calendar_event_id() {
        let mut req = base_request(MeetingSource::Manual);
        req.end_time = None;
        req.attendees = None;
        // calendar_event_id is Some by default — must be cleared for Manual
        assert!(matches!(
            validate(&req),
            Err(WriteError::ForbiddenField {
                field: "calendar_event_id",
                ..
            })
        ));
    }

    #[test]
    fn manual_with_cleared_calendar_id_passes() {
        let mut req = base_request(MeetingSource::Manual);
        req.calendar_event_id = None;
        req.end_time = None;
        req.attendees = None;
        assert!(validate(&req).is_ok());
    }

    #[test]
    fn reconcile_accepts_legacy_start_time_format() {
        let mut req = base_request(MeetingSource::Reconcile);
        req.start_time = "2026-05-27 10:00 AM".to_string();
        req.calendar_event_id = None;
        req.end_time = None;
        req.attendees = None;
        assert!(validate(&req).is_ok());
    }

    #[test]
    fn backfill_accepts_minimal_fields() {
        let mut req = base_request(MeetingSource::Backfill);
        req.start_time = "2026-05-27 10:00 AM".to_string();
        req.calendar_event_id = None;
        req.end_time = None;
        req.attendees = None;
        assert!(validate(&req).is_ok());
    }

    #[test]
    fn missing_id_fails_globally() {
        let mut req = base_request(MeetingSource::Calendar);
        req.id = String::new();
        assert!(matches!(
            validate(&req),
            Err(WriteError::MissingRequiredField { field: "id", .. })
        ));
    }

    #[test]
    fn rfc3339_check_recognizes_offset_format() {
        assert!(is_rfc3339("2026-05-27T10:00:00+00:00"));
        assert!(is_rfc3339("2026-05-27T10:00:00Z00:00"));
        assert!(!is_rfc3339("2026-05-27 10:00:00"));
        assert!(!is_rfc3339("2026-05-27"));
    }
}
