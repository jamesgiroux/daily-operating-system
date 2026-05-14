use chrono::{Duration, Utc};
use rusqlite::{params, OptionalExtension};
use serde_json::json;

use crate::db::{ActionDb, DbError};

pub const STATUS_PENDING: &str = "pending";
pub const STATUS_RUNNING: &str = "running";
pub const STATUS_COMPLETED: &str = "completed";
pub const STATUS_DEAD_LETTERED: &str = "dead_lettered";
pub const STATUS_CYCLE_DETECTED: &str = "cycle_detected";

pub const KIND_SIGNAL_INVALIDATION: &str = "signal_invalidation";
pub const KIND_CLAIM_RECOMPUTE: &str = "claim_recompute";
pub const KIND_TARGETED_REPAIR: &str = "targeted_repair";
pub const KIND_TRANSFORM: &str = "transform";
pub const KIND_MAINTENANCE_APPLY: &str = "maintenance_apply";
pub const KIND_OUTBOX_REPLAY: &str = "outbox_replay";

pub const DEFAULT_QUEUE_PENDING_CAP: i64 = 10_000;
const DEFAULT_CHAIN_DEPTH_CAP: i64 = 16;
const CLAIM_RECOMPUTE_ABILITY_ID: &str = "claim_recompute";
const CLAIM_RECOMPUTE_ABILITY_VERSION: &str = "1";
const TARGETED_REPAIR_POLICY_REPAIR_OPERATION: &str = "targeted_claim_repair:PolicyRepair";

#[derive(Debug, Clone)]
pub struct EnqueueInvalidationJob {
    pub job_kind: String,
    pub operation: String,
    pub origin_signal_id: Option<String>,
    pub subject_type: String,
    pub subject_id: String,
    pub ability_id: String,
    pub ability_version: String,
    pub source_claim_version: i64,
    pub source_asof: Option<String>,
    pub input_snapshot_hash: Option<String>,
    pub provider_fingerprint: Option<String>,
    pub prompt_fingerprint: Option<String>,
    pub payload_json: serde_json::Value,
    pub coalescing_key: Option<String>,
    pub chain_id: Option<String>,
    pub parent_job_id: Option<String>,
    pub successor_of_job_id: Option<String>,
    pub depth: i64,
    pub chain_ancestry: Vec<String>,
    pub max_attempts: i64,
    pub priority: i64,
    pub raw_signal_count: i64,
}

impl EnqueueInvalidationJob {
    pub fn claim_recompute_from_signal(
        origin_signal_id: &str,
        subject_type: &str,
        subject_id: &str,
        source_claim_version: i64,
    ) -> Self {
        let input_snapshot_hash =
            claim_recompute_input_hash(subject_type, subject_id, source_claim_version);
        Self {
            job_kind: KIND_CLAIM_RECOMPUTE.to_string(),
            operation: "claim_recompute".to_string(),
            origin_signal_id: Some(origin_signal_id.to_string()),
            subject_type: subject_type.to_string(),
            subject_id: subject_id.to_string(),
            ability_id: CLAIM_RECOMPUTE_ABILITY_ID.to_string(),
            ability_version: CLAIM_RECOMPUTE_ABILITY_VERSION.to_string(),
            source_claim_version,
            source_asof: None,
            input_snapshot_hash: Some(input_snapshot_hash.clone()),
            provider_fingerprint: None,
            prompt_fingerprint: None,
            payload_json: json!({}),
            coalescing_key: Some(claim_recompute_coalescing_key(
                subject_type,
                subject_id,
                &input_snapshot_hash,
            )),
            chain_id: None,
            parent_job_id: None,
            successor_of_job_id: None,
            depth: 0,
            chain_ancestry: Vec::new(),
            max_attempts: 5,
            priority: 0,
            raw_signal_count: 1,
        }
    }
}

