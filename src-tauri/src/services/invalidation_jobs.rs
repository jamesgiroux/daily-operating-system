use std::sync::Arc;

use serde_json::json;

use crate::db::invalidation_jobs::{
    EnqueueInvalidationJob, InvalidationJob, InvalidationQueueBounds, JobFailureDisposition,
    TerminalizationOutcome, DEFAULT_QUEUE_PENDING_CAP,
};
use crate::db::ActionDb;
use crate::services::context::ServiceContext;
use crate::state::AppState;

const STARTUP_DRAIN_LIMIT: usize = 100;
const TARGETED_REPAIR_DRAIN_LIMIT: usize = 100;
const TARGETED_REPAIR_IDLE_POLL_MS: u64 = 250;
const TARGETED_REPAIR_ERROR_POLL_MS: u64 = 2_000;
const QUEUE_PENDING_CAP_ENV: &str = "DAILYOS_INVALIDATION_JOBS_PENDING_CAP";

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
        .current_claim_version_for_subject(subject_type, subject_id)
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
    let recompute = run_claim_recompute(ctx, db, &job);
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
                log::warn!("Claim recompute drain stopped: {error}");
                break;
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
                log::warn!("Targeted claim repair drain stopped: {error}");
                break;
            }
        }
    }
}

pub async fn run_targeted_claim_repair_worker(state: Arc<AppState>) {
    let worker_id = format!("targeted-repair-worker-{}", uuid::Uuid::new_v4());
    loop {
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
                log::warn!("Targeted claim repair worker iteration failed: {error}");
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
) -> Result<(), String> {
    let subject_ref = subject_ref_json(&job.subject_type, &job.subject_id)?;
    let _claims = crate::services::claims::load_claims_active_for_surface(
        db,
        &subject_ref,
        None,
        crate::services::context::ClaimDismissalSurface::TauriReport.as_str(),
    )
    .map_err(|e| format!("load active claims for recompute: {e}"))?;

    if job.subject_type.eq_ignore_ascii_case("account") {
        crate::services::intelligence::recompute_entity_health(
            ctx,
            db,
            &job.subject_id,
            "account",
        )?;
    }

    Ok(())
}

fn subject_ref_json(subject_type: &str, subject_id: &str) -> Result<String, String> {
    let kind = match subject_type.to_ascii_lowercase().as_str() {
        "account" | "project" | "person" | "meeting" | "email" => subject_type,
        other => return Err(format!("unsupported claim recompute subject type: {other}")),
    };
    Ok(json!({ "kind": kind, "id": subject_id }).to_string())
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use rusqlite::params;

    use super::*;
    use crate::db::test_utils::test_db;
    use crate::db::DbAccount;
    use crate::intelligence::IntelligenceJson;
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
}
