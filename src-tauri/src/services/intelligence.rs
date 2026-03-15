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
    app_handle: Option<&tauri::AppHandle>,
) -> Result<crate::intelligence::IntelligenceJson, String> {
    use crate::intel_queue::{
        gather_enrichment_input, run_enrichment, write_enrichment_results, IntelPriority,
        IntelRequest,
    };

    log::warn!(
        "[I535] enrich_entity ENTERED: entity_id={}, type={}, provider={}",
        entity_id,
        entity_type,
        state.context_provider().provider_name(),
    );

    let request = IntelRequest::new(entity_id, entity_type, IntelPriority::Manual);

    // Manual refresh: clear circuit breaker so enrichment proceeds (I410)
    let entity_id_for_reset = request.entity_id.clone();
    let _ = state
        .db_write(move |db| {
            crate::self_healing::scheduler::reset_circuit_breaker(db, &entity_id_for_reset);
            Ok(())
        })
        .await;

    let input = match gather_enrichment_input(state, &request) {
        Ok(input) => input,
        Err(e) => {
            log::warn!(
                "[I535] gather_enrichment_input FAILED for {}: {}",
                request.entity_id,
                e
            );
            return Err(e);
        }
    };

    let ai_config = state
        .config
        .read()
        .ok()
        .and_then(|g| g.as_ref().map(|c| c.ai_models.clone()))
        .unwrap_or_default();

    // I535/ADR-0100: Glean-first enrichment for manual refresh.
    // Try Glean chat if connected, fall back to PTY on failure.
    let _permit = state
        .heavy_work_semaphore
        .acquire()
        .await
        .map_err(|_| "Heavy work semaphore closed".to_string())?;

    let provider = state.context_provider();
    let is_remote = provider.is_remote();
    let glean_endpoint = provider.remote_endpoint().map(|s| s.to_string());
    log::warn!(
        "[I535] enrich_entity: provider={}, is_remote={}, endpoint={:?}, has_ctx={}, entity={} ({})",
        provider.provider_name(),
        is_remote,
        glean_endpoint.is_some(),
        input.intelligence_context.is_some(),
        input.entity_name,
        input.entity_type,
    );
    let parsed = if is_remote {
        // Try Glean-first path
        let mut glean_result = None;
        if let (Some(ref endpoint), Some(ref ctx)) = (&glean_endpoint, &input.intelligence_context) {
            let provider =
                crate::intelligence::glean_provider::GleanIntelligenceProvider::new(endpoint);
            match provider
                .enrich_entity(
                    &input.entity_id,
                    &input.entity_type,
                    &input.entity_name,
                    ctx,
                    input.relationship.as_deref(),
                    app_handle,
                )
                .await
            {
                Ok(intel) => {
                    log::info!(
                        "[I535] Manual Glean enrichment succeeded for {}",
                        input.entity_name
                    );
                    let inferred = if let Ok(raw) = serde_json::to_string(&intel) {
                        crate::intelligence::extract_inferred_relationships(&raw)
                    } else {
                        Vec::new()
                    };
                    glean_result = Some(crate::intel_queue::EnrichmentParseResult {
                        intel,
                        inferred_relationships: inferred,
                    });
                }
                Err(e) => {
                    log::warn!(
                        "[I535] Manual Glean enrichment failed for {}, falling back to PTY: {}",
                        input.entity_name,
                        e
                    );
                }
            }
        }

        match glean_result {
            Some(parsed) => parsed,
            None => {
                // Fallback to PTY
                let input_for_enrichment = input.clone();
                let ai_config_for_enrichment = ai_config.clone();
                let app_handle_clone = app_handle.cloned();
                tauri::async_runtime::spawn_blocking(move || {
                    run_enrichment(&input_for_enrichment, &ai_config_for_enrichment, app_handle_clone.as_ref())
                })
                .await
                .map_err(|e| format!("Enrichment task panicked: {}", e))??
            }
        }
    } else {
        // Local-only: direct PTY path
        let input_for_enrichment = input.clone();
        let ai_config_for_enrichment = ai_config.clone();
        let app_handle_clone = app_handle.cloned();
        tauri::async_runtime::spawn_blocking(move || {
            run_enrichment(&input_for_enrichment, &ai_config_for_enrichment, app_handle_clone.as_ref())
        })
        .await
        .map_err(|e| format!("Enrichment task panicked: {}", e))??
    };

    let final_intel = write_enrichment_results(state, &input, &parsed.intel, Some(&ai_config))?;
    if !parsed.inferred_relationships.is_empty() {
        let engine = state.signals.engine.clone();
        let entity_id_for_persist = input.entity_id.clone();
        let entity_type_for_persist = input.entity_type.clone();
        let inferred = parsed.inferred_relationships.clone();
        state
            .db_write(move |db| {
                upsert_inferred_relationships_from_enrichment(
                    db,
                    engine.as_ref(),
                    &entity_type_for_persist,
                    &entity_id_for_persist,
                    &inferred,
                )
                .map(|_| ())
            })
            .await?;
    }

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

/// Persist AI-inferred person relationships for an enrichment run (I504).
///
/// - Skips invalid/self edges.
/// - Never overwrites strong user-confirmed edges.
/// - Uses deterministic IDs so re-enrichment reinforces instead of duplicating.
/// - Emits `relationship_inferred` only when creating a new AI edge.
pub fn upsert_inferred_relationships_from_enrichment(
    db: &ActionDb,
    engine: &PropagationEngine,
    entity_type: &str,
    entity_id: &str,
    inferred: &[crate::intelligence::prompts::InferredRelationship],
) -> Result<usize, String> {
    if inferred.is_empty() {
        return Ok(0);
    }

    db.with_transaction(|tx| {
        let mut inserted = 0usize;

        for rel in inferred {
            if rel.from_person_id.trim().is_empty()
                || rel.to_person_id.trim().is_empty()
                || rel.from_person_id == rel.to_person_id
            {
                continue;
            }

            if rel
                .relationship_type
                .parse::<crate::db::person_relationships::RelationshipType>()
                .is_err()
            {
                log::warn!(
                    "intelligence service: skipping invalid inferred relationship type '{}'",
                    rel.relationship_type
                );
                continue;
            }

            let direction = if rel.relationship_type == "manager" {
                "directed"
            } else {
                "symmetric"
            };
            let mut from_person_id = rel.from_person_id.clone();
            let mut to_person_id = rel.to_person_id.clone();
            if direction == "symmetric" && from_person_id > to_person_id {
                std::mem::swap(&mut from_person_id, &mut to_person_id);
            }

            let existing = tx
                .get_relationships_between(&from_person_id, &to_person_id)
                .map_err(|e| format!("relationship lookup failed: {e}"))?;
            if existing
                .iter()
                .any(|r| r.source == "user_confirmed" && r.confidence >= 0.8)
            {
                continue;
            }

            let existing_ai = existing.iter().find(|r| r.source == "ai_enrichment");
            let relationship_id = existing_ai
                .map(|r| r.id.clone())
                .unwrap_or_else(|| format!("pr-ai-{from_person_id}-{to_person_id}"));

            tx.upsert_person_relationship(&crate::db::person_relationships::UpsertRelationship {
                id: &relationship_id,
                from_person_id: &from_person_id,
                to_person_id: &to_person_id,
                relationship_type: &rel.relationship_type,
                direction,
                confidence: 0.6,
                context_entity_id: Some(entity_id),
                context_entity_type: Some(entity_type),
                source: "ai_enrichment",
                rationale: rel.rationale.as_deref(),
            })
            .map_err(|e| format!("relationship upsert failed: {e}"))?;

            if existing_ai.is_none() {
                let signal_value = format!(
                    "{from_person_id} -> {to_person_id} ({})",
                    rel.relationship_type
                );
                crate::services::signals::emit_and_propagate(
                    tx,
                    engine,
                    entity_type,
                    entity_id,
                    "relationship_inferred",
                    "ai_enrichment",
                    Some(signal_value.as_str()),
                    0.6,
                )
                .map_err(|e| format!("relationship_inferred signal failed: {e}"))?;
                inserted += 1;
            }
        }

        Ok(inserted)
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

            // Read intelligence from DB (source of truth post-I513), not disk.
            // Fall back to disk if DB doesn't have it (legacy path).
            let existing_intel = db
                .get_entity_intelligence(&entity_id)
                .ok()
                .flatten();
            let intel = if let Some(existing) = existing_intel {
                crate::intelligence::apply_intelligence_field_update_in_memory(
                    existing,
                    &field_path,
                    &value,
                )?
            } else {
                crate::intelligence::apply_intelligence_field_update(&dir, &field_path, &value)?
            };
            // Write to disk for MCP sidecar compatibility (best-effort)
            let _ = crate::intelligence::write_intelligence_json(&dir, &intel);

            // I530: Distinguish curation (delete/clear) from correction (edit).
            // Empty value = user removed the item → curation, no source penalty.
            // Non-empty value = user corrected the item → correction, source penalized.
            let is_curation = value.trim().is_empty()
                || value == "[]"
                || value == "null";

            db.with_transaction(|tx| {
                tx.upsert_entity_intelligence(&intel)
                    .map_err(|e| e.to_string())?;
                let (signal_type, source, confidence) = if is_curation {
                    ("intelligence_curated", "user_curation", 0.5)
                } else {
                    ("user_correction", "user_edit", 1.0)
                };
                crate::services::signals::emit(
                    tx,
                    &entity_type,
                    &entity_id,
                    signal_type,
                    source,
                    Some(&format!("{{\"field\":\"{}\"}}", field_path)),
                    confidence,
                )
                .map_err(|e| format!("signal emit failed: {e}"))?;
                Ok(())
            })?;

            // Self-healing: only record correction (not curation) to lower quality score
            if !is_curation {
                crate::self_healing::feedback::record_enrichment_correction(
                    db,
                    &entity_id,
                    &entity_type,
                    "intel_queue",
                );
            }

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

            // Capture linked stakeholders with scoring-relevant roles before
            // the vec is consumed by apply_stakeholders_update.
            let scoring_roles: Vec<(String, String)> = if entity_type == "account" {
                stakeholders
                    .iter()
                    .filter_map(|s| {
                        let role = s.role.as_deref().unwrap_or("").to_lowercase();
                        let person_id = s.person_id.as_deref()?;
                        if role.contains("champion")
                            || role.contains("executive")
                            || role.contains("technical")
                            || role.contains("decision")
                        {
                            Some((person_id.to_string(), role))
                        } else {
                            None
                        }
                    })
                    .collect()
            } else {
                Vec::new()
            };

            let intel = crate::intelligence::apply_stakeholders_update(&dir, stakeholders)?;

            db.with_transaction(|tx| {
                tx.upsert_entity_intelligence(&intel)
                    .map_err(|e| e.to_string())?;

                // Sync scoring-relevant stakeholder roles to account_stakeholders
                // so health scoring (champion health, stakeholder coverage) picks them up.
                for (person_id, role) in &scoring_roles {
                    tx.add_account_team_member(&entity_id, person_id, role)
                        .map_err(|e| {
                            log::warn!("Stakeholder sync to account_stakeholders failed for {person_id}: {e}");
                            e.to_string()
                        })
                        .ok(); // Non-fatal — don't fail the whole update
                }

                // Recompute health immediately so stakeholder changes reflect
                // in champion_health + stakeholder_coverage dimensions without
                // waiting for a full enrichment cycle.
                if entity_type == "account" && !scoring_roles.is_empty() {
                    if let Some(acct) = account.as_ref() {
                        let health = crate::intelligence::health_scoring::compute_account_health(tx, acct, None);
                        let health_json = serde_json::to_string(&health).ok();
                        tx.conn.execute(
                            "UPDATE entity_assessment SET health_json = ?1 WHERE entity_id = ?2",
                            rusqlite::params![health_json, entity_id],
                        ).ok();
                        tx.conn.execute(
                            "INSERT INTO entity_quality (entity_id, entity_type, health_score, health_trend)
                             VALUES (?1, 'account', ?2, ?3)
                             ON CONFLICT(entity_id) DO UPDATE SET
                                 health_score = excluded.health_score,
                                 health_trend = excluded.health_trend",
                            rusqlite::params![
                                entity_id,
                                health.score,
                                serde_json::to_string(&health.trend).ok(),
                            ],
                        ).ok();
                    }
                }

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

/// I576: Dismiss an intelligence item, creating a tombstone to prevent re-creation.
///
/// Removes the item from the specified Vec field and adds a `DismissedItem`
/// tombstone that prevents future enrichment from re-creating it.
pub async fn dismiss_intelligence_item(
    entity_id: &str,
    entity_type: &str,
    field: &str,
    item_text: &str,
    state: &AppState,
) -> Result<(), String> {
    let config = state.config.read().map_err(|_| "Lock poisoned")?.clone();
    let config = config.ok_or("No configuration loaded")?;
    let workspace_path = config.workspace_path.clone();

    let engine = state.signals.engine.clone();
    let entity_id = entity_id.to_string();
    let entity_type = entity_type.to_string();
    let field = field.to_string();
    let item_text = item_text.to_string();
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

            let mut intel = crate::intelligence::read_intelligence_json(&dir)?;

            // Add tombstone
            intel.dismissed_items.push(crate::intelligence::DismissedItem {
                field: field.clone(),
                content: item_text.clone(),
                dismissed_at: chrono::Utc::now().to_rfc3339(),
            });

            // Remove item from the relevant Vec by matching text
            let item_lower = item_text.to_lowercase();
            match field.as_str() {
                "risks" => intel.risks.retain(|r| !r.text.to_lowercase().contains(&item_lower)),
                "recentWins" => intel.recent_wins.retain(|w| !w.text.to_lowercase().contains(&item_lower)),
                "stakeholderInsights" => intel.stakeholder_insights.retain(|s| !s.name.to_lowercase().contains(&item_lower)),
                "valueDelivered" => intel.value_delivered.retain(|v| !v.statement.to_lowercase().contains(&item_lower)),
                "competitiveContext" => intel.competitive_context.retain(|c| !c.competitor.to_lowercase().contains(&item_lower)),
                "organizationalChanges" => intel.organizational_changes.retain(|o| !o.person.to_lowercase().contains(&item_lower)),
                "expansionSignals" => intel.expansion_signals.retain(|e| !e.opportunity.to_lowercase().contains(&item_lower)),
                "openCommitments" => {
                    if let Some(ref mut ocs) = intel.open_commitments {
                        ocs.retain(|c| !c.description.to_lowercase().contains(&item_lower));
                    }
                }
                _ => return Err(format!("Cannot dismiss items from field: {}", field)),
            }

            crate::intelligence::write_intelligence_json(&dir, &intel)?;

            db.with_transaction(|tx| {
                tx.upsert_entity_intelligence(&intel)
                    .map_err(|e| e.to_string())?;
                crate::services::signals::emit_and_propagate(
                    tx,
                    &engine,
                    &entity_type,
                    &entity_id,
                    "intelligence_curated",
                    "user_curation",
                    Some(&format!("{{\"field\":\"{field}\",\"dismissed\":\"{item_text}\"}}",)),
                    0.5,
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
                &*app_state.context_provider(),
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


#[cfg(test)]
mod inferred_relationship_tests {
    use super::upsert_inferred_relationships_from_enrichment;
    use crate::db::person_relationships::UpsertRelationship;
    use crate::db::test_utils::test_db;
    use crate::intelligence::prompts::InferredRelationship;

    fn seed_people(db: &crate::db::ActionDb) {
        db.conn_ref()
            .execute(
                "INSERT INTO people (id, email, name, updated_at) VALUES ('p1', 'p1@example.com', 'Alice', '2026-03-01T00:00:00Z')",
                [],
            )
            .expect("seed p1");
        db.conn_ref()
            .execute(
                "INSERT INTO people (id, email, name, updated_at) VALUES ('p2', 'p2@example.com', 'Bob', '2026-03-01T00:00:00Z')",
                [],
            )
            .expect("seed p2");
    }

    #[test]
    fn upsert_inferred_relationships_inserts_and_reinforces_without_duplicates() {
        let db = test_db();
        seed_people(&db);
        let engine = crate::signals::propagation::PropagationEngine::default();
        let inferred = vec![InferredRelationship {
            from_person_id: "p1".to_string(),
            to_person_id: "p2".to_string(),
            relationship_type: "collaborator".to_string(),
            rationale: Some("They co-own onboarding rollout workstreams.".to_string()),
        }];

        let inserted = upsert_inferred_relationships_from_enrichment(
            &db, &engine, "account", "acc-1", &inferred,
        )
        .expect("first upsert");
        assert_eq!(inserted, 1);

        let rels = db
            .get_relationships_between("p1", "p2")
            .expect("relationship query");
        assert_eq!(rels.len(), 1);
        assert_eq!(rels[0].source, "ai_enrichment");
        assert!((rels[0].confidence - 0.6).abs() < f64::EPSILON);
        assert_eq!(rels[0].direction, "symmetric");
        assert_eq!(rels[0].context_entity_id.as_deref(), Some("acc-1"));
        assert_eq!(
            rels[0].rationale.as_deref(),
            Some("They co-own onboarding rollout workstreams.")
        );

        let signal_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM signal_events
                 WHERE entity_type = 'account'
                   AND entity_id = 'acc-1'
                   AND signal_type = 'relationship_inferred'
                   AND source = 'ai_enrichment'",
                [],
                |row| row.get(0),
            )
            .expect("signal count");
        assert_eq!(signal_count, 1);

        let inserted_second = upsert_inferred_relationships_from_enrichment(
            &db, &engine, "account", "acc-1", &inferred,
        )
        .expect("second upsert");
        assert_eq!(
            inserted_second, 0,
            "re-enrichment should reinforce, not duplicate"
        );
        assert_eq!(
            db.get_relationships_between("p1", "p2")
                .expect("relationship query 2")
                .len(),
            1
        );
    }

    #[test]
    fn upsert_inferred_relationships_skips_strong_user_confirmed_edges() {
        let db = test_db();
        seed_people(&db);
        let engine = crate::signals::propagation::PropagationEngine::default();
        db.upsert_person_relationship(&UpsertRelationship {
            id: "rel-user-1",
            from_person_id: "p1",
            to_person_id: "p2",
            relationship_type: "peer",
            direction: "symmetric",
            confidence: 0.9,
            context_entity_id: Some("acc-1"),
            context_entity_type: Some("account"),
            source: "user_confirmed",
            rationale: None,
        })
        .expect("seed user relationship");

        let inferred = vec![InferredRelationship {
            from_person_id: "p1".to_string(),
            to_person_id: "p2".to_string(),
            relationship_type: "manager".to_string(),
            rationale: Some("Model inferred reporting relationship.".to_string()),
        }];

        let inserted = upsert_inferred_relationships_from_enrichment(
            &db, &engine, "account", "acc-1", &inferred,
        )
        .expect("upsert");
        assert_eq!(inserted, 0);

        let rels = db
            .get_relationships_between("p1", "p2")
            .expect("relationship query");
        assert_eq!(rels.len(), 1);
        assert_eq!(rels[0].source, "user_confirmed");
        assert_eq!(rels[0].relationship_type.to_string(), "peer");
    }
}

#[cfg(test)]
mod live_acceptance_tests {
    use std::collections::HashSet;
    use std::path::PathBuf;
    use std::sync::Arc;

    use chrono::Utc;
    use rusqlite::{params, OptionalExtension};

    use super::enrich_entity;
    use crate::db::data_lifecycle::{purge_source, DataSource};
    use crate::db::{ActionDb, DbPerson};
    use crate::intel_queue::{write_enrichment_results, EnrichmentInput};
    use crate::intelligence::{
        write_intelligence_json, AccountHealth, ConsistencyStatus, DimensionScore, HealthSource,
        HealthTrend, IntelRisk, IntelligenceJson, RelationshipDimensions,
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
        let intel = enrich_entity(entity_id.clone(), entity_type.clone(), &state, None)
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
        let previous_file = previous_db.clone();

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
                item_source: None,
                discrepancy: None,
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
            computed_health: None,
            entity_name: String::new(),
            relationship: None,
            intelligence_context: None,
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

    /// Wave 1 live acceptance (I503 + I528) on an encrypted snapshot of the
    /// user's real DB. Safe: mutates backup only.
    #[test]
    #[ignore = "Live validation: uses real DB snapshot and performs destructive purge checks on snapshot only"]
    fn wave1_live_snapshot_i503_i528_acceptance() {
        let live_db = ActionDb::open().expect("open live DB");
        let backup_path = crate::db_backup::backup_database(&live_db).expect("create live backup");
        let snapshot_db =
            ActionDb::open_at(PathBuf::from(&backup_path)).expect("open snapshot backup DB");

        // ---------------------------------------------------------------------
        // I503: structured health write/read + legacy compatibility
        // ---------------------------------------------------------------------
        let structured = IntelligenceJson {
            version: 1,
            entity_id: "wave1-i503-structured".to_string(),
            entity_type: "account".to_string(),
            enriched_at: Utc::now().to_rfc3339(),
            health: Some(AccountHealth {
                score: 82.0,
                band: "green".to_string(),
                source: HealthSource::Computed,
                confidence: 0.78,
                trend: HealthTrend {
                    direction: "improving".to_string(),
                    rationale: Some("Usage and expansion improved".to_string()),
                    timeframe: "30d".to_string(),
                    confidence: 0.7,
                },
                dimensions: RelationshipDimensions {
                    meeting_cadence: DimensionScore {
                        score: 80.0,
                        weight: 0.2,
                        evidence: vec!["weekly exec sync".to_string()],
                        trend: "improving".to_string(),
                    },
                    email_engagement: DimensionScore::default(),
                    stakeholder_coverage: DimensionScore::default(),
                    champion_health: DimensionScore::default(),
                    financial_proximity: DimensionScore::default(),
                    signal_momentum: DimensionScore::default(),
                },
                divergence: None,
                narrative: Some("Healthy multi-threaded account".to_string()),
                recommended_actions: vec!["Expand to procurement".to_string()],
            }),
            ..Default::default()
        };
        snapshot_db
            .upsert_entity_intelligence(&structured)
            .expect("upsert structured health");
        let structured_back = snapshot_db
            .get_entity_intelligence("wave1-i503-structured")
            .expect("get structured health row")
            .expect("structured row missing");
        let structured_health = structured_back
            .health
            .expect("structured health should deserialize");
        assert_eq!(structured_health.score, 82.0);
        assert_eq!(structured_health.band, "green");
        assert_eq!(structured_health.trend.direction, "improving");

        snapshot_db
            .conn_ref()
            .execute(
                "INSERT OR REPLACE INTO entity_assessment (entity_id, entity_type, enriched_at, source_file_count)
                 VALUES (?1, 'account', ?2, 0)",
                params!["wave1-i503-legacy", Utc::now().to_rfc3339()],
            )
            .expect("seed legacy entity_assessment row");
        let legacy_trend_json = serde_json::json!({
            "direction": "declining",
            "rationale": "Legacy trend payload"
        })
        .to_string();
        snapshot_db
            .conn_ref()
            .execute(
                "INSERT OR REPLACE INTO entity_quality (entity_id, entity_type, health_score, health_trend)
                 VALUES (?1, 'account', 35.0, ?2)",
                params!["wave1-i503-legacy", legacy_trend_json],
            )
            .expect("seed legacy entity_quality row");
        let legacy_back = snapshot_db
            .get_entity_intelligence("wave1-i503-legacy")
            .expect("get legacy compatibility row")
            .expect("legacy row missing");
        let legacy_health = legacy_back
            .health
            .expect("legacy scalar health should synthesize into structured health");
        assert_eq!(legacy_health.band, "red");
        assert_eq!(legacy_health.score, 35.0);
        assert_eq!(legacy_health.trend.direction, "declining");

        let legacy_dir = tempfile::tempdir().expect("legacy tempdir");
        let legacy_json = serde_json::json!({
            "entityId": "legacy-file-entity",
            "entityType": "account",
            "healthScore": 73.0,
            "healthTrend": {
                "direction": "stable",
                "rationale": "Legacy file compatibility"
            }
        });
        std::fs::write(
            legacy_dir.path().join("intelligence.json"),
            serde_json::to_string_pretty(&legacy_json).expect("serialize legacy intelligence file"),
        )
        .expect("write legacy intelligence file");
        let parsed_legacy_file = crate::intelligence::read_intelligence_json(legacy_dir.path())
            .expect("read legacy intelligence file");
        let file_health = parsed_legacy_file
            .health
            .expect("healthScore/healthTrend should map to structured health");
        assert_eq!(file_health.score, 73.0);
        assert_eq!(file_health.band, "green");
        assert_eq!(file_health.trend.direction, "stable");

        // ---------------------------------------------------------------------
        // I528: purge semantics (glean + google) against snapshot
        // ---------------------------------------------------------------------
        let marker = format!("wave1-i528-{}", Utc::now().timestamp());
        let account_id: String = snapshot_db
            .conn_ref()
            .query_row("SELECT id FROM accounts LIMIT 1", [], |row| row.get(0))
            .expect("load existing account id for FK-safe purge seeding");

        let google_person_id = format!("{marker}-p-google");
        let glean_person_id = format!("{marker}-p-glean");
        let user_person_id = format!("{marker}-p-user");
        let google_enrichment_sources = serde_json::json!({
            "linkedin_url": {"source": "google", "at": "2026-03-07T00:00:00Z"},
            "bio": {"source": "user", "at": "2026-03-07T00:00:00Z"}
        })
        .to_string();
        let person = DbPerson {
            id: google_person_id.clone(),
            email: format!("{marker}-google@example.com"),
            name: format!("{marker}-google"),
            organization: Some("Wave1 Org".to_string()),
            role: Some("Director".to_string()),
            relationship: "external".to_string(),
            notes: None,
            tracker_path: None,
            last_seen: None,
            first_seen: None,
            meeting_count: 0,
            updated_at: Utc::now().to_rfc3339(),
            archived: false,
            linkedin_url: Some("https://linkedin.com/in/wave1".to_string()),
            twitter_handle: None,
            phone: None,
            photo_url: None,
            bio: Some("Wave1 profile".to_string()),
            title_history: None,
            company_industry: None,
            company_size: None,
            company_hq: None,
            last_enriched_at: None,
            enrichment_sources: Some(google_enrichment_sources.clone()),
        };
        snapshot_db
            .upsert_person(&person)
            .expect("seed snapshot person");
        snapshot_db
            .conn_ref()
            .execute(
                "UPDATE people
                 SET linkedin_url = ?1, bio = ?2, enrichment_sources = ?3
                 WHERE id = ?4",
                params![
                    "https://linkedin.com/in/wave1",
                    "Wave1 profile",
                    google_enrichment_sources,
                    google_person_id.clone()
                ],
            )
            .expect("seed people profile fields");
        snapshot_db
            .upsert_person(&DbPerson {
                id: glean_person_id.clone(),
                email: format!("{marker}-glean@example.com"),
                name: format!("{marker}-glean"),
                organization: Some("Wave1 Org".to_string()),
                role: Some("Champion".to_string()),
                relationship: "external".to_string(),
                notes: None,
                tracker_path: None,
                last_seen: None,
                first_seen: None,
                meeting_count: 0,
                updated_at: Utc::now().to_rfc3339(),
                archived: false,
                linkedin_url: None,
                twitter_handle: None,
                phone: None,
                photo_url: None,
                bio: None,
                title_history: None,
                company_industry: None,
                company_size: None,
                company_hq: None,
                last_enriched_at: None,
                enrichment_sources: None,
            })
            .expect("seed glean person");
        snapshot_db
            .upsert_person(&DbPerson {
                id: user_person_id.clone(),
                email: format!("{marker}-user@example.com"),
                name: format!("{marker}-user"),
                organization: Some("Wave1 Org".to_string()),
                role: Some("Champion".to_string()),
                relationship: "external".to_string(),
                notes: None,
                tracker_path: None,
                last_seen: None,
                first_seen: None,
                meeting_count: 0,
                updated_at: Utc::now().to_rfc3339(),
                archived: false,
                linkedin_url: None,
                twitter_handle: None,
                phone: None,
                photo_url: None,
                bio: None,
                title_history: None,
                company_industry: None,
                company_size: None,
                company_hq: None,
                last_enriched_at: None,
                enrichment_sources: None,
            })
            .expect("seed user person");

        let account_glean = account_id.clone();
        let account_google = account_id.clone();
        let account_user = account_id.clone();

        let user_stakeholders_before: i64 = snapshot_db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM account_stakeholders WHERE data_source = 'user'",
                [],
                |row| row.get(0),
            )
            .expect("count user stakeholders before");
        let user_signals_before: i64 = snapshot_db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM signal_events WHERE source = 'user'",
                [],
                |row| row.get(0),
            )
            .expect("count user signals before");
        let user_relationships_before: i64 = snapshot_db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM person_relationships WHERE source = 'user_confirmed'",
                [],
                |row| row.get(0),
            )
            .expect("count user relationships before");

        snapshot_db
            .conn_ref()
            .execute(
                "INSERT INTO account_stakeholders (account_id, person_id, role, data_source)
                 VALUES (?1, ?2, 'champion', 'glean')",
                params![account_glean, glean_person_id],
            )
            .expect("seed glean stakeholder");
        snapshot_db
            .conn_ref()
            .execute(
                "INSERT INTO account_stakeholders (account_id, person_id, role, data_source)
                 VALUES (?1, ?2, 'champion', 'google')",
                params![account_google, google_person_id],
            )
            .expect("seed google stakeholder");
        snapshot_db
            .conn_ref()
            .execute(
                "INSERT INTO account_stakeholders (account_id, person_id, role, data_source)
                 VALUES (?1, ?2, 'champion', 'user')",
                params![account_user, user_person_id],
            )
            .expect("seed user stakeholder");

        snapshot_db
            .conn_ref()
            .execute(
                "INSERT INTO signal_events (id, entity_type, entity_id, signal_type, source, confidence)
                 VALUES (?1, 'account', ?2, 'profile_update', 'glean', 0.8)",
                params![format!("{marker}-sig-glean"), account_glean],
            )
            .expect("seed glean signal");
        snapshot_db
            .conn_ref()
            .execute(
                "INSERT INTO signal_events (id, entity_type, entity_id, signal_type, source, confidence)
                 VALUES (?1, 'account', ?2, 'profile_update', 'google', 0.8)",
                params![format!("{marker}-sig-google"), account_google],
            )
            .expect("seed google signal");
        snapshot_db
            .conn_ref()
            .execute(
                "INSERT INTO signal_events (id, entity_type, entity_id, signal_type, source, confidence)
                 VALUES (?1, 'account', ?2, 'profile_update', 'user', 0.8)",
                params![format!("{marker}-sig-user"), account_user],
            )
            .expect("seed user signal");

        snapshot_db
            .conn_ref()
            .execute(
                "INSERT INTO person_relationships
                 (id, from_person_id, to_person_id, relationship_type, direction, confidence, source)
                 VALUES (?1, ?2, ?2, 'peer', 'symmetric', 0.8, 'glean')",
                params![format!("{marker}-rel-glean"), glean_person_id],
            )
            .expect("seed glean relationship");
        snapshot_db
            .conn_ref()
            .execute(
                "INSERT INTO person_relationships
                 (id, from_person_id, to_person_id, relationship_type, direction, confidence, source)
                 VALUES (?1, ?2, ?2, 'peer', 'symmetric', 0.8, 'google')",
                params![format!("{marker}-rel-google"), google_person_id],
            )
            .expect("seed google relationship");
        snapshot_db
            .conn_ref()
            .execute(
                "INSERT INTO person_relationships
                 (id, from_person_id, to_person_id, relationship_type, direction, confidence, source)
                 VALUES (?1, ?2, ?2, 'peer', 'symmetric', 0.9, 'user_confirmed')",
                params![format!("{marker}-rel-user"), user_person_id],
            )
            .expect("seed user relationship");

        let glean_report = purge_source(&snapshot_db, DataSource::Glean).expect("purge glean");
        assert_eq!(glean_report.source, "glean");
        assert!(
            glean_report.people_cleared >= 1
                && glean_report.signals_deleted >= 1
                && glean_report.relationships_deleted >= 1,
            "glean purge should remove source-owned records"
        );

        let glean_stakeholders_left: i64 = snapshot_db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM account_stakeholders WHERE data_source = 'glean'",
                [],
                |row| row.get(0),
            )
            .expect("count glean stakeholders");
        let glean_signals_left: i64 = snapshot_db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM signal_events WHERE source = 'glean'",
                [],
                |row| row.get(0),
            )
            .expect("count glean signals");
        let glean_relationships_left: i64 = snapshot_db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM person_relationships WHERE source = 'glean'",
                [],
                |row| row.get(0),
            )
            .expect("count glean relationships");
        assert_eq!(glean_stakeholders_left, 0);
        assert_eq!(glean_signals_left, 0);
        assert_eq!(glean_relationships_left, 0);

        let user_stakeholders_mid: i64 = snapshot_db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM account_stakeholders WHERE data_source = 'user'",
                [],
                |row| row.get(0),
            )
            .expect("count user stakeholders after glean purge");
        let user_signals_mid: i64 = snapshot_db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM signal_events WHERE source = 'user'",
                [],
                |row| row.get(0),
            )
            .expect("count user signals after glean purge");
        let user_relationships_mid: i64 = snapshot_db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM person_relationships WHERE source = 'user_confirmed'",
                [],
                |row| row.get(0),
            )
            .expect("count user relationships after glean purge");
        assert_eq!(user_stakeholders_mid, user_stakeholders_before + 1);
        assert_eq!(user_signals_mid, user_signals_before + 1);
        assert_eq!(user_relationships_mid, user_relationships_before + 1);

        let google_report = purge_source(&snapshot_db, DataSource::Google).expect("purge google");
        assert_eq!(google_report.source, "google");
        assert!(
            google_report.people_cleared >= 1
                && google_report.signals_deleted >= 1
                && google_report.relationships_deleted >= 1,
            "google purge should remove source-owned records"
        );

        let google_stakeholders_left: i64 = snapshot_db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM account_stakeholders WHERE data_source = 'google'",
                [],
                |row| row.get(0),
            )
            .expect("count google stakeholders");
        let google_signals_left: i64 = snapshot_db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM signal_events WHERE source = 'google'",
                [],
                |row| row.get(0),
            )
            .expect("count google signals");
        let google_relationships_left: i64 = snapshot_db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM person_relationships WHERE source = 'google'",
                [],
                |row| row.get(0),
            )
            .expect("count google relationships");
        assert_eq!(google_stakeholders_left, 0);
        assert_eq!(google_signals_left, 0);
        assert_eq!(google_relationships_left, 0);

        let user_stakeholders_after: i64 = snapshot_db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM account_stakeholders WHERE data_source = 'user'",
                [],
                |row| row.get(0),
            )
            .expect("count user stakeholders after google purge");
        let user_signals_after: i64 = snapshot_db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM signal_events WHERE source = 'user'",
                [],
                |row| row.get(0),
            )
            .expect("count user signals after google purge");
        let user_relationships_after: i64 = snapshot_db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM person_relationships WHERE source = 'user_confirmed'",
                [],
                |row| row.get(0),
            )
            .expect("count user relationships after google purge");
        assert_eq!(user_stakeholders_after, user_stakeholders_before + 1);
        assert_eq!(user_signals_after, user_signals_before + 1);
        assert_eq!(user_relationships_after, user_relationships_before + 1);

        let (linkedin_after, bio_after, sources_after): (
            Option<String>,
            Option<String>,
            Option<String>,
        ) = snapshot_db
            .conn_ref()
            .query_row(
                "SELECT linkedin_url, bio, enrichment_sources FROM people WHERE id = ?1",
                params![google_person_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("read person profile after google purge");
        assert!(
            linkedin_after.is_none(),
            "google-owned linkedin_url should be cleared; got linkedin={:?}, sources={:?}",
            linkedin_after,
            sources_after
        );
        assert_eq!(
            bio_after.as_deref(),
            Some("Wave1 profile"),
            "user-owned bio should remain"
        );
    }

    /// Wave 1 live acceptance (I504) against real data enrichment path.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "Live validation: runs real AI enrichment and checks inferred relationships in local DB"]
    async fn wave1_live_i504_end_to_end_relationship_acceptance() {
        let state = Arc::new(AppState::new());
        let _ = state.init_db_service().await;

        // Prefer an account that already has AI-inferred edges and >=3 stakeholders
        // for deterministic validation across reruns.
        let candidate = state
            .db_read(|db| {
                db.conn_ref()
                    .query_row(
                        "SELECT pr.context_entity_id
                         FROM person_relationships pr
                         JOIN account_stakeholders s
                           ON s.account_id = pr.context_entity_id
                         WHERE pr.source = 'ai_enrichment'
                           AND pr.context_entity_type = 'account'
                         GROUP BY pr.context_entity_id
                         HAVING COUNT(DISTINCT s.person_id) >= 3
                         ORDER BY COUNT(*) ASC, COUNT(DISTINCT s.person_id) ASC
                         LIMIT 1",
                        [],
                        |row| row.get::<_, String>(0),
                    )
                    .optional()
                    .map_err(|e| format!("preferred candidate query failed: {e}"))
            })
            .await
            .expect("preferred candidate query error");

        let account_id = if let Some(id) = candidate {
            id
        } else {
            state
                .db_read(|db| {
                    db.conn_ref()
                        .query_row(
                            "SELECT s.account_id
                             FROM account_stakeholders s
                             GROUP BY s.account_id
                             HAVING COUNT(DISTINCT s.person_id) >= 3
                             ORDER BY COUNT(DISTINCT s.person_id) ASC
                             LIMIT 1",
                            [],
                            |row| row.get::<_, String>(0),
                        )
                        .map_err(|e| format!("fallback candidate query failed: {e}"))
                })
                .await
                .expect("fallback candidate query error")
        };

        let (before_rows, before_signals): (i64, i64) = state
            .db_read({
                let account_id = account_id.clone();
                move |db| {
                    let rows: i64 = db
                        .conn_ref()
                        .query_row(
                            "SELECT COUNT(*)
                             FROM person_relationships
                             WHERE source = 'ai_enrichment'
                               AND context_entity_type = 'account'
                               AND context_entity_id = ?1",
                            params![account_id],
                            |row| row.get(0),
                        )
                        .map_err(|e| format!("count AI relationships before failed: {e}"))?;
                    let signals: i64 = db
                        .conn_ref()
                        .query_row(
                            "SELECT COUNT(*)
                             FROM signal_events
                             WHERE entity_type = 'account'
                               AND entity_id = ?1
                               AND signal_type = 'relationship_inferred'
                               AND source = 'ai_enrichment'",
                            params![account_id],
                            |row| row.get(0),
                        )
                        .map_err(|e| format!("count relationship signals before failed: {e}"))?;
                    Ok((rows, signals))
                }
            })
            .await
            .expect("read i504 pre-state failed");

        let _ = enrich_entity(account_id.clone(), "account".to_string(), &state, None)
            .await
            .expect("manual enrich_entity for i504 validation failed");

        let (rows_after_first, ids_after_first, manager_bad, peer_bad, signals_after_first): (
            Vec<(String, f64, String, String, Option<String>)>,
            HashSet<String>,
            i64,
            i64,
            i64,
        ) = state
            .db_read({
                let account_id = account_id.clone();
                move |db| {
                    let mut stmt = db
                        .conn_ref()
                        .prepare(
                            "SELECT id, confidence, relationship_type, direction, context_entity_id
                             FROM person_relationships
                             WHERE source = 'ai_enrichment'
                               AND context_entity_type = 'account'
                               AND context_entity_id = ?1",
                        )
                        .map_err(|e| format!("prepare relationship read failed: {e}"))?;
                    let mapped = stmt
                        .query_map(params![account_id.clone()], |row| {
                            Ok((
                                row.get::<_, String>(0)?,
                                row.get::<_, f64>(1)?,
                                row.get::<_, String>(2)?,
                                row.get::<_, String>(3)?,
                                row.get::<_, Option<String>>(4)?,
                            ))
                        })
                        .map_err(|e| format!("query relationship read failed: {e}"))?;
                    let mut rows = Vec::new();
                    let mut ids = HashSet::new();
                    for row in mapped {
                        let row =
                            row.map_err(|e| format!("relationship row decode failed: {e}"))?;
                        ids.insert(row.0.clone());
                        rows.push(row);
                    }

                    let manager_bad: i64 = db
                        .conn_ref()
                        .query_row(
                            "SELECT COUNT(*)
                             FROM person_relationships
                             WHERE source = 'ai_enrichment'
                               AND context_entity_type = 'account'
                               AND context_entity_id = ?1
                               AND relationship_type = 'manager'
                               AND direction != 'directed'",
                            params![account_id.clone()],
                            |row| row.get(0),
                        )
                        .map_err(|e| format!("manager direction check failed: {e}"))?;
                    let peer_bad: i64 = db
                        .conn_ref()
                        .query_row(
                            "SELECT COUNT(*)
                             FROM person_relationships
                             WHERE source = 'ai_enrichment'
                               AND context_entity_type = 'account'
                               AND context_entity_id = ?1
                               AND relationship_type IN ('peer', 'collaborator')
                               AND direction != 'symmetric'",
                            params![account_id.clone()],
                            |row| row.get(0),
                        )
                        .map_err(|e| format!("peer/collaborator direction check failed: {e}"))?;
                    let signals: i64 = db
                        .conn_ref()
                        .query_row(
                            "SELECT COUNT(*)
                             FROM signal_events
                             WHERE entity_type = 'account'
                               AND entity_id = ?1
                               AND signal_type = 'relationship_inferred'
                               AND source = 'ai_enrichment'",
                            params![account_id],
                            |row| row.get(0),
                        )
                        .map_err(|e| {
                            format!("count relationship signals after first failed: {e}")
                        })?;

                    Ok((rows, ids, manager_bad, peer_bad, signals))
                }
            })
            .await
            .expect("read i504 post-first-run state failed");

        assert!(
            !rows_after_first.is_empty(),
            "I504 AC1: account enrichment with >=3 stakeholders should produce ai_enrichment relationship rows"
        );
        for (_, confidence, _, _, context_entity_id) in &rows_after_first {
            assert!(
                (*confidence - 0.6).abs() < 1e-9,
                "I504 AC2: inferred relationship confidence must be 0.6"
            );
            assert_eq!(
                context_entity_id.as_deref(),
                Some(account_id.as_str()),
                "I504 AC2: context_entity_id must be the enriched account"
            );
        }
        assert_eq!(
            manager_bad, 0,
            "I504 AC3: manager relationships must be directed"
        );
        assert_eq!(
            peer_bad, 0,
            "I504 AC3: peer/collaborator relationships must be symmetric"
        );

        let inserted_first_run = (rows_after_first.len() as i64 - before_rows).max(0);
        if inserted_first_run > 0 {
            assert!(
                signals_after_first >= before_signals + inserted_first_run,
                "I504 AC6: relationship_inferred signals should grow with new inserted edges"
            );
        } else {
            assert!(
                signals_after_first > 0,
                "I504 AC6: account should have relationship_inferred signal history for ai_enrichment edges"
            );
        }

        let _ = enrich_entity(account_id.clone(), "account".to_string(), &state, None)
            .await
            .expect("second enrich_entity for i504 validation failed");

        let (rows_after_second, ids_after_second, reinforced_after_second): (i64, i64, i64) = state
            .db_read({
                let account_id = account_id.clone();
                move |db| {
                    let rows: i64 = db
                        .conn_ref()
                        .query_row(
                            "SELECT COUNT(*)
                             FROM person_relationships
                             WHERE source = 'ai_enrichment'
                               AND context_entity_type = 'account'
                               AND context_entity_id = ?1",
                            params![account_id.clone()],
                            |row| row.get(0),
                        )
                        .map_err(|e| format!("count rows after second enrichment failed: {e}"))?;
                    let ids: i64 = db
                        .conn_ref()
                        .query_row(
                            "SELECT COUNT(DISTINCT id)
                             FROM person_relationships
                             WHERE source = 'ai_enrichment'
                               AND context_entity_type = 'account'
                               AND context_entity_id = ?1",
                            params![account_id],
                            |row| row.get(0),
                        )
                        .map_err(|e| {
                            format!("count distinct ids after second enrichment failed: {e}")
                        })?;
                    let reinforced: i64 = db
                        .conn_ref()
                        .query_row(
                            "SELECT COUNT(*)
                             FROM person_relationships
                             WHERE source = 'ai_enrichment'
                               AND context_entity_type = 'account'
                               AND context_entity_id = ?1
                               AND last_reinforced_at IS NOT NULL",
                            params![account_id],
                            |row| row.get(0),
                        )
                        .map_err(|e| {
                            format!("count reinforced edges after second enrichment failed: {e}")
                        })?;
                    Ok((rows, ids, reinforced))
                }
            })
            .await
            .expect("read i504 post-second-run state failed");

        assert_eq!(
            rows_after_second, ids_after_second,
            "I504 AC4: re-enrichment must not create duplicate AI relationship IDs"
        );
        assert!(
            rows_after_second >= rows_after_first.len() as i64
                && ids_after_second as usize >= ids_after_first.len(),
            "I504 AC4: second enrichment should preserve or reinforce existing inferred edges"
        );
        assert!(
            reinforced_after_second > 0,
            "I504 AC4: re-enrichment should reinforce existing edges (last_reinforced_at set)"
        );
    }
}
