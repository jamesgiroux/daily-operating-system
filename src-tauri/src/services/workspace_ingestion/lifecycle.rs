//! Workspace file lifecycle types and the user-correction escalation API.
//!
//! Mirrors the `workspace_file_lifecycle` table (migration v250 + v251). The
//! row is the provenance carrier for every workspace-derived claim — W1-A
//! commits no claims; that is W3-A's wave. The state-machine transitions
//! enumerated below pin which W2/W3 transitions will emit signals via the
//! `contracts::SignalEmitter` trait once W3-B's `WorkspaceSignalEmitter` impl
//! is wired in `wiring.rs`.

use abilities_runtime::abilities::provenance::source::DataSource;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::entity::EntityType;

use super::contracts::{FileIdentity, WorkspaceCategory, WorkspaceFileKind};
use super::pipeline::EntityRef;

/// Source lifecycle per wave plan §Goal + cycle 2 amendment
/// (`pending_entity_assignment`) plus W5 source-management policy states.
///
/// State-machine transitions that will trigger `contracts::SignalEmitter`
/// calls in W2/W3:
/// - `Pending`/`Ingesting` → `Ingested`: W2-A's `IngestPipeline::run` emits
///   `emit_file_ingested` (W3-B maps to `SignalType::WorkspaceFileIngested`).
/// - `Ingesting` → `Rejected`: W2-A emits `emit_file_rejected` per typed
///   `contracts::RejectionReason` (maps to `WorkspaceFileRejected`; audit-only,
///   not in invalidation allowlist).
/// - `Pending` → `PendingEntityAssignment`: W2-A emits
///   `emit_file_pending_entity_assignment` when intake cannot resolve entity
///   (maps to `WorkspaceFilePendingEntityAssignment`).
/// - Any state → `Quarantined`: W2-A's `quarantine_source(..., file_id,
///   reason, actor)` emits `emit_file_quarantined` (maps to
///   `WorkspaceFileQuarantined`; triggers invalidation for derived state).
/// - `Ingested` → `Superseded`: W2-A on successful re-ingestion of a file at
///   the same `(file_id, content_sha256)` key; no signal directly, but the
///   subsequent `Ingested` row emits its own `emit_file_ingested`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleState {
    Pending,
    PendingEntityAssignment,
    Ingesting,
    Ingested,
    Superseded,
    Rejected,
    Quarantined,
    Ignored,
    Scratchpad,
    Archived,
    Deleted,
}

/// Audit record of a user correction that bypassed automated lifecycle
/// transitions. Populated by W4-A quarantine action and W1-C
/// `link::override_link`. Stored as `(user_override_actor, user_override_at)`
/// in the `workspace_file_lifecycle` row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserOverride {
    pub actor_id: String,
    pub at: DateTime<Utc>,
}

/// One row of the `workspace_file_lifecycle` table (migration v250 + v251).
///
/// The struct constructor takes all four wave-plan-mandated fields
/// (`source_type`, `source_asof`, `data_source`, `lifecycle_state`) at
/// construction time; the trybuild test at `src-tauri/tests/trybuild/`
/// (added by §8) enforces the can't-omit invariant per wave-plan
/// §Architecture invariants.
#[derive(Debug, Clone)]
pub struct WorkspaceFileLifecycle {
    pub file_id: String,
    pub canonical_path: String,
    pub device: u64,
    pub inode: u64,
    pub source_type: WorkspaceFileKind,
    pub data_source: DataSource,
    pub lifecycle_state: LifecycleState,
    pub source_asof: DateTime<Utc>,
    pub entity_id: Option<String>,
    pub entity_type: Option<String>,
    pub content_sha256: Option<String>,
    pub category: Option<WorkspaceCategory>,
    pub user_override: Option<UserOverride>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Errors returned by the lifecycle service.
#[derive(Debug)]
pub enum LifecycleError {
    /// No `workspace_file_lifecycle` row with the given `file_id`.
    FileNotFound,
    /// Lifecycle transition rejected because the from→to pair is not
    /// permitted by the state machine.
    InvalidStateTransition {
        from: LifecycleState,
        to: LifecycleState,
    },
    /// SQLite or DB-layer error escape hatch. Carries the underlying error
    /// message for telemetry.
    DbError(String),
}

impl std::fmt::Display for LifecycleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FileNotFound => write!(f, "workspace file not found"),
            Self::InvalidStateTransition { from, to } => {
                write!(f, "invalid lifecycle transition: {from:?} → {to:?}")
            }
            Self::DbError(msg) => write!(f, "lifecycle db error: {msg}"),
        }
    }
}

