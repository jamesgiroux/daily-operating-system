//! Surface-independent composition contract substrate types per ADR-0130.
//!
//! `Composition` is the typed block tree the substrate ships to surfaces
//! (Tauri React, WordPress via SurfaceClient, MCP head, CLI head). Surfaces
//! render compositions; they do not author them.
//!
//! ## Substrate-owned authorship
//!
//! Per ADR-0130 §1, ONLY abilities (substrate code) construct compositions.
//! Surface code, command handlers, hooks, and tests outside `abilities/*`
//! MUST NOT construct `Composition` directly. The constructor visibility
//! ([`Composition::new`]) is `pub(crate)` so Rust enforces this at the
//! module boundary; a defense-in-depth grep lint at
//! `scripts/check_composition_authorship.sh` scans non-abilities call sites
//! and fails CI on direct construction.
//!
//! ## Provenance lives once
//!
//! Canonical provenance lives exactly once on `AbilityOutput<Composition>`
//! per ADR-0102 §6 + ADR-0105 §8 "lives-once" invariant. Blocks carry a
//! compact [`ProvenanceRef`] into that canonical envelope, not a copy.
//!
//! ## Payload shape trade-off
//!
//! [`Block::attributes`] is `serde_json::Value`, not the typed `BlockPayload`
//! enum ADR-0130 §2 names. This narrows the substrate type to the most
//! general shape that works for both today's AC (free-form attributes keyed
//! by `block_type`) and a future typed-payload migration. The W3 ability
//! contracts and renderers are the load-bearing typings for payload schemas;
//! the substrate-side shape stays general so per-block-type schema evolution
//! does not churn this file.
//!
//! See:
//! - ADR-0130 §2 (Composition primitives) and §3.1 (Custom block fallback).
//! - `.docs/plans/dos-546/phase-0/06-composition-provenance-ref.md`.
//! - `.docs/plans/dos-546/phase-0/07-custom-block-fallback-projection.md`.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::provenance::envelope::{InvocationId, Provenance, SchemaVersion};
use super::provenance::field::{FieldAttributionError, FieldPath};

// ---------------------------------------------------------------------------
// Identifier newtypes
// ---------------------------------------------------------------------------

/// Stable identifier for a [`Composition`] document.
///
/// Distinct from `provenance::envelope::CompositionId`, which labels child
/// edges inside a composed provenance graph. This identifier names the
/// composition document itself.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct CompositionDocId(pub String);

impl CompositionDocId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Stable identifier for a [`Section`] within a [`Composition`].
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct SectionId(pub String);

impl SectionId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Stable identifier for a [`Block`] within a [`Section`].
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct BlockId(pub String);

impl BlockId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Server-assigned monotonic watermark per Phase 0 artifact 02.
///
/// Increments per concurrent re-publish of the same `CompositionDocId`.
/// Surfaces refresh on stale-version rejection.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct CompositionVersion(pub u64);

impl CompositionVersion {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn bump(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }
}

// ---------------------------------------------------------------------------
// ClaimRef
// ---------------------------------------------------------------------------

/// Reference into the claim substrate (ADR-0125). Preserved exactly across
/// fallback projection per ADR-0130 §3.1 step 5 — never dereferenced to
/// backfill dropped payload fields.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct ClaimRef {
    pub claim_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claim_version: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field_path: Option<FieldPath>,
}

impl ClaimRef {
    pub fn new(claim_id: impl Into<String>) -> Self {
        Self {
            claim_id: claim_id.into(),
            claim_version: None,
            field_path: None,
        }
    }

    pub fn with_version(claim_id: impl Into<String>, version: u64) -> Self {
        Self {
            claim_id: claim_id.into(),
            claim_version: Some(version),
            field_path: None,
        }
    }

