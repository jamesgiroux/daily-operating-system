//! Durable correction propagation for the correction loop.
//!
//! `services::claims::record_claim_feedback` remains the single feedback
//! writer. This module is the writer-owned durable middle layer: it persists
//! the correction envelope, claim-type source reliability deltas, subject-fit
//! deltas, and bounded logical propagation targets in the same transaction as
//! the `claim_feedback` row.

use std::sync::Arc;

use abilities_runtime::sensitivity::ClaimDismissalSurface;
use chrono::Duration;
use rusqlite::{params, OptionalExtension};
use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::abilities::feedback::{FeedbackAction, RepairAction};
use crate::db::claim_invalidation::SubjectRef;
use crate::db::claims::IntelligenceClaim;
use crate::db::ActionDb;
use crate::services::claims::{ClaimError, ClaimFeedbackInput};
use crate::services::context::ServiceContext;
use crate::state::AppState;

pub(crate) const SOURCE_KEY_VERSION: i64 = 1;
pub(crate) const SOURCE_KEY_SIGNAL_TYPE: &str = "user_feedback";
const SOURCE_KEY_EPOCH_DOMAIN: &str = "dailyos.w4.source_reliability.epoch";
const SOURCE_KEY_HASH_DOMAIN: &str = "dailyos.w4.source_reliability.key";
const DEFAULT_MAX_PROPAGATION_ROWS: usize = 32;
const STARTUP_DRAIN_LIMIT: usize = 100;
const SOURCE_BACKFILL_STARTUP_BATCH_ROWS: usize = 100;
const WORKER_IDLE_POLL_MS: u64 = 250;
const WORKER_ERROR_POLL_MS: u64 = 2_000;
const RUNNING_JOB_LEASE_SECONDS: i64 = 300;
const SOURCE_BACKFILL_RUNNING_LEASE_SECONDS: i64 = 300;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceReliabilityKey {
    pub data_source: String,
    pub source_key_kind: String,
    pub source_key_version: i64,
    pub source_key_epoch_hash: String,
    pub source_key_hash: String,
    pub claim_type: String,
    pub signal_type: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ClaimTypeSourceReliability {
    Active(f64),
    Excluded,
}

#[derive(Debug, Clone)]
pub(crate) struct FeedbackPropagationWrite<'a> {
    pub feedback_id: &'a str,
    pub claim: &'a IntelligenceClaim,
    pub input: &'a ClaimFeedbackInput,
    pub subject: &'a SubjectRef,
    pub repair: RepairAction,
    pub repair_job_id: Option<&'a str>,
    pub claim_file_apply_key: Option<&'a str>,
    pub now: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeedbackPropagationSummary {
    pub envelope_written: bool,
    pub source_reliability_delta_written: bool,
    pub subject_inference_delta_written: bool,
    pub propagation_job_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceReliabilityBackfillReport {
    pub run_id: String,
    pub status: String,
    pub eligible_feedback_rows: usize,
    pub applied_delta_rows: usize,
    pub skipped_feedback_rows: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FeedbackPropagationProcessOutcome {
    NoJob,
    Completed { job_id: String },
    Stale { job_id: String },
    RetryScheduled { job_id: String },
    DeadLettered { job_id: String },
}

#[derive(Debug, Clone)]
struct FeedbackPropagationJob {
    id: String,
    feedback_id: String,
    target_kind: String,
    operation: String,
    sync_class: String,
    scope_json: String,
    retry_count: i64,
    max_attempts: i64,
}

#[derive(Debug, Clone)]
struct SourceBackfillFeedbackRow {
    feedback_id: String,
    claim_id: String,
    feedback_type: String,
    payload_json: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JobCompletion {
    Completed(&'static str),
    Stale(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SyncClass {
    BoundedSync,
    Async,
}

impl SyncClass {
    fn as_str(self) -> &'static str {
        match self {
            Self::BoundedSync => "bounded_sync",
            Self::Async => "async",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PropagationStatus {
    Pending,
    Completed,
    Coalesced,
}

impl PropagationStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Completed => "completed",
            Self::Coalesced => "coalesced",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PropagationTarget {
    target_kind: &'static str,
    operation: &'static str,
    sync_class: SyncClass,
    status: PropagationStatus,
    coalescing_key: String,
    scope_json: serde_json::Value,
    cursor_json: Option<serde_json::Value>,
}

pub(crate) fn record_feedback_propagation_in_tx(
    tx: &ActionDb,
    write: FeedbackPropagationWrite<'_>,
) -> Result<FeedbackPropagationSummary, ClaimError> {
    let source_key = derive_source_reliability_key(
        &write.claim.data_source,
        write.claim.source_ref.as_deref(),
        write.input.payload_json.as_deref(),
        write.claim.item_hash.as_deref(),
        write.subject,
        &write.claim.claim_type,
        SOURCE_KEY_SIGNAL_TYPE,
    )?;
    let action_metadata = parse_payload_json(write.input.payload_json.as_deref())?;
    let subject_hash = pii_safe_hash(
        "subject",
        "dailyos.w4.correction.subject",
        &[&canonical_subject_storage(write.subject)],
    )?;

    insert_correction_envelope(tx, &write, &source_key, &subject_hash, &action_metadata)?;
    let source_reliability_delta_written = apply_source_reliability_delta(tx, &write, &source_key)?;
    let subject_inference_delta_written = apply_subject_inference_delta(tx, &write, &subject_hash)?;

    let mut targets = propagation_targets_for_feedback(&write, &source_key, &subject_hash);
    if targets.len() > DEFAULT_MAX_PROPAGATION_ROWS {
        targets = vec![coalesced_target_for_feedback(
            &write,
            targets.len(),
            &subject_hash,
        )];
    }
    for target in &targets {
        insert_propagation_target(tx, write.feedback_id, write.input.action, target, write.now)?;
    }

    Ok(FeedbackPropagationSummary {
        envelope_written: true,
        source_reliability_delta_written,
        subject_inference_delta_written,
        propagation_job_count: targets.len(),
    })
}

pub(crate) fn derive_source_reliability_key_for_claim(
    claim: &IntelligenceClaim,
    subject_type: &str,
    signal_type: &str,
) -> Result<SourceReliabilityKey, ClaimError> {
    let subject = subject_from_kind_id(subject_type, claim.subject_ref.as_str())?;
    let source_content_payload = source_content_hash_payload_for_claim(claim);
    derive_source_reliability_key(
        &claim.data_source,
        claim.source_ref.as_deref(),
        source_content_payload.as_deref(),
        claim.item_hash.as_deref(),
        &subject,
        &claim.claim_type,
        signal_type,
    )
}

pub(crate) fn read_claim_type_source_reliability(
    db: &ActionDb,
    key: &SourceReliabilityKey,
) -> Result<Option<ClaimTypeSourceReliability>, ClaimError> {
    let row = db
        .conn_ref()
        .query_row(
            "SELECT alpha, beta, excluded_at, stale_key_version
               FROM source_claim_type_reliability
              WHERE source_key_version = ?1
                AND source_key_epoch_hash = ?2
                AND source_key_hash = ?3
                AND claim_type = ?4
                AND signal_type = ?5",
            params![
                key.source_key_version,
                key.source_key_epoch_hash,
                key.source_key_hash,
                key.claim_type,
                key.signal_type,
            ],
            |row| {
                Ok((
                    row.get::<_, f64>(0)?,
                    row.get::<_, f64>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                ))
            },
        )
        .optional()
        .map_err(|error| {
            if sqlite_missing_w4_reliability_table(&error) {
                ClaimError::InvalidFeedback("w4_source_reliability_table_missing".to_string())
            } else {
                ClaimError::Rusqlite(error)
            }
        })?;

    let Some((alpha, beta, excluded_at, stale_key_version)) = row else {
        return Ok(None);
    };
    if excluded_at.is_some() || stale_key_version.is_some() {
        return Ok(Some(ClaimTypeSourceReliability::Excluded));
    }
    let denominator = alpha + beta;
    if !alpha.is_finite()
        || !beta.is_finite()
        || alpha < 0.0
        || beta < 0.0
        || !denominator.is_finite()
        || denominator <= 0.0
    {
        return Err(ClaimError::InvalidFeedback(
            "malformed source_claim_type_reliability components".to_string(),
        ));
    }
    Ok(Some(ClaimTypeSourceReliability::Active(
        alpha / denominator,
    )))
}

pub(crate) fn run_source_reliability_feedback_backfill(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    max_rows: usize,
) -> Result<SourceReliabilityBackfillReport, ClaimError> {
    ctx.check_mutation_allowed()
        .map_err(|error| ClaimError::Mode(error.to_string()))?;
    if max_rows == 0 {
        return Err(ClaimError::InvalidFeedback(
            "source reliability backfill requires max_rows > 0".to_string(),
        ));
    }
    let run_id = Uuid::new_v4().to_string();
    let now_value = ctx.clock.now();
    let now = now_value.to_rfc3339();
    let lease_expired_before =
        (now_value - Duration::seconds(SOURCE_BACKFILL_RUNNING_LEASE_SECONDS)).to_rfc3339();
    let current_epoch_hash = source_key_epoch_hash()?;
    db.with_transaction(|tx| {
        reclaim_source_backfill_runs_for_restart(
            tx,
            &current_epoch_hash,
            &lease_expired_before,
            &now,
        )
        .map_err(|error| error.to_string())?;
        tx.conn_ref()
            .execute(
                "INSERT INTO source_reliability_backfill_runs (
                    id, status, source_key_version, source_key_epoch_hash,
                    cursor_json, retry_count, started_at, created_at, updated_at
                 ) VALUES (?1, 'running', ?2, ?3, '{}', 0, ?4, ?4, ?4)",
                params![&run_id, SOURCE_KEY_VERSION, &current_epoch_hash, &now,],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    })
    .map_err(ClaimError::Transaction)?;

    let work = db.with_transaction(|tx| {
        let rows = load_feedback_rows_for_source_backfill(tx, max_rows)
            .map_err(|error| error.to_string())?;
        let mut eligible = 0usize;
        let mut applied = 0usize;
        let mut skipped = 0usize;
        let mut high_water_feedback_id: Option<String> = None;
        for row in rows {
            high_water_feedback_id = Some(row.feedback_id.clone());
            let Some(action) = feedback_action_from_storage(&row.feedback_type) else {
                skipped += 1;
                continue;
            };
            if source_reliability_delta(action).is_none() {
                skipped += 1;
                continue;
            }
            let Some(claim) =
                crate::services::claims::load_claim_by_id(tx.conn_ref(), &row.claim_id)
                    .map_err(|error| error.to_string())?
            else {
                skipped += 1;
                continue;
            };
            let subject_value: serde_json::Value = match serde_json::from_str(&claim.subject_ref) {
                Ok(value) => value,
                Err(_) => {
                    record_source_backfill_terminal_skip_delta(
                        tx,
                        &row,
                        &claim,
                        "malformed_subject_ref",
                        &now,
                    )
                    .map_err(|error| error.to_string())?;
                    skipped += 1;
                    continue;
                }
            };
            let subject = match crate::services::claims::subject_ref_from_json(&subject_value) {
                Ok(subject) => subject,
                Err(_) => {
                    record_source_backfill_terminal_skip_delta(
                        tx,
                        &row,
                        &claim,
                        "malformed_subject_ref",
                        &now,
                    )
                    .map_err(|error| error.to_string())?;
                    skipped += 1;
                    continue;
                }
            };
            let input = ClaimFeedbackInput {
                claim_id: row.claim_id.clone(),
                action,
                actor: "user".to_string(),
                actor_id: None,
                payload_json: row.payload_json.clone(),
            };
            let key = match derive_source_reliability_key(
                &claim.data_source,
                claim.source_ref.as_deref(),
                input.payload_json.as_deref(),
                claim.item_hash.as_deref(),
                &subject,
                &claim.claim_type,
                SOURCE_KEY_SIGNAL_TYPE,
            ) {
                Ok(key) => key,
                Err(_) => {
                    record_source_backfill_terminal_skip_delta(
                        tx,
                        &row,
                        &claim,
                        "malformed_source_key_material",
                        &now,
                    )
                    .map_err(|error| error.to_string())?;
                    skipped += 1;
                    continue;
                }
            };
            eligible += 1;
            let inserted = apply_source_reliability_delta(
                tx,
                &FeedbackPropagationWrite {
                    feedback_id: &row.feedback_id,
                    claim: &claim,
                    input: &input,
                    subject: &subject,
                    repair: RepairAction::None,
                    repair_job_id: None,
                    claim_file_apply_key: None,
                    now: &now,
                },
                &key,
            )
            .map_err(|error| error.to_string())?;
            if inserted {
                applied += 1;
            }
        }

        let cursor_json = stable_json_string(&json!({
            "eligible_feedback_rows": eligible,
            "applied_delta_rows": applied,
            "skipped_feedback_rows": skipped,
            "max_rows": max_rows
        }))
        .map_err(|error| error.to_string())?;
        tx.conn_ref()
            .execute(
                "UPDATE source_reliability_backfill_runs
                    SET status = 'completed',
                        cursor_json = ?2,
                        high_water_feedback_id = ?3,
                        completed_at = ?4,
                        terminalized_at = ?4,
                        updated_at = ?4
                  WHERE id = ?1",
                params![&run_id, cursor_json, high_water_feedback_id, &now],
            )
            .map_err(|error| error.to_string())?;
        Ok(SourceReliabilityBackfillReport {
            run_id: run_id.clone(),
            status: "completed".to_string(),
            eligible_feedback_rows: eligible,
            applied_delta_rows: applied,
            skipped_feedback_rows: skipped,
        })
    });

    match work {
        Ok(report) => Ok(report),
        Err(error) => {
            mark_source_backfill_run_failed(
                db,
                &run_id,
                "source_reliability_backfill_failed",
                &now,
            )?;
            Err(ClaimError::Transaction(error))
        }
    }
}

fn mark_source_backfill_run_failed(
    db: &ActionDb,
    run_id: &str,
    reason_code: &str,
    now: &str,
) -> Result<(), ClaimError> {
    let cursor_json = stable_json_string(&json!({
        "terminal_failure": true,
        "reason_code": reason_code
    }))?;
    db.with_transaction(|tx| {
        tx.conn_ref()
            .execute(
                "UPDATE source_reliability_backfill_runs
                    SET status = 'failed',
                        cursor_json = ?2,
                        retry_count = retry_count + 1,
                        failure_reason_code = ?3,
                        terminalized_at = ?4,
                        updated_at = ?4
                  WHERE id = ?1
                    AND terminalized_at IS NULL",
                params![run_id, cursor_json, reason_code, now],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    })
    .map_err(ClaimError::Transaction)
}

fn reclaim_source_backfill_runs_for_restart(
    tx: &ActionDb,
    current_epoch_hash: &str,
    lease_expired_before: &str,
    now: &str,
) -> Result<(), ClaimError> {
    tx.conn_ref().execute(
        "UPDATE source_reliability_backfill_runs
                SET status = 'stale_key_version',
                    failure_reason_code = 'source_key_version_changed',
                    stale_key_version_at = ?3,
                    terminalized_at = ?3,
                    updated_at = ?3
              WHERE status IN ('pending', 'running', 'failed')
                AND (
                    source_key_version != ?1
                    OR source_key_epoch_hash != ?2
                )
                AND terminalized_at IS NULL",
        params![SOURCE_KEY_VERSION, current_epoch_hash, now],
    )?;
    tx.conn_ref().execute(
        "UPDATE source_reliability_backfill_runs
                SET status = 'aborted',
                    retry_count = retry_count + 1,
                    failure_reason_code = 'restart_reclaimed_interrupted_run',
                    terminalized_at = ?3,
                    updated_at = ?3
              WHERE status = 'running'
                AND source_key_version = ?1
                AND source_key_epoch_hash = ?2
                AND updated_at <= ?4
                AND terminalized_at IS NULL",
        params![
            SOURCE_KEY_VERSION,
            current_epoch_hash,
            now,
            lease_expired_before
        ],
    )?;
    Ok(())
}

pub async fn drain_source_reliability_feedback_backfill(state: &Arc<AppState>) {
    for _ in 0..STARTUP_DRAIN_LIMIT {
        let result = state
            .db_write(move |db| {
                let clock = crate::services::context::SystemClock;
                let rng = crate::services::context::SystemRng;
                let ext = crate::services::context::ExternalClients::default();
                let ctx = ServiceContext::new_live(&clock, &rng, &ext);
                run_source_reliability_feedback_backfill(
                    &ctx,
                    db,
                    SOURCE_BACKFILL_STARTUP_BATCH_ROWS,
                )
                .map_err(|error| error.to_string())
            })
            .await;

        match result {
            Ok(report) if report.eligible_feedback_rows == 0 => break,
            Ok(report) => log::info!(
                "Source reliability feedback backfill processed run {}: eligible={}, applied={}, skipped={}",
                report.run_id,
                report.eligible_feedback_rows,
                report.applied_delta_rows,
                report.skipped_feedback_rows
            ),
            Err(error) => {
                let message = error.to_string();
                state
                    .recover_db_service_after_access_error(&error, "Source reliability backfill")
                    .await;
                log::warn!("Source reliability feedback backfill stopped: {message}");
                break;
            }
        }
    }
}

pub(crate) fn process_one_feedback_propagation_job(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    worker_id: &str,
) -> Result<FeedbackPropagationProcessOutcome, ClaimError> {
    ctx.check_mutation_allowed()
        .map_err(|error| ClaimError::Mode(error.to_string()))?;
    let Some(job) = claim_next_feedback_propagation_job(ctx, db, worker_id)? else {
        return Ok(FeedbackPropagationProcessOutcome::NoJob);
    };
    let job_id = job.id.clone();
    let completion = if job_requires_standalone_transactions(&job) {
        let completion = run_feedback_propagation_job(ctx, db, &job);
        match completion {
            Ok(completion) => db
                .with_transaction(|tx| {
                    mark_feedback_propagation_job_complete(ctx, tx, &job, completion)
                        .map_err(|error| error.to_string())?;
                    Ok(completion)
                })
                .map_err(ClaimError::Transaction),
            Err(error) => Err(error),
        }
    } else {
        db.with_transaction(|tx| {
            let completion =
                run_feedback_propagation_job(ctx, tx, &job).map_err(|error| error.to_string())?;
            mark_feedback_propagation_job_complete(ctx, tx, &job, completion)
                .map_err(|error| error.to_string())?;
            Ok(completion)
        })
        .map_err(ClaimError::Transaction)
    };

    match completion {
        Ok(JobCompletion::Completed(_)) => {
            Ok(FeedbackPropagationProcessOutcome::Completed { job_id })
        }
        Ok(JobCompletion::Stale(_)) => Ok(FeedbackPropagationProcessOutcome::Stale { job_id }),
        Err(error) => mark_feedback_propagation_job_failed(ctx, db, &job, &error.to_string()),
    }
}

fn job_requires_standalone_transactions(job: &FeedbackPropagationJob) -> bool {
    matches!(
        (job.target_kind.as_str(), job.operation.as_str()),
        ("salience_surfacing", "rerank_bounded_candidates")
    )
}

pub async fn drain_pending_feedback_propagation_jobs(state: &Arc<AppState>) {
    let worker_id = format!("feedback-propagation-startup-{}", Uuid::new_v4());
    for _ in 0..STARTUP_DRAIN_LIMIT {
        let worker_id = worker_id.clone();
        let result = state
            .db_write(move |db| {
                let clock = crate::services::context::SystemClock;
                let rng = crate::services::context::SystemRng;
                let ext = crate::services::context::ExternalClients::default();
                let ctx = ServiceContext::new_live(&clock, &rng, &ext);
                process_one_feedback_propagation_job(&ctx, db, &worker_id)
                    .map_err(|error| error.to_string())
            })
            .await;

        match result {
            Ok(FeedbackPropagationProcessOutcome::NoJob) => break,
            Ok(outcome) => log::info!("Feedback propagation drain processed {outcome:?}"),
            Err(error) => {
                let message = error.to_string();
                state
                    .recover_db_service_after_access_error(&error, "Feedback propagation drain")
                    .await;
                log::warn!("Feedback propagation drain stopped: {message}");
                break;
            }
        }
    }
}

pub async fn run_feedback_propagation_worker(state: Arc<AppState>) {
    let worker_id = format!("feedback-propagation-worker-{}", Uuid::new_v4());
    loop {
        if state.is_database_recovery_required() {
            log::warn!("Feedback propagation worker stopped: database recovery required");
            break;
        }
        let worker_id_for_db = worker_id.clone();
        let result = state
            .db_write(move |db| {
                let clock = crate::services::context::SystemClock;
                let rng = crate::services::context::SystemRng;
                let ext = crate::services::context::ExternalClients::default();
                let ctx = ServiceContext::new_live(&clock, &rng, &ext);
                process_one_feedback_propagation_job(&ctx, db, &worker_id_for_db)
                    .map_err(|error| error.to_string())
            })
            .await;

        match result {
            Ok(FeedbackPropagationProcessOutcome::NoJob) => {
                tokio::time::sleep(std::time::Duration::from_millis(WORKER_IDLE_POLL_MS)).await;
            }
            Ok(outcome) => log::info!("Feedback propagation worker processed {outcome:?}"),
            Err(error) => {
                let message = error.to_string();
                state
                    .recover_db_service_after_access_error(&error, "Feedback propagation worker")
                    .await;
                if error.is_retryable() && !message.contains("file is not a database") {
                    log::debug!(
                        "Feedback propagation worker iteration retrying after transient DB contention: {message}"
                    );
                } else {
                    log::warn!("Feedback propagation worker iteration failed: {message}");
                }
                tokio::time::sleep(std::time::Duration::from_millis(WORKER_ERROR_POLL_MS)).await;
            }
        }
    }
}

fn claim_next_feedback_propagation_job(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    worker_id: &str,
) -> Result<Option<FeedbackPropagationJob>, ClaimError> {
    let now_value = ctx.clock.now();
    let now = now_value.to_rfc3339();
    let lease_expired_before =
        (now_value - Duration::seconds(RUNNING_JOB_LEASE_SECONDS)).to_rfc3339();
    let run_id = format!("{worker_id}:{}", Uuid::new_v4());
    db.with_transaction(|tx| {
        reclaim_expired_feedback_propagation_jobs(tx, &lease_expired_before, &now)
            .map_err(|error| error.to_string())?;
        let job_id = tx
            .conn_ref()
            .query_row(
                "SELECT id
                   FROM claim_feedback_propagation_jobs
                  WHERE status = 'pending'
                    AND datetime(next_run_at) <= datetime(?1)
                  ORDER BY
                    CASE target_kind
                        WHEN 'claim_recompute' THEN 0
                        WHEN 'targeted_repair' THEN 1
                        WHEN 'derived_context' THEN 2
                        ELSE 3
                    END,
                    created_at ASC
                  LIMIT 1",
                params![&now],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        let Some(job_id) = job_id else {
            return Ok(None);
        };

        tx.conn_ref()
            .execute(
                "UPDATE claim_feedback_propagation_jobs
                SET status = 'running',
                    enqueue_run_id = ?2,
                    retry_count = retry_count + 1,
                    updated_at = ?3
              WHERE id = ?1
                AND status = 'pending'",
                params![&job_id, &run_id, &now],
            )
            .map_err(|error| error.to_string())?;

        read_feedback_propagation_job(tx, &job_id).map_err(|error| error.to_string())
    })
    .map_err(ClaimError::Transaction)
}

fn reclaim_expired_feedback_propagation_jobs(
    tx: &ActionDb,
    lease_expired_before: &str,
    now: &str,
) -> Result<(), ClaimError> {
    tx.conn_ref().execute(
        "UPDATE claim_feedback_propagation_jobs
            SET status = 'dead_lettered',
                failure_reason_code = 'worker_lease_expired',
                dead_lettered_at = ?2,
                updated_at = ?2
          WHERE status = 'running'
            AND updated_at <= ?1
            AND retry_count >= max_attempts",
        params![lease_expired_before, now],
    )?;
    tx.conn_ref().execute(
        "UPDATE claim_feedback_propagation_jobs
            SET status = 'pending',
                stale_reason = NULL,
                failure_reason_code = 'worker_lease_expired',
                enqueue_run_id = NULL,
                next_run_at = ?2,
                updated_at = ?2
          WHERE status = 'running'
            AND updated_at <= ?1
            AND retry_count < max_attempts",
        params![lease_expired_before, now],
    )?;
    Ok(())
}

fn read_feedback_propagation_job(
    db: &ActionDb,
    job_id: &str,
) -> Result<Option<FeedbackPropagationJob>, ClaimError> {
    db.conn_ref()
        .query_row(
            "SELECT id, feedback_id, target_kind, operation, sync_class,
                    scope_json, retry_count, max_attempts
               FROM claim_feedback_propagation_jobs
              WHERE id = ?1",
            params![job_id],
            |row| {
                Ok(FeedbackPropagationJob {
                    id: row.get(0)?,
                    feedback_id: row.get(1)?,
                    target_kind: row.get(2)?,
                    operation: row.get(3)?,
                    sync_class: row.get(4)?,
                    scope_json: row.get(5)?,
                    retry_count: row.get(6)?,
                    max_attempts: row.get(7)?,
                })
            },
        )
        .optional()
        .map_err(ClaimError::Rusqlite)
}

fn load_feedback_rows_for_source_backfill(
    db: &ActionDb,
    max_rows: usize,
) -> Result<Vec<SourceBackfillFeedbackRow>, ClaimError> {
    let limit = i64::try_from(max_rows).map_err(|_| {
        ClaimError::InvalidFeedback("source reliability backfill max_rows too large".to_string())
    })?;
    let mut stmt = db.conn_ref().prepare(
        "SELECT feedback.id, feedback.claim_id, feedback.feedback_type, feedback.payload_json
           FROM claim_feedback feedback
          WHERE feedback.feedback_type IN ('confirm_current', 'wrong_source', 'mark_false')
            AND NOT EXISTS (
                SELECT 1
                  FROM source_reliability_feedback_deltas delta
                 WHERE delta.feedback_id = feedback.id
                   AND delta.signal_type = 'user_feedback'
                   AND delta.status IN ('applied', 'redacted', 'source_removed', 'stale_key_version')
            )
          ORDER BY feedback.submitted_at ASC, feedback.id ASC
          LIMIT ?1",
    )?;
    let rows = stmt
        .query_map(params![limit], |row| {
            Ok(SourceBackfillFeedbackRow {
                feedback_id: row.get(0)?,
                claim_id: row.get(1)?,
                feedback_type: row.get(2)?,
                payload_json: row.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn run_feedback_propagation_job(
    ctx: &ServiceContext<'_>,
    tx: &ActionDb,
    job: &FeedbackPropagationJob,
) -> Result<JobCompletion, ClaimError> {
    match (job.target_kind.as_str(), job.operation.as_str()) {
        ("claim_recompute", "targeted_claim_recompute") => {
            let (subject_type, subject_id) = recompute_subject_from_scope(&job.scope_json)?;
            crate::services::invalidation_jobs::enqueue_direct_claim_recompute_in_tx(
                tx,
                &subject_type,
                &subject_id,
                "claim_feedback_propagation",
            )
            .map_err(|error| {
                ClaimError::Transaction(format!("enqueue claim recompute: {error}"))
            })?;
            Ok(JobCompletion::Completed("claim_recompute_enqueued"))
        }
        ("targeted_repair", _) => Ok(JobCompletion::Completed("targeted_repair_already_enqueued")),
        ("derived_context", _) => {
            let subject = subject_ref_from_scope(&job.scope_json)?;
            tx.bump_for_subject(&subject)?;
            Ok(JobCompletion::Completed("subject_claim_version_bumped"))
        }
        ("prep_regeneration", "enqueue_prep_regeneration") => {
            let (subject_type, subject_id) = recompute_subject_from_scope(&job.scope_json)?;
            if subject_type != "meeting" {
                return Ok(JobCompletion::Stale(
                    "prep_regeneration_requires_meeting_subject",
                ));
            }
            let scope: serde_json::Value = serde_json::from_str(&job.scope_json)?;
            let field_path = scope
                .get("field_path")
                .and_then(serde_json::Value::as_str)
                .filter(|value| !value.is_empty());
            crate::services::meeting_prep_status::write::enqueue_prep_regeneration_job_in_tx(
                tx,
                &subject_id,
                field_path,
                &ctx.clock.now().to_rfc3339(),
            )
            .map_err(|error| {
                ClaimError::Transaction(format!("enqueue prep regeneration: {error}"))
            })?;
            Ok(JobCompletion::Completed("prep_regeneration_enqueued"))
        }
        ("salience_surfacing", "rerank_bounded_candidates") => {
            run_salience_surfacing_propagation(ctx, tx, job)
        }
        ("surface_policy", "apply_named_surface_suppression") => {
            run_surface_policy_propagation(tx, job, &ctx.clock.now().to_rfc3339())
        }
        ("review_queue", _) => run_review_queue_propagation(tx, job, &ctx.clock.now().to_rfc3339()),
        ("subject_graph", _) => {
            run_subject_graph_propagation(tx, job, &ctx.clock.now().to_rfc3339())
        }
        _ => Ok(JobCompletion::Stale("propagation_worker_not_available")),
    }
}

fn run_salience_surfacing_propagation(
    ctx: &ServiceContext<'_>,
    tx: &ActionDb,
    job: &FeedbackPropagationJob,
) -> Result<JobCompletion, ClaimError> {
    let scope: serde_json::Value = serde_json::from_str(&job.scope_json)?;
    let claim_ids = claim_ids_from_salience_scope(&scope)?;
    let surface = scope
        .get("surface")
        .and_then(serde_json::Value::as_str)
        .and_then(ClaimDismissalSurface::from_name)
        .unwrap_or(ClaimDismissalSurface::Briefing);
    let engine = crate::signals::propagation::PropagationEngine::new();
    let mut salience_refreshed = 0usize;
    let mut surfacing_refreshed = 0usize;
    let mut skipped = 0usize;

    for claim_id in claim_ids.into_iter().take(25) {
        let claim_id = crate::services::recommendations::contracts::ClaimId(claim_id);
        let stable_source_signal_id = format!("feedback-propagation:{}:{}", job.id, claim_id.0);
        let stable_salience_evaluation_id =
            stable_feedback_propagation_salience_evaluation_id(job, &claim_id.0, surface);
        match crate::services::recommendations::salience::recompute_salience_for_claim_with_evaluation_id(
            ctx,
            tx,
            crate::services::recommendations::salience::ScoreSalienceRequest {
                schema_version:
                    crate::services::recommendations::salience::SCORE_SALIENCE_SCHEMA_VERSION,
                claim_id: claim_id.clone(),
            },
            stable_salience_evaluation_id,
        ) {
            Ok(_) => salience_refreshed += 1,
            Err(crate::services::recommendations::salience::SalienceError::ClaimNotFound(_))
            | Err(crate::services::recommendations::salience::SalienceError::ClaimNotVisible(_)) => {
                skipped += 1;
                continue;
            }
            Err(error) => {
                return Err(ClaimError::Transaction(format!(
                    "refresh salience for feedback: {error}"
                )));
            }
        }

        let input = crate::services::recommendations::surfacing::SurfacingEvaluationInput {
            schema_version:
                crate::services::recommendations::surfacing::SURFACING_EVALUATION_SCHEMA_VERSION,
            claim_id,
            render_surface: surface,
            surface_class: crate::services::recommendations::surfacing::SurfaceClass::Primary,
            trigger_refs: Vec::new(),
            source_signal_id: Some(stable_source_signal_id),
            trigger_source_asof: None,
            evidence_signature_changed: false,
            subject_version_changed: true,
        };
        match crate::services::recommendations::surfacing::evaluate_surfacing_for_claim(
            ctx, tx, &engine, input,
        ) {
            Ok(_) => surfacing_refreshed += 1,
            Err(crate::services::recommendations::surfacing::SurfacingError::ClaimNotFound(_))
            | Err(
                crate::services::recommendations::surfacing::SurfacingError::NotRecommendationClaim(
                    _,
                ),
            )
            | Err(crate::services::recommendations::surfacing::SurfacingError::ClaimNotVisible(
                _,
            )) => {
                skipped += 1;
            }
            Err(error) => {
                return Err(ClaimError::Transaction(format!(
                    "refresh surfacing for feedback: {error}"
                )));
            }
        }
    }

    if salience_refreshed > 0 || surfacing_refreshed > 0 {
        Ok(JobCompletion::Completed("salience_and_surfacing_refreshed"))
    } else if skipped > 0 {
        Ok(JobCompletion::Stale("no_recommendation_surface_candidate"))
    } else {
        Ok(JobCompletion::Stale("salience_scope_empty"))
    }
}

fn stable_feedback_propagation_salience_evaluation_id(
    job: &FeedbackPropagationJob,
    claim_id: &str,
    surface: ClaimDismissalSurface,
) -> String {
    let source_signal_id = format!("feedback-propagation:{}:{claim_id}", job.id);
    let mut hasher = Sha256::new();
    hasher.update(claim_id.as_bytes());
    hasher.update(b"\x1f");
    hasher.update(surface.as_str().as_bytes());
    hasher.update(b"\x1f");
    hasher.update(source_signal_id.as_bytes());
    format!(
        "salience-eval-feedback-{}",
        hex::encode(&hasher.finalize()[..16])
    )
}

fn claim_ids_from_salience_scope(scope: &serde_json::Value) -> Result<Vec<String>, ClaimError> {
    if let Some(values) = scope.get("claim_ids").and_then(serde_json::Value::as_array) {
        let claim_ids = values
            .iter()
            .filter_map(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>();
        if !claim_ids.is_empty() {
            return Ok(claim_ids);
        }
    }
    let claim_id = scope
        .get("claim_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            ClaimError::InvalidFeedback("salience propagation scope missing claim_id".to_string())
        })?;
    Ok(vec![claim_id.to_string()])
}

fn run_surface_policy_propagation(
    tx: &ActionDb,
    job: &FeedbackPropagationJob,
    now: &str,
) -> Result<JobCompletion, ClaimError> {
    let scope: serde_json::Value = serde_json::from_str(&job.scope_json)?;
    let claim_id = claim_id_from_scope(&job.scope_json)?;
    let Some(surface) = scope
        .get("surface")
        .and_then(serde_json::Value::as_str)
        .and_then(ClaimDismissalSurface::from_name)
    else {
        return Ok(JobCompletion::Stale("surface_policy_unknown_surface"));
    };
    tx.conn_ref().execute(
        "INSERT INTO claim_surface_dismissals (
             claim_id, surface, feedback_id, actor, dismissed_at
         ) VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(claim_id, surface) DO UPDATE SET
             feedback_id = excluded.feedback_id,
             actor = excluded.actor,
             dismissed_at = excluded.dismissed_at",
        params![
            &claim_id,
            surface.as_str(),
            &job.feedback_id,
            "claim_feedback_propagation",
            now
        ],
    )?;
    let subject = subject_for_claim_id(tx, &claim_id)?;
    tx.bump_for_subject(&subject)?;
    Ok(JobCompletion::Completed(
        "surface_policy_suppression_applied",
    ))
}

fn run_review_queue_propagation(
    tx: &ActionDb,
    job: &FeedbackPropagationJob,
    now: &str,
) -> Result<JobCompletion, ClaimError> {
    let claim_id = claim_id_from_scope(&job.scope_json)?;
    let resolved_deferrals = tx.conn_ref().execute(
        "UPDATE claim_review_deferrals
            SET resolved_at = ?1,
                updated_at = ?1
          WHERE target_kind = 'claim'
            AND target_id = ?2
            AND resolved_at IS NULL",
        params![now, &claim_id],
    )?;
    tx.conn_ref().execute(
        "INSERT INTO claim_feedback_review_queue_events (
            id, feedback_id, claim_id, operation, resolved_deferrals,
            reason_code, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            Uuid::new_v4().to_string(),
            &job.feedback_id,
            &claim_id,
            &job.operation,
            resolved_deferrals,
            "review_queue_reconciled",
            now,
        ],
    )?;
    Ok(JobCompletion::Completed("review_queue_reconciled"))
}

fn run_subject_graph_propagation(
    tx: &ActionDb,
    job: &FeedbackPropagationJob,
    now: &str,
) -> Result<JobCompletion, ClaimError> {
    let claim_id = claim_id_from_scope(&job.scope_json)?;
    let subject = subject_ref_from_scope(&job.scope_json)?;
    let subject_json = canonical_subject_storage(&subject);
    tx.conn_ref().execute(
        "UPDATE entity_graph_version
            SET version = version + 1
          WHERE id = 1",
        [],
    )?;
    let version_after: Option<i64> = tx
        .conn_ref()
        .query_row(
            "SELECT version FROM entity_graph_version WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .optional()?;
    tx.conn_ref().execute(
        "INSERT INTO claim_feedback_subject_graph_invalidations (
            id, feedback_id, claim_id, subject_json, operation, reason_code,
            entity_graph_version_after, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            Uuid::new_v4().to_string(),
            &job.feedback_id,
            &claim_id,
            &subject_json,
            &job.operation,
            "subject_graph_invalidated",
            version_after,
            now,
        ],
    )?;
    Ok(JobCompletion::Completed("subject_graph_invalidated"))
}

fn mark_feedback_propagation_job_complete(
    ctx: &ServiceContext<'_>,
    tx: &ActionDb,
    job: &FeedbackPropagationJob,
    completion: JobCompletion,
) -> Result<(), ClaimError> {
    let now = ctx.clock.now().to_rfc3339();
    let (status, reason_code, completed_at) = match completion {
        JobCompletion::Completed(reason) => ("completed", reason, Some(now.as_str())),
        JobCompletion::Stale(reason) => ("stale", reason, None),
    };
    tx.conn_ref().execute(
        "UPDATE claim_feedback_propagation_jobs
            SET status = ?2,
                stale_reason = CASE WHEN ?2 = 'stale' THEN ?3 ELSE stale_reason END,
                failure_reason_code = NULL,
                completed_at = ?4,
                updated_at = ?5
          WHERE id = ?1
            AND status = 'running'",
        params![&job.id, status, reason_code, completed_at, &now],
    )?;
    insert_propagation_outcome_status(tx, job, status, Some(reason_code), &now)
}

fn mark_feedback_propagation_job_failed(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    job: &FeedbackPropagationJob,
    error: &str,
) -> Result<FeedbackPropagationProcessOutcome, ClaimError> {
    let now = ctx.clock.now().to_rfc3339();
    let next_run_at = (ctx.clock.now()
        + Duration::seconds(feedback_propagation_retry_backoff_seconds(job.retry_count)))
    .to_rfc3339();
    let terminal = job.retry_count >= job.max_attempts;
    db.with_transaction(|tx| {
        if terminal {
            tx.conn_ref()
                .execute(
                    "UPDATE claim_feedback_propagation_jobs
                    SET status = 'dead_lettered',
                        failure_reason_code = ?2,
                        dead_lettered_at = ?3,
                        updated_at = ?3
                  WHERE id = ?1",
                    params![&job.id, error, &now],
                )
                .map_err(|error| error.to_string())?;
            insert_propagation_outcome_status(tx, job, "dead_lettered", Some(error), &now)
                .map_err(|error| error.to_string())?;
        } else {
            tx.conn_ref()
                .execute(
                    "UPDATE claim_feedback_propagation_jobs
                    SET status = 'pending',
                        failure_reason_code = ?2,
                        updated_at = ?3,
                        next_run_at = ?4
                  WHERE id = ?1",
                    params![&job.id, error, &now, &next_run_at],
                )
                .map_err(|error| error.to_string())?;
            insert_propagation_outcome_status(tx, job, "pending", Some("retry_scheduled"), &now)
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    })
    .map_err(ClaimError::Transaction)?;

    if terminal {
        Ok(FeedbackPropagationProcessOutcome::DeadLettered {
            job_id: job.id.clone(),
        })
    } else {
        Ok(FeedbackPropagationProcessOutcome::RetryScheduled {
            job_id: job.id.clone(),
        })
    }
}

fn feedback_propagation_retry_backoff_seconds(attempts: i64) -> i64 {
    let exponent = attempts.saturating_sub(1).min(4) as u32;
    4_i64.pow(exponent)
}

fn insert_propagation_outcome_status(
    tx: &ActionDb,
    job: &FeedbackPropagationJob,
    status: &str,
    reason_code: Option<&str>,
    now: &str,
) -> Result<(), ClaimError> {
    let outcome_id = Uuid::new_v4().to_string();
    tx.conn_ref().execute(
        "INSERT INTO claim_feedback_propagation_outcomes (
            id, job_id, feedback_id, target_kind, operation, sync_class,
            status, reason_code, observed_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            outcome_id,
            job.id,
            job.feedback_id,
            job.target_kind,
            job.operation,
            job.sync_class,
            status,
            reason_code,
            now,
        ],
    )?;
    Ok(())
}

fn recompute_subject_from_scope(scope_json: &str) -> Result<(String, String), ClaimError> {
    let subject = subject_ref_from_scope(scope_json)?;
    let (subject_type, subject_id) = subject_kind_id(&subject);
    let Some(subject_id) = subject_id else {
        return Err(ClaimError::SubjectRef(
            "claim recompute propagation requires a concrete subject".to_string(),
        ));
    };
    Ok((subject_type.to_string(), subject_id.to_string()))
}

fn subject_ref_from_scope(scope_json: &str) -> Result<SubjectRef, ClaimError> {
    let scope: serde_json::Value = serde_json::from_str(scope_json)?;
    let raw_subject = scope
        .get("subject")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            ClaimError::InvalidFeedback("propagation scope missing subject".to_string())
        })?;
    let value: serde_json::Value = serde_json::from_str(raw_subject)?;
    crate::services::claims::subject_ref_from_json(&value)
}

fn claim_id_from_scope(scope_json: &str) -> Result<String, ClaimError> {
    let scope: serde_json::Value = serde_json::from_str(scope_json)?;
    scope
        .get("claim_id")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| {
            ClaimError::InvalidFeedback("propagation scope missing claim_id".to_string())
        })
}

fn subject_for_claim_id(tx: &ActionDb, claim_id: &str) -> Result<SubjectRef, ClaimError> {
    let subject_ref_json: String = tx
        .conn_ref()
        .query_row(
            "SELECT subject_ref FROM intelligence_claims WHERE id = ?1",
            params![claim_id],
            |row| row.get(0),
        )
        .map_err(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => {
                ClaimError::SubjectRef(format!("claim {claim_id} not found"))
            }
            other => ClaimError::Rusqlite(other),
        })?;
    let value: serde_json::Value = serde_json::from_str(&subject_ref_json)?;
    crate::services::claims::subject_ref_from_json(&value)
}

fn insert_correction_envelope(
    tx: &ActionDb,
    write: &FeedbackPropagationWrite<'_>,
    source_key: &SourceReliabilityKey,
    subject_hash: &str,
    action_metadata: &serde_json::Value,
) -> Result<(), ClaimError> {
    let subject_json = canonical_subject_storage(write.subject);
    let (subject_kind, subject_id) = subject_kind_id(write.subject);
    let corrected_subject_ref_json = action_metadata
        .get("corrected_subject_ref")
        .or_else(|| action_metadata.get("corrected_subject"))
        .map(stable_json_string)
        .transpose()?;
    let source_ref_hash = write
        .claim
        .source_ref
        .as_deref()
        .map(|source_ref| {
            pii_safe_hash(
                "source_ref",
                "dailyos.w4.correction.source_ref",
                &[&write.claim.data_source, source_ref],
            )
        })
        .transpose()?;
    let payload_hash = stable_json_hash(action_metadata)?;
    let replay_key = pii_safe_hash(
        "replay",
        "dailyos.w4.correction.replay_key",
        &[
            write.claim.item_hash.as_deref().unwrap_or(&write.claim.id),
            &subject_json,
            &write.claim.claim_type,
            write.claim.field_path.as_deref().unwrap_or(""),
            write.input.action.as_str(),
            &payload_hash,
        ],
    )?;
    let action_metadata_json = stable_json_string(action_metadata)?;

    tx.conn_ref().execute(
        "INSERT INTO claim_feedback_correction_envelopes (
            feedback_id, claim_id, action, actor, actor_id, surface,
            target_receipt_json, asserted_subject_ref_json, asserted_subject_kind,
            asserted_subject_id, corrected_subject_ref_json, field_path, data_source,
            source_ref, source_ref_hash, source_asof, source_key_version,
            source_key_epoch_hash, source_key_hash, claim_type, sensitivity,
            idempotency_key, replay_key, action_metadata_json, lifecycle_state,
            created_at, updated_at
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6,
            ?7, ?8, ?9,
            ?10, ?11, ?12, ?13,
            ?14, ?15, ?16, ?17,
            ?18, ?19, ?20, ?21,
            ?22, ?23, ?24, 'active',
            ?25, ?25
        )",
        params![
            write.feedback_id,
            write.claim.id,
            write.input.action.as_str(),
            write.input.actor,
            write.input.actor_id.as_deref(),
            surface_for_payload(action_metadata),
            target_receipt_for_payload(action_metadata)?,
            subject_json,
            subject_kind,
            subject_id,
            corrected_subject_ref_json,
            write.claim.field_path.as_deref(),
            write.claim.data_source,
            write.claim.source_ref.as_deref(),
            source_ref_hash,
            write.claim.source_asof.as_deref(),
            source_key.source_key_version,
            source_key.source_key_epoch_hash,
            source_key.source_key_hash,
            write.claim.claim_type,
            enum_storage(&write.claim.sensitivity)?,
            write.claim_file_apply_key,
            replay_key,
            action_metadata_json,
            write.now,
        ],
    )?;
    let _ = subject_hash;
    Ok(())
}

fn apply_source_reliability_delta(
    tx: &ActionDb,
    write: &FeedbackPropagationWrite<'_>,
    key: &SourceReliabilityKey,
) -> Result<bool, ClaimError> {
    let Some((effect_kind, alpha_delta, beta_delta)) = source_reliability_delta(write.input.action)
    else {
        return Ok(false);
    };
    let delta_id = Uuid::new_v4().to_string();
    let inserted = tx.conn_ref().execute(
        "INSERT OR IGNORE INTO source_reliability_feedback_deltas (
            id, feedback_id, source_key_version, source_key_epoch_hash,
            source_key_hash, data_source, source_key_kind, claim_type,
            signal_type, effect_kind, alpha_delta, beta_delta, status, applied_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 'applied', ?13)",
        params![
            delta_id,
            write.feedback_id,
            key.source_key_version,
            key.source_key_epoch_hash,
            key.source_key_hash,
            key.data_source,
            key.source_key_kind,
            key.claim_type,
            key.signal_type,
            effect_kind,
            alpha_delta,
            beta_delta,
            write.now,
        ],
    )?;
    if inserted == 0 {
        return Ok(false);
    }

    tx.conn_ref().execute(
        "INSERT INTO source_claim_type_reliability (
            source_key_version, source_key_epoch_hash, source_key_hash,
            data_source, source_key_kind, claim_type, signal_type,
            alpha, beta, update_count, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1.0 + ?8, 1.0 + ?9, 1, ?10)
        ON CONFLICT(source_key_version, source_key_epoch_hash, source_key_hash, claim_type, signal_type)
        DO UPDATE SET
            data_source = ?4,
            source_key_kind = ?5,
            alpha = alpha + ?8,
            beta = beta + ?9,
            update_count = update_count + 1,
            updated_at = ?10",
        params![
            key.source_key_version,
            key.source_key_epoch_hash,
            key.source_key_hash,
            key.data_source,
            key.source_key_kind,
            key.claim_type,
            key.signal_type,
            alpha_delta,
            beta_delta,
            write.now,
        ],
    )?;
    Ok(true)
}

fn record_source_backfill_terminal_skip_delta(
    tx: &ActionDb,
    row: &SourceBackfillFeedbackRow,
    claim: &IntelligenceClaim,
    reason_code: &str,
    now: &str,
) -> Result<(), ClaimError> {
    let key = derive_source_reliability_skip_key(
        &claim.data_source,
        claim.source_ref.as_deref(),
        row.payload_json.as_deref(),
        claim.item_hash.as_deref(),
        &claim.claim_type,
        reason_code,
    )?;
    let delta_id = Uuid::new_v4().to_string();
    tx.conn_ref().execute(
        "INSERT OR IGNORE INTO source_reliability_feedback_deltas (
            id, feedback_id, source_key_version, source_key_epoch_hash,
            source_key_hash, data_source, source_key_kind, claim_type,
            signal_type, effect_kind, alpha_delta, beta_delta, status, applied_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 0.0, 0.0, 'redacted', ?11)",
        params![
            delta_id,
            &row.feedback_id,
            key.source_key_version,
            key.source_key_epoch_hash,
            key.source_key_hash,
            key.data_source,
            key.source_key_kind,
            key.claim_type,
            key.signal_type,
            format!("backfill_skip:{reason_code}"),
            now,
        ],
    )?;
    Ok(())
}

fn apply_subject_inference_delta(
    tx: &ActionDb,
    write: &FeedbackPropagationWrite<'_>,
    subject_hash: &str,
) -> Result<bool, ClaimError> {
    if write.input.action != FeedbackAction::WrongSubject {
        return Ok(false);
    }
    let payload = parse_payload_json(write.input.payload_json.as_deref())?;
    let corrected_subject_ref_hash = payload
        .get("corrected_subject_ref")
        .or_else(|| payload.get("corrected_subject"))
        .map(stable_json_hash)
        .transpose()?;

    let delta_id = Uuid::new_v4().to_string();
    let inserted = tx.conn_ref().execute(
        "INSERT OR IGNORE INTO subject_inference_reliability_deltas (
            id, feedback_id, subject_ref_hash, corrected_subject_ref_hash, claim_type,
            signal_type, alpha_delta, beta_delta, status, applied_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, 'user_feedback', 0.0, 1.0, 'applied', ?6)",
        params![
            delta_id,
            write.feedback_id,
            subject_hash,
            corrected_subject_ref_hash,
            write.claim.claim_type,
            write.now,
        ],
    )?;
    if inserted == 0 {
        return Ok(false);
    }

    tx.conn_ref().execute(
        "INSERT INTO subject_inference_reliability (
            subject_ref_hash, claim_type, signal_type, alpha, beta, update_count, updated_at
        ) VALUES (?1, ?2, 'user_feedback', 1.0, 2.0, 1, ?3)
        ON CONFLICT(subject_ref_hash, claim_type, signal_type)
        DO UPDATE SET
            beta = beta + 1.0,
            update_count = update_count + 1,
            updated_at = ?3",
        params![subject_hash, write.claim.claim_type, write.now],
    )?;
    Ok(true)
}

fn propagation_targets_for_feedback(
    write: &FeedbackPropagationWrite<'_>,
    source_key: &SourceReliabilityKey,
    subject_hash: &str,
) -> Vec<PropagationTarget> {
    let action = write.input.action;
    let subject_key = canonical_subject_storage(write.subject);
    let mut targets = vec![
        target(
            "receipt_surface_update",
            "direct_receipt_rerender",
            SyncClass::BoundedSync,
            PropagationStatus::Completed,
            format!(
                "{}:receipt:{}",
                write.feedback_id,
                direct_surface_suffix(write)
            ),
            json!({"feedback_id": write.feedback_id, "claim_id": write.claim.id}),
            None,
        ),
        target(
            "claim_recompute",
            "targeted_claim_recompute",
            SyncClass::Async,
            PropagationStatus::Pending,
            format!("claim_recompute:{subject_key}:{}", write.claim.claim_type),
            json!({
                "claim_id": write.claim.id,
                "subject": subject_key,
                "claim_type": write.claim.claim_type,
                "chunk_size": 25
            }),
            Some(json!({"chunk_cursor": null, "chunk_size": 25})),
        ),
        target(
            "derived_context",
            "invalidate_build_intelligence_context",
            SyncClass::Async,
            PropagationStatus::Pending,
            format!("context:build_intelligence:{subject_key}"),
            json!({"subject": subject_key, "subject_ref_hash": subject_hash}),
            None,
        ),
        target(
            "derived_context",
            "invalidate_gather_account_context",
            SyncClass::Async,
            PropagationStatus::Pending,
            format!("context:gather_account:{subject_key}"),
            json!({"subject": subject_key, "subject_ref_hash": subject_hash}),
            None,
        ),
        target(
            "salience_surfacing",
            "rerank_bounded_candidates",
            SyncClass::Async,
            PropagationStatus::Pending,
            format!(
                "salience:{subject_key}:{}:{}",
                write.claim.claim_type, write.claim.id
            ),
            json!({
                "claim_id": write.claim.id,
                "subject": subject_key,
                "surface": direct_surface_suffix(write),
                "limit": 25
            }),
            Some(json!({"candidate_limit": 25, "cursor": null})),
        ),
    ];

    if source_reliability_delta(action).is_some() {
        targets.push(target(
            "source_reliability_delta",
            "apply_claim_type_delta",
            SyncClass::Async,
            PropagationStatus::Completed,
            format!(
                "source_reliability:{}:{}:{}",
                source_key.source_key_hash, source_key.claim_type, source_key.signal_type
            ),
            json!({
                "source_key_version": source_key.source_key_version,
                "source_key_epoch_hash": source_key.source_key_epoch_hash,
                "source_key_hash": source_key.source_key_hash,
                "claim_type": source_key.claim_type,
                "signal_type": source_key.signal_type
            }),
            None,
        ));
    }

    match action {
        FeedbackAction::ConfirmCurrent => {
            targets.push(review_queue_target(write, "close_review_item"));
        }
        FeedbackAction::MarkOutdated => {
            targets.push(repair_target(write, "freshness_refresh"));
            targets.push(prep_target(write, "claim_freshness_changed"));
        }
        FeedbackAction::MarkFalse => {
            targets.push(repair_target(write, "contradiction_reconcile"));
            targets.push(target(
                "subject_graph",
                "invalidate_claim_edges",
                SyncClass::Async,
                PropagationStatus::Pending,
                format!("edges:{subject_key}:{}", write.claim.id),
                json!({"claim_id": write.claim.id, "subject": subject_key}),
                None,
            ));
            targets.push(review_queue_target(write, "refresh_reconciliation_queue"));
        }
        FeedbackAction::WrongSubject => {
            targets.push(target(
                "subject_inference_delta",
                "apply_subject_fit_delta",
                SyncClass::Async,
                PropagationStatus::Completed,
                format!(
                    "subject_inference:{subject_hash}:{}",
                    write.claim.claim_type
                ),
                json!({"subject_ref_hash": subject_hash, "claim_type": write.claim.claim_type}),
                None,
            ));
            targets.push(target(
                "subject_graph",
                "invalidate_subject_edges",
                SyncClass::Async,
                PropagationStatus::Pending,
                format!("subject_graph:{subject_key}:{}", write.claim.id),
                json!({"claim_id": write.claim.id, "subject": subject_key}),
                None,
            ));
            targets.push(prep_target(write, "wrong_subject"));
        }
        FeedbackAction::WrongSource => {
            targets.push(repair_target(write, "source_support_repair"));
        }
        FeedbackAction::CannotVerify => {
            targets.push(repair_target(write, "bounded_corroboration"));
            targets.push(review_queue_target(
                write,
                "candidate_if_repair_budget_exceeded",
            ));
        }
        FeedbackAction::NeedsNuance => {
            targets.push(repair_target(write, "superseder_reconcile"));
            targets.push(review_queue_target(
                write,
                "human_review_if_nuance_required",
            ));
        }
        FeedbackAction::SurfaceInappropriate => {
            targets.push(target(
                "surface_policy",
                "apply_named_surface_suppression",
                SyncClass::BoundedSync,
                PropagationStatus::Pending,
                format!(
                    "surface_policy:{}:{}",
                    write.claim.id,
                    direct_surface_suffix(write)
                ),
                json!({"claim_id": write.claim.id, "surface": direct_surface_suffix(write)}),
                None,
            ));
            targets.push(review_queue_target(write, "policy_conflict_review"));
        }
        FeedbackAction::NotRelevantHere => {
            targets.push(target(
                "relevance_ranking",
                "demote_invocation_context",
                SyncClass::BoundedSync,
                PropagationStatus::Completed,
                format!(
                    "relevance:{}:{}",
                    write.claim.id,
                    direct_surface_suffix(write)
                ),
                json!({"claim_id": write.claim.id, "surface": direct_surface_suffix(write)}),
                None,
            ));
        }
        FeedbackAction::MergeIntent => {
            targets.push(target(
                "merge_proposal",
                "persist_merge_candidate",
                SyncClass::BoundedSync,
                PropagationStatus::Completed,
                format!("merge:{}:{}", write.feedback_id, write.claim.id),
                json!({"claim_id": write.claim.id}),
                None,
            ));
            targets.push(review_queue_target(write, "merge_candidate_review"));
        }
    }

    targets
}

fn target(
    target_kind: &'static str,
    operation: &'static str,
    sync_class: SyncClass,
    status: PropagationStatus,
    coalescing_key: String,
    scope_json: serde_json::Value,
    cursor_json: Option<serde_json::Value>,
) -> PropagationTarget {
    PropagationTarget {
        target_kind,
        operation,
        sync_class,
        status,
        coalescing_key,
        scope_json,
        cursor_json,
    }
}

fn repair_target(
    write: &FeedbackPropagationWrite<'_>,
    operation: &'static str,
) -> PropagationTarget {
    target(
        "targeted_repair",
        operation,
        SyncClass::Async,
        PropagationStatus::Pending,
        format!("targeted_repair:{}:{operation}", write.claim.id),
        json!({
            "claim_id": write.claim.id,
            "repair_job_id": write.repair_job_id,
            "repair": repair_action_storage(write.repair)
        }),
        None,
    )
}

fn prep_target(write: &FeedbackPropagationWrite<'_>, reason: &'static str) -> PropagationTarget {
    let subject_key = canonical_subject_storage(write.subject);
    target(
        "prep_regeneration",
        "enqueue_prep_regeneration",
        SyncClass::Async,
        PropagationStatus::Pending,
        format!(
            "prep:{subject_key}:{}",
            write.claim.field_path.as_deref().unwrap_or("*")
        ),
        json!({
            "subject": subject_key,
            "claim_id": write.claim.id,
            "field_path": write.claim.field_path,
            "reason": reason
        }),
        Some(json!({"cursor": null, "page_size": 25})),
    )
}

fn review_queue_target(
    write: &FeedbackPropagationWrite<'_>,
    operation: &'static str,
) -> PropagationTarget {
    target(
        "review_queue",
        operation,
        SyncClass::Async,
        PropagationStatus::Pending,
        format!("review:{}:{operation}", write.claim.id),
        json!({"claim_id": write.claim.id}),
        None,
    )
}

fn coalesced_target_for_feedback(
    write: &FeedbackPropagationWrite<'_>,
    original_count: usize,
    subject_hash: &str,
) -> PropagationTarget {
    target(
        "coalesced_batch",
        "bounded_feedback_propagation_batch",
        SyncClass::Async,
        PropagationStatus::Coalesced,
        format!("feedback_batch:{}:{subject_hash}", write.feedback_id),
        json!({
            "feedback_id": write.feedback_id,
            "claim_id": write.claim.id,
            "original_target_count": original_count,
            "max_logical_targets": DEFAULT_MAX_PROPAGATION_ROWS
        }),
        Some(json!({"cursor": null, "page_size": DEFAULT_MAX_PROPAGATION_ROWS})),
    )
}

fn insert_propagation_target(
    tx: &ActionDb,
    feedback_id: &str,
    action: FeedbackAction,
    target: &PropagationTarget,
    now: &str,
) -> Result<(), ClaimError> {
    let job_id = Uuid::new_v4().to_string();
    let scope_json = stable_json_string(&target.scope_json)?;
    let cursor_json = target
        .cursor_json
        .as_ref()
        .map(stable_json_string)
        .transpose()?;
    if let Some(existing_job_id) = tx
        .conn_ref()
        .query_row(
            "SELECT id
               FROM claim_feedback_propagation_jobs
              WHERE target_kind = ?1
                AND operation = ?2
                AND coalescing_key = ?3
                AND sync_class = ?4
                AND status IN ('pending', 'running')
              ORDER BY created_at ASC
              LIMIT 1",
            params![
                target.target_kind,
                target.operation,
                target.coalescing_key,
                target.sync_class.as_str(),
            ],
            |row| row.get::<_, String>(0),
        )
        .optional()?
    {
        tx.conn_ref().execute(
            "UPDATE claim_feedback_propagation_jobs
                SET next_run_at = CASE
                        WHEN status = 'pending' AND datetime(next_run_at) > datetime(?2) THEN ?2
                        ELSE next_run_at
                    END,
                    updated_at = ?2
              WHERE id = ?1",
            params![&existing_job_id, now],
        )?;
        let outcome_id = Uuid::new_v4().to_string();
        tx.conn_ref().execute(
            "INSERT INTO claim_feedback_propagation_outcomes (
                id, job_id, feedback_id, target_kind, operation, sync_class, status, reason_code
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'coalesced', 'coalesced_into_existing_target')",
            params![
                outcome_id,
                existing_job_id,
                feedback_id,
                target.target_kind,
                target.operation,
                target.sync_class.as_str(),
            ],
        )?;
        return Ok(());
    }
    tx.conn_ref().execute(
        "INSERT OR IGNORE INTO claim_feedback_propagation_jobs (
            id, feedback_id, action, target_kind, operation, sync_class,
            status, coalescing_key, scope_json, cursor_json, next_run_at,
            created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11, ?11)",
        params![
            job_id,
            feedback_id,
            action.as_str(),
            target.target_kind,
            target.operation,
            target.sync_class.as_str(),
            target.status.as_str(),
            target.coalescing_key,
            scope_json,
            cursor_json,
            now,
        ],
    )?;
    let stored_job_id = tx
        .conn_ref()
        .query_row(
            "SELECT id
               FROM claim_feedback_propagation_jobs
              WHERE feedback_id = ?1
                AND target_kind = ?2
                AND operation = ?3
                AND coalescing_key = ?4
                AND sync_class = ?5",
            params![
                feedback_id,
                target.target_kind,
                target.operation,
                target.coalescing_key,
                target.sync_class.as_str(),
            ],
            |row| row.get::<_, String>(0),
        )
        .map_err(ClaimError::Rusqlite)?;
    let outcome_id = Uuid::new_v4().to_string();
    tx.conn_ref().execute(
        "INSERT INTO claim_feedback_propagation_outcomes (
            id, job_id, feedback_id, target_kind, operation, sync_class, status, reason_code
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            outcome_id,
            stored_job_id,
            feedback_id,
            target.target_kind,
            target.operation,
            target.sync_class.as_str(),
            target.status.as_str(),
            initial_reason_code(target.status),
        ],
    )?;
    Ok(())
}

fn derive_source_reliability_key(
    data_source: &str,
    source_ref: Option<&str>,
    payload_json: Option<&str>,
    item_hash: Option<&str>,
    subject: &SubjectRef,
    claim_type: &str,
    signal_type: &str,
) -> Result<SourceReliabilityKey, ClaimError> {
    let payload = parse_payload_json(payload_json)?;
    let source_content_hash = payload
        .get("source_content_hash")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());

    let (source_key_kind, material) =
        if let Some(source_ref) = source_ref.map(str::trim).filter(|value| !value.is_empty()) {
            (
                "source_ref".to_string(),
                format!("data_source={data_source}\nsource_ref={source_ref}"),
            )
        } else if let Some(source_content_hash) = source_content_hash {
            (
                "source_content_hash".to_string(),
                format!("data_source={data_source}\nsource_content_hash={source_content_hash}"),
            )
        } else if let Some(item_hash) = item_hash.map(str::trim).filter(|value| !value.is_empty()) {
            (
                "item_hash".to_string(),
                format!("data_source={data_source}\nitem_hash={item_hash}"),
            )
        } else {
            let (subject_kind, _) = subject_kind_id(subject);
            (
                "legacy_broad_fallback".to_string(),
                format!(
                "data_source={data_source}\nclaim_type={claim_type}\nsubject_type={subject_kind}"
            ),
            )
        };

    let source_key_epoch_hash = pii_safe_hash(
        "source_epoch",
        SOURCE_KEY_EPOCH_DOMAIN,
        &[&SOURCE_KEY_VERSION.to_string()],
    )?;
    let source_key_hash = pii_safe_hash(
        "source",
        SOURCE_KEY_HASH_DOMAIN,
        &[&SOURCE_KEY_VERSION.to_string(), &source_key_kind, &material],
    )?;

    Ok(SourceReliabilityKey {
        data_source: data_source.to_string(),
        source_key_kind,
        source_key_version: SOURCE_KEY_VERSION,
        source_key_epoch_hash,
        source_key_hash,
        claim_type: claim_type.to_string(),
        signal_type: signal_type.to_string(),
    })
}

