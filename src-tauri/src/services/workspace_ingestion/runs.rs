//! Ingestion-run tracking — DOS-465 (W1-C) substrate.
//!
//! Wraps the `document_ingestion_runs` table (migration v253). Provides
//! `start_run` / `complete_run` / `find_by_idempotency_key` with two-tier
//! idempotency:
//!
//! 1. **SQL layer (storage fence):** UNIQUE partial index on
//!    `(file_id, content_sha256, mode) WHERE status = 'success'` — only one
//!    successful row per idempotency key can exist.
//!
//! 2. **Service layer (preflight):** `start_run` checks `find_by_idempotency_key`
//!    AND active in-progress rows. Returns typed
//!    `RunsError::AlreadyCompleted { existing }` or
//!    `RunsError::AlreadyInProgress { existing_run_id }` before attempting
//!    INSERT. Prevents duplicate work even when the SQL UNIQUE wouldn't fire
//!    yet (the prior run hasn't completed).
//!
//! In-progress staleness threshold: 1 hour. Older in-progress rows are
//! marked `aborted` and the new `start_run` proceeds (assumes crashed run).
//!
//! Retry lineage: `mode = Forced` requires `retry_of_run_id` non-None. The
//! prior run is left untouched in the table; the partial UNIQUE allows
//! coexistence because the new row enters in_progress status first.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Opaque UUID4 identifier for an ingestion run.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IngestionRunId(pub String);

/// Ingestion mode per L0 V1.3 §4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IngestionMode {
    /// First-time ingestion of a previously-unseen file.
    Initial,
    /// Re-ingestion of a known file whose content changed since last run.
    Incremental,
    /// Explicit user-triggered re-ingestion (requires retry_of_run_id).
    Forced,
    /// Backfill-binary ingestion of pre-existing files (W5-A consumer).
    Backfill,
}

/// Lifecycle status of an ingestion run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IngestionRunStatus {
    InProgress,
    Success,
    Failed,
    Aborted,
}

/// Request shape for `start_run` per L0 V1.3 §4.
#[derive(Debug, Clone)]
pub struct StartRunSeed {
    pub file_id: String,
    pub mode: IngestionMode,
    pub content_sha256: String,
    pub file_size_bytes: u64,
    pub extractor_version: String,
    pub retry_of_run_id: Option<IngestionRunId>,
}

/// Receipt returned by `find_by_idempotency_key` and embedded in
/// `RunsError::AlreadyCompleted`.
#[derive(Debug, Clone)]
pub struct ExistingRunReceipt {
    pub run_id: IngestionRunId,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: IngestionRunStatus,
    pub claim_count_produced: u64,
}

/// Errors returned by `RunsRepo`. Per L0 V1.3 §4 (fold #3+#4): in-progress
/// idempotency returns `AlreadyInProgress`; success idempotency returns
/// `AlreadyCompleted` with the existing receipt (NOT generic `DbError`).
#[derive(Debug)]
pub enum RunsError {
    NotFound,
    AlreadyCompleted { existing: ExistingRunReceipt },
    AlreadyInProgress { existing_run_id: IngestionRunId },
    ContentSha256Mismatch,
    DbError(String),
}

impl std::fmt::Display for RunsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "ingestion run not found"),
            Self::AlreadyCompleted { existing } => {
                write!(f, "ingestion run already completed: {:?}", existing.run_id)
            }
            Self::AlreadyInProgress { existing_run_id } => {
                write!(f, "ingestion run already in progress: {existing_run_id:?}")
            }
            Self::ContentSha256Mismatch => write!(f, "content sha256 mismatch"),
            Self::DbError(msg) => write!(f, "runs db error: {msg}"),
        }
    }
}

impl std::error::Error for RunsError {}

/// Repository for `document_ingestion_runs` (v253 table).
pub struct RunsRepo;

impl RunsRepo {
    /// Starts a new ingestion run with two-tier idempotency.
    ///
    /// 1. Pre-check via `find_by_idempotency_key` for ANY status:
    ///    - `Success` row exists → return `Err(AlreadyCompleted { existing })`.
    ///    - `InProgress` row exists, <1h old → return `Err(AlreadyInProgress)`.
    ///    - `InProgress` row exists, >=1h old → mark prior `Aborted`, proceed.
    ///    - No row → INSERT new in_progress row.
    /// 2. For `mode = Forced` with `retry_of_run_id`: bypass idempotency (the
    ///    user explicitly requested retry); the partial UNIQUE on
    ///    `WHERE status='success'` allows the new in_progress row to coexist.
    pub fn start_run(_seed: StartRunSeed) -> Result<IngestionRunId, RunsError> {
        unimplemented!(
            "W1-C implementing agent: implement two-tier idempotency per \
             runs.rs module doc-comment + L0 V1.3 §4 fold #3."
        )
    }

    /// Transitions a run to a terminal state.
    pub fn complete_run(
        _run_id: &IngestionRunId,
        _status: IngestionRunStatus,
        _claim_count: u64,
        _error_log: Option<serde_json::Value>,
    ) -> Result<(), RunsError> {
        unimplemented!(
            "W1-C implementing agent: UPDATE document_ingestion_runs SET \
             status = ?, completed_at = now, claim_count_produced = ?, error_log = ? \
             WHERE run_id = ?. If status='success', the SQL UNIQUE partial \
             enforces no duplicates."
        )
    }

    /// Returns the most recent successful run matching the idempotency key,
    /// if any. Used by `start_run` preflight and by W4-A history display.
    pub fn find_by_idempotency_key(
        _file_id: &str,
        _content_sha256: &str,
        _mode: IngestionMode,
    ) -> Result<Option<ExistingRunReceipt>, RunsError> {
        unimplemented!(
            "W1-C implementing agent: SELECT run_id, completed_at, status, \
             claim_count_produced FROM document_ingestion_runs WHERE file_id = ? \
             AND content_sha256 = ? AND mode = ? AND status = 'success' \
             ORDER BY completed_at DESC LIMIT 1."
        )
    }
}
