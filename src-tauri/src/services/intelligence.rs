// Intelligence service — extracted from commands.rs (I402)
// Business logic for entity intelligence CRUD, enrichment, and risk briefings.

use std::path::Path;

use crate::db::ActionDb;
use crate::signals::propagation::PropagationEngine;
use crate::state::AppState;

/// Enrich an entity via the intelligence queue (split-lock pattern).
pub async fn enrich_entity(
    entity_id: String,
    entity_type: String,
    state: &AppState,
) -> Result<crate::intelligence::IntelligenceJson, String> {
    use crate::intel_queue::{
        gather_enrichment_input, run_enrichment, write_enrichment_results, IntelPriority,
        IntelRequest,
    };

    let request = IntelRequest {
        entity_id,
        entity_type,
        priority: IntelPriority::Manual,
        requested_at: std::time::Instant::now(),
        retry_count: 0,
    };

    // Manual refresh: clear circuit breaker so enrichment proceeds (I410)
    let entity_id_for_reset = request.entity_id.clone();
    let _ = state
        .db_write(move |db| {
            crate::self_healing::scheduler::reset_circuit_breaker(db, &entity_id_for_reset);
            Ok(())
        })
        .await;

    let input = gather_enrichment_input(state, &request)?;

    let ai_config = state
        .config
        .read()
        .ok()
        .and_then(|g| g.as_ref().map(|c| c.ai_models.clone()))
        .unwrap_or_default();

    // Run PTY enrichment on a blocking thread and gate heavy work so UI-facing
    // manual refreshes cannot monopolize the async runtime.
    let _permit = state
        .heavy_work_semaphore
        .acquire()
        .await
        .map_err(|_| "Heavy work semaphore closed".to_string())?;
    let input_for_enrichment = input.clone();
    let ai_config_for_enrichment = ai_config.clone();
    let intel = tauri::async_runtime::spawn_blocking(move || {
        run_enrichment(&input_for_enrichment, &ai_config_for_enrichment)
    })
    .await
    .map_err(|e| format!("Enrichment task panicked: {}", e))??;

    let final_intel = write_enrichment_results(state, &input, &intel, Some(&ai_config))?;

    Ok(final_intel)
}

pub fn persist_entity_keywords(
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    keywords_json: &str,
) -> Result<(), String> {
    if entity_type != "account" && entity_type != "project" {
        return Ok(());
    }

    db.with_transaction(|tx| {
        match entity_type {
            "account" => tx
                .update_account_keywords(entity_id, keywords_json)
                .map_err(|e| format!("keywords update failed: {e}"))?,
            "project" => tx
                .update_project_keywords(entity_id, keywords_json)
                .map_err(|e| format!("keywords update failed: {e}"))?,
            _ => {}
        }

        crate::services::signals::emit(
            tx,
            entity_type,
            entity_id,
            "keywords_updated",
            "ai_enrichment",
            None,
            0.7,
        )
        .map_err(|e| format!("signal emit failed: {e}"))?;

        Ok(())
    })
}

pub fn upsert_assessment_from_enrichment(
    db: &ActionDb,
    engine: &PropagationEngine,
    entity_type: &str,
    entity_id: &str,
    intel: &crate::intelligence::IntelligenceJson,
) -> Result<(), String> {
    db.with_transaction(|tx| {
        tx.upsert_entity_intelligence(intel)
            .map_err(|e| e.to_string())?;
        crate::services::signals::emit_and_propagate(
            tx,
            engine,
            entity_type,
            entity_id,
            "entity_intelligence_updated",
            "ai_enrichment",
            None,
            0.8,
        )
        .map_err(|e| format!("signal emit failed: {e}"))?;
        Ok(())
    })
}

