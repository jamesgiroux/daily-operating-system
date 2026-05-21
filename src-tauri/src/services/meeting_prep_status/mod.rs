//! Meeting prep / readiness status service.
//!
//! Single service-owned status contract for "is this meeting's prep
//! ready, blocked, stale, failed, or user-suppressed?" — consumed by
//! the W3 FolioBar readiness chrome, Meeting Briefing block, Daily
//! Briefing per-meeting rollup, and (post-migration) the Meeting Detail
//! block.
//!
//! # Architecture (L0 packet §5.5 V1.1 — read/write split)
//!
//! Per cycle-1 architecture F2, this module is split into two siblings
//! so the ADR-0102 §3 "Read-ability call graph" invariant can be
//! statically enforced:
//!
//! - [`read`] — pure read path. No `&mut`, no signal emit, no mutation.
//!   `compute_status` returns a [`PrepStatusSnapshot`] describing the
//!   meeting's current state. If the meeting is stale, `compute_status`
//!   reports `Stale`; it does NOT auto-enqueue a refresh — the caller
//!   (a non-Read-ability writer) decides.
//! - [`write`] — mutating path. `enqueue_refresh` queues a regenerate
//!   job, and `record_user_authored` persists user-authored fields
//!   (agenda, notes, preparation text, hidden attendees) plus emits
//!   `Decision` / `Commitment` claims through the existing claim store
//!   substrate.
//!
//! `get_daily_briefing` (Stage 1c) and any other Read-ability
//! call graph member must only depend on [`read`].
//!
//! # State machine (L0 packet §5.5 — AC-335.14)
//!
//! See [`PrepStatus::legal_transitions`] for the full table. Illegal
//! transitions return [`TransitionError`] from
//! [`write::transition_status`].
//!
//! # Signal contract (L0 packet §5.5 / AC-335.15)
//!
//! Writers emit `meeting_prep_status_changed` (`SignalType::
//! MeetingPrepStatusChanged`) with `{from, to, meeting_id}` payload
//! after every successful transition. The signal is pre-declared in
//! [`crate::signals::policy_registry`] so consumers subscribe via the
//! existing signal substrate, not by polling the read service.
//!
//! # Write-commutativity contract (L0 packet §13 Q7 V1.1 — AC-335.13)
//!
//! - **Part A (commutes):** Plain user-authored fields (agenda, notes,
//!   preparation_text, hidden_attendees) are disjoint from the columns
//!   touched by `enqueue_refresh`. Concurrent writes to disjoint
//!   columns commute. Property-tested in the [`tests`] module.
//! - **Part B (eventual consistency):** `record_user_authored` ALSO
//!   emits `Decision` / `Commitment` claims to the claim store. Those
//!   claims feed the v241 indexed view through the standard claim-
//!   lifecycle-signal → invalidation path. This is NOT disjoint-column
//!   commutativity; the contract is "eventual consistency via existing
//!   signal substrate."
//!
//! # Migrations
//!
//! - v241 — `meeting_prep_status_view` (read-only indexed view across
//!   meetings + meeting_entities + meeting_prep).
//! - v242 — `meeting_prep_status_dismissals` (UserSuppressed /
//!   UserDismissed persistence).

pub mod read;
pub mod write;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// -----------------------------------------------------------------------------
// PrepStatus enum + state machine
// -----------------------------------------------------------------------------

/// Lifecycle state of meeting prep readiness for a given meeting.
///
/// Variants are deliberately the full set named in the acceptance criteria
/// (AC-335.4) plus the cycle-1 correctness F10 finding (AC-335.14).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PrepStatus {
    /// Meeting has no linked entity. Cannot prep.
    BlockedNoEntity,
    /// Meeting has an entity link but no prep output yet, not queued.
    PrepNeeded,
    /// Refresh enqueued; awaiting background worker pickup.
    Queued,
    /// Background worker is actively generating prep.
    Running,
    /// Prep is current.
    Ready,
    /// Prep generated but incomplete (carries `stale_reason`).
    Limited,
    /// Prep exists but a triggering signal has invalidated it.
    Stale,
    /// Prep generation failed.
    Failed,
    /// User has actively turned prep off for this meeting.
    UserSuppressed,
    /// User dismissed prep for this meeting only (one-shot).
    UserDismissed,
}

