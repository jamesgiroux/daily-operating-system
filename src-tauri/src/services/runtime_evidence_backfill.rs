//! Service-owned runtime evidence backfills.
//!
//! Schema migrations may request a backfill, but durable intelligence writes
//! stay here so claims, provenance, trust recompute, and mutation policy remain
//! under the services boundary.

use rusqlite::{params, OptionalExtension};

use crate::db::ActionDb;
use crate::services::account_fact_claims::{
    backfill_account_fact_claims, AccountFactPromotionReport,
};
use crate::services::action_claims::{
    backfill_action_open_loop_claims_batch, ActionClaimPromotionReport,
};
use crate::services::context::ServiceContext;
use crate::services::intelligence::{
    backfill_entity_intelligence_projection_claims_batch,
    EntityIntelligenceProjectionBackfillReport,
};

pub const RUNTIME_EVIDENCE_BACKFILL_265_REQUESTED_AT_KEY: &str =
    "runtime_evidence_backfill_265_requested_at";
pub const RUNTIME_EVIDENCE_BACKFILL_265_STARTED_AT_KEY: &str =
    "runtime_evidence_backfill_265_started_at";
pub const RUNTIME_EVIDENCE_BACKFILL_265_COMPLETED_AT_KEY: &str =
    "runtime_evidence_backfill_265_completed_at";
pub const RUNTIME_EVIDENCE_BACKFILL_266_REQUESTED_AT_KEY: &str =
    "runtime_evidence_backfill_266_requested_at";
pub const RUNTIME_EVIDENCE_BACKFILL_266_STARTED_AT_KEY: &str =
    "runtime_evidence_backfill_266_started_at";
pub const RUNTIME_EVIDENCE_BACKFILL_266_COMPLETED_AT_KEY: &str =
    "runtime_evidence_backfill_266_completed_at";
pub const RUNTIME_EVIDENCE_BACKFILL_266_ENTITY_OFFSET_KEY: &str =
    "runtime_evidence_backfill_266_entity_offset";
pub const RUNTIME_EVIDENCE_BACKFILL_266_ACTION_OFFSET_KEY: &str =
    "runtime_evidence_backfill_266_action_offset";
pub const RUNTIME_EVIDENCE_BACKFILL_266_STORAGE_HALTED_AT_KEY: &str =
    "runtime_evidence_backfill_266_storage_halted_at";

const RUNTIME_EVIDENCE_BACKFILL_266_ENTITY_BATCH_SIZE: usize = 50;
const RUNTIME_EVIDENCE_BACKFILL_266_ACTION_BATCH_SIZE: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeEvidenceBackfillReport {
    pub account_fact_report: AccountFactPromotionReport,
    pub entity_intelligence_report: EntityIntelligenceProjectionBackfillReport,
    pub action_claim_report: ActionClaimPromotionReport,
    pub completed: bool,
}

impl RuntimeEvidenceBackfillReport {
    pub fn recompute_jobs_enqueued(&self) -> u32 {
        self.account_fact_report
            .recompute_jobs_enqueued
            .saturating_add(
                u32::try_from(self.entity_intelligence_report.recompute_jobs_enqueued)
                    .unwrap_or(u32::MAX),
            )
            .saturating_add(
                u32::try_from(self.action_claim_report.recompute_jobs_enqueued).unwrap_or(u32::MAX),
            )
    }

    pub fn error_count(&self) -> usize {
        self.account_fact_report.claim_errors.len()
            + self.account_fact_report.recompute_enqueue_errors.len()
            + self.account_fact_report.source_ref_errors.len()
            + self.entity_intelligence_report.errors.len()
            + self.action_claim_report.errors.len()
    }
}