/// Update a single field in an entity's intelligence.json with signal emission.
pub async fn update_intelligence_field(
    entity_id: &str,
    entity_type: &str,
    field_path: &str,
    value: &str,
    state: &AppState,
) -> Result<(), String> {
    let config = state.config.read().map_err(|_| "Lock poisoned")?.clone();
    let config = config.ok_or("No configuration loaded")?;
    let workspace_path = config.workspace_path.clone();

    let entity_id = entity_id.to_string();
    let entity_type = entity_type.to_string();
    let field_path = field_path.to_string();
    let value = value.to_string();
    state
        .db_write(move |db| {
            let workspace = Path::new(&workspace_path);

            let account = if entity_type == "account" {
                db.get_account(&entity_id).map_err(|e| e.to_string())?
            } else {
                None
            };

            let entity_name = match entity_type.as_str() {
                "account" => account.as_ref().map(|a| a.name.clone()),
                "project" => db
                    .get_project(&entity_id)
                    .map_err(|e| e.to_string())?
                    .map(|p| p.name),
                "person" => db
                    .get_person(&entity_id)
                    .map_err(|e| e.to_string())?
                    .map(|p| p.name),
                _ => return Err(format!("Unsupported entity type: {}", entity_type)),
            }
            .ok_or_else(|| format!("{} '{}' not found", entity_type, entity_id))?;

            let dir = crate::intelligence::resolve_entity_dir(
                workspace,
                &entity_type,
                &entity_name,
                account.as_ref(),
            )?;

            let intel =
                crate::intelligence::apply_intelligence_field_update(&dir, &field_path, &value)?;

            db.with_transaction(|tx| {
                tx.upsert_entity_intelligence(&intel)
                    .map_err(|e| e.to_string())?;
                crate::services::signals::emit(
                    tx,
                    &entity_type,
                    &entity_id,
                    "user_correction",
                    "user_edit",
                    Some(&format!("{{\"field\":\"{}\"}}", field_path)),
                    1.0,
                )
                .map_err(|e| format!("signal emit failed: {e}"))?;
                Ok(())
            })?;

            // Self-healing: record user correction to lower quality score (I409)
            crate::self_healing::feedback::record_enrichment_correction(
                db,
                &entity_id,
                &entity_type,
                "intel_queue",
            );

            Ok(())
        })
        .await
}

/// Bulk-replace the stakeholder list in an entity's intelligence.json.
pub async fn update_stakeholders(
    entity_id: &str,
    entity_type: &str,
    stakeholders: Vec<crate::intelligence::StakeholderInsight>,
    state: &AppState,
) -> Result<(), String> {
    let config = state.config.read().map_err(|_| "Lock poisoned")?.clone();
    let config = config.ok_or("No configuration loaded")?;
    let workspace_path = config.workspace_path.clone();

    let engine = state.signals.engine.clone();
    let entity_id = entity_id.to_string();
    let entity_type = entity_type.to_string();
    state
        .db_write(move |db| {
            let workspace = Path::new(&workspace_path);

            let account = if entity_type == "account" {
                db.get_account(&entity_id).map_err(|e| e.to_string())?
            } else {
                None
            };

            let entity_name = match entity_type.as_str() {
                "account" => account.as_ref().map(|a| a.name.clone()),
                "project" => db
                    .get_project(&entity_id)
                    .map_err(|e| e.to_string())?
                    .map(|p| p.name),
                "person" => db
                    .get_person(&entity_id)
                    .map_err(|e| e.to_string())?
                    .map(|p| p.name),
                _ => return Err(format!("Unsupported entity type: {}", entity_type)),
            }
            .ok_or_else(|| format!("{} '{}' not found", entity_type, entity_id))?;

            let dir = crate::intelligence::resolve_entity_dir(
                workspace,
                &entity_type,
                &entity_name,
                account.as_ref(),
            )?;

            let intel = crate::intelligence::apply_stakeholders_update(&dir, stakeholders)?;

            db.with_transaction(|tx| {
                tx.upsert_entity_intelligence(&intel)
                    .map_err(|e| e.to_string())?;
                crate::services::signals::emit_and_propagate(
                    tx,
                    &engine,
                    &entity_type,
                    &entity_id,
                    "stakeholders_updated",
                    "user_edit",
                    None,
                    0.9,
                )
                .map_err(|e| format!("signal emit failed: {e}"))?;
                Ok(())
            })?;

            Ok(())
        })
        .await
}

