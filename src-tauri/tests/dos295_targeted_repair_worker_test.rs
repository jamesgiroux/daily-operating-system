#![cfg(feature = "test-harness")]

use std::ffi::OsString;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use dailyos_lib::abilities::feedback::FeedbackAction;
use dailyos_lib::db::{ActionDb, DbAccount, DbError};
use dailyos_lib::services::claims::{
    commit_claim, is_claim_dismissed_on_surface, record_claim_feedback, ClaimError,
    ClaimFeedbackInput, ClaimProposal, CommittedClaim,
};
use dailyos_lib::services::context::{
    ClaimDismissalSurface, ExternalClients, SeedableRng, ServiceContext, SystemClock,
};
use dailyos_lib::state::AppState;
use rusqlite::params;

const INVALIDATION_PENDING_CAP_ENV: &str = "DAILYOS_INVALIDATION_JOBS_PENDING_CAP";

static TARGETED_REPAIR_ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

struct EnvVarGuard {
    name: &'static str,
    previous: Option<OsString>,
}

impl EnvVarGuard {
    fn set(name: &'static str, value: &str) -> Self {
        let previous = std::env::var_os(name);
        std::env::set_var(name, value);
        Self { name, previous }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => std::env::set_var(self.name, value),
            None => std::env::remove_var(self.name),
        }
    }
}

fn seed_account(db: &ActionDb, id: &str) {
    let account = DbAccount {
        id: id.to_string(),
        name: format!("DOS-295 Worker Account {id}"),
        updated_at: "2026-05-09T00:00:00Z".to_string(),
        ..Default::default()
    };
    db.upsert_account(&account).expect("seed account");
}

fn proposal(account_id: &str, field_path: &str, text: &str) -> ClaimProposal {
    ClaimProposal {
        id: None,
        expected_claim_version: None,
        subject_ref: serde_json::json!({
            "kind": "account",
            "id": account_id
        })
        .to_string(),
        claim_type: "risk".to_string(),
        field_path: Some(field_path.to_string()),
        topic_key: None,
        text: text.to_string(),
        actor: "agent:test".to_string(),
        data_source: "unit_test".to_string(),
        source_ref: Some(serde_json::json!({ "fixture": field_path }).to_string()),
        source_asof: Some("2026-05-09T00:00:00Z".to_string()),
        observed_at: "2026-05-09T00:00:00Z".to_string(),
        provenance_json: "{}".to_string(),
        metadata_json: None,
        thread_id: None,
        temporal_scope: None,
        sensitivity: None,
        supersedes: None,
        tombstone: None,
    }
}

fn feedback_payload(action: FeedbackAction) -> Option<String> {
    match action {
        FeedbackAction::WrongSubject => Some(
            serde_json::json!({
                "corrected_subject": {
                    "kind": "account",
                    "id": "acct-dos295-corrected"
                }
            })
            .to_string(),
        ),
        FeedbackAction::WrongSource => {
            Some(serde_json::json!({ "source_ref": "source-does-not-support" }).to_string())
        }
        FeedbackAction::SurfaceInappropriate => {
            Some(serde_json::json!({ "surface": "briefing" }).to_string())
        }
        _ => None,
    }
}

fn feedback_input(claim_id: String, action: FeedbackAction) -> ClaimFeedbackInput {
    ClaimFeedbackInput {
        claim_id,
        action,
        actor: "user:test".to_string(),
        actor_id: Some("user-test".to_string()),
        payload_json: feedback_payload(action),
    }
}

fn surface_feedback_input(claim_id: String, surface: &str) -> ClaimFeedbackInput {
    ClaimFeedbackInput {
        claim_id,
        action: FeedbackAction::SurfaceInappropriate,
        actor: "user:test".to_string(),
        actor_id: Some("user-test".to_string()),
        payload_json: Some(serde_json::json!({ "surface": surface }).to_string()),
    }
}

fn inserted_claim_id(committed: CommittedClaim) -> String {
    match committed {
        CommittedClaim::Inserted { claim } => claim.id,
        other => panic!("expected inserted claim, got {other:?}"),
    }
}

fn assert_pending_cap_rejection(error: &ClaimError) {
    assert!(
        matches!(
            error,
            ClaimError::Db(DbError::InvalidArgument(message))
                if message.contains("invalidation queue pending cap 1 reached")
        ),
        "expected visible pending-cap rejection, got {error}"
    );
}