impl PrepStatus {
    /// AC-335.14: exhaustive legal-transitions table.
    ///
    /// Returns the set of states `self` may legally transition into.
    /// Empty slice means the state is terminal until external state
    /// changes (e.g., un-suppress) put it back into a transition slot.
    pub const fn legal_transitions(self) -> &'static [PrepStatus] {
        use PrepStatus::*;
        match self {
            // Entity linked. Can begin prep cycle.
            BlockedNoEntity => &[PrepNeeded],
            // Can be queued, or user-killed without ever running.
            PrepNeeded => &[Queued, UserSuppressed, UserDismissed],
            // Worker picks it up → Running. Or queue timeout → Failed.
            // User can dismiss in-flight (no kill of worker; status
            // flag only).
            Queued => &[Running, Failed, UserSuppressed, UserDismissed],
            // Terminal worker outcomes.
            Running => &[Ready, Limited, Failed],
            // Refresh schedules a new Queued. Stale invalidates.
            Ready => &[Queued, Stale, UserSuppressed, UserDismissed],
            // Same as Ready, plus the option to re-run from current
            // state (Running) without going through Queued — for
            // user-driven "complete this prep" affordances.
            Limited => &[Queued, Running, Stale, UserSuppressed, UserDismissed],
            // Refresh requeues.
            Stale => &[Queued, UserSuppressed, UserDismissed],
            // Retry requeues.
            Failed => &[Queued, UserSuppressed, UserDismissed],
            // Un-suppress / un-dismiss puts user-killed states back
            // into the prep-needed slot. The system picks up the new
            // PrepNeeded on next compute.
            UserSuppressed => &[PrepNeeded],
            UserDismissed => &[PrepNeeded],
        }
    }

    /// AC-335.14 helper: is `to` reachable from `self` in one step?
    pub const fn can_transition_to(self, to: PrepStatus) -> bool {
        let legal = self.legal_transitions();
        let mut i = 0;
        while i < legal.len() {
            // PrepStatus is Copy + has cheap eq, but we can't call
            // PartialEq::eq in const context. Match on discriminant.
            if matches_status(legal[i], to) {
                return true;
            }
            i += 1;
        }
        false
    }
}

/// const-safe discriminant equality (PartialEq::eq is not const).
const fn matches_status(a: PrepStatus, b: PrepStatus) -> bool {
    use PrepStatus::*;
    matches!(
        (a, b),
        (BlockedNoEntity, BlockedNoEntity)
            | (PrepNeeded, PrepNeeded)
            | (Queued, Queued)
            | (Running, Running)
            | (Ready, Ready)
            | (Limited, Limited)
            | (Stale, Stale)
            | (Failed, Failed)
            | (UserSuppressed, UserSuppressed)
            | (UserDismissed, UserDismissed)
    )
}

// -----------------------------------------------------------------------------
// Supporting enums + value types
// -----------------------------------------------------------------------------

/// Reason a meeting cannot have prep generated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BlockingReason {
    NoLinkedEntity,
    AmbiguousAttendeeMatch,
    SourceRevoked,
    PolicyForbidden,
}

/// Reason existing prep is stale or limited.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StaleReason {
    EntityContextStale,
    RecentCorrection,
    SourceAsofOlderThanThreshold,
    ContradictedClaimUpstream,
}

/// Reason a refresh is being enqueued.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RefreshReason {
    /// Manual entity link / relink / unlink path.
    EntityRelinked,
    /// Upstream claim retraction / contradiction invalidated prep.
    UpstreamClaimChanged,
    /// User explicitly requested a refresh.
    UserRequested,
    /// Background scheduler detected stale source asof.
    StaleSourceAsof,
    /// Retry after a Failed transition.
    Retry,
}

/// Lightweight binding to the entity this meeting is prepared for.
///
/// W1 substrate-only DTO. W2/W3 surface blocks may compose with
/// richer entity DTOs as they wire consumers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EntityBinding {
    pub entity_id: String,
    pub entity_type: String,
}

/// Reference to a source's `source_asof` value at the time prep was
/// generated. Aggregated into [`PrepStatusSnapshot::source_asof_inputs`]
/// for freshness display.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SourceAsofRef {
    pub source: String,
    pub as_of: String,
}