    pub fn with_field(claim_id: impl Into<String>, version: u64, field_path: FieldPath) -> Self {
        Self {
            claim_id: claim_id.into(),
            claim_version: Some(version),
            field_path: Some(field_path),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct ClaimRefIndex(pub usize);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BindingRole {
    Source,
    ComputedFrom,
    DisplayOnly,
    FeedbackTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct FieldBinding {
    pub field_path: FieldPath,
    pub role: BindingRole,
    pub claim_refs: Vec<ClaimRefIndex>,
}

// ---------------------------------------------------------------------------
// CompositionKind / EntityRef / AbilityRef / RenderHints (ADR-0130 §2)
// ---------------------------------------------------------------------------

/// Canonical composition kinds per ADR-0130 §2. Extension point via
/// [`CompositionKind::Custom`] for surface-registered kinds.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CompositionKind {
    Briefing,
    EntityPage,
    PrepPack,
    Report,
    Callout,
    Custom { type_id: String },
}

impl CompositionKind {
    pub fn type_id(&self) -> &str {
        match self {
            Self::Briefing => "briefing",
            Self::EntityPage => "entity_page",
            Self::PrepPack => "prep_pack",
            Self::Report => "report",
            Self::Callout => "callout",
            Self::Custom { type_id } => type_id.as_str(),
        }
    }
}

/// Reference to the entity a composition is about, if any. Newtype over the
/// canonical claim-substrate entity identifier; v1.4.2 ships as a transparent
/// `String` to defer entity-namespace work to claim-substrate plumbing.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct EntityRef(pub String);

impl EntityRef {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Reference to the ability that produced a composition. Mirrors
/// `Provenance.ability_name` on the canonical envelope; ships as a
/// transparent `String` newtype so name-shape changes ripple through one
/// place.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct AbilityRef(pub String);

impl AbilityRef {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Surface-neutral render density hint. Renderers map this onto native
/// spacing tokens; the substrate stays unaware of surface-specific layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Density {
    Compact,
    #[default]
    Comfortable,
    Spacious,
}

/// Surface-neutral render hints per ADR-0130 §2. Renderers MAY honor these;
/// the contract is "hints," not directives. Future fields land here as
/// optional with defaults so existing serialized compositions keep working.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
pub struct RenderHints {
    #[serde(default)]
    pub emphasis: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub density: Option<Density>,
}

// ---------------------------------------------------------------------------
// ProvenanceRef
// ---------------------------------------------------------------------------

/// Compact reference into the canonical provenance envelope that lives once
/// on `AbilityOutput<Composition>.provenance` per ADR-0102 §6 and ADR-0105
/// §8 "lives-once" invariant.
///
/// Renderers resolve a `ProvenanceRef` by fetching the canonical envelope
/// for `invocation_id` from the runtime provenance store, reading the
/// `FieldAttribution` at `field_path`, and passing the resolved envelope
/// through ADR-0108's actor-filtered renderer.
///
/// Block-builder validation: see [`Block::new`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProvenanceRef {
    pub invocation_id: InvocationId,
    pub field_path: FieldPath,
}

impl ProvenanceRef {
    pub fn new(invocation_id: InvocationId, field_path: FieldPath) -> Self {
        Self {
            invocation_id,
            field_path,
        }
    }

    /// Construct from a raw JSON Pointer string. Returns
    /// [`FieldAttributionError::InvalidFieldPath`] if the pointer is
    /// malformed.
    pub fn from_pointer(
        invocation_id: InvocationId,
        pointer: impl Into<String>,
    ) -> Result<Self, FieldAttributionError> {
        Ok(Self {
            invocation_id,
            field_path: FieldPath::new(pointer)?,
        })
    }
}

// ---------------------------------------------------------------------------
// BlockType taxonomy
// ---------------------------------------------------------------------------

/// Canonical block-type taxonomy per ADR-0130 §3 + Phase 0 artifact 05's
/// `CompositionBlockType`. Initial set; extensible via [`BlockType::Custom`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BlockType {
    AccountOverview,
    ClaimSummary,
    EvidenceList,
    HealthSnapshot,
    RelationshipMap,
    RiskCallout,
    ActionList,
    MarkdownDocument,
    /// Extension point: ability-registered type. Renderers that do not know
    /// the `type_id` apply [`project_to_nearest_known`] per ADR-0130 §3.1.
    Custom {
        type_id: String,
    },
}

impl BlockType {
    /// Stable type identifier used by the fallback projection's lexicographic
    /// final tie-break. Custom types use their declared `type_id` verbatim;
    /// canonical types use a stable snake-case identifier.
    pub fn type_id(&self) -> &str {
        match self {
            Self::AccountOverview => "account_overview",
            Self::ClaimSummary => "claim_summary",
            Self::EvidenceList => "evidence_list",
            Self::HealthSnapshot => "health_snapshot",
            Self::RelationshipMap => "relationship_map",
            Self::RiskCallout => "risk_callout",
            Self::ActionList => "action_list",
            Self::MarkdownDocument => "markdown_document",
            Self::Custom { type_id } => type_id.as_str(),
        }
    }

    pub fn is_custom(&self) -> bool {
        matches!(self, Self::Custom { .. })
    }
}

// ---------------------------------------------------------------------------
// Composition / Section / Block
// ---------------------------------------------------------------------------

/// Trust band cap applied during fallback projection per ADR-0130 §3.1 step 9.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SalienceBand {
    Critical,
    Important,
    Contextual,
    Background,
}

/// Surface-neutral block salience per ADR-0130 §2.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Salience {
    pub weight: f32,
    pub band: SalienceBand,
    pub reason: String,
}

impl Default for Salience {
    fn default() -> Self {
        Self {
            weight: 0.5,
            band: SalienceBand::Contextual,
            reason: String::new(),
        }
    }
}

/// Composition document metadata per ADR-0130 §2.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CompositionMetadata {
    pub schema_version: SchemaVersion,
    #[schemars(with = "String")]
    pub generated_at: DateTime<Utc>,
    /// Server-assigned monotonic watermark per Phase 0 artifact 02.
    pub composition_version: CompositionVersion,
    /// Ability that produced this composition; mirrors
    /// `Provenance.ability_name` on the wrapper for cheap reference.
    pub generated_by: String,
}

/// Section layout hint. Surface-neutral; renderers map this to native layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SectionLayout {
    #[default]
    Stacked,
    Grid,
    Inline,
}

/// A section groups blocks under an optional editorial label and carries
/// section-level salience per ADR-0130 §2.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Section {
    pub id: SectionId,
    /// Optional editorial heading. Matches ADR-0130 §2 `Section.label`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub blocks: Vec<Block>,
    #[serde(default)]
    pub layout: SectionLayout,
    /// Section-level salience per ADR-0130 §2. Renderers MAY use this to
    /// reorder or trim sections under surface budget constraints.
    #[serde(default)]
    pub salience: Salience,
}

impl Section {
    pub fn new(id: SectionId, blocks: Vec<Block>) -> Self {
        Self {
            id,
            label: None,
            blocks,
            layout: SectionLayout::default(),
            salience: Salience::default(),
        }
    }
}