impl std::error::Error for LifecycleError {}

/// User-correction escalation: move a `Rejected` file back to `Pending` for
/// re-ingestion under user override. W4-A wires this from the
/// source-management block; W1-A pre-defines the API so the `user_override`
/// field on `WorkspaceFileLifecycle` is non-vacuous from W1.
///
/// Stub at W1-A — the real DB update lands when W1-C's `runs.rs` exposes the
/// transaction helper and W4-A's UI invokes it. Returning
/// `LifecycleError::DbError` until then keeps callers honest about the
/// "not-yet-wired" state without hiding the API.
pub fn escalate_to_pending(_file_id: &str, _actor: &str) -> Result<(), LifecycleError> {
    Err(LifecycleError::DbError(
        "escalate_to_pending: W1-A stub; W4-A wires the real DB update".to_string(),
    ))
}

pub struct LifecycleRepo;

impl LifecycleRepo {
    pub fn insert_pending(
        conn: &Connection,
        file_id: &str,
        identity: &FileIdentity,
        source_type: &WorkspaceFileKind,
        source_asof: DateTime<Utc>,
        entity: Option<&EntityRef>,
    ) -> Result<(), LifecycleError> {
        let data_source = DataSource::WorkspaceFile {
            kind: source_type.clone(),
        };
        let data_source_json = serde_json::to_string(&data_source)
            .map_err(|e| LifecycleError::DbError(e.to_string()))?;
        let canonical_path = identity.canonical_path.to_string_lossy().to_string();
        let entity_id = entity.map(|e| e.entity_id.0.as_str());
        let entity_type = entity.map(|e| e.entity_type.as_str());

        conn.execute(
            "INSERT OR IGNORE INTO workspace_file_lifecycle \
             (file_id, canonical_path, device, inode, source_type, data_source, \
              lifecycle_state, source_asof, entity_id, entity_type) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'pending', ?7, ?8, ?9)",
            params![
                file_id,
                canonical_path,
                identity.device as i64,
                identity.inode as i64,
                workspace_file_kind_slug(source_type),
                data_source_json,
                source_asof.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                entity_id,
                entity_type,
            ],
        )
        .map(|_| ())
        .map_err(|e| LifecycleError::DbError(e.to_string()))
    }

    pub fn transition(
        conn: &Connection,
        file_id: &str,
        from: LifecycleState,
        to: LifecycleState,
    ) -> Result<(), LifecycleError> {
        if !is_valid_transition(from, to) {
            return Err(LifecycleError::InvalidStateTransition { from, to });
        }
        let rows = conn
            .execute(
                "UPDATE workspace_file_lifecycle SET lifecycle_state = ?1, \
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') \
                 WHERE file_id = ?2 AND lifecycle_state = ?3",
                params![
                    lifecycle_state_slug(to),
                    file_id,
                    lifecycle_state_slug(from)
                ],
            )
            .map_err(|e| LifecycleError::DbError(e.to_string()))?;
        if rows > 0 {
            return Ok(());
        }
        let current = Self::get(conn, file_id)?.ok_or(LifecycleError::FileNotFound)?;
        if current.lifecycle_state == to {
            Ok(())
        } else {
            Err(LifecycleError::InvalidStateTransition {
                from: current.lifecycle_state,
                to,
            })
        }
    }

    pub fn record_user_override(
        conn: &Connection,
        file_id: &str,
        actor: &str,
    ) -> Result<(), LifecycleError> {
        let rows = conn
            .execute(
                "UPDATE workspace_file_lifecycle SET user_override_actor = ?1, \
                 user_override_at = strftime('%Y-%m-%dT%H:%M:%fZ','now'), \
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') \
                 WHERE file_id = ?2",
                params![actor, file_id],
            )
            .map_err(|e| LifecycleError::DbError(e.to_string()))?;
        if rows == 0 {
            Err(LifecycleError::FileNotFound)
        } else {
            Ok(())
        }
    }