fn derive_source_reliability_skip_key(
    data_source: &str,
    source_ref: Option<&str>,
    payload_json: Option<&str>,
    item_hash: Option<&str>,
    claim_type: &str,
    reason_code: &str,
) -> Result<SourceReliabilityKey, ClaimError> {
    let source_content_hash = payload_source_content_hash(payload_json);
    let (source_key_kind, material) =
        if let Some(source_ref) = source_ref.map(str::trim).filter(|value| !value.is_empty()) {
            (
                "source_ref".to_string(),
                format!("data_source={data_source}\nsource_ref={source_ref}"),
            )
        } else if let Some(source_content_hash) = source_content_hash.as_deref() {
            (
                "source_content_hash".to_string(),
                format!("data_source={data_source}\nsource_content_hash={source_content_hash}"),
            )
        } else if let Some(item_hash) = item_hash.map(str::trim).filter(|value| !value.is_empty()) {
            (
                "item_hash".to_string(),
                format!("data_source={data_source}\nitem_hash={item_hash}"),
            )
        } else {
            (
                "invalid_historical_row".to_string(),
                format!(
                    "data_source={data_source}\nclaim_type={claim_type}\nreason_code={reason_code}"
                ),
            )
        };

    let source_key_epoch_hash = source_key_epoch_hash()?;
    let source_key_hash = pii_safe_hash(
        "source",
        SOURCE_KEY_HASH_DOMAIN,
        &[&SOURCE_KEY_VERSION.to_string(), &source_key_kind, &material],
    )?;

    Ok(SourceReliabilityKey {
        data_source: data_source.to_string(),
        source_key_kind,
        source_key_version: SOURCE_KEY_VERSION,
        source_key_epoch_hash,
        source_key_hash,
        claim_type: claim_type.to_string(),
        signal_type: SOURCE_KEY_SIGNAL_TYPE.to_string(),
    })
}

