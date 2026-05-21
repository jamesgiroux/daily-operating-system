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
use rusqlite::{params, Connection, OptionalExtension};
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
    EntityIntake,
    Frontmatter,
    Classifier,
    UserRelink,
    DriveMetadata,
    McpPlacement,
    Backfill,
}

impl LinkAttributionSource {
    pub fn is_classifier_class(&self) -> bool {
        matches!(self, Self::Classifier | Self::Backfill | Self::DriveMetadata)
    }

    pub fn as_storage_str(self) -> &'static str {
        match self {
            Self::EntityIntake => "entity_intake",
            Self::Frontmatter => "frontmatter",
            Self::Classifier => "classifier",
            Self::UserRelink => "user_relink",
            Self::DriveMetadata => "drive_metadata",
            Self::McpPlacement => "mcp_placement",
            Self::Backfill => "backfill",
        }
    }

    pub fn from_storage_str(s: &str) -> Option<Self> {
        match s {
            "entity_intake" => Some(Self::EntityIntake),
            "frontmatter" => Some(Self::Frontmatter),
            "classifier" => Some(Self::Classifier),
            "user_relink" => Some(Self::UserRelink),
            "drive_metadata" => Some(Self::DriveMetadata),
            "mcp_placement" => Some(Self::McpPlacement),
            "backfill" => Some(Self::Backfill),
            _ => None,
        }
    }
}

/// Snake_case serde tag for `EntityType` — same shape as workspace_category_registry.
fn entity_type_slug(et: EntityType) -> &'static str {
    match et {
        EntityType::Account => "account",
        EntityType::Person => "person",
        EntityType::Project => "project",
        EntityType::Other => "other",
    }
}

fn entity_type_from_slug(s: &str) -> Option<EntityType> {
    match s {
        "account" => Some(EntityType::Account),
        "person" => Some(EntityType::Person),
        "project" => Some(EntityType::Project),
        "other" => Some(EntityType::Other),
        _ => None,
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
            } => write!(f, "link tombstoned at {rejected_at}: {rejected_reason}"),
            Self::EntityTypeUnknown => write!(f, "entity_type unknown"),
            Self::DbError(msg) => write!(f, "link db error: {msg}"),
        }
    }
}

impl std::error::Error for LinkError {}

fn parse_dt(s: Option<String>) -> Option<DateTime<Utc>> {
    s.as_deref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc))
}

fn row_to_link(row: &rusqlite::Row<'_>) -> rusqlite::Result<DocumentEntityLink> {
    let link_id: String = row.get(0)?;
    let file_id: String = row.get(1)?;
    let entity_type_str: String = row.get(2)?;
    let entity_id: String = row.get(3)?;
    let attribution_str: String = row.get(4)?;
    let confidence: f64 = row.get(5)?;
    let rationale: Option<String> = row.get(6)?;
    let actor: String = row.get(7)?;
    let user_override_actor: Option<String> = row.get(8)?;
    let user_override_at: Option<String> = row.get(9)?;
    let rejected: i64 = row.get(10)?;
    let rejected_at: Option<String> = row.get(11)?;
    let rejected_reason: Option<String> = row.get(12)?;
    let created_at: String = row.get(13)?;
    let updated_at: String = row.get(14)?;

    let user_override = match (user_override_actor, parse_dt(user_override_at)) {
        (Some(actor), Some(at)) => Some(UserOverride {
            actor_id: actor,
            at,
        }),
        _ => None,
    };

    Ok(DocumentEntityLink {
        link_id: DocumentEntityLinkId(link_id),
        file_id,
        entity_type: entity_type_from_slug(&entity_type_str).unwrap_or(EntityType::Other),
        entity_id,
        attribution_source: LinkAttributionSource::from_storage_str(&attribution_str)
            .unwrap_or(LinkAttributionSource::Classifier),
        confidence,
        rationale,
        actor,
        user_override,
        rejected: rejected != 0,
        rejected_at: parse_dt(rejected_at),
        rejected_reason,
        created_at: parse_dt(Some(created_at)).unwrap_or_else(Utc::now),
        updated_at: parse_dt(Some(updated_at)).unwrap_or_else(Utc::now),
    })
}

