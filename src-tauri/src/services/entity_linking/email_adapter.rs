//! Email adapter — DbEmail + thread → LinkingContext.
//!
//! Responsible for Phase 2 fact-recording on the email surface:
//! mapping From/To/Cc participants and resolving the thread for P2 inheritance.
//! The adapter does NOT run linking logic; it only builds the context
//! that evaluate_phases() consumes.

use crate::db::ActionDb;
use crate::db::types::DbEmail;

use super::types::LinkingContext;

/// Convert an email (and its thread context) into a LinkingContext for evaluate().
///
/// Reads the current graph_version from entity_graph_version (O(1)).
/// Does not write anything to the DB.
pub fn build_context(
    _email: &DbEmail,
    _thread_primary_entity_id: Option<&str>,
    _db: &ActionDb,
) -> Result<LinkingContext, String> {
    // TODO(Lane-E): implement email → LinkingContext mapping.
    unimplemented!("Lane E: email_adapter::build_context")
}