pub fn run_runtime_evidence_backfill_if_pending(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
) -> Result<Option<RuntimeEvidenceBackfillReport>, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    if !migration_state_exists(db)? {
        return Ok(None);
    }
    let account_fact_pending =
        migration_state_value(db, RUNTIME_EVIDENCE_BACKFILL_265_REQUESTED_AT_KEY)?.is_some()
            && migration_state_value(db, RUNTIME_EVIDENCE_BACKFILL_265_COMPLETED_AT_KEY)?.is_none();
    let entity_intelligence_pending =
        migration_state_value(db, RUNTIME_EVIDENCE_BACKFILL_266_REQUESTED_AT_KEY)?.is_some()
            && migration_state_value(db, RUNTIME_EVIDENCE_BACKFILL_266_COMPLETED_AT_KEY)?.is_none();
    if !account_fact_pending && !entity_intelligence_pending {
        return Ok(None);
    }
    if entity_intelligence_pending
        && migration_state_value(db, RUNTIME_EVIDENCE_BACKFILL_266_STORAGE_HALTED_AT_KEY)?.is_some()
    {
        return Err(
            "runtime evidence backfill 266 is paused after a prior storage-health failure"
                .to_string(),
        );
    }

    if account_fact_pending {
        record_marker(
            db,
            RUNTIME_EVIDENCE_BACKFILL_265_STARTED_AT_KEY,
            ctx.clock.now().timestamp(),
        )?;
    }
    if entity_intelligence_pending {
        record_marker(
            db,
            RUNTIME_EVIDENCE_BACKFILL_266_STARTED_AT_KEY,
            ctx.clock.now().timestamp(),
        )?;
    }

    let account_fact_report = if account_fact_pending {
        backfill_account_fact_claims(ctx, db)?
    } else {
        AccountFactPromotionReport::default()
    };
    let action_claim_report = if entity_intelligence_pending {
        let offset = migration_state_offset(db, RUNTIME_EVIDENCE_BACKFILL_266_ACTION_OFFSET_KEY)?;
        let report = backfill_action_open_loop_claims_batch(
            ctx,
            db,
            offset,
            RUNTIME_EVIDENCE_BACKFILL_266_ACTION_BATCH_SIZE,
        )?;
        halt_on_storage_health_errors(ctx, db, &report.errors)?;
        record_marker(
            db,
            RUNTIME_EVIDENCE_BACKFILL_266_ACTION_OFFSET_KEY,
            i64::try_from(report.next_offset).unwrap_or(i64::MAX),
        )?;
        report
    } else {
        ActionClaimPromotionReport::default()
    };
    let entity_intelligence_report = if entity_intelligence_pending && action_claim_report.finished
    {
        let offset = migration_state_offset(db, RUNTIME_EVIDENCE_BACKFILL_266_ENTITY_OFFSET_KEY)?;
        let report = backfill_entity_intelligence_projection_claims_batch(
            ctx,
            db,
            offset,
            RUNTIME_EVIDENCE_BACKFILL_266_ENTITY_BATCH_SIZE,
        )?;
        halt_on_storage_health_errors(ctx, db, &report.errors)?;
        record_marker(
            db,
            RUNTIME_EVIDENCE_BACKFILL_266_ENTITY_OFFSET_KEY,
            i64::try_from(report.next_offset).unwrap_or(i64::MAX),
        )?;
        report
    } else {
        EntityIntelligenceProjectionBackfillReport::default()
    };
    let account_fact_completed = account_fact_report.claim_errors.is_empty()
        && account_fact_report.recompute_enqueue_errors.is_empty()
        && account_fact_report.source_ref_errors.is_empty();
    let entity_intelligence_completed = entity_intelligence_report.finished;
    let action_claim_completed = !entity_intelligence_pending || action_claim_report.finished;
    if account_fact_pending && account_fact_completed {
        record_marker(
            db,
            RUNTIME_EVIDENCE_BACKFILL_265_COMPLETED_AT_KEY,
            ctx.clock.now().timestamp(),
        )?;
    }
    if entity_intelligence_pending && entity_intelligence_completed && action_claim_completed {
        record_marker(
            db,
            RUNTIME_EVIDENCE_BACKFILL_266_COMPLETED_AT_KEY,
            ctx.clock.now().timestamp(),
        )?;
    }
    let completed = (!account_fact_pending || account_fact_completed)
        && (!entity_intelligence_pending
            || (entity_intelligence_completed && action_claim_completed));

    Ok(Some(RuntimeEvidenceBackfillReport {
        account_fact_report,
        entity_intelligence_report,
        action_claim_report,
        completed,
    }))
}

pub fn runtime_evidence_backfill_266_resume_pending(db: &ActionDb) -> Result<bool, String> {
    if !migration_state_exists(db)? {
        return Ok(false);
    }
    let requested =
        migration_state_value(db, RUNTIME_EVIDENCE_BACKFILL_266_REQUESTED_AT_KEY)?.is_some();
    let started =
        migration_state_value(db, RUNTIME_EVIDENCE_BACKFILL_266_STARTED_AT_KEY)?.is_some();
    let completed =
        migration_state_value(db, RUNTIME_EVIDENCE_BACKFILL_266_COMPLETED_AT_KEY)?.is_some();

    Ok(requested && started && !completed)
}

pub fn error_indicates_storage_health_failure(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    lower.contains("disk i/o error")
        || lower.contains("file is not a database")
        || lower.contains("database disk image is malformed")
        || lower.contains("sqlite_notadb")
        || lower.contains("sqlcipher key verification failed")
}

fn halt_on_storage_health_errors(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    errors: &[String],
) -> Result<(), String> {
    if !errors
        .iter()
        .any(|error| error_indicates_storage_health_failure(error))
    {
        return Ok(());
    }
    let halted_at = ctx.clock.now().timestamp();
    record_marker(
        db,
        RUNTIME_EVIDENCE_BACKFILL_266_STORAGE_HALTED_AT_KEY,
        halted_at,
    )
    .map_err(|error| format!("runtime evidence backfill storage halt marker failed: {error}"))?;
    Err("runtime evidence backfill halted after storage-health failure".to_string())
}

