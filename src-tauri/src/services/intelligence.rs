// Intelligence service — extracted from commands.rs
// Business logic for entity intelligence CRUD, enrichment, and risk briefings.

use std::path::Path;

use crate::db::ActionDb;
use crate::intel_queue::{
    apply_enrichment_side_writes, compose_enrichment_intelligence,
    fenced_write_enrichment_intelligence, gather_enrichment_input,
    record_enrichment_contamination_rejection, run_enrichment,
    run_enrichment_post_commit_side_effects, EnrichmentComposition, IntelPriority, IntelRequest,
};
use crate::pty::AiUsageContext;
use crate::services::context::ServiceContext;
use crate::signals::propagation::PropagationEngine;
use crate::state::AppState;
use tauri::Emitter;

/// Preserve user-confirmed value_delivered items during re-enrichment.
///
/// Items with `item_source.source == "user_correction"` are user-confirmed and must
/// survive re-enrichment. New AI items are merged in, deduplicating by fuzzy statement match.
fn merge_user_confirmed_values(
    new_intel: &mut crate::intelligence::IntelligenceJson,
    existing: &crate::intelligence::IntelligenceJson,
) {
    // Collect user-confirmed items from existing data
    let user_confirmed: Vec<_> = existing
        .value_delivered
        .iter()
        .filter(|v| {
            v.item_source
                .as_ref()
                .is_some_and(|s| s.source == "user_correction")
        })
        .cloned()
        .collect();

    if user_confirmed.is_empty() {
        return;
    }

    // Build set of existing user-confirmed statements (lowercased, trimmed) for dedup
    let confirmed_statements: std::collections::HashSet<String> = user_confirmed
        .iter()
        .map(|v| v.statement.trim().to_lowercase())
        .collect();

    // Remove AI items that duplicate user-confirmed items
    new_intel
        .value_delivered
        .retain(|v| !confirmed_statements.contains(&v.statement.trim().to_lowercase()));

    // Prepend user-confirmed items (they take priority)
    let mut merged = user_confirmed;
    merged.append(&mut new_intel.value_delivered);
    merged.truncate(10); // Cap at 10
    new_intel.value_delivered = merged;
}

fn subject_ref_for_entity(entity_type: &str, entity_id: &str) -> Result<String, String> {
    match entity_type {
        "account" | "project" | "person" | "meeting" => Ok(serde_json::json!({
            "kind": entity_type,
            "id": entity_id,
        })
        .to_string()),
        other => Err(format!("Unsupported claim projection subject: {other}")),
    }
}

fn projection_metadata(value: serde_json::Value) -> Option<String> {
    serde_json::to_string(&serde_json::json!({
        "legacy_projection_value": value,
    }))
    .ok()
}

fn non_empty_join(parts: impl IntoIterator<Item = String>) -> Option<String> {
    let joined = parts
        .into_iter()
        .filter(|part| !part.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    if joined.trim().is_empty() {
        None
    } else {
        Some(joined)
    }
}

fn current_state_projection_text(state: &crate::intelligence::io::CurrentState) -> Option<String> {
    non_empty_join(
        [
            (!state.working.is_empty()).then(|| format!("Working: {}", state.working.join("; "))),
            (!state.not_working.is_empty())
                .then(|| format!("Not working: {}", state.not_working.join("; "))),
            (!state.unknowns.is_empty())
                .then(|| format!("Unknowns: {}", state.unknowns.join("; "))),
        ]
        .into_iter()
        .flatten(),
    )
}

fn company_context_projection_text(
    context: &crate::intelligence::io::CompanyContext,
) -> Option<String> {
    non_empty_join(
        [
            context.description.clone(),
            context.industry.as_ref().map(|v| format!("Industry: {v}")),
            context.size.as_ref().map(|v| format!("Size: {v}")),
            context
                .headquarters
                .as_ref()
                .map(|v| format!("Headquarters: {v}")),
            context.additional_context.clone(),
        ]
        .into_iter()
        .flatten(),
    )
}

fn stakeholder_engagement_projection_text(
    insight: &crate::intelligence::io::StakeholderInsight,
) -> Option<String> {
    non_empty_join(
        [
            insight.engagement.clone(),
            insight.assessment.clone(),
            insight.role.clone().map(|role| format!("Role: {role}")),
            (!insight.name.trim().is_empty()).then(|| format!("Stakeholder: {}", insight.name)),
        ]
        .into_iter()
        .flatten(),
    )
}

struct ProjectionClaimInput<'a> {
    subject_ref: &'a str,
    actor: &'a str,
    data_source: &'a str,
    source_asof: Option<&'a str>,
    claim_type: &'a str,
    field_path: &'a str,
    text: &'a str,
    legacy_value: serde_json::Value,
}

fn commit_projection_claim(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    input: ProjectionClaimInput<'_>,
) -> Result<(), String> {
    if input.text.trim().is_empty() {
        return Ok(());
    }
    crate::services::claims::commit_claim(
        ctx,
        db,
        crate::services::claims::ClaimProposal {
            subject_ref: input.subject_ref.to_string(),
            claim_type: input.claim_type.to_string(),
            field_path: Some(input.field_path.to_string()),
            topic_key: None,
            text: input.text.to_string(),
            actor: input.actor.to_string(),
            data_source: input.data_source.to_string(),
            source_ref: None,
            source_asof: input.source_asof.map(str::to_string),
            observed_at: ctx.clock.now().to_rfc3339(),
            provenance_json: "{}".to_string(),
            metadata_json: projection_metadata(input.legacy_value),
            thread_id: None,
            temporal_scope: None,
            sensitivity: None,
            tombstone: None,
        },
    )
    .map(|_| ())
    .map_err(|e| format!("commit {} projection claim failed: {e}", input.claim_type))
}

pub(crate) fn commit_claim_shaped_intelligence_projection(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    intel: &crate::intelligence::IntelligenceJson,
    actor: &str,
    data_source: &str,
) -> Result<(), String> {
    let subject_ref = subject_ref_for_entity(&intel.entity_type, &intel.entity_id)?;
    let source_asof = (!intel.enriched_at.trim().is_empty()).then_some(intel.enriched_at.as_str());

    if let Some(summary) = intel.executive_assessment.as_deref() {
        commit_projection_claim(
            ctx,
            db,
            ProjectionClaimInput {
                subject_ref: &subject_ref,
                actor,
                data_source,
                source_asof,
                claim_type: "entity_summary",
                field_path: "executiveAssessment",
                text: summary,
                legacy_value: serde_json::Value::String(summary.to_string()),
            },
        )?;
    }

    for (idx, risk) in intel.risks.iter().enumerate() {
        commit_projection_claim(
            ctx,
            db,
            ProjectionClaimInput {
                subject_ref: &subject_ref,
                actor,
                data_source,
                source_asof,
                claim_type: "entity_risk",
                field_path: &format!("risks[{idx}]"),
                text: &risk.text,
                legacy_value: serde_json::to_value(risk)
                    .unwrap_or_else(|_| serde_json::json!({ "text": &risk.text })),
            },
        )?;
    }

    for (idx, win) in intel.recent_wins.iter().enumerate() {
        commit_projection_claim(
            ctx,
            db,
            ProjectionClaimInput {
                subject_ref: &subject_ref,
                actor,
                data_source,
                source_asof,
                claim_type: "entity_win",
                field_path: &format!("recentWins[{idx}]"),
                text: &win.text,
                legacy_value: serde_json::to_value(win)
                    .unwrap_or_else(|_| serde_json::json!({ "text": &win.text })),
            },
        )?;
    }

    if let Some(state) = intel.current_state.as_ref() {
        if let Some(text) = current_state_projection_text(state) {
            commit_projection_claim(
                ctx,
                db,
                ProjectionClaimInput {
                    subject_ref: &subject_ref,
                    actor,
                    data_source,
                    source_asof,
                    claim_type: "entity_current_state",
                    field_path: "currentState",
                    text: &text,
                    legacy_value: serde_json::to_value(state)
                        .unwrap_or_else(|_| serde_json::json!({ "text": text.clone() })),
                },
            )?;
        }
    }

    for (idx, value) in intel.value_delivered.iter().enumerate() {
        commit_projection_claim(
            ctx,
            db,
            ProjectionClaimInput {
                subject_ref: &subject_ref,
                actor,
                data_source,
                source_asof,
                claim_type: "value_delivered",
                field_path: &format!("valueDelivered[{idx}]"),
                text: &value.statement,
                legacy_value: serde_json::to_value(value)
                    .unwrap_or_else(|_| serde_json::json!({ "statement": &value.statement })),
            },
        )?;
    }

    for (idx, insight) in intel.stakeholder_insights.iter().enumerate() {
        let Some(person_id) = insight.person_id.as_deref() else {
            continue;
        };
        let Some(text) = stakeholder_engagement_projection_text(insight) else {
            continue;
        };
        let person_subject_ref = subject_ref_for_entity("person", person_id)?;
        commit_projection_claim(
            ctx,
            db,
            ProjectionClaimInput {
                subject_ref: &person_subject_ref,
                actor,
                data_source,
                source_asof,
                claim_type: "stakeholder_engagement",
                field_path: &format!("stakeholderInsights[{idx}].engagement"),
                text: &text,
                legacy_value: serde_json::to_value(insight)
                    .unwrap_or_else(|_| serde_json::json!({ "engagement": text.clone() })),
            },
        )?;
    }

    if intel.entity_type == "account" {
        if let Some(context) = intel.company_context.as_ref() {
            if let Some(text) = company_context_projection_text(context) {
                commit_projection_claim(
                    ctx,
                    db,
                    ProjectionClaimInput {
                        subject_ref: &subject_ref,
                        actor,
                        data_source,
                        source_asof,
                        claim_type: "company_context",
                        field_path: "companyContext",
                        text: &text,
                        legacy_value: serde_json::to_value(context)
                            .unwrap_or_else(|_| serde_json::json!({ "description": text.clone() })),
                    },
                )?;
            }
        }
    }

    Ok(())
}

fn stage_failure_message(stage: &str) -> &str {
    match stage {
        "context_gather" => "context gather",
        "pty_permit" => "PTY permit acquisition",
        "pty_enrichment" => "Claude PTY enrichment",
        "write_results" => "result writeback",
        "relationship_persist" => "relationship persistence",
        _ => stage,
    }
}

fn emit_manual_refresh_failed(
    ctx: &ServiceContext<'_>,
    app_handle: Option<&tauri::AppHandle>,
    entity_id: &str,
    entity_type: &str,
    entity_label: &str,
    stage: &str,
    error: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    if let Some(app) = app_handle {
        let payload = serde_json::json!({
            "phase": "failed",
            "message": format!(
                "Insight refresh failed for {} during {}",
                entity_label,
                stage_failure_message(stage)
            ),
            "count": 1,
            "manual": true,
            "entityId": entity_id,
            "entityType": entity_type,
            "stage": stage,
            "error": error,
        });
        if let Err(e) = app.emit("background-work-status", payload.clone()) {
            log::warn!("emit manual refresh background failure status failed: {e}");
        }
        if let Err(e) = app.emit("intelligence-refresh-failed", payload) {
            log::warn!("emit manual refresh failure event failed: {e}");
        }
    }
    Ok(())
}

fn emit_manual_refresh_failed_best_effort(
    ctx: &ServiceContext<'_>,
    app_handle: Option<&tauri::AppHandle>,
    entity_id: &str,
    entity_type: &str,
    entity_label: &str,
    stage: &str,
    error: &str,
) {
    if let Err(e) = emit_manual_refresh_failed(
        ctx,
        app_handle,
        entity_id,
        entity_type,
        entity_label,
        stage,
        error,
    ) {
        log::warn!("emit manual refresh failure notification failed: {e}");
    }
}

fn manual_refresh_error(stage: &str, error: &str) -> String {
    format!(
        "manual refresh failed during {}: {}",
        stage_failure_message(stage),
        error
    )
}