/// Typed composition block per ADR-0130 §2.
///
/// `attributes` is a free-shape JSON value whose schema is determined by
/// `block_type`. Unknown `Custom` types render via
/// [`project_to_nearest_known`] per ADR-0130 §3.1.
///
/// **Provenance lives once.** The `provenance` field is a [`ProvenanceRef`]
/// into the canonical envelope on `AbilityOutput<Composition>`; the block
/// never embeds a copy of `Provenance`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Block {
    pub id: BlockId,
    pub block_type: BlockType,
    pub attributes: serde_json::Value,
    pub claim_refs: Vec<ClaimRef>,
    #[serde(default)]
    pub field_bindings: Vec<FieldBinding>,
    pub provenance: ProvenanceRef,
    #[serde(default)]
    pub salience: Salience,
    /// Surface-neutral render hints per ADR-0130 §2. Optional in serialized
    /// form so existing compositions keep deserializing.
    #[serde(default)]
    pub render_hints: RenderHints,
}

impl Block {
    /// Construct a block, validating that `provenance.field_path` resolves
    /// into `output.provenance.field_attributions`.
    ///
    /// Per ADR-0130 §2 "Block-builder validation": broad attribution at
    /// construction time is rejected. Producers must point at the field
    /// that actually carries the attribution for this block.
    ///
    /// `output_provenance` is the canonical envelope from the
    /// `AbilityOutput<Composition>` wrapper. When `output_provenance` is
    /// `None`, construction is deferred-validation: the block is built
    /// with the contract that the caller will run [`Block::validate_against`]
    /// once the envelope is available. This deferred path exists for
    /// composition builders that assemble blocks before sealing the
    /// envelope.
    pub fn new(
        id: BlockId,
        block_type: BlockType,
        attributes: serde_json::Value,
        claim_refs: Vec<ClaimRef>,
        provenance: ProvenanceRef,
        output_provenance: Option<&Provenance>,
    ) -> Result<Self, BlockBuildError> {
        if provenance.invocation_id == InvocationId(uuid::Uuid::nil()) {
            return Err(BlockBuildError::NilInvocationId);
        }

        if let Some(envelope) = output_provenance {
            Self::validate_field_path(&provenance, envelope)?;
        }

        Ok(Self {
            id,
            block_type,
            attributes,
            claim_refs,
            field_bindings: Vec::new(),
            provenance,
            salience: Salience::default(),
            render_hints: RenderHints::default(),
        })
    }

    /// Validate this block's `provenance.field_path` against the canonical
    /// envelope. Use when the envelope wasn't available at construction.
    pub fn validate_against(&self, envelope: &Provenance) -> Result<(), BlockBuildError> {
        Self::validate_field_path(&self.provenance, envelope)
    }

    fn validate_field_path(
        provenance: &ProvenanceRef,
        envelope: &Provenance,
    ) -> Result<(), BlockBuildError> {
        if provenance.invocation_id != envelope.invocation_id {
            return Err(BlockBuildError::InvocationMismatch);
        }

        if envelope
            .field_attributions
            .contains_key(&provenance.field_path)
        {
            return Ok(());
        }

        // Exact-path miss — accept only if a parent path covers it, per
        // ADR-0130 §2 "Resolution. ... fallback to invocation-level
        // provenance is labeled as less specific." Block construction must
        // still reject paths that do not resolve at all.
        let covered = envelope
            .field_attributions
            .keys()
            .any(|attr_path| attr_path.covers(&provenance.field_path));

        if covered {
            Ok(())
        } else {
            Err(BlockBuildError::UnresolvedFieldPath {
                field_path: provenance.field_path.as_str().to_string(),
            })
        }
    }
}

/// Composition is the surface-independent block tree per ADR-0130 §2.
///
/// Produced ONLY by abilities (ADR-0130 §1, substrate-owned authorship).
/// Surfaces never construct compositions; they receive them through the
/// normal ability-invocation path and render them.
///
/// Shape mirrors ADR-0130 §2:
/// - `id` — composition document identifier.
/// - `kind` — canonical [`CompositionKind`].
/// - `subject` — optional [`EntityRef`]; for entity-scoped compositions.
/// - `sections` — typed block tree.
/// - `salience` — top-level relevance weight; renderers MAY use this when
///   ranking compositions across multiple feeds.
/// - `generated_at` / `generated_by` — ability authorship attribution,
///   mirroring `Provenance.produced_at` / `Provenance.ability_name` on the
///   wrapper envelope.
/// - `metadata` — schema + monotonic [`CompositionVersion`] watermark per
///   Phase 0 artifact 02.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Composition {
    pub id: CompositionDocId,
    pub kind: CompositionKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<EntityRef>,
    pub sections: Vec<Section>,
    #[serde(default)]
    pub salience: Salience,
    #[schemars(with = "String")]
    pub generated_at: DateTime<Utc>,
    pub generated_by: AbilityRef,
    pub metadata: CompositionMetadata,
}

impl Composition {
    /// Construct a [`Composition`].
    ///
    /// **Visibility is `pub(crate)` by design** per ADR-0130 §1
    /// "substrate-owned authorship." Only code inside the `abilities-runtime`
    /// crate (substrate, ability implementations, ability-runtime tests) may
    /// construct a composition. Surfaces, command handlers, hooks, and other
    /// consumer crates receive `Composition` through the normal
    /// ability-invocation path and render it; they never author it.
    ///
    /// A grep-based defense-in-depth lint at
    /// `scripts/check_composition_authorship.sh` scans non-abilities
    /// call sites for `Composition::new` / `Composition {` and fails CI on
    /// drift; the Rust visibility is the primary gate.
    #[allow(clippy::too_many_arguments, dead_code)]
    pub(crate) fn new(
        id: CompositionDocId,
        kind: CompositionKind,
        subject: Option<EntityRef>,
        sections: Vec<Section>,
        salience: Salience,
        generated_at: DateTime<Utc>,
        generated_by: AbilityRef,
        metadata: CompositionMetadata,
    ) -> Self {
        Self {
            id,
            kind,
            subject,
            sections,
            salience,
            generated_at,
            generated_by,
            metadata,
        }
    }

