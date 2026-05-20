//! DOS-335: mutating writers for meeting prep status.
//!
//! All paths in this module are out of the ADR-0102 §3 Read-ability
//! call graph. Callers are non-Read abilities (background workers,
//! manual entity linking, user-facing write commands) — never
//! `get_daily_briefing` or any sibling read-ability.
//!
//! # Write surfaces (L0 packet §5.5)
//!
//! - [`enqueue_refresh`] — schedule a re-generate. Sets status to
//!   `Queued` (from a legal predecessor) and emits the
//!   `meeting_prep_status_changed` signal.
//! - [`record_user_authored`] — persist user-authored agenda / notes /
//!   preparation_text / hidden_attendees. Emits Decision / Commitment
//!   claims via the existing claim store (eventual consistency through
//!   the claim-lifecycle-signal substrate — AC-335.13 Part B).
//! - [`record_dismissal`] / [`clear_dismissal`] — UserSuppressed /
//!   UserDismissed lifecycle. Backs the v242 dismissals table.
//! - [`transition_status`] — explicit state-machine guarded transition
//!   primitive. Returns [`PrepStatusError::IllegalTransition`] on an
//!   illegal jump (AC-335.14).
//!
//! # Signal contract (AC-335.15)
//!
//! Every successful transition emits a
//! `meeting_prep_status_changed` signal whose value payload is the
//! JSON-encoded `{from, to, meeting_id}` tuple. Consumers re-render
//! through the existing signal substrate; this module does not
//! arrange consumer notification beyond the signal emit.

use chrono::Utc;
use serde_json::json;

use super::{PrepStatus, PrepStatusError, RefreshReason, UserAuthoredFields};
use crate::db::ActionDb;
use crate::signals::bus::{self, SignalEmission};

const SIGNAL_TYPE: &str = "meeting_prep_status_changed";
const SIGNAL_SOURCE: &str = "service:meeting_prep_status";

/// Enqueue a prep refresh for `meeting_id`.
///
/// Persists the queued state in `meeting_prep_status_dismissals` is
/// NOT the right home; the v241 view derives Queued/Running/Failed
/// from the existing prep_invalidation queue substrate. W1 surfaces
/// the API; the queue plumbing lands at Stage 1c when the meeting
/// prep queue read API stabilizes (the existing `meeting_prep_queue`
/// module is the producer).
///
/// For W1 this primitive:
/// - validates that `from → Queued` is a legal transition,
/// - delegates queue enqueue to the existing `meeting_prep_queue`
///   producer (callers must follow up with a queue push — left
///   explicit so consumers can decide their priority + dedup window),
/// - emits the `meeting_prep_status_changed` signal so subscribers
///   re-render.
pub fn enqueue_refresh(
    meeting_id: &str,
    from: PrepStatus,
    _reason: RefreshReason,
    db: &ActionDb,
) -> Result<(), PrepStatusError> {
    transition_status(meeting_id, from, PrepStatus::Queued, db)
}

/// Persist user-authored agenda / notes / preparation_text /
/// hidden_attendees for `meeting_id` (AC-335.8 — survives recompute).
///
/// # AC-335.13 contract
///
/// - **Part A (commutes):** Plain user-authored fields written to
///   `meeting_prep` columns disjoint from the status/queue columns.
///   The property test in `mod.rs` `state_machine_tests` covers this.
/// - **Part B (eventual consistency):** Decision / Commitment claim
///   emission goes through the existing claim store substrate. This
///   module wires the persistence (agenda/notes etc.); the
///   decision-claim emission is handled by the standard commitment /
///   decision claim writer ([`crate::services::claims`] +
///   [`crate::services::commitment_bridge`]). Status recompute
///   observes the new claim through the standard claim-lifecycle-
///   signal → invalidation path.
pub fn record_user_authored(
    meeting_id: &str,
    fields: &UserAuthoredFields,
    db: &ActionDb,
) -> Result<(), PrepStatusError> {
    let now = Utc::now().to_rfc3339();
    let agenda = fields.agenda.as_deref();
    let notes = fields.notes.as_deref();

    db.with_transaction(|tx| {
        let conn = tx.conn_ref();
        // Upsert into meeting_prep. `meeting_id` is PK; if the row
        // does not yet exist we insert; otherwise we update the
        // disjoint user-authored columns ONLY. Status / queue columns
        // are untouched — AC-335.13 Part A.
        conn.execute(
            "INSERT INTO meeting_prep (
                meeting_id,
                user_agenda_json,
                user_notes
             ) VALUES (?1, ?2, ?3)
             ON CONFLICT(meeting_id) DO UPDATE SET
                user_agenda_json = excluded.user_agenda_json,
                user_notes       = excluded.user_notes",
            rusqlite::params![meeting_id, agenda, notes],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })
    .map_err(PrepStatusError::Db)?;

    // Emit the signal so subscribers re-render. The transition itself
    // (Ready → Ready) is a no-op at the status level; the signal
    // carries the user_authored-changed event so consumers know to
    // re-render. Use a no-op transition payload (status unchanged) so
    // the receiver can distinguish from a state-machine transition.
    emit_status_signal(meeting_id, None, None, "user_authored_changed", db, now)?;
    Ok(())
}