fn source_key_epoch_hash() -> Result<String, ClaimError> {
    pii_safe_hash(
        "source_epoch",
        SOURCE_KEY_EPOCH_DOMAIN,
        &[&SOURCE_KEY_VERSION.to_string()],
    )
}

fn source_reliability_delta(action: FeedbackAction) -> Option<(&'static str, f64, f64)> {
    match action {
        FeedbackAction::ConfirmCurrent => Some(("user_corroboration", 1.0, 0.0)),
        FeedbackAction::WrongSource => Some(("wrong_source", 0.0, 1.0)),
        FeedbackAction::MarkFalse => Some(("false_source_support", 0.0, 0.25)),
        FeedbackAction::MarkOutdated
        | FeedbackAction::WrongSubject
        | FeedbackAction::CannotVerify
        | FeedbackAction::NeedsNuance
        | FeedbackAction::SurfaceInappropriate
        | FeedbackAction::NotRelevantHere
        | FeedbackAction::MergeIntent => None,
    }
}

fn feedback_action_from_storage(raw: &str) -> Option<FeedbackAction> {
    match raw {
        "confirm_current" => Some(FeedbackAction::ConfirmCurrent),
        "mark_outdated" => Some(FeedbackAction::MarkOutdated),
        "mark_false" => Some(FeedbackAction::MarkFalse),
        "wrong_subject" => Some(FeedbackAction::WrongSubject),
        "wrong_source" => Some(FeedbackAction::WrongSource),
        "cannot_verify" => Some(FeedbackAction::CannotVerify),
        "needs_nuance" => Some(FeedbackAction::NeedsNuance),
        "surface_inappropriate" => Some(FeedbackAction::SurfaceInappropriate),
        "not_relevant_here" => Some(FeedbackAction::NotRelevantHere),
        "merge_intent" => Some(FeedbackAction::MergeIntent),
        _ => None,
    }
}

