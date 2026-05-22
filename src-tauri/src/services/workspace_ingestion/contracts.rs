//! Frozen shared-type surface for the workspace ingestion service (v1.4.5 W1-A).
//!
//! Every subsequent v1.4.5 lane (W1-B/W1-C/W2-A/W3-A/W3-B/W3-C) imports types and
//! traits from here. The signature shape is frozen by the L0 packet; bodies of
//! `WorkspaceCategory::{as_slug, from_slug}` are the implementation work but the
//! contract surface itself is wave-plan-amendment territory, not implementation
//! freedom.
//!
//! W1-A deliberately consumes canonical substrate primitives — `SourceAttribution`,
//! `WorkspaceFileKind`, `SubjectRef`, `ClaimType`, `ClaimSensitivity`, `DataSource`
//! — from `abilities_runtime` rather than reinventing them. The CI grep gate at
//! `tests/no_substrate_reinvention.rs` enforces this for future lanes.
//!
//! Reconciliation of the dual `SubjectRef` enums (the abilities-runtime
//! variant-tuple imported here vs the db-layer struct-variant at
//! `src/db/claim_invalidation.rs:58`) is tracked as a Codebase Maintenance ticket
//! in project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`. W1-A imports the
//! abilities-runtime variant by deliberate choice; downstream lanes inherit.

use std::fs::File;
use std::path::PathBuf;

// Top-level `pub use` re-exports of canonical substrate primitives consumed
// by W1-A contracts. Downstream lanes can `use
// services::workspace_ingestion::contracts::*` and get the full surface
// without separately resolving each substrate path. The CI grep gate at
// `tests/no_substrate_reinvention.rs` explicitly exempts `pub use` — it only
// fires on `pub struct/enum/trait` definitions, which is the reinvention
// shape that fired three times during L0.
pub use abilities_runtime::abilities::claims::ClaimType;
pub use abilities_runtime::abilities::provenance::source::{
    DataSource, SourceAttribution, WorkspaceFileKind,
};
pub use abilities_runtime::abilities::provenance::subject::SubjectRef;
pub use abilities_runtime::types::ClaimSensitivity;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::lifecycle;

/// Atomic identity of a workspace file. Populated by `registry::open_validated`
/// (W1-B) immediately after the `O_NOFOLLOW`/`FILE_FLAG_OPEN_REPARSE_POINT` open,
/// so device+inode are captured against the actual file handle (closes the
/// validate-then-read TOCTOU race per ADR-0098  security gate).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileIdentity {
    pub canonical_path: PathBuf,
    pub device: u64,
    pub inode: u64,
}

/// Entity-relative sub-category for AI navigation (cycle 8 Option B-prime).
///
/// **Wire shape:** serializes per `#[serde(tag = "kind", content = "name")]` so
/// known variants render as `{"kind": "presentations"}` (no `content` field
/// because the variant carries no data) and `Other(s)` renders as
/// `{"kind": "other", "name": "<lowercase_ascii_slug>"}`.
///
/// **Slug responsibilities:**
/// - `as_slug()` / `from_slug()` enforce only the lexical slug shape (lowercase
///   ASCII matching `^[a-z0-9_-]+$`).
/// - Per-entity registry-allowedness (what `Other` slugs are valid for a given
///   entity type) is W1-B's `WorkspaceCategoryRegistry::validate` boundary, not
///   `WorkspaceCategory`'s.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "name")]
pub enum WorkspaceCategory {
    Presentations,
    Transcripts,
    Meetings,
    Notes,
    Contracts,
    Attachments,
    Other(String),
}

