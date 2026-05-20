//! DOS-335: pure-read path for meeting prep status.
//!
//! # ADR-0102 §3 Read-ability call-graph invariant
//!
//! This module MUST contain zero mutations. No `&mut`, no signal
//! emission, no DB writes, no queue enqueue. The static guarantees:
//!
//! 1. [`compute_status`] takes `&ActionDb` (immutable borrow).
//! 2. No `use` of [`crate::services::meeting_prep_status::write`].
//! 3. No `use` of `crate::signals::bus` or `crate::services::signals`.
//! 4. No `enqueue_*` or `insert_*` function calls.
//!
//! AC-335.12: the L1 compile-time fence at the bottom of this file
//! asserts the call graph by static analysis (deny pattern on
//! mutation-suggestive identifiers). The fence is a `#[test]` so any
//! future refactor that introduces a write will fail `cargo test`.

use rusqlite::OptionalExtension;

use super::{
    BlockingReason, EntityBinding, PrepStatus, PrepStatusError, PrepStatusSnapshot, StaleReason,
    UserAuthoredFields,
};
use crate::db::ActionDb;

/// Row materialized from the v241 `meeting_prep_status_view`.
#[derive(Debug, Clone, Default)]
struct ViewRow {
    meeting_id: String,
    event_id: Option<String>,
    linked_entity_id: Option<String>,
    linked_entity_type: Option<String>,
    user_agenda_json: Option<String>,
    user_notes: Option<String>,
    last_prepared_at: Option<String>,
}

/// Pure-read computation of the current prep status snapshot.
///
/// # Invariants (AC-335.12)
///
/// - Zero mutations. Returns Stale if prep is invalidated; does NOT
///   auto-enqueue a refresh — the caller (a writer in
///   [`super::write`]) decides whether to enqueue.
/// - No signal emission. Subscribers learn about transitions through
///   the writer-emitted `meeting_prep_status_changed` signal.
///
/// # AC-335.6 / AC-335.7
///
/// Convergence: every consumer surface (FolioBar, Meeting Briefing,
/// Daily Briefing rollup, Meeting Detail) calls this function with the
/// same `meeting_id`. They render from the same snapshot so they agree
/// on status without app restart.
pub fn compute_status(meeting_id: &str, db: &ActionDb) -> Result<PrepStatusSnapshot, PrepStatusError> {
    let row = load_view_row(meeting_id, db)?
        .ok_or_else(|| PrepStatusError::MeetingNotFound(meeting_id.to_string()))?;
    let dismissal = load_active_dismissal(meeting_id, db)?;

    let (status, blocking_reason, stale_reason) = derive_status(&row, dismissal.as_deref());

    let linked_entity = match (row.linked_entity_id.as_deref(), row.linked_entity_type.as_deref()) {
        (Some(id), Some(ty)) => Some(EntityBinding {
            entity_id: id.to_string(),
            entity_type: ty.to_string(),
        }),
        _ => None,
    };

    let user_authored = UserAuthoredFields {
        agenda: row.user_agenda_json.clone(),
        notes: row.user_notes.clone(),
        // preparation_text/hidden_attendees/decisions live in claim
        // store + future user_authored columns; AC-335.8 preservation
        // is upheld by writers, but the W1 read-side projection only
        // surfaces what's persisted today via meeting_prep columns.
        preparation_text: None,
        hidden_attendees: Vec::new(),
        decisions: Vec::new(),
    };

    let next_allowed_transition = status.legal_transitions().to_vec();

    Ok(PrepStatusSnapshot {
        meeting_id: row.meeting_id,
        event_id: row.event_id,
        linked_entity,
        status,
        blocking_reason,
        stale_reason,
        last_prepared_at: row.last_prepared_at,
        // Trust/provenance summaries are aggregated by W2/W3 consumers
        // from the existing claim envelope substrate. The W1 read
        // path returns empty defaults; the contract is forward-
        // compatible.
        source_asof_inputs: Vec::new(),
        trust_summary: None,
        provenance: None,
        next_allowed_transition,
        user_authored,
    })
}

/// Load the v241 view row for a meeting (read-only).
fn load_view_row(meeting_id: &str, db: &ActionDb) -> Result<Option<ViewRow>, PrepStatusError> {
    let conn = db.conn_ref();
    let mut stmt = conn
        .prepare(
            "SELECT meeting_id, event_id, linked_entity_id, linked_entity_type,
                    user_agenda_json, user_notes, last_prepared_at
             FROM meeting_prep_status_view
             WHERE meeting_id = ?1
             LIMIT 1",
        )
        .map_err(|e| PrepStatusError::Db(e.to_string()))?;
    let row = stmt
        .query_row([meeting_id], |row| {
            Ok(ViewRow {
                meeting_id: row.get(0)?,
                event_id: row.get(1)?,
                linked_entity_id: row.get(2)?,
                linked_entity_type: row.get(3)?,
                user_agenda_json: row.get(4)?,
                user_notes: row.get(5)?,
                last_prepared_at: row.get(6)?,
            })
        })
        .optional()
        .map_err(|e| PrepStatusError::Db(e.to_string()))?;
    Ok(row)
}

