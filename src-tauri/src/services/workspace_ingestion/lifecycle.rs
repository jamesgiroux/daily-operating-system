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
use serde::{Deserialize, Serialize};

use super::contracts::{WorkspaceCategory, WorkspaceFileKind};

/// Seven-state lifecycle per DOS-463 §Goal + cycle 2 amendment
/// (`pending_entity_assignment`).
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
/// - Any state → `Quarantined`: W2-A's `quarantine_source(file_id, reason,
///   actor)` emits `emit_file_quarantined` (maps to
///   `WorkspaceFileQuarantined`; triggers claim retraction for the file's
///   prior claims).
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
            Self::InvalidStateTransition { from, to } => write!(
                f,
                "invalid lifecycle transition: {from:?} → {to:?}"
            ),
            Self::DbError(msg) => write!(f, "lifecycle db error: {msg}"),
        }
    }
}

impl std::error::Error for LifecycleError {}

/// User-correction escalation: move a `Rejected` file back to `Pending` for
/// re-ingestion under user override. W4-A (DOS-472) wires this from the
/// source-management block; W1-A pre-defines the API so the `user_override`
/// field on `WorkspaceFileLifecycle` is non-vacuous from W1.
///
/// Stub at W1-A — the real DB update lands when W1-C's `runs.rs` exposes the
/// transaction helper and W4-A's UI invokes it. Returning
/// `LifecycleError::DbError` until then keeps callers honest about the
/// "not-yet-wired" state without hiding the API.
pub fn escalate_to_pending(_file_id: &str, _actor: &str) -> Result<(), LifecycleError> {
    Err(LifecycleError::DbError(
        "escalate_to_pending: W1-A stub; W4-A (DOS-472) wires the real DB update".to_string(),
    ))
}