/// Enrich an entity via the intelligence queue (split-lock pattern).
pub async fn enrich_entity(
    ctx: &ServiceContext<'_>,
    entity_id: String,
    entity_type: String,
    state: &std::sync::Arc<AppState>,
    app_handle: Option<&tauri::AppHandle>,
) -> Result<crate::intelligence::IntelligenceJson, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;

    log::warn!(
        "[I535] enrich_entity ENTERED: entity_id={}, type={}, provider={}",
        entity_id,
        entity_type,
        state.context_provider().provider_name(),
    );

    let request = IntelRequest::new(entity_id, entity_type, IntelPriority::Manual);
    let manual_entity_id = request.entity_id.clone();

    if let Some(app) = app_handle {
        if let Err(e) = app.emit(
            "background-work-status",
            serde_json::json!({
                "phase": "started",
                "message": format!("Updating insights for {}…", manual_entity_id),
                "count": 1,
                "manual": true,
            }),
        ) {
            log::warn!("emit manual refresh start status failed for {manual_entity_id}: {e}");
        }
    }

    // Manual refresh: clear circuit breaker so enrichment proceeds
    let entity_id_for_reset = request.entity_id.clone();
    if let Err(e) = state
        .db_write(move |db| {
            crate::self_healing::scheduler::reset_circuit_breaker(db, &entity_id_for_reset);
            Ok(())
        })
        .await
    {
        log::warn!("reset circuit breaker before manual refresh failed: {e}");
    }

    let input = match gather_enrichment_input(state, &request) {
        Ok(input) => input,
        Err(e) => {
            log::warn!(
                "[I535] gather_enrichment_input FAILED for {}: {}",
                request.entity_id,
                e
            );
            emit_manual_refresh_failed_best_effort(
                ctx,
                app_handle,
                &request.entity_id,
                &request.entity_type,
                &request.entity_id,
                "context_gather",
                &e,
            );
            return Err(manual_refresh_error("context_gather", &e));
        }
    };

    let ai_config = state
        .config
        .read()
        .as_ref()
        .map(|c| c.ai_models.clone())
        .unwrap_or_default();

    // /ADR-0100: Glean-first enrichment for manual refresh.
    // Try Glean chat if connected, fall back to PTY on failure.
    // Timeout on user-facing permit acquisition — return a friendly message
    // instead of blocking indefinitely when background enrichment is running.
    let _permit = match tokio::time::timeout(
        std::time::Duration::from_secs(10),
        state.permits.user_initiated.acquire(),
    )
    .await
    {
        Ok(Ok(permit)) => permit,
        Ok(Err(_)) => {
            let error = "PTY permit closed";
            emit_manual_refresh_failed_best_effort(
                ctx,
                app_handle,
                &input.entity_id,
                &input.entity_type,
                &input.entity_name,
                "pty_permit",
                error,
            );
            return Err(manual_refresh_error("pty_permit", error));
        }
        Err(_) => {
            let error = "Background work in progress — your refresh is queued and will run shortly";
            emit_manual_refresh_failed_best_effort(
                ctx,
                app_handle,
                &input.entity_id,
                &input.entity_type,
                &input.entity_name,
                "pty_permit",
                error,
            );
            return Err(manual_refresh_error("pty_permit", error));
        }
    };

    //  single coherent snapshot of context state —
    // is_remote + Glean Arc captured under one read-lock acquisition.
    // Avoids the L2 codex race where a Local switch between separate
    // getters could leave callers in a mixed-state world.
    let snap = state.context_snapshot();
    let is_remote = snap.is_remote();
    let glean_endpoint = snap.remote_endpoint();
    log::warn!(
        "[I535] enrich_entity: provider={}, is_remote={}, endpoint={:?}, has_ctx={}, entity={} ({})",
        snap.provider_name(),
        is_remote,
        glean_endpoint.is_some(),
        input.intelligence_context.is_some(),
        input.entity_name,
        input.entity_type,
    );
    let parsed = if is_remote {
        // Try Glean-first path
        let mut glean_result = None;
        if let (Some(_endpoint), Some(ref ctx)) = (&glean_endpoint, &input.intelligence_context) {
            //  route through the snapshot's Glean Arc
            // per ADR-0091. Falls through to PTY when the snapshot shows
            // None (bridge cleared by atomic Local swap). The snapshot
            // captured above is immutable here, so a concurrent settings
            // change cannot perturb the routing decision mid-call.
            let provider = match snap.glean_intelligence_provider.clone() {
                Some(p) => Some(p),
                None => {
                    log::warn!(
                        "[I535] Context-mode snapshot for manual refresh on {} \
                         shows is_remote=true but no Glean Arc; settings raced this \
                         call. Falling through to PTY per ADR-0091.",
                        input.entity_name
                    );
                    None
                }
            };
            // This path is the services::intelligence manual-refresh entry,
            // always user-initiated — pass is_background=false so the UI
            // gets degraded/fallback toasts.
            if let Some(provider) = provider {
                match provider
                    .enrich_entity(
                        &input.entity_id,
                        &input.entity_type,
                        &input.entity_name,
                        ctx,
                        input.relationship.as_deref(),
                        app_handle,
                        false,
                        input.active_preset.as_ref(),
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
                        // Surface the fallback loudly — otherwise users see
                        // local-sourced items on a Glean-mode account with no
                        // signal that Glean enrichment couldn't complete.
                        {
                            let mut audit = state.audit_log.lock();
                            if let Err(audit_error) = audit.append(
                                "data_access",
                                "glean_enrichment_fellback_to_pty",
                                serde_json::json!({
                                    "entity_id": input.entity_id,
                                    "entity_type": input.entity_type,
                                    "entity_name": input.entity_name,
                                    "reason": e.to_string(),
                                }),
                            ) {
                                log::warn!(
                                    "append Glean fallback audit entry failed: {audit_error}"
                                );
                            }
                        }
                        if let Some(handle) = app_handle {
                            if let Err(emit_error) = handle.emit(
                                "enrichment-glean-fallback",
                                serde_json::json!({
                                    "entity_id": input.entity_id,
                                    "entity_type": input.entity_type,
                                    "entity_name": input.entity_name,
                                    "reason": e.to_string(),
                                }),
                            ) {
                                log::warn!("emit Glean fallback event failed: {emit_error}");
                            }
                        }
                    }
                }
            } // end if let Some(provider) — bridge-empty case skipped Glean and
              // falls through to the PTY path below via glean_result == None.
        }

        match glean_result {
            Some(parsed) => parsed,
            None => {
                // Fallback to PTY
                let input_for_enrichment = input.clone();
                let ai_config_for_enrichment = ai_config.clone();
                let app_handle_clone = app_handle.cloned();
                let pty_result = tauri::async_runtime::spawn_blocking(move || {
                    let usage_context =
                        AiUsageContext::new("intelligence", "manual_entity_enrichment")
                            .with_trigger("manual_refresh");
                    run_enrichment(
                        &input_for_enrichment,
                        &ai_config_for_enrichment,
                        app_handle_clone.as_ref(),
                        usage_context,
                    )
                })
                .await;
                match pty_result {
                    Ok(Ok(parsed)) => parsed,
                    Ok(Err(e)) => {
                        emit_manual_refresh_failed_best_effort(
                            ctx,
                            app_handle,
                            &input.entity_id,
                            &input.entity_type,
                            &input.entity_name,
                            "pty_enrichment",
                            &e,
                        );
                        return Err(manual_refresh_error("pty_enrichment", &e));
                    }
                    Err(e) => {
                        let error = format!("Enrichment task panicked: {}", e);
                        emit_manual_refresh_failed_best_effort(
                            ctx,
                            app_handle,
                            &input.entity_id,
                            &input.entity_type,
                            &input.entity_name,
                            "pty_enrichment",
                            &error,
                        );
                        return Err(manual_refresh_error("pty_enrichment", &error));
                    }
                }
            }
        }
    } else {
        // Local-only: direct PTY path
        let input_for_enrichment = input.clone();
        let ai_config_for_enrichment = ai_config.clone();
        let app_handle_clone = app_handle.cloned();
        let pty_result = tauri::async_runtime::spawn_blocking(move || {
            let usage_context = AiUsageContext::new("intelligence", "manual_entity_enrichment")
                .with_trigger("manual_refresh");
            run_enrichment(
                &input_for_enrichment,
                &ai_config_for_enrichment,
                app_handle_clone.as_ref(),
                usage_context,
            )
        })
        .await;
        match pty_result {
            Ok(Ok(parsed)) => parsed,
            Ok(Err(e)) => {
                emit_manual_refresh_failed_best_effort(
                    ctx,
                    app_handle,
                    &input.entity_id,
                    &input.entity_type,
                    &input.entity_name,
                    "pty_enrichment",
                    &e,
                );
                return Err(manual_refresh_error("pty_enrichment", &e));
            }
            Err(e) => {
                let error = format!("Enrichment task panicked: {}", e);
                emit_manual_refresh_failed_best_effort(
                    ctx,
                    app_handle,
                    &input.entity_id,
                    &input.entity_type,
                    &input.entity_name,
                    "pty_enrichment",
                    &error,
                );
                return Err(manual_refresh_error("pty_enrichment", &error));
            }
        }
    };

    let db = ActionDb::open().map_err(|e| {
        let e = format!("Failed to open DB: {e}");
        emit_manual_refresh_failed_best_effort(
            ctx,
            app_handle,
            &input.entity_id,
            &input.entity_type,
            &input.entity_name,
            "write_results",
            &e,
        );
        manual_refresh_error("write_results", &e)
    })?;
    let composition = match compose_enrichment_intelligence(
        state,
        &db,
        &input,
        &parsed.intel,
        Some(&ai_config),
    ) {
        Ok(composition) => composition,
        Err(e) => {
            emit_manual_refresh_failed_best_effort(
                ctx,
                app_handle,
                &input.entity_id,
                &input.entity_type,
                &input.entity_name,
                "write_results",
                &e,
            );
            return Err(manual_refresh_error("write_results", &e));
        }
    };
    let final_intel = match composition {
        EnrichmentComposition::Persist(prepared) => {
            if let Err(e) = db.with_transaction(|tx| {
                apply_enrichment_side_writes(ctx, tx, &input, &prepared)?;
                upsert_assessment_from_enrichment_in_active_transaction(
                    ctx,
                    tx,
                    &state.signals.engine,
                    &input.entity_type,
                    &input.entity_id,
                    prepared.intelligence(),
                )
            }) {
                emit_manual_refresh_failed_best_effort(
                    ctx,
                    app_handle,
                    &input.entity_id,
                    &input.entity_type,
                    &input.entity_name,
                    "write_results",
                    &e,
                );
                return Err(manual_refresh_error("write_results", &e));
            }
            fenced_write_enrichment_intelligence(&db, &input.entity_dir, prepared.intelligence());
            run_enrichment_post_commit_side_effects(state, &input, &db, prepared.intelligence());
            prepared.into_intelligence()
        }
        EnrichmentComposition::SkipDueToContamination {
            prior, rejection, ..
        } => {
            record_enrichment_contamination_rejection(ctx, state, &input, &db, &rejection);
            prior
        }
    };
    if !parsed.inferred_relationships.is_empty() {
        let engine = state.signals.engine.clone();
        let entity_id_for_persist = input.entity_id.clone();
        let entity_type_for_persist = input.entity_type.clone();
        let inferred = parsed.inferred_relationships.clone();
        let state_for_ctx = state.clone();
        state
            .db_write(move |db| {
                let ctx = state_for_ctx.live_service_context();
                upsert_inferred_relationships_from_enrichment(
                    &ctx,
                    db,
                    engine.as_ref(),
                    &entity_type_for_persist,
                    &entity_id_for_persist,
                    &inferred,
                )
                .map(|_| ())
            })
            .await
            .map_err(|e| {
                emit_manual_refresh_failed_best_effort(
                    ctx,
                    app_handle,
                    &input.entity_id,
                    &input.entity_type,
                    &input.entity_name,
                    "relationship_persist",
                    &e,
                );
                manual_refresh_error("relationship_persist", &e)
            })?;
    }

    if let Some(app) = app_handle {
        if let Err(e) = app.emit(
            "background-work-status",
            serde_json::json!({
                "phase": "completed",
                "message": format!("Insights updated for {}", input.entity_name),
                "count": 1,
                "manual": true,
            }),
        ) {
            log::warn!(
                "emit manual refresh completion status failed for {}: {e}",
                input.entity_id
            );
        }
    }

    Ok(final_intel)
}

