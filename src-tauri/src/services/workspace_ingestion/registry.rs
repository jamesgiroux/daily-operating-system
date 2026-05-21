//! Workspace source registry — DOS-464 (W1-B) substrate.
//!
//! Two responsibilities:
//!
//! 1. **Path validation trust boundary.** `WorkspaceSourceRegistry::open_validated`
//!    is the single entry point for opening workspace files. Every ingestion
//!    code path goes through it. Closes path-traversal, TOCTOU race,
//!    hardlink, bind-mount, and cross-device attack vectors per L0 V1.3 §7
//!    security gate.
//!
//! 2. **Per-entity category registry.** `WorkspaceCategoryRegistry` maps
//!    `crate::entity::EntityType` → allowed `WorkspaceCategory` values per
//!    the cycle-8 Option B-prime amendment + cycle-12 substrate-consumption
//!    sweep. `register_other` extends the registry at runtime with
//!    lex-validated user-defined slugs.
//!
//! Trust boundary architecture (canonicalize-first; O_NOFOLLOW removed per
//! V1.2 fold #2 as vacuous post-canonicalize; nlink>1 hardlink defense per
//! V1.2 fold #1):
//!
//! ```text
//! 1. lex-validate per component (.., NFKC normalization, reject NUL/...)
//! 2. canonicalize → strict-child-of-workspace_root or OutsideWorkspace
//! 3. lstat canonical_path → record (dev, ino)
//!    assert lstat.dev == workspace_root_dev (cross-mount)
//! 4. open canonical_path (plain open; no O_NOFOLLOW since canonicalize ran)
//! 5. fstat the open File:
//!    (a) (fstat.dev, fstat.ino) == (lstat.dev, lstat.ino) [TOCTOU close]
//!    (b) fstat.dev == workspace_root_dev [defense in depth]
//!    (c) fstat.nlink == 1 [hardlink defense — refuse all multi-link]
//! 6. return (File, FileIdentity { canonical_path, dev: fstat.dev, ino: fstat.ino })
//! ```
//!
//! Slug regex: `^[a-z][a-z0-9_-]{0,31}$` (lowercase ASCII, max 32 chars,
//! starts with letter; rejects whitespace, uppercase, special chars).
//!
//! Windows path validation deferred to follow-up ticket (V1.1 fold #19); Unix
//! is the v1.4.5 path. On Windows, `open_validated` currently returns
//! `RejectionReason::OutsideWorkspace` with a platform-not-supported log.

use std::fs::File;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};

use crate::entity::EntityType;
use super::contracts::{FileIdentity, RejectionReason, WorkspaceCategory, WorkspaceFileKind};

/// Frozen regex for `Other(slug)` slug shape validation. ASCII only, lowercase
/// only, max 32 chars, must start with a letter, allows digits/underscore/hyphen.
/// Per L0 V1.3 §6.
pub const SLUG_REGEX: &str = r"^[a-z][a-z0-9_-]{0,31}$";

/// Validation error: caller-provided category is not registered for the entity type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryNotAllowed {
    pub category: String,
    pub entity_type: EntityType,
    pub allowed: Vec<String>,
}

impl std::fmt::Display for CategoryNotAllowed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "category `{}` not allowed for entity_type {:?}; allowed: [{}]",
            self.category,
            self.entity_type,
            self.allowed.join(", ")
        )
    }
}

impl std::error::Error for CategoryNotAllowed {}

/// Errors returned by `register_other` and other category-registry mutations.
#[derive(Debug)]
pub enum RegisterError {
    /// Slug fails the lex-shape regex.
    MalformedSlug { slug: String },
    /// Entity type not recognized (currently unreachable with typed `EntityType`; reserved for future runtime types).
    EntityTypeUnknown,
    DbError(String),
}

impl std::fmt::Display for RegisterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MalformedSlug { slug } => {
                write!(f, "malformed slug `{slug}` — fails regex {SLUG_REGEX}")
            }
            Self::EntityTypeUnknown => write!(f, "entity_type unknown"),
            Self::DbError(msg) => write!(f, "register db error: {msg}"),
        }
    }
}

impl std::error::Error for RegisterError {}

/// Errors returned by `WorkspaceCategoryRegistry::resolve_path`.
#[derive(Debug)]
pub enum ResolvePathError {
    /// Category not allowed for the entity_type (per `validate`).
    CategoryNotAllowed(CategoryNotAllowed),
    /// `EntityType::Other` is not routable to a workspace path in v1.4.5 (deferred to follow-up).
    EntityTypeNotRoutable,
}