    /// Construct an empty `Composition` (no sections, no salience, no
    /// subject) at a specific version + timestamp. Substrate chokepoint
    /// callers (e.g. `commit_composition`) use this to materialize the
    /// initial shell before any ability fills it in; fixture/test code
    /// uses it to seed deterministic composition rows.
    ///
    /// This is the *only* `pub` constructor on `Composition` — all
    /// substantive composition construction still flows through the
    /// `pub(crate) new(...)` path inside `abilities-runtime` per ADR-0130
    /// §1. `empty` is a constrained surface (id + version + timestamp
    /// only) that can't smuggle authored content across the substrate
    /// boundary.
    pub fn empty(
        id: CompositionDocId,
        version: CompositionVersion,
        generated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            kind: CompositionKind::EntityPage,
            subject: None,
            sections: Vec::new(),
            salience: Salience::default(),
            generated_at,
            generated_by: AbilityRef::new("test_composer"),
            metadata: CompositionMetadata {
                schema_version: SchemaVersion(1),
                generated_at,
                composition_version: version,
                generated_by: "test_composer".to_string(),
            },
        }
    }

    /// Iterate every block in the composition in stable order.
    pub fn blocks(&self) -> impl Iterator<Item = &Block> {
        self.sections
            .iter()
            .flat_map(|section| section.blocks.iter())
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors surfaced by [`Block::new`] and [`Block::validate_against`].
#[derive(Debug, Error, PartialEq)]
pub enum BlockBuildError {
    #[error("ProvenanceRef.invocation_id is nil; blocks must reference a real ability invocation")]
    NilInvocationId,
    #[error("ProvenanceRef.invocation_id does not match the canonical envelope")]
    InvocationMismatch,
    #[error("ProvenanceRef.field_path {field_path:?} does not resolve in the canonical envelope's field_attributions")]
    UnresolvedFieldPath { field_path: String },
    #[error(transparent)]
    FieldPath(#[from] FieldAttributionError),
}

// ---------------------------------------------------------------------------
// Custom block fallback projection (ADR-0130 §3.1 + Phase 0 artifact 07)
// ---------------------------------------------------------------------------

/// Schema descriptor for a known block type. Drives [`project_to_nearest_known`]
/// scoring per ADR-0130 §3.1 step 1.
#[derive(Debug, Clone)]
pub struct BlockDescriptor {
    pub block_type: BlockType,
    pub composition_kind: Option<String>,
    pub required_pointers: BTreeSet<String>,
    pub optional_pointers: BTreeSet<String>,
    pub render_annotations: BTreeSet<String>,
    pub type_namespace: Option<String>,
}

impl BlockDescriptor {
    pub fn new(block_type: BlockType) -> Self {
        Self {
            block_type,
            composition_kind: None,
            required_pointers: BTreeSet::new(),
            optional_pointers: BTreeSet::new(),
            render_annotations: BTreeSet::new(),
            type_namespace: None,
        }
    }
}

/// Schema descriptor for the unknown block being projected.
#[derive(Debug, Clone, Default)]
pub struct UnknownBlockSchema {
    pub type_id: String,
    pub composition_kind: Option<String>,
    pub required_pointers: BTreeSet<String>,
    pub optional_pointers: BTreeSet<String>,
    pub render_annotations: BTreeSet<String>,
}

/// Outcome of [`project_to_nearest_known`].
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectionResult {
    pub original_type: String,
    pub selected_type: BlockType,
    pub projected_attributes: serde_json::Value,
    pub claim_refs: Vec<ClaimRef>,
    pub provenance: ProvenanceRef,
    pub trust_band_cap: TrustBandCap,
    pub banner: FallbackBanner,
    pub diagnostic: ProjectionDiagnostic,
}

/// Trust band cap per ADR-0130 §3.1 step 9. Fallback MUST NOT upgrade trust.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TrustBandCap {
    NeedsVerification,
}

/// Non-dismissible banner per ADR-0130 §3.1 step 8. Banner text uses
/// product vocabulary; internal terms MUST NOT appear in user-visible copy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FallbackBanner {
    pub text: String,
    pub dismissible: bool,
}

/// Operator-visibility diagnostic per Phase 0 artifact 07 "Failure Semantics".
/// MUST NOT contain dropped payload values.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectionDiagnostic {
    pub original_type: String,
    pub selected_type: String,
    pub projected_pointer_count: usize,
    pub dropped_pointer_count: usize,
    pub reason: &'static str,
}

#[derive(Debug, Clone, Copy, Default)]
struct NearestTypeScore {
    kind_match: u32,
    required_overlap: u32,
    optional_overlap: u32,
    annotation_similarity: u32,
    namespace_similarity: u32,
}

impl NearestTypeScore {
    fn total(self) -> u32 {
        self.kind_match
            + self.required_overlap
            + self.optional_overlap
            + self.annotation_similarity
            + self.namespace_similarity
    }
}