/// Repository for `document_entity_links` (v254 table).
pub struct LinkRepo;

impl LinkRepo {
    /// Creates or resurrects a document/entity link. UNIMPLEMENTED — defer to
    /// next iteration. Implementation requires careful `BEGIN IMMEDIATE`
    /// transactional handling for classifier-class sources per V1.3 fold #1.
    #[allow(clippy::too_many_arguments)]
    pub fn add_link(
        conn: &Connection,
        file_id: &str,
        entity_type: EntityType,
        entity_id: &str,
        attribution_source: LinkAttributionSource,
        confidence: f64,
        rationale: Option<&str>,
        actor: &str,
    ) -> Result<DocumentEntityLinkId, LinkError> {
        let et_slug = entity_type_slug(entity_type);
        // Open BEGIN IMMEDIATE transaction to serialize the (optional)
        // tombstone-check + INSERT atomically. This closes the V1.2 race
        // where a concurrent reject_link between SELECT and INSERT could
        // let a classifier source create an active row after tombstone.
        conn.execute("BEGIN IMMEDIATE", [])
            .map_err(|e| LinkError::DbError(e.to_string()))?;

        // Tombstone guard for classifier-class sources only.
        if attribution_source.is_classifier_class() {
            let tombstoned: Option<(String, String)> = conn
                .query_row(
                    "SELECT rejected_at, rejected_reason FROM document_entity_links \
                     WHERE file_id = ?1 AND entity_type = ?2 AND entity_id = ?3 \
                       AND rejected = 1",
                    params![file_id, et_slug, entity_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()
                .map_err(|e| {
                    drop(conn.execute("ROLLBACK", []));
                    LinkError::DbError(e.to_string())
                })?;
            if let Some((rejected_at_raw, rejected_reason)) = tombstoned {
                drop(conn.execute("ROLLBACK", []));
                let rejected_at = parse_dt(Some(rejected_at_raw)).unwrap_or_else(Utc::now);
                return Err(LinkError::Tombstoned {
                    rejected_at,
                    rejected_reason,
                });
            }
        }

        let link_id = uuid::Uuid::new_v4().to_string();
        // INSERT with ON CONFLICT DO NOTHING (matching the partial UNIQUE
        // on rejected=0). If conflict (active duplicate exists), the
        // INSERT yields zero rows and we fall back to SELECT to return
        // the existing link_id.
        let inserted_id: Option<String> = conn
            .query_row(
                "INSERT INTO document_entity_links \
                 (link_id, file_id, entity_type, entity_id, attribution_source, \
                  confidence, rationale, actor) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) \
                 ON CONFLICT (file_id, entity_type, entity_id) WHERE rejected = 0 \
                 DO NOTHING RETURNING link_id",
                params![
                    link_id,
                    file_id,
                    et_slug,
                    entity_id,
                    attribution_source.as_storage_str(),
                    confidence,
                    rationale,
                    actor,
                ],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| {
                drop(conn.execute("ROLLBACK", []));
                LinkError::DbError(e.to_string())
            })?;

        let final_id = match inserted_id {
            Some(id) => id,
            None => {
                // Active duplicate present; fetch its link_id.
                conn.query_row(
                    "SELECT link_id FROM document_entity_links \
                     WHERE file_id = ?1 AND entity_type = ?2 AND entity_id = ?3 \
                       AND rejected = 0",
                    params![file_id, et_slug, entity_id],
                    |row| row.get(0),
                )
                .map_err(|e| {
                    drop(conn.execute("ROLLBACK", []));
                    LinkError::DbError(e.to_string())
                })?
            }
        };
        conn.execute("COMMIT", [])
            .map_err(|e| LinkError::DbError(e.to_string()))?;
        Ok(DocumentEntityLinkId(final_id))
    }

    /// Lists active (and optionally rejected) links for a file.
    pub fn list_links_for_file(
        conn: &Connection,
        file_id: &str,
        include_rejected: bool,
    ) -> Result<Vec<DocumentEntityLink>, LinkError> {
        let sql = if include_rejected {
            "SELECT link_id, file_id, entity_type, entity_id, attribution_source, confidence, \
             rationale, actor, user_override_actor, user_override_at, rejected, rejected_at, \
             rejected_reason, created_at, updated_at \
             FROM document_entity_links WHERE file_id = ?1 ORDER BY created_at"
        } else {
            "SELECT link_id, file_id, entity_type, entity_id, attribution_source, confidence, \
             rationale, actor, user_override_actor, user_override_at, rejected, rejected_at, \
             rejected_reason, created_at, updated_at \
             FROM document_entity_links WHERE file_id = ?1 AND rejected = 0 ORDER BY created_at"
        };

        let mut stmt = conn
            .prepare(sql)
            .map_err(|e| LinkError::DbError(e.to_string()))?;
        let rows = stmt
            .query_map(params![file_id], row_to_link)
            .map_err(|e| LinkError::DbError(e.to_string()))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| LinkError::DbError(e.to_string()))
    }

    /// Endorse-existing-active API. Marks an active link as user-confirmed
    /// (sets `user_override_actor` + `user_override_at`); emits the
    /// link-changed signal via the supplied `SignalEmitter`. Returns
    /// `LinkError::NotFound` if no active row exists for the triple.
    ///
    /// Does NOT resurrect rejected links — that path goes through
    /// `add_link(... UserRelink)` (V1.3 fold #2).
    pub fn override_link(
        conn: &Connection,
        emitter: &dyn SignalEmitter,
        file_id: &str,
        entity_type: EntityType,
        entity_id: &str,
        actor: &str,
    ) -> Result<(), LinkError> {
        let rows = conn
            .execute(
                "UPDATE document_entity_links SET \
                 user_override_actor = ?1, \
                 user_override_at = strftime('%Y-%m-%dT%H:%M:%fZ','now'), \
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') \
                 WHERE file_id = ?2 AND entity_type = ?3 AND entity_id = ?4 AND rejected = 0",
                params![actor, file_id, entity_type_slug(entity_type), entity_id],
            )
            .map_err(|e| LinkError::DbError(e.to_string()))?;
        if rows == 0 {
            return Err(LinkError::NotFound);
        }
        emitter.emit_link_changed(file_id, entity_id, actor);
        Ok(())
    }

    /// Reject-existing API. Flips `rejected = 1` and populates
    /// `rejected_at` and `rejected_reason`. The partial UNIQUE on
    /// `WHERE rejected = 0` releases its hold; future `add_link` attempts
    /// from classifier sources hit the `Tombstoned` guard.
    pub fn reject_link(
        conn: &Connection,
        file_id: &str,
        entity_type: EntityType,
        entity_id: &str,
        _actor: &str,
        reason: &str,
    ) -> Result<(), LinkError> {
        let rows = conn
            .execute(
                "UPDATE document_entity_links SET \
                 rejected = 1, \
                 rejected_at = strftime('%Y-%m-%dT%H:%M:%fZ','now'), \
                 rejected_reason = ?1, \
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') \
                 WHERE file_id = ?2 AND entity_type = ?3 AND entity_id = ?4 AND rejected = 0",
                params![reason, file_id, entity_type_slug(entity_type), entity_id],
            )
            .map_err(|e| LinkError::DbError(e.to_string()))?;
        if rows == 0 {
            return Err(LinkError::NotFound);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::workspace_ingestion::contracts::NullSignalEmitter;

    fn fresh_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory sqlite");
        conn.execute_batch(include_str!(
            "../../migrations/250_workspace_file_lifecycle.sql"
        ))
        .expect("v250 apply");
        conn.execute_batch(include_str!(
            "../../migrations/254_document_entity_links.sql"
        ))
        .expect("v254 apply");
        // Seed a workspace_file_lifecycle row for FK.
        conn.execute(
            "INSERT INTO workspace_file_lifecycle (file_id, canonical_path, device, inode, \
             source_type, data_source, source_asof) VALUES (?, ?, ?, ?, ?, ?, ?)",
            params!["wf-1", "test/path", 0_i64, 0_i64, "inbox", "{}", "2026-05-21T00:00:00Z"],
        )
        .expect("seed file_lifecycle");
        conn
    }

    fn insert_link(
        conn: &Connection,
        link_id: &str,
        file_id: &str,
        entity_type: EntityType,
        entity_id: &str,
        attribution: LinkAttributionSource,
        rejected: bool,
    ) {
        conn.execute(
            "INSERT INTO document_entity_links \
             (link_id, file_id, entity_type, entity_id, attribution_source, confidence, actor, rejected) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                link_id,
                file_id,
                entity_type_slug(entity_type),
                entity_id,
                attribution.as_storage_str(),
                0.5_f64,
                "test-actor",
                if rejected { 1_i64 } else { 0_i64 }
            ],
        )
        .expect("insert link");
    }

    #[test]
    fn list_links_for_file_filters_rejected_when_include_false() {
        let conn = fresh_conn();
        insert_link(
            &conn,
            "l-1",
            "wf-1",
            EntityType::Account,
            "acme",
            LinkAttributionSource::Classifier,
            false,
        );
        insert_link(
            &conn,
            "l-2",
            "wf-1",
            EntityType::Account,
            "rejected-co",
            LinkAttributionSource::Classifier,
            true,
        );
        let active = LinkRepo::list_links_for_file(&conn, "wf-1", false).expect("Ok");
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].link_id.0, "l-1");

