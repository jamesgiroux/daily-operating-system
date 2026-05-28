//! Raw DB access for migrations and admin tooling.
//!
//! Only the v1.4.8 backfill migration, devtools, and demo seeds should call
//! these — production writers go through one of the four adapters. The
//! pre-commit grep guard ensures that.

use crate::db::{ActionDb, DbError, DbMeeting, EnsureMeetingHistoryInput, MeetingSyncOutcome};

/// Direct UPSERT of a full meeting record across the 3 tables. Bypasses
/// invariant checking; reserve for backfill migrations.
pub fn raw_upsert_meeting(db: &ActionDb, meeting: &DbMeeting) -> Result<(), DbError> {
    db.upsert_meeting(meeting)
}

/// Direct INSERT-or-update of meetings + stub child rows. Bypasses invariant
/// checking; reserve for migrations and demo seeds.
pub fn raw_ensure_meeting_in_history(
    db: &ActionDb,
    input: EnsureMeetingHistoryInput<'_>,
) -> Result<MeetingSyncOutcome, DbError> {
    db.ensure_meeting_in_history(input)
}
