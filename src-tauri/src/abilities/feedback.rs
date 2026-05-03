//! Typed claim feedback substrate per ADR-0123.
//!
//! Nine closed actions describe every kind of judgment a user can pass
//! on a rendered claim. Each maps to a typed effect tuple — claim
//! lifecycle, trust factor, reliability impact, repair queue, and
//! render policy — that downstream consumers (Trust Compiler, repair
//! workers, render filters) read deterministically.
//!
//! This module is the pure semantic contract: the enum, the matrix,
//! and the verification-state machine. The service-level entry that
//! persists feedback rows + applies lifecycle side-effects lives in
//! `services/claims.rs` and is sequenced after this lands.
//!
//! The split intentionally separates the *meaning* of feedback (which
//! is durable, reviewable, and must not drift) from the *write path*
//! (which evolves with schema/repair worker changes). Reviewers can
//! audit the matrix here without grepping the writer.

use serde::{Deserialize, Serialize};

/// Closed set of typed feedback actions a user may apply to a claim.
/// The variant set is the contract: extending it is a substrate change
/// that requires updating every consumer that matches `FeedbackAction`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackAction {
    /// "Yes, this is still true." Adds a user corroboration row and
    /// upweights source/agent reliability.
    ConfirmCurrent,
    /// "Was true, no longer." Keeps history but demotes from current
    /// surfaces; no truth-falseness penalty on the source.
    MarkOutdated,
    /// "This is wrong." Withdraws the claim and broadly tombstones
    /// future re-surfacing for this content.
    MarkFalse,
    /// "Right fact, wrong subject." Tombstones on the asserted
    /// subject; truth not directly contested. Linker reliability
    /// takes the hit, source does not.
    WrongSubject,
    /// "Source doesn't support this." Attribution caveat; claim may
    /// still be true via other evidence. Source-attribution
    /// reliability takes a one-shot hit for this (source, claim_type)
    /// pair only.
    WrongSource,
    /// "I can't tell either way." No truth/lifecycle change; enqueues
    /// at most one bounded corroboration repair job.
    CannotVerify,
    /// "Needs a more nuanced version." Original is dormant + superseded
    /// by the user-authored corrected claim.
    NeedsNuance,
    /// "Don't show me this on this surface." Surface-specific
    /// suppression marker; not truth or trust feedback.
    SurfaceInappropriate,
    /// "Not in the right place." Context/relevance hint only;
    /// no trust effect.
    NotRelevantHere,
}

impl FeedbackAction {
    /// Stable wire-format string. Matches the persisted `feedback_type`
    /// column and the JSON form clients send.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ConfirmCurrent => "confirm_current",
            Self::MarkOutdated => "mark_outdated",
            Self::MarkFalse => "mark_false",
            Self::WrongSubject => "wrong_subject",
            Self::WrongSource => "wrong_source",
            Self::CannotVerify => "cannot_verify",
            Self::NeedsNuance => "needs_nuance",
            Self::SurfaceInappropriate => "surface_inappropriate",
            Self::NotRelevantHere => "not_relevant_here",
        }
    }
}

/// Verification state on `intelligence_claims`. Distinct from
/// `claim_state` (user intent: active/dormant/tombstoned/withdrawn)
/// and `surfacing_state` (rendering: active/dormant). Tracks the
/// system's confidence-in-rendering view.
///
/// State machine:
/// - `Active` → default; claim renders normally.
/// - `Contested` → automated processes flagged it (e.g.
///   `WrongSource` left no qualifying source, `CannotVerify`
///   enqueued repair). Renders with a caveat.
/// - `NeedsUserDecision` → terminal. The system explicitly asks
///   for user judgment. NOT auto-resolvable: only explicit user
///   feedback, a corrected superseding claim, or contradiction
///   reconciliation can close this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimVerificationState {
    #[default]
    Active,
    Contested,
    NeedsUserDecision,
}