/// Generate a risk briefing for an account (async, PTY enrichment).
pub async fn generate_risk_briefing(
    state: &std::sync::Arc<AppState>,
    account_id: &str,
) -> Result<crate::types::RiskBriefing, String> {
    let app_state = state.clone();
    let account_id = account_id.to_string();

    let task = tauri::async_runtime::spawn_blocking(move || {
        let input = {
            let db_guard = app_state
                .db
                .lock()
                .map_err(|_| "Lock poisoned".to_string())?;
            let db = db_guard
                .as_ref()
                .ok_or_else(|| "Database not initialized".to_string())?;

            let config_guard = app_state
                .config
                .read()
                .map_err(|_| "Config lock poisoned".to_string())?;
            let config = config_guard
                .as_ref()
                .ok_or_else(|| "Config not initialized".to_string())?;

            let workspace = std::path::Path::new(&config.workspace_path);
            crate::risk_briefing::gather_risk_input(
                workspace,
                db,
                &account_id,
                config.user_name.clone(),
                config.ai_models.clone(),
                app_state.context_provider.as_ref(),
            )?
        };

        let briefing = crate::risk_briefing::run_risk_enrichment(&input)?;

        // Store in reports table for unified tracking (I398)
        if let Ok(db) = crate::db::ActionDb::open() {
            let _ =
                crate::reports::risk::store_risk_briefing_in_reports(&db, &account_id, &briefing);
        }

        Ok(briefing)
    });

    match task.await {
        Ok(result) => result,
        Err(e) => Err(format!("Risk briefing task panicked: {}", e)),
    }
}

/// Read a cached risk briefing for an account (fast, no AI).
pub fn get_risk_briefing(
    db: &ActionDb,
    state: &AppState,
    account_id: &str,
) -> Result<crate::types::RiskBriefing, String> {
    // Try reports table first (I398 — DB-backed storage)
    if let Some(briefing) = crate::reports::risk::load_risk_briefing_from_reports(db, account_id) {
        return Ok(briefing);
    }

    // Fall back to disk (legacy path)
    let config_guard = state.config.read().map_err(|_| "Config lock poisoned")?;
    let config = config_guard.as_ref().ok_or("Config not initialized")?;

    let account = db
        .get_account(account_id)
        .map_err(|e| format!("DB error: {}", e))?
        .ok_or_else(|| format!("Account not found: {}", account_id))?;

    let workspace = std::path::Path::new(&config.workspace_path);
    let account_dir = crate::accounts::resolve_account_dir(workspace, &account);
    crate::risk_briefing::read_risk_briefing(&account_dir)
}

/// Save an edited risk briefing back to disk (user corrections).
pub fn save_risk_briefing(
    db: &ActionDb,
    state: &AppState,
    account_id: &str,
    briefing: &crate::types::RiskBriefing,
) -> Result<(), String> {
    let config_guard = state.config.read().map_err(|_| "Config lock poisoned")?;
    let config = config_guard.as_ref().ok_or("Config not initialized")?;

    let account = db
        .get_account(account_id)
        .map_err(|e| format!("DB error: {}", e))?
        .ok_or_else(|| format!("Account not found: {}", account_id))?;

    let workspace = std::path::Path::new(&config.workspace_path);
    let account_dir = crate::accounts::resolve_account_dir(workspace, &account);
    crate::risk_briefing::write_risk_briefing(&account_dir, briefing)
}

#[cfg(test)]
mod live_acceptance_tests {
    use std::sync::Arc;

    use chrono::Utc;
    use rusqlite::OptionalExtension;

    use super::enrich_entity;
    use crate::intel_queue::{write_enrichment_results, EnrichmentInput};
    use crate::intelligence::{
        read_intelligence_json, write_intelligence_json, ConsistencyStatus, IntelRisk,
        IntelligenceJson,
    };
    use crate::state::AppState;