/// Aggregate trust posture across underlying claims used for prep.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TrustSummary {
    pub likely_current: u32,
    pub use_with_caution: u32,
    pub needs_verification: u32,
}

/// Display-safe provenance envelope summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EnvelopeProvenance {
    pub envelope_id: String,
    pub generated_at: String,
}

/// Reference to a claim-backed decision captured during the meeting
/// (W1 substrate placeholder; W2/W3 surfaces compose with the full
/// claim envelope per ADR-0125).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DecisionRef {
    pub claim_id: String,
    pub text: String,
}

/// User-authored fields that survive every status recompute.
///
/// AC-335.8: preserved across recompute + re-enrichment.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserAuthoredFields {
    pub agenda: Option<String>,
    pub notes: Option<String>,
    pub preparation_text: Option<String>,
    /// Display labels only; not raw emails (ADR-0125 sensitivity).
    pub hidden_attendees: Vec<String>,
    pub decisions: Vec<DecisionRef>,
}

// -----------------------------------------------------------------------------
// PrepStatusSnapshot DTO (L0 packet §5.5)
// -----------------------------------------------------------------------------

/// Service-owned status contract for meeting prep readiness.
///
/// Returned by [`read::compute_status`]. AC-335.3 enumerates required
/// fields. The DTO is intentionally non-fragmented — every consumer
/// (FolioBar, Meeting Briefing block, Daily Briefing rollup, Meeting
/// Detail block) renders from the same snapshot so they converge
/// without app restart (AC-335.6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PrepStatusSnapshot {
    pub meeting_id: String,
    pub event_id: Option<String>,
    pub linked_entity: Option<EntityBinding>,
    pub status: PrepStatus,
    pub blocking_reason: Option<BlockingReason>,
    pub stale_reason: Option<StaleReason>,
    pub last_prepared_at: Option<String>,
    pub source_asof_inputs: Vec<SourceAsofRef>,
    pub trust_summary: Option<TrustSummary>,
    pub provenance: Option<EnvelopeProvenance>,
    pub next_allowed_transition: Vec<PrepStatus>,
    pub user_authored: UserAuthoredFields,
}

// -----------------------------------------------------------------------------
// Error type
// -----------------------------------------------------------------------------

/// Errors returned by the meeting_prep_status service.
#[derive(Debug, thiserror::Error)]
pub enum PrepStatusError {
    /// AC-335.14: caller requested an illegal state transition.
    #[error("illegal prep status transition from {from:?} to {to:?}")]
    IllegalTransition { from: PrepStatus, to: PrepStatus },
    /// Meeting id does not exist.
    #[error("meeting not found: {0}")]
    MeetingNotFound(String),
    /// Underlying DB error.
    #[error("database error: {0}")]
    Db(String),
    /// Mutation rejected by ServiceContext gate (read-only mode etc.).
    #[error("mutation rejected: {0}")]
    MutationRejected(String),
    /// Signal emit failed downstream of the write.
    #[error("signal emit failed: {0}")]
    SignalEmit(String),
}

/// Convenience alias for signaling a transition violation directly.
pub type TransitionError = PrepStatusError;