#[derive(Debug, Clone)]
pub struct InvalidationJob {
    pub id: String,
    pub job_kind: String,
    pub operation: String,
    pub status: String,
    pub priority: i64,
    pub chain_id: String,
    pub parent_job_id: Option<String>,
    pub successor_of_job_id: Option<String>,
    pub origin_signal_id: Option<String>,
    pub depth: i64,
    pub chain_ancestry_json: String,
    pub idempotency_key: String,
    pub coalescing_key: Option<String>,
    pub subject_type: String,
    pub subject_id: String,
    pub ability_id: String,
    pub ability_version: String,
    pub source_claim_version: i64,
    pub latest_source_claim_version: i64,
    pub source_asof: Option<String>,
    pub input_snapshot_hash: Option<String>,
    pub provider_fingerprint: Option<String>,
    pub prompt_fingerprint: Option<String>,
    pub payload_json: String,
    pub first_signal_id: Option<String>,
    pub latest_signal_id: Option<String>,
    pub raw_signal_count: i64,
    pub attempts: i64,
    pub max_attempts: i64,
    pub lease_owner: Option<String>,
    pub lease_expires_at: Option<String>,
    pub stale_marker_json: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidationJobReceipt {
    pub job_id: String,
    pub chain_id: String,
    pub status: String,
    pub coalesced: bool,
    pub successor_of_job_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidationQueueBounds {
    pub pending_cap: i64,
}

impl InvalidationQueueBounds {
    pub fn with_pending_cap(pending_cap: i64) -> Self {
        Self {
            pending_cap: pending_cap.max(1),
        }
    }
}

impl Default for InvalidationQueueBounds {
    fn default() -> Self {
        Self {
            pending_cap: DEFAULT_QUEUE_PENDING_CAP,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobFailureDisposition {
    RetryScheduled,
    DeadLettered,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalizationOutcome {
    Fresh,
    Stale {
        successor_job_id: Option<String>,
        current_source_claim_version: i64,
    },
}

impl ActionDb {
    pub fn enqueue_invalidation_job(
        &self,
        input: EnqueueInvalidationJob,
    ) -> Result<InvalidationJobReceipt, DbError> {
        self.enqueue_invalidation_job_with_bounds(input, InvalidationQueueBounds::default())
    }

    pub fn enqueue_invalidation_job_with_pending_cap(
        &self,
        input: EnqueueInvalidationJob,
        pending_cap: i64,
    ) -> Result<InvalidationJobReceipt, DbError> {
        self.enqueue_invalidation_job_with_bounds(
            input,
            InvalidationQueueBounds::with_pending_cap(pending_cap),
        )
    }

    pub fn enqueue_invalidation_job_with_bounds(
        &self,
        input: EnqueueInvalidationJob,
        bounds: InvalidationQueueBounds,
    ) -> Result<InvalidationJobReceipt, DbError> {
        let mut outcome: Option<Result<InvalidationJobReceipt, DbError>> = None;
        let tx_result = self.with_transaction(|tx| {
            let result = tx.enqueue_invalidation_job_inner(input, bounds);
            let tx_result = result
                .as_ref()
                .map(|_| ())
                .map_err(std::string::ToString::to_string);
            outcome = Some(result);
            tx_result
        });

        match tx_result {
            Ok(()) => outcome.unwrap_or_else(|| {
                Err(DbError::InvalidArgument(
                    "invalidation job enqueue did not produce a result".to_string(),
                ))
            }),
            Err(error) => match outcome {
                Some(Err(db_error)) => Err(db_error),
                _ => Err(DbError::Migration(error)),
            },
        }
    }

    pub fn get_invalidation_job(&self, job_id: &str) -> Result<Option<InvalidationJob>, DbError> {
        self.conn_ref()
            .query_row(
                "SELECT
                    id, job_kind, operation, status, priority, chain_id,
                    parent_job_id, successor_of_job_id, origin_signal_id,
                    depth, chain_ancestry_json, idempotency_key, coalescing_key,
                    subject_type, subject_id, ability_id, ability_version,
                    source_claim_version, latest_source_claim_version, source_asof,
                    input_snapshot_hash, provider_fingerprint, prompt_fingerprint,
                    payload_json, first_signal_id, latest_signal_id, raw_signal_count,
                    attempts, max_attempts, lease_owner, lease_expires_at,
                    stale_marker_json
                 FROM invalidation_jobs
                 WHERE id = ?1",
                params![job_id],
                map_invalidation_job,
            )
            .optional()
            .map_err(DbError::Sqlite)
    }

    pub fn claim_next_claim_recompute_job(
        &self,
        worker_id: &str,
        lease_seconds: i64,
    ) -> Result<Option<InvalidationJob>, DbError> {
        let now = Utc::now();
        let now_str = now.to_rfc3339();
        let lease_expires_at = (now + Duration::seconds(lease_seconds)).to_rfc3339();
        let mut outcome: Option<Result<Option<InvalidationJob>, DbError>> = None;
        let tx_result = self.with_transaction(|tx| {
            let result = tx.claim_next_job_inner(
                KIND_CLAIM_RECOMPUTE,
                worker_id,
                &now_str,
                &lease_expires_at,
            );
            let tx_result = result
                .as_ref()
                .map(|_| ())
                .map_err(std::string::ToString::to_string);
            outcome = Some(result);
            tx_result
        });

        match tx_result {
            Ok(()) => outcome.unwrap_or(Ok(None)),
            Err(error) => Err(DbError::Migration(error)),
        }
    }

    pub fn mark_invalidation_job_failed(
        &self,
        job_id: &str,
        error: &str,
    ) -> Result<JobFailureDisposition, DbError> {
        let mut outcome: Option<Result<JobFailureDisposition, DbError>> = None;
        let tx_result = self.with_transaction(|tx| {
            let result = tx.mark_job_failed_inner(job_id, error);
            let tx_result = result
                .as_ref()
                .map(|_| ())
                .map_err(std::string::ToString::to_string);
            outcome = Some(result);
            tx_result
        });

        match tx_result {
            Ok(()) => outcome.unwrap_or_else(|| {
                Err(DbError::InvalidArgument(
                    "invalidation job failure did not produce a result".to_string(),
                ))
            }),
            Err(error) => Err(DbError::Migration(error)),
        }
    }

    pub fn terminalize_claim_recompute_job(
        &self,
        job_id: &str,
    ) -> Result<TerminalizationOutcome, DbError> {
        let mut outcome: Option<Result<TerminalizationOutcome, DbError>> = None;
        let tx_result = self.with_transaction(|tx| {
            let result = tx.terminalize_claim_recompute_job_inner(job_id);
            let tx_result = result
                .as_ref()
                .map(|_| ())
                .map_err(std::string::ToString::to_string);
            outcome = Some(result);
            tx_result
        });

        match tx_result {
            Ok(()) => outcome.unwrap_or_else(|| {
                Err(DbError::InvalidArgument(
                    "claim recompute terminalization did not produce a result".to_string(),
                ))
            }),
            Err(error) => Err(DbError::Migration(error)),
        }
    }

    pub fn list_dead_lettered_invalidation_jobs(
        &self,
        limit: i64,
    ) -> Result<Vec<InvalidationJob>, DbError> {
        let mut stmt = self.conn_ref().prepare(
            "SELECT
                id, job_kind, operation, status, priority, chain_id,
                parent_job_id, successor_of_job_id, origin_signal_id,
                depth, chain_ancestry_json, idempotency_key, coalescing_key,
                subject_type, subject_id, ability_id, ability_version,
                source_claim_version, latest_source_claim_version, source_asof,
                input_snapshot_hash, provider_fingerprint, prompt_fingerprint,
                payload_json, first_signal_id, latest_signal_id, raw_signal_count,
                attempts, max_attempts, lease_owner, lease_expires_at,
                stale_marker_json
             FROM invalidation_jobs
             WHERE status = 'dead_lettered'
             ORDER BY dead_lettered_at DESC, updated_at DESC
             LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], map_invalidation_job)?;
        let mut jobs = Vec::new();
        for row in rows {
            jobs.push(row?);
        }
        Ok(jobs)
    }

    pub fn current_subject_claim_version(
        &self,
        subject_type: &str,
        subject_id: &str,
    ) -> Result<i64, DbError> {
        let normalized = subject_type.to_ascii_lowercase();
        let query = match normalized.as_str() {
            "account" => Some(("accounts", "id")),
            "project" => Some(("projects", "id")),
            "person" => Some(("people", "id")),
            "meeting" => Some(("meetings", "id")),
            "email" => Some(("emails", "email_id")),
            "global" => {
                return self
                    .conn_ref()
                    .query_row(
                        "SELECT value FROM migration_state WHERE key = 'global_claim_epoch'",
                        [],
                        |row| row.get::<_, i64>(0),
                    )
                    .optional()
                    .map(|value| value.unwrap_or(0))
                    .map_err(DbError::Sqlite);
            }
            _ => None,
        };

        let Some((table, id_column)) = query else {
            return Err(DbError::InvalidArgument(format!(
                "unsupported claim recompute subject type: {subject_type}"
            )));
        };
        let sql = format!("SELECT claim_version FROM {table} WHERE {id_column} = ?1");
        self.conn_ref()
            .query_row(&sql, params![subject_id], |row| row.get::<_, i64>(0))
            .optional()
            .map(|value| value.unwrap_or(0))
            .map_err(DbError::Sqlite)
    }

    fn enqueue_invalidation_job_inner(
        &self,
        mut input: EnqueueInvalidationJob,
        bounds: InvalidationQueueBounds,
    ) -> Result<InvalidationJobReceipt, DbError> {
        validate_job_input(&input)?;
        if input.raw_signal_count <= 0 {
            input.raw_signal_count = 1;
        }
        if input.max_attempts <= 0 {
            input.max_attempts = 5;
        }

        if let Some(coalescing_key) = input.coalescing_key.as_deref() {
            if let Some(job) = self.find_pending_coalesced_job(coalescing_key)? {
                self.update_pending_coalesced_job(&job.id, &input)?;
                let updated = self.get_invalidation_job(&job.id)?.ok_or_else(|| {
                    DbError::InvalidArgument("coalesced job disappeared".to_string())
                })?;
                return Ok(receipt_for_job(&updated, true));
            }

            if let Some(running) = self.find_running_coalesced_job(coalescing_key)? {
                input.chain_id = Some(running.chain_id.clone());
                input.parent_job_id = Some(running.id.clone());
                input.successor_of_job_id = Some(running.id.clone());
                input.depth = running.depth;
                input.chain_ancestry = parse_ancestry(&running.chain_ancestry_json)?;
                return self.insert_new_job(input, true, bounds);
            }
        } else if let Some(existing) =
            self.find_active_idempotency_job(&derive_idempotency_key(&input))?
        {
            return Ok(receipt_for_job(&existing, true));
        }

        self.insert_new_job(input, false, bounds)
    }

    fn insert_new_job(
        &self,
        input: EnqueueInvalidationJob,
        coalesced: bool,
        bounds: InvalidationQueueBounds,
    ) -> Result<InvalidationJobReceipt, DbError> {
        let pending_depth = self.pending_invalidation_job_depth()?;
        if pending_depth >= bounds.pending_cap {
            if let Some(job) = self.find_pending_job_for_aggressive_coalesce(&input)? {
                self.update_pending_coalesced_job(&job.id, &input)?;
                let updated = self.get_invalidation_job(&job.id)?.ok_or_else(|| {
                    DbError::InvalidArgument("aggressively coalesced job disappeared".to_string())
                })?;
                log::warn!(
                    "invalidation queue pending cap {} reached; aggressively coalesced {} into {}",
                    bounds.pending_cap,
                    input.subject_id,
                    updated.id
                );
                return Ok(receipt_for_job(&updated, true));
            }

            return Err(DbError::InvalidArgument(format!(
                "invalidation queue pending cap {} reached; enqueue rejected",
                bounds.pending_cap
            )));
        }

        let now = Utc::now().to_rfc3339();
        let id = format!("job-{}", uuid::Uuid::new_v4());
        let chain_id = input
            .chain_id
            .clone()
            .unwrap_or_else(|| format!("chain-{}", uuid::Uuid::new_v4()));
        let idempotency_key = derive_idempotency_key(&input);
        let output_id = invalidation_output_id(&input);
        let cycle_detected = input.depth > DEFAULT_CHAIN_DEPTH_CAP
            || input
                .chain_ancestry
                .iter()
                .any(|ancestor| ancestor == &output_id);
        let status = if cycle_detected {
            STATUS_CYCLE_DETECTED
        } else {
            STATUS_PENDING
        };
        let stale_marker_json = if cycle_detected {
            Some(
                json!({
                    "reason": "cycle_detected",
                    "output_id": output_id,
                    "chain_id": chain_id,
                })
                .to_string(),
            )
        } else {
            None
        };
        let payload_json = serde_json::to_string(&input.payload_json)
            .map_err(|e| DbError::InvalidArgument(format!("invalid payload_json: {e}")))?;
        let ancestry_json = serde_json::to_string(&input.chain_ancestry)
            .map_err(|e| DbError::InvalidArgument(format!("invalid chain ancestry: {e}")))?;
        let first_signal_id = input.origin_signal_id.clone();
        let latest_signal_id = input.origin_signal_id.clone();

        self.conn_ref().execute(
            "INSERT INTO invalidation_jobs (
                id, job_kind, operation, status, priority, chain_id,
                parent_job_id, successor_of_job_id, origin_signal_id,
                depth, chain_ancestry_json, idempotency_key, coalescing_key,
                subject_type, subject_id, ability_id, ability_version,
                source_claim_version, latest_source_claim_version, source_asof,
                input_snapshot_hash, provider_fingerprint, prompt_fingerprint,
                payload_json, first_signal_id, latest_signal_id, raw_signal_count,
                covered_since_at, covered_until_at, attempts, max_attempts,
                next_run_at, stale_marker_json, completed_at, created_at, updated_at
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6,
                ?7, ?8, ?9,
                ?10, ?11, ?12, ?13,
                ?14, ?15, ?16, ?17,
                ?18, ?19, ?20,
                ?21, ?22, ?23,
                ?24, ?25, ?26, ?27,
                ?28, ?28, 0, ?29,
                ?28, ?30, ?31, ?28, ?28
             )",
            params![
                &id,
                &input.job_kind,
                &input.operation,
                status,
                input.priority,
                &chain_id,
                input.parent_job_id.as_deref(),
                input.successor_of_job_id.as_deref(),
                input.origin_signal_id.as_deref(),
                input.depth,
                &ancestry_json,
                &idempotency_key,
                input.coalescing_key.as_deref(),
                &input.subject_type,
                &input.subject_id,
                &input.ability_id,
                &input.ability_version,
                input.source_claim_version,
                input.source_claim_version,
                input.source_asof.as_deref(),
                input.input_snapshot_hash.as_deref(),
                input.provider_fingerprint.as_deref(),
                input.prompt_fingerprint.as_deref(),
                &payload_json,
                first_signal_id.as_deref(),
                latest_signal_id.as_deref(),
                input.raw_signal_count,
                &now,
                input.max_attempts,
                stale_marker_json.as_deref(),
                if cycle_detected {
                    Some(now.as_str())
                } else {
                    None
                },
            ],
        )?;

        Ok(InvalidationJobReceipt {
            job_id: id,
            chain_id,
            status: status.to_string(),
            coalesced,
            successor_of_job_id: input.successor_of_job_id,
        })
    }

    fn update_pending_coalesced_job(
        &self,
        job_id: &str,
        input: &EnqueueInvalidationJob,
    ) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        let payload_json = serde_json::to_string(&input.payload_json)
            .map_err(|e| DbError::InvalidArgument(format!("invalid payload_json: {e}")))?;
        let next_idempotency_key = derive_idempotency_key(input);
        self.conn_ref().execute(
            "UPDATE invalidation_jobs
             SET latest_signal_id = COALESCE(?2, latest_signal_id),
                 raw_signal_count = raw_signal_count + ?3,
                 latest_source_claim_version =
                    CASE
                        WHEN ?4 > latest_source_claim_version THEN ?4
                        ELSE latest_source_claim_version
                    END,
                 idempotency_key =
                    CASE
                        WHEN ?4 > latest_source_claim_version THEN ?5
                        ELSE idempotency_key
                    END,
                 source_asof = COALESCE(?6, source_asof),
                 input_snapshot_hash = COALESCE(?7, input_snapshot_hash),
                 provider_fingerprint = COALESCE(?8, provider_fingerprint),
                 prompt_fingerprint = COALESCE(?9, prompt_fingerprint),
                 payload_json = ?10,
                 covered_until_at = ?11,
                 updated_at = ?11
             WHERE id = ?1
               AND status = 'pending'",
            params![
                job_id,
                input.origin_signal_id.as_deref(),
                input.raw_signal_count.max(1),
                input.source_claim_version,
                &next_idempotency_key,
                input.source_asof.as_deref(),
                input.input_snapshot_hash.as_deref(),
                input.provider_fingerprint.as_deref(),
                input.prompt_fingerprint.as_deref(),
                &payload_json,
                &now,
            ],
        )?;
        Ok(())
    }

    fn claim_next_job_inner(
        &self,
        job_kind: &str,
        worker_id: &str,
        now: &str,
        lease_expires_at: &str,
    ) -> Result<Option<InvalidationJob>, DbError> {
        let job_id: Option<String> = self
            .conn_ref()
            .query_row(
                "SELECT id
                 FROM invalidation_jobs
                 WHERE job_kind = ?1
                   AND (
                        (status = 'pending' AND datetime(next_run_at) <= datetime(?2))
                        OR
                        (status = 'running'
                         AND lease_expires_at IS NOT NULL
                         AND datetime(lease_expires_at) <= datetime(?2))
                   )
                 ORDER BY
                    CASE status WHEN 'running' THEN 0 ELSE 1 END,
                    priority DESC,
                    created_at ASC
                 LIMIT 1",
                params![job_kind, now],
                |row| row.get(0),
            )
            .optional()?;

        let Some(job_id) = job_id else {
            return Ok(None);
        };

        self.conn_ref().execute(
            "UPDATE invalidation_jobs
             SET status = 'running',
                 lease_owner = ?2,
                 lease_expires_at = ?3,
                 claimed_at = ?4,
                 attempts = attempts + 1,
                 updated_at = ?4
             WHERE id = ?1
               AND status IN ('pending', 'running')",
            params![&job_id, worker_id, lease_expires_at, now],
        )?;

        self.get_invalidation_job(&job_id)
    }

    fn mark_job_failed_inner(
        &self,
        job_id: &str,
        error: &str,
    ) -> Result<JobFailureDisposition, DbError> {
        let (attempts, max_attempts): (i64, i64) = self.conn_ref().query_row(
            "SELECT attempts, max_attempts FROM invalidation_jobs WHERE id = ?1",
            params![job_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let now = Utc::now();
        if attempts >= max_attempts {
            let now_str = now.to_rfc3339();
            let stale_marker = json!({
                "reason": "dead_lettered",
                "error": error,
                "job_id": job_id,
            })
            .to_string();
            self.conn_ref().execute(
                "UPDATE invalidation_jobs
                 SET status = 'dead_lettered',
                     lease_owner = NULL,
                     lease_expires_at = NULL,
                     dead_lettered_at = ?2,
                     last_error = ?3,
                     stale_marker_json = ?4,
                     updated_at = ?2
                 WHERE id = ?1",
                params![job_id, &now_str, error, &stale_marker],
            )?;
            Ok(JobFailureDisposition::DeadLettered)
        } else {
            let next = now + Duration::seconds(retry_backoff_seconds(attempts));
            let next_str = next.to_rfc3339();
            let now_str = now.to_rfc3339();
            self.conn_ref().execute(
                "UPDATE invalidation_jobs
                 SET status = 'pending',
                     lease_owner = NULL,
                     lease_expires_at = NULL,
                     next_run_at = ?2,
                     last_error = ?3,
                     updated_at = ?4
                 WHERE id = ?1",
                params![job_id, &next_str, error, &now_str],
            )?;
            Ok(JobFailureDisposition::RetryScheduled)
        }
    }

    fn terminalize_claim_recompute_job_inner(
        &self,
        job_id: &str,
    ) -> Result<TerminalizationOutcome, DbError> {
        let job = self
            .get_invalidation_job(job_id)?
            .ok_or_else(|| DbError::InvalidArgument(format!("job {job_id} not found")))?;
        if job.status != STATUS_RUNNING {
            return Err(DbError::InvalidArgument(format!(
                "job {job_id} is not running"
            )));
        }

        let current_source_claim_version =
            self.current_subject_claim_version(&job.subject_type, &job.subject_id)?;
        let successor = if let Some(coalescing_key) = job.coalescing_key.as_deref() {
            self.find_active_successor_covering(
                coalescing_key,
                &job.id,
                current_source_claim_version,
                job.latest_source_claim_version,
            )?
        } else {
            None
        };

        let mut successor_job_id = successor.map(|job| job.id);
        if current_source_claim_version > job.latest_source_claim_version
            && successor_job_id.is_none()
        {
            let successor_input =
                successor_input_for_stale_terminalization(&job, current_source_claim_version)?;
            let receipt = self.enqueue_invalidation_job_inner(
                successor_input,
                InvalidationQueueBounds::default(),
            )?;
            successor_job_id = Some(receipt.job_id);
        }

        let stale = current_source_claim_version > job.latest_source_claim_version
            || successor_job_id.is_some();
        let now = Utc::now().to_rfc3339();
        let stale_marker = if stale {
            Some(
                json!({
                    "reason": "successor_covers_newer_watermark",
                    "job_id": job.id,
                    "successor_job_id": successor_job_id,
                    "job_source_claim_version": job.latest_source_claim_version,
                    "current_source_claim_version": current_source_claim_version,
                })
                .to_string(),
            )
        } else {
            None
        };

        self.conn_ref().execute(
            "UPDATE invalidation_jobs
             SET status = 'completed',
                 completed_at = ?2,
                 lease_owner = NULL,
                 lease_expires_at = NULL,
                 stale_marker_json = ?3,
                 updated_at = ?2
             WHERE id = ?1",
            params![job_id, &now, stale_marker.as_deref()],
        )?;

        if stale {
            Ok(TerminalizationOutcome::Stale {
                successor_job_id,
                current_source_claim_version,
            })
        } else {
            Ok(TerminalizationOutcome::Fresh)
        }
    }

    fn pending_invalidation_job_depth(&self) -> Result<i64, DbError> {
        self.conn_ref()
            .query_row(
                "SELECT COUNT(*)
                 FROM invalidation_jobs
                 WHERE status = 'pending'",
                [],
                |row| row.get(0),
            )
            .map_err(DbError::Sqlite)
    }

    fn find_pending_job_for_aggressive_coalesce(
        &self,
        input: &EnqueueInvalidationJob,
    ) -> Result<Option<InvalidationJob>, DbError> {
        if input.job_kind == KIND_TARGETED_REPAIR {
            let Some(coalescing_key) = input.coalescing_key.as_deref() else {
                return Ok(None);
            };

            return self
                .conn_ref()
                .query_row(
                    "SELECT
                    id, job_kind, operation, status, priority, chain_id,
                    parent_job_id, successor_of_job_id, origin_signal_id,
                    depth, chain_ancestry_json, idempotency_key, coalescing_key,
                    subject_type, subject_id, ability_id, ability_version,
                    source_claim_version, latest_source_claim_version, source_asof,
                    input_snapshot_hash, provider_fingerprint, prompt_fingerprint,
                    payload_json, first_signal_id, latest_signal_id, raw_signal_count,
                    attempts, max_attempts, lease_owner, lease_expires_at,
                    stale_marker_json
                 FROM invalidation_jobs
                 WHERE status = 'pending'
                   AND job_kind = ?1
                   AND operation = ?2
                   AND subject_type = ?3
                   AND subject_id = ?4
                   AND ability_id = ?5
                   AND ability_version = ?6
                   AND coalescing_key = ?7
                 ORDER BY priority DESC, updated_at DESC, created_at DESC
                 LIMIT 1",
                    params![
                        &input.job_kind,
                        &input.operation,
                        &input.subject_type,
                        &input.subject_id,
                        &input.ability_id,
                        &input.ability_version,
                        coalescing_key,
                    ],
                    map_invalidation_job,
                )
                .optional()
                .map_err(DbError::Sqlite);
        }

        self.conn_ref()
            .query_row(
                "SELECT
                    id, job_kind, operation, status, priority, chain_id,
                    parent_job_id, successor_of_job_id, origin_signal_id,
                    depth, chain_ancestry_json, idempotency_key, coalescing_key,
                    subject_type, subject_id, ability_id, ability_version,
                    source_claim_version, latest_source_claim_version, source_asof,
                    input_snapshot_hash, provider_fingerprint, prompt_fingerprint,
                    payload_json, first_signal_id, latest_signal_id, raw_signal_count,
                    attempts, max_attempts, lease_owner, lease_expires_at,
                    stale_marker_json
                 FROM invalidation_jobs
                 WHERE status = 'pending'
                   AND job_kind = ?1
                   AND operation = ?2
                   AND subject_type = ?3
                   AND subject_id = ?4
                   AND ability_id = ?5
                   AND ability_version = ?6
                 ORDER BY priority DESC, updated_at DESC, created_at DESC
                 LIMIT 1",
                params![
                    &input.job_kind,
                    &input.operation,
                    &input.subject_type,
                    &input.subject_id,
                    &input.ability_id,
                    &input.ability_version,
                ],
                map_invalidation_job,
            )
            .optional()
            .map_err(DbError::Sqlite)
    }

    fn find_pending_coalesced_job(
        &self,
        coalescing_key: &str,
    ) -> Result<Option<InvalidationJob>, DbError> {
        self.find_job_by_sql(
            "SELECT
                id, job_kind, operation, status, priority, chain_id,
                parent_job_id, successor_of_job_id, origin_signal_id,
                depth, chain_ancestry_json, idempotency_key, coalescing_key,
                subject_type, subject_id, ability_id, ability_version,
                source_claim_version, latest_source_claim_version, source_asof,
                input_snapshot_hash, provider_fingerprint, prompt_fingerprint,
                payload_json, first_signal_id, latest_signal_id, raw_signal_count,
                attempts, max_attempts, lease_owner, lease_expires_at,
                stale_marker_json
             FROM invalidation_jobs
             WHERE coalescing_key = ?1
               AND status = 'pending'
             ORDER BY
               CASE WHEN successor_of_job_id IS NOT NULL THEN 0 ELSE 1 END,
               created_at DESC
             LIMIT 1",
            coalescing_key,
        )
    }

    fn find_running_coalesced_job(
        &self,
        coalescing_key: &str,
    ) -> Result<Option<InvalidationJob>, DbError> {
        self.find_job_by_sql(
            "SELECT
                id, job_kind, operation, status, priority, chain_id,
                parent_job_id, successor_of_job_id, origin_signal_id,
                depth, chain_ancestry_json, idempotency_key, coalescing_key,
                subject_type, subject_id, ability_id, ability_version,
                source_claim_version, latest_source_claim_version, source_asof,
                input_snapshot_hash, provider_fingerprint, prompt_fingerprint,
                payload_json, first_signal_id, latest_signal_id, raw_signal_count,
                attempts, max_attempts, lease_owner, lease_expires_at,
                stale_marker_json
             FROM invalidation_jobs
             WHERE coalescing_key = ?1
               AND status = 'running'
             ORDER BY claimed_at DESC, created_at DESC
             LIMIT 1",
            coalescing_key,
        )
    }

    fn find_active_idempotency_job(
        &self,
        idempotency_key: &str,
    ) -> Result<Option<InvalidationJob>, DbError> {
        self.find_job_by_sql(
            "SELECT
                id, job_kind, operation, status, priority, chain_id,
                parent_job_id, successor_of_job_id, origin_signal_id,
                depth, chain_ancestry_json, idempotency_key, coalescing_key,
                subject_type, subject_id, ability_id, ability_version,
                source_claim_version, latest_source_claim_version, source_asof,
                input_snapshot_hash, provider_fingerprint, prompt_fingerprint,
                payload_json, first_signal_id, latest_signal_id, raw_signal_count,
                attempts, max_attempts, lease_owner, lease_expires_at,
                stale_marker_json
             FROM invalidation_jobs
             WHERE idempotency_key = ?1
               AND status IN ('pending', 'running')
               AND successor_of_job_id IS NULL
             ORDER BY created_at ASC
             LIMIT 1",
            idempotency_key,
        )
    }

    fn find_active_successor_covering(
        &self,
        coalescing_key: &str,
        job_id: &str,
        current_source_claim_version: i64,
        job_source_claim_version: i64,
    ) -> Result<Option<InvalidationJob>, DbError> {
        self.conn_ref()
            .query_row(
                "SELECT
                    id, job_kind, operation, status, priority, chain_id,
                    parent_job_id, successor_of_job_id, origin_signal_id,
                    depth, chain_ancestry_json, idempotency_key, coalescing_key,
                    subject_type, subject_id, ability_id, ability_version,
                    source_claim_version, latest_source_claim_version, source_asof,
                    input_snapshot_hash, provider_fingerprint, prompt_fingerprint,
                    payload_json, first_signal_id, latest_signal_id, raw_signal_count,
                    attempts, max_attempts, lease_owner, lease_expires_at,
                    stale_marker_json
                 FROM invalidation_jobs
                 WHERE coalescing_key = ?1
                   AND id != ?2
                   AND status IN ('pending', 'running')
                   AND (
                        successor_of_job_id = ?2
                        OR latest_source_claim_version > ?4
                        OR latest_source_claim_version >= ?3
                   )
                 ORDER BY latest_source_claim_version DESC, created_at DESC
                 LIMIT 1",
                params![
                    coalescing_key,
                    job_id,
                    current_source_claim_version,
                    job_source_claim_version
                ],
                map_invalidation_job,
            )
            .optional()
            .map_err(DbError::Sqlite)
    }

    fn find_job_by_sql(&self, sql: &str, key: &str) -> Result<Option<InvalidationJob>, DbError> {
        self.conn_ref()
            .query_row(sql, params![key], map_invalidation_job)
            .optional()
            .map_err(DbError::Sqlite)
    }
}

pub fn claim_recompute_input_hash(
    subject_type: &str,
    subject_id: &str,
    source_claim_version: i64,
) -> String {
    format!("{subject_type}:{subject_id}:claims:{source_claim_version}")
}

pub fn claim_recompute_coalescing_key(
    subject_type: &str,
    subject_id: &str,
    input_snapshot_hash: &str,
) -> String {
    let input_scope = input_snapshot_hash
        .rsplit_once(':')
        .map(|(scope, _)| scope)
        .unwrap_or(input_snapshot_hash);
    format!(
        "claim_recompute:{subject_type}:{subject_id}:{}:{}:{input_scope}",
        CLAIM_RECOMPUTE_ABILITY_ID, CLAIM_RECOMPUTE_ABILITY_VERSION
    )
}

fn validate_job_input(input: &EnqueueInvalidationJob) -> Result<(), DbError> {
    let valid_kind = matches!(
        input.job_kind.as_str(),
        KIND_SIGNAL_INVALIDATION
            | KIND_CLAIM_RECOMPUTE
            | KIND_TARGETED_REPAIR
            | KIND_TRANSFORM
            | KIND_MAINTENANCE_APPLY
            | KIND_OUTBOX_REPLAY
    );
    if !valid_kind {
        return Err(DbError::InvalidArgument(format!(
            "unsupported invalidation job kind: {}",
            input.job_kind
        )));
    }
    if input.operation.trim().is_empty()
        || input.subject_type.trim().is_empty()
        || input.subject_id.trim().is_empty()
        || input.ability_id.trim().is_empty()
        || input.ability_version.trim().is_empty()
    {
        return Err(DbError::InvalidArgument(
            "invalidation job identity fields cannot be empty".to_string(),
        ));
    }
    if input.source_claim_version < 0 {
        return Err(DbError::InvalidArgument(
            "source claim version cannot be negative".to_string(),
        ));
    }
    Ok(())
}

fn derive_idempotency_key(input: &EnqueueInvalidationJob) -> String {
    format!(
        "{}|{}|{}:{}|{}:{}|source_claim_version={}|source_asof={}|input={}|provider={}|prompt={}",
        input.job_kind,
        input.operation,
        input.subject_type,
        input.subject_id,
        input.ability_id,
        input.ability_version,
        input.source_claim_version,
        input.source_asof.as_deref().unwrap_or(""),
        input.input_snapshot_hash.as_deref().unwrap_or(""),
        input.provider_fingerprint.as_deref().unwrap_or(""),
        input.prompt_fingerprint.as_deref().unwrap_or("")
    )
}

fn invalidation_output_id(input: &EnqueueInvalidationJob) -> String {
    format!(
        "{}:{}:{}:{}:{}:{}",
        input.job_kind,
        input.operation,
        input.subject_type,
        input.subject_id,
        input.ability_id,
        input.ability_version
    )
}

fn parse_ancestry(json: &str) -> Result<Vec<String>, DbError> {
    serde_json::from_str::<Vec<String>>(json)
        .map_err(|e| DbError::InvalidArgument(format!("invalid chain ancestry: {e}")))
}

fn successor_input_for_stale_terminalization(
    job: &InvalidationJob,
    current_source_claim_version: i64,
) -> Result<EnqueueInvalidationJob, DbError> {
    let ancestry = parse_ancestry(&job.chain_ancestry_json)?;
    let payload_json =
        serde_json::from_str::<serde_json::Value>(&job.payload_json).unwrap_or_else(|_| json!({}));
    Ok(EnqueueInvalidationJob {
        job_kind: job.job_kind.clone(),
        operation: job.operation.clone(),
        origin_signal_id: job.origin_signal_id.clone(),
        subject_type: job.subject_type.clone(),
        subject_id: job.subject_id.clone(),
        ability_id: job.ability_id.clone(),
        ability_version: job.ability_version.clone(),
        source_claim_version: current_source_claim_version,
        source_asof: job.source_asof.clone(),
        input_snapshot_hash: Some(claim_recompute_input_hash(
            &job.subject_type,
            &job.subject_id,
            current_source_claim_version,
        )),
        provider_fingerprint: job.provider_fingerprint.clone(),
        prompt_fingerprint: job.prompt_fingerprint.clone(),
        payload_json,
        coalescing_key: job.coalescing_key.clone(),
        chain_id: Some(job.chain_id.clone()),
        parent_job_id: Some(job.id.clone()),
        successor_of_job_id: Some(job.id.clone()),
        depth: job.depth,
        chain_ancestry: ancestry,
        max_attempts: job.max_attempts,
        priority: job.priority,
        raw_signal_count: 1,
    })
}

fn retry_backoff_seconds(attempts: i64) -> i64 {
    let exponent = attempts.saturating_sub(1).min(4) as u32;
    4_i64.pow(exponent)
}

fn receipt_for_job(job: &InvalidationJob, coalesced: bool) -> InvalidationJobReceipt {
    InvalidationJobReceipt {
        job_id: job.id.clone(),
        chain_id: job.chain_id.clone(),
        status: job.status.clone(),
        coalesced,
        successor_of_job_id: job.successor_of_job_id.clone(),
    }
}

fn map_invalidation_job(row: &rusqlite::Row<'_>) -> rusqlite::Result<InvalidationJob> {
    Ok(InvalidationJob {
        id: row.get(0)?,
        job_kind: row.get(1)?,
        operation: row.get(2)?,
        status: row.get(3)?,
        priority: row.get(4)?,
        chain_id: row.get(5)?,
        parent_job_id: row.get(6)?,
        successor_of_job_id: row.get(7)?,
        origin_signal_id: row.get(8)?,
        depth: row.get(9)?,
        chain_ancestry_json: row.get(10)?,
        idempotency_key: row.get(11)?,
        coalescing_key: row.get(12)?,
        subject_type: row.get(13)?,
        subject_id: row.get(14)?,
        ability_id: row.get(15)?,
        ability_version: row.get(16)?,
        source_claim_version: row.get(17)?,
        latest_source_claim_version: row.get(18)?,
        source_asof: row.get(19)?,
        input_snapshot_hash: row.get(20)?,
        provider_fingerprint: row.get(21)?,
        prompt_fingerprint: row.get(22)?,
        payload_json: row.get(23)?,
        first_signal_id: row.get(24)?,
        latest_signal_id: row.get(25)?,
        raw_signal_count: row.get(26)?,
        attempts: row.get(27)?,
        max_attempts: row.get(28)?,
        lease_owner: row.get(29)?,
        lease_expires_at: row.get(30)?,
        stale_marker_json: row.get(31)?,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use parking_lot::Mutex;
    use tempfile::tempdir;

    use super::*;
    use crate::db::key_provider::{rekey_database, DbKeyProvider, EncryptionKey, UserIdentity};
    use crate::db::test_utils::test_db;

    #[derive(Debug)]
    struct RotatingFixtureKeyProvider {
        current: Mutex<EncryptionKey>,
        next: EncryptionKey,
    }

    impl RotatingFixtureKeyProvider {
        fn new(current: &str, next: &str) -> Self {
            Self {
                current: Mutex::new(EncryptionKey::from_hex(current.to_string())),
                next: EncryptionKey::from_hex(next.to_string()),
            }
        }
    }

    impl DbKeyProvider for RotatingFixtureKeyProvider {
        fn get_or_create_key(
            &self,
            _user: &UserIdentity,
        ) -> crate::db::key_provider::Result<EncryptionKey> {
            Ok(self.current.lock().clone())
        }

        fn rotate_key(
            &self,
            user: &UserIdentity,
        ) -> crate::db::key_provider::Result<EncryptionKey> {
            let mut current = self.current.lock();
            rekey_database(user.db_path(), &current, &self.next)?;
            *current = self.next.clone();
            Ok(current.clone())
        }
    }

    fn seed_account(db: &ActionDb, account_id: &str, claim_version: i64) {
        db.conn_ref()
            .execute(
                "INSERT INTO accounts (id, name, updated_at, claim_version)
                 VALUES (?1, ?2, '2026-05-08T00:00:00Z', ?3)",
                params![account_id, format!("Account {account_id}"), claim_version],
            )
            .expect("seed account");
    }

    fn claim_input(signal_id: &str, account_id: &str, version: i64) -> EnqueueInvalidationJob {
        EnqueueInvalidationJob::claim_recompute_from_signal(
            signal_id, "account", account_id, version,
        )
    }

    fn targeted_policy_repair_input(
        signal_id: &str,
        claim_id: &str,
        surface: &str,
        version: i64,
    ) -> EnqueueInvalidationJob {
        let subject_type = "account";
        let subject_id = "acct-policy";
        let ability_id = "targeted_claim_repair";
        let ability_version = "1";
        let input_scope = format!("targeted_repair:{subject_type}:{subject_id}:{claim_id}:claims");
        let input_snapshot_hash = format!("{input_scope}:{version}");
        let coalescing_key = format!(
            "targeted_claim_repair:{subject_type}:{subject_id}:{claim_id}:PolicyRepair:surface:{surface}:{ability_id}:{ability_version}:{input_scope}"
        );

        EnqueueInvalidationJob {
            job_kind: KIND_TARGETED_REPAIR.to_string(),
            operation: TARGETED_REPAIR_POLICY_REPAIR_OPERATION.to_string(),
            origin_signal_id: Some(signal_id.to_string()),
            subject_type: subject_type.to_string(),
            subject_id: subject_id.to_string(),
            ability_id: ability_id.to_string(),
            ability_version: ability_version.to_string(),
            source_claim_version: version,
            source_asof: None,
            input_snapshot_hash: Some(input_snapshot_hash),
            provider_fingerprint: Some("provider:test".to_string()),
            prompt_fingerprint: Some("prompt:test".to_string()),
            payload_json: serde_json::json!({
                "claim_id": claim_id,
                "feedback_id": format!("feedback-{surface}"),
                "repair_action": "PolicyRepair",
            }),
            coalescing_key: Some(coalescing_key),
            chain_id: None,
            parent_job_id: None,
            successor_of_job_id: None,
            depth: 0,
            chain_ancestry: Vec::new(),
            max_attempts: 3,
            priority: 0,
            raw_signal_count: 1,
        }
    }

    fn targeted_bounded_corroboration_input(
        signal_id: &str,
        subject_id: &str,
        claim_id: &str,
        feedback_id: &str,
        version: i64,
    ) -> EnqueueInvalidationJob {
        let subject_type = "account";
        let ability_id = "targeted_claim_repair";
        let ability_version = "1";
        let repair_action = "BoundedCorroboration";
        let operation = format!("targeted_claim_repair:{repair_action}");
        let input_scope = format!("targeted_repair:{subject_type}:{subject_id}:{claim_id}:claims");
        let input_snapshot_hash = format!("{input_scope}:{version}");
        let coalescing_key = format!(
            "targeted_claim_repair:{subject_type}:{subject_id}:{claim_id}:{repair_action}:claim:{ability_id}:{ability_version}:{input_scope}"
        );

        EnqueueInvalidationJob {
            job_kind: KIND_TARGETED_REPAIR.to_string(),
            operation,
            origin_signal_id: Some(signal_id.to_string()),
            subject_type: subject_type.to_string(),
            subject_id: subject_id.to_string(),
            ability_id: ability_id.to_string(),
            ability_version: ability_version.to_string(),
            source_claim_version: version,
            source_asof: None,
            input_snapshot_hash: Some(input_snapshot_hash),
            provider_fingerprint: Some("provider:test".to_string()),
            prompt_fingerprint: Some("prompt:test".to_string()),
            payload_json: serde_json::json!({
                "claim_id": claim_id,
                "feedback_id": feedback_id,
                "repair_action": repair_action,
            }),
            coalescing_key: Some(coalescing_key),
            chain_id: None,
            parent_job_id: None,
            successor_of_job_id: None,
            depth: 0,
            chain_ancestry: Vec::new(),
            max_attempts: 3,
            priority: 0,
            raw_signal_count: 1,
        }
    }

    #[test]
    fn coalescing_mutates_pending_but_creates_running_successor() {
        let db = test_db();
        seed_account(&db, "acct-queue", 1);

        let first = db
            .enqueue_invalidation_job(claim_input("sig-1", "acct-queue", 1))
            .expect("enqueue first");
        let second = db
            .enqueue_invalidation_job(claim_input("sig-2", "acct-queue", 2))
            .expect("enqueue second");
        assert_eq!(second.job_id, first.job_id);
        assert!(second.coalesced);

        let pending = db
            .get_invalidation_job(&first.job_id)
            .expect("read pending")
            .expect("pending job");
        assert_eq!(pending.status, STATUS_PENDING);
        assert_eq!(pending.latest_source_claim_version, 2);
        assert_eq!(pending.latest_signal_id.as_deref(), Some("sig-2"));
        assert_eq!(pending.raw_signal_count, 2);

        let running = db
            .claim_next_claim_recompute_job("worker-a", 30)
            .expect("claim")
            .expect("claimed job");
        assert_eq!(running.id, first.job_id);
        assert_eq!(running.status, STATUS_RUNNING);

        let third = db
            .enqueue_invalidation_job(claim_input("sig-3", "acct-queue", 3))
            .expect("enqueue successor");
        assert_ne!(third.job_id, running.id);
        assert_eq!(third.chain_id, running.chain_id);
        assert_eq!(
            third.successor_of_job_id.as_deref(),
            Some(running.id.as_str())
        );

        let running_after = db
            .get_invalidation_job(&running.id)
            .expect("read running")
            .expect("running job");
        assert_eq!(running_after.latest_source_claim_version, 2);

        let fourth = db
            .enqueue_invalidation_job(claim_input("sig-4", "acct-queue", 4))
            .expect("coalesce successor");
        assert_eq!(fourth.job_id, third.job_id);
        let successor = db
            .get_invalidation_job(&third.job_id)
            .expect("read successor")
            .expect("successor job");
        assert_eq!(successor.latest_source_claim_version, 4);
        assert_eq!(successor.raw_signal_count, 2);
        assert_eq!(successor.latest_signal_id.as_deref(), Some("sig-4"));
    }

    #[test]
    fn terminalization_proves_freshness_and_leaves_successor_for_new_watermark() {
        let db = test_db();
        seed_account(&db, "acct-fresh", 1);
        let receipt = db
            .enqueue_invalidation_job(claim_input("sig-1", "acct-fresh", 1))
            .expect("enqueue");
        let claimed = db
            .claim_next_claim_recompute_job("worker-a", 30)
            .expect("claim")
            .expect("claimed job");
        assert_eq!(claimed.id, receipt.job_id);

        db.conn_ref()
            .execute(
                "UPDATE accounts SET claim_version = 2 WHERE id = 'acct-fresh'",
                [],
            )
            .expect("advance claim version");

        let outcome = db
            .terminalize_claim_recompute_job(&claimed.id)
            .expect("terminalize");
        let TerminalizationOutcome::Stale {
            successor_job_id: Some(successor_id),
            current_source_claim_version,
        } = outcome
        else {
            panic!("expected stale terminalization with successor");
        };
        assert_eq!(current_source_claim_version, 2);

        let completed = db
            .get_invalidation_job(&claimed.id)
            .expect("read completed")
            .expect("completed job");
        assert_eq!(completed.status, STATUS_COMPLETED);
        assert!(completed.stale_marker_json.is_some());

        let successor = db
            .get_invalidation_job(&successor_id)
            .expect("read successor")
            .expect("successor job");
        assert_eq!(successor.status, STATUS_PENDING);
        assert_eq!(successor.latest_source_claim_version, 2);
        assert_eq!(successor.chain_id, completed.chain_id);
    }

    #[test]
    fn retry_exhaustion_is_visible_as_dead_letter() {
        let db = test_db();
        seed_account(&db, "acct-dead", 1);
        let mut input = claim_input("sig-1", "acct-dead", 1);
        input.max_attempts = 1;
        let receipt = db.enqueue_invalidation_job(input).expect("enqueue");
        let claimed = db
            .claim_next_claim_recompute_job("worker-a", 30)
            .expect("claim")
            .expect("claimed job");
        assert_eq!(claimed.id, receipt.job_id);

        let disposition = db
            .mark_invalidation_job_failed(&claimed.id, "permanent failure")
            .expect("mark failed");
        assert_eq!(disposition, JobFailureDisposition::DeadLettered);

        let dead = db
            .list_dead_lettered_invalidation_jobs(10)
            .expect("list dead letters");
        assert_eq!(dead.len(), 1);
        assert_eq!(dead[0].id, receipt.job_id);
        assert!(dead[0].stale_marker_json.is_some());
    }

    #[test]
    fn cycle_detection_creates_terminal_visible_job() {
        let db = test_db();
        let mut input = claim_input("sig-1", "acct-cycle", 1);
        let output_id = invalidation_output_id(&input);
        input.chain_ancestry.push(output_id);
        let receipt = db.enqueue_invalidation_job(input).expect("enqueue");
        assert_eq!(receipt.status, STATUS_CYCLE_DETECTED);
        let job = db
            .get_invalidation_job(&receipt.job_id)
            .expect("read job")
            .expect("cycle job");
        assert_eq!(job.status, STATUS_CYCLE_DETECTED);
        assert!(job.stale_marker_json.is_some());
    }

    #[test]
    fn pending_cap_aggressively_coalesces_matching_output() {
        let db = test_db();
        seed_account(&db, "acct-cap-coalesce", 1);
        let mut first = claim_input("sig-1", "acct-cap-coalesce", 1);
        first.coalescing_key = Some("first-window".to_string());
        let first_receipt = db
            .enqueue_invalidation_job_with_pending_cap(first, 1)
            .expect("enqueue first");

        let mut second = claim_input("sig-2", "acct-cap-coalesce", 2);
        second.coalescing_key = Some("second-window".to_string());
        let second_receipt = db
            .enqueue_invalidation_job_with_pending_cap(second, 1)
            .expect("aggressively coalesce second");

        assert_eq!(second_receipt.job_id, first_receipt.job_id);
        assert!(second_receipt.coalesced);

        let job = db
            .get_invalidation_job(&first_receipt.job_id)
            .expect("read job")
            .expect("job");
        assert_eq!(job.latest_source_claim_version, 2);
        assert_eq!(job.latest_signal_id.as_deref(), Some("sig-2"));
        assert_eq!(job.raw_signal_count, 2);
    }

    #[test]
    fn pending_cap_rejects_policy_repair_with_distinct_surface_key() {
        let db = test_db();
        seed_account(&db, "acct-policy", 1);
        let first = db
            .enqueue_invalidation_job_with_pending_cap(
                targeted_policy_repair_input("sig-policy-1", "claim-policy", "briefing", 1),
                1,
            )
            .expect("enqueue first surface repair");

        let err = db
            .enqueue_invalidation_job_with_pending_cap(
                targeted_policy_repair_input(
                    "sig-policy-2",
                    "claim-policy",
                    "tauri_entity_detail",
                    2,
                ),
                1,
            )
            .expect_err("distinct surface repair should reject at cap");
        assert!(
            matches!(
                err,
                DbError::InvalidArgument(ref message)
                    if message.contains("invalidation queue pending cap 1 reached")
            ),
            "expected visible InvalidArgument cap rejection, got {err}"
        );

        let job = db
            .get_invalidation_job(&first.job_id)
            .expect("read first surface job")
            .expect("first surface job");
        let payload: serde_json::Value =
            serde_json::from_str(&job.payload_json).expect("payload json");
        assert_eq!(
            payload
                .get("feedback_id")
                .and_then(serde_json::Value::as_str),
            Some("feedback-briefing")
        );
        assert_eq!(job.raw_signal_count, 1);
    }

    #[test]
    fn pending_cap_rejects_targeted_repair_with_distinct_claim_key() {
        let db = test_db();
        let subject_id = "acct-targeted-repair-cap";
        seed_account(&db, subject_id, 1);
        let first = db
            .enqueue_invalidation_job_with_pending_cap(
                targeted_bounded_corroboration_input(
                    "sig-repair-1",
                    subject_id,
                    "claim-repair-1",
                    "feedback-repair-1",
                    1,
                ),
                1,
            )
            .expect("enqueue first targeted repair");

        let err = db
            .enqueue_invalidation_job_with_pending_cap(
                targeted_bounded_corroboration_input(
                    "sig-repair-2",
                    subject_id,
                    "claim-repair-2",
                    "feedback-repair-2",
                    1,
                ),
                1,
            )
            .expect_err("distinct claim repair should reject at cap");
        assert!(
            matches!(
                err,
                DbError::InvalidArgument(ref message)
                    if message.contains("invalidation queue pending cap 1 reached")
            ),
            "expected visible InvalidArgument cap rejection, got {err}"
        );

        let job = db
            .get_invalidation_job(&first.job_id)
            .expect("read first targeted repair job")
            .expect("first targeted repair job");
        let payload: serde_json::Value =
            serde_json::from_str(&job.payload_json).expect("payload json");
        assert_eq!(
            payload.get("claim_id").and_then(serde_json::Value::as_str),
            Some("claim-repair-1")
        );
        assert_eq!(
            payload
                .get("feedback_id")
                .and_then(serde_json::Value::as_str),
            Some("feedback-repair-1")
        );
        assert_eq!(job.raw_signal_count, 1);
    }

    #[test]
    fn pending_cap_rejects_distinct_output_visibly() {
        let db = test_db();
        seed_account(&db, "acct-cap-a", 1);
        seed_account(&db, "acct-cap-b", 1);
        db.enqueue_invalidation_job_with_pending_cap(claim_input("sig-1", "acct-cap-a", 1), 1)
            .expect("enqueue first");

        let err = db
            .enqueue_invalidation_job_with_pending_cap(claim_input("sig-2", "acct-cap-b", 1), 1)
            .expect_err("distinct output should be rejected at cap");
        assert!(
            matches!(
                err,
                DbError::InvalidArgument(ref message)
                    if message.contains("invalidation queue pending cap 1 reached")
            ),
            "expected visible InvalidArgument cap rejection, got {err}"
        );
    }

    #[test]
    fn pending_job_survives_database_reopen() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("restart.db");
        {
            let db = ActionDb::open_at_unencrypted(path.clone()).expect("open first");
            seed_account(&db, "acct-restart", 1);
            db.conn_ref()
                .execute(
                    "INSERT INTO signal_events (
                        id, entity_type, entity_id, signal_type, data_source,
                        confidence, decay_half_life_days, created_at
                     ) VALUES (
                        'sig-1', 'account', 'acct-restart', 'claim_trust_changed',
                        'test', 1.0, 30, '2026-05-08T00:00:00Z'
                     )",
                    [],
                )
                .expect("seed signal");
            db.enqueue_invalidation_job(claim_input("sig-1", "acct-restart", 1))
                .expect("enqueue");
        }
        {
            let db = ActionDb::open_at_unencrypted(path).expect("open second");
            let claimed = db
                .claim_next_claim_recompute_job("worker-after-restart", 30)
                .expect("claim")
                .expect("job survived restart");
            assert_eq!(claimed.subject_id, "acct-restart");
            assert_eq!(claimed.status, STATUS_RUNNING);
        }
    }

    #[test]
    fn pending_job_survives_simulated_key_rotation_reopen() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("restart-rotated.db");
        let provider = Arc::new(RotatingFixtureKeyProvider::new(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789",
        ));

        {
            let db = ActionDb::open_at(path.clone(), provider.clone()).expect("open first");
            seed_account(&db, "acct-rotated-restart", 1);
            db.conn_ref()
                .execute(
                    "INSERT INTO signal_events (
                        id, entity_type, entity_id, signal_type, data_source,
                        confidence, decay_half_life_days, created_at
                     ) VALUES (
                        'sig-rotated-1', 'account', 'acct-rotated-restart',
                        'claim_trust_changed', 'test', 1.0, 30,
                        '2026-05-08T00:00:00Z'
                     )",
                    [],
                )
                .expect("seed signal");
            db.enqueue_invalidation_job(claim_input("sig-rotated-1", "acct-rotated-restart", 1))
                .expect("enqueue");
        }

        provider
            .rotate_key(&UserIdentity::local(path.clone()))
            .expect("rotate key");

        {
            let db = ActionDb::open_at(path, provider).expect("open second after rotation");
            let claimed = db
                .claim_next_claim_recompute_job("worker-after-rotation-restart", 30)
                .expect("claim")
                .expect("job survived restart after key rotation");
            assert_eq!(claimed.subject_id, "acct-rotated-restart");
            assert_eq!(claimed.status, STATUS_RUNNING);
        }
    }
}