pub fn persist_entity_keywords(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    keywords_json: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
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
            ctx,
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
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    engine: &PropagationEngine,
    entity_type: &str,
    entity_id: &str,
    intel: &crate::intelligence::IntelligenceJson,
) -> Result<(), String> {
    db.with_transaction(|tx| {
        upsert_assessment_from_enrichment_in_active_transaction(
            ctx,
            tx,
            engine,
            entity_type,
            entity_id,
            intel,
        )
    })
}

pub(crate) fn upsert_assessment_from_enrichment_in_active_transaction(
    ctx: &ServiceContext<'_>,
    tx: &ActionDb,
    engine: &PropagationEngine,
    entity_type: &str,
    entity_id: &str,
    intel: &crate::intelligence::IntelligenceJson,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    // Merge value_delivered — preserve user-confirmed items during re-enrichment.
    let mut intel = intel.clone();
    if let Ok(Some(existing)) = tx.get_entity_intelligence(entity_id) {
        merge_user_confirmed_values(&mut intel, &existing);
    }
    commit_claim_shaped_intelligence_projection(
        ctx,
        tx,
        &intel,
        "agent:intelligence",
        "ai_enrichment",
    )?;
    crate::services::derived_state::upsert_entity_intelligence_legacy_snapshot(tx, &intel)
        .map_err(|e| e.to_string())?;
    crate::services::signals::emit_and_propagate(
        ctx,
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

    // After enrichment, reconcile AI objectives with user objectives
    if entity_type == "account" {
        if let Err(e) = crate::services::success_plans::reconcile_objectives(ctx, tx, entity_id) {
            log::warn!("Objective reconciliation failed for {entity_id}: {e}");
        }
    }

    // DOS Work-tab: Best-effort bridge of AI-inferred commitments → Actions.
    // Enrichment write is the source of truth; bridge errors must not fail it.
    if entity_type == "account" {
        if let Some(ref commitments) = intel.open_commitments {
            match crate::services::commitment_bridge::sync_ai_commitments(
                ctx,
                tx,
                entity_type,
                entity_id,
                commitments,
            ) {
                Ok(summary) => log::info!(
                    "commitment_bridge: {} created, {} updated, {} tombstoned-skip, {} missing-id ({}:{})",
                    summary.created,
                    summary.updated,
                    summary.skipped_tombstoned,
                    summary.skipped_missing_id,
                    entity_type,
                    entity_id
                ),
                Err(e) => log::warn!(
                    "commitment_bridge sync failed for {entity_type}:{entity_id} (non-fatal): {e}"
                ),
            }
        }
    }

    Ok(())
}

/// Persist an assessment snapshot without emitting enrichment lifecycle signals.
///
/// Progressive-write paths use this helper so the final authoritative write can
/// remain the single point for signal emission and downstream invalidation.
pub fn upsert_assessment_snapshot(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    intel: &crate::intelligence::IntelligenceJson,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    db.with_transaction(|tx| {
        commit_claim_shaped_intelligence_projection(
            ctx,
            tx,
            intel,
            "agent:intel_queue",
            "ai_enrichment_progressive",
        )?;
        crate::services::derived_state::upsert_entity_intelligence_legacy_snapshot(tx, intel)
            .map_err(|e| e.to_string())?;

        // Path 2c: Store domains from Glean enrichment (if present).
        // When Glean enrichment populates intel.domains (extracted from stakeholder emails),
        // persist them to account_domains for entity resolution.
        // Only applies to account entities.
        if intel.entity_type == "account" && !intel.domains.is_empty() {
            tx.merge_account_domains_enrichment(&intel.entity_id, &intel.domains)
                .map_err(|e| {
                    format!(
                        "Failed to store domains for account {}: {}",
                        intel.entity_id, e
                    )
                })?;
            log::debug!(
                "Intelligence service: stored {} domains for account '{}'",
                intel.domains.len(),
                intel.entity_id
            );
        }

        Ok(())
    })?;

    Ok(())
}

/// Persist the Glean leading-signals JSON blob on `entity_assessment`
/// and emit the four callout-worthy signals derived from it.
///
/// Wrapped in a transaction so the blob write and signal emissions either all
/// land or all roll back. Source is tagged `glean_leading_signals` at confidence
/// 0.8 (champion_at_risk), 0.75 (competitor_decision_relevant), 0.7
/// (sentiment_divergence), 0.75 (budget_cycle_locked) — matching the tier policy
/// of other Glean-derived signals registered in `signals/callouts.rs`.
pub fn upsert_health_outlook_signals(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    engine: &PropagationEngine,
    entity_type: &str,
    entity_id: &str,
    signals: &crate::intelligence::glean_leading_signals::HealthOutlookSignals,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let blob = serde_json::to_string(signals)
        .map_err(|e| format!("Failed to serialize health_outlook_signals: {e}"))?;

    db.with_transaction(|tx| {
        crate::services::derived_state::upsert_health_outlook_signals_legacy_projection(
            tx,
            entity_id,
            entity_type,
            &blob,
        )
        .map_err(|e| format!("Failed to upsert health_outlook_signals_json: {e}"))?;

        let derived = signals.derive_signals();

        if let Some(payload) = derived.champion_at_risk {
            crate::services::signals::emit_and_propagate(
                ctx,
                tx,
                engine,
                entity_type,
                entity_id,
                "champion_at_risk",
                "glean_leading_signals",
                Some(&payload),
                0.8,
            )
            .map_err(|e| format!("champion_at_risk emit failed: {e}"))?;
        }

        if let Some(payload) = derived.sentiment_divergence {
            crate::services::signals::emit_and_propagate(
                ctx,
                tx,
                engine,
                entity_type,
                entity_id,
                "sentiment_divergence",
                "glean_leading_signals",
                Some(&payload),
                0.7,
            )
            .map_err(|e| format!("sentiment_divergence emit failed: {e}"))?;
        }

        for payload in derived.competitor_decision_relevant {
            crate::services::signals::emit_and_propagate(
                ctx,
                tx,
                engine,
                entity_type,
                entity_id,
                "competitor_decision_relevant",
                "glean_leading_signals",
                Some(&payload),
                0.75,
            )
            .map_err(|e| format!("competitor_decision_relevant emit failed: {e}"))?;
        }

        if let Some(payload) = derived.budget_cycle_locked {
            crate::services::signals::emit_and_propagate(
                ctx,
                tx,
                engine,
                entity_type,
                entity_id,
                "budget_cycle_locked",
                "glean_leading_signals",
                Some(&payload),
                0.75,
            )
            .map_err(|e| format!("budget_cycle_locked emit failed: {e}"))?;
        }

        Ok(())
    })
}

/// Persist AI-inferred person relationships for an enrichment run.
///
/// - Skips invalid/self edges.
/// - Never overwrites strong user-confirmed edges.
/// - Uses deterministic IDs so re-enrichment reinforces instead of duplicating.
/// - Emits `relationship_inferred` only when creating a new AI edge.
pub fn upsert_inferred_relationships_from_enrichment(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    engine: &PropagationEngine,
    entity_type: &str,
    entity_id: &str,
    inferred: &[crate::intelligence::prompts::InferredRelationship],
) -> Result<usize, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
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
                    ctx,
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
    ctx: &ServiceContext<'_>,
    entity_id: &str,
    entity_type: &str,
    field_path: &str,
    value: &str,
    state: &AppState,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let config = state.config.read().clone();
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

            // DB is sole source of truth — no filesystem fallback.
            // propagate DB read errors instead of collapsing them into "no row".
            let existing_intel = db
                .get_entity_intelligence(&entity_id)
                .map_err(|e| format!("DB read failed for entity {entity_id}: {e}"))?;
            let intel = match existing_intel {
                Some(existing) => crate::intelligence::apply_intelligence_field_update_in_memory(
                    existing,
                    &field_path,
                    &value,
                )?,
                None => {
                    return Err(format!(
                        "I644: no DB intelligence row for {} — cannot update field",
                        entity_id
                    ))
                }
            };

            // Distinguish curation (delete/clear) from correction (edit).
            // Empty value = user removed the item → curation, no source penalty.
            // Non-empty value = user corrected the item → correction, source penalized.
            let is_curation = value.trim().is_empty() || value == "[]" || value == "null";

            // DB-first ordering. Commit canonical state first; the
            // legacy file cache is written AFTER commit as best-effort.
            db.with_transaction(|tx| {
                tx.upsert_entity_intelligence(&intel)
                    .map_err(|e| e.to_string())?;
                let clock = crate::services::context::SystemClock;
                let rng = crate::services::context::SystemRng;
                let ext = crate::services::context::ExternalClients::default();
                let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &ext);
                let (signal_type, source, confidence) = if is_curation {
                    ("intelligence_curated", "user_curation", 0.5)
                } else {
                    ("user_correction", "user_edit", 1.0)
                };
                crate::services::signals::emit(
                    &ctx,
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

            // Post-commit file write — best-effort cache. DB is canonical from here.
            // routed through the schema-epoch fence so a concurrent
            // migration can preempt stale cache writes.
            crate::intelligence::write_fence::post_commit_fenced_write(
                db,
                &dir,
                &intel,
                &format!("entity={entity_id} field={field_path}"),
            );

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
    ctx: &ServiceContext<'_>,
    entity_id: &str,
    entity_type: &str,
    stakeholders: Vec<crate::intelligence::StakeholderInsight>,
    state: &AppState,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let config = state.config.read().clone();
    let config = config.ok_or("No configuration loaded")?;
    let workspace_path = config.workspace_path.clone();
    let active_preset = state.active_preset.read().clone();

    let engine = state.signals.engine.clone();
    let entity_id = entity_id.to_string();
    let entity_type = entity_type.to_string();
    let sourced_at = ctx.clock.now().to_rfc3339();
    let stakeholders = stakeholders
        .into_iter()
        .map(|mut stakeholder| {
            stakeholder.source = Some("user".to_string());
            stakeholder.item_source = Some(crate::intelligence::ItemSource {
                source: "user_correction".to_string(),
                confidence: 1.0,
                sourced_at: sourced_at.clone(),
                reference: Some("user stakeholder edit".to_string()),
            });
            stakeholder
        })
        .collect::<Vec<_>>();

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
            // the vec is consumed by the in-memory intelligence update.
            let scoring_roles: Vec<(String, String)> = if entity_type == "account" {
                stakeholders
                    .iter()
                    .filter_map(|s| {
                        let role = s.role.as_deref().unwrap_or("").to_lowercase();
                        let engagement = s.engagement.as_deref().unwrap_or("").to_lowercase();
                        let person_id = s.person_id.as_deref()?;
                        // Check BOTH role and engagement — user may set champion
                        // via either the Team panel (role) or EngagementSelector (engagement)
                        let effective = if !engagement.is_empty() {
                            &engagement
                        } else {
                            &role
                        };
                        if effective.contains("champion")
                            || effective.contains("executive")
                            || effective.contains("technical")
                            || effective.contains("decision")
                        {
                            Some((person_id.to_string(), effective.to_string()))
                        } else {
                            None
                        }
                    })
                    .collect()
            } else {
                Vec::new()
            };

            // DB-first: prefer intelligence from DB over disk
            // propagate DB read errors instead of collapsing them.
            let existing_intel = db
                .get_entity_intelligence(&entity_id)
                .map_err(|e| format!("DB read failed for entity {entity_id}: {e}"))?;
            // Compose IntelligenceJson in memory only. Disk write is deferred
            // to post_commit_fenced_write below so disk and DB stay consistent
            // under transaction rollback. Pre-cycle-10 the disk-fallback branch
            // wrote the file BEFORE the transaction, which could leave disk ahead
            // of DB if the new error-propagating subscriber rolled back.
            let intel = if let Some(existing) = existing_intel {
                crate::intelligence::apply_stakeholders_update_in_memory(existing, stakeholders)?
            } else {
                let disk_intel = crate::intelligence::io::read_intelligence_json(&dir)?;
                crate::intelligence::apply_stakeholders_update_in_memory(disk_intel, stakeholders)?
            };

            // DB-first ordering. The legacy file cache is written AFTER
            // the transaction commits.
            db.with_transaction(|tx| {
                tx.upsert_entity_intelligence(&intel)
                    .map_err(|e| e.to_string())?;

                let clock = crate::services::context::SystemClock;
                let rng = crate::services::context::SystemRng;
                let ext = crate::services::context::ExternalClients::default();
                let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &ext);

                // Sync scoring-relevant stakeholder roles to account_stakeholders
                // so health scoring (champion health, stakeholder coverage) picks them up.
                // Errors propagate to roll back the entire enrichment write — a failed
                // stakeholder cache rebuild signal must not leave account_stakeholders
                // partially updated. The B2 contract requires atomicity between the
                // membership write and the cache invalidation.
                for (person_id, role) in &scoring_roles {
                    crate::services::accounts::add_team_member_with_cache_rebuild(
                        &ctx, tx, &entity_id, person_id, role,
                    )?;
                }

                // Recompute health immediately so stakeholder changes reflect
                // in key_advocate_health + stakeholder_coverage dimensions without
                // waiting for a full enrichment cycle.
                if entity_type == "account" && !scoring_roles.is_empty() {
                    if let Some(acct) = account.as_ref() {
                        let health =
                            crate::intelligence::health_scoring::compute_account_health_with_preset(
                                tx,
                                acct,
                                intel.org_health.as_ref(),
                                active_preset.as_ref(),
                            );
                        crate::services::derived_state::upsert_entity_health_legacy_projection(
                            tx, &entity_id, "account", &health,
                        )
                        .ok();
                    }
                }

                crate::services::signals::emit_and_propagate(
                    &ctx,
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

            // Post-commit file write — best-effort cache. DB is canonical from here.
            // routed through the schema-epoch fence.
            crate::intelligence::write_fence::post_commit_fenced_write(
                db,
                &dir,
                &intel,
                &format!("entity={entity_id}"),
            );

            Ok(())
        })
        .await
}

/// Dismiss an intelligence item, creating a tombstone to prevent re-creation.
///
/// Removes the item from the specified Vec field and adds a `DismissedItem`
/// tombstone that prevents future enrichment from re-creating it.
pub async fn dismiss_intelligence_item(
    ctx: &ServiceContext<'_>,
    entity_id: &str,
    entity_type: &str,
    field: &str,
    item_text: &str,
    state: &AppState,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let config = state.config.read().clone();
    let config = config.ok_or("No configuration loaded")?;
    let workspace_path = config.workspace_path.clone();

    let engine = state.signals.engine.clone();
    let entity_id = entity_id.to_string();
    let entity_type = entity_type.to_string();
    let field = field.to_string();
    let item_text = item_text.to_string();
    let dismissed_at = ctx.clock.now().to_rfc3339();
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

            // DB is sole source of truth — no filesystem fallback.
            // propagate DB read errors instead of collapsing them into "no row";
            // the previous `.ok.flatten` masked connection failures behind the "no row" message.
            let existing_intel = db
                .get_entity_intelligence(&entity_id)
                .map_err(|e| format!("DB read failed for entity {entity_id}: {e}"))?;
            let mut intel = existing_intel.ok_or_else(|| {
                format!(
                    "I644: no DB intelligence row for {} — cannot dismiss item",
                    entity_id
                )
            })?;

            // Add tombstone
            intel
                .dismissed_items
                .push(crate::intelligence::DismissedItem {
                    field: field.clone(),
                    content: item_text.clone(),
                    dismissed_at: dismissed_at.clone(),
                });

            // Remove item from the relevant Vec by matching text
            let item_lower = item_text.to_lowercase();
            match field.as_str() {
                "risks" => intel
                    .risks
                    .retain(|r| !r.text.to_lowercase().contains(&item_lower)),
                "recentWins" => intel
                    .recent_wins
                    .retain(|w| !w.text.to_lowercase().contains(&item_lower)),
                "stakeholderInsights" => intel
                    .stakeholder_insights
                    .retain(|s| !s.name.to_lowercase().contains(&item_lower)),
                "valueDelivered" => intel
                    .value_delivered
                    .retain(|v| !v.statement.to_lowercase().contains(&item_lower)),
                "competitiveContext" => intel
                    .competitive_context
                    .retain(|c| !c.competitor.to_lowercase().contains(&item_lower)),
                "organizationalChanges" => intel
                    .organizational_changes
                    .retain(|o| !o.person.to_lowercase().contains(&item_lower)),
                "expansionSignals" => intel
                    .expansion_signals
                    .retain(|e| !e.opportunity.to_lowercase().contains(&item_lower)),
                "openCommitments" => {
                    if let Some(ref mut ocs) = intel.open_commitments {
                        ocs.retain(|c| !c.description.to_lowercase().contains(&item_lower));
                    }
                }
                _ => return Err(format!("Cannot dismiss items from field: {}", field)),
            }

            // DB-first ordering. The transaction commits the canonical
            // state; the legacy `intelligence.json` cache is written AFTER commit
            // and treated as best-effort. A file write failure does not roll back
            // DB state — the projection writer will repair file drift on
            // the next claim touch.
            db.with_transaction(|tx| {
                tx.upsert_entity_intelligence(&intel)
                    .map_err(|e| e.to_string())?;

                // Record feedback event + suppression tombstone.
                // propagate errors so a failed insert no longer leaves
                // a ghost-resurrectable item.
                tx.record_feedback_event(&crate::db::feedback::FeedbackEventInput {
                    entity_id: &entity_id,
                    entity_type: &entity_type,
                    field_key: &field,
                    item_key: Some(&item_text),
                    feedback_type: "dismiss",
                    source_system: None,
                    source_kind: Some("intelligence"),
                    previous_value: Some(&item_text),
                    corrected_value: None,
                    reason: None,
                })
                .map_err(|e| format!("record_feedback_event: {e}"))?;
                tx.create_suppression_tombstone(
                    &entity_id,
                    &field,
                    Some(&item_text),
                    crate::intelligence::canonicalization::maybe_item_hash_for_field(
                        &field,
                        Some(&item_text),
                    )
                    .as_deref(),
                    Some("intelligence"),
                    None,
                )
                .map_err(|e| format!("create_suppression_tombstone: {e}"))?;

                // Shadow-write tombstone claim into the new substrate.
                // Failure logged but not propagated; legacy write above remains
                // authoritative until the claim-read gate migration lands.
                let subject_kind = match entity_type.as_str() {
                    "account" => "Account",
                    "person" => "Person",
                    "project" => "Project",
                    "meeting" => "Meeting",
                    _ => "Account",
                };
                let claim_type = match field.as_str() {
                    "risks" => "risk",
                    "recentWins" | "wins" => "win",
                    _ => "intelligence_field_dismissed",
                };
                crate::services::claims::shadow_write_tombstone_claim(
                    tx,
                    crate::services::claims::ShadowTombstoneClaim {
                        subject_kind,
                        subject_id: &entity_id,
                        claim_type,
                        field_path: Some(&field),
                        text: &item_text,
                        actor: "user",
                        source_scope: Some("intelligence"),
                        observed_at: &dismissed_at,
                        expires_at: None,
                    },
                );

                Ok(())
            })?;

            // Post-commit side effects. emit_and_propagate dispatches engine.propagate
            // which can enqueue cross-entity intel work; running it after commit
            // means a downstream propagation failure cannot roll back the user's
            // dismiss intent. DB is the source of truth; emission failures log.
            let clock = crate::services::context::SystemClock;
            let rng = crate::services::context::SystemRng;
            let ext = crate::services::context::ExternalClients::default();
            let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &ext);
            if let Err(e) = crate::services::signals::emit_and_propagate(
                &ctx,
                db,
                &engine,
                &entity_type,
                &entity_id,
                "intelligence_curated",
                "user_curation",
                Some(&format!(
                    "{{\"field\":\"{field}\",\"dismissed\":\"{item_text}\"}}",
                )),
                0.5,
            ) {
                log::warn!(
                    "post-commit signal emission failed; \
                     repair_target=signals_engine \
                     entity={entity_id} field={field}: {e}"
                );
            }

            // Post-commit file write — best-effort cache. DB is canonical from here.
            // routed through the schema-epoch fence.
            crate::intelligence::write_fence::post_commit_fenced_write(
                db,
                &dir,
                &intel,
                &format!("entity={entity_id} field={field}"),
            );

            Ok(())
        })
        .await
}

