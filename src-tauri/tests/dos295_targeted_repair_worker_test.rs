#![cfg(feature = "test-harness")]

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use dailyos_lib::abilities::feedback::FeedbackAction;
use dailyos_lib::db::{ActionDb, DbAccount};
use dailyos_lib::services::claims::{
    commit_claim, record_claim_feedback, ClaimFeedbackInput, ClaimProposal, CommittedClaim,
};
use dailyos_lib::services::context::{ExternalClients, SeedableRng, ServiceContext, SystemClock};
use dailyos_lib::state::AppState;
use rusqlite::params;

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

fn inserted_claim_id(committed: CommittedClaim) -> String {
    match committed {
        CommittedClaim::Inserted { claim } => claim.id,
        other => panic!("expected inserted claim, got {other:?}"),
    }
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

    let repair_job_ids = state
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
                let feedback = record_claim_feedback(&ctx, db, feedback_input(claim_id, action))
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
        .expect("seed feedback jobs");

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

    let targeted_repair_jobs: i64 = state
        .db_read(move |db| {
            db.conn_ref()
                .query_row(
                    "SELECT count(*)
                     FROM invalidation_jobs
                     WHERE job_kind = 'targeted_repair'
                       AND status = 'completed'",
                    [],
                    |row| row.get(0),
                )
                .map_err(|e| e.to_string())
        })
        .await
        .expect("read completed targeted repair count");
    assert_eq!(targeted_repair_jobs, 6);
}