impl WorkspaceCategory {
    /// Canonical wire+path slug. Single source of truth used by
    /// `PlaceDocumentInput.category`, `PlaceDocumentReceipt.category`,
    /// `PlacementError::CategoryNotAllowed.allowed`,
    /// `WorkspaceGraphQueryInput.category_filter`, `file_links[].category`,
    /// AND the filesystem path segment.
    pub fn as_slug(&self) -> &str {
        match self {
            Self::Presentations => "presentations",
            Self::Transcripts => "transcripts",
            Self::Meetings => "meetings",
            Self::Notes => "notes",
            Self::Contracts => "contracts",
            Self::Attachments => "attachments",
            Self::Other(s) => s.as_str(),
        }
    }

    /// Parses a slug into a known variant or `Other(s)` if `s` passes the
    /// lexical shape check (`^[a-z0-9_-]+$`). Returns `None` for slugs that
    /// fail the shape check. Does NOT verify per-entity registry-allowedness —
    /// that is W1-B's `WorkspaceCategoryRegistry::validate` boundary.
    pub fn from_slug(s: &str) -> Option<Self> {
        match s {
            "presentations" => Some(Self::Presentations),
            "transcripts" => Some(Self::Transcripts),
            "meetings" => Some(Self::Meetings),
            "notes" => Some(Self::Notes),
            "contracts" => Some(Self::Contracts),
            "attachments" => Some(Self::Attachments),
            other if is_valid_slug_shape(other) => Some(Self::Other(other.to_string())),
            _ => None,
        }
    }
}

fn is_valid_slug_shape(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

/// Path/format/race rejection reasons. W1-B emits path variants from
/// `open_validated`; W2-A's pipeline emits size/format variants after the open
/// succeeds. The W2-A pipeline owns signal emission at the boundary (W3-B's
/// `SignalEmitter` impl maps `RejectionReason` → `WorkspaceFileRejected`
/// signal); W1-B and W1-C never call `emit_signal` directly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RejectionReason {
    PathTraversalAttempt,
    SymlinkRefused,
    SymlinkRaced,
    OutsideWorkspace,
    FileTooLarge,
    UnsupportedFormat,
}

/// Extraction shape returned by W3-A's `Extractor::extract`. Wrapper that
/// bundles extractor output with workspace-specific metadata BEFORE conversion
/// to the canonical commit shape (`dailyos_lib::services::claims::ClaimProposal`
/// at `services/claims.rs:74`, 18 fields).
///
/// W3-A is responsible for the `into_commit(self, actor: &str) -> ClaimProposal`
/// mapping that translates this extraction shape into the commit shape, packing
/// workspace-specific bits (`lifecycle_state`, `initial_source_reliability`,
/// `ingestion_run_id`) into the commit shape's `metadata_json` field per
/// existing `services::claims` convention.
///
/// W1-A defines the extraction shape only; downstream commit happens through
/// `services::claims::commit_claim`, the single writer of `intelligence_claims`.
#[derive(Debug, Clone)]
pub struct WorkspaceClaimProposal {
    pub claim_type: ClaimType,
    /// `subject_ref` entity ids are canonical v1.4.0 entity-slug format
    /// (consumed by `services::claims::commit_claim`).
    pub subject_ref: SubjectRef,
    pub content: serde_json::Value,
    pub source_asof: DateTime<Utc>,
    pub data_source: DataSource,
    pub sensitivity: ClaimSensitivity,
    /// Canonical 7-field `SourceAttribution` consumed from existing substrate at
    /// `abilities_runtime::abilities::provenance::source::SourceAttribution`
    /// (`provenance/source.rs:303`). W3-A constructs via
    /// `SourceAttribution::new(
    ///     DataSource::WorkspaceFile { kind },
    ///     vec![abilities_runtime::abilities::provenance::source::SourceIdentifier::Document {
    ///         document_id: abilities_runtime::abilities::provenance::DocumentId::new(file_id),
    ///         chunk_id: None,
    ///     }],
    ///     observed_at,
    ///     Some(source_asof),
    ///     evidence_weight, // f32, validated 0.0..=1.0 by ::new
    ///     None,            // synthesis_marker: None for workspace files
    /// )`.
    pub source_attribution: SourceAttribution,
    /// Workspace-specific run linkage. NOT a `SourceIdentifier` — separate
    /// concept (the run id tracks "which ingestion attempt produced this
    /// proposal", not "what document is this evidence from").
    pub ingestion_run_id: String,
    pub lifecycle_state: lifecycle::LifecycleState,
    /// One trust-factor input populated by W3-A's extractor (normalized
    /// 0.0–1.0 derived from extraction confidence). Satisfies wave-plan
    /// §Architecture invariants line 323 "at least one trust factor input from
    /// creation". At `services::claims::commit_claim` time this single field
    /// is combined with corroboration query, contradiction query, freshness
    /// decay per `data_source`/`source_asof`, etc. to construct the full
    /// `abilities_runtime::abilities::trust::types::TrustFactorInputs`
    /// (11-field struct). W1-A does NOT reinvent any trust-factor type
    /// (V1.1's `TrustFactorInput` was caught in cycle 2 and reverted).
    pub initial_source_reliability: f64,
}