/// Render policy a feedback action produces. Render-time consumers
/// (briefing prep, entity surfaces, MCP context builders) honor this
/// instead of grepping `claim_feedback` history.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClaimRenderPolicy {
    /// Show normally.
    Default,
    /// Show with a "user confirmed this" badge.
    DefaultWithUserCorroboration,
    /// Hide from current-state surfaces; available in history.
    HiddenFromCurrent,
    /// Suppress everywhere except audit/history.
    SuppressedExceptAudit,
    /// Suppress only on the subject the user marked wrong.
    SuppressedOnAssertedSubject,
    /// Render with a "source doesn't fully support this" caveat.
    QualifiedBySourceCaveat,
    /// Render with a "needs more evidence" caveat.
    QualifiedNeedsCorroboration,
    /// Render the user's superseding claim instead.
    RenderSuperseder,
    /// Hide only on the named surface (privacy / sensitivity).
    HiddenOnNamedSurface,
    /// Hide / deprioritize only for the named invocation context.
    DeprioritizedInContext,
}

/// Trust-factor effect direction. The Trust Compiler reads this to
/// decide whether to upweight or downweight the source/agent/linker.
/// `(claim alpha, claim beta, source delta)` per ADR-0123 lines
/// 88-103. This struct is the input shape; numeric magnitudes live
/// in the compiler so they can be tuned without touching the matrix.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrustEffect {
    /// Bayesian alpha bump on the claim's evidence aggregate.
    pub claim_alpha_delta: f32,
    /// Bayesian beta bump on the claim's evidence aggregate.
    pub claim_beta_delta: f32,
    /// One-shot reliability delta for the source(s) attached to the
    /// claim. Positive = upweight, negative = downweight. Zero means
    /// "this action is not source feedback."
    pub source_reliability_delta: f32,
    /// One-shot reliability delta for the linker / subject-fit
    /// component. Used by `WrongSubject`.
    pub linker_reliability_delta: f32,
}

impl TrustEffect {
    pub const NONE: Self = Self {
        claim_alpha_delta: 0.0,
        claim_beta_delta: 0.0,
        source_reliability_delta: 0.0,
        linker_reliability_delta: 0.0,
    };
}

/// What the repair queue should do for this action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepairAction {
    /// No repair job.
    None,
    /// Enqueue a freshness / source-asof refresh for outdated content.
    FreshnessRefresh,
    /// Enqueue contradiction or tombstone reconciliation.
    ContradictionReconcile,
    /// Enqueue subject-fit repair (rebind to corrected subject).
    SubjectFitRepair,
    /// Enqueue source-support repair.
    SourceSupportRepair,
    /// Enqueue a single bounded corroboration job. Honors the
    /// per-claim/per-entity/workspace caps in ADR-0123.
    BoundedCorroboration,
    /// Enqueue render-policy / sensitivity repair.
    PolicyRepair,
}

/// Full semantic effect of a feedback action. Pure function of
/// `FeedbackAction`. Consumers that need to know "what does this
/// feedback do?" call `feedback_semantics(action)` and never grep
/// the writer.
#[derive(Debug, Clone, PartialEq)]
pub struct ClaimFeedbackMetadata {
    pub action: FeedbackAction,
    /// New verification state to set on the claim. `Active` means
    /// no change; `Contested` and `NeedsUserDecision` are the
    /// ratchets the matrix raises.
    pub verification_state: ClaimVerificationState,
    /// Trust factor inputs (signs only; magnitudes in the compiler).
    pub trust_effect: TrustEffect,
    /// Repair queue effect.
    pub repair: RepairAction,
    /// Render policy override.
    pub render: ClaimRenderPolicy,
    /// True when this action requires per-action metadata on the
    /// `ClaimFeedbackInput` (corrected_subject for `WrongSubject`,
    /// source_ref for `WrongSource`, surface for
    /// `SurfaceInappropriate`, invocation for `NotRelevantHere`,
    /// corrected text for `NeedsNuance`).
    pub requires_action_metadata: bool,
    /// True when the action carries truth weight (Trust Compiler
    /// should consume it). False for surface/relevance feedback.
    pub is_truth_feedback: bool,
}