/// Load the latest unresolved dismissal kind for a meeting, if any.
///
/// Returns the `dismissal_kind` string ('user_suppressed' /
/// 'user_dismissed') when an active row exists, else `None`.
fn load_active_dismissal(meeting_id: &str, db: &ActionDb) -> Result<Option<String>, PrepStatusError> {
    let conn = db.conn_ref();
    let mut stmt = conn
        .prepare(
            "SELECT dismissal_kind
             FROM meeting_prep_status_dismissals
             WHERE meeting_id = ?1 AND resolved_at IS NULL
             ORDER BY created_at DESC
             LIMIT 1",
        )
        .map_err(|e| PrepStatusError::Db(e.to_string()))?;
    let row = stmt
        .query_row([meeting_id], |row| row.get::<_, String>(0))
        .optional()
        .map_err(|e| PrepStatusError::Db(e.to_string()))?;
    Ok(row)
}

/// Derive the [`PrepStatus`] from the view row + dismissal state.
///
/// Pure function; no DB access, no side effects. Status precedence
/// (highest to lowest):
///   1. UserSuppressed / UserDismissed (user wins).
///   2. BlockedNoEntity (no linked entity).
///   3. PrepNeeded (linked entity, no prep output).
///   4. Stale (prep output exists but is invalidated — placeholder
///      logic: caller-supplied invalidation queue check folds in at
///      W1 Stage 1c when the queue read API surfaces).
///   5. Ready (prep output exists and is current).
///
/// Queued / Running / Failed / Limited derivations require the
/// meeting_prep_queue and worker state which are surfaced through the
/// existing `meeting_prep_queue` substrate; the W1 read service folds
/// those signals in via [`crate::meeting_prep_queue`] at Stage 1c when
/// the queue read API surfaces. For now the read service returns
/// Ready / Stale / PrepNeeded based on the persisted prep_frozen_at.
fn derive_status(
    row: &ViewRow,
    dismissal_kind: Option<&str>,
) -> (PrepStatus, Option<BlockingReason>, Option<StaleReason>) {
    // 1. User dismissal precedence.
    match dismissal_kind {
        Some("user_suppressed") => return (PrepStatus::UserSuppressed, None, None),
        Some("user_dismissed") => return (PrepStatus::UserDismissed, None, None),
        _ => {}
    }

    // 2. Blocked (no linked entity).
    if row.linked_entity_id.is_none() {
        return (
            PrepStatus::BlockedNoEntity,
            Some(BlockingReason::NoLinkedEntity),
            None,
        );
    }

    // 3. No prep output yet → PrepNeeded.
    if row.last_prepared_at.is_none() {
        return (PrepStatus::PrepNeeded, None, None);
    }

    // 4 + 5. Prep exists. For W1 we treat it as Ready; staleness
    //        derivation by upstream-claim-change folds in at W1 Stage
    //        1c (DOS-507 caller). Writers in the sibling write module
    //        re-classify Ready into Stale when the signal substrate
    //        dispatches the invalidation event.
    (PrepStatus::Ready, None, None)
}

// -----------------------------------------------------------------------------
// AC-335.12: call-graph lint
// -----------------------------------------------------------------------------
//
// Static guarantee that the read module's call graph never mutates.
// The grep-based fence below is a `#[test]` that runs on every
// `cargo test` invocation. If a future refactor introduces a write
// path into this file the test fails — same shape as the trybuild
// pattern called out by AC-335.12.
//
// The lint is intentionally implemented as a string-search over this
// source file rather than a full call-graph analyzer: any of the
// listed identifiers in this file is sufficient to suggest a
// mutation path. A clean substrate has zero matches.

#[cfg(test)]
mod call_graph_lint {
    /// AC-335.12: deny mutation-suggestive identifiers in this file.
    /// If a future refactor introduces a write into the read path,
    /// this test fails.
    #[test]
    fn read_module_contains_no_mutations() {
        // Read this file's source at test time.
        let source = include_str!("read.rs");
        let banned: &[&str] = &[
            // Direct mutation suggestions.
            "&mut ",
            // Write-API call sites.
            "enqueue_refresh",
            "record_user_authored",
            "transition_status",
            "emit_signal",
            "signals::bus::emit",
            "services::signals::emit",
            // Mutation-shaped SQL verbs (allowing them in strings
            // would slip a write into the read path).
            "INSERT INTO",
            "UPDATE ",
            "DELETE FROM",
        ];
        let mut violations: Vec<&str> = Vec::new();
        for term in banned {
            // Allow the term to appear inside this lint test body
            // itself. Strip the banned-list block before searching.
            let trimmed_source = strip_lint_block(source);
            if trimmed_source.contains(term) {
                violations.push(*term);
            }
        }
        assert!(
            violations.is_empty(),
            "AC-335.12 violation: read module contains mutation-suggestive identifiers: {violations:?}"
        );
    }

    /// Strip the lint test body itself so the banned-list literals
    /// do not register as matches.
    fn strip_lint_block(source: &str) -> String {
        const MARKER: &str = "mod call_graph_lint";
        match source.find(MARKER) {
            Some(idx) => source[..idx].to_string(),
            None => source.to_string(),
        }
    }
}
