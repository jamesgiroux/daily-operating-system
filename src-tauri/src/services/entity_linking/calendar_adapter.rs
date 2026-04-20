//! Calendar adapter — GoogleCalendarEvent → LinkingContext.
//!
//! Responsible for Phase 2 fact-recording on the calendar surface:
//! mapping attendees to Participant structs and resolving graph_version.
//! The adapter does NOT run linking logic; it only builds the context
//! that evaluate_phases() consumes.

use crate::db::ActionDb;
use crate::types::CalendarEvent;

use super::types::LinkingContext;

/// Convert a calendar event into a LinkingContext for evaluate().
///
/// Reads the current graph_version from entity_graph_version (O(1)).
/// Does not write anything to the DB.
pub fn build_context(_event: &CalendarEvent, _db: &ActionDb) -> Result<LinkingContext, String> {
    // TODO(Lane-D): implement calendar → LinkingContext mapping.
    unimplemented!("Lane D: calendar_adapter::build_context")
}
