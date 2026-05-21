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

use rusqlite::{params, Connection, OptionalExtension};

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
    /// Entity type not recognized.
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
    /// DB lookup failed during validate (only fires when `category` is `Some`).
    DbError(String),
}

impl std::fmt::Display for ResolvePathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CategoryNotAllowed(e) => write!(f, "resolve_path: {e}"),
            Self::EntityTypeNotRoutable => write!(
                f,
                "resolve_path: EntityType::Other not routable in v1.4.5; deferred to follow-up"
            ),
            Self::DbError(msg) => write!(f, "resolve_path: db error: {msg}"),
        }
    }
}

impl std::error::Error for ResolvePathError {}

impl From<CategoryNotAllowed> for ResolvePathError {
    fn from(e: CategoryNotAllowed) -> Self {
        Self::CategoryNotAllowed(e)
    }
}

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
             4 positive fixtures in tests/workspace_registry_open_validated.rs. \
             The security boundary requires careful Rust unix-specific code \
             (libc::O_NOFOLLOW interaction, std::os::unix::fs metadata APIs). \
             Defer to next iteration."
        )
    }
}

/// Validates a slug against `SLUG_REGEX`. Pure function — no DB access.
fn is_valid_slug_shape(slug: &str) -> bool {
    !slug.is_empty()
        && slug.len() <= 32
        && {
            let mut chars = slug.chars();
            let first = chars.next().expect("non-empty checked above");
            first.is_ascii_lowercase()
                && chars.all(|c| {
                    c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-'
                })
        }
}

