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

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

/// Opaque UUID4 identifier for an ingestion run.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IngestionRunId(pub String);

/// Ingestion mode per L0 V1.3 §4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IngestionMode {
    Initial,
    Incremental,
    Forced,
    Backfill,
}

impl IngestionMode {
    pub fn as_storage_str(self) -> &'static str {
        match self {
            Self::Initial => "initial",
            Self::Incremental => "incremental",
            Self::Forced => "forced",
            Self::Backfill => "backfill",
        }
    }

    pub fn from_storage_str(s: &str) -> Option<Self> {
        match s {
            "initial" => Some(Self::Initial),
            "incremental" => Some(Self::Incremental),
            "forced" => Some(Self::Forced),
            "backfill" => Some(Self::Backfill),
            _ => None,
        }
    }
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

impl IngestionRunStatus {
    pub fn as_storage_str(self) -> &'static str {
        match self {
            Self::InProgress => "in_progress",
            Self::Success => "success",
            Self::Failed => "failed",
            Self::Aborted => "aborted",
        }
    }

    pub fn from_storage_str(s: &str) -> Option<Self> {
        match s {
            "in_progress" => Some(Self::InProgress),
            "success" => Some(Self::Success),
            "failed" => Some(Self::Failed),
            "aborted" => Some(Self::Aborted),
            _ => None,
        }
    }
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
    /// See module doc-comment for the full algorithm. UNIMPLEMENTED — defer to
    /// next iteration. The implementation requires careful handling of:
    /// - SQL UNIQUE partial index interaction with INSERT
    /// - 1-hour staleness threshold (use `chrono::Duration::hours(1)`)
    /// - Retry lineage (`mode = Forced` requires `retry_of_run_id`)
    /// - Wall-clock determinism for tests
    pub fn start_run(_conn: &Connection, _seed: StartRunSeed) -> Result<IngestionRunId, RunsError> {
        unimplemented!(
            "W1-C implementing agent: implement two-tier idempotency per \
             runs.rs module doc-comment + L0 V1.3 §4 fold #3."
        )
    }

    /// Transitions a run to a terminal state. Returns `NotFound` if no row
    /// with that `run_id` exists. Note: if `status = Success` and a prior
    /// success row already exists for the idempotency triple, the UNIQUE
    /// partial index fires and returns `RunsError::DbError`. (V1.3 fold #6
    /// idempotency convergence requires the service-layer guard in
    /// `start_run` to catch this earlier; `complete_run` is a thin SQL
    /// wrapper at this layer.)
    pub fn complete_run(
        conn: &Connection,
        run_id: &IngestionRunId,
        status: IngestionRunStatus,
        claim_count: u64,
        error_log: Option<serde_json::Value>,
    ) -> Result<(), RunsError> {
        let error_log_json = error_log
            .map(|v| serde_json::to_string(&v).unwrap_or_else(|_| "null".to_string()));
        let rows = conn
            .execute(
                "UPDATE document_ingestion_runs SET \
                 status = ?1, \
                 completed_at = strftime('%Y-%m-%dT%H:%M:%fZ','now'), \
                 claim_count_produced = ?2, \
                 error_log = ?3 \
                 WHERE run_id = ?4",
                params![
                    status.as_storage_str(),
                    claim_count as i64,
                    error_log_json,
                    run_id.0
                ],
            )
            .map_err(|e| RunsError::DbError(e.to_string()))?;

        if rows == 0 {
            Err(RunsError::NotFound)
        } else {
            Ok(())
        }
    }

    /// Returns the most recent successful run matching the idempotency key,
    /// if any. Used by `start_run` preflight and by W4-A history display.
    pub fn find_by_idempotency_key(
        conn: &Connection,
        file_id: &str,
        content_sha256: &str,
        mode: IngestionMode,
    ) -> Result<Option<ExistingRunReceipt>, RunsError> {
        conn.query_row(
            "SELECT run_id, completed_at, status, claim_count_produced \
             FROM document_ingestion_runs \
             WHERE file_id = ?1 AND content_sha256 = ?2 AND mode = ?3 \
               AND status = 'success' \
             ORDER BY completed_at DESC LIMIT 1",
            params![file_id, content_sha256, mode.as_storage_str()],
            |row| {
                let run_id: String = row.get(0)?;
                let completed_at_raw: Option<String> = row.get(1)?;
                let status_raw: String = row.get(2)?;
                let claim_count: i64 = row.get(3)?;
                let completed_at = completed_at_raw
                    .as_deref()
                    .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                    .map(|dt| dt.with_timezone(&Utc));
                Ok(ExistingRunReceipt {
                    run_id: IngestionRunId(run_id),
                    completed_at,
                    status: IngestionRunStatus::from_storage_str(&status_raw)
                        .unwrap_or(IngestionRunStatus::Success),
                    claim_count_produced: claim_count as u64,
                })
            },
        )
        .optional()
        .map_err(|e| RunsError::DbError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory sqlite");
        // W1-A v250 (FK target) + W1-C v253 (runs table).
        conn.execute_batch(include_str!(
            "../../migrations/250_workspace_file_lifecycle.sql"
        ))
        .expect("v250 apply");
        conn.execute_batch(include_str!(
            "../../migrations/253_document_ingestion_runs.sql"
        ))
        .expect("v253 apply");
        // Seed one workspace_file_lifecycle row for FK.
        conn.execute(
            "INSERT INTO workspace_file_lifecycle (file_id, canonical_path, device, inode, \
             source_type, data_source, source_asof) VALUES (?, ?, ?, ?, ?, ?, ?)",
            params!["wf-1", "test/path", 0_i64, 0_i64, "inbox", "{}", "2026-05-21T00:00:00Z"],
        )
        .expect("seed file_lifecycle");
        conn
    }

    fn insert_run(
        conn: &Connection,
        run_id: &str,
        file_id: &str,
        content_sha256: &str,
        mode: IngestionMode,
        status: IngestionRunStatus,
    ) {
        conn.execute(
            "INSERT INTO document_ingestion_runs \
             (run_id, file_id, mode, status, content_sha256, file_size_bytes, extractor_version) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![
                run_id,
                file_id,
                mode.as_storage_str(),
                status.as_storage_str(),
                content_sha256,
                1024_i64,
                "test-extractor-v1"
            ],
        )
        .expect("insert run");
    }

    #[test]
    fn find_by_idempotency_key_returns_none_when_no_row() {
        let conn = fresh_conn();
        let result = RunsRepo::find_by_idempotency_key(
            &conn,
            "wf-1",
            "deadbeef",
            IngestionMode::Initial,
        )
        .expect("Ok");
        assert!(result.is_none());
    }

    #[test]
    fn find_by_idempotency_key_skips_in_progress_returns_only_success() {
        let conn = fresh_conn();
        insert_run(
            &conn,
            "run-pending",
            "wf-1",
            "deadbeef",
            IngestionMode::Initial,
            IngestionRunStatus::InProgress,
        );
        let result = RunsRepo::find_by_idempotency_key(
            &conn,
            "wf-1",
            "deadbeef",
            IngestionMode::Initial,
        )
        .expect("Ok");
        assert!(result.is_none(), "in_progress should be filtered out");
    }

    #[test]
    fn find_by_idempotency_key_returns_receipt_on_success_row() {
        let conn = fresh_conn();
        insert_run(
            &conn,
            "run-good",
            "wf-1",
            "deadbeef",
            IngestionMode::Initial,
            IngestionRunStatus::Success,
        );
        // Mark it as completed (need a completed_at).
        RunsRepo::complete_run(
            &conn,
            &IngestionRunId("run-good".to_string()),
            IngestionRunStatus::Success,
            5,
            None,
        )
        .expect("complete_run");

        let receipt = RunsRepo::find_by_idempotency_key(
            &conn,
            "wf-1",
            "deadbeef",
            IngestionMode::Initial,
        )
        .expect("Ok")
        .expect("Some");
        assert_eq!(receipt.run_id.0, "run-good");
        assert_eq!(receipt.status, IngestionRunStatus::Success);
        assert_eq!(receipt.claim_count_produced, 5);
        assert!(receipt.completed_at.is_some());
    }

    #[test]
    fn complete_run_returns_notfound_for_missing_run_id() {
        let conn = fresh_conn();
        let err = RunsRepo::complete_run(
            &conn,
            &IngestionRunId("nonexistent".to_string()),
            IngestionRunStatus::Success,
            0,
            None,
        )
        .expect_err("missing run_id");
        assert!(matches!(err, RunsError::NotFound));
    }

    #[test]
    fn complete_run_persists_status_claim_count_error_log() {
        let conn = fresh_conn();
        insert_run(
            &conn,
            "run-1",
            "wf-1",
            "abc",
            IngestionMode::Initial,
            IngestionRunStatus::InProgress,
        );
        let err = serde_json::json!({"errors": ["one", "two"]});
        RunsRepo::complete_run(
            &conn,
            &IngestionRunId("run-1".to_string()),
            IngestionRunStatus::Failed,
            42,
            Some(err.clone()),
        )
        .expect("Ok");

        let (status_str, claim_count, error_log): (String, i64, Option<String>) = conn
            .query_row(
                "SELECT status, claim_count_produced, error_log FROM document_ingestion_runs \
                 WHERE run_id = 'run-1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("query");
        assert_eq!(status_str, "failed");
        assert_eq!(claim_count, 42);
        assert!(error_log.unwrap().contains("\"one\""));
    }

    #[test]
    fn complete_run_to_success_then_duplicate_success_hits_unique_partial_index() {
        let conn = fresh_conn();
        // Two in_progress runs with same idempotency key are possible (partial unique
        // only covers status='success'). Completing both to success should hit UNIQUE.
        insert_run(
            &conn,
            "run-a",
            "wf-1",
            "abc",
            IngestionMode::Initial,
            IngestionRunStatus::InProgress,
        );
        insert_run(
            &conn,
            "run-b",
            "wf-1",
            "abc",
            IngestionMode::Initial,
            IngestionRunStatus::InProgress,
        );
        RunsRepo::complete_run(
            &conn,
            &IngestionRunId("run-a".to_string()),
            IngestionRunStatus::Success,
            10,
            None,
        )
        .expect("first success Ok");
        let err = RunsRepo::complete_run(
            &conn,
            &IngestionRunId("run-b".to_string()),
            IngestionRunStatus::Success,
            10,
            None,
        )
        .expect_err("second success should violate UNIQUE");
        assert!(matches!(err, RunsError::DbError(_)));
        // Confirm only one success row exists.
        let success_count: i64 = conn
            .query_row(
                "SELECT count(*) FROM document_ingestion_runs WHERE status = 'success'",
                [],
                |row| row.get(0),
            )
            .expect("count");
        assert_eq!(success_count, 1, "UNIQUE partial index enforces single success row");
    }
}