/// Deterministic schema-bounded projection of an unknown block onto the
/// nearest known type per ADR-0130 §3.1 and Phase 0 artifact 07.
///
/// **Determinism:** same input → same output across runs, machines, and
/// registry iteration order. Final tie-break is lexicographic on the
/// candidate's `block_type.type_id()`.
///
/// **Privacy boundary:** unknown payload fields outside the intersected
/// pointer set are dropped, not displayed. `claim_refs` and `provenance`
/// are preserved exactly.
pub fn project_to_nearest_known(
    unknown_block: &Block,
    unknown_schema: &UnknownBlockSchema,
    known_types: &[BlockDescriptor],
) -> ProjectionResult {
    let nearest = select_nearest_known_type(unknown_schema, known_types);

    let original_type = unknown_block.block_type.type_id().to_string();

    let Some(nearest) = nearest else {
        return generic_text_fallback(unknown_block, original_type);
    };

    let intersected: BTreeSet<&String> = unknown_schema
        .required_pointers
        .iter()
        .chain(unknown_schema.optional_pointers.iter())
        .filter(|ptr| {
            nearest.required_pointers.contains(*ptr) || nearest.optional_pointers.contains(*ptr)
        })
        .collect();

    if intersected.is_empty() {
        return generic_text_fallback(unknown_block, original_type);
    }

    let projected_attributes = project_pointers(&unknown_block.attributes, &intersected);
    let projected_pointer_count = intersected.len();
    let dropped_pointer_count = unknown_schema
        .required_pointers
        .len()
        .saturating_add(unknown_schema.optional_pointers.len())
        .saturating_sub(projected_pointer_count);

    let selected_type_id = nearest.block_type.type_id().to_string();
    let banner_text = format!("Rendered as {selected_type_id} — payload may be incomplete");

    ProjectionResult {
        original_type: original_type.clone(),
        selected_type: nearest.block_type.clone(),
        projected_attributes,
        claim_refs: unknown_block.claim_refs.clone(),
        provenance: unknown_block.provenance.clone(),
        trust_band_cap: TrustBandCap::NeedsVerification,
        banner: FallbackBanner {
            text: banner_text,
            dismissible: false,
        },
        diagnostic: ProjectionDiagnostic {
            original_type,
            selected_type: selected_type_id,
            projected_pointer_count,
            dropped_pointer_count,
            reason: "unknown_block_type",
        },
    }
}

fn generic_text_fallback(unknown_block: &Block, original_type: String) -> ProjectionResult {
    let selected_type_id = "dailyos/text".to_string();
    let dropped = unknown_block
        .attributes
        .as_object()
        .map(|obj| obj.len())
        .unwrap_or(0);
    ProjectionResult {
        original_type: original_type.clone(),
        selected_type: BlockType::Custom {
            type_id: selected_type_id.clone(),
        },
        projected_attributes: serde_json::Value::Object(serde_json::Map::new()),
        claim_refs: unknown_block.claim_refs.clone(),
        provenance: unknown_block.provenance.clone(),
        trust_band_cap: TrustBandCap::NeedsVerification,
        banner: FallbackBanner {
            text: "Rendered as dailyos/text — payload may be incomplete".to_string(),
            dismissible: false,
        },
        diagnostic: ProjectionDiagnostic {
            original_type,
            selected_type: selected_type_id,
            projected_pointer_count: 0,
            dropped_pointer_count: dropped,
            reason: "unknown_block_type",
        },
    }
}

fn select_nearest_known_type<'a>(
    unknown_schema: &UnknownBlockSchema,
    candidates: &'a [BlockDescriptor],
) -> Option<&'a BlockDescriptor> {
    if candidates.is_empty() {
        return None;
    }

    let mut scored: Vec<(&BlockDescriptor, NearestTypeScore)> = candidates
        .iter()
        .map(|candidate| (candidate, score_candidate(unknown_schema, candidate)))
        .collect();

    // Tie-break per ADR-0130 §3.1 step 1 + Phase 0 artifact 07 §"Tie-break":
    // total desc, kind_match desc, required_overlap desc, optional_overlap
    // desc, annotation_similarity desc, then lexicographic type_id asc.
    scored.sort_by(|(a_desc, a_score), (b_desc, b_score)| {
        b_score
            .total()
            .cmp(&a_score.total())
            .then(b_score.kind_match.cmp(&a_score.kind_match))
            .then(b_score.required_overlap.cmp(&a_score.required_overlap))
            .then(b_score.optional_overlap.cmp(&a_score.optional_overlap))
            .then(
                b_score
                    .annotation_similarity
                    .cmp(&a_score.annotation_similarity),
            )
            .then(a_desc.block_type.type_id().cmp(b_desc.block_type.type_id()))
    });

    let (winner, score) = scored.first()?;
    if score.total() == 0 {
        return None;
    }
    Some(*winner)
}

fn score_candidate(unknown: &UnknownBlockSchema, candidate: &BlockDescriptor) -> NearestTypeScore {
    let mut score = NearestTypeScore::default();

    if let (Some(unknown_kind), Some(candidate_kind)) =
        (&unknown.composition_kind, &candidate.composition_kind)
    {
        if unknown_kind == candidate_kind {
            score.kind_match = 100;
        }
    }

    score.required_overlap = unknown
        .required_pointers
        .intersection(&candidate.required_pointers)
        .count() as u32
        * 10;

    score.optional_overlap = unknown
        .optional_pointers
        .intersection(&candidate.optional_pointers)
        .count() as u32
        * 2;

    let annotation_overlap = unknown
        .render_annotations
        .intersection(&candidate.render_annotations)
        .count() as u32;
    score.annotation_similarity = annotation_overlap.saturating_mul(4).min(20);

    score.namespace_similarity =
        namespace_similarity(&unknown.type_id, candidate.type_namespace.as_deref());

    score
}