    pub fn update_category(
        conn: &Connection,
        file_id: &str,
        category: Option<&WorkspaceCategory>,
    ) -> Result<(), LifecycleError> {
        let rows = conn
            .execute(
                "UPDATE workspace_file_lifecycle SET category = ?1, \
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') \
                 WHERE file_id = ?2",
                params![category.map(WorkspaceCategory::as_slug), file_id],
            )
            .map_err(|e| LifecycleError::DbError(e.to_string()))?;
        if rows == 0 {
            Err(LifecycleError::FileNotFound)
        } else {
            Ok(())
        }
    }

    pub(crate) fn update_content_sha256(
        conn: &Connection,
        file_id: &str,
        content_sha256: &str,
    ) -> Result<(), LifecycleError> {
        let rows = conn
            .execute(
                "UPDATE workspace_file_lifecycle SET content_sha256 = ?1, \
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') \
                 WHERE file_id = ?2",
                params![content_sha256, file_id],
            )
            .map_err(|e| LifecycleError::DbError(e.to_string()))?;
        if rows == 0 {
            Err(LifecycleError::FileNotFound)
        } else {
            Ok(())
        }
    }

    pub fn get(
        conn: &Connection,
        file_id: &str,
    ) -> Result<Option<WorkspaceFileLifecycle>, LifecycleError> {
        conn.query_row(
            "SELECT file_id, canonical_path, device, inode, source_type, data_source, \
             lifecycle_state, source_asof, entity_id, entity_type, content_sha256, category, \
             user_override_actor, user_override_at, created_at, updated_at \
             FROM workspace_file_lifecycle WHERE file_id = ?1",
            params![file_id],
            row_to_lifecycle,
        )
        .optional()
        .map_err(|e| LifecycleError::DbError(e.to_string()))
    }

    pub fn set_entity(
        conn: &Connection,
        file_id: &str,
        entity_type: EntityType,
        entity_id: &str,
        entity_name: Option<&str>,
    ) -> Result<(), LifecycleError> {
        let rows = conn
            .execute(
                "UPDATE workspace_file_lifecycle SET entity_type = ?1, entity_id = ?2, \
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') \
                 WHERE file_id = ?3",
                params![entity_type.as_str(), entity_id, file_id],
            )
            .map_err(|e| LifecycleError::DbError(e.to_string()))?;
        let _path_segment = entity_name;
        if rows == 0 {
            Err(LifecycleError::FileNotFound)
        } else {
            Ok(())
        }
    }
}

fn row_to_lifecycle(row: &rusqlite::Row<'_>) -> rusqlite::Result<WorkspaceFileLifecycle> {
    let source_type_raw: String = row.get(4)?;
    let data_source_raw: String = row.get(5)?;
    let lifecycle_state_raw: String = row.get(6)?;
    let source_asof_raw: String = row.get(7)?;
    let category_raw: Option<String> = row.get(11)?;
    let override_actor: Option<String> = row.get(12)?;
    let override_at_raw: Option<String> = row.get(13)?;
    let created_at_raw: String = row.get(14)?;
    let updated_at_raw: String = row.get(15)?;

    Ok(WorkspaceFileLifecycle {
        file_id: row.get(0)?,
        canonical_path: row.get(1)?,
        device: row.get::<_, i64>(2)? as u64,
        inode: row.get::<_, i64>(3)? as u64,
        source_type: workspace_file_kind_from_slug(&source_type_raw)
            .unwrap_or(WorkspaceFileKind::Inbox),
        data_source: serde_json::from_str(&data_source_raw).unwrap_or(DataSource::WorkspaceFile {
            kind: WorkspaceFileKind::Inbox,
        }),
        lifecycle_state: lifecycle_state_from_slug(&lifecycle_state_raw)
            .unwrap_or(LifecycleState::Pending),
        source_asof: parse_dt(&source_asof_raw),
        entity_id: row.get(8)?,
        entity_type: row.get(9)?,
        content_sha256: row.get(10)?,
        category: category_raw
            .as_deref()
            .and_then(WorkspaceCategory::from_slug),
        user_override: match (override_actor, override_at_raw.as_deref().map(parse_dt)) {
            (Some(actor_id), Some(at)) => Some(UserOverride { actor_id, at }),
            _ => None,
        },
        created_at: parse_dt(&created_at_raw),
        updated_at: parse_dt(&updated_at_raw),
    })
}

