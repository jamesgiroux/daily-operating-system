//! Document/entity link service — DOS-465 (W1-C) substrate.
//!
//! Wraps the `document_entity_links` table (migration v254). Per L0 question 10
//! cycle-2 resolution: relational table, NOT a claim (an entity-doc link is
//! metadata about ingestion provenance, not an assertable claim about the
//! world).
//!
//! Tombstone semantics — TWO-LAYER defense (V1.3 fold #1):
//!
//! 1. **SQL layer:** partial UNIQUE on `(file_id, entity_type, entity_id)
//!    WHERE rejected = 0` — prevents duplicate ACTIVE rows for the same triple.
//!
//! 2. **Service layer:** `add_link` for `attribution_source ∈ {Classifier,
//!    Backfill, DriveMetadata}` wraps the SELECT-rejected-row + INSERT in a
//!    `BEGIN IMMEDIATE` transaction. Classifier sources cannot silently
//!    resurrect a rejected link. Atomic guard against the V1.2 race where a
//!    concurrent `reject_link` between SELECT and INSERT could let an
//!    active row be created after tombstone.
//!
//! User-driven attribution sources (`EntityIntake`, `UserRelink`, `McpPlacement`,
//! `Frontmatter`) BYPASS the rejected-row check — the user is intentionally
//! resurrecting per the DOS-465 issue AC carve-out. The SQL partial UNIQUE
//! still prevents an active duplicate.
//!
//! `override_link` is the **endorse-existing** API: it UPDATEs an existing
//! active link (sets `user_override_*`), returns `LinkError::NotFound` if no
//! active row exists. It does NOT resurrect rejected links — that goes through
//! `add_link(... UserRelink)` (V1.3 fold #2).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::entity::EntityType;
use super::contracts::SignalEmitter;
use super::lifecycle::UserOverride;

/// Opaque UUID4 identifier for a document/entity link row.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DocumentEntityLinkId(pub String);

/// Attribution-source taxonomy per L0 V1.3 §4. Snake_case serde tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkAttributionSource {
    /// User explicitly bound the file to the entity via the entity-intake block.
    EntityIntake,
    /// File's frontmatter (e.g., markdown `entity_id:` header) named the binding.
    Frontmatter,
    /// Automatic classifier inferred the binding from content.
    Classifier,
    /// User re-bound a file to an entity (post-classifier-error or post-reject).
    UserRelink,
    /// Drive document metadata (folder, label, etc.) named the binding.
    DriveMetadata,
    /// MCP placement contract delivered the file pre-bound (DOS-474).
    McpPlacement,
    /// W5-A backfill bound pre-existing files based on conservative heuristics.
    Backfill,
}

impl LinkAttributionSource {
    /// Classifier-class sources subject to tombstone-guard (must NOT resurrect rejected links).
    pub fn is_classifier_class(&self) -> bool {
        matches!(self, Self::Classifier | Self::Backfill | Self::DriveMetadata)
    }
}

/// Row shape of `document_entity_links` per L0 V1.3 §4 + §6.
#[derive(Debug, Clone)]
pub struct DocumentEntityLink {
    pub link_id: DocumentEntityLinkId,
    pub file_id: String,
    pub entity_type: EntityType,
    pub entity_id: String,
    pub attribution_source: LinkAttributionSource,
    pub confidence: f64,
    pub rationale: Option<String>,
    pub actor: String,
    pub user_override: Option<UserOverride>,
    pub rejected: bool,
    pub rejected_at: Option<DateTime<Utc>>,
    pub rejected_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Errors returned by `LinkRepo`. Per L0 V1.3 §4.
#[derive(Debug)]
pub enum LinkError {
    NotFound,
    DuplicateActive,
    AlreadyRejected,
    Tombstoned {
        rejected_at: DateTime<Utc>,
        rejected_reason: String,
    },
    EntityTypeUnknown,
    DbError(String),
}

impl std::fmt::Display for LinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "document/entity link not found"),
            Self::DuplicateActive => write!(f, "duplicate active link"),
            Self::AlreadyRejected => write!(f, "link already rejected"),
            Self::Tombstoned {
                rejected_at,
                rejected_reason,
            } => write!(
                f,
                "link tombstoned at {rejected_at}: {rejected_reason}"
            ),
            Self::EntityTypeUnknown => write!(f, "entity_type unknown"),
            Self::DbError(msg) => write!(f, "link db error: {msg}"),
        }
    }
}