impl std::fmt::Display for ResolvePathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CategoryNotAllowed(e) => write!(f, "resolve_path: {e}"),
            Self::EntityTypeNotRoutable => write!(
                f,
                "resolve_path: EntityType::Other not routable in v1.4.5; deferred to follow-up"
            ),
        }
    }
}

impl std::error::Error for ResolvePathError {}

/// Workspace source-type allowlist registry. Read-only at runtime for the
/// canonical 7 `WorkspaceFileKind` variants (seeded at v252 migration time).
pub struct WorkspaceSourceRegistry;

impl WorkspaceSourceRegistry {
    /// Open a workspace-relative path under the path validation trust boundary.
    ///
    /// Returns `(File, FileIdentity)` on success. Returns typed
    /// `RejectionReason` for every attack vector enumerated in L0 V1.3 §7
    /// fixture suite — never reads bytes before all checks pass.
    ///
    /// Unix-only at v1.4.5. Windows path returns `RejectionReason::OutsideWorkspace`
    /// with a platform-not-supported log until follow-up ticket lands.
    pub fn open_validated(_path: &Path) -> Result<(File, FileIdentity), RejectionReason> {
        unimplemented!(
            "W1-B implementing agent: implement the 6-step canonicalize-first \
             algorithm per registry.rs module doc-comment. Unix-only. 15 negative + \
             4 positive fixtures in tests/workspace_registry_open_validated.rs."
        )
    }
}

/// Per-entity-type category registry. Defines which `WorkspaceCategory` slugs
/// are allowed under which `EntityType` (canonical 6 categories × 3 entity
/// types seeded at v252; runtime extension via `register_other`).
pub struct WorkspaceCategoryRegistry;

impl WorkspaceCategoryRegistry {
    /// Returns `Ok(())` if the category is allowed for the entity_type;
    /// `Err(CategoryNotAllowed)` with the full allowed-list otherwise.
    ///
    /// `EntityType::Other` always returns `Err(CategoryNotAllowed { allowed: vec![] })`
    /// — Other-typed entities cannot bind workspace files in v1.4.5 (V1.3 fold #5).
    pub fn validate(
        _category: &WorkspaceCategory,
        _entity_type: EntityType,
    ) -> Result<(), CategoryNotAllowed> {
        unimplemented!(
            "W1-B implementing agent: SELECT … FROM workspace_category_registry WHERE \
             entity_type = ?1 AND category_slug = ?2; map missing row to \
             CategoryNotAllowed with full allowed-list."
        )
    }

    /// Resolve the workspace-relative path for a file binding.
    ///
    /// Rules per cycle-8 Option B-prime + V1.3 fold #3:
    /// - `source_type == Inbox` → `_inbox/{filename}` regardless of entity.
    /// - `entity_type == Other` → `Err(ResolvePathError::EntityTypeNotRoutable)`.
    /// - `category == Some(c)` → `{Accounts|People|Projects}/{entity_name}/{c.as_slug()}/{filename}` (after validate).
    /// - `category == None` → `{Accounts|People|Projects}/{entity_name}/{filename}` (entity root).
    pub fn resolve_path(
        _entity_type: EntityType,
        _entity_name: &str,
        _category: Option<&WorkspaceCategory>,
        _filename: &str,
        _source_type: WorkspaceFileKind,
    ) -> Result<PathBuf, ResolvePathError> {
        unimplemented!(
            "W1-B implementing agent: per L0 V1.3 §4 — Inbox sentinel first, \
             EntityType::Other → Err(EntityTypeNotRoutable), validate-then-route \
             for Account/Person/Project."
        )
    }

    /// Register a new `Other(slug)` category for an entity type. Lex-validates
    /// the slug against `SLUG_REGEX` before INSERTing. Parameterized SQL only
    /// — no string-format interpolation of slug into INSERT.
    pub fn register_other(
        _entity_type: EntityType,
        _slug: &str,
    ) -> Result<(), RegisterError> {
        unimplemented!(
            "W1-B implementing agent: regex-check slug; if Ok, INSERT INTO \
             workspace_category_registry (entity_type, category_slug) VALUES (?, ?)."
        )
    }
}

/// Audit record helper for security-fixture tests: timestamp tracking when
/// `open_validated` rejected a path. Not stored on disk; transient for tests.
#[derive(Debug)]
pub struct RejectionAudit {
    pub at: DateTime<Utc>,
    pub reason: RejectionReason,
}