/// Recompute health dimensions for an account without full re-enrichment.
///
/// Called when signals arrive that affect health (meetings, emails, stakeholder changes).
/// Updates both the DB (entity_assessment.health_json + entity_quality) and the
/// in-memory IntelligenceJson so downstream surfaces see fresh scores.
pub fn recompute_entity_health(
    ctx: &ServiceContext<'_>,
    db: &crate::db::ActionDb,
    entity_id: &str,
    entity_type: &str,
) -> Result<(), String> {
    recompute_entity_health_with_preset(ctx, db, entity_id, entity_type, None)
}

/// Recompute health dimensions with active preset weights when available.
pub fn recompute_entity_health_with_preset(
    ctx: &ServiceContext<'_>,
    db: &crate::db::ActionDb,
    entity_id: &str,
    entity_type: &str,
    preset: Option<&crate::presets::schema::RolePreset>,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    if entity_type != "account" {
        return Ok(()); // Health scoring is account-only for now
    }

    // Need the DbAccount for lifecycle weights and contract proximity
    let account = match db.get_account(entity_id).map_err(|e| e.to_string())? {
        Some(a) => a,
        None => return Ok(()), // Account not found, nothing to recompute
    };

    // Get existing intelligence
    let intel = match db.get_entity_intelligence(entity_id).ok().flatten() {
        Some(i) => i,
        None => return Ok(()), // No intelligence yet, nothing to recompute
    };

    // Pass org_health from existing intelligence so the 40/60 baseline
    // blend fires consistently (previously passed None, diverging from enrichment scores)
    let org_health_ref = intel.org_health.as_ref();
    let health = crate::intelligence::health_scoring::compute_account_health_with_preset(
        db,
        &account,
        org_health_ref,
        preset,
    );

    // Health is a computed snapshot, not a clean text claim.
    crate::services::derived_state::upsert_entity_health_legacy_projection(
        db,
        entity_id,
        entity_type,
        &health,
    )
    .map_err(|e| e.to_string())?;

    // Note: disk write (for MCP sidecar) is skipped here because we don't have
    // workspace access. The DB is the primary source; disk catches up on next
    // full enrichment cycle.

    log::info!(
        "Health recomputed for {} after signal arrival (score={:.1}, band={})",
        entity_id,
        health.score,
        health.band,
    );

    Ok(())
}

