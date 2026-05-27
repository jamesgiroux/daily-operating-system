//! Meeting-row writer substrate (Lane B of v1.4.8 Meetings Substrate v2).
//!
//! Production code MUST route every `INSERT INTO meetings` / `UPDATE meetings
//! SET` through `write` or one of the source-specific helpers below. The
//! pre-commit grep at `.githooks/pre-commit` enforces this.
//!
//! The four adapters (`Calendar`, `Reconcile`, `Manual`, `Backfill`) each
//! declare an invariant set the row must satisfy — see `invariants.rs` for
//! the matrix. Validation happens before any DB call so the failure surface
//! is unambiguous and consistent.
//!
//! ## Why
//!
//! Prior to this substrate every write path had its own ad-hoc field set,
//! and "is end_time required?" / "is calendar_event_id required?" answers
//! diverged silently. The class of bugs that produced (briefing producer
//! treating personal blocks as customer meetings, schedule rendering
//! cancelled events, etc.) lives upstream of the read-side filter Lane A
//! installed.
//!
//! ## ADR cites
//!
//! - ADR-0102 (boundary heuristic): typed writer adapters are a service-layer
//!   concern, not a new ability.
//! - ADR-0101 Rule 4 (mutations through `services/`): every meeting-row
//!   mutation flows through this module.

pub mod backfill_adapter;
pub mod calendar_adapter;
pub mod invariants;
pub mod manual_adapter;
pub mod reconcile_adapter;
pub mod repository;
pub mod types;

#[cfg(test)]
mod integration_tests;

pub use types::{MeetingSource, WriteError, WriteOutcome, WriteRequest};

use crate::db::ActionDb;

/// Dispatch a write to the source-specific adapter.
pub fn write(db: &ActionDb, req: &WriteRequest) -> Result<WriteOutcome, WriteError> {
    match req.source {
        MeetingSource::Calendar => calendar_adapter::write(db, req),
        MeetingSource::Reconcile => reconcile_adapter::write(db, req),
        MeetingSource::Manual => manual_adapter::write(db, req),
        MeetingSource::Backfill => backfill_adapter::write(db, req),
    }
}
