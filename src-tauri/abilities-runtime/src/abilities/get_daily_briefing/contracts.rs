//! `DailyBriefingOutput` DTO contract.
//!
//! Read-side composition over existing meeting prep status + entity intelligence +
//! daily readiness substrate. Per L0 packet §5.10 and the locked decisions in
//! §13 Q9 (Read-only; no auto-enqueue) and cycle-1 correctness F3
//! (`BriefingState` is a composed struct, NOT a flat enum — real briefings have
//! multi-dimensional state).
//!
//! Coexists with the `get_daily_readiness` ability while this typed
//! briefing envelope that composes per-subject `EntityIntelligenceEnvelope`s
//! (§5.1) and per-meeting prep status (§5.5). The readiness ability remains
//! the provider-backed synthesis path; this ability is pure read composition.

use chrono::NaiveDate;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::abilities::get_entity_intelligence::contracts::{
    CandidateSetRef, Cursor, EnvelopeProvenance, Paginated,
};
use crate::abilities::trust::types::TrustBand;
use crate::types::ClaimSensitivity;

/// Schema version for `get_daily_briefing` envelope.
pub const BRIEFING_SCHEMA_VERSION: u32 = 1;

// ---- input -----------------------------------------------------------------

/// Briefing section discriminator. `None` (or empty list) requests all sections.
#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord,
)]
#[serde(rename_all = "snake_case")]
pub enum BriefingSection {
    State,
    CurrentMeeting,
    NextMeeting,
    UpcomingMeetings,
    WatchProposals,
    TrustSummary,
}

impl BriefingSection {
    pub const ALL: &'static [BriefingSection] = &[
        BriefingSection::State,
        BriefingSection::CurrentMeeting,
        BriefingSection::NextMeeting,
        BriefingSection::UpcomingMeetings,
        BriefingSection::WatchProposals,
        BriefingSection::TrustSummary,
    ];
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DailyBriefingInput {
    pub schema_version: u32,
    /// Briefing date. Caller is responsible for picking the user's local-day
    /// boundary; the substrate doesn't second-guess the date.
    #[schemars(with = "String")]
    pub date: NaiveDate,
    /// Workspace scope to assemble the briefing for. Mirrors the existing
    /// `get_daily_readiness` workspace_id contract.
    pub workspace_id: String,
    /// Optional cursor for `upcoming_meetings` re-invocation (AC-507.10).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upcoming_meetings_cursor: Option<Cursor>,
    /// `None` = return all sections; non-empty subset = only those sections populated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sections: Option<Vec<BriefingSection>>,
}

// ---- meeting brief ref ----------------------------------------------------

/// Reference to a meeting included in the briefing — enough to render a row,
/// compose a deeper detail surface, and route a feedback action without
/// transporting full prep payloads.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MeetingBriefRef {
    pub meeting_id: String,
    pub title: Option<String>,
    pub starts_at: Option<String>,
    pub ends_at: Option<String>,
    pub linked_entity_type: Option<String>,
    pub linked_entity_id: Option<String>,
    /// PrepStatus discriminant in lower_snake_case (e.g. `ready`,
    /// `prep_needed`, `queued`, `stale`, `user_suppressed`). Mirrors the
    /// snapshot read from the meeting_prep_status service.
    pub prep_status: String,
    pub blocking_reason: Option<String>,
    pub stale_reason: Option<String>,
    pub last_prepared_at: Option<String>,
}

// ---- briefing state — composed (cycle-1 correctness F3) --------------------

/// AC-507.4: `BriefingState` is a **composed struct**, not a flat enum.
/// Real briefings carry multi-dimensional posture — "available + needs-prep for
/// one of N meetings + has corrections + advisory" is a normal day, and a flat
/// enum forces a precedence that doesn't exist in the domain.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BriefingState {
    pub availability: BriefingAvailability,
    pub freshness: BriefingFreshness,
    pub integrity: BriefingIntegrity,
    pub advisories: Vec<BriefingAdvisory>,
}

/// Overall presence of a briefing for the requested date.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", rename_all_fields = "camelCase")]
pub enum BriefingAvailability {
    Available,
    Empty { reason: BriefingEmptyReason },
    AuthLocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BriefingEmptyReason {
    NoMeetings,
    DateOutsideKnownWindow,
    WorkspaceUnknown,
}

/// Freshness posture across the briefing's underlying prep + claim inputs.
/// Independent of integrity — a Fresh briefing can have HasCorrections.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", rename_all_fields = "camelCase")]
pub enum BriefingFreshness {
    Fresh,
    Stale { reason: BriefingStaleReason },
    NeedsPreparation { meeting_ids: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BriefingStaleReason {
    SourceAsofOlderThanThreshold,
    UpstreamClaimChanged,
    EntityContextStale,
}

/// Claim-store integrity — corrections / ambiguity that consumers should
/// surface even when the briefing is otherwise fresh.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", rename_all_fields = "camelCase")]
pub enum BriefingIntegrity {
    Clean,
    HasCorrections { superseded_claim_ids: Vec<String> },
    HasAmbiguity { ambiguous_pairs: Vec<AmbiguityPair> },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AmbiguityPair {
    pub claim_id_a: String,
    pub claim_id_b: String,
    pub reason: String,
}

/// Typed non-blocking advisory surfaced alongside the briefing state.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", rename_all_fields = "camelCase")]
pub enum BriefingAdvisory {
    /// A watch proposal is available for review. Pairs with the
    /// `watch_proposals` envelope field.
    WatchProposal {
        proposal_id: String,
        summary: String,
    },
    /// One or more meetings have no linked entity — render a relink CTA but
    /// don't block the rest of the briefing.
    UnlinkedMeetings { meeting_ids: Vec<String> },
    /// Source connector reported a partial failure; briefing still rendered
    /// with the readers that succeeded.
    PartialReadFailure { advisory: String },
}

// ---- watch proposals + trust summary ---------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WatchProposal {
    pub proposal_id: String,
    pub subject_kind: String,
    pub subject_id: String,
    pub headline: String,
    pub trust_band: TrustBand,
    pub sensitivity: ClaimSensitivity,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BriefingTrustSummary {
    pub aggregate_band: TrustBand,
    pub likely_current_count: u32,
    pub use_with_caution_count: u32,
    pub needs_verification_count: u32,
}

impl BriefingTrustSummary {
    pub fn unscored() -> Self {
        Self {
            aggregate_band: TrustBand::Unscored,
            likely_current_count: 0,
            use_with_caution_count: 0,
            needs_verification_count: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceAsofRef {
    pub source: String,
    pub as_of: String,
}

// ---- output envelope -------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DailyBriefingOutput {
    pub schema_version: u32,
    #[schemars(with = "String")]
    pub date: NaiveDate,
    pub state: BriefingState,
    pub current_meeting: Option<MeetingBriefRef>,
    pub next_meeting: Option<MeetingBriefRef>,
    /// AC-507.10 — paginated. Cursor is opaque server-signed; consumers
    /// re-invoke with `upcoming_meetings_cursor` to continue.
    pub upcoming_meetings: Paginated<MeetingBriefRef>,
    pub candidate_set: CandidateSetRef,
    pub watch_proposals: Vec<WatchProposal>,
    pub trust_summary: BriefingTrustSummary,
    pub provenance: EnvelopeProvenance,
    pub sensitivity: ClaimSensitivity,
    pub source_asof_inputs: Vec<SourceAsofRef>,
}