/// Bulk recompute health scores for all accounts.
/// Called once after deploying formula fixes to ensure consistency.
pub fn bulk_recompute_health(db: &crate::db::ActionDb) -> Result<usize, String> {
    let accounts = db.get_all_accounts().map_err(|e| e.to_string())?;
    let mut recomputed = 0;
    let clock = crate::services::context::SystemClock;
    let rng = crate::services::context::SystemRng;
    let ext = crate::services::context::ExternalClients::default();
    let ctx = ServiceContext::new_live(&clock, &rng, &ext);

    for account in &accounts {
        if let Err(e) = recompute_entity_health(&ctx, db, &account.id, "account") {
            log::warn!("Health recompute failed for {}: {}", account.id, e);
            continue;
        }
        recomputed += 1;
    }

    log::info!(
        "Bulk health recompute complete: {}/{} accounts rescored",
        recomputed,
        accounts.len()
    );
    Ok(recomputed)
}

/// Generate a risk briefing for an account (async, PTY enrichment).
pub async fn generate_risk_briefing(
    state: &std::sync::Arc<AppState>,
    account_id: &str,
    app_handle: Option<tauri::AppHandle>,
) -> Result<crate::types::RiskBriefing, String> {
    let app_state = state.clone();
    let account_id = account_id.to_string();
    let progress_handle = app_handle.clone();

    let task = tauri::async_runtime::spawn_blocking(move || {
        let input = {
            let db =
                crate::db::ActionDb::open().map_err(|e| format!("Database unavailable: {e}"))?;

            let config_guard = app_state.config.read();
            let config = config_guard
                .as_ref()
                .ok_or_else(|| "Config not initialized".to_string())?;

            let workspace = std::path::Path::new(&config.workspace_path);
            crate::risk_briefing::gather_risk_input(
                workspace,
                &db,
                &account_id,
                config.user_name.clone(),
                config.ai_models.clone(),
                &*app_state.context_provider(),
            )?
        };

        let briefing = crate::risk_briefing::run_risk_enrichment(&input, progress_handle.as_ref())?;

        // Store in reports table for unified tracking
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
    // Try reports table first (DB-backed storage)
    if let Some(briefing) = crate::reports::risk::load_risk_briefing_from_reports(db, account_id) {
        return Ok(briefing);
    }

    // Fall back to disk (legacy path)
    let config_guard = state.config.read();
    let config = config_guard.as_ref().ok_or("Config not initialized")?;

    let account = db
        .get_account(account_id)
        .map_err(|e| format!("DB error: {}", e))?
        .ok_or_else(|| format!("Account not found: {}", account_id))?;

    let workspace = std::path::Path::new(&config.workspace_path);
    let account_dir = crate::accounts::resolve_account_dir(workspace, &account);
    crate::risk_briefing::read_risk_briefing(&account_dir)
}

// =============================================================================
// Recommended Action Track / Dismiss
// =============================================================================

/// Track (accept) a recommended action — creates a real action with
/// source_type "intelligence" and emits a recommendation_accepted signal.
pub async fn track_recommendation(
    ctx: &ServiceContext<'_>,
    entity_id: &str,
    entity_type: &str,
    index: usize,
    state: &AppState,
) -> Result<String, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let engine = state.signals.engine.clone();
    let entity_id = entity_id.to_string();
    let entity_type = entity_type.to_string();
    let now = ctx.clock.now().to_rfc3339();

    state
        .db_write(move |db| {
            // Read current intelligence to find the recommendation
            let intel = db
                .get_entity_intelligence(&entity_id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("No intelligence found for {}", entity_id))?;

            let rec = intel
                .recommended_actions
                .get(index)
                .ok_or_else(|| format!("Recommendation index {} out of bounds", index))?;

            // Create the action
            let id = uuid::Uuid::new_v4().to_string();
            let action = crate::db::DbAction {
                id: id.clone(),
                title: rec.title.clone(),
                priority: rec.priority,
                status: crate::action_status::UNSTARTED.to_string(),
                created_at: now.clone(),
                due_date: rec.suggested_due.clone(),
                completed_at: None,
                account_id: if entity_type == "account" {
                    Some(entity_id.clone())
                } else {
                    None
                },
                project_id: if entity_type == "project" {
                    Some(entity_id.clone())
                } else {
                    None
                },
                source_type: Some("intelligence".to_string()),
                source_id: Some(entity_id.clone()),
                source_label: Some("Based on account intelligence".to_string()),
                action_kind: crate::action_status::KIND_TASK.to_string(),
                context: Some(rec.rationale.clone()),
                waiting_on: None,
                updated_at: now,
                person_id: if entity_type == "person" {
                    Some(entity_id.clone())
                } else {
                    None
                },
                account_name: None,
                next_meeting_title: None,
                next_meeting_start: None,
                needs_decision: false,
                decision_owner: None,
                decision_stakes: None,
                linear_identifier: None,
                linear_url: None,
            };

            db.upsert_action(&action).map_err(|e| e.to_string())?;

            // Remove the tracked recommendation from intel to prevent duplicates
            let mut updated_intel = intel.clone();
            if index < updated_intel.recommended_actions.len() {
                updated_intel.recommended_actions.remove(index);
                db.upsert_entity_intelligence(&updated_intel)
                    .map_err(|e| e.to_string())?;
            }

            // Emit recommendation_accepted signal
            let clock = crate::services::context::SystemClock;
            let rng = crate::services::context::SystemRng;
            let ext = crate::services::context::ExternalClients::default();
            let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &ext);
            if let Err(e) = crate::services::signals::emit_and_propagate(
                &ctx,
                db,
                &engine,
                &entity_type,
                &entity_id,
                "recommendation_accepted",
                "intelligence",
                Some(&format!(
                    "{{\"action_id\":\"{}\",\"title\":\"{}\"}}",
                    id,
                    rec.title.replace('"', "\\\"")
                )),
                0.8,
            ) {
                log::warn!(
                    "emit recommendation accepted signal failed for {entity_type}:{entity_id}: {e}"
                );
            }

            Ok(id)
        })
        .await
}

/// Dismiss a recommended action — removes it from intelligence and
/// emits a recommendation_rejected signal (low confidence correction).
pub async fn dismiss_recommendation(
    ctx: &ServiceContext<'_>,
    entity_id: &str,
    entity_type: &str,
    index: usize,
    state: &AppState,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let config = state.config.read().clone();
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

            // DB is sole source of truth — no filesystem fallback.
            let mut intel = db
                .get_entity_intelligence(&entity_id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| {
                    format!(
                        "DOS-92: no DB intelligence row for {} — cannot dismiss recommendation",
                        entity_id
                    )
                })?;

            if index >= intel.recommended_actions.len() {
                return Err(format!("Recommendation index {} out of bounds", index));
            }

            let removed = intel.recommended_actions.remove(index);

            // DB-first ordering. Commit DB state; file write + signal
            // emission run after as best-effort post-commit work.
            db.upsert_entity_intelligence(&intel)
                .map_err(|e| e.to_string())?;

            // Post-commit signal emission. Failures log; DB is source of truth.
            let clock = crate::services::context::SystemClock;
            let rng = crate::services::context::SystemRng;
            let ext = crate::services::context::ExternalClients::default();
            let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &ext);
            if let Err(e) = crate::services::signals::emit_and_propagate(
                &ctx,
                db,
                &engine,
                &entity_type,
                &entity_id,
                "recommendation_rejected",
                "user_correction",
                Some(&format!(
                    "{{\"title\":\"{}\"}}",
                    removed.title.replace('"', "\\\"")
                )),
                0.3,
            ) {
                log::warn!(
                    "post-commit signal emission failed; \
                     repair_target=signals_engine \
                     entity={entity_id}: {e}"
                );
            }

            // Post-commit file write — best-effort cache.
            // routed through the schema-epoch fence.
            crate::intelligence::write_fence::post_commit_fenced_write(
                db,
                &dir,
                &intel,
                &format!("entity={entity_id}"),
            );

            Ok(())
        })
        .await
}

///  / Wave 0e: Mark an open commitment as done.
///
/// Removes the commitment at `index` from `openCommitments`, promotes it
/// into `valueDelivered` as a completion record, persists the updated
/// intelligence, and emits a `commitment_completed` signal so downstream
/// health scoring and briefing callouts see the transition.
///
/// Entity lookup and filesystem write mirror `dismiss_recommendation` so
/// the DB and on-disk intelligence.json stay in lockstep.
pub async fn mark_commitment_done(
    ctx: &ServiceContext<'_>,
    entity_id: &str,
    entity_type: &str,
    index: usize,
    state: &AppState,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let config = state.config.read().clone();
    let config = config.ok_or("No configuration loaded")?;
    let workspace_path = config.workspace_path.clone();

    let engine = state.signals.engine.clone();
    let entity_id = entity_id.to_string();
    let entity_type = entity_type.to_string();
    let now = ctx.clock.now().to_rfc3339();

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

            let mut intel = db
                .get_entity_intelligence(&entity_id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| {
                    format!(
                        "No DB intelligence row for {} — cannot mark commitment done",
                        entity_id
                    )
                })?;

            let commitment = {
                let list = intel
                    .open_commitments
                    .as_mut()
                    .ok_or_else(|| format!("Entity {} has no open commitments", entity_id))?;
                if index >= list.len() {
                    return Err(format!("Commitment index {} out of bounds", index));
                }
                list.remove(index)
            };

            // Promote into value_delivered as a completion record. The
            // "date" field takes now(); the original source is preserved so
            // the Context value-delivered chapter can show provenance.
            intel
                .value_delivered
                .push(crate::intelligence::io::ValueItem {
                    date: Some(now.clone()),
                    statement: commitment.description.clone(),
                    source: commitment.source.clone(),
                    impact: None,
                    item_source: Some(crate::intelligence::io::ItemSource {
                        source: "commitment_completed".to_string(),
                        confidence: 0.95,
                        sourced_at: now.clone(),
                        reference: commitment.owner.clone(),
                    }),
                    discrepancy: None,
                });

            // DB-first ordering. Commit canonical state; file + signal
            // run after as best-effort post-commit work.
            db.upsert_entity_intelligence(&intel)
                .map_err(|e| e.to_string())?;

            let clock = crate::services::context::SystemClock;
            let rng = crate::services::context::SystemRng;
            let ext = crate::services::context::ExternalClients::default();
            let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &ext);
            if let Err(e) = crate::services::signals::emit_and_propagate(
                &ctx,
                db,
                &engine,
                &entity_type,
                &entity_id,
                "commitment_completed",
                "user_curation",
                Some(&format!(
                    "{{\"description\":\"{}\"}}",
                    commitment.description.replace('"', "\\\"")
                )),
                0.85,
            ) {
                log::warn!(
                    "post-commit signal emission failed; \
                     repair_target=signals_engine \
                     entity={entity_id}: {e}"
                );
            }

            // routed through the schema-epoch fence.
            crate::intelligence::write_fence::post_commit_fenced_write(
                db,
                &dir,
                &intel,
                &format!("entity={entity_id}"),
            );

            Ok(())
        })
        .await
}

/// Get recommended actions for all entities (for use in the actions page).
pub fn get_all_recommended_actions(
    db: &ActionDb,
) -> Result<Vec<crate::intelligence::io::RecommendedAction>, String> {
    // Query all entity_assessment rows that have dimensions_json containing recommendedActions
    let conn = db.conn_ref();
    let mut stmt = conn
        .prepare("SELECT dimensions_json FROM entity_assessment WHERE dimensions_json IS NOT NULL")
        .map_err(|e| e.to_string())?;

    let mut all_actions = Vec::new();
    let rows = stmt
        .query_map([], |row| {
            let json: Option<String> = row.get(0)?;
            Ok(json)
        })
        .map_err(|e| e.to_string())?;

    for row in rows {
        if let Ok(Some(json)) = row {
            if let Ok(blob) = serde_json::from_str::<crate::intelligence::io::DimensionsBlob>(&json)
            {
                all_actions.extend(blob.recommended_actions);
            }
        }
    }

    Ok(all_actions)
}

