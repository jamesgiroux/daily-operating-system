//! DOS-459 — `EntityIntelligenceEnvelope` DTO contract.
//!
//! Read-side projection over existing claim/proposal/touchpoint substrate.
//! Per L0 packet `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md` §5.1
//! and the locked decisions in §13 (server-signed cursor pagination, per-fact `ProvenanceRef`,
//! typed empty reasons, 1-outer-N-inner block model).
//!
//! Coexists with the existing `get_entity_context` ability — DOS-459 ships the typed
//! envelope; consolidation deferred to v1.5.x per §13 Q1/Q6.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::abilities::list_open_loops::OpenLoop;
use crate::abilities::provenance::SubjectRef;
use crate::abilities::trust::types::TrustBand;
use crate::sensitivity::{ClaimVerificationState, RenderableClaimText};
use crate::types::{ClaimSensitivity, ClaimState, SurfacingState};

/// Schema version for `get_entity_intelligence` envelope.
pub const ENVELOPE_SCHEMA_VERSION: u32 = 1;

// ---- input -----------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EntityKind {
    Account,
    Project,
    Person,
}

impl EntityKind {
    /// Lower-snake_case identifier matching the legacy `get_entity_context` API.
    pub fn as_lower_str(&self) -> &'static str {
        match self {
            EntityKind::Account => "account",
            EntityKind::Project => "project",
            EntityKind::Person => "person",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContextDepth {
    Shallow,
    Standard,
    Deep,
}

impl ContextDepth {
    pub(crate) fn into_legacy(self) -> crate::abilities::get_entity_context::ContextDepth {
        match self {
            ContextDepth::Shallow => crate::abilities::get_entity_context::ContextDepth::Shallow,
            ContextDepth::Standard => crate::abilities::get_entity_context::ContextDepth::Standard,
            ContextDepth::Deep => crate::abilities::get_entity_context::ContextDepth::Deep,
        }
    }
}

/// Envelope section discriminator.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum EnvelopeSection {
    Facts,
    Health,
    MetadataProposals,
    OpenLoops,
    Touchpoints,
    Threads,
    Record,
}

impl EnvelopeSection {
    /// All section variants in canonical order — `sections` map enumerates these per AC-459.2.
    pub const ALL: &'static [EnvelopeSection] = &[
        EnvelopeSection::Facts,
        EnvelopeSection::Health,
        EnvelopeSection::MetadataProposals,
        EnvelopeSection::OpenLoops,
        EnvelopeSection::Touchpoints,
        EnvelopeSection::Threads,
        EnvelopeSection::Record,
    ];
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EntityIntelligenceInput {
    pub schema_version: u32,
    pub entity_type: EntityKind,
    pub entity_id: String,
    pub depth: ContextDepth,
    /// `None` = return all sections; non-empty subset = only those sections populated;
    /// excluded sections still appear in `sections` map with `Empty { reason: NotRequested }`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sections: Option<Vec<EnvelopeSection>>,
}

// ---- cursor + pagination ---------------------------------------------------

/// Opaque server-signed cursor token. Per §13 Q11: cursor signing key + rotation policy
/// is a v1.4.6/v1.4.7 concern (not a substrate change for W1) — at W1 we transport
/// an opaque string and let later substrate sign/verify it.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(transparent)]
pub struct Cursor(pub String);

impl Cursor {
    pub fn new(token: impl Into<String>) -> Self {
        Self(token.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Per-list cursor lifecycle state (cycle-1 correctness F4).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CursorState {
    /// Pagination is consistent — `next_cursor` continues the same logical sequence.
    Stable,
    /// Rows shifted under us (a concurrent insert/retract). Caller MAY continue;
    /// some rows may be skipped or duplicated. Advisory is human-readable.
    DataShifted { advisory: String },
    /// The cursor is no longer valid. Caller MUST restart pagination from page 1.
    Invalidated {
        reason: String,
        restart_required: bool,
    },
}

/// Generic paginated wrapper for envelope list-shape fields.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Paginated<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<Cursor>,
    pub total_hint: Option<u64>,
    pub cursor_state: CursorState,
}

impl<T> Paginated<T> {
    pub fn empty_stable() -> Self {
        Self {
            items: Vec::new(),
            next_cursor: None,
            total_hint: Some(0),
            cursor_state: CursorState::Stable,
        }
    }

    pub fn stable(items: Vec<T>) -> Self {
        let total = items.len() as u64;
        Self {
            items,
            next_cursor: None,
            total_hint: Some(total),
            cursor_state: CursorState::Stable,
        }
    }
}

// ---- section state --------------------------------------------------------

/// Typed empty-state reasons per §5.1. No `null` without reason.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EmptyReason {
    NotConnected,
    NotProcessedYet,
    FilteredOutBySubject,
    NoRelevantTouchpoints,
    Stale,
    NoEvidenceBackedProposal,
    UnsupportedForSubject,
    NotRequested,
    /// Per AC-459.10 — one section failing emits this; others continue.
    PartialFailure { advisory: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SectionState {
    Present { item_count: u64 },
    Empty { reason: EmptyReason },
}

// ---- normalized subject ----------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedSubject {
    pub kind: EntityKind,
    pub id: String,
    pub subject_ref: SubjectRef,
    pub display_label: String,
}

// ---- per-fact provenance reference (ADR-0130 §2 amendment) ----------------

/// Per-fact provenance pointer per cycle-1 architecture F9 — points into the
/// envelope-level `EnvelopeProvenance.sources` index. Prevents 64 KB serialized
/// provenance blowup that ADR-0108 names.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProvenanceRef {
    pub source_ids: Vec<String>,
}

impl ProvenanceRef {
    pub fn empty() -> Self {
        Self {
            source_ids: Vec::new(),
        }
    }

