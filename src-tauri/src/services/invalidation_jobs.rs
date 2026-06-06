use std::sync::Arc;

use serde_json::json;

use crate::db::invalidation_jobs::{
    claim_recompute_coalescing_key, claim_recompute_input_hash, EnqueueInvalidationJob,
    InvalidationJob, InvalidationQueueBounds, JobFailureDisposition, TerminalizationOutcome,
    DEFAULT_QUEUE_PENDING_CAP, KIND_CLAIM_RECOMPUTE,
};
use crate::db::ActionDb;
use crate::db_service::DbAccessError;
use crate::services::context::ServiceContext;
use crate::state::AppState;

const STARTUP_DRAIN_LIMIT: usize = 100;
const TARGETED_REPAIR_DRAIN_LIMIT: usize = 100;
const CLAIM_RECOMPUTE_IDLE_POLL_MS: u64 = 250;
const CLAIM_RECOMPUTE_ERROR_POLL_MS: u64 = 2_000;
const TARGETED_REPAIR_IDLE_POLL_MS: u64 = 250;
const TARGETED_REPAIR_ERROR_POLL_MS: u64 = 2_000;
const QUEUE_PENDING_CAP_ENV: &str = "DAILYOS_INVALIDATION_JOBS_PENDING_CAP";
const CLAIM_RECOMPUTE_SYNC_CLAIM_CAP_ENV: &str = "DAILYOS_CLAIM_RECOMPUTE_SYNC_CLAIM_CAP";
const DEFAULT_CLAIM_RECOMPUTE_SYNC_CLAIM_CAP: usize = 25;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidationJobQueueConfig {
    pub pending_cap: i64,
}

impl InvalidationJobQueueConfig {
    pub fn from_env() -> Self {
        let pending_cap = std::env::var(QUEUE_PENDING_CAP_ENV)
            .ok()
            .and_then(|raw| raw.parse::<i64>().ok())
            .filter(|cap| *cap > 0)
            .unwrap_or(DEFAULT_QUEUE_PENDING_CAP);
        Self { pending_cap }
    }

    fn bounds(self) -> InvalidationQueueBounds {
        InvalidationQueueBounds::with_pending_cap(self.pending_cap)
    }
}