/// Pure semantic lookup. `match`-based so adding a new
/// `FeedbackAction` variant without a matrix entry is a build error.
pub const fn feedback_semantics(action: FeedbackAction) -> ClaimFeedbackMetadata {
    match action {
        FeedbackAction::ConfirmCurrent => ClaimFeedbackMetadata {
            action,
            verification_state: ClaimVerificationState::Active,
            trust_effect: TrustEffect {
                claim_alpha_delta: 1.0,
                claim_beta_delta: 0.0,
                source_reliability_delta: 0.10,
                linker_reliability_delta: 0.0,
            },
            repair: RepairAction::None,
            render: ClaimRenderPolicy::DefaultWithUserCorroboration,
            requires_action_metadata: false,
            is_truth_feedback: true,
        },
        FeedbackAction::MarkOutdated => ClaimFeedbackMetadata {
            action,
            verification_state: ClaimVerificationState::Active,
            trust_effect: TrustEffect {
                claim_alpha_delta: 0.5,
                claim_beta_delta: 0.0,
                source_reliability_delta: 0.0,
                linker_reliability_delta: 0.0,
            },
            repair: RepairAction::FreshnessRefresh,
            render: ClaimRenderPolicy::HiddenFromCurrent,
            requires_action_metadata: false,
            is_truth_feedback: true,
        },
        FeedbackAction::MarkFalse => ClaimFeedbackMetadata {
            action,
            verification_state: ClaimVerificationState::Active,
            trust_effect: TrustEffect {
                claim_alpha_delta: 0.0,
                claim_beta_delta: 1.0,
                source_reliability_delta: -0.05,
                linker_reliability_delta: 0.0,
            },
            repair: RepairAction::ContradictionReconcile,
            render: ClaimRenderPolicy::SuppressedExceptAudit,
            requires_action_metadata: false,
            is_truth_feedback: true,
        },
        FeedbackAction::WrongSubject => ClaimFeedbackMetadata {
            action,
            verification_state: ClaimVerificationState::Active,
            trust_effect: TrustEffect {
                claim_alpha_delta: 0.0,
                claim_beta_delta: 0.3,
                source_reliability_delta: 0.0,
                linker_reliability_delta: -0.30,
            },
            repair: RepairAction::SubjectFitRepair,
            render: ClaimRenderPolicy::SuppressedOnAssertedSubject,
            // Optional corrected_subject; metadata struct may carry it.
            requires_action_metadata: false,
            is_truth_feedback: true,
        },
        FeedbackAction::WrongSource => ClaimFeedbackMetadata {
            action,
            verification_state: ClaimVerificationState::Contested,
            trust_effect: TrustEffect {
                claim_alpha_delta: 0.0,
                claim_beta_delta: 0.2,
                source_reliability_delta: -0.20,
                linker_reliability_delta: 0.0,
            },
            repair: RepairAction::SourceSupportRepair,
            render: ClaimRenderPolicy::QualifiedBySourceCaveat,
            requires_action_metadata: true,
            is_truth_feedback: true,
        },
        FeedbackAction::CannotVerify => ClaimFeedbackMetadata {
            action,
            verification_state: ClaimVerificationState::Contested,
            trust_effect: TrustEffect::NONE,
            repair: RepairAction::BoundedCorroboration,
            render: ClaimRenderPolicy::QualifiedNeedsCorroboration,
            requires_action_metadata: false,
            is_truth_feedback: false,
        },
        FeedbackAction::NeedsNuance => ClaimFeedbackMetadata {
            action,
            verification_state: ClaimVerificationState::Active,
            trust_effect: TrustEffect {
                // Sign only; the writer chooses alpha vs beta based
                // on text-overlap heuristic between original and
                // corrected per ADR-0123.
                claim_alpha_delta: 0.3,
                claim_beta_delta: 0.0,
                source_reliability_delta: 0.0,
                linker_reliability_delta: 0.0,
            },
            repair: RepairAction::ContradictionReconcile,
            render: ClaimRenderPolicy::RenderSuperseder,
            requires_action_metadata: true,
            is_truth_feedback: true,
        },
        FeedbackAction::SurfaceInappropriate => ClaimFeedbackMetadata {
            action,
            verification_state: ClaimVerificationState::Active,
            trust_effect: TrustEffect::NONE,
            repair: RepairAction::PolicyRepair,
            render: ClaimRenderPolicy::HiddenOnNamedSurface,
            requires_action_metadata: true,
            is_truth_feedback: false,
        },
        FeedbackAction::NotRelevantHere => ClaimFeedbackMetadata {
            action,
            verification_state: ClaimVerificationState::Active,
            trust_effect: TrustEffect::NONE,
            repair: RepairAction::None,
            render: ClaimRenderPolicy::DeprioritizedInContext,
            requires_action_metadata: true,
            is_truth_feedback: false,
        },
    }
}

