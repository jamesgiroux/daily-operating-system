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
use std::io;
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

use crate::db::ActionDb;
use crate::entity::EntityType;
use crate::services::context::ServiceContext;
use crate::signals::propagation::PropagationEngine;

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
    ManagedOutputRoot,
    SymlinkRefused,
    SymlinkRaced,
    OutsideWorkspace,
    FileTooLarge,
    UnsupportedFormat,
}

/// Canonical DB/link-backed subject that extraction may attribute claims to.
/// Request DTOs are hints only; the pipeline populates this after verifying the
/// entity row and an active `document_entity_links` row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedLinkedSubject {
    pub entity_type: EntityType,
    pub entity_id: String,
    pub entity_name: Option<String>,
    pub link_id: String,
}

impl ResolvedLinkedSubject {
    pub fn subject_kind_slug(&self) -> &'static str {
        match self.entity_type {
            EntityType::Account => "account",
            EntityType::Project => "project",
            EntityType::Person => "person",
            EntityType::Other => "other",
        }
    }

    pub fn is_claim_supported(&self) -> bool {
        matches!(
            self.entity_type,
            EntityType::Account | EntityType::Project | EntityType::Person
        )
    }
}

/// System-owned extraction context. The extractor reads content from the
/// validated file handle, but all authority-bearing metadata comes from this
/// object.
pub struct ExtractionContext<'a> {
    pub file_id: &'a str,
    pub identity: &'a FileIdentity,
    pub content: &'a str,
    pub source_type: WorkspaceFileKind,
    pub source_asof: DateTime<Utc>,
    pub resolved_category: Option<&'a WorkspaceCategory>,
    pub linked_subject: Option<&'a ResolvedLinkedSubject>,
    pub ingestion_run_id: &'a str,
    pub observed_at: DateTime<Utc>,
    pub invocation_actor: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DroppedFactSource {
    Frontmatter,
    Body,
    Structured,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DroppedFact {
    pub reason: String,
    pub source: DroppedFactSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claim_type_candidate: Option<String>,
}

impl DroppedFact {
    pub fn new(reason: impl Into<String>, source: DroppedFactSource) -> Self {
        Self {
            reason: reason.into(),
            source,
            field: None,
            claim_type_candidate: None,
        }
    }

    pub fn with_field(mut self, field: impl Into<String>) -> Self {
        self.field = Some(field.into());
        self
    }

    pub fn with_claim_type_candidate(mut self, claim_type: impl Into<String>) -> Self {
        self.claim_type_candidate = Some(claim_type.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtractionWarning {
    pub code: String,
    pub message: String,
}

impl ExtractionWarning {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ExtractionReport {
    pub proposals: Vec<WorkspaceClaimProposal>,
    pub dropped_facts: Vec<DroppedFact>,
    pub warnings: Vec<ExtractionWarning>,
}

#[derive(Debug)]
pub enum ExtractionError {
    Io(io::Error),
    Provenance(String),
    UnsupportedContent(String),
}

impl std::fmt::Display for ExtractionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(
                f,
                "workspace extraction I/O error: kind={:?} os={:?}",
                error.kind(),
                error.raw_os_error()
            ),
            Self::Provenance(message) => {
                write!(f, "workspace extraction provenance error: {message}")
            }
            Self::UnsupportedContent(message) => {
                write!(f, "workspace extraction unsupported content: {message}")
            }
        }
    }
}

impl std::error::Error for ExtractionError {}

impl From<io::Error> for ExtractionError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

/// Extraction shape returned by W3-A's `Extractor::extract`. The subject is the
/// pipeline-verified link subject, never a frontmatter/body/path-derived value.
/// Conversion to `services::claims::ClaimProposal` happens in the pipeline and
/// commits through `services::claims::commit_claim`.
#[derive(Debug, Clone)]
pub struct WorkspaceClaimProposal {
    pub claim_type: ClaimType,
    pub subject: ResolvedLinkedSubject,
    pub text: String,
    pub field_path: Option<String>,
    pub topic_key: Option<String>,
    pub source_asof: DateTime<Utc>,
    pub observed_at: DateTime<Utc>,
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
    pub source_ref: String,
}

/// Extraction trait. W3-A's `WorkspaceExtractor` implements this; W2-A
/// constructs the pipeline with `NullExtractor` by default and `wiring.rs`
/// swaps in `WorkspaceExtractor` after W3-A merges.
pub trait Extractor: Send + Sync {
    fn extract(
        &self,
        file: &mut File,
        context: &ExtractionContext<'_>,
    ) -> Result<ExtractionReport, ExtractionError>;
}

/// Context required for workspace signal emission. The caller supplies the live
/// service/DB/propagation handles so emitters use the service signal facade
/// instead of opening their own handles or reaching into `signals::bus`.
pub struct SignalEmitContext<'a, 'svc> {
    pub services: &'a ServiceContext<'svc>,
    pub db: &'a ActionDb,
    pub propagation: Option<&'a PropagationEngine>,
}

impl<'a, 'svc> SignalEmitContext<'a, 'svc> {
    pub fn new(
        services: &'a ServiceContext<'svc>,
        db: &'a ActionDb,
        propagation: Option<&'a PropagationEngine>,
    ) -> Self {
        Self {
            services,
            db,
            propagation,
        }
    }
}

#[derive(Debug)]
pub enum SignalEmitError {
    MissingEntityTarget(&'static str),
    MissingPropagationEngine(&'static str),
    Serialize {
        signal_type: &'static str,
        message: String,
    },
    Emit {
        signal_type: &'static str,
        message: String,
    },
}

impl std::fmt::Display for SignalEmitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingEntityTarget(signal_type) => {
                write!(f, "workspace signal {signal_type} missing entity target")
            }
            Self::MissingPropagationEngine(signal_type) => {
                write!(
                    f,
                    "workspace signal {signal_type} missing propagation engine"
                )
            }
            Self::Serialize {
                signal_type,
                message,
            } => write!(
                f,
                "workspace signal {signal_type} payload serialization failed: {message}"
            ),
            Self::Emit {
                signal_type,
                message,
            } => write!(
                f,
                "workspace signal {signal_type} emission failed: {message}"
            ),
        }
    }
}

impl std::error::Error for SignalEmitError {}

/// Signal-emission trait. Each method maps 1:1 to a
/// `SignalType::WorkspaceFile*` variant in `signals/policy_registry.rs`.
/// Implementations are fallible so required invalidating signals cannot be
/// silently dropped.
pub trait SignalEmitter: Send + Sync {
    fn emit_file_ingested(
        &self,
        ctx: &SignalEmitContext<'_, '_>,
        file_id: &str,
        ingestion_run_id: &str,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<(), SignalEmitError>;
    fn emit_file_rejected(
        &self,
        ctx: &SignalEmitContext<'_, '_>,
        file_id: Option<&str>,
        reason: RejectionReason,
    ) -> Result<(), SignalEmitError>;
    fn emit_file_pending_entity_assignment(
        &self,
        ctx: &SignalEmitContext<'_, '_>,
        file_id: &str,
        ingestion_run_id: &str,
    ) -> Result<(), SignalEmitError>;
    fn emit_file_quarantined(
        &self,
        ctx: &SignalEmitContext<'_, '_>,
        file_id: &str,
        reason: &str,
        actor: &str,
        entity_type: Option<&str>,
        entity_id: Option<&str>,
    ) -> Result<(), SignalEmitError>;
    fn emit_link_changed(
        &self,
        ctx: &SignalEmitContext<'_, '_>,
        file_id: &str,
        entity_type: &str,
        entity_id: &str,
        actor: &str,
    ) -> Result<(), SignalEmitError>;
}

/// No-op extractor. Lets W1 + W2 compile against the `Extractor` trait before
/// W3-A's real impl lands.
pub struct NullExtractor;

impl Extractor for NullExtractor {
    fn extract(
        &self,
        _file: &mut File,
        _context: &ExtractionContext<'_>,
    ) -> Result<ExtractionReport, ExtractionError> {
        Ok(ExtractionReport::default())
    }
}

/// No-op signal emitter. Lets W1 (including `link::override_link`) compile
/// against the `SignalEmitter` trait before W3-B's real impl lands.
pub struct NullSignalEmitter;

impl SignalEmitter for NullSignalEmitter {
    fn emit_file_ingested(
        &self,
        _ctx: &SignalEmitContext<'_, '_>,
        _file_id: &str,
        _ingestion_run_id: &str,
        _entity_type: &str,
        _entity_id: &str,
    ) -> Result<(), SignalEmitError> {
        Ok(())
    }

    fn emit_file_rejected(
        &self,
        _ctx: &SignalEmitContext<'_, '_>,
        _file_id: Option<&str>,
        _reason: RejectionReason,
    ) -> Result<(), SignalEmitError> {
        Ok(())
    }

    fn emit_file_pending_entity_assignment(
        &self,
        _ctx: &SignalEmitContext<'_, '_>,
        _file_id: &str,
        _ingestion_run_id: &str,
    ) -> Result<(), SignalEmitError> {
        Ok(())
    }

    fn emit_file_quarantined(
        &self,
        _ctx: &SignalEmitContext<'_, '_>,
        _file_id: &str,
        _reason: &str,
        _actor: &str,
        _entity_type: Option<&str>,
        _entity_id: Option<&str>,
    ) -> Result<(), SignalEmitError> {
        Ok(())
    }

    fn emit_link_changed(
        &self,
        _ctx: &SignalEmitContext<'_, '_>,
        _file_id: &str,
        _entity_type: &str,
        _entity_id: &str,
        _actor: &str,
    ) -> Result<(), SignalEmitError> {
        Ok(())
    }
}