impl Default for InvalidationJobQueueConfig {
    fn default() -> Self {
        Self {
            pending_cap: DEFAULT_QUEUE_PENDING_CAP,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaimRecomputeProcessOutcome {
    NoJob,
    CompletedFresh {
        job_id: String,
    },
    CompletedStale {
        job_id: String,
        successor_job_id: Option<String>,
    },
    RetryScheduled {
        job_id: String,
    },
    DeadLettered {
        job_id: String,
    },
}

pub fn enqueue_signal_claim_recompute_in_tx(
    tx: &ActionDb,
    origin_signal_id: &str,
    subject_type: &str,
    subject_id: &str,
) -> Result<crate::db::invalidation_jobs::InvalidationJobReceipt, String> {
    enqueue_signal_claim_recompute_with_config_in_tx(
        tx,
        origin_signal_id,
        subject_type,
        subject_id,
        InvalidationJobQueueConfig::from_env(),
    )
}

pub fn enqueue_signal_claim_recompute_with_config_in_tx(
    tx: &ActionDb,
    origin_signal_id: &str,
    subject_type: &str,
    subject_id: &str,
    config: InvalidationJobQueueConfig,
) -> Result<crate::db::invalidation_jobs::InvalidationJobReceipt, String> {
    let source_claim_version = tx
        .current_subject_claim_version(subject_type, subject_id)
        .map_err(|e| e.to_string())?;
    let input = EnqueueInvalidationJob::claim_recompute_from_signal(
        origin_signal_id,
        subject_type,
        subject_id,
        source_claim_version,
    );
    tx.enqueue_invalidation_job_with_bounds(input, config.bounds())
        .map_err(|e| e.to_string())
}

pub fn enqueue_direct_claim_recompute_in_tx(
    tx: &ActionDb,
    subject_type: &str,
    subject_id: &str,
    reason_code: &str,
) -> Result<crate::db::invalidation_jobs::InvalidationJobReceipt, String> {
    let source_claim_version = tx
        .current_subject_claim_version(subject_type, subject_id)
        .map_err(|e| e.to_string())?;
    let input_snapshot_hash =
        claim_recompute_input_hash(subject_type, subject_id, source_claim_version);
    let input = EnqueueInvalidationJob {
        job_kind: KIND_CLAIM_RECOMPUTE.to_string(),
        operation: "claim_recompute".to_string(),
        origin_signal_id: None,
        subject_type: subject_type.to_string(),
        subject_id: subject_id.to_string(),
        ability_id: "claim_recompute".to_string(),
        ability_version: "1".to_string(),
        source_claim_version,
        source_asof: None,
        input_snapshot_hash: Some(input_snapshot_hash.clone()),
        provider_fingerprint: None,
        prompt_fingerprint: None,
        payload_json: json!({ "reason_code": reason_code }),
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
    };
    tx.enqueue_invalidation_job_with_bounds(input, InvalidationJobQueueConfig::from_env().bounds())
        .map_err(|e| e.to_string())
}

pub fn process_one_claim_recompute_job(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    worker_id: &str,
) -> Result<ClaimRecomputeProcessOutcome, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let Some(job) = db
        .claim_next_claim_recompute_job(worker_id, 60)
        .map_err(|e| e.to_string())?
    else {
        return Ok(ClaimRecomputeProcessOutcome::NoJob);
    };

    let job_id = job.id.clone();
    let claim_count = match active_claim_count_for_subject(db, &job.subject_type, &job.subject_id) {
        Ok(count) => count,
        Err(error) if error.starts_with("unsupported claim recompute subject type:") => 0,
        Err(error) => return Err(error),
    };
    let claim_cap = claim_recompute_sync_claim_cap();
    if claim_count > claim_cap {
        log::info!(
            "Claim recompute subject {}:{} has {} active claims, above advisory worker cap {}; processing through durable recompute instead of dead-lettering by size",
            job.subject_type,
            job.subject_id,
            claim_count,
            claim_cap
        );
    }

    let recompute = run_claim_recompute(ctx, db, &job, claim_count, claim_cap);
    if let Err(error) = recompute {
        let disposition = db
            .mark_invalidation_job_failed(&job_id, &error)
            .map_err(|e| e.to_string())?;
        return Ok(match disposition {
            JobFailureDisposition::RetryScheduled => {
                ClaimRecomputeProcessOutcome::RetryScheduled { job_id }
            }
            JobFailureDisposition::DeadLettered => {
                ClaimRecomputeProcessOutcome::DeadLettered { job_id }
            }
        });
    }

    match db
        .terminalize_claim_recompute_job(&job_id)
        .map_err(|e| e.to_string())?
    {
        TerminalizationOutcome::Fresh => {
            Ok(ClaimRecomputeProcessOutcome::CompletedFresh { job_id })
        }
        TerminalizationOutcome::Stale {
            successor_job_id, ..
        } => Ok(ClaimRecomputeProcessOutcome::CompletedStale {
            job_id,
            successor_job_id,
        }),
    }
}

fn claim_recompute_sync_claim_cap() -> usize {
    std::env::var(CLAIM_RECOMPUTE_SYNC_CLAIM_CAP_ENV)
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|cap| *cap > 0)
        .unwrap_or(DEFAULT_CLAIM_RECOMPUTE_SYNC_CLAIM_CAP)
}

fn active_claim_count_for_subject(
    db: &ActionDb,
    subject_type: &str,
    subject_id: &str,
) -> Result<usize, String> {
    let subject_ref = subject_ref_json(subject_type, subject_id)?;
    let value: serde_json::Value = serde_json::from_str(&subject_ref)
        .map_err(|error| format!("claim recompute subject ref build failed: {error}"))?;
    let kind = value
        .get("kind")
        .and_then(|kind| kind.as_str())
        .ok_or_else(|| "claim recompute subject kind missing".to_string())?;
    let id = value
        .get("id")
        .and_then(|id| id.as_str())
        .ok_or_else(|| "claim recompute subject id missing".to_string())?;

    db.conn_ref()
        .query_row(
            "SELECT COUNT(*)
               FROM intelligence_claims
              WHERE json_valid(subject_ref) = 1
                AND lower(json_extract(subject_ref, '$.kind')) = lower(?1)
                AND json_extract(subject_ref, '$.id') = ?2
                AND claim_state = 'active'
                AND surfacing_state = 'active'",
            rusqlite::params![kind, id],
            |row| row.get::<_, usize>(0),
        )
        .map_err(|error| format!("claim recompute active claim count failed: {error}"))
}

async fn recover_db_service_after_worker_error(
    state: &Arc<AppState>,
    error: &DbAccessError,
    context: &'static str,
) -> String {
    let message = error.to_string();
    state
        .recover_db_service_after_access_error(error, context)
        .await;
    message
}

fn log_worker_iteration_error(worker_name: &str, error: &DbAccessError, message: &str) {
    if error.is_retryable() && !message.contains("file is not a database") {
        log::debug!("{worker_name} iteration retrying after transient DB contention: {message}");
    } else {
        log::warn!("{worker_name} iteration failed: {message}");
    }
}

pub async fn drain_pending_claim_recomputes(state: &Arc<AppState>) {
    let worker_id = format!("claim-recompute-startup-{}", uuid::Uuid::new_v4());
    for _ in 0..STARTUP_DRAIN_LIMIT {
        let worker_id = worker_id.clone();
        let result = state
            .db_write(move |db| {
                let clock = crate::services::context::SystemClock;
                let rng = crate::services::context::SystemRng;
                let ext = crate::services::context::ExternalClients::default();
                let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &ext);
                process_one_claim_recompute_job(&ctx, db, &worker_id)
            })
            .await;

        match result {
            Ok(ClaimRecomputeProcessOutcome::NoJob) => break,
            Ok(outcome) => log::info!("Claim recompute drain processed {outcome:?}"),
            Err(error) => {
                let message =
                    recover_db_service_after_worker_error(state, &error, "Claim recompute drain")
                        .await;
                log::warn!("Claim recompute drain stopped: {message}");
                break;
            }
        }
    }
}