fn parse_payload_json(payload_json: Option<&str>) -> Result<serde_json::Value, ClaimError> {
    let Some(raw) = payload_json
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(json!({}));
    };
    Ok(serde_json::from_str(raw)?)
}

fn payload_source_content_hash(payload_json: Option<&str>) -> Option<String> {
    let payload = parse_payload_json(payload_json).ok()?;
    payload
        .get("source_content_hash")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn source_content_hash_payload_for_claim(claim: &IntelligenceClaim) -> Option<String> {
    let metadata = claim
        .metadata_json
        .as_deref()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())?;
    let source_content_hash = metadata
        .get("source_content_hash")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    Some(json!({ "source_content_hash": source_content_hash }).to_string())
}

fn stable_json_string(value: &serde_json::Value) -> Result<String, ClaimError> {
    Ok(serde_json::to_string(value)?)
}

fn stable_json_hash(value: &serde_json::Value) -> Result<String, ClaimError> {
    let raw = stable_json_string(value)?;
    Ok(format!(
        "sha256:{}",
        hex::encode(Sha256::digest(raw.as_bytes()))
    ))
}

fn enum_storage<T: Serialize>(value: &T) -> Result<String, ClaimError> {
    Ok(serde_json::to_string(value)?.trim_matches('"').to_string())
}