fn namespace_similarity(unknown_type: &str, candidate_ns: Option<&str>) -> u32 {
    let Some(candidate_ns) = candidate_ns else {
        return 0;
    };
    if unknown_type.starts_with(candidate_ns) {
        5
    } else {
        0
    }
}

/// Reconstruct only the pointers in `keep` from `source`. Container objects
/// are rebuilt to hold allowed leaves; siblings are NEVER copied wholesale.
/// Per ADR-0130 §3.1 step 4.
fn project_pointers(source: &serde_json::Value, keep: &BTreeSet<&String>) -> serde_json::Value {
    let mut out = serde_json::Map::new();

    for pointer in keep {
        let Some(value) = source.pointer(pointer) else {
            continue;
        };

        // Decompose the JSON pointer into path segments and reconstruct.
        let segments: Vec<&str> = pointer.split('/').filter(|s| !s.is_empty()).collect();
        if segments.is_empty() {
            continue;
        }

        insert_at_path(&mut out, &segments, value.clone());
    }

    serde_json::Value::Object(out)
}

fn insert_at_path(
    target: &mut serde_json::Map<String, serde_json::Value>,
    segments: &[&str],
    value: serde_json::Value,
) {
    if segments.is_empty() {
        return;
    }
    if segments.len() == 1 {
        target.insert(segments[0].to_string(), value);
        return;
    }

    let head = segments[0];
    let entry = target
        .entry(head.to_string())
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));

    if let serde_json::Value::Object(child) = entry {
        insert_at_path(child, &segments[1..], value);
    } else {
        // Conflict: existing leaf at intermediate path. Overwrite with a
        // fresh container — projection is target-shape driven.
        let mut child = serde_json::Map::new();
        insert_at_path(&mut child, &segments[1..], value);
        *entry = serde_json::Value::Object(child);
    }
}

// ---------------------------------------------------------------------------
// Composition-fingerprint helper (deterministic round-trip indicator)
// ---------------------------------------------------------------------------

/// Stable JSON canonicalization for fingerprint tests. Deterministic key
/// order via `BTreeMap` traversal.
pub fn fingerprint_json(composition: &Composition) -> Result<String, serde_json::Error> {
    let value = serde_json::to_value(composition)?;
    let canonical = canonicalize(value);
    serde_json::to_string(&canonical)
}

fn canonicalize(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let sorted: BTreeMap<String, serde_json::Value> =
                map.into_iter().map(|(k, v)| (k, canonicalize(v))).collect();
            let mut out = serde_json::Map::new();
            for (k, v) in sorted {
                out.insert(k, v);
            }
            serde_json::Value::Object(out)
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.into_iter().map(canonicalize).collect())
        }
        other => other,
    }
}

impl ProjectionResult {
    pub fn projected_pointer_count(&self) -> usize {
        self.diagnostic.projected_pointer_count
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_provenance_ref() -> ProvenanceRef {
        ProvenanceRef::from_pointer(
            InvocationId(uuid::Uuid::from_u128(
                0x1234_5678_90ab_cdef_1122_3344_5566_7788,
            )),
            "/sections/0/blocks/0",
        )
        .unwrap()
    }

    fn sample_block(id: &str) -> Block {
        Block {
            id: BlockId::new(id),
            block_type: BlockType::AccountOverview,
            attributes: json!({"name": "Acme"}),
            claim_refs: vec![ClaimRef::new("claim-1")],
            field_bindings: Vec::new(),
            provenance: sample_provenance_ref(),
            salience: Salience::default(),
            render_hints: RenderHints::default(),
        }
    }

    fn sample_generated_at() -> DateTime<Utc> {
        chrono::TimeZone::with_ymd_and_hms(&chrono::Utc, 2026, 5, 11, 0, 0, 0).unwrap()
    }

    fn sample_composition() -> Composition {
        Composition::new(
            CompositionDocId::new("comp-1"),
            CompositionKind::EntityPage,
            Some(EntityRef::new("account:acme")),
            vec![Section::new(
                SectionId::new("sec-1"),
                vec![sample_block("blk-1")],
            )],
            Salience::default(),
            sample_generated_at(),
            AbilityRef::new("test.ability"),
            CompositionMetadata {
                schema_version: SchemaVersion(1),
                generated_at: sample_generated_at(),
                composition_version: CompositionVersion::new(1),
                generated_by: "test.ability".to_string(),
            },
        )
    }

    #[test]
    fn composition_roundtrips_through_serde() {
        let original = sample_composition();
        let json = serde_json::to_string(&original).expect("serialize");
        let decoded: Composition = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original, decoded);
    }

    #[test]
    fn provenance_ref_stays_compact() {
        let pref = sample_provenance_ref();
        let bytes = serde_json::to_vec(&pref).expect("serialize");
        // ADR-0130 §2: ~80-200 bytes typical; assert the 256-byte ceiling.
        assert!(
            bytes.len() <= 256,
            "ProvenanceRef serialized to {} bytes; cap is 256",
            bytes.len()
        );
    }