#[cfg(test)]
mod mutation_smoke_tests {
    use crate::db::test_utils::test_db;
    use crate::db::{AccountType, DbAccount};
    use crate::intel_queue::{
        apply_enrichment_side_writes, compose_enrichment_intelligence_with_policy,
        EnrichmentComposition, EnrichmentInput,
    };
    use crate::intelligence::contamination::ContaminationValidation;
    use crate::intelligence::io::{IntelligenceJson, StakeholderInsight};
    use crate::intelligence::write_fence::post_commit_fenced_write;
    use crate::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};
    use crate::signals::propagation::PropagationEngine;
    use crate::state::AppState;
    use chrono::TimeZone;
    use rusqlite::params;
    use std::path::Path;

    fn test_ctx<'a>(
        clock: &'a FixedClock,
        rng: &'a SeedableRng,
        ext: &'a ExternalClients,
    ) -> ServiceContext<'a> {
        ServiceContext::test_live(clock, rng, ext)
    }

    fn make_account(id: &str) -> DbAccount {
        DbAccount {
            id: id.to_string(),
            name: format!("Account {id}"),
            lifecycle: Some("active".to_string()),
            arr: Some(100_000.0),
            health: None,
            contract_start: Some("2025-01-01".to_string()),
            contract_end: Some("2027-01-01".to_string()),
            nps: None,
            tracker_path: None,
            parent_id: None,
            account_type: AccountType::Customer,
            updated_at: chrono::Utc::now().to_rfc3339(),
            archived: false,
            keywords: None,
            keywords_extracted_at: None,
            metadata: None,
            ..Default::default()
        }
    }

    fn signal_count(db: &crate::db::ActionDb, entity_id: &str, signal_type: &str) -> i64 {
        db.conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM signal_events WHERE entity_id = ?1 AND signal_type = ?2",
                params![entity_id, signal_type],
                |row| row.get(0),
            )
            .unwrap_or(0)
    }

    fn seed_account_domain(db: &crate::db::ActionDb, account_id: &str, domain: &str) {
        db.conn_ref()
            .execute(
                "INSERT INTO account_domains (account_id, domain, source) VALUES (?1, ?2, 'test')",
                params![account_id, domain],
            )
            .expect("seed account domain");
    }

    fn make_enrichment_input(entity_id: &str, entity_dir: &Path) -> EnrichmentInput {
        EnrichmentInput {
            workspace: entity_dir.to_path_buf(),
            entity_dir: entity_dir.to_path_buf(),
            entity_id: entity_id.to_string(),
            entity_type: "account".to_string(),
            prompt: String::new(),
            file_manifest: Vec::new(),
            file_count: 0,
            computed_health: None,
            entity_name: format!("Account {entity_id}"),
            relationship: None,
            intelligence_context: None,
            active_preset: None,
        }
    }

    #[test]
    fn test_persist_entity_keywords() {
        let db = test_db();
        let account = make_account("acc-kw");
        db.upsert_account(&account).unwrap();
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        let keywords_json = r#"["onboarding", "enterprise", "SaaS"]"#;
        super::persist_entity_keywords(&ctx, &db, "account", "acc-kw", keywords_json)
            .expect("persist_entity_keywords");

        // Verify keywords stored
        let stored: Option<String> = db
            .conn_ref()
            .query_row(
                "SELECT keywords FROM accounts WHERE id = 'acc-kw'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(stored.as_deref(), Some(keywords_json));

        // Verify signal
        assert!(
            signal_count(&db, "acc-kw", "keywords_updated") > 0,
            "Expected keywords_updated signal"
        );
    }

    #[test]
    fn test_upsert_assessment_from_enrichment() {
        let db = test_db();
        let engine = PropagationEngine::default();
        let account = make_account("acc-intel");
        db.upsert_account(&account).unwrap();

        let intel = IntelligenceJson {
            entity_id: "acc-intel".to_string(),
            entity_type: "account".to_string(),
            enriched_at: chrono::Utc::now().to_rfc3339(),
            executive_assessment: Some("Strong account with growing adoption.".to_string()),
            ..Default::default()
        };
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        super::upsert_assessment_from_enrichment(
            &ctx,
            &db,
            &engine,
            "account",
            "acc-intel",
            &intel,
        )
        .expect("upsert_assessment_from_enrichment");

        // Verify entity_assessment row exists
        let exists: bool = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) > 0 FROM entity_assessment WHERE entity_id = 'acc-intel'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(exists, "entity_assessment row should exist");

        // Verify signal
        assert!(
            signal_count(&db, "acc-intel", "entity_intelligence_updated") > 0,
            "Expected entity_intelligence_updated signal"
        );
    }

    #[test]
    fn update_stakeholders_disk_db_atomicity_under_rollback() {
        let db = test_db();
        let engine = PropagationEngine::default();
        let account = make_account("acc-stakeholder-rollback");
        db.upsert_account(&account).unwrap();

        let dir = tempfile::tempdir().expect("tempdir");
        let old_intel = IntelligenceJson {
            entity_id: "acc-stakeholder-rollback".to_string(),
            entity_type: "account".to_string(),
            enriched_at: "2026-05-03T00:00:00Z".to_string(),
            executive_assessment: Some("old stakeholder state".to_string()),
            stakeholder_insights: vec![StakeholderInsight {
                name: "Old Owner".to_string(),
                role: Some("buyer".to_string()),
                ..Default::default()
            }],
            ..Default::default()
        };
        db.upsert_entity_intelligence(&old_intel).unwrap();
        post_commit_fenced_write(&db, dir.path(), &old_intel, "seed disk intelligence");
        let before_disk =
            std::fs::read_to_string(dir.path().join("intelligence.json")).expect("read seed disk");

        db.conn_ref()
            .execute_batch(
                "CREATE TRIGGER fail_stakeholders_updated_signal
                 BEFORE INSERT ON signal_events
                 WHEN NEW.signal_type = 'stakeholders_updated'
                 BEGIN
                   SELECT RAISE(ABORT, 'forced stakeholders_updated rollback');
                 END;",
            )
            .expect("install rollback trigger");

        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 3, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        let new_intel = crate::intelligence::apply_stakeholders_update_in_memory(
            old_intel.clone(),
            vec![StakeholderInsight {
                name: "New Owner".to_string(),
                role: Some("champion".to_string()),
                ..Default::default()
            }],
        )
        .expect("compose stakeholder update");

        let result = db.with_transaction(|tx| {
            tx.upsert_entity_intelligence(&new_intel)
                .map_err(|e| e.to_string())?;
            crate::services::signals::emit_and_propagate(
                &ctx,
                tx,
                &engine,
                "account",
                "acc-stakeholder-rollback",
                "stakeholders_updated",
                "user_edit",
                None,
                0.9,
            )
            .map_err(|e| format!("signal emit failed: {e}"))?;
            Ok(())
        });
        if result.is_ok() {
            crate::intelligence::write_fence::post_commit_fenced_write(
                &db,
                dir.path(),
                &new_intel,
                "test stakeholder rollback",
            );
        }

        assert!(
            result.is_err_and(|err| err.contains("forced stakeholders_updated rollback")),
            "transaction should surface forced rollback"
        );
        let after_disk = std::fs::read_to_string(dir.path().join("intelligence.json"))
            .expect("read disk after rollback");
        assert_eq!(
            after_disk, before_disk,
            "disk cache must not change when stakeholder DB transaction rolls back"
        );
        let persisted = db
            .get_entity_intelligence("acc-stakeholder-rollback")
            .expect("read DB intelligence")
            .expect("existing DB intelligence");
        assert_eq!(
            persisted.executive_assessment.as_deref(),
            Some("old stakeholder state"),
            "DB intelligence must roll back to the pre-update state"
        );
    }

    #[test]
    fn enrich_entity_disk_db_atomicity_under_rollback() {
        let db = test_db();
        let engine = PropagationEngine::default();
        let account = make_account("acc-enrich-rollback");
        db.upsert_account(&account).unwrap();

        let dir = tempfile::tempdir().expect("tempdir");
        let old_intel = IntelligenceJson {
            entity_id: "acc-enrich-rollback".to_string(),
            entity_type: "account".to_string(),
            enriched_at: "2026-05-03T00:00:00Z".to_string(),
            executive_assessment: Some("old enrichment state".to_string()),
            ..Default::default()
        };
        db.upsert_entity_intelligence(&old_intel).unwrap();
        post_commit_fenced_write(&db, dir.path(), &old_intel, "seed disk intelligence");
        let before_disk =
            std::fs::read_to_string(dir.path().join("intelligence.json")).expect("read seed disk");

        db.conn_ref()
            .execute_batch(
                "CREATE TRIGGER fail_entity_intelligence_updated_signal
                 BEFORE INSERT ON signal_events
                 WHEN NEW.signal_type = 'entity_intelligence_updated'
                 BEGIN
                   SELECT RAISE(ABORT, 'forced entity_intelligence_updated rollback');
                 END;",
            )
            .expect("install rollback trigger");

        let new_intel = IntelligenceJson {
            entity_id: "acc-enrich-rollback".to_string(),
            entity_type: "account".to_string(),
            enriched_at: "2026-05-03T01:00:00Z".to_string(),
            executive_assessment: Some("new enrichment state".to_string()),
            ..Default::default()
        };
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 3, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        let result = super::upsert_assessment_from_enrichment(
            &ctx,
            &db,
            &engine,
            "account",
            "acc-enrich-rollback",
            &new_intel,
        );
        if result.is_ok() {
            crate::intel_queue::fenced_write_enrichment_intelligence(&db, dir.path(), &new_intel);
        }

        assert!(
            result.is_err_and(|err| err.contains("forced entity_intelligence_updated rollback")),
            "enrichment persistence should surface forced rollback"
        );
        let after_disk = std::fs::read_to_string(dir.path().join("intelligence.json"))
            .expect("read disk after rollback");
        assert_eq!(
            after_disk, before_disk,
            "disk cache must not change when enrichment DB transaction rolls back"
        );
        let persisted = db
            .get_entity_intelligence("acc-enrich-rollback")
            .expect("read DB intelligence")
            .expect("existing DB intelligence");
        assert_eq!(
            persisted.executive_assessment.as_deref(),
            Some("old enrichment state"),
            "DB intelligence must roll back to the pre-enrichment state"
        );
    }

    #[test]
    fn compose_enrichment_skips_persistence_on_contamination_reject() {
        let db = test_db();
        let target = make_account("acc-contamination-target");
        let foreign = make_account("acc-contamination-foreign");
        db.upsert_account(&target).unwrap();
        db.upsert_account(&foreign).unwrap();
        seed_account_domain(&db, "acc-contamination-target", "target.example");
        seed_account_domain(&db, "acc-contamination-foreign", "vip-test.com");

        let dir = tempfile::tempdir().expect("tempdir");
        let prior = IntelligenceJson {
            entity_id: "acc-contamination-target".to_string(),
            entity_type: "account".to_string(),
            enriched_at: "2026-05-03T00:00:00Z".to_string(),
            executive_assessment: Some("prior trusted assessment".to_string()),
            ..Default::default()
        };
        db.upsert_entity_intelligence(&prior).unwrap();
        post_commit_fenced_write(&db, dir.path(), &prior, "seed disk intelligence");
        let before_disk =
            std::fs::read_to_string(dir.path().join("intelligence.json")).expect("read seed disk");

        let input = make_enrichment_input("acc-contamination-target", dir.path());
        let contaminated = IntelligenceJson {
            entity_id: "acc-contamination-target".to_string(),
            entity_type: "account".to_string(),
            enriched_at: "2026-05-03T01:00:00Z".to_string(),
            executive_assessment: Some(
                "WordPress VIP performance at vip-test.com remains stable.".to_string(),
            ),
            ..Default::default()
        };

        let composition = compose_enrichment_intelligence_with_policy(
            &db,
            &input,
            &contaminated,
            None,
            ContaminationValidation::RejectOnHit,
        )
        .expect("compose contamination reject");

        match composition {
            EnrichmentComposition::SkipDueToContamination { prior, .. } => {
                assert_eq!(
                    prior.executive_assessment.as_deref(),
                    Some("prior trusted assessment")
                );
            }
            EnrichmentComposition::Persist(_) => {
                panic!("contaminated enrichment should skip persistence")
            }
        }

        let after_disk = std::fs::read_to_string(dir.path().join("intelligence.json"))
            .expect("read disk after reject");
        assert_eq!(
            after_disk, before_disk,
            "disk cache must not change when contamination rejects enrichment"
        );
        let persisted = db
            .get_entity_intelligence("acc-contamination-target")
            .expect("read DB intelligence")
            .expect("existing DB intelligence");
        assert_eq!(
            persisted.executive_assessment.as_deref(),
            Some("prior trusted assessment"),
            "DB intelligence must preserve the prior row on contamination reject"
        );
    }

    #[test]
    fn compose_enrichment_full_path_rollback_atomicity() {
        let db = test_db();
        let engine = PropagationEngine::default();
        let account = make_account("acc-compose-rollback");
        db.upsert_account(&account).unwrap();
        seed_account_domain(&db, "acc-compose-rollback", "compose.example");

        let dir = tempfile::tempdir().expect("tempdir");
        let prior = IntelligenceJson {
            entity_id: "acc-compose-rollback".to_string(),
            entity_type: "account".to_string(),
            enriched_at: "2026-05-03T00:00:00Z".to_string(),
            executive_assessment: Some("old compose state".to_string()),
            ..Default::default()
        };
        db.upsert_entity_intelligence(&prior).unwrap();
        post_commit_fenced_write(&db, dir.path(), &prior, "seed disk intelligence");
        let before_disk =
            std::fs::read_to_string(dir.path().join("intelligence.json")).expect("read seed disk");

        db.conn_ref()
            .execute_batch(
                "CREATE TRIGGER fail_full_enrichment_path_signal
                 BEFORE INSERT ON signal_events
                 WHEN NEW.signal_type = 'entity_intelligence_updated'
                 BEGIN
                   SELECT RAISE(ABORT, 'forced compose full path rollback');
                 END;",
            )
            .expect("install rollback trigger");

        let input = make_enrichment_input("acc-compose-rollback", dir.path());
        let incoming = IntelligenceJson {
            entity_id: "acc-compose-rollback".to_string(),
            entity_type: "account".to_string(),
            enriched_at: "2026-05-03T01:00:00Z".to_string(),
            executive_assessment: Some("new compose state".to_string()),
            stakeholder_insights: vec![StakeholderInsight {
                name: "New Buyer".to_string(),
                role: Some("economic buyer".to_string()),
                engagement: Some("engaged".to_string()),
                ..Default::default()
            }],
            ..Default::default()
        };
        let prepared = match compose_enrichment_intelligence_with_policy(
            &db,
            &input,
            &incoming,
            None,
            ContaminationValidation::Off,
        )
        .expect("compose full path")
        {
            EnrichmentComposition::Persist(prepared) => prepared,
            EnrichmentComposition::SkipDueToContamination { .. } => {
                panic!("clean enrichment unexpectedly rejected for contamination")
            }
        };

        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 3, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        let state = AppState::new();

        let result = db.with_transaction(|tx| {
            apply_enrichment_side_writes(&ctx, tx, &input, &prepared)?;
            super::upsert_assessment_from_enrichment_in_active_transaction(
                &ctx,
                tx,
                &engine,
                "account",
                "acc-compose-rollback",
                prepared.intelligence(),
            )
        });
        if result.is_ok() {
            crate::intel_queue::fenced_write_enrichment_intelligence(
                &db,
                dir.path(),
                prepared.intelligence(),
            );
            crate::intel_queue::run_enrichment_post_commit_side_effects(
                &state,
                &input,
                &db,
                prepared.intelligence(),
            );
        }

        assert!(
            result.is_err_and(|err| err.contains("forced compose full path rollback")),
            "full enrichment path should surface forced rollback"
        );
        let suggestion_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM stakeholder_suggestions WHERE account_id = 'acc-compose-rollback'",
                [],
                |row| row.get(0),
            )
            .expect("count stakeholder suggestions");
        assert_eq!(
            suggestion_count, 0,
            "compose stakeholder side writes must roll back with enrichment upsert"
        );
        let after_disk = std::fs::read_to_string(dir.path().join("intelligence.json"))
            .expect("read disk after rollback");
        assert_eq!(
            after_disk, before_disk,
            "disk cache must not change when full enrichment transaction rolls back"
        );
        let persisted = db
            .get_entity_intelligence("acc-compose-rollback")
            .expect("read DB intelligence")
            .expect("existing DB intelligence");
        assert_eq!(
            persisted.executive_assessment.as_deref(),
            Some("old compose state"),
            "DB intelligence must roll back to the pre-compose state"
        );
    }

    #[test]
    fn test_recompute_entity_health() {
        let db = test_db();
        let account = make_account("acc-health");
        db.upsert_account(&account).unwrap();

        // Seed minimal intelligence so recompute has something to work with
        let intel = IntelligenceJson {
            entity_id: "acc-health".to_string(),
            entity_type: "account".to_string(),
            enriched_at: chrono::Utc::now().to_rfc3339(),
            ..Default::default()
        };
        db.upsert_entity_intelligence(&intel).unwrap();
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        super::recompute_entity_health(&ctx, &db, "acc-health", "account")
            .expect("recompute_entity_health");

        // Verify entity_quality updated with health_score
        let score: Option<f64> = db
            .conn_ref()
            .query_row(
                "SELECT health_score FROM entity_quality WHERE entity_id = 'acc-health'",
                [],
                |row| row.get(0),
            )
            .ok();
        assert!(score.is_some(), "entity_quality should have a health_score");
    }

    #[test]
    fn test_recompute_health_skips_non_account() {
        let db = test_db();
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        // Should silently succeed for non-account types
        let result = super::recompute_entity_health(&ctx, &db, "proj-1", "project");
        assert!(result.is_ok(), "Should be Ok for non-account entity type");
    }

    #[test]
    fn test_persist_keywords_skips_unsupported_type() {
        let db = test_db();
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        // Should silently succeed for unsupported entity types
        let result = super::persist_entity_keywords(&ctx, &db, "person", "p-1", r#"["test"]"#);
        assert!(result.is_ok(), "Should be Ok for unsupported entity type");

        // No signal should be emitted
        assert_eq!(
            signal_count(&db, "p-1", "keywords_updated"),
            0,
            "No signal for unsupported type"
        );
    }
}