fn target_receipt_for_payload(payload: &serde_json::Value) -> Result<Option<String>, ClaimError> {
    payload
        .get("target_receipt")
        .map(stable_json_string)
        .transpose()
}

fn surface_for_payload(payload: &serde_json::Value) -> String {
    payload
        .get("surface")
        .or_else(|| payload.get("direct_surface"))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown")
        .to_string()
}

fn direct_surface_suffix(write: &FeedbackPropagationWrite<'_>) -> String {
    parse_payload_json(write.input.payload_json.as_deref())
        .ok()
        .map(|payload| surface_for_payload(&payload))
        .filter(|surface| surface != "unknown")
        .unwrap_or_else(|| "unknown".to_string())
}

fn subject_kind_id(subject: &SubjectRef) -> (&'static str, Option<&str>) {
    match subject {
        SubjectRef::Account { id } => ("account", Some(id.as_str())),
        SubjectRef::Meeting { id } => ("meeting", Some(id.as_str())),
        SubjectRef::Person { id } => ("person", Some(id.as_str())),
        SubjectRef::Project { id } => ("project", Some(id.as_str())),
        SubjectRef::Email { id } => ("email", Some(id.as_str())),
        SubjectRef::Action { id } => ("action", Some(id.as_str())),
        SubjectRef::Multi(_) => ("multi", None),
        SubjectRef::Global => ("global", None),
    }
}