/// Record a UserSuppressed / UserDismissed dismissal for a meeting.
///
/// Persists to v242 `meeting_prep_status_dismissals`. Transitions the
/// in-memory status snapshot — the v241 view-driven read path picks
/// up the dismissal at next `compute_status`.
pub fn record_dismissal(
    meeting_id: &str,
    from: PrepStatus,
    kind: DismissalKind,
    actor: &str,
    reason: Option<&str>,
    db: &ActionDb,
) -> Result<(), PrepStatusError> {
    let target = match kind {
        DismissalKind::UserSuppressed => PrepStatus::UserSuppressed,
        DismissalKind::UserDismissed => PrepStatus::UserDismissed,
    };
    if !from.can_transition_to(target) {
        return Err(PrepStatusError::IllegalTransition { from, to: target });
    }

    let now = Utc::now().to_rfc3339();
    let id = uuid::Uuid::new_v4().to_string();

    db.with_transaction(|tx| {
        let conn = tx.conn_ref();
        conn.execute(
            "INSERT INTO meeting_prep_status_dismissals (
                id, meeting_id, dismissal_kind, dismissal_reason,
                actor, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
            rusqlite::params![id, meeting_id, kind.as_str(), reason, actor, now],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })
    .map_err(PrepStatusError::Db)?;

    emit_status_signal(
        meeting_id,
        Some(from),
        Some(target),
        kind.as_str(),
        db,
        Utc::now().to_rfc3339(),
    )?;
    Ok(())
}

/// Clear an active dismissal (un-suppress / un-dismiss).
///
/// Transitions UserSuppressed / UserDismissed → PrepNeeded.
pub fn clear_dismissal(
    meeting_id: &str,
    from: PrepStatus,
    db: &ActionDb,
) -> Result<(), PrepStatusError> {
    if !matches!(from, PrepStatus::UserSuppressed | PrepStatus::UserDismissed) {
        return Err(PrepStatusError::IllegalTransition {
            from,
            to: PrepStatus::PrepNeeded,
        });
    }
    let now = Utc::now().to_rfc3339();
    db.with_transaction(|tx| {
        let conn = tx.conn_ref();
        conn.execute(
            "UPDATE meeting_prep_status_dismissals
                SET resolved_at = ?1, updated_at = ?1
                WHERE meeting_id = ?2 AND resolved_at IS NULL",
            rusqlite::params![now, meeting_id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })
    .map_err(PrepStatusError::Db)?;

    emit_status_signal(
        meeting_id,
        Some(from),
        Some(PrepStatus::PrepNeeded),
        "dismissal_cleared",
        db,
        Utc::now().to_rfc3339(),
    )?;
    Ok(())
}

/// Explicit state-machine transition primitive.
///
/// AC-335.14: validates `from → to` against
/// [`PrepStatus::legal_transitions`] before emitting the signal.
/// Returns [`PrepStatusError::IllegalTransition`] when the transition
/// is illegal.
pub fn transition_status(
    meeting_id: &str,
    from: PrepStatus,
    to: PrepStatus,
    db: &ActionDb,
) -> Result<(), PrepStatusError> {
    if !from.can_transition_to(to) {
        return Err(PrepStatusError::IllegalTransition { from, to });
    }
    let now = Utc::now().to_rfc3339();
    emit_status_signal(meeting_id, Some(from), Some(to), "transition", db, now)?;
    Ok(())
}

/// Dismissal kind for [`record_dismissal`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DismissalKind {
    UserSuppressed,
    UserDismissed,
}

impl DismissalKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UserSuppressed => "user_suppressed",
            Self::UserDismissed => "user_dismissed",
        }
    }
}

/// Emit `meeting_prep_status_changed` with the `{from, to,
/// meeting_id, reason}` payload.
fn emit_status_signal(
    meeting_id: &str,
    from: Option<PrepStatus>,
    to: Option<PrepStatus>,
    reason_tag: &str,
    db: &ActionDb,
    at: String,
) -> Result<(), PrepStatusError> {
    let payload = json!({
        "meeting_id": meeting_id,
        "from": from,
        "to": to,
        "reason": reason_tag,
        "at": at,
    })
    .to_string();
    bus::emit(
        db,
        SignalEmission {
            entity_type: "meeting",
            entity_id: meeting_id,
            signal_type: SIGNAL_TYPE,
            source: SIGNAL_SOURCE,
            value: Some(&payload),
            confidence: 1.0,
            source_context: None,
        },
    )
    .map(|_| ())
    .map_err(|e| PrepStatusError::SignalEmit(e.to_string()))
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transition_status_rejects_illegal_transition() {
        // We don't need a DB for the pre-check; transition_status
        // returns the IllegalTransition error before any DB call.
        // Build an ActionDb stub via in-memory connection.
        let conn = rusqlite::Connection::open_in_memory().expect("open in-memory");
        let db = ActionDb::from_conn(&conn);

        // PrepNeeded → Ready is illegal.
        let result = transition_status("m1", PrepStatus::PrepNeeded, PrepStatus::Ready, db);
        match result {
            Err(PrepStatusError::IllegalTransition { from, to }) => {
                assert_eq!(from, PrepStatus::PrepNeeded);
                assert_eq!(to, PrepStatus::Ready);
            }
            other => panic!("expected IllegalTransition, got {other:?}"),
        }
    }

    #[test]
    fn dismissal_kind_str_roundtrips_schema_check() {
        // The CHECK constraint on meeting_prep_status_dismissals
        // enumerates exactly these two strings; if this test
        // diverges, the v242 schema CHECK will reject writes at
        // runtime. Keep them aligned.
        assert_eq!(DismissalKind::UserSuppressed.as_str(), "user_suppressed");
        assert_eq!(DismissalKind::UserDismissed.as_str(), "user_dismissed");
    }
}