        let all = LinkRepo::list_links_for_file(&conn, "wf-1", true).expect("Ok");
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn list_links_for_file_returns_empty_for_unknown_file() {
        let conn = fresh_conn();
        let result = LinkRepo::list_links_for_file(&conn, "no-such-file", false).expect("Ok");
        assert!(result.is_empty());
    }

    #[test]
    fn override_link_updates_active_row_and_emits_signal() {
        let conn = fresh_conn();
        insert_link(
            &conn,
            "l-1",
            "wf-1",
            EntityType::Account,
            "acme",
            LinkAttributionSource::Classifier,
            false,
        );
        let emitter = NullSignalEmitter;
        LinkRepo::override_link(
            &conn,
            &emitter,
            "wf-1",
            EntityType::Account,
            "acme",
            "user-1",
        )
        .expect("Ok");

        let links = LinkRepo::list_links_for_file(&conn, "wf-1", false).expect("Ok");
        assert_eq!(links.len(), 1);
        let user_override = links[0].user_override.as_ref().expect("user_override set");
        assert_eq!(user_override.actor_id, "user-1");
    }

    #[test]
    fn override_link_returns_notfound_for_missing_triple() {
        let conn = fresh_conn();
        let emitter = NullSignalEmitter;
        let err = LinkRepo::override_link(
            &conn,
            &emitter,
            "wf-1",
            EntityType::Account,
            "nobody",
            "user-1",
        )
        .expect_err("missing");
        assert!(matches!(err, LinkError::NotFound));
    }