    /// Live acceptance check for I527 using the user's real local dataset.
    /// Run manually:
    /// `cargo test --lib services::intelligence::live_acceptance_tests::i527_live_end_to_end_data_flow -- --ignored --nocapture`
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "Live validation: requires configured local DB/workspace and AI runtime"]
    async fn i527_live_end_to_end_data_flow() {
        let state = Arc::new(AppState::new());
        let _ = state.init_db_service().await;

        // Pick a real account/project entity that has attendee history.
        let candidate = state
            .db_read(|db| {
                let mut stmt = db
                    .conn_ref()
                    .prepare(
                        "SELECT me.entity_id, me.entity_type
                         FROM meeting_entities me
                         JOIN meeting_attendees ma ON ma.meeting_id = me.meeting_id
                         WHERE me.entity_type IN ('account', 'project')
                         GROUP BY me.entity_id, me.entity_type
                         ORDER BY COUNT(DISTINCT ma.person_id) DESC,
                                  COUNT(DISTINCT ma.meeting_id) DESC
                         LIMIT 1",
                    )
                    .map_err(|e| format!("prepare candidate query failed: {e}"))?;

                let mut rows = stmt
                    .query([])
                    .map_err(|e| format!("candidate query failed: {e}"))?;

                if let Some(row) = rows
                    .next()
                    .map_err(|e| format!("candidate row read failed: {e}"))?
                {
                    let entity_id: String = row
                        .get(0)
                        .map_err(|e| format!("candidate entity_id read failed: {e}"))?;
                    let entity_type: String = row
                        .get(1)
                        .map_err(|e| format!("candidate entity_type read failed: {e}"))?;
                    Ok(Some((entity_id, entity_type)))
                } else {
                    Ok(None)
                }
            })
            .await
            .expect("failed to select live entity candidate")
            .expect("no account/project with attendee evidence found in live DB");

        let (entity_id, entity_type) = candidate;
        eprintln!(
            "I527 live validation using entity: {} ({})",
            entity_id, entity_type
        );

        // End-to-end path: gather context -> AI enrichment -> deterministic consistency pass ->
        // write intelligence.json + DB cache.
        let intel = enrich_entity(entity_id.clone(), entity_type.clone(), &state)
            .await
            .expect("manual enrich_entity failed");

        assert!(
            intel.consistency_status.is_some(),
            "consistency_status must be set after enrichment write path"
        );
        assert!(
            intel.consistency_checked_at.is_some(),
            "consistency_checked_at must be set after enrichment write path"
        );

        let entity_id_for_db = entity_id.clone();
        let persisted = state
            .db_read(move |db| {
                db.get_entity_intelligence(&entity_id_for_db)
                    .map_err(|e| format!("get_entity_intelligence failed: {e}"))
            })
            .await
            .expect("DB read failed")
            .expect("persisted entity_intelligence row missing after enrichment");

        assert_eq!(
            persisted.consistency_status, intel.consistency_status,
            "DB cache consistency status should match write result",
        );
        assert!(
            persisted.consistency_checked_at.is_some(),
            "DB cache must persist consistency_checked_at"
        );

        // Pull one real linked meeting and run full briefing refresh path.
        let entity_id_for_meeting = entity_id.clone();
        let entity_type_for_meeting = entity_type.clone();
        let meeting_id = state
            .db_read(move |db| {
                db.conn_ref()
                    .query_row(
                        "SELECT meeting_id
                         FROM meeting_entities
                         WHERE entity_id = ?1 AND entity_type = ?2
                         ORDER BY meeting_id DESC
                         LIMIT 1",
                        rusqlite::params![entity_id_for_meeting, entity_type_for_meeting],
                        |row| row.get::<_, String>(0),
                    )
                    .optional()
                    .map_err(|e| format!("meeting lookup failed: {e}"))
            })
            .await
            .expect("meeting lookup query failed")
            .expect("no linked meeting found for entity");

        let refresh =
            crate::services::meetings::refresh_meeting_briefing_full(&state, &meeting_id, None)
                .await
                .expect("refresh_meeting_briefing_full failed");

        assert!(
            refresh.prep_rebuilt_sync || refresh.prep_queued,
            "refresh should rebuild prep sync or queue it"
        );

        let detail = crate::services::meetings::get_meeting_intelligence(&state, &meeting_id)
            .await
            .expect("get_meeting_intelligence failed after refresh");
        let prep = detail
            .prep
            .expect("meeting detail should include prep after manual refresh");
        assert!(
            prep.consistency_status.is_some(),
            "meeting prep should include propagated consistency_status"
        );
    }

    /// Live deterministic-guardrail validation for I527 acceptance criteria:
    /// - contradiction auto-correction/flagging
    /// - refresh overwrite (not stuck on corrected/flagged state)
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "Live validation: mutates one real entity intelligence row, then restores it"]
    async fn i527_live_deterministic_guardrail_acceptance() {
        let state = Arc::new(AppState::new());
        let _ = state.init_db_service().await;

        let workspace_path = state
            .config
            .read()
            .expect("config lock poisoned")
            .as_ref()
            .map(|c| c.workspace_path.clone())
            .expect("No config loaded");
        let workspace = std::path::Path::new(&workspace_path);

        let candidate = state
            .db_read(|db| {
                let mut stmt = db
                    .conn_ref()
                    .prepare(
                        "SELECT me.entity_id, me.entity_type
                         FROM meeting_entities me
                         JOIN meeting_attendees ma ON ma.meeting_id = me.meeting_id
                         LEFT JOIN signal_events se
                           ON se.entity_id = me.entity_id
                          AND se.entity_type = me.entity_type
                          AND se.superseded_by IS NULL
                          AND se.created_at >= datetime('now', '-14 days')
                         WHERE me.entity_type IN ('account', 'project')
                         GROUP BY me.entity_id, me.entity_type
                         HAVING COUNT(DISTINCT ma.meeting_id) >= 1
                            AND COUNT(DISTINCT se.id) >= 2
                         ORDER BY COUNT(DISTINCT se.id) DESC,
                                  COUNT(DISTINCT ma.meeting_id) DESC
                         LIMIT 1",
                    )
                    .map_err(|e| format!("prepare candidate query failed: {e}"))?;

                let mut rows = stmt
                    .query([])
                    .map_err(|e| format!("candidate query failed: {e}"))?;

                if let Some(row) = rows
                    .next()
                    .map_err(|e| format!("candidate row read failed: {e}"))?
                {
                    let entity_id: String = row
                        .get(0)
                        .map_err(|e| format!("candidate entity_id read failed: {e}"))?;
                    let entity_type: String = row
                        .get(1)
                        .map_err(|e| format!("candidate entity_type read failed: {e}"))?;
                    Ok(Some((entity_id, entity_type)))
                } else {
                    Ok(None)
                }
            })
            .await
            .expect("candidate lookup failed")
            .expect("no suitable live entity with attendee+signal evidence found");

        let (entity_id, entity_type) = candidate;
        let entity_dir = state
            .db_read({
                let entity_id = entity_id.clone();
                let entity_type = entity_type.clone();
                let workspace = workspace.to_path_buf();
                move |db| {
                    let account_opt = if entity_type == "account" {
                        db.get_account(&entity_id).map_err(|e| e.to_string())?
                    } else {
                        None
                    };
                    let entity_name = match entity_type.as_str() {
                        "account" => account_opt
                            .as_ref()
                            .map(|a| a.name.clone())
                            .ok_or_else(|| format!("account not found: {entity_id}"))?,
                        "project" => db
                            .get_project(&entity_id)
                            .map_err(|e| e.to_string())?
                            .map(|p| p.name)
                            .ok_or_else(|| format!("project not found: {entity_id}"))?,
                        _ => return Err(format!("unsupported entity_type: {}", entity_type)),
                    };
                    crate::intelligence::resolve_entity_dir(
                        &workspace,
                        &entity_type,
                        &entity_name,
                        account_opt.as_ref(),
                    )
                }
            })
            .await
            .expect("resolve entity dir failed");

        let facts = state
            .db_read({
                let entity_id = entity_id.clone();
                let entity_type = entity_type.clone();
                move |db| crate::intelligence::build_fact_context(db, &entity_id, &entity_type)
            })
            .await
            .expect("build_fact_context failed");

        let stakeholder = facts
            .stakeholders
            .iter()
            .find(|s| s.attendance_count > 0)
            .expect("candidate entity has no attendance-backed stakeholder")
            .name
            .clone();

        let previous_db = state
            .db_read({
                let entity_id = entity_id.clone();
                move |db| {
                    db.get_entity_intelligence(&entity_id)
                        .map_err(|e| e.to_string())
                }
            })
            .await
            .expect("previous DB read failed");
        let previous_file = read_intelligence_json(&entity_dir).ok();

        let contradictory = IntelligenceJson {
            version: 1,
            entity_id: entity_id.clone(),
            entity_type: entity_type.clone(),
            enriched_at: Utc::now().to_rfc3339(),
            executive_assessment: Some(format!(
                "{} has never appeared in a recorded meeting and no new progress signals since the prior assessment.",
                stakeholder
            )),
            risks: vec![IntelRisk {
                text: format!(
                    "{} has never appeared in a recorded meeting.",
                    stakeholder
                ),
                source: Some("live-acceptance-test".to_string()),
                urgency: "critical".to_string(),
            }],
            ..Default::default()
        };

        let input = EnrichmentInput {
            workspace: workspace.to_path_buf(),
            entity_dir: entity_dir.clone(),
            entity_id: entity_id.clone(),
            entity_type: entity_type.clone(),
            prompt: String::new(),
            file_manifest: Vec::new(),
            file_count: 0,
        };

        let first = write_enrichment_results(&state, &input, &contradictory, None)
            .expect("first write_enrichment_results failed");

        let first_assessment = first
            .executive_assessment
            .as_deref()
            .unwrap_or_default()
            .to_lowercase();
        assert!(
            !first_assessment.contains("never appeared"),
            "absence contradiction should be auto-corrected"
        );
        assert!(
            !first_assessment.contains("no new progress signals"),
            "no-progress contradiction should be auto-corrected when 14d signals >= 2"
        );
        assert!(
            matches!(
                first.consistency_status,
                Some(ConsistencyStatus::Corrected) | Some(ConsistencyStatus::Flagged)
            ),
            "contradictory payload must be corrected or flagged"
        );

        let clean = IntelligenceJson {
            version: 1,
            entity_id: entity_id.clone(),
            entity_type: entity_type.clone(),
            enriched_at: Utc::now().to_rfc3339(),
            executive_assessment: Some("Fresh validated summary from later refresh.".to_string()),
            ..Default::default()
        };
        let second = write_enrichment_results(&state, &input, &clean, None)
            .expect("second write_enrichment_results failed");

        assert!(
            second
                .executive_assessment
                .as_deref()
                .unwrap_or_default()
                .contains("Fresh validated summary"),
            "later refresh should overwrite prior corrected/flagged output"
        );
        assert!(
            second.consistency_checked_at.is_some(),
            "later refresh must still run a new consistency pass"
        );

        // Restore prior data so live workspace stays unchanged after validation.
        match previous_db {
            Some(prev) => {
                let _ = state
                    .db_write(move |db| {
                        db.upsert_entity_intelligence(&prev)
                            .map_err(|e| e.to_string())
                    })
                    .await;
            }
            None => {
                let entity_id_for_delete = entity_id.clone();
                let _ = state
                    .db_write(move |db| {
                        db.delete_entity_intelligence(&entity_id_for_delete)
                            .map_err(|e| e.to_string())
                    })
                    .await;
            }
        }

        if let Some(prev_file) = previous_file {
            let _ = write_intelligence_json(&entity_dir, &prev_file);
        } else {
            let _ = std::fs::remove_file(entity_dir.join("intelligence.json"));
        }
    }

    /// Live Janus scenario: if Matt Wickham has attendance evidence, a
    /// "never appeared" claim must be flagged/corrected.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "Live validation for Janus/Matt evidence path"]
    async fn i527_live_janus_matt_absence_guardrail() {
        let state = Arc::new(AppState::new());
        let _ = state.init_db_service().await;

        let entity_id = state
            .db_read(|db| {
                db.conn_ref()
                    .query_row(
                        "SELECT entity_id
                         FROM meeting_entities
                         WHERE entity_type = 'account'
                           AND LOWER(entity_id) LIKE '%janus%'
                         LIMIT 1",
                        [],
                        |row| row.get::<_, String>(0),
                    )
                    .optional()
                    .map_err(|e| format!("janus lookup failed: {e}"))
            })
            .await
            .expect("janus lookup query failed")
            .expect("No Janus entity linked in meeting_entities");

        let facts = state
            .db_read({
                let entity_id = entity_id.clone();
                move |db| crate::intelligence::build_fact_context(db, &entity_id, "account")
            })
            .await
            .expect("build_fact_context failed for Janus");

        let matt = facts
            .stakeholders
            .iter()
            .find(|s| s.name.to_lowercase().contains("wickham"))
            .expect("Matt Wickham not found in Janus stakeholder facts");
        assert!(
            matt.attendance_count >= 1,
            "Matt Wickham should have deterministic attendance evidence for this scenario"
        );

        let contradictory = IntelligenceJson {
            version: 1,
            entity_id: entity_id.clone(),
            entity_type: "account".to_string(),
            enriched_at: Utc::now().to_rfc3339(),
            executive_assessment: Some(
                "Matt Wickham has never appeared in a recorded meeting.".to_string(),
            ),
            ..Default::default()
        };

        let report = crate::intelligence::check_consistency(&contradictory, &facts);
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.code == "ABSENCE_CONTRADICTION"),
            "False absence claim should be detected for Janus/Matt"
        );

        let repaired =
            crate::intelligence::apply_deterministic_repairs(&contradictory, &report, &facts);
        let post = crate::intelligence::check_consistency(&repaired, &facts);
        assert!(
            post.findings
                .iter()
                .all(|f| f.code != "ABSENCE_CONTRADICTION"),
            "Deterministic repair should clear Janus/Matt absence contradiction"
        );
    }
}