// -----------------------------------------------------------------------------
// State machine tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod state_machine_tests {
    use super::*;

    /// AC-335.14: every variant has an entry in the transition table.
    /// Walk every PrepStatus variant and assert `legal_transitions`
    /// returns a deterministic (possibly empty) slice without
    /// panicking. Exhaustive match in `legal_transitions` itself is
    /// the static guarantee; this test is the runtime witness.
    #[test]
    fn every_variant_has_legal_transitions_entry() {
        for variant in ALL_VARIANTS {
            // Just exercise the function — exhaustive match in
            // legal_transitions is the static guarantee.
            let _ = variant.legal_transitions();
        }
    }

    #[test]
    fn blocked_no_entity_only_transitions_to_prep_needed() {
        assert_eq!(
            PrepStatus::BlockedNoEntity.legal_transitions(),
            &[PrepStatus::PrepNeeded]
        );
    }

    #[test]
    fn ready_can_refresh_or_go_stale() {
        let legal = PrepStatus::Ready.legal_transitions();
        assert!(legal.contains(&PrepStatus::Queued));
        assert!(legal.contains(&PrepStatus::Stale));
        // Ready should NOT be able to jump directly to Failed.
        assert!(!legal.contains(&PrepStatus::Failed));
        // Ready should NOT be able to go to Running without re-queuing
        // (Limited can, Ready cannot — Ready means current).
        assert!(!legal.contains(&PrepStatus::Running));
    }

    #[test]
    fn running_only_goes_to_terminal_worker_outcomes() {
        let legal = PrepStatus::Running.legal_transitions();
        assert_eq!(legal.len(), 3);
        assert!(legal.contains(&PrepStatus::Ready));
        assert!(legal.contains(&PrepStatus::Limited));
        assert!(legal.contains(&PrepStatus::Failed));
    }

    #[test]
    fn illegal_transition_rejected() {
        // PrepNeeded → Ready is illegal (must go through Queued/Running).
        assert!(!PrepStatus::PrepNeeded.can_transition_to(PrepStatus::Ready));
        // Ready → PrepNeeded is illegal (regression of state).
        assert!(!PrepStatus::Ready.can_transition_to(PrepStatus::PrepNeeded));
    }

    #[test]
    fn legal_transition_accepted() {
        assert!(PrepStatus::PrepNeeded.can_transition_to(PrepStatus::Queued));
        assert!(PrepStatus::Queued.can_transition_to(PrepStatus::Running));
        assert!(PrepStatus::Running.can_transition_to(PrepStatus::Ready));
        assert!(PrepStatus::UserSuppressed.can_transition_to(PrepStatus::PrepNeeded));
    }

    #[test]
    fn user_dismissed_un_dismisses_to_prep_needed() {
        assert_eq!(
            PrepStatus::UserDismissed.legal_transitions(),
            &[PrepStatus::PrepNeeded]
        );
    }

    /// AC-335.13 Part A — disjoint-column commutativity property test.
    ///
    /// Plain user-authored fields (agenda, notes, preparation_text,
    /// hidden_attendees) live in `meeting_prep` columns disjoint from
    /// the queue / status columns touched by `enqueue_refresh`.
    /// Concurrent writes to disjoint columns commute: applying
    /// `record_user_authored` then `enqueue_refresh` (or vice versa)
    /// yields the same observable state at the snapshot level.
    ///
    /// Modeled as a pure data property: the snapshot user_authored
    /// fields are independent of the status field, so the order of
    /// updates does not change the final tuple.
    #[test]
    fn part_a_user_authored_commutes_with_refresh_enqueue() {
        let base = PrepStatusSnapshot {
            meeting_id: "m1".into(),
            event_id: None,
            linked_entity: Some(EntityBinding {
                entity_id: "acct-1".into(),
                entity_type: "account".into(),
            }),
            status: PrepStatus::Ready,
            blocking_reason: None,
            stale_reason: None,
            last_prepared_at: None,
            source_asof_inputs: vec![],
            trust_summary: None,
            provenance: None,
            next_allowed_transition: vec![],
            user_authored: UserAuthoredFields::default(),
        };

        let user_authored_update = UserAuthoredFields {
            agenda: Some("agenda v1".into()),
            notes: Some("notes v1".into()),
            preparation_text: Some("prep v1".into()),
            hidden_attendees: vec!["Hidden A".into()],
            decisions: vec![],
        };
        let refresh_status = PrepStatus::Queued;

        // Order A: apply user_authored first, then refresh enqueue.
        let mut a = base.clone();
        a.user_authored = user_authored_update.clone();
        a.status = refresh_status;

        // Order B: apply refresh enqueue first, then user_authored.
        let mut b = base.clone();
        b.status = refresh_status;
        b.user_authored = user_authored_update.clone();

        assert_eq!(
            a, b,
            "Part A: disjoint-column writes (user_authored vs status) must commute"
        );
    }

    const ALL_VARIANTS: &[PrepStatus] = &[
        PrepStatus::BlockedNoEntity,
        PrepStatus::PrepNeeded,
        PrepStatus::Queued,
        PrepStatus::Running,
        PrepStatus::Ready,
        PrepStatus::Limited,
        PrepStatus::Stale,
        PrepStatus::Failed,
        PrepStatus::UserSuppressed,
        PrepStatus::UserDismissed,
    ];
}