fn canonical_subject_storage(subject: &SubjectRef) -> String {
    match subject {
        SubjectRef::Account { id } => json!({"kind": "account", "id": id}).to_string(),
        SubjectRef::Meeting { id } => json!({"kind": "meeting", "id": id}).to_string(),
        SubjectRef::Person { id } => json!({"kind": "person", "id": id}).to_string(),
        SubjectRef::Project { id } => json!({"kind": "project", "id": id}).to_string(),
        SubjectRef::Email { id } => json!({"kind": "email", "id": id}).to_string(),
        SubjectRef::Action { id } => json!({"kind": "action", "id": id}).to_string(),
        SubjectRef::Multi(subjects) => {
            let refs: Vec<serde_json::Value> = subjects
                .iter()
                .map(|subject| {
                    serde_json::from_str(&canonical_subject_storage(subject))
                        .unwrap_or_else(|_| json!({"kind":"unknown"}))
                })
                .collect();
            json!({"kind": "multi", "subjects": refs}).to_string()
        }
        SubjectRef::Global => json!({"kind": "global"}).to_string(),
    }
}

fn subject_from_kind_id(
    subject_type: &str,
    subject_ref_json: &str,
) -> Result<SubjectRef, ClaimError> {
    let id = serde_json::from_str::<serde_json::Value>(subject_ref_json)
        .ok()
        .and_then(|value| {
            value
                .get("id")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        });
    let subject_type = subject_type.trim().to_ascii_lowercase();
    match (subject_type.as_str(), id) {
        ("account" | "accounts", Some(id)) => Ok(SubjectRef::Account { id }),
        ("meeting" | "meetings", Some(id)) => Ok(SubjectRef::Meeting { id }),
        ("person" | "people", Some(id)) => Ok(SubjectRef::Person { id }),
        ("project" | "projects", Some(id)) => Ok(SubjectRef::Project { id }),
        ("email" | "emails", Some(id)) => Ok(SubjectRef::Email { id }),
        ("action" | "actions", Some(id)) => Ok(SubjectRef::Action { id }),
        ("global", _) => Ok(SubjectRef::Global),
        _ => {
            let value: serde_json::Value = serde_json::from_str(subject_ref_json)?;
            crate::services::claims::subject_ref_from_json(&value)
        }
    }
}