///  end-to-end: parse Glean JSON → normalize → upsert to DB column → re-read.
///
/// This test exercises the full path from Glean's raw response through
/// `parse_leading_signals`, `upsert_health_outlook_signals`, and the SELECT
/// read that populates `AccountDetailResult.glean_signals`. It validates:
///  1. Bucket dispatch (champion_risk, channel_sentiment, commercial_signals survive)
///  2. JSON roundtrip through the DB column (camelCase storage ↔ camelCase struct)
///  3. Signal emissions (champion_at_risk, sentiment_divergence) fire correctly
#[cfg(test)]
mod dos15_leading_signals_db_tests {
    use crate::db::test_utils::test_db;
    use crate::db::{AccountType, DbAccount};
    use crate::intelligence::glean_leading_signals::{parse_leading_signals, HealthOutlookSignals};
    use crate::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};
    use crate::signals::propagation::PropagationEngine;
    use chrono::TimeZone;

    fn test_ctx<'a>(
        clock: &'a FixedClock,
        rng: &'a SeedableRng,
        ext: &'a ExternalClients,
    ) -> ServiceContext<'a> {
        ServiceContext::test_live(clock, rng, ext)
    }

    fn seed_account(db: &crate::db::ActionDb, id: &str) {
        let account = DbAccount {
            id: id.to_string(),
            name: format!("Test Account {id}"),
            lifecycle: Some("active".to_string()),
            arr: Some(200_000.0),
            account_type: AccountType::Customer,
            ..Default::default()
        };
        db.upsert_account(&account).unwrap();
        db.conn_ref()
            .execute(
                "INSERT OR IGNORE INTO entity_assessment (entity_id) VALUES (?1)",
                rusqlite::params![id],
            )
            .unwrap();
    }

    fn signal_count(db: &crate::db::ActionDb, entity_id: &str, signal_type: &str) -> i64 {
        db.conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM signal_events WHERE entity_id = ?1 AND signal_type = ?2",
                rusqlite::params![entity_id, signal_type],
                |row| row.get(0),
            )
            .unwrap_or(0)
    }

    /// Core roundtrip: parse Glean's snake_case JSON → upsert → SELECT → verify.
    #[test]
    fn glean_json_roundtrip_through_db_column() {
        let db = test_db();
        let engine = PropagationEngine::default();
        let entity_id = "dos15-roundtrip-test";
        seed_account(&db, entity_id);
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        let glean_output = r#"{
            "champion_risk": {
                "champion_name": "Robin Taylor",
                "at_risk": true,
                "risk_level": "high",
                "risk_evidence": ["email response slowed 3x", "missed last QBR"],
                "backup_champion_candidates": [
                    { "name": "Jamie Lee", "role": "VP Ops", "engagement_level": "medium" }
                ]
            },
            "channel_sentiment": {
                "email": { "sentiment": "cooling", "trend_30d": "worsening" },
                "support_tickets": { "sentiment": "frustrated", "trend_30d": "worsening" },
                "divergence_detected": true,
                "divergence_summary": "tickets frustrated while Slack still cordial"
            },
            "commercial_signals": {
                "arr_direction": "flat",
                "payment_behavior": "on-time"
            },
            "quote_wall": [
                {
                    "quote": "We need better integration support.",
                    "speaker": "Robin Taylor",
                    "role": "Director of Data",
                    "date": "2026-03-15",
                    "source": "Gong",
                    "sentiment": "negative"
                }
            ]
        }"#;

        // Step 1: parse Glean's snake_case output into the normalized struct.
        let parsed =
            parse_leading_signals(glean_output).expect("parse_leading_signals should succeed");

        // Verify bucket dispatch before persistence.
        {
            let cr = parsed.champion_risk.as_ref().expect("champion_risk bucket");
            assert!(cr.at_risk, "champion should be at_risk");
            assert_eq!(cr.champion_name.as_deref(), Some("Robin Taylor"));
            assert_eq!(cr.risk_evidence.len(), 2, "2 evidence items");
            assert_eq!(cr.backup_champion_candidates.len(), 1);

            let cs = parsed
                .channel_sentiment
                .as_ref()
                .expect("channel_sentiment bucket");
            assert!(cs.divergence_detected, "divergence_detected should be true");

            let comm = parsed
                .commercial_signals
                .as_ref()
                .expect("commercial_signals bucket");
            assert_eq!(comm.arr_direction.as_deref(), Some("flat"));

            assert_eq!(parsed.quote_wall.len(), 1, "quote_wall should have 1 entry");
        }

        // Step 2: upsert to DB via the service function (also emits signals).
        super::upsert_health_outlook_signals(&ctx, &db, &engine, "account", entity_id, &parsed)
            .expect("upsert_health_outlook_signals should succeed");

        // Step 3: re-read from DB column (mirrors AccountDetailResult assembly).
        let stored_json: Option<String> = db
            .conn_ref()
            .query_row(
                "SELECT health_outlook_signals_json FROM entity_assessment WHERE entity_id = ?1",
                rusqlite::params![entity_id],
                |row| row.get(0),
            )
            .expect("SELECT should succeed");

        let stored_json = stored_json.expect("health_outlook_signals_json should not be NULL");
        let reread: HealthOutlookSignals =
            serde_json::from_str(&stored_json).expect("DB JSON should deserialize");

        // Step 4: verify every populated field survived the roundtrip.
        let cr = reread
            .champion_risk
            .as_ref()
            .expect("champion_risk after roundtrip");
        assert_eq!(cr.champion_name.as_deref(), Some("Robin Taylor"));
        assert!(cr.at_risk);
        assert_eq!(cr.risk_level.as_deref(), Some("high"));
        assert_eq!(cr.risk_evidence.len(), 2);
        assert_eq!(cr.backup_champion_candidates.len(), 1);
        assert_eq!(cr.backup_champion_candidates[0].name, "Jamie Lee");

        let cs = reread
            .channel_sentiment
            .as_ref()
            .expect("channel_sentiment after roundtrip");
        assert!(cs.divergence_detected);
        assert_eq!(
            cs.divergence_summary.as_deref(),
            Some("tickets frustrated while Slack still cordial")
        );

        assert_eq!(reread.quote_wall.len(), 1);
        assert_eq!(
            reread.quote_wall[0].quote,
            "We need better integration support."
        );

        // Step 5: verify derived signals were emitted correctly.
        assert!(
            signal_count(&db, entity_id, "champion_at_risk") > 0,
            "champion_at_risk signal should be emitted"
        );
        assert!(
            signal_count(&db, entity_id, "sentiment_divergence") > 0,
            "sentiment_divergence signal should be emitted"
        );
        // No competitor_decision_relevant or budget_cycle_locked in this fixture.
        assert_eq!(
            signal_count(&db, entity_id, "competitor_decision_relevant"),
            0,
            "no competitor signal expected"
        );
        assert_eq!(
            signal_count(&db, entity_id, "budget_cycle_locked"),
            0,
            "no budget_cycle_locked signal expected"
        );
    }

    /// NULL-safety: reading an account with no Glean enrichment must not panic.
    #[test]
    fn null_column_reads_as_none() {
        let db = test_db();
        let entity_id = "dos15-no-glean";
        seed_account(&db, entity_id);

        let result: Option<String> = db
            .conn_ref()
            .query_row(
                "SELECT health_outlook_signals_json FROM entity_assessment WHERE entity_id = ?1",
                rusqlite::params![entity_id],
                |row| row.get(0),
            )
            .ok()
            .flatten();

        assert!(result.is_none(), "unset column should read as NULL");
    }

    /// Idempotency: calling upsert twice overwrites cleanly — no duplicate rows.
    #[test]
    fn upsert_is_idempotent() {
        let db = test_db();
        let engine = PropagationEngine::default();
        let entity_id = "dos15-idempotent";
        seed_account(&db, entity_id);
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        let first = parse_leading_signals(r#"{"champion_risk": null, "quote_wall": []}"#)
            .expect("parse first");
        super::upsert_health_outlook_signals(&ctx, &db, &engine, "account", entity_id, &first)
            .expect("first upsert");

        let second = parse_leading_signals(
            r#"{"champion_risk": {"champion_name": "New Champion", "at_risk": false, "risk_evidence": []}, "quote_wall": []}"#,
        )
        .expect("parse second");
        super::upsert_health_outlook_signals(&ctx, &db, &engine, "account", entity_id, &second)
            .expect("second upsert");

        // Read back — should reflect second write.
        let stored: Option<String> = db
            .conn_ref()
            .query_row(
                "SELECT health_outlook_signals_json FROM entity_assessment WHERE entity_id = ?1",
                rusqlite::params![entity_id],
                |row| row.get(0),
            )
            .ok()
            .flatten();

        let reread: HealthOutlookSignals =
            serde_json::from_str(&stored.expect("should be set")).expect("deserialize");
        let cr = reread.champion_risk.as_ref().expect("champion_risk");
        assert_eq!(cr.champion_name.as_deref(), Some("New Champion"));
    }
}