fn parse_dt(raw: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

fn is_valid_transition(from: LifecycleState, to: LifecycleState) -> bool {
    if from == to {
        return true;
    }
    matches!(
        (from, to),
        (LifecycleState::Pending, LifecycleState::Ingesting)
            | (LifecycleState::Pending, LifecycleState::Ingested)
            | (LifecycleState::Pending, LifecycleState::Rejected)
            | (
                LifecycleState::Pending,
                LifecycleState::PendingEntityAssignment
            )
            | (LifecycleState::Ingesting, LifecycleState::Ingested)
            | (LifecycleState::Ingesting, LifecycleState::Rejected)
            | (
                LifecycleState::Ingesting,
                LifecycleState::PendingEntityAssignment
            )
            | (
                LifecycleState::PendingEntityAssignment,
                LifecycleState::Pending
            )
            | (LifecycleState::Ingested, LifecycleState::Superseded)
            | (_, LifecycleState::Quarantined)
            | (_, LifecycleState::Ignored)
            | (_, LifecycleState::Scratchpad)
            | (_, LifecycleState::Archived)
            | (_, LifecycleState::Deleted)
            | (LifecycleState::Rejected, LifecycleState::Pending)
    )
}

pub fn lifecycle_state_slug(state: LifecycleState) -> &'static str {
    match state {
        LifecycleState::Pending => "pending",
        LifecycleState::PendingEntityAssignment => "pending_entity_assignment",
        LifecycleState::Ingesting => "ingesting",
        LifecycleState::Ingested => "ingested",
        LifecycleState::Superseded => "superseded",
        LifecycleState::Rejected => "rejected",
        LifecycleState::Quarantined => "quarantined",
        LifecycleState::Ignored => "ignored",
        LifecycleState::Scratchpad => "scratchpad",
        LifecycleState::Archived => "archived",
        LifecycleState::Deleted => "deleted",
    }
}

pub fn lifecycle_state_from_slug(slug: &str) -> Option<LifecycleState> {
    match slug {
        "pending" => Some(LifecycleState::Pending),
        "pending_entity_assignment" => Some(LifecycleState::PendingEntityAssignment),
        "ingesting" => Some(LifecycleState::Ingesting),
        "ingested" => Some(LifecycleState::Ingested),
        "superseded" => Some(LifecycleState::Superseded),
        "rejected" => Some(LifecycleState::Rejected),
        "quarantined" => Some(LifecycleState::Quarantined),
        "ignored" => Some(LifecycleState::Ignored),
        "scratchpad" => Some(LifecycleState::Scratchpad),
        "archived" => Some(LifecycleState::Archived),
        "deleted" => Some(LifecycleState::Deleted),
        _ => None,
    }
}

pub fn workspace_file_kind_slug(kind: &WorkspaceFileKind) -> &'static str {
    match kind {
        WorkspaceFileKind::Inbox => "inbox",
        WorkspaceFileKind::EntityDoc => "entity_doc",
        WorkspaceFileKind::DriveSync => "drive_sync",
        WorkspaceFileKind::UserAttachment => "user_attachment",
        WorkspaceFileKind::GranolaTranscript => "granola_transcript",
        WorkspaceFileKind::QuillTranscript => "quill_transcript",
        WorkspaceFileKind::McpPlacement => "mcp_placement",
    }
}

pub fn workspace_file_kind_from_slug(slug: &str) -> Option<WorkspaceFileKind> {
    match slug {
        "inbox" => Some(WorkspaceFileKind::Inbox),
        "entity_doc" => Some(WorkspaceFileKind::EntityDoc),
        "drive_sync" => Some(WorkspaceFileKind::DriveSync),
        "user_attachment" => Some(WorkspaceFileKind::UserAttachment),
        "granola_transcript" => Some(WorkspaceFileKind::GranolaTranscript),
        "quill_transcript" => Some(WorkspaceFileKind::QuillTranscript),
        "mcp_placement" => Some(WorkspaceFileKind::McpPlacement),
        _ => None,
    }
}