fn repair_action_storage(repair: RepairAction) -> &'static str {
    match repair {
        RepairAction::None => "none",
        RepairAction::FreshnessRefresh => "freshness_refresh",
        RepairAction::ContradictionReconcile => "contradiction_reconcile",
        RepairAction::SubjectFitRepair => "subject_fit_repair",
        RepairAction::SourceSupportRepair => "source_support_repair",
        RepairAction::BoundedCorroboration => "bounded_corroboration",
        RepairAction::PolicyRepair => "policy_repair",
    }
}

fn initial_reason_code(status: PropagationStatus) -> Option<&'static str> {
    match status {
        PropagationStatus::Pending => Some("queued"),
        PropagationStatus::Completed => Some("completed_inline"),
        PropagationStatus::Coalesced => Some("coalesced_bounded_batch"),
    }
}

fn sqlite_missing_w4_reliability_table(error: &rusqlite::Error) -> bool {
    matches!(error, rusqlite::Error::SqliteFailure(_, Some(message)) if message.contains("source_claim_type_reliability"))
}

fn pii_safe_hash(prefix: &str, domain: &str, components: &[&str]) -> Result<String, ClaimError> {
    #[cfg(test)]
    {
        return Ok(crate::db::local_db_keyed_audit_tag_for_tests(
            "w4-claim-feedback-propagation-test-secret",
            prefix,
            domain,
            components,
        ));
    }

    #[cfg(not(test))]
    {
        crate::db::local_db_keyed_audit_tag(prefix, domain, components)
            .map_err(|error| ClaimError::Transaction(format!("derive PII-safe hash: {error}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use rusqlite::params;

    use crate::db::claims::{ClaimSensitivity, TemporalScope};
    use crate::db::test_utils::test_db;
    use crate::services::claims::{commit_claim, ClaimProposal, CommittedClaim};
    use crate::services::context::{ExternalClients, FixedClock, SeedableRng};
    use crate::services::recommendations::contracts::{
        RecommendationDraft, RecommendedAction, SalienceScore,
    };
    use crate::services::recommendations::recommendation::metadata_envelope;

    fn claim(source_ref: Option<&str>) -> IntelligenceClaim {
        IntelligenceClaim {
            id: "claim-1".to_string(),
            subject_ref: r#"{"kind":"account","id":"acct-1"}"#.to_string(),
            claim_type: "risk".to_string(),
            field_path: Some("summary.risk".to_string()),
            topic_key: None,
            text: "Account risk is elevated".to_string(),
            dedup_key: "dedup".to_string(),
            item_hash: Some("item-hash-1".to_string()),
            actor: "system".to_string(),
            data_source: "workspace_file".to_string(),
            source_ref: source_ref.map(str::to_string),
            source_asof: Some("2026-06-05T00:00:00Z".to_string()),
            observed_at: "2026-06-05T00:00:00Z".to_string(),
            created_at: "2026-06-05T00:00:00Z".to_string(),
            provenance_json: "{}".to_string(),
            metadata_json: None,
            claim_state: crate::db::claims::ClaimState::Active,
            surfacing_state: crate::db::claims::SurfacingState::Active,
            demotion_reason: None,
            reactivated_at: None,
            retraction_reason: None,
            expires_at: None,
            superseded_by: None,
            trust_score: None,
            trust_computed_at: None,
            trust_version: None,
            thread_id: None,
            temporal_scope: crate::db::claims::TemporalScope::State,
            sensitivity: crate::db::claims::ClaimSensitivity::Internal,
            verification_state: crate::abilities::feedback::ClaimVerificationState::Active,
            verification_reason: None,
            needs_user_decision_at: None,
            claim_version: 1,
        }
    }

    fn recommendation_claim_proposal(source_ref: &str) -> ClaimProposal {
        let observed_at = Utc.with_ymd_and_hms(2026, 6, 5, 11, 0, 0).unwrap();
        let draft = RecommendationDraft {
            subject: abilities_runtime::abilities::provenance::subject::SubjectRef::Account(
                "acct-reclaim".to_string(),
            ),
            recommended_action: RecommendedAction::Custom {
                action_kind: "retryRegression".to_string(),
                payload: json!({"fixture": "salience_surfacing_retry"}),
            },
            evidence: Vec::new(),
            provenance_json: "{}".to_string(),
            source_ref: Some(source_ref.to_string()),
            source_asof: Some(observed_at),
            observed_at,
            text: "Review the account before the next customer call".to_string(),
            salience: SalienceScore {
                total: 0.91,
                factors: Vec::new(),
            },
        };
        ClaimProposal {
            id: None,
            expected_claim_version: None,
            subject_ref: r#"{"kind":"account","id":"acct-reclaim"}"#.to_string(),
            claim_type: "recommendation".to_string(),
            field_path: Some("recommendation.custom.retryRegression".to_string()),
            topic_key: Some("custom.retryRegression".to_string()),
            text: draft.text.clone(),
            actor: "agent:test".to_string(),
            data_source: "recommendation".to_string(),
            source_ref: draft.source_ref.clone(),
            source_asof: draft.source_asof.map(|dt| dt.to_rfc3339()),
            observed_at: draft.observed_at.to_rfc3339(),
            provenance_json: draft.provenance_json.clone(),
            metadata_json: Some(
                serde_json::to_string(&metadata_envelope(&draft))
                    .expect("serialize recommendation metadata"),
            ),
            thread_id: None,
            temporal_scope: None,
            sensitivity: None,
            supersedes: None,
            tombstone: None,
        }
    }

    #[test]
    fn source_ref_takes_precedence_over_payload_source_content_hash() {
        let payload = Some(r#"{"source_content_hash":"content-hash"}"#);
        let key = derive_source_reliability_key(
            "workspace_file",
            Some("workspace_file:raw-source"),
            payload,
            Some("item-hash"),
            &SubjectRef::Account {
                id: "acct-1".to_string(),
            },
            "risk",
            "user_feedback",
        )
        .expect("key");

        assert_eq!(key.source_key_kind, "source_ref");
        assert_eq!(key.source_key_version, SOURCE_KEY_VERSION);
        assert_eq!(key.claim_type, "risk");
    }

    #[test]
    fn payload_source_content_hash_beats_item_hash_when_source_ref_missing() {
        let key = derive_source_reliability_key(
            "workspace_file",
            None,
            Some(r#"{"source_content_hash":"content-hash"}"#),
            Some("item-hash"),
            &SubjectRef::Account {
                id: "acct-1".to_string(),
            },
            "risk",
            "user_feedback",
        )
        .expect("key");

        assert_eq!(key.source_key_kind, "source_content_hash");
    }

    #[test]
    fn propagation_map_covers_all_actions_with_bounded_rows() {
        let claim = claim(Some("source-1"));
        let subject = SubjectRef::Account {
            id: "acct-1".to_string(),
        };
        for action in [
            FeedbackAction::ConfirmCurrent,
            FeedbackAction::MarkOutdated,
            FeedbackAction::MarkFalse,
            FeedbackAction::WrongSubject,
            FeedbackAction::WrongSource,
            FeedbackAction::CannotVerify,
            FeedbackAction::NeedsNuance,
            FeedbackAction::SurfaceInappropriate,
            FeedbackAction::NotRelevantHere,
            FeedbackAction::MergeIntent,
        ] {
            let input = ClaimFeedbackInput {
                claim_id: claim.id.clone(),
                action,
                actor: "user".to_string(),
                actor_id: None,
                payload_json: payload_for_action(action),
            };
            let write = FeedbackPropagationWrite {
                feedback_id: "feedback-1",
                claim: &claim,
                input: &input,
                subject: &subject,
                repair: RepairAction::None,
                repair_job_id: None,
                claim_file_apply_key: None,
                now: "2026-06-05T00:00:00Z",
            };
            let source_key =
                derive_source_reliability_key_for_claim(&claim, "account", SOURCE_KEY_SIGNAL_TYPE)
                    .expect("source key");
            let targets = propagation_targets_for_feedback(&write, &source_key, "subject_hash");
            assert!(
                !targets.is_empty(),
                "{} must have propagation targets",
                action.as_str()
            );
            assert!(
                targets.len() <= DEFAULT_MAX_PROPAGATION_ROWS,
                "{} fan-out must stay bounded",
                action.as_str()
            );
        }
    }

    #[test]
    fn expired_running_propagation_job_is_reclaimed_after_restart() {
        let db = test_db();
        let seed_clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 6, 5, 11, 0, 0).unwrap());
        let seed_rng = SeedableRng::new(8);
        let seed_external = ExternalClients::default();
        let seed_ctx = ServiceContext::test_live(&seed_clock, &seed_rng, &seed_external)
            .with_actor("user:test");
        let committed = commit_claim(
            &seed_ctx,
            &db,
            ClaimProposal {
                id: None,
                expected_claim_version: None,
                subject_ref: r#"{"kind":"account","id":"acct-reclaim"}"#.to_string(),
                claim_type: "risk".to_string(),
                field_path: Some("summary.risk".to_string()),
                topic_key: None,
                text: "Synthetic risk claim".to_string(),
                actor: "agent:test".to_string(),
                data_source: "unit_test".to_string(),
                source_ref: None,
                source_asof: Some("2026-06-05T11:00:00Z".to_string()),
                observed_at: "2026-06-05T11:00:00Z".to_string(),
                provenance_json: "{}".to_string(),
                metadata_json: None,
                thread_id: None,
                temporal_scope: Some(TemporalScope::State),
                sensitivity: Some(ClaimSensitivity::Internal),
                supersedes: None,
                tombstone: None,
            },
        )
        .expect("seed claim through claim service");
        let claim_id = match committed {
            CommittedClaim::Inserted { claim } => claim.id,
            other => panic!("expected inserted seed claim, got {other:?}"),
        };
        db.conn_ref()
            .execute(
                "INSERT INTO claim_feedback (
                    id, claim_id, feedback_type, actor, submitted_at
                 ) VALUES (
                    'feedback-reclaim', ?1, 'wrong_source', 'user',
                    '2026-06-05T11:00:00+00:00'
                 )",
                params![claim_id],
            )
            .expect("seed feedback");
        let scope_json = json!({
            "subject": "{\"kind\":\"account\",\"id\":\"acct-reclaim\"}"
        })
        .to_string();
        for (job_id, retry_count, max_attempts) in [
            ("job-reclaimable", 1_i64, 5_i64),
            ("job-exhausted", 5_i64, 5_i64),
        ] {
            db.conn_ref()
                .execute(
                    "INSERT INTO claim_feedback_propagation_jobs (
                        id, feedback_id, action, target_kind, operation, sync_class,
                        status, coalescing_key, scope_json, enqueue_run_id,
                        retry_count, max_attempts, created_at, updated_at
                     ) VALUES (
                        ?1, 'feedback-reclaim', 'wrong_source', 'derived_context',
                        'refresh_derived_context', 'async', 'running', ?2, ?3,
                        'previous-worker', ?4, ?5,
                        '2026-06-05T11:00:00+00:00',
                        '2026-06-05T11:50:00+00:00'
                     )",
                    params![
                        job_id,
                        format!("reclaim:{job_id}"),
                        &scope_json,
                        retry_count,
                        max_attempts
                    ],
                )
                .expect("seed propagation job");
        }

        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 6, 5, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(9);
        let external = ExternalClients::default();
        let ctx = ServiceContext::test_live(&clock, &rng, &external).with_actor("system:worker");
        let job = claim_next_feedback_propagation_job(&ctx, &db, "worker-after-restart")
            .expect("claim next job")
            .expect("expired running job should be reclaimed");

        assert_eq!(job.id, "job-reclaimable");
        assert_eq!(job.retry_count, 2);
        let (reclaimed_status, exhausted_status): (String, String) = db
            .conn_ref()
            .query_row(
                "SELECT
                    (SELECT status FROM claim_feedback_propagation_jobs WHERE id = 'job-reclaimable'),
                    (SELECT status FROM claim_feedback_propagation_jobs WHERE id = 'job-exhausted')",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read reclaimed statuses");
        assert_eq!(reclaimed_status, "running");
        assert_eq!(exhausted_status, "dead_lettered");
    }

    #[test]
    fn failed_propagation_job_schedules_due_retry_without_immediate_reclaim() {
        let db = test_db();
        let seed_clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 6, 5, 11, 0, 0).unwrap());
        let seed_rng = SeedableRng::new(28);
        let seed_external = ExternalClients::default();
        let seed_ctx = ServiceContext::test_live(&seed_clock, &seed_rng, &seed_external)
            .with_actor("system:test");
        let committed = commit_claim(
            &seed_ctx,
            &db,
            recommendation_claim_proposal("recommendation-source-delayed-retry"),
        )
        .expect("commit recommendation claim");
        let claim_id = match committed {
            CommittedClaim::Inserted { claim } => claim.id,
            other => panic!("expected inserted recommendation claim, got {other:?}"),
        };
        db.conn_ref()
            .execute(
                "INSERT INTO claim_feedback (
                    id, claim_id, feedback_type, actor, submitted_at
                 ) VALUES (
                    'feedback-delayed-retry', ?1, 'confirm_current', 'user',
                    '2026-06-05T11:00:00+00:00'
                 )",
                params![&claim_id],
            )
            .expect("seed feedback");
        db.conn_ref()
            .execute(
                "INSERT INTO claim_feedback_propagation_jobs (
                    id, feedback_id, action, target_kind, operation, sync_class,
                    status, coalescing_key, scope_json, enqueue_run_id,
                    retry_count, max_attempts, next_run_at, created_at, updated_at
                 ) VALUES (
                    'job-delayed-retry', 'feedback-delayed-retry', 'confirm_current',
                    'derived_context', 'refresh_derived_context', 'async',
                    'running', 'delayed:retry', '{}', 'worker-before-failure',
                    1, 5, '2026-06-05T12:00:00+00:00',
                    '2026-06-05T11:00:00+00:00',
                    '2026-06-05T12:00:00+00:00'
                 )",
                [],
            )
            .expect("seed running propagation job");

        let fail_clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 6, 5, 12, 0, 0).unwrap());
        let fail_rng = SeedableRng::new(29);
        let fail_external = ExternalClients::default();
        let fail_ctx = ServiceContext::test_live(&fail_clock, &fail_rng, &fail_external)
            .with_actor("system:worker");
        let job = read_feedback_propagation_job(&db, "job-delayed-retry")
            .expect("read job")
            .expect("job exists");
        let outcome =
            mark_feedback_propagation_job_failed(&fail_ctx, &db, &job, "transient_dependency")
                .expect("schedule retry");
        assert!(matches!(
            outcome,
            FeedbackPropagationProcessOutcome::RetryScheduled { ref job_id }
                if job_id == "job-delayed-retry"
        ));
        let (status, next_run_at): (String, String) = db
            .conn_ref()
            .query_row(
                "SELECT status, next_run_at
                   FROM claim_feedback_propagation_jobs
                  WHERE id = 'job-delayed-retry'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read scheduled retry");
        assert_eq!(status, "pending");
        assert_eq!(
            next_run_at,
            Utc.with_ymd_and_hms(2026, 6, 5, 12, 0, 1)
                .unwrap()
                .to_rfc3339()
        );

        let immediate = claim_next_feedback_propagation_job(&fail_ctx, &db, "immediate-worker")
            .expect("check immediate claim");
        assert!(
            immediate.is_none(),
            "scheduled retry must not be claimable before next_run_at"
        );

        let due_clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 6, 5, 12, 0, 1).unwrap());
        let due_rng = SeedableRng::new(30);
        let due_external = ExternalClients::default();
        let due_ctx = ServiceContext::test_live(&due_clock, &due_rng, &due_external)
            .with_actor("system:worker");
        let claimed = claim_next_feedback_propagation_job(&due_ctx, &db, "due-worker")
            .expect("claim due retry")
            .expect("retry should be due");
        assert_eq!(claimed.id, "job-delayed-retry");
        assert_eq!(claimed.retry_count, 2);
    }

    #[test]
    fn w4_salience_surfacing_retry_is_idempotent_after_crash_window() {
        let db = test_db();
        let seed_clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 6, 5, 11, 0, 0).unwrap());
        let seed_rng = SeedableRng::new(18);
        let seed_external = ExternalClients::default();
        let seed_ctx = ServiceContext::test_live(&seed_clock, &seed_rng, &seed_external)
            .with_actor("system:test");
        let committed = commit_claim(
            &seed_ctx,
            &db,
            recommendation_claim_proposal("recommendation-source-retry"),
        )
        .expect("commit recommendation claim");
        let claim_id = match committed {
            CommittedClaim::Inserted { claim } => claim.id,
            other => panic!("expected inserted recommendation claim, got {other:?}"),
        };

        db.conn_ref()
            .execute(
                "INSERT INTO claim_feedback (
                    id, claim_id, feedback_type, actor, submitted_at
                 ) VALUES (
                    'feedback-salience-retry', ?1, 'confirm_current', 'user',
                    '2026-06-05T11:00:00+00:00'
                 )",
                params![&claim_id],
            )
            .expect("seed feedback");
        let scope_json = json!({
            "claim_id": claim_id,
            "surface": "briefing"
        })
        .to_string();
        db.conn_ref()
            .execute(
                "INSERT INTO claim_feedback_propagation_jobs (
                    id, feedback_id, action, target_kind, operation, sync_class,
                    status, coalescing_key, scope_json, enqueue_run_id,
                    retry_count, max_attempts, created_at, updated_at
                 ) VALUES (
                    'job-salience-retry', 'feedback-salience-retry', 'confirm_current',
                    'salience_surfacing', 'rerank_bounded_candidates', 'async',
                    'running', 'salience:retry', ?1, 'previous-worker', 1, 5,
                    '2026-06-05T11:00:00+00:00',
                    '2026-06-05T11:50:00+00:00'
                 )",
                params![&scope_json],
            )
            .expect("seed running propagation job");

        let crash_clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 6, 5, 11, 55, 0).unwrap());
        let crash_rng = SeedableRng::new(19);
        let crash_external = ExternalClients::default();
        let crash_ctx = ServiceContext::test_live(&crash_clock, &crash_rng, &crash_external)
            .with_actor("system:worker");
        let job = read_feedback_propagation_job(&db, "job-salience-retry")
            .expect("read job")
            .expect("job exists");
        run_feedback_propagation_job(&crash_ctx, &db, &job)
            .expect("side effects commit before simulated completion crash");

        let retry_clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 6, 5, 12, 5, 0).unwrap());
        let retry_rng = SeedableRng::new(20);
        let retry_external = ExternalClients::default();
        let retry_ctx = ServiceContext::test_live(&retry_clock, &retry_rng, &retry_external)
            .with_actor("system:worker");
        let outcome = process_one_feedback_propagation_job(&retry_ctx, &db, "retry-worker")
            .expect("retry reclaimed job");
        let (job_status, failure_reason): (String, Option<String>) = db
            .conn_ref()
            .query_row(
                "SELECT status, failure_reason_code
                   FROM claim_feedback_propagation_jobs
                  WHERE id = 'job-salience-retry'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read retry job status");
        assert!(matches!(
            outcome,
            FeedbackPropagationProcessOutcome::Completed { ref job_id }
                if job_id == "job-salience-retry"
        ), "unexpected retry outcome: {outcome:?}; status={job_status}; failure={failure_reason:?}");

        let (decision_rows, idempotency_keys, stable_eval_ids, stable_factors): (
            i64,
            i64,
            i64,
            i64,
        ) = db
            .conn_ref()
            .query_row(
                "SELECT
                    (SELECT COUNT(*)
                       FROM surfacing_decisions
                      WHERE source_signal_id LIKE 'feedback-propagation:job-salience-retry:%'),
                    (SELECT COUNT(DISTINCT idempotency_key)
                       FROM surfacing_decisions
                      WHERE source_signal_id LIKE 'feedback-propagation:job-salience-retry:%'),
                    (SELECT COUNT(DISTINCT evaluation_id)
                       FROM salience_factors
                      WHERE evaluation_id LIKE 'salience-eval-feedback-%'),
                    (SELECT COUNT(*)
                       FROM salience_factors
                      WHERE evaluation_id LIKE 'salience-eval-feedback-%')",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("count retry side effects");
        assert_eq!(decision_rows, 1);
        assert_eq!(idempotency_keys, 1);
        assert_eq!(stable_eval_ids, 1);
        assert!(stable_factors > 0);
    }

    fn payload_for_action(action: FeedbackAction) -> Option<String> {
        let value = match action {
            FeedbackAction::WrongSource => json!({"source_ref": "source-1"}),
            FeedbackAction::NeedsNuance => json!({"corrected_text": "More nuanced text"}),
            FeedbackAction::SurfaceInappropriate => json!({"surface": "entity_detail"}),
            FeedbackAction::NotRelevantHere => json!({"invocation_id": "invocation-1"}),
            FeedbackAction::MergeIntent => {
                json!({"merge_target": {"kind": "account", "id": "acct-2"}})
            }
            _ => json!({}),
        };
        Some(value.to_string())
    }
}