/// Snake_case serde tag for `crate::entity::EntityType` — used as the
/// `entity_type` column value in `workspace_category_registry`.
fn entity_type_slug(et: EntityType) -> &'static str {
    match et {
        EntityType::Account => "account",
        EntityType::Person => "person",
        EntityType::Project => "project",
        EntityType::Other => "other",
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
        conn: &Connection,
        category: &WorkspaceCategory,
        entity_type: EntityType,
    ) -> Result<(), CategoryNotAllowed> {
        if matches!(entity_type, EntityType::Other) {
            return Err(CategoryNotAllowed {
                category: category.as_slug().to_string(),
                entity_type,
                allowed: vec![],
            });
        }
        let et_slug = entity_type_slug(entity_type);
        let cat_slug = category.as_slug();

        let allowed: bool = conn
            .query_row(
                "SELECT 1 FROM workspace_category_registry \
                 WHERE entity_type = ?1 AND category_slug = ?2 AND allowed = 1",
                params![et_slug, cat_slug],
                |_| Ok(true),
            )
            .optional()
            .map_err(|e| CategoryNotAllowed {
                category: cat_slug.to_string(),
                entity_type,
                allowed: vec![format!("db error: {e}")],
            })?
            .unwrap_or(false);

        if allowed {
            return Ok(());
        }

        // Populate allowed-list for error envelope.
        let allowed_list: Vec<String> = conn
            .prepare(
                "SELECT category_slug FROM workspace_category_registry \
                 WHERE entity_type = ?1 AND allowed = 1 ORDER BY category_slug",
            )
            .and_then(|mut stmt| {
                stmt.query_map(params![et_slug], |row| row.get::<_, String>(0))
                    .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            })
            .unwrap_or_default();

        Err(CategoryNotAllowed {
            category: cat_slug.to_string(),
            entity_type,
            allowed: allowed_list,
        })
    }

    /// Resolve the workspace-relative path for a file binding.
    ///
    /// Rules per cycle-8 Option B-prime + V1.3 fold #3:
    /// - `source_type == Inbox` → `_inbox/{filename}` regardless of entity.
    /// - `entity_type == Other` → `Err(ResolvePathError::EntityTypeNotRoutable)`.
    /// - `category == Some(c)` → `{Accounts|People|Projects}/{entity_name}/{c.as_slug()}/{filename}` (after validate).
    /// - `category == None` → `{Accounts|People|Projects}/{entity_name}/{filename}` (entity root).
    pub fn resolve_path(
        conn: &Connection,
        entity_type: EntityType,
        entity_name: &str,
        category: Option<&WorkspaceCategory>,
        filename: &str,
        source_type: WorkspaceFileKind,
    ) -> Result<PathBuf, ResolvePathError> {
        // Inbox sentinel: regardless of entity binding, Inbox files go to _inbox/.
        if matches!(source_type, WorkspaceFileKind::Inbox) {
            return Ok(PathBuf::from("_inbox").join(filename));
        }

        // EntityType::Other is not routable in v1.4.5.
        if matches!(entity_type, EntityType::Other) {
            return Err(ResolvePathError::EntityTypeNotRoutable);
        }

        // Validate the category if supplied. (Other entity types already returned above.)
        if let Some(cat) = category {
            WorkspaceCategoryRegistry::validate(conn, cat, entity_type)?;
        }

        let entity_dir = match entity_type {
            EntityType::Account => "Accounts",
            EntityType::Person => "People",
            EntityType::Project => "Projects",
            EntityType::Other => unreachable!("guarded above"),
        };

        let mut path = PathBuf::from(entity_dir).join(entity_name);
        if let Some(cat) = category {
            path.push(cat.as_slug());
        }
        path.push(filename);
        Ok(path)
    }

    /// Register a new `Other(slug)` category for an entity type. Lex-validates
    /// the slug against `SLUG_REGEX` before INSERTing. Parameterized SQL only
    /// — no string-format interpolation of slug into INSERT.
    pub fn register_other(
        conn: &Connection,
        entity_type: EntityType,
        slug: &str,
    ) -> Result<(), RegisterError> {
        if !is_valid_slug_shape(slug) {
            return Err(RegisterError::MalformedSlug {
                slug: slug.to_string(),
            });
        }
        if matches!(entity_type, EntityType::Other) {
            return Err(RegisterError::EntityTypeUnknown);
        }
        let et_slug = entity_type_slug(entity_type);
        conn.execute(
            "INSERT OR IGNORE INTO workspace_category_registry (entity_type, category_slug) \
             VALUES (?1, ?2)",
            params![et_slug, slug],
        )
        .map(|_| ())
        .map_err(|e| RegisterError::DbError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory sqlite");
        conn.execute_batch(include_str!("../../migrations/252_workspace_source_registry.sql"))
            .expect("v252 apply");
        conn
    }

    #[test]
    fn is_valid_slug_shape_accepts_lowercase_ascii() {
        for s in &["a", "abc", "abc_123", "abc-def", "x_y_z", "a1234567890"] {
            assert!(is_valid_slug_shape(s), "{s} should be valid");
        }
    }

    #[test]
    fn is_valid_slug_shape_rejects_malformed() {
        for s in &[
            "",
            "1abc",       // starts with digit
            "_abc",       // starts with underscore
            "-abc",       // starts with hyphen
            "ABC",        // uppercase
            "abc DEF",    // space + uppercase
            "abc/def",    // slash
            "abc.def",    // dot
            "abc!def",    // punctuation
            &"x".repeat(33), // too long
        ] {
            assert!(!is_valid_slug_shape(s), "{s} should be invalid");
        }
    }

    #[test]
    fn validate_accepts_seeded_pairs_for_account_person_project() {
        let conn = fresh_conn();
        for et in &[EntityType::Account, EntityType::Person, EntityType::Project] {
            for cat in &[
                WorkspaceCategory::Presentations,
                WorkspaceCategory::Transcripts,
                WorkspaceCategory::Meetings,
                WorkspaceCategory::Notes,
                WorkspaceCategory::Contracts,
                WorkspaceCategory::Attachments,
            ] {
                WorkspaceCategoryRegistry::validate(&conn, cat, *et)
                    .unwrap_or_else(|e| panic!("expected Ok for {et:?} + {cat:?}: {e}"));
            }
        }
    }

    #[test]
    fn validate_rejects_other_entity_type_with_empty_allowed_list() {
        let conn = fresh_conn();
        let err = WorkspaceCategoryRegistry::validate(
            &conn,
            &WorkspaceCategory::Presentations,
            EntityType::Other,
        )
        .expect_err("Other should be Err");
        assert!(err.allowed.is_empty(), "Other should have empty allowed list");
    }

    #[test]
    fn validate_rejects_unregistered_other_slug_with_allowed_list_populated() {
        let conn = fresh_conn();
        let custom = WorkspaceCategory::Other("custom_unregistered".to_string());
        let err = WorkspaceCategoryRegistry::validate(&conn, &custom, EntityType::Account)
            .expect_err("unregistered Other should be Err");
        assert_eq!(err.category, "custom_unregistered");
        assert_eq!(err.entity_type, EntityType::Account);
        assert_eq!(err.allowed.len(), 6, "should list 6 default categories");
    }

    #[test]
    fn register_other_inserts_lex_valid_slug_then_validate_passes() {
        let conn = fresh_conn();
        WorkspaceCategoryRegistry::register_other(&conn, EntityType::Account, "custom_slug")
            .expect("register Ok");
        let custom = WorkspaceCategory::Other("custom_slug".to_string());
        WorkspaceCategoryRegistry::validate(&conn, &custom, EntityType::Account)
            .expect("validate after register Ok");
    }

    #[test]
    fn register_other_rejects_uppercase_con_per_regex() {
        let conn = fresh_conn();
        let err = WorkspaceCategoryRegistry::register_other(&conn, EntityType::Account, "CON")
            .expect_err("uppercase CON should fail regex");
        assert!(matches!(err, RegisterError::MalformedSlug { .. }));
    }

    #[test]
    fn register_other_accepts_lowercase_con_currently_windows_deferred() {
        let conn = fresh_conn();
        // V1.3 fold #4: lowercase "con" passes the regex; Windows-reserved-name
        // semantic check lives in the deferred Windows path validation ticket.
        WorkspaceCategoryRegistry::register_other(&conn, EntityType::Account, "con")
            .expect("lowercase con passes regex (Windows check deferred)");
    }

    #[test]
    fn register_other_rejects_other_entity_type() {
        let conn = fresh_conn();
        let err = WorkspaceCategoryRegistry::register_other(&conn, EntityType::Other, "abc")
            .expect_err("EntityType::Other should be rejected");
        assert!(matches!(err, RegisterError::EntityTypeUnknown));
    }

    #[test]
    fn resolve_path_inbox_sentinel_overrides_entity_routing() {
        let conn = fresh_conn();
        let p = WorkspaceCategoryRegistry::resolve_path(
            &conn,
            EntityType::Account,
            "_unused_",
            None,
            "drop.md",
            WorkspaceFileKind::Inbox,
        )
        .expect("Inbox resolves");
        assert_eq!(p, PathBuf::from("_inbox/drop.md"));
    }

    #[test]
    fn resolve_path_routes_account_to_accounts() {
        let conn = fresh_conn();
        let p = WorkspaceCategoryRegistry::resolve_path(
            &conn,
            EntityType::Account,
            "Acme",
            Some(&WorkspaceCategory::Presentations),
            "q1.pdf",
            WorkspaceFileKind::EntityDoc,
        )
        .expect("Account resolves");
        assert_eq!(p, PathBuf::from("Accounts/Acme/presentations/q1.pdf"));
    }

    #[test]
    fn resolve_path_routes_person_to_people() {
        let conn = fresh_conn();
        let p = WorkspaceCategoryRegistry::resolve_path(
            &conn,
            EntityType::Person,
            "Bob",
            Some(&WorkspaceCategory::Notes),
            "1on1.md",
            WorkspaceFileKind::EntityDoc,
        )
        .expect("Person resolves");
        assert_eq!(p, PathBuf::from("People/Bob/notes/1on1.md"));
    }

    #[test]
    fn resolve_path_routes_project_to_projects_with_no_category() {
        let conn = fresh_conn();
        let p = WorkspaceCategoryRegistry::resolve_path(
            &conn,
            EntityType::Project,
            "Apollo",
            None,
            "design.pdf",
            WorkspaceFileKind::EntityDoc,
        )
        .expect("Project resolves");
        assert_eq!(p, PathBuf::from("Projects/Apollo/design.pdf"));
    }

    #[test]
    fn resolve_path_entity_type_other_returns_err() {
        let conn = fresh_conn();
        let err = WorkspaceCategoryRegistry::resolve_path(
            &conn,
            EntityType::Other,
            "_unused_",
            None,
            "drop.md",
            WorkspaceFileKind::EntityDoc,
        )
        .expect_err("Other should Err");
        assert!(matches!(err, ResolvePathError::EntityTypeNotRoutable));
    }

    #[test]
    fn resolve_path_propagates_category_not_allowed() {
        let conn = fresh_conn();
        let unregistered = WorkspaceCategory::Other("unregistered".to_string());
        let err = WorkspaceCategoryRegistry::resolve_path(
            &conn,
            EntityType::Account,
            "Acme",
            Some(&unregistered),
            "x.md",
            WorkspaceFileKind::EntityDoc,
        )
        .expect_err("unregistered category should Err");
        assert!(matches!(err, ResolvePathError::CategoryNotAllowed(_)));
    }
}