/// Extraction trait. W3-A's `WorkspaceExtractor` implements this; W2-A
/// constructs the pipeline with `NullExtractor` by default and `wiring.rs`
/// swaps in `WorkspaceExtractor` after W3-A merges.
pub trait Extractor: Send + Sync {
    fn extract(
        &self,
        file: &File,
        identity: &FileIdentity,
        source_type: WorkspaceFileKind,
    ) -> Vec<WorkspaceClaimProposal>;
}

/// Signal-emission trait. Each method maps 1:1 to a
/// `SignalType::WorkspaceFile*` variant in W3-B's `signals/policy_registry.rs`
/// (5 methods → 5 variants). W3-B's `WorkspaceSignalEmitter` implements this;
/// W2-A constructs the pipeline with `NullSignalEmitter` by default and
/// `wiring.rs` swaps in `WorkspaceSignalEmitter` after W3-B merges. W1-C's
/// `link::override_link` consumes `&dyn SignalEmitter` for `emit_link_changed`.
pub trait SignalEmitter: Send + Sync {
    fn emit_file_ingested(
        &self,
        file_id: &str,
        file_hash: &str,
        ingestion_run_id: &str,
        entity_id: Option<&str>,
    );
    fn emit_file_rejected(&self, file_id: Option<&str>, reason: RejectionReason);
    fn emit_file_pending_entity_assignment(&self, file_id: &str, ingestion_run_id: &str);
    fn emit_file_quarantined(&self, file_id: &str, reason: &str, actor: &str);
    fn emit_link_changed(&self, file_id: &str, entity_id: &str, actor: &str);
}

/// No-op extractor. Lets W1 + W2 compile against the `Extractor` trait before
/// W3-A's real impl lands.
pub struct NullExtractor;

impl Extractor for NullExtractor {
    fn extract(
        &self,
        _file: &File,
        _identity: &FileIdentity,
        _source_type: WorkspaceFileKind,
    ) -> Vec<WorkspaceClaimProposal> {
        Vec::new()
    }
}

/// No-op signal emitter. Lets W1 (including `link::override_link`) compile
/// against the `SignalEmitter` trait before W3-B's real impl lands.
pub struct NullSignalEmitter;

impl SignalEmitter for NullSignalEmitter {
    fn emit_file_ingested(
        &self,
        _file_id: &str,
        _file_hash: &str,
        _ingestion_run_id: &str,
        _entity_id: Option<&str>,
    ) {
    }

    fn emit_file_rejected(&self, _file_id: Option<&str>, _reason: RejectionReason) {}

    fn emit_file_pending_entity_assignment(&self, _file_id: &str, _ingestion_run_id: &str) {}

    fn emit_file_quarantined(&self, _file_id: &str, _reason: &str, _actor: &str) {}

    fn emit_link_changed(&self, _file_id: &str, _entity_id: &str, _actor: &str) {}
}