pub async fn run_claim_recompute_worker(state: Arc<AppState>) {
    let worker_id = format!("claim-recompute-worker-{}", uuid::Uuid::new_v4());
    loop {
        if state.is_database_recovery_required() {
            log::warn!("Claim recompute worker stopped: database recovery required");
            break;
        }
        let worker_id_for_db = worker_id.clone();
        let result = state
            .db_write(move |db| {
                let clock = crate::services::context::SystemClock;
                let rng = crate::services::context::SystemRng;
                let ext = crate::services::context::ExternalClients::default();
                let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &ext);
                process_one_claim_recompute_job(&ctx, db, &worker_id_for_db)
            })
            .await;

        match result {
            Ok(ClaimRecomputeProcessOutcome::NoJob) => {
                tokio::time::sleep(std::time::Duration::from_millis(
                    CLAIM_RECOMPUTE_IDLE_POLL_MS,
                ))
                .await;
            }
            Ok(outcome) => {
                log::info!("Claim recompute worker processed {outcome:?}");
            }
            Err(error) => {
                let message =
                    recover_db_service_after_worker_error(&state, &error, "Claim recompute worker")
                        .await;
                log_worker_iteration_error("Claim recompute worker", &error, &message);
                tokio::time::sleep(std::time::Duration::from_millis(
                    CLAIM_RECOMPUTE_ERROR_POLL_MS,
                ))
                .await;
            }
        }
    }
}

pub async fn drain_pending_targeted_claim_repairs(state: &Arc<AppState>) {
    let worker_id = format!("targeted-repair-startup-{}", uuid::Uuid::new_v4());
    for _ in 0..TARGETED_REPAIR_DRAIN_LIMIT {
        let worker_id = worker_id.clone();
        let result = state
            .db_write(move |db| {
                let clock = crate::services::context::SystemClock;
                let rng = crate::services::context::SystemRng;
                let ext = crate::services::context::ExternalClients::default();
                let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &ext);
                crate::services::claims::targeted_repair_process_next_job(&ctx, db, &worker_id)
                    .map_err(|e| e.to_string())
            })
            .await;

        match result {
            Ok(crate::services::claims::TargetedRepairProcessOutcome::NoJob) => break,
            Ok(outcome) => log::info!("Targeted claim repair drain processed {outcome:?}"),
            Err(error) => {
                let message = recover_db_service_after_worker_error(
                    state,
                    &error,
                    "Targeted claim repair drain",
                )
                .await;
                log::warn!("Targeted claim repair drain stopped: {message}");
                break;
            }
        }
    }
}