fn migration_state_exists(db: &ActionDb) -> Result<bool, String> {
    db.conn_ref()
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM sqlite_master
                 WHERE type = 'table' AND name = 'migration_state'
             )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(|e| format!("runtime evidence backfill schema probe failed: {e}"))
}

fn migration_state_value(db: &ActionDb, key: &str) -> Result<Option<i64>, String> {
    db.conn_ref()
        .query_row(
            "SELECT value FROM migration_state WHERE key = ?1",
            [key],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|e| format!("runtime evidence backfill marker read failed for {key}: {e}"))
}

fn migration_state_offset(db: &ActionDb, key: &str) -> Result<usize, String> {
    Ok(migration_state_value(db, key)?
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(0))
}

fn record_marker(db: &ActionDb, key: &str, value: i64) -> Result<(), String> {
    db.conn_ref()
        .execute(
            "INSERT OR REPLACE INTO migration_state (key, value) VALUES (?1, ?2)",
            params![key, value],
        )
        .map_err(|e| format!("runtime evidence backfill marker write failed for {key}: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;
    use crate::db::{AccountType, DbAccount};
    use crate::services::claims::load_claims_active;
    use crate::services::context::{ExternalClients, FixedClock, SeedableRng};

    fn test_db() -> ActionDb {
        ActionDb::from_connection_for_tests(crate::migrations::migrated_in_memory_for_tests())
    }

    fn test_ctx<'a>(
        clock: &'a FixedClock,
        rng: &'a SeedableRng,
        ext: &'a ExternalClients,
    ) -> ServiceContext<'a> {
        ServiceContext::test_live(clock, rng, ext)
    }

    fn account(id: &str) -> DbAccount {
        DbAccount {
            id: id.to_string(),
            name: "Example Account".to_string(),
            account_type: AccountType::Customer,
            updated_at: "2026-05-20T00:00:00Z".to_string(),
            arr_range_low: Some(125_000.0),
            arr_range_high: Some(125_000.0),
            ..Default::default()
        }
    }

    #[test]
    fn runtime_evidence_backfill_runs_once_when_requested_by_migration() {
        let db = test_db();
        db.upsert_account(&account("acct-runtime-backfill"))
            .expect("seed account");
        db.conn_ref()
            .execute(
                "UPDATE accounts
                    SET arr_range_low = 125000,
                        arr_range_high = 125000
                  WHERE id = 'acct-runtime-backfill'",
                [],
            )
            .expect("seed source-less schema fact");
        record_marker(&db, RUNTIME_EVIDENCE_BACKFILL_265_REQUESTED_AT_KEY, 1)
            .expect("request marker");
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 22, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(22);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        let first = run_runtime_evidence_backfill_if_pending(&ctx, &db)
            .expect("run backfill")
            .expect("pending backfill");
        assert!(first.completed);
        assert_eq!(first.account_fact_report.claims_committed, 1);
        assert!(
            migration_state_value(&db, RUNTIME_EVIDENCE_BACKFILL_265_COMPLETED_AT_KEY)
                .expect("completion marker read")
                .is_some()
        );

        let subject_ref =
            serde_json::json!({"kind": "account", "id": "acct-runtime-backfill"}).to_string();
        let active_claims =
            load_claims_active(&db, &subject_ref, Some("account_fact")).expect("active claims");
        assert!(active_claims
            .iter()
            .any(|claim| claim.text == "arr: 125,000"));

        let second = run_runtime_evidence_backfill_if_pending(&ctx, &db).expect("second run");
        assert_eq!(second, None);
    }

    #[test]
    fn runtime_evidence_backfill_266_advances_in_bounded_slices() {
        let db = test_db();
        db.conn_ref()
            .execute(
                "DELETE FROM migration_state
                  WHERE key IN (?1, ?2, ?3, ?4, ?5, ?6)",
                [
                    RUNTIME_EVIDENCE_BACKFILL_265_REQUESTED_AT_KEY,
                    RUNTIME_EVIDENCE_BACKFILL_265_COMPLETED_AT_KEY,
                    RUNTIME_EVIDENCE_BACKFILL_266_REQUESTED_AT_KEY,
                    RUNTIME_EVIDENCE_BACKFILL_266_COMPLETED_AT_KEY,
                    RUNTIME_EVIDENCE_BACKFILL_266_ENTITY_OFFSET_KEY,
                    RUNTIME_EVIDENCE_BACKFILL_266_ACTION_OFFSET_KEY,
                ],
            )
            .expect("clear runtime markers");
        for idx in 0..(RUNTIME_EVIDENCE_BACKFILL_266_ENTITY_BATCH_SIZE + 1) {
            db.conn_ref()
                .execute(
                    "INSERT OR REPLACE INTO entity_assessment (
                        entity_id,
                        entity_type,
                        enriched_at,
                        executive_assessment
                    ) VALUES (?1, 'account', '2026-05-20T00:00:00Z', ?2)",
                    rusqlite::params![
                        format!("acct-runtime-slice-{idx:02}"),
                        format!("Runtime slice summary {idx}")
                    ],
                )
                .expect("seed entity assessment");
        }
        record_marker(&db, RUNTIME_EVIDENCE_BACKFILL_266_REQUESTED_AT_KEY, 1)
            .expect("request marker");
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 22, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(24);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        let first = run_runtime_evidence_backfill_if_pending(&ctx, &db)
            .expect("first slice")
            .expect("pending backfill");
        assert!(!first.completed);
        assert_eq!(
            first.entity_intelligence_report.rows_examined,
            RUNTIME_EVIDENCE_BACKFILL_266_ENTITY_BATCH_SIZE
        );
        assert_eq!(
            migration_state_value(&db, RUNTIME_EVIDENCE_BACKFILL_266_ENTITY_OFFSET_KEY)
                .expect("entity offset read"),
            Some(RUNTIME_EVIDENCE_BACKFILL_266_ENTITY_BATCH_SIZE as i64)
        );
        assert!(
            migration_state_value(&db, RUNTIME_EVIDENCE_BACKFILL_266_COMPLETED_AT_KEY)
                .expect("completion read")
                .is_none()
        );

        let second = run_runtime_evidence_backfill_if_pending(&ctx, &db)
            .expect("second slice")
            .expect("pending backfill");
        assert!(second.completed);
        assert_eq!(second.entity_intelligence_report.rows_examined, 1);
        assert!(
            migration_state_value(&db, RUNTIME_EVIDENCE_BACKFILL_266_COMPLETED_AT_KEY)
                .expect("completion read")
                .is_some()
        );
    }

    #[test]
    fn storage_health_error_detection_catches_sqlite_corruption_signals() {
        for message in [
            "SQLite error: disk I/O error",
            "file is not a database",
            "rusqlite error: database disk image is malformed",
            "SQLITE_NOTADB while opening DB",
            "SQLCipher key verification failed (database unreadable): disk I/O error",
        ] {
            assert!(error_indicates_storage_health_failure(message), "{message}");
        }
        assert!(!error_indicates_storage_health_failure(
            "database is locked"
        ));
    }

    #[test]
    fn runtime_evidence_backfill_266_resume_pending_only_after_started_marker() {
        let db = test_db();

        assert!(!runtime_evidence_backfill_266_resume_pending(&db).expect("initial check"));

        record_marker(&db, RUNTIME_EVIDENCE_BACKFILL_266_REQUESTED_AT_KEY, 1)
            .expect("request marker");
        assert!(!runtime_evidence_backfill_266_resume_pending(&db).expect("requested check"));

        record_marker(&db, RUNTIME_EVIDENCE_BACKFILL_266_STARTED_AT_KEY, 2)
            .expect("started marker");
        assert!(runtime_evidence_backfill_266_resume_pending(&db).expect("started check"));

        record_marker(&db, RUNTIME_EVIDENCE_BACKFILL_266_COMPLETED_AT_KEY, 3)
            .expect("completed marker");
        assert!(!runtime_evidence_backfill_266_resume_pending(&db).expect("completed check"));
    }

    #[test]
    fn runtime_evidence_backfill_stops_when_storage_health_marker_exists() {
        let db = test_db();
        record_marker(&db, RUNTIME_EVIDENCE_BACKFILL_266_REQUESTED_AT_KEY, 1)
            .expect("request marker");
        record_marker(&db, RUNTIME_EVIDENCE_BACKFILL_266_STORAGE_HALTED_AT_KEY, 2)
            .expect("halt marker");
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 22, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(25);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        let error = run_runtime_evidence_backfill_if_pending(&ctx, &db)
            .expect_err("halted backfill must not resume on startup");
        assert!(error.contains("paused after a prior storage-health failure"));
    }

    #[test]
    fn runtime_evidence_backfill_skips_without_request_marker() {
        let db = test_db();
        db.conn_ref()
            .execute(
                "DELETE FROM migration_state
	                  WHERE key IN (?1, ?2)",
                [
                    RUNTIME_EVIDENCE_BACKFILL_265_REQUESTED_AT_KEY,
                    RUNTIME_EVIDENCE_BACKFILL_266_REQUESTED_AT_KEY,
                ],
            )
            .expect("clear request marker");
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 22, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(23);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        let report = run_runtime_evidence_backfill_if_pending(&ctx, &db).expect("backfill probe");
        assert_eq!(report, None);
    }
}