    pub fn from_ids(ids: impl IntoIterator<Item = String>) -> Self {
        Self {
            source_ids: ids.into_iter().collect(),
        }
    }
}

// ---- envelope-level provenance + trust + sensitivity ---------------------

/// Display-safe per-source descriptor for the envelope-level provenance index.
/// Raw source identifiers (Glean doc IDs, meeting IDs, URLs, etc.) are redacted
/// per DOS-477 — never emitted unless render policy permits.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EnvelopeProvenanceSource {
    /// Stable id used by `ProvenanceRef.source_ids` to point into this index.
    pub id: String,
    pub label: String,
    pub source_type: Option<String>,
    #[schemars(with = "Option<String>")]
    pub as_of: Option<DateTime<Utc>>,
    pub redacted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EnvelopeProvenance {
    pub sources: Vec<EnvelopeProvenanceSource>,
    pub redaction_applied: bool,
}

impl EnvelopeProvenance {
    pub fn empty() -> Self {
        Self {
            sources: Vec::new(),
            redaction_applied: false,
        }
    }
}

/// Aggregate trust posture plus per-section caveats.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EnvelopeTrustSummary {
    pub aggregate_band: TrustBand,
    pub section_caveats: BTreeMap<EnvelopeSection, String>,
}

impl EnvelopeTrustSummary {
    pub fn unscored() -> Self {
        Self {
            aggregate_band: TrustBand::Unscored,
            section_caveats: BTreeMap::new(),
        }
    }
}

// ---- claim-backed fact, health story, metadata proposals ------------------

/// Claim freshness band — duplicate of receipt enum but lives in the envelope
/// to keep the contract self-describing.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Freshness {
    Current,
    Aging,
    Stale,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EntityFact {
    pub claim_id: String,
    pub subject_ref: SubjectRef,
    pub field_path: Option<String>,
    pub claim_type: String,
    /// Claim text rendered through sensitivity gate — never raw.
    pub rendered_text: RenderableClaimText,
    pub trust_band: TrustBand,
    pub freshness: Freshness,
    #[schemars(with = "Option<String>")]
    pub source_asof: Option<DateTime<Utc>>,
    pub sensitivity: ClaimSensitivity,
    pub lifecycle_state: ClaimState,
    pub surfacing_state: SurfacingState,
    pub verification_state: ClaimVerificationState,
    /// ADR-0130 §2 amendment — per-fact provenance is a *reference* into the envelope's
    /// top-level `EnvelopeProvenance.sources`, NOT an inlined `Vec<ProvenanceSource>`.
    pub provenance: ProvenanceRef,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HealthStory {
    pub headline: Option<String>,
    pub rows: Vec<HealthStoryRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HealthStoryRow {
    pub label: String,
    pub body: String,
    pub evidence_claim_ids: Vec<String>,
    pub provenance: ProvenanceRef,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MetadataProposal {
    pub proposal_id: String,
    pub subject_ref: SubjectRef,
    pub field_path: String,
    pub current_value: Option<String>,
    pub proposed_value: String,
    pub trust_band: TrustBand,
    pub sensitivity: ClaimSensitivity,
    pub provenance: ProvenanceRef,
}

// ---- open loop with receipt target ----------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptTargetRef {
    pub claim_id: String,
    pub subject_ref: SubjectRef,
    pub field_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OpenLoopWithReceipt {
    pub open_loop: OpenLoop,
    pub receipt_target: ReceiptTargetRef,
    pub trust_band: TrustBand,
    pub freshness: Freshness,
    pub provenance: ProvenanceRef,
}

// ---- touchpoint bundle ----------------------------------------------------

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TouchpointKind {
    Meeting,
    EmailThread,
    Document,
    Salesforce,
    Linear,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InclusionReason {
    SubjectMatch,
    EntityLink,
    AttendeeMatch,
    DomainMatch,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExclusionReason {
    SubjectMismatch,
    OutsideWindow,
    LowConfidence,
    Suppressed,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Touchpoint {
    pub meeting_id: Option<String>,
    pub kind: TouchpointKind,
    #[schemars(with = "String")]
    pub when: DateTime<Utc>,
    pub subject_ref: SubjectRef,
    pub inclusion_reason: InclusionReason,
    pub exclusion_reason: Option<ExclusionReason>,
    pub trust_band: TrustBand,
    pub freshness: Freshness,
    pub provenance: ProvenanceRef,
}

/// Per §5.2 candidate-set primitive — typed basis for "why these touchpoints belong".
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CandidateSetRef {
    #[schemars(with = "Option<String>")]
    pub window_start: Option<DateTime<Utc>>,
    #[schemars(with = "Option<String>")]
    pub window_end: Option<DateTime<Utc>>,
    pub filter_description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SubjectScope {
    pub primary: SubjectRef,
    pub also_includes: Vec<SubjectRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TouchpointBundle {
    pub upcoming: Paginated<Touchpoint>,
    pub recent: Paginated<Touchpoint>,
    pub candidate_set: CandidateSetRef,
    pub empty_reason: Option<EmptyReason>,
    pub subject_scope: SubjectScope,
}

// ---- threads + record entries --------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ThreadSummary {
    pub thread_id: String,
    pub title: Option<String>,
    #[schemars(with = "Option<String>")]
    pub last_activity_at: Option<DateTime<Utc>>,
    pub message_count: u32,
    pub provenance: ProvenanceRef,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecordEntry {
    pub claim_id: String,
    pub subject_ref: SubjectRef,
    pub claim_type: String,
    #[schemars(with = "String")]
    pub recorded_at: DateTime<Utc>,
    pub rendered_text: RenderableClaimText,
    pub trust_band: TrustBand,
    pub sensitivity: ClaimSensitivity,
    pub provenance: ProvenanceRef,
}

// ---- envelope --------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EntityIntelligenceEnvelope {
    pub schema_version: u32,
    pub subject: NormalizedSubject,
    pub sections: BTreeMap<EnvelopeSection, SectionState>,
    pub facts: Paginated<EntityFact>,
    pub health_story: Option<HealthStory>,
    pub metadata_proposals: Paginated<MetadataProposal>,
    pub open_loops: Paginated<OpenLoopWithReceipt>,
    pub touchpoints: Paginated<TouchpointBundle>,
    pub threads: Paginated<ThreadSummary>,
    pub record_entries: Paginated<RecordEntry>,
    pub trust: EnvelopeTrustSummary,
    pub provenance: EnvelopeProvenance,
    pub sensitivity: ClaimSensitivity,
}