#[cfg(test)]
mod inferred_relationship_tests {
    use super::upsert_inferred_relationships_from_enrichment;
    use crate::db::person_relationships::UpsertRelationship;
    use crate::db::test_utils::test_db;
    use crate::intelligence::prompts::InferredRelationship;
    use crate::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};
    use chrono::TimeZone;

    fn test_ctx<'a>(
        clock: &'a FixedClock,
        rng: &'a SeedableRng,
        ext: &'a ExternalClients,
    ) -> ServiceContext<'a> {
        ServiceContext::test_live(clock, rng, ext)
    }

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
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        let inferred = vec![InferredRelationship {
            from_person_id: "p1".to_string(),
            to_person_id: "p2".to_string(),
            relationship_type: "collaborator".to_string(),
            rationale: Some("They co-own onboarding rollout workstreams.".to_string()),
        }];

        let inserted = upsert_inferred_relationships_from_enrichment(
            &ctx, &db, &engine, "account", "acc-1", &inferred,
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
            &ctx, &db, &engine, "account", "acc-1", &inferred,
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
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
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
            &ctx, &db, &engine, "account", "acc-1", &inferred,
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
    use crate::intel_queue::{
        apply_enrichment_side_writes, compose_enrichment_intelligence,
        fenced_write_enrichment_intelligence, run_enrichment_post_commit_side_effects,
        EnrichmentComposition, EnrichmentInput,
    };
    use crate::intelligence::{
        write_intelligence_json, AccountHealth, ConsistencyStatus, DimensionScore, HealthSource,
        HealthTrend, IntelRisk, IntelligenceJson, RelationshipDimensions,
    };
    use crate::state::AppState;

    /// Live acceptance check for using the user's real local dataset.
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
        let ctx = state.live_service_context();
        let intel = enrich_entity(&ctx, entity_id.clone(), entity_type.clone(), &state, None)
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

        let refresh = crate::services::meetings::refresh_meeting_briefing_full(
            &ctx,
            &state,
            &meeting_id,
            None,
        )
        .await
        .expect("refresh_meeting_briefing_full failed");

        assert!(
            refresh.prep_rebuilt_sync || refresh.prep_queued,
            "refresh should rebuild prep sync or queue it"
        );

        let detail = crate::services::meetings::get_meeting_intelligence(&ctx, &state, &meeting_id)
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

    /// Live deterministic-guardrail validation for acceptance criteria:
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
                headline: None,
                evidence: None,
                kind_label: None,
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
            active_preset: None,
        };

        let db = ActionDb::open().expect("open DB for first enrichment persistence");
        let ctx = state.live_service_context();
        let first_prepared =
            match compose_enrichment_intelligence(&state, &db, &input, &contradictory, None)
                .expect("first compose_enrichment_intelligence failed")
            {
                EnrichmentComposition::Persist(prepared) => prepared,
                EnrichmentComposition::SkipDueToContamination { .. } => {
                    panic!("first enrichment unexpectedly rejected for contamination")
                }
            };
        db.with_transaction(|tx| {
            apply_enrichment_side_writes(&ctx, tx, &input, &first_prepared)?;
            super::upsert_assessment_from_enrichment_in_active_transaction(
                &ctx,
                tx,
                &state.signals.engine,
                &input.entity_type,
                &input.entity_id,
                first_prepared.intelligence(),
            )
        })
        .expect("first enrichment DB persistence failed");
        fenced_write_enrichment_intelligence(&db, &input.entity_dir, first_prepared.intelligence());
        run_enrichment_post_commit_side_effects(&state, &input, &db, first_prepared.intelligence());
        let first = first_prepared.into_intelligence();

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
        let db = ActionDb::open().expect("open DB for second enrichment persistence");
        let ctx = state.live_service_context();
        let second_prepared =
            match compose_enrichment_intelligence(&state, &db, &input, &clean, None)
                .expect("second compose_enrichment_intelligence failed")
            {
                EnrichmentComposition::Persist(prepared) => prepared,
                EnrichmentComposition::SkipDueToContamination { .. } => {
                    panic!("second enrichment unexpectedly rejected for contamination")
                }
            };
        db.with_transaction(|tx| {
            apply_enrichment_side_writes(&ctx, tx, &input, &second_prepared)?;
            super::upsert_assessment_from_enrichment_in_active_transaction(
                &ctx,
                tx,
                &state.signals.engine,
                &input.entity_type,
                &input.entity_id,
                second_prepared.intelligence(),
            )
        })
        .expect("second enrichment DB persistence failed");
        fenced_write_enrichment_intelligence(
            &db,
            &input.entity_dir,
            second_prepared.intelligence(),
        );
        run_enrichment_post_commit_side_effects(
            &state,
            &input,
            &db,
            second_prepared.intelligence(),
        );
        let second = second_prepared.into_intelligence();

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
            // Test cleanup: best-effort restore. Bypasses the schema-epoch
            // fence intentionally — this runs at end-of-test to restore the
            // pre-test workspace state and has no live migration to honor.
            // fence-exempt: test-cleanup
            write_intelligence_json(&entity_dir, &prev_file).ok();
        } else {
            std::fs::remove_file(entity_dir.join("intelligence.json")).ok();
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

    /// Wave 1 live acceptance  on an encrypted snapshot of the
    /// user's real DB. Safe: mutates backup only.
    #[test]
    #[ignore = "Live validation: uses real DB snapshot and performs destructive purge checks on snapshot only"]
    fn wave1_live_snapshot_i503_i528_acceptance() {
        let live_db = ActionDb::open().expect("open live DB");
        let backup_path = crate::db_backup::backup_database(&live_db).expect("create live backup");
        let snapshot_db =
            ActionDb::open_at(PathBuf::from(&backup_path)).expect("open snapshot backup DB");

        // ---------------------------------------------------------------------
        // structured health write/read + legacy compatibility
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
                sufficient_data: false, // Only 1 dimension populated in test
                trend: HealthTrend {
                    direction: "improving".to_string(),
                    rationale: Some("Usage and expansion improved".to_string()),
                    timeframe: "30d".to_string(),
                    confidence: 0.7,
                    ..Default::default()
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
                    key_advocate_health: DimensionScore::default(),
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
        // purge semantics (glean + google) against snapshot
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
                "INSERT INTO account_stakeholders (account_id, person_id, data_source)
                 VALUES (?1, ?2, 'glean')",
                params![account_glean, glean_person_id],
            )
            .expect("seed glean stakeholder");
        snapshot_db
            .conn_ref()
            .execute(
                "INSERT INTO account_stakeholder_roles (account_id, person_id, role, data_source)
                 VALUES (?1, ?2, 'champion', 'glean')",
                params![account_glean, glean_person_id],
            )
            .expect("seed glean stakeholder role");
        snapshot_db
            .conn_ref()
            .execute(
                "INSERT INTO account_stakeholders (account_id, person_id, data_source)
                 VALUES (?1, ?2, 'google')",
                params![account_google, google_person_id],
            )
            .expect("seed google stakeholder");
        snapshot_db
            .conn_ref()
            .execute(
                "INSERT INTO account_stakeholder_roles (account_id, person_id, role, data_source)
                 VALUES (?1, ?2, 'champion', 'google')",
                params![account_google, google_person_id],
            )
            .expect("seed google stakeholder role");
        snapshot_db
            .conn_ref()
            .execute(
                "INSERT INTO account_stakeholders (account_id, person_id, data_source)
                 VALUES (?1, ?2, 'user')",
                params![account_user, user_person_id],
            )
            .expect("seed user stakeholder");
        snapshot_db
            .conn_ref()
            .execute(
                "INSERT INTO account_stakeholder_roles (account_id, person_id, role, data_source)
                 VALUES (?1, ?2, 'champion', 'user')",
                params![account_user, user_person_id],
            )
            .expect("seed user stakeholder role");

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

    /// Wave 1 live acceptance  against real data enrichment path.
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

        let ctx = state.live_service_context();
        let _ = enrich_entity(
            &ctx,
            account_id.clone(),
            "account".to_string(),
            &state,
            None,
        )
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

        let _ = enrich_entity(
            &ctx,
            account_id.clone(),
            "account".to_string(),
            &state,
            None,
        )
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