    #[test]
    fn block_construction_rejects_nil_invocation() {
        let bad_ref = ProvenanceRef::new(
            InvocationId(uuid::Uuid::nil()),
            FieldPath::new("/x").unwrap(),
        );
        let err = Block::new(
            BlockId::new("b"),
            BlockType::AccountOverview,
            json!({}),
            vec![],
            bad_ref,
            None,
        )
        .expect_err("nil invocation must reject");
        assert_eq!(err, BlockBuildError::NilInvocationId);
    }

    /// Build a minimal but real Provenance envelope with a single field
    /// attribution at `/sections/0/blocks/0`. Used by the block-validation
    /// tests to exercise the OK and rejection paths against an actual
    /// envelope rather than the deferred (None) path.
    fn fixture_envelope_with_attribution() -> Provenance {
        use crate::abilities::provenance::{FieldAttribution, SubjectAttribution, SubjectRef};

        let subject = SubjectAttribution::direct_confident(SubjectRef::Account("acct-1".into()));
        let mut attributions = BTreeMap::new();
        attributions.insert(
            FieldPath::new("/sections/0/blocks/0").unwrap(),
            FieldAttribution::constant(subject.clone()),
        );

        let mut envelope = crate::abilities::provenance::envelope::provenance_for_test(
            "fixture",
            sample_generated_at(),
            subject,
            Vec::new(),
            Vec::new(),
            attributions,
            None,
            Vec::new(),
        );
        // Pin the invocation_id to match sample_provenance_ref so resolution
        // succeeds on the OK path.
        envelope.invocation_id = InvocationId(uuid::Uuid::from_u128(
            0x1234_5678_90ab_cdef_1122_3344_5566_7788,
        ));
        envelope
    }

    #[test]
    fn block_construction_validates_field_path_when_envelope_provided() {
        let envelope = fixture_envelope_with_attribution();
        let pref = sample_provenance_ref();

        let block = Block::new(
            BlockId::new("b"),
            BlockType::AccountOverview,
            json!({}),
            vec![],
            pref,
            Some(&envelope),
        )
        .expect("OK path: field_path resolves into envelope.field_attributions");
        assert_eq!(block.block_type.type_id(), "account_overview");
    }

    #[test]
    fn block_construction_rejects_field_path_outside_envelope() {
        let envelope = fixture_envelope_with_attribution();
        let bad_ref = ProvenanceRef::new(
            envelope.invocation_id,
            FieldPath::new("/never_attributed").unwrap(),
        );

        let err = Block::new(
            BlockId::new("b"),
            BlockType::AccountOverview,
            json!({}),
            vec![],
            bad_ref,
            Some(&envelope),
        )
        .expect_err("unresolved field_path must reject");
        match err {
            BlockBuildError::UnresolvedFieldPath { field_path } => {
                assert_eq!(field_path, "/never_attributed");
            }
            other => panic!("expected UnresolvedFieldPath, got {other:?}"),
        }
    }

    #[test]
    fn projection_is_deterministic_across_runs() {
        let unknown = Block {
            id: BlockId::new("u"),
            block_type: BlockType::Custom {
                type_id: "dailyos/unknown-foo".to_string(),
            },
            attributes: json!({
                "title": "T",
                "body": "B",
                "secret_email": "user@example.com",
            }),
            claim_refs: vec![ClaimRef::new("c-1")],
            field_bindings: Vec::new(),
            provenance: sample_provenance_ref(),
            salience: Salience::default(),
            render_hints: RenderHints::default(),
        };

        let unknown_schema = UnknownBlockSchema {
            type_id: "dailyos/unknown-foo".to_string(),
            composition_kind: Some("entity_page".to_string()),
            required_pointers: ["/title".to_string(), "/body".to_string()]
                .into_iter()
                .collect(),
            optional_pointers: BTreeSet::new(),
            render_annotations: BTreeSet::new(),
        };

        let mut markdown_descriptor = BlockDescriptor::new(BlockType::MarkdownDocument);
        markdown_descriptor.composition_kind = Some("entity_page".to_string());
        markdown_descriptor.required_pointers = ["/title".to_string(), "/body".to_string()]
            .into_iter()
            .collect();

        let mut action_descriptor = BlockDescriptor::new(BlockType::ActionList);
        action_descriptor.composition_kind = Some("entity_page".to_string());
        action_descriptor.required_pointers = ["/title".to_string()].into_iter().collect();

        let candidates_a = vec![markdown_descriptor.clone(), action_descriptor.clone()];
        let candidates_b = vec![action_descriptor, markdown_descriptor];

        let result_a = project_to_nearest_known(&unknown, &unknown_schema, &candidates_a);
        let result_b = project_to_nearest_known(&unknown, &unknown_schema, &candidates_b);

        assert_eq!(result_a, result_b, "projection must be order-independent");
        assert_eq!(result_a.selected_type, BlockType::MarkdownDocument);
        // Dropped fields MUST NOT leak.
        assert!(
            result_a
                .projected_attributes
                .pointer("/secret_email")
                .is_none(),
            "dropped payload field leaked into projection"
        );
        // claim_refs preserved exactly.
        assert_eq!(result_a.claim_refs.len(), 1);
        assert_eq!(result_a.claim_refs[0].claim_id, "c-1");
        // provenance preserved exactly.
        assert_eq!(result_a.provenance, unknown.provenance);
        // trust band cap applied.
        assert_eq!(result_a.trust_band_cap, TrustBandCap::NeedsVerification);
        // banner is non-dismissible and uses product vocabulary.
        assert!(!result_a.banner.dismissible);
        assert!(result_a.banner.text.contains("payload may be incomplete"));
    }