    #[test]
    fn override_link_does_not_resurrect_rejected_link() {
        let conn = fresh_conn();
        insert_link(
            &conn,
            "l-1",
            "wf-1",
            EntityType::Account,
            "acme",
            LinkAttributionSource::Classifier,
            true, // rejected
        );
        let emitter = NullSignalEmitter;
        // override_link only touches active rows; rejected link → NotFound.
        let err = LinkRepo::override_link(
            &conn,
            &emitter,
            "wf-1",
            EntityType::Account,
            "acme",
            "user-1",
        )
        .expect_err("rejected link should NotFound");
        assert!(matches!(err, LinkError::NotFound));
    }

    #[test]
    fn reject_link_flips_rejected_flag_and_populates_audit_fields() {
        let conn = fresh_conn();
        insert_link(
            &conn,
            "l-1",
            "wf-1",
            EntityType::Account,
            "acme",
            LinkAttributionSource::Classifier,
            false,
        );
        LinkRepo::reject_link(
            &conn,
            "wf-1",
            EntityType::Account,
            "acme",
            "user-1",
            "wrong entity",
        )
        .expect("reject Ok");

        let links = LinkRepo::list_links_for_file(&conn, "wf-1", true).expect("Ok");
        assert_eq!(links.len(), 1);
        assert!(links[0].rejected);
        assert_eq!(links[0].rejected_reason.as_deref(), Some("wrong entity"));
        assert!(links[0].rejected_at.is_some());
    }