pub async fn run_targeted_claim_repair_worker(state: Arc<AppState>) {
    let worker_id = format!("targeted-repair-worker-{}", uuid::Uuid::new_v4());
    loop {
        if state.is_database_recovery_required() {
            log::warn!("Targeted claim repair worker stopped: database recovery required");
            break;
        }
        let worker_id_for_db = worker_id.clone();
        let result = state
            .db_write(move |db| {
                let clock = crate::services::context::SystemClock;
                let rng = crate::services::context::SystemRng;
                let ext = crate::services::context::ExternalClients::default();
                let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &ext);
                crate::services::claims::targeted_repair_process_next_job(
                    &ctx,
                    db,
                    &worker_id_for_db,
                )
                .map_err(|e| e.to_string())
            })
            .await;

        match result {
            Ok(crate::services::claims::TargetedRepairProcessOutcome::NoJob) => {
                tokio::time::sleep(std::time::Duration::from_millis(
                    TARGETED_REPAIR_IDLE_POLL_MS,
                ))
                .await;
            }
            Ok(outcome) => {
                log::info!("Targeted claim repair worker processed {outcome:?}");
            }
            Err(error) => {
                let message = recover_db_service_after_worker_error(
                    &state,
                    &error,
                    "Targeted claim repair worker",
                )
                .await;
                log_worker_iteration_error("Targeted claim repair worker", &error, &message);
                tokio::time::sleep(std::time::Duration::from_millis(
                    TARGETED_REPAIR_ERROR_POLL_MS,
                ))
                .await;
            }
        }
    }
}

fn run_claim_recompute(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    job: &InvalidationJob,
    claim_count: usize,
    claim_cap: usize,
) -> Result<(), String> {
    db.with_transaction(|tx| {
        let chunk = claim_recompute_chunk_from_job(job, claim_count, claim_cap)?;
        let recompute_complete = if let Some(chunk) = chunk {
            let page_report =
                crate::services::trust_recompute::recompute_claim_trust_for_subject_page(
                    ctx,
                    tx,
                    &job.subject_type,
                    &job.subject_id,
                    chunk.after_created_at.as_deref(),
                    chunk.after_claim_id.as_deref(),
                    chunk.limit,
                )?;
            if let Some(next_cursor) = page_report.next_cursor {
                enqueue_claim_recompute_chunk_successor(tx, job, &chunk, next_cursor)?;
                false
            } else {
                true
            }
        } else {
            crate::services::trust_recompute::recompute_claim_trust_for_subject(
                ctx,
                tx,
                &job.subject_type,
                &job.subject_id,
            )?;
            true
        };

        if recompute_complete && job.subject_type.eq_ignore_ascii_case("account") {
            crate::services::intelligence::recompute_entity_health(
                ctx,
                tx,
                &job.subject_id,
                "account",
            )?;
        }

        Ok(())
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ClaimRecomputeChunk {
    after_created_at: Option<String>,
    after_claim_id: Option<String>,
    limit: usize,
    index: i64,
}

fn claim_recompute_chunk_from_job(
    job: &InvalidationJob,
    claim_count: usize,
    claim_cap: usize,
) -> Result<Option<ClaimRecomputeChunk>, String> {
    let payload: serde_json::Value = serde_json::from_str(&job.payload_json)
        .map_err(|error| format!("invalid claim recompute payload: {error}"))?;
    if let Some(chunk) = payload.get("chunk") {
        let limit = chunk
            .get("limit")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .filter(|value| *value > 0)
            .map(|value| value.min(claim_cap))
            .unwrap_or(claim_cap);
        return Ok(Some(ClaimRecomputeChunk {
            after_created_at: chunk
                .get("after_created_at")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string),
            after_claim_id: chunk
                .get("after_claim_id")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string),
            limit,
            index: chunk
                .get("index")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0),
        }));
    }

    if claim_count > claim_cap {
        return Ok(Some(ClaimRecomputeChunk {
            after_created_at: None,
            after_claim_id: None,
            limit: claim_cap,
            index: 0,
        }));
    }

    Ok(None)
}