async fn wait_for_jobs_completed(
    state: Arc<AppState>,
    job_ids: Vec<String>,
) -> Result<Vec<(String, Option<String>)>, String> {
    let wait_result = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let ids = job_ids.clone();
            let rows = state
                .db_read(move |db| {
                    let mut rows = Vec::new();
                    for job_id in ids {
                        let row = db
                            .conn_ref()
                            .query_row(
                                "SELECT status, completed_at
                                 FROM invalidation_jobs
                                 WHERE id = ?1",
                                params![job_id],
                                |row| {
                                    Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
                                },
                            )
                            .map_err(|e| e.to_string())?;
                        rows.push(row);
                    }
                    Ok(rows)
                })
                .await?;
            if rows.iter().all(|(status, _)| status == "completed") {
                return Ok::<Vec<(String, Option<String>)>, String>(rows);
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await;

    match wait_result {
        Ok(result) => result,
        Err(_) => {
            let ids = job_ids.clone();
            let rows = state
                .db_read(move |db| {
                    let mut rows = Vec::new();
                    for job_id in ids {
                        let row = db
                            .conn_ref()
                            .query_row(
                                "SELECT id, status, attempts, last_error
                                 FROM invalidation_jobs
                                 WHERE id = ?1",
                                params![job_id],
                                |row| {
                                    Ok((
                                        row.get::<_, String>(0)?,
                                        row.get::<_, String>(1)?,
                                        row.get::<_, i64>(2)?,
                                        row.get::<_, Option<String>>(3)?,
                                    ))
                                },
                            )
                            .map_err(|e| e.to_string())?;
                        rows.push(row);
                    }
                    Ok(rows)
                })
                .await?;
            Err(format!(
                "repair worker should complete all jobs within 5 seconds; statuses={rows:?}"
            ))
        }
    }
}

#[tokio::test]
async fn targeted_repair_worker_completes_all_repair_actions_from_feedback() {
    let temp = tempfile::tempdir().expect("tempdir");
    let db_path = temp.path().join("dos295-targeted-repair-worker.db");
    let svc = dailyos_lib::db_service::DbService::open_at_unencrypted_for_tests(db_path)
        .await
        .expect("open test db service");
    let state = Arc::new(AppState::test_with_db_service(svc));

    let repair_job_ids = {
        let _env_lock = TARGETED_REPAIR_ENV_LOCK.lock().await;
        state
            .db_write(|db| {
                seed_account(db, "acct-dos295-worker");
                seed_account(db, "acct-dos295-corrected");
                let clock = SystemClock;
                let rng = SeedableRng::new(295);
                let external = ExternalClients::default();
                let ctx = ServiceContext::new_live(&clock, &rng, &external);
                let actions = [
                    FeedbackAction::MarkFalse,
                    FeedbackAction::CannotVerify,
                    FeedbackAction::MarkOutdated,
                    FeedbackAction::WrongSubject,
                    FeedbackAction::WrongSource,
                    FeedbackAction::SurfaceInappropriate,
                ];

                let mut ids = Vec::new();
                for action in actions {
                    let field_path = format!("health.risk.{}", action.as_str());
                    let claim_id = inserted_claim_id(
                        commit_claim(
                            &ctx,
                            db,
                            proposal(
                                "acct-dos295-worker",
                                &field_path,
                                &format!("DOS-295 {:?} repair fixture", action),
                            ),
                        )
                        .map_err(|e| e.to_string())?,
                    );
                    let feedback =
                        record_claim_feedback(&ctx, db, feedback_input(claim_id, action))
                            .map_err(|e| e.to_string())?;
                    ids.push(
                        feedback
                            .repair_job_id
                            .ok_or_else(|| format!("{action:?} should enqueue repair"))?,
                    );
                }
                Ok(ids)
            })
            .await
            .expect("seed feedback jobs")
    };

    assert_eq!(repair_job_ids.len(), 6);

    let worker = tokio::spawn(
        dailyos_lib::services::invalidation_jobs::run_targeted_claim_repair_worker(state.clone()),
    );

    let rows = wait_for_jobs_completed(state.clone(), repair_job_ids.clone())
        .await
        .expect("repair jobs completed");

    worker.abort();
    for (status, completed_at) in rows {
        assert_eq!(status, "completed");
        assert!(completed_at.is_some());
        assert!(Utc::now().to_rfc3339() >= completed_at.unwrap());
    }

    let completed_targeted_repair_jobs: i64 = state
        .db_read(move |db| {
            let mut completed = 0;
            for job_id in repair_job_ids {
                let row: (String, String) = db
                    .conn_ref()
                    .query_row(
                        "SELECT job_kind, status
                         FROM invalidation_jobs
                         WHERE id = ?1",
                        params![job_id],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .map_err(|e| e.to_string())?;
                if row.0 == "targeted_repair" && row.1 == "completed" {
                    completed += 1;
                }
            }
            Ok::<_, String>(completed)
        })
        .await
        .expect("read completed targeted repair count");
    assert_eq!(completed_targeted_repair_jobs, 6);
}

#[tokio::test]
async fn targeted_repair_pending_cap_does_not_drop_distinct_surface_feedback() {
    let temp = tempfile::tempdir().expect("tempdir");
    let db_path = temp.path().join("dos295-targeted-repair-surface-cap.db");
    let svc = dailyos_lib::db_service::DbService::open_at_unencrypted_for_tests(db_path)
        .await
        .expect("open test db service");
    let state = Arc::new(AppState::test_with_db_service(svc));

    let (claim_id, first_job_id) = {
        let _env_lock = TARGETED_REPAIR_ENV_LOCK.lock().await;
        let _env_guard = EnvVarGuard::set(INVALIDATION_PENDING_CAP_ENV, "1");
        state
            .db_write(|db| {
                seed_account(db, "acct-dos295-surface-cap");
                let clock = SystemClock;
                let rng = SeedableRng::new(296);
                let external = ExternalClients::default();
                let ctx = ServiceContext::new_live(&clock, &rng, &external);
                let claim_id = inserted_claim_id(
                    commit_claim(
                        &ctx,
                        db,
                        proposal(
                            "acct-dos295-surface-cap",
                            "health.risk.surface_cap",
                            "DOS-295 surface cap repair fixture",
                        ),
                    )
                    .map_err(|e| e.to_string())?,
                );

                let first = record_claim_feedback(
                    &ctx,
                    db,
                    surface_feedback_input(claim_id.clone(), "briefing"),
                )
                .map_err(|e| e.to_string())?;
                let second = record_claim_feedback(
                    &ctx,
                    db,
                    surface_feedback_input(claim_id.clone(), "entity_detail"),
                );
                let Err(error) = second else {
                    panic!("distinct surface repair should reject while pending cap is full");
                };
                assert_pending_cap_rejection(&error);

                let first_job_id = first
                    .repair_job_id
                    .ok_or_else(|| "first surface feedback should enqueue repair".to_string())?;
                Ok::<_, String>((claim_id, first_job_id))
            })
            .await
            .expect("seed first capped surface feedback")
    };

    let worker = tokio::spawn(
        dailyos_lib::services::invalidation_jobs::run_targeted_claim_repair_worker(state.clone()),
    );

    wait_for_jobs_completed(state.clone(), vec![first_job_id])
        .await
        .expect("first surface repair completed");

    let second_job_id = {
        let _env_lock = TARGETED_REPAIR_ENV_LOCK.lock().await;
        let _env_guard = EnvVarGuard::set(INVALIDATION_PENDING_CAP_ENV, "1");
        let claim_id = claim_id.clone();
        state
            .db_write(move |db| {
                let clock = SystemClock;
                let rng = SeedableRng::new(297);
                let external = ExternalClients::default();
                let ctx = ServiceContext::new_live(&clock, &rng, &external);
                let second = record_claim_feedback(
                    &ctx,
                    db,
                    surface_feedback_input(claim_id, "entity_detail"),
                )
                .map_err(|e| e.to_string())?;
                second
                    .repair_job_id
                    .ok_or_else(|| "retried surface feedback should enqueue repair".to_string())
            })
            .await
            .expect("retry second surface feedback after drain")
    };

    wait_for_jobs_completed(state.clone(), vec![second_job_id])
        .await
        .expect("second surface repair completed");

    worker.abort();

    let assertion_claim_id = claim_id.clone();
    let surfaces = state
        .db_read(move |db| {
            assert!(is_claim_dismissed_on_surface(
                db,
                &assertion_claim_id,
                ClaimDismissalSurface::Briefing.as_str()
            )
            .map_err(|e| e.to_string())?);
            assert!(is_claim_dismissed_on_surface(
                db,
                &assertion_claim_id,
                ClaimDismissalSurface::TauriEntityDetail.as_str()
            )
            .map_err(|e| e.to_string())?);
            db.conn_ref()
                .prepare(
                    "SELECT surface
                     FROM claim_surface_dismissals
                     WHERE claim_id = ?1
                     ORDER BY surface ASC",
                )
                .map_err(|e| e.to_string())?
                .query_map(params![&assertion_claim_id], |row| row.get::<_, String>(0))
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())
        })
        .await
        .expect("read surface dismissals");
    assert_eq!(
        surfaces,
        vec![
            ClaimDismissalSurface::Briefing.as_str().to_string(),
            ClaimDismissalSurface::TauriEntityDetail
                .as_str()
                .to_string(),
        ]
    );
}
