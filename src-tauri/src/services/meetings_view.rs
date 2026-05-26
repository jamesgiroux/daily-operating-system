//! Surface-relevant meetings projection. One service-owned function that
//! resolves the user's local-day window, runs the meetings query, and applies
//! a caller-declared intent filter so consumers don't see types they wouldn't
//! render.
//!
//! Lane A of v1.4.8 (Meetings Substrate v2) closes DOS-771: the previous read
//! path returned every row in the date range, including `meeting_type =
//! 'personal'`, which the briefing producer then counted toward "needs prep"
//! advisories on calendar days with zero customer meetings.
//!
//! ## ADR cites
//!
//! - **ADR-0102** (service-layer boundary heuristic): a trivial projection
//!   that filters rows by caller intent is service-layer, not a new ability.
//! - **ADR-0101 Rule 5** (`read_*` functions don't write): pure projection,
//!   no derived state or claim mutation.
//! - **ADR-0111** (workspace-scoped reads): every row carries
//!   `workspace_scope`; callers tag the rendered surface they're seeding.
//!
//! ## Two-implementations note
//!
//! `focus_capacity::should_exclude_meeting` is the semantic twin used by the
//! dashboard / executive intelligence path. Lane A leaves that filter in
//! place; the dashboard/entities migration is a follow-up under DOS-773.

use chrono::NaiveDate;
use chrono_tz::Tz;

use crate::helpers::today_meeting_filter_for_date;

// Post-projection row: type-filter declared by `MeetingsViewIntent` has
// already been applied. A `SurfaceMeeting` returned from `read_surface_meetings`
// with `Briefing` intent will never carry `meeting_type = 'personal'`. The
// alias is intentional for smallest-diff in Lane A; future iterations may
// wrap it in a newtype with a phantom intent tag so the projection invariant
// is compile-time.
pub use abilities_runtime::services::context::{
    DailyReadinessMeetingSnapshot as SurfaceMeeting, MeetingsViewIntent,
};

/// Surface-relevant meetings for one local day, filtered to the rows the
/// caller's intent will render. The transcript-archive predicate (cancelled
/// in calendar) always applies; the type predicate is intent-driven.
pub fn read_surface_meetings(
    db: &crate::db::ActionDb,
    workspace_scope: &str,
    date: NaiveDate,
    tz: &Tz,
    intent: MeetingsViewIntent,
) -> Result<Vec<SurfaceMeeting>, String> {
    let window = today_meeting_filter_for_date(date, tz);
    let conn = db.conn_ref();
    let mut stmt = conn
        .prepare(
            "SELECT m.id, m.title, m.start_time, m.end_time, m.meeting_type
             FROM meetings m
             LEFT JOIN meeting_transcripts mt ON mt.meeting_id = m.id
             WHERE m.start_time >= ?1 AND m.start_time < ?2
             AND (mt.intelligence_state IS NULL OR mt.intelligence_state != 'archived')
             ORDER BY m.start_time ASC",
        )
        .map_err(|error| error.to_string())?;

    let rows = stmt
        .query_map(rusqlite::params![window.utc_start, window.utc_end], |row| {
            let meeting_type: String = row.get(4)?;
            Ok((
                SurfaceMeeting {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    starts_at: row.get(2)?,
                    ends_at: row.get(3)?,
                    workspace_scope: workspace_scope.to_string(),
                },
                meeting_type,
            ))
        })
        .map_err(|error| error.to_string())?;

    let mut out = Vec::new();
    for row in rows {
        let (meeting, meeting_type) = row.map_err(|error| error.to_string())?;
        if !intent_includes(intent, &meeting_type) {
            continue;
        }
        out.push(meeting);
    }
    Ok(out)
}

fn intent_includes(intent: MeetingsViewIntent, meeting_type: &str) -> bool {
    match intent {
        MeetingsViewIntent::Briefing | MeetingsViewIntent::Schedule => meeting_type != "personal",
        MeetingsViewIntent::AllRows => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn briefing_excludes_personal() {
        assert!(!intent_includes(MeetingsViewIntent::Briefing, "personal"));
        assert!(intent_includes(MeetingsViewIntent::Briefing, "customer"));
        assert!(intent_includes(MeetingsViewIntent::Briefing, "internal"));
    }

    #[test]
    fn schedule_excludes_personal() {
        assert!(!intent_includes(MeetingsViewIntent::Schedule, "personal"));
        assert!(intent_includes(MeetingsViewIntent::Schedule, "customer"));
    }

    #[test]
    fn all_rows_includes_everything() {
        assert!(intent_includes(MeetingsViewIntent::AllRows, "personal"));
        assert!(intent_includes(MeetingsViewIntent::AllRows, "customer"));
        assert!(intent_includes(MeetingsViewIntent::AllRows, "internal"));
    }
}