impl std::error::Error for LinkError {}

/// Repository for `document_entity_links` (v254 table).
pub struct LinkRepo;

impl LinkRepo {
    /// Creates or resurrects a document/entity link.
    ///
    /// For `attribution_source.is_classifier_class()` sources:
    /// 1. `BEGIN IMMEDIATE` transaction.
    /// 2. SELECT for `rejected = 1` row → if exists, ROLLBACK + return `Tombstoned`.
    /// 3. INSERT … ON CONFLICT (file_id, entity_type, entity_id) WHERE rejected = 0 DO NOTHING RETURNING link_id.
    /// 4. If RETURNING returns 0 rows (active duplicate already exists), SELECT existing link_id.
    /// 5. COMMIT.
    ///
    /// For user-driven sources (EntityIntake/UserRelink/McpPlacement/Frontmatter):
    /// skips the rejected-row check (intentional user-driven resurrection per
    /// DOS-465 AC carve-out), but still wraps INSERT in a transaction for atomicity.
    pub fn add_link(
        _file_id: &str,
        _entity_type: EntityType,
        _entity_id: &str,
        _attribution_source: LinkAttributionSource,
        _confidence: f64,
        _rationale: Option<&str>,
        _actor: &str,
    ) -> Result<DocumentEntityLinkId, LinkError> {
        unimplemented!(
            "W1-C implementing agent: implement the BEGIN IMMEDIATE flow per \
             link.rs module doc-comment + L0 V1.3 §4 fold #1."
        )
    }

    /// Lists active (and optionally rejected) links for a file.
    pub fn list_links_for_file(
        _file_id: &str,
        _include_rejected: bool,
    ) -> Result<Vec<DocumentEntityLink>, LinkError> {
        unimplemented!(
            "W1-C implementing agent: SELECT * FROM document_entity_links WHERE \
             file_id = ? AND (include_rejected OR rejected = 0)."
        )
    }

    /// Endorse-existing-active API. Marks an active link as user-confirmed
    /// (sets `user_override_actor` + `user_override_at`); emits the
    /// link-changed signal via the supplied `SignalEmitter`. Returns
    /// `LinkError::NotFound` if no active row exists for the triple.
    ///
    /// Does NOT resurrect rejected links — that path goes through
    /// `add_link(... UserRelink)` (V1.3 fold #2).
    pub fn override_link(
        _emitter: &dyn SignalEmitter,
        _file_id: &str,
        _entity_type: EntityType,
        _entity_id: &str,
        _actor: &str,
    ) -> Result<(), LinkError> {
        unimplemented!(
            "W1-C implementing agent: UPDATE … WHERE rejected = 0 AND triple; \
             if zero rows affected, return NotFound. After UPDATE, call \
             emitter.emit_link_changed(file_id, entity_id, actor)."
        )
    }

    /// Reject-existing API. Flips `rejected = 1` and populates
    /// `rejected_at` and `rejected_reason`. The partial UNIQUE on
    /// `WHERE rejected = 0` releases its hold; future `add_link` attempts
    /// from classifier sources hit the `Tombstoned` guard.
    pub fn reject_link(
        _file_id: &str,
        _entity_type: EntityType,
        _entity_id: &str,
        _actor: &str,
        _reason: &str,
    ) -> Result<(), LinkError> {
        unimplemented!(
            "W1-C implementing agent: UPDATE document_entity_links SET rejected = 1, \
             rejected_at = now, rejected_reason = ? WHERE file_id = ? AND entity_type = ? \
             AND entity_id = ?. If zero rows affected, return NotFound."
        )
    }
}