    #[test]
    fn projection_falls_back_to_generic_text_when_no_intersection() {
        let unknown = Block {
            id: BlockId::new("u"),
            block_type: BlockType::Custom {
                type_id: "dailyos/unknown-bar".to_string(),
            },
            attributes: json!({"weird": "value"}),
            claim_refs: vec![ClaimRef::new("c-1")],
            field_bindings: Vec::new(),
            provenance: sample_provenance_ref(),
            salience: Salience::default(),
            render_hints: RenderHints::default(),
        };

        let unknown_schema = UnknownBlockSchema {
            type_id: "dailyos/unknown-bar".to_string(),
            composition_kind: None,
            required_pointers: BTreeSet::new(),
            optional_pointers: BTreeSet::new(),
            render_annotations: BTreeSet::new(),
        };

        let result = project_to_nearest_known(&unknown, &unknown_schema, &[]);
        assert_eq!(
            result.selected_type,
            BlockType::Custom {
                type_id: "dailyos/text".to_string()
            }
        );
        assert_eq!(result.projected_pointer_count(), 0);
        assert_eq!(result.claim_refs.len(), 1, "claim_refs preserved");
        assert_eq!(
            result.provenance, unknown.provenance,
            "provenance preserved"
        );
    }

    #[test]
    fn fingerprint_is_stable_across_calls() {
        let comp = sample_composition();
        let fp_1 = fingerprint_json(&comp).expect("fingerprint 1");
        let fp_2 = fingerprint_json(&comp).expect("fingerprint 2");
        assert_eq!(fp_1, fp_2, "fingerprint must be stable");
    }

    #[test]
    fn composition_version_is_monotonic() {
        let v1 = CompositionVersion::new(1);
        let v2 = v1.bump().expect("version 2");
        let v3 = v2.bump().expect("version 3");
        assert!(v1 < v2);
        assert!(v2 < v3);
    }

    #[test]
    fn composition_widened_fields_roundtrip() {
        let original = sample_composition();
        let json = serde_json::to_string(&original).expect("serialize widened composition");
        // Verbatim ADR-0130 §2 fields must appear at the top level.
        assert!(json.contains("\"kind\""));
        assert!(json.contains("\"subject\""));
        assert!(json.contains("\"salience\""));
        assert!(json.contains("\"generated_at\""));
        assert!(json.contains("\"generated_by\""));
        let decoded: Composition = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original, decoded);
        assert_eq!(decoded.kind.type_id(), "entity_page");
        assert_eq!(
            decoded.subject.as_ref().map(|s| s.as_str()),
            Some("account:acme")
        );
        assert_eq!(decoded.generated_by.as_str(), "test.ability");
    }

    #[test]
    fn section_label_and_salience_roundtrip() {
        let mut section = Section::new(SectionId::new("sec-2"), vec![sample_block("blk-2")]);
        section.label = Some("Strategic Risks".to_string());
        section.salience = Salience {
            weight: 0.9,
            band: SalienceBand::Critical,
            reason: "high-priority cluster".to_string(),
        };

        let json = serde_json::to_string(&section).expect("serialize");
        assert!(json.contains("\"label\""));
        assert!(json.contains("\"salience\""));
        let decoded: Section = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded.label.as_deref(), Some("Strategic Risks"));
        assert_eq!(decoded.salience.band, SalienceBand::Critical);
    }

    #[test]
    fn block_render_hints_default_and_roundtrip() {
        let block = sample_block("blk-3");
        // Default render_hints: emphasis=false, density=None.
        assert!(!block.render_hints.emphasis);
        assert_eq!(block.render_hints.density, None);

        let json = serde_json::to_string(&block).expect("serialize");
        let decoded: Block = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded.render_hints, RenderHints::default());

        // Non-default emphasis + density round-trips.
        let mut emphasized = block.clone();
        emphasized.render_hints = RenderHints {
            emphasis: true,
            density: Some(Density::Compact),
        };
        let json2 = serde_json::to_string(&emphasized).expect("serialize");
        let decoded2: Block = serde_json::from_str(&json2).expect("deserialize");
        assert!(decoded2.render_hints.emphasis);
        assert_eq!(decoded2.render_hints.density, Some(Density::Compact));
    }

    #[test]
    fn composition_kind_custom_carries_type_id() {
        let custom = CompositionKind::Custom {
            type_id: "research_brief".to_string(),
        };
        assert_eq!(custom.type_id(), "research_brief");
        let json = serde_json::to_string(&custom).expect("serialize");
        let decoded: CompositionKind = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded, custom);
    }

    #[test]
    fn projection_tie_break_is_lexicographic() {
        // Two candidates with identical scores must resolve by
        // lexicographically smaller type_id.
        let unknown_schema = UnknownBlockSchema {
            type_id: "dailyos/test".to_string(),
            composition_kind: None,
            required_pointers: ["/x".to_string()].into_iter().collect(),
            optional_pointers: BTreeSet::new(),
            render_annotations: BTreeSet::new(),
        };

        let mut a = BlockDescriptor::new(BlockType::AccountOverview);
        a.required_pointers = ["/x".to_string()].into_iter().collect();
        let mut b = BlockDescriptor::new(BlockType::RiskCallout);
        b.required_pointers = ["/x".to_string()].into_iter().collect();

        // account_overview < risk_callout lexicographically.
        let candidates = vec![b, a];
        let winner = select_nearest_known_type(&unknown_schema, &candidates).unwrap();
        assert_eq!(winner.block_type, BlockType::AccountOverview);
    }
}