    #[test]
    fn reject_link_returns_notfound_when_no_active_row() {
        let conn = fresh_conn();
        let err = LinkRepo::reject_link(
            &conn,
            "wf-1",
            EntityType::Account,
            "ghost",
            "user-1",
            "doesn't exist",
        )
        .expect_err("missing");
        assert!(matches!(err, LinkError::NotFound));
    }

    #[test]
    fn add_link_inserts_new_active_row_via_classifier() {
        let conn = fresh_conn();
        let id = LinkRepo::add_link(
            &conn,
            "wf-1",
            EntityType::Account,
            "acme",
            LinkAttributionSource::Classifier,
            0.7,
            Some("inferred"),
            "agent-1",
        )
        .expect("Ok");
        assert_eq!(id.0.len(), 36);
        let links = LinkRepo::list_links_for_file(&conn, "wf-1", false).expect("Ok");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].confidence, 0.7);
        assert_eq!(links[0].rationale.as_deref(), Some("inferred"));
    }

    #[test]
    fn add_link_classifier_duplicate_returns_existing_id_idempotent() {
        let conn = fresh_conn();
        let id1 = LinkRepo::add_link(
            &conn,
            "wf-1",
            EntityType::Account,
            "acme",
            LinkAttributionSource::Classifier,
            0.5,
            None,
            "agent-1",
        )
        .expect("first Ok");
        let id2 = LinkRepo::add_link(
            &conn,
            "wf-1",
            EntityType::Account,
            "acme",
            LinkAttributionSource::Classifier,
            0.9,
            Some("re-attempt"),
            "agent-2",
        )
        .expect("second Ok");
        assert_eq!(id1, id2, "duplicate add_link should return existing link_id");
        let links = LinkRepo::list_links_for_file(&conn, "wf-1", false).expect("Ok");
        assert_eq!(links.len(), 1, "no new row created");
        // Original values preserved (DO NOTHING semantics).
        assert_eq!(links[0].confidence, 0.5);
    }

    #[test]
    fn add_link_classifier_blocked_by_tombstone_returns_typed_err() {
        let conn = fresh_conn();
        let id = LinkRepo::add_link(
            &conn,
            "wf-1",
            EntityType::Account,
            "acme",
            LinkAttributionSource::Classifier,
            0.5,
            None,
            "agent-1",
        )
        .expect("first Ok");
        // Reject the link.
        LinkRepo::reject_link(
            &conn,
            "wf-1",
            EntityType::Account,
            "acme",
            "user-1",
            "wrong entity",
        )
        .expect("reject Ok");
        // Classifier retries — should be blocked.
        let err = LinkRepo::add_link(
            &conn,
            "wf-1",
            EntityType::Account,
            "acme",
            LinkAttributionSource::Classifier,
            0.5,
            None,
            "agent-1",
        )
        .expect_err("classifier should be Tombstoned");
        match err {
            LinkError::Tombstoned {
                rejected_reason, ..
            } => {
                assert_eq!(rejected_reason, "wrong entity");
            }
            other => panic!("expected Tombstoned, got {other:?}"),
        }
        // No new active link created.
        let active = LinkRepo::list_links_for_file(&conn, "wf-1", false).expect("Ok");
        assert!(active.is_empty(), "rejected link must NOT resurrect via classifier");
        // But the rejected row still exists.
        let all = LinkRepo::list_links_for_file(&conn, "wf-1", true).expect("Ok");
        assert_eq!(all.len(), 1);
        assert!(all[0].rejected);
        // Sanity: confirm id is unused (the rejected row keeps its original id).
        assert_eq!(all[0].link_id, id);
    }

    #[test]
    fn add_link_user_relink_bypasses_tombstone_and_creates_active_row() {
        let conn = fresh_conn();
        // Seed classifier link, reject it.
        LinkRepo::add_link(
            &conn,
            "wf-1",
            EntityType::Account,
            "acme",
            LinkAttributionSource::Classifier,
            0.5,
            None,
            "agent-1",
        )
        .expect("classifier add Ok");
        LinkRepo::reject_link(
            &conn,
            "wf-1",
            EntityType::Account,
            "acme",
            "user-1",
            "wrong, but…",
        )
        .expect("reject Ok");

        // User-relink: should bypass tombstone guard and create a NEW active row.
        let relink_id = LinkRepo::add_link(
            &conn,
            "wf-1",
            EntityType::Account,
            "acme",
            LinkAttributionSource::UserRelink,
            1.0,
            Some("user confirms binding"),
            "user-1",
        )
        .expect("UserRelink bypass Ok");

        let active = LinkRepo::list_links_for_file(&conn, "wf-1", false).expect("Ok");
        assert_eq!(active.len(), 1, "UserRelink creates fresh active row");
        assert_eq!(active[0].link_id, relink_id);
        assert!(matches!(
            active[0].attribution_source,
            LinkAttributionSource::UserRelink
        ));
    }

    #[test]
    fn add_link_multi_entity_allowed_for_same_file() {
        let conn = fresh_conn();
        let a = LinkRepo::add_link(
            &conn,
            "wf-1",
            EntityType::Account,
            "acme",
            LinkAttributionSource::Classifier,
            0.5,
            None,
            "agent-1",
        )
        .expect("Ok");
        let b = LinkRepo::add_link(
            &conn,
            "wf-1",
            EntityType::Project,
            "apollo",
            LinkAttributionSource::Classifier,
            0.5,
            None,
            "agent-1",
        )
        .expect("Ok");
        assert_ne!(a, b);
        let links = LinkRepo::list_links_for_file(&conn, "wf-1", false).expect("Ok");
        assert_eq!(links.len(), 2);
    }

    #[test]
    fn partial_unique_index_prevents_duplicate_active_insert_via_raw_sql() {
        // Sanity check: confirms the v254 partial UNIQUE index actually fires.
        // Note: add_link with proper UPSERT handles this gracefully; this test
        // exercises the raw INSERT path to verify the index is in place.
        let conn = fresh_conn();
        insert_link(
            &conn,
            "l-1",
            "wf-1",
            EntityType::Account,
            "acme",
            LinkAttributionSource::Classifier,
            false,
        );
        let result = conn.execute(
            "INSERT INTO document_entity_links \
             (link_id, file_id, entity_type, entity_id, attribution_source, confidence, actor) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            params!["l-2", "wf-1", "account", "acme", "classifier", 0.5_f64, "test-actor"],
        );
        assert!(result.is_err(), "second active insert for same triple should fail UNIQUE");
    }
}