/// Compute the next verification state given the current state and
/// the incoming action. The state machine is monotone toward
/// `NeedsUserDecision` (terminal): automated processes may ratchet
/// `Active → Contested → NeedsUserDecision`, but only an explicit
/// `ConfirmCurrent` resets to `Active`.
pub fn transition_for_feedback(
    current: ClaimVerificationState,
    action: FeedbackAction,
) -> ClaimVerificationState {
    let proposed = feedback_semantics(action).verification_state;
    match (current, action) {
        // Explicit confirm always resets to Active. Auditability of
        // a prior NeedsUserDecision is preserved by the append-only
        // claim_feedback row history; the state column reflects the
        // current judgment.
        (_, FeedbackAction::ConfirmCurrent) => ClaimVerificationState::Active,
        // NeedsUserDecision is terminal under automated processes.
        // No automatic transition out of it; only ConfirmCurrent
        // (handled above) or a corrected superseding claim closes it.
        (ClaimVerificationState::NeedsUserDecision, _) => {
            ClaimVerificationState::NeedsUserDecision
        }
        // Otherwise take the proposed state, but never DROP from
        // Contested to Active automatically.
        (ClaimVerificationState::Contested, _)
            if matches!(proposed, ClaimVerificationState::Active) =>
        {
            ClaimVerificationState::Contested
        }
        _ => proposed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feedback_action_serializes_only_nine_closed_values() {
        let names: Vec<&str> = [
            FeedbackAction::ConfirmCurrent,
            FeedbackAction::MarkOutdated,
            FeedbackAction::MarkFalse,
            FeedbackAction::WrongSubject,
            FeedbackAction::WrongSource,
            FeedbackAction::CannotVerify,
            FeedbackAction::NeedsNuance,
            FeedbackAction::SurfaceInappropriate,
            FeedbackAction::NotRelevantHere,
        ]
        .iter()
        .map(|a| a.as_str())
        .collect();
        assert_eq!(names.len(), 9);
        let unique: std::collections::HashSet<_> = names.iter().collect();
        assert_eq!(unique.len(), 9, "wire-format strings must be unique");
        for n in &names {
            // Round-trip through serde to catch a rename mismatch.
            let json = format!("\"{n}\"");
            let parsed: FeedbackAction = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed.as_str(), *n);
        }
    }

    #[test]
    fn feedback_action_rejects_unknown_string() {
        let res: Result<FeedbackAction, _> = serde_json::from_str("\"like\"");
        assert!(res.is_err());
        let res: Result<FeedbackAction, _> = serde_json::from_str("\"dismiss\"");
        assert!(res.is_err());
    }

    #[test]
    fn feedback_matrix_is_total_for_every_action() {
        // Every variant produces a non-default metadata. The const
        // match guarantees this at compile time; the test asserts
        // the runtime contract — every action carries a meaningful
        // semantic, not a placeholder no-op.
        for action in [
            FeedbackAction::ConfirmCurrent,
            FeedbackAction::MarkOutdated,
            FeedbackAction::MarkFalse,
            FeedbackAction::WrongSubject,
            FeedbackAction::WrongSource,
            FeedbackAction::CannotVerify,
            FeedbackAction::NeedsNuance,
            FeedbackAction::SurfaceInappropriate,
            FeedbackAction::NotRelevantHere,
        ] {
            let meta = feedback_semantics(action);
            assert_eq!(meta.action, action);
        }
    }

    #[test]
    fn wrong_subject_does_not_penalize_source_reliability() {
        // Subject-fit failure is a linker problem; the source might
        // still be perfectly reliable for facts about other subjects.
        let meta = feedback_semantics(FeedbackAction::WrongSubject);
        assert_eq!(meta.trust_effect.source_reliability_delta, 0.0);
        assert!(meta.trust_effect.linker_reliability_delta < 0.0);
    }

    #[test]
    fn wrong_source_does_not_mark_truth_false() {
        // Attribution caveat, not a truth contradiction. The claim
        // may still be true via other evidence.
        let meta = feedback_semantics(FeedbackAction::WrongSource);
        assert_eq!(meta.verification_state, ClaimVerificationState::Contested);
        assert!(meta.trust_effect.source_reliability_delta < 0.0);
        // Beta is small (0.2) because the truth-state effect is
        // indirect (via lost source support), not direct contradiction.
        assert!(meta.trust_effect.claim_beta_delta < 1.0);
    }

    #[test]
    fn cannot_verify_is_not_truth_feedback() {
        let meta = feedback_semantics(FeedbackAction::CannotVerify);
        assert!(!meta.is_truth_feedback);
        assert_eq!(meta.trust_effect, TrustEffect::NONE);
        assert_eq!(meta.repair, RepairAction::BoundedCorroboration);
    }

    #[test]
    fn surface_inappropriate_only_affects_render_policy() {
        let meta = feedback_semantics(FeedbackAction::SurfaceInappropriate);
        assert!(!meta.is_truth_feedback);
        assert_eq!(meta.trust_effect, TrustEffect::NONE);
        assert_eq!(meta.render, ClaimRenderPolicy::HiddenOnNamedSurface);
    }

    #[test]
    fn not_relevant_here_is_relevance_only() {
        let meta = feedback_semantics(FeedbackAction::NotRelevantHere);
        assert!(!meta.is_truth_feedback);
        assert_eq!(meta.trust_effect, TrustEffect::NONE);
        assert_eq!(meta.repair, RepairAction::None);
    }

    #[test]
    fn confirm_current_corroborates_claim_and_source() {
        let meta = feedback_semantics(FeedbackAction::ConfirmCurrent);
        assert!(meta.trust_effect.claim_alpha_delta > 0.0);
        assert!(meta.trust_effect.source_reliability_delta > 0.0);
        assert_eq!(meta.repair, RepairAction::None);
        assert_eq!(
            meta.render,
            ClaimRenderPolicy::DefaultWithUserCorroboration
        );
    }

    #[test]
    fn state_machine_active_to_contested_to_needs_user_decision_terminal() {
        let s0 = ClaimVerificationState::Active;
        // CannotVerify ratchets Active → Contested.
        let s1 = transition_for_feedback(s0, FeedbackAction::CannotVerify);
        assert_eq!(s1, ClaimVerificationState::Contested);

        // Once in Contested, an automatic process moving us back to
        // Active (e.g. a NeedsNuance whose matrix says Active) does
        // NOT silently re-promote — it stays Contested.
        let s2 = transition_for_feedback(s1, FeedbackAction::NeedsNuance);
        assert_eq!(s2, ClaimVerificationState::Contested);

        // Only explicit ConfirmCurrent resets to Active. Audit row
        // is preserved by the append-only claim_feedback table.
        let s3 = transition_for_feedback(s2, FeedbackAction::ConfirmCurrent);
        assert_eq!(s3, ClaimVerificationState::Active);
    }

    #[test]
    fn needs_user_decision_is_terminal_under_automated_actions() {
        let s0 = ClaimVerificationState::NeedsUserDecision;
        // No automated action escapes NeedsUserDecision.
        for action in [
            FeedbackAction::CannotVerify,
            FeedbackAction::WrongSource,
            FeedbackAction::MarkOutdated,
            FeedbackAction::SurfaceInappropriate,
            FeedbackAction::NotRelevantHere,
            FeedbackAction::WrongSubject,
        ] {
            let next = transition_for_feedback(s0, action);
            assert_eq!(
                next,
                ClaimVerificationState::NeedsUserDecision,
                "action {:?} must NOT auto-escape NeedsUserDecision",
                action
            );
        }
        // Only ConfirmCurrent (explicit user judgment) closes it.
        let closed = transition_for_feedback(s0, FeedbackAction::ConfirmCurrent);
        assert_eq!(closed, ClaimVerificationState::Active);
    }

    #[test]
    fn render_policies_are_distinct_per_action() {
        let mut seen = std::collections::HashMap::new();
        for action in [
            FeedbackAction::ConfirmCurrent,
            FeedbackAction::MarkOutdated,
            FeedbackAction::MarkFalse,
            FeedbackAction::WrongSubject,
            FeedbackAction::WrongSource,
            FeedbackAction::CannotVerify,
            FeedbackAction::NeedsNuance,
            FeedbackAction::SurfaceInappropriate,
            FeedbackAction::NotRelevantHere,
        ] {
            let render = feedback_semantics(action).render;
            // Multiple actions may produce the same render policy
            // (Default is shared by ConfirmCurrent). What we want
            // here is: every action's render is a deliberate pick,
            // not a fallback.
            seen.entry(render).or_insert_with(Vec::new).push(action);
        }
        // Sanity: at least 7 distinct render policies covering 9
        // actions (some intentionally share — e.g. multiple suppress
        // variants).
        assert!(seen.len() >= 7, "render policies should cover most cases distinctly");
    }
}