fn enqueue_claim_recompute_chunk_successor(
    tx: &ActionDb,
    job: &InvalidationJob,
    current_chunk: &ClaimRecomputeChunk,
    next_cursor: crate::services::trust_recompute::TrustRecomputeCursor,
) -> Result<(), String> {
    let input_snapshot_hash =
        claim_recompute_input_hash(&job.subject_type, &job.subject_id, job.source_claim_version);
    let next_index = current_chunk.index + 1;
    let input = EnqueueInvalidationJob {
        job_kind: KIND_CLAIM_RECOMPUTE.to_string(),
        operation: "claim_recompute_chunk".to_string(),
        origin_signal_id: job.origin_signal_id.clone(),
        subject_type: job.subject_type.clone(),
        subject_id: job.subject_id.clone(),
        ability_id: "claim_recompute".to_string(),
        ability_version: "1".to_string(),
        source_claim_version: job.source_claim_version,
        source_asof: job.source_asof.clone(),
        input_snapshot_hash: Some(input_snapshot_hash.clone()),
        provider_fingerprint: job.provider_fingerprint.clone(),
        prompt_fingerprint: job.prompt_fingerprint.clone(),
        payload_json: json!({
            "reason_code": "chunk_successor",
            "chunk": {
                "after_created_at": next_cursor.created_at,
                "after_claim_id": next_cursor.claim_id,
                "limit": current_chunk.limit,
                "index": next_index,
                "parent_job_id": job.id,
            }
        }),
        coalescing_key: Some(format!(
            "claim_recompute_chunk:{}:{}:{}:{}",
            job.subject_type, job.subject_id, input_snapshot_hash, next_index
        )),
        chain_id: Some(job.chain_id.clone()),
        parent_job_id: Some(job.id.clone()),
        successor_of_job_id: Some(job.id.clone()),
        depth: job.depth + 1,
        chain_ancestry: serde_json::from_str(&job.chain_ancestry_json).unwrap_or_default(),
        max_attempts: job.max_attempts,
        priority: job.priority,
        raw_signal_count: job.raw_signal_count,
    };
    tx.enqueue_invalidation_job_with_bounds(input, InvalidationJobQueueConfig::from_env().bounds())
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn subject_ref_json(subject_type: &str, subject_id: &str) -> Result<String, String> {
    let kind = match subject_type.to_ascii_lowercase().as_str() {
        "account" | "project" | "person" | "meeting" => subject_type,
        other => return Err(format!("unsupported claim recompute subject type: {other}")),
    };
    Ok(json!({ "kind": kind, "id": subject_id }).to_string())
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use rusqlite::params;

    use super::*;
    use crate::db::claims::{ClaimSensitivity, TemporalScope};
    use crate::db::test_utils::test_db;
    use crate::db::DbAccount;
    use crate::intelligence::IntelligenceJson;
    use crate::services::claims::{commit_claim, ClaimProposal};
    use crate::services::context::{ExternalClients, FixedClock, SeedableRng};

    fn test_ctx<'a>(
        clock: &'a FixedClock,
        rng: &'a SeedableRng,
        ext: &'a ExternalClients,
    ) -> ServiceContext<'a> {
        ServiceContext::new_live(clock, rng, ext)
    }

    fn seed_account_with_intelligence(db: &ActionDb, account_id: &str) {
        let account = DbAccount {
            id: account_id.to_string(),
            name: format!("Account {account_id}"),
            updated_at: "2026-05-08T00:00:00Z".to_string(),
            ..Default::default()
        };
        db.upsert_account(&account).expect("seed account");
        let intel = IntelligenceJson {
            executive_assessment_render_policy: None,
            entity_id: account_id.to_string(),
            entity_type: "account".to_string(),
            enriched_at: Utc::now().to_rfc3339(),
            ..Default::default()
        };
        db.upsert_entity_intelligence(&intel)
            .expect("seed intelligence");
    }

    fn active_claim_proposal(account_id: &str, index: usize) -> ClaimProposal {
        ClaimProposal {
            id: None,
            expected_claim_version: None,
            subject_ref: json!({"kind": "account", "id": account_id}).to_string(),
            claim_type: "risk".to_string(),
            field_path: Some(format!("health.risk.{index}")),
            topic_key: None,
            text: format!("Synthetic risk claim {index}"),
            actor: "agent:test".to_string(),
            data_source: "unit_test".to_string(),
            source_ref: Some(format!("fixture://source-{index}")),
            source_asof: Some("2026-05-08T00:00:00Z".to_string()),
            observed_at: "2026-05-08T00:00:00Z".to_string(),
            provenance_json: "{}".to_string(),
            metadata_json: None,
            thread_id: None,
            temporal_scope: Some(TemporalScope::State),
            sensitivity: Some(ClaimSensitivity::Internal),
            supersedes: None,
            tombstone: None,
        }
    }

    #[test]
    fn signal_to_job_to_claim_recompute_round_trips_in_one_transaction() {
        let db = test_db();
        let account_id = "acct-roundtrip";
        seed_account_with_intelligence(&db, account_id);
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 8, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(7);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        let (signal_id, job_id, chain_id) = db
            .with_transaction(|tx| {
                let payload = json!({ "field": "trust" });
                let signal_id = crate::services::signals::emit_in_transaction(
                    &ctx,
                    tx,
                    "account",
                    account_id,
                    "claim_trust_changed",
                    "test",
                    payload,
                )
                .map_err(|e| e.to_string())?;
                let receipt =
                    enqueue_signal_claim_recompute_in_tx(tx, &signal_id, "account", account_id)?;
                Ok((signal_id, receipt.job_id, receipt.chain_id))
            })
            .expect("transactional signal and job");

        assert!(!chain_id.is_empty());
        let signal_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM signal_events WHERE id = ?1",
                params![&signal_id],
                |row| row.get(0),
            )
            .expect("signal count");
        assert_eq!(signal_count, 1);

        let outcome =
            process_one_claim_recompute_job(&ctx, &db, "worker-roundtrip").expect("process job");
        assert_eq!(
            outcome,
            ClaimRecomputeProcessOutcome::CompletedFresh {
                job_id: job_id.clone()
            }
        );

        let job = db
            .get_invalidation_job(&job_id)
            .expect("read job")
            .expect("job row");
        assert_eq!(job.status, crate::db::invalidation_jobs::STATUS_COMPLETED);
        assert_eq!(job.chain_id, chain_id);

        let health_score: Option<f64> = db
            .conn_ref()
            .query_row(
                "SELECT health_score FROM entity_quality WHERE entity_id = ?1",
                params![account_id],
                |row| row.get(0),
            )
            .ok();
        assert!(health_score.is_some());
    }

    #[test]
    fn enqueue_failure_rolls_back_signal_event() {
        let db = test_db();
        seed_account_with_intelligence(&db, "acct-rollback");
        db.conn_ref()
            .execute_batch(
                "CREATE TRIGGER fail_invalidation_insert
                 BEFORE INSERT ON invalidation_jobs
                 BEGIN
                   SELECT RAISE(ABORT, 'forced invalidation enqueue failure');
                 END;",
            )
            .expect("create trigger");
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 8, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(7);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        let result = db.with_transaction(|tx| {
            let payload = json!({ "field": "trust" });
            let signal_id = crate::services::signals::emit_in_transaction(
                &ctx,
                tx,
                "account",
                "acct-rollback",
                "claim_trust_changed",
                "test",
                payload,
            )
            .map_err(|e| e.to_string())?;
            enqueue_signal_claim_recompute_in_tx(tx, &signal_id, "account", "acct-rollback")?;
            Ok(())
        });
        assert!(result.is_err());

        let signal_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM signal_events WHERE entity_id = 'acct-rollback'",
                [],
                |row| row.get(0),
            )
            .expect("signal count");
        assert_eq!(signal_count, 0);
    }

    #[test]
    fn oversized_claim_recompute_subject_does_not_dead_letter_by_size() {
        let db = test_db();
        let account_id = "acct-large-recompute";
        seed_account_with_intelligence(&db, account_id);
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 8, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(7);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        for index in 0..=DEFAULT_CLAIM_RECOMPUTE_SYNC_CLAIM_CAP {
            commit_claim(&ctx, &db, active_claim_proposal(account_id, index))
                .expect("commit active claim");
        }

        let receipt =
            enqueue_direct_claim_recompute_in_tx(&db, "account", account_id, "oversized_test")
                .expect("enqueue direct recompute");
        let outcome =
            process_one_claim_recompute_job(&ctx, &db, "worker-large").expect("process job");
        assert!(
            matches!(
                outcome,
                ClaimRecomputeProcessOutcome::CompletedFresh { .. }
                    | ClaimRecomputeProcessOutcome::CompletedStale { .. }
            ),
            "oversized subject must not dead-letter solely by active claim count, got {outcome:?}"
        );

        let job = db
            .get_invalidation_job(&receipt.job_id)
            .expect("read recompute job")
            .expect("job row");
        assert_ne!(
            job.status,
            crate::db::invalidation_jobs::STATUS_DEAD_LETTERED
        );

        let (successor_count, chunk_limit, chunk_index): (i64, i64, i64) = db
            .conn_ref()
            .query_row(
                "SELECT count(*),
                        json_extract(payload_json, '$.chunk.limit'),
                        json_extract(payload_json, '$.chunk.index')
                   FROM invalidation_jobs
                  WHERE job_kind = 'claim_recompute'
                    AND operation = 'claim_recompute_chunk'
                    AND parent_job_id = ?1
                    AND status = 'pending'",
                params![&receipt.job_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("read chunk successor");
        assert_eq!(successor_count, 1);
        assert_eq!(chunk_limit as usize, DEFAULT_CLAIM_RECOMPUTE_SYNC_CLAIM_CAP);
        assert_eq!(chunk_index, 1);

        let second_outcome =
            process_one_claim_recompute_job(&ctx, &db, "worker-large-2").expect("process chunk");
        assert!(
            matches!(
                second_outcome,
                ClaimRecomputeProcessOutcome::CompletedFresh { .. }
                    | ClaimRecomputeProcessOutcome::CompletedStale { .. }
            ),
            "successor chunk must complete, got {second_outcome:?}"
        );
        let (pending_chunks, completed_chunks): (i64, i64) = db
            .conn_ref()
            .query_row(
                "SELECT
                    sum(CASE WHEN status = 'pending' THEN 1 ELSE 0 END),
                    sum(CASE WHEN status = 'completed' THEN 1 ELSE 0 END)
                   FROM invalidation_jobs
                  WHERE job_kind = 'claim_recompute'
                    AND operation = 'claim_recompute_chunk'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read final chunk state");
        assert_eq!(pending_chunks, 0);
        assert_eq!(completed_chunks, 1);
    }

    #[test]
    fn unsupported_subject_dead_letters_after_exhaustion() {
        let db = test_db();
        let mut input = EnqueueInvalidationJob::claim_recompute_from_signal(
            "sig-unsupported",
            "account",
            "acct-placeholder",
            0,
        );
        input.subject_type = "unsupported".to_string();
        input.max_attempts = 1;
        let receipt = db.enqueue_invalidation_job(input).expect("enqueue");

        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 8, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(7);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        let outcome =
            process_one_claim_recompute_job(&ctx, &db, "worker-dead").expect("process job");
        assert_eq!(
            outcome,
            ClaimRecomputeProcessOutcome::DeadLettered {
                job_id: receipt.job_id.clone()
            }
        );

        let dead = db
            .list_dead_lettered_invalidation_jobs(10)
            .expect("dead letters");
        assert_eq!(dead.len(), 1);
        assert_eq!(dead[0].id, receipt.job_id);
    }

    #[test]
    fn email_subject_dead_letters_after_exhaustion() {
        let db = test_db();
        let mut input = EnqueueInvalidationJob::claim_recompute_from_signal(
            "sig-email-unsupported",
            "account",
            "acct-placeholder",
            0,
        );
        input.subject_type = "email".to_string();
        input.subject_id = "message-fixture".to_string();
        input.max_attempts = 1;
        let receipt = db.enqueue_invalidation_job(input).expect("enqueue");

        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 8, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(7);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        let outcome =
            process_one_claim_recompute_job(&ctx, &db, "worker-email-dead").expect("process job");
        assert_eq!(
            outcome,
            ClaimRecomputeProcessOutcome::DeadLettered {
                job_id: receipt.job_id.clone()
            }
        );

        let dead = db
            .list_dead_lettered_invalidation_jobs(10)
            .expect("dead letters");
        assert_eq!(dead.len(), 1);
        assert_eq!(dead[0].id, receipt.job_id);
    }
}
