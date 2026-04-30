//! Report generation service — async dispatch with spawn_blocking.

use std::sync::Arc;

use tauri::{AppHandle, Emitter};

use crate::reports::generator::run_report_generation;
use crate::reports::swot::gather_swot_input;
use crate::reports::{get_report, get_reports_for_entity, upsert_report, ReportRow};
use crate::state::AppState;

/// Generate a report for an entity.
/// Blocks for ~60-300s (PTY call). Returns the stored report row.
///
/// I547: For `book_of_business`, uses parallel section generation with
/// optional Glean pre-fetch. Falls back to monolithic on failure.
pub async fn generate_report(
    ctx: &crate::services::context::ServiceContext<'_>,
    state: &Arc<AppState>,
    entity_id: &str,
    entity_type: &str,
    report_type_str: &str,
    spotlight_account_ids: Option<&[String]>,
    app_handle: Option<AppHandle>,
) -> Result<ReportRow, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let state = state.clone();
    let entity_id = entity_id.to_string();
    let entity_type = entity_type.to_string();
    let report_type_str = report_type_str.to_string();
    let spotlight_account_ids = spotlight_account_ids.map(|s| s.to_vec());

    let task = tauri::async_runtime::spawn_blocking(move || -> Result<ReportRow, String> {
        // Phase 1: Gather input under brief DB lock
        if report_type_str == "book_of_business" {
            let ctx = state.live_service_context();
            return generate_book_of_business(
                &ctx,
                &state,
                spotlight_account_ids.as_deref(),
                app_handle.as_ref(),
            );
        }
        if report_type_str == "swot" {
            let ctx = state.live_service_context();
            return generate_swot_report(
                &ctx,
                &state,
                &entity_id,
                &entity_type,
                app_handle.as_ref(),
            );
        }

        let mut input = {
            let db = crate::db::ActionDb::open().map_err(|e| format!("DB open failed: {e}"))?;

            let config_guard = state.config.read();
            let config = config_guard.as_ref().ok_or("Config not initialized")?;
            let workspace = std::path::Path::new(&config.workspace_path);
            let ai_models = config.ai_models.clone();

            let ctx_arc = state.context_provider();
            let ctx_provider = ctx_arc.as_ref();

            match report_type_str.as_str() {
                "swot" => gather_swot_input(
                    workspace,
                    &db,
                    &entity_id,
                    &entity_type,
                    ai_models,
                    ctx_provider,
                )?,
                "account_health" => {
                    let active_preset = config.role.clone();
                    crate::reports::account_health::gather_account_health_input(
                        workspace,
                        &db,
                        &entity_id,
                        ai_models,
                        &active_preset,
                        ctx_provider,
                    )?
                }
                "weekly_impact" => {
                    let active_preset = config.role.clone();
                    crate::reports::weekly_impact::gather_weekly_impact_input(
                        workspace,
                        &db,
                        ai_models,
                        &active_preset,
                    )?
                }
                "monthly_wrapped" => {
                    let active_preset = config.role.clone();
                    crate::reports::monthly_wrapped::gather_monthly_wrapped_input(
                        workspace,
                        &db,
                        ai_models,
                        &active_preset,
                    )?
                }
                "ebr_qbr" => {
                    let active_preset = config.role.clone();
                    crate::reports::ebr_qbr::gather_ebr_qbr_input(
                        workspace,
                        &db,
                        &entity_id,
                        ai_models,
                        &active_preset,
                        ctx_provider,
                    )?
                }
                _ => return Err(format!("Unknown report type: {}", report_type_str)),
            }
        };

        // Phase 1.5: Inject relevant user context into prompt (I413).
        if let Ok(db_ctx) = crate::db::ActionDb::open() {
            crate::reports::prompts::append_user_context(
                &mut input.prompt,
                &db_ctx,
                Some(state.embedding_model.as_ref()),
                &input.entity_name,
            );
        }

        // Step 2: Run PTY (no DB lock held)
        let stdout = run_report_generation(&input)?;

        // Parse report-type-specific response
        let content_json = match report_type_str.as_str() {
            "swot" => {
                let swot = crate::reports::swot::parse_swot_response(&stdout)?;
                serde_json::to_string(&swot)
                    .map_err(|e| format!("Failed to serialize SWOT: {}", e))?
            }
            "account_health" => {
                let health =
                    crate::reports::account_health::parse_account_health_response(&stdout)?;
                serde_json::to_string(&health)
                    .map_err(|e| format!("Failed to serialize Account Health: {}", e))?
            }
            "weekly_impact" => {
                let impact = crate::reports::weekly_impact::parse_weekly_impact_response(&stdout)?;
                serde_json::to_string(&impact)
                    .map_err(|e| format!("Failed to serialize Weekly Impact: {}", e))?
            }
            "monthly_wrapped" => {
                let wrapped =
                    crate::reports::monthly_wrapped::parse_monthly_wrapped_response(&stdout)?;
                serde_json::to_string(&wrapped)
                    .map_err(|e| format!("Failed to serialize Monthly Wrapped: {}", e))?
            }
            "ebr_qbr" => {
                let ebr = crate::reports::ebr_qbr::parse_ebr_qbr_response(&stdout)?;
                serde_json::to_string(&ebr)
                    .map_err(|e| format!("Failed to serialize EBR/QBR: {}", e))?
            }
            _ => return Err(format!("Unknown report type: {}", report_type_str)),
        };

        // Phase 3: Write to DB
        let db = crate::db::ActionDb::open().map_err(|e| format!("DB open failed: {e}"))?;

        let report_id = upsert_report(
            &db,
            &input.entity_id,
            &input.entity_type,
            &input.report_type,
            &content_json,
            &input.intel_hash,
        )?;

        get_report(
            &db,
            &input.entity_id,
            &input.entity_type,
            &input.report_type,
        )?
        .ok_or_else(|| format!("Report {} not found after insert", report_id))
    });

    match task.await {
        Ok(result) => result,
        Err(e) => Err(format!("Report generation task panicked: {}", e)),
    }
}

fn generate_swot_report(
    ctx: &crate::services::context::ServiceContext<'_>,
    state: &Arc<AppState>,
    entity_id: &str,
    entity_type: &str,
    app_handle: Option<&AppHandle>,
) -> Result<ReportRow, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let gathered = {
        let db = crate::db::ActionDb::open().map_err(|e| format!("DB open failed: {e}"))?;

        let config_guard = state.config.read();
        let config = config_guard.as_ref().ok_or("Config not initialized")?;
        let workspace = std::path::Path::new(&config.workspace_path);
        let ai_models = config.ai_models.clone();
        let ctx_arc = state.context_provider();
        let ctx_provider = ctx_arc.as_ref();

        crate::reports::swot::gather_swot_data(
            workspace,
            &db,
            entity_id,
            entity_type,
            ai_models,
            ctx_provider,
        )?
    };

    let content = crate::reports::swot::run_parallel_swot_generation(&gathered, app_handle)?;
    let content_json =
        serde_json::to_string(&content).map_err(|e| format!("Failed to serialize SWOT: {}", e))?;

    let db = crate::db::ActionDb::open().map_err(|e| format!("DB open failed: {e}"))?;

    let report_id = upsert_report(
        &db,
        &gathered.entity_id,
        &gathered.entity_type,
        "swot",
        &content_json,
        &gathered.intel_hash,
    )?;

    get_report(&db, &gathered.entity_id, &gathered.entity_type, "swot")?
        .ok_or_else(|| format!("Report {} not found after insert", report_id))
}

/// I547: Parallel Book of Business generation pipeline.
///
/// Phase 1: Gather data (DB lock)
/// Step 2: Glean pre-fetch (no lock, when connected)
/// Phase 3: Wave 1 — 6 sections in parallel
/// Phase 4: Wave 2 — executiveSummary sequential
/// Phase 5: Merge + write to DB
fn generate_book_of_business(
    ctx: &crate::services::context::ServiceContext<'_>,
    state: &Arc<AppState>,
    spotlight_account_ids: Option<&[String]>,
    app_handle: Option<&AppHandle>,
) -> Result<ReportRow, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    use crate::reports::book_of_business::*;

    // Phase 1: Gather data under brief DB lock
    let mut gather = {
        let db = crate::db::ActionDb::open().map_err(|e| format!("DB open failed: {e}"))?;

        let config_guard = state.config.read();
        let config = config_guard.as_ref().ok_or("Config not initialized")?;
        let workspace = std::path::Path::new(&config.workspace_path);
        let ai_models = config.ai_models.clone();
        let active_preset = config.role.clone();

        gather_book_of_business_data(
            workspace,
            &db,
            ai_models,
            &active_preset,
            spotlight_account_ids,
        )?
    };

    // Phase 1.5: Pre-compute user context block (I413 semantic search).
    // Fresh DB connection so the Phase 1 lock is already released.
    if let Ok(db_ctx) = crate::db::ActionDb::open() {
        let mut ctx_buf = String::new();
        crate::reports::prompts::append_user_context(
            &mut ctx_buf,
            &db_ctx,
            Some(state.embedding_model.as_ref()),
            "Book of Business",
        );
        gather.user_context_block = ctx_buf;
    }

    // Step 2: Glean pre-fetch (no DB lock)
    let glean_ctx = {
        let ctx_arc = state.context_provider();
        if ctx_arc.is_remote() {
            if let Some(endpoint) = ctx_arc.remote_endpoint() {
                let account_names: Vec<String> = gather
                    .snapshot
                    .iter()
                    .take(20) // Top 20 by ARR (already sorted)
                    .map(|s| s.account_name.clone())
                    .collect();
                log::info!(
                    "[I547] Glean connected — pre-fetching portfolio context for {} accounts",
                    account_names.len()
                );
                let ctx = prefetch_glean_portfolio_context(endpoint, &account_names);

                // Emit glean phase completion event
                if let Some(handle) = app_handle {
                    let _ = handle.emit(
                        "bob-section-progress",
                        BobSectionProgress {
                            section_name: "glean".to_string(),
                            completed: 0,
                            total: 7,
                            wave: 0,
                        },
                    );
                }
                ctx
            } else {
                GleanPortfolioContext::default()
            }
        } else {
            GleanPortfolioContext::default()
        }
    };

    // Phase 3: Mechanical builders + single synthesis call
    let ai_response = run_bob_generation(&gather, &glean_ctx, &gather.metrics, app_handle)?;

    // Phase 4: Assemble mechanical + synthesis into final content
    let health_overview = build_health_overview(&gather.snapshot);
    let risk_accounts = build_risk_accounts(&gather.snapshot, &gather.raw_accounts);
    let expansion_accounts = build_expansion_accounts(&gather.snapshot, &gather.raw_accounts);
    let year_end_outlook =
        build_year_end_outlook(gather.metrics.total_arr, gather.metrics.at_risk_arr);

    let content = assemble_book_content(
        ai_response,
        gather.metrics.clone(),
        health_overview,
        risk_accounts,
        expansion_accounts,
        year_end_outlook,
    );
    let content_json = serde_json::to_string(&content)
        .map_err(|e| format!("Failed to serialize BoB content: {}", e))?;

    // Audit trail
    let _ = crate::audit::write_audit_entry(
        &gather.workspace,
        "report_book_of_business",
        &gather.user_entity_id,
        &content_json,
    );

    let db = crate::db::ActionDb::open().map_err(|e| format!("DB open failed: {e}"))?;

    let report_id = upsert_report(
        &db,
        &gather.user_entity_id,
        "user",
        "book_of_business",
        &content_json,
        &gather.intel_hash,
    )?;

    get_report(&db, &gather.user_entity_id, "user", "book_of_business")?
        .ok_or_else(|| format!("Report {} not found after insert", report_id))
}

/// Read a cached report (fast, no AI). Returns None if not found.
pub fn get_report_cached(
    db: &crate::db::ActionDb,
    entity_id: &str,
    entity_type: &str,
    report_type: &str,
) -> Result<Option<ReportRow>, String> {
    get_report(db, entity_id, entity_type, report_type)
}

/// Fetch all reports for an entity.
pub fn get_all_reports_for_entity(
    db: &crate::db::ActionDb,
    entity_id: &str,
    entity_type: &str,
) -> Result<Vec<ReportRow>, String> {
    get_reports_for_entity(db, entity_id, entity_type)
}

/// Auto-generate monthly wrapped on 1st of month if not already done this month (I419).
pub async fn generate_monthly_wrapped_if_needed(
    state: &std::sync::Arc<crate::state::AppState>,
) -> Result<(), String> {
    use crate::reports::monthly_wrapped::prior_calendar_month;

    let (month_start, _) = prior_calendar_month();
    let intel_hash_key = format!("month-{}", month_start.format("%Y-%m"));

    let already_exists = {
        let db = crate::db::ActionDb::open().map_err(|e| format!("DB open failed: {e}"))?;

        let user_id: String = db
            .conn_ref()
            .query_row(
                "SELECT CAST(id AS TEXT) FROM user_entity LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap_or_else(|_| "1".to_string());

        crate::reports::get_report(&db, &user_id, "user", "monthly_wrapped")?
            .map(|r| r.intel_hash == intel_hash_key)
            .unwrap_or(false)
    };

    if already_exists {
        log::debug!(
            "Scheduler: monthly wrapped already generated for {}",
            intel_hash_key
        );
        return Ok(());
    }

    log::info!(
        "Scheduler: auto-generating monthly wrapped for {}",
        intel_hash_key
    );
    let ctx = state.live_service_context();
    generate_report(
        &ctx,
        state,
        "user",
        "user",
        "monthly_wrapped",
        None,
        None,
    )
    .await?;
    Ok(())
}

/// Save user edits to a report's content_json.
pub fn save_report(
    ctx: &crate::services::context::ServiceContext<'_>,
    db: &crate::db::ActionDb,
    entity_id: &str,
    entity_type: &str,
    report_type: &str,
    content_json: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    crate::reports::save_report_content(db, entity_id, entity_type, report_type, content_json)
}

/// Auto-generate weekly impact on Monday if not already done this week (I418).
pub async fn generate_weekly_impact_if_needed(
    state: &std::sync::Arc<crate::state::AppState>,
) -> Result<(), String> {
    use crate::reports::weekly_impact::prior_work_week;

    let (week_start, _) = prior_work_week();
    let intel_hash_key = format!("week-{}", week_start.format("%Y-%m-%d"));

    // Check if we already have a report for this week
    let already_exists = {
        let db = crate::db::ActionDb::open().map_err(|e| format!("DB open failed: {e}"))?;

        // Get user entity ID as string
        let user_id: String = db
            .conn_ref()
            .query_row(
                "SELECT CAST(id AS TEXT) FROM user_entity LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap_or_else(|_| "1".to_string());

        crate::reports::get_report(&db, &user_id, "user", "weekly_impact")?
            .map(|r| r.intel_hash == intel_hash_key)
            .unwrap_or(false)
    };

    if already_exists {
        log::debug!(
            "Scheduler: weekly impact already generated for {}",
            intel_hash_key
        );
        return Ok(());
    }

    log::info!(
        "Scheduler: auto-generating weekly impact for {}",
        intel_hash_key
    );
    let ctx = state.live_service_context();
    generate_report(&ctx, state, "user", "user", "weekly_impact", None, None).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::test_db;
    use crate::reports::upsert_report;
    use crate::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};
    use chrono::TimeZone;

    fn test_ctx<'a>(
        clock: &'a FixedClock,
        rng: &'a SeedableRng,
        ext: &'a ExternalClients,
    ) -> ServiceContext<'a> {
        ServiceContext::test_live(clock, rng, ext)
    }

    #[test]
    fn test_save_and_get_report() {
        let db = test_db();
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        // Save via service layer
        save_report(
            &ctx,
            &db,
            "acc-1",
            "account",
            "account_health",
            r#"{"score": 75}"#,
        )
        .expect("save_report should not fail on missing row (no-op update)");

        // Insert via upsert_report first, then retrieve
        upsert_report(
            &db,
            "acc-1",
            "account",
            "account_health",
            r#"{"score": 75}"#,
            "hash-1",
        )
        .expect("upsert_report");

        let cached = get_report_cached(&db, "acc-1", "account", "account_health")
            .expect("get_report_cached");
        assert!(
            cached.is_some(),
            "Report should be retrievable after upsert"
        );
        let report = cached.unwrap();
        assert_eq!(report.entity_id, "acc-1");
        assert_eq!(report.report_type, "account_health");
        assert_eq!(report.content_json, r#"{"score": 75}"#);
        assert_eq!(report.intel_hash, "hash-1");
        assert!(!report.is_stale);
    }

    #[test]
    fn test_get_all_reports_for_entity() {
        let db = test_db();

        upsert_report(
            &db,
            "acc-2",
            "account",
            "account_health",
            r#"{"score": 80}"#,
            "h1",
        )
        .expect("upsert health");
        upsert_report(
            &db,
            "acc-2",
            "account",
            "swot",
            r#"{"strengths": []}"#,
            "h2",
        )
        .expect("upsert swot");

        let reports = get_all_reports_for_entity(&db, "acc-2", "account")
            .expect("get_all_reports_for_entity");
        assert_eq!(reports.len(), 2, "Should have 2 reports");

        let types: Vec<&str> = reports.iter().map(|r| r.report_type.as_str()).collect();
        assert!(types.contains(&"account_health"));
        assert!(types.contains(&"swot"));
    }

    #[test]
    fn test_save_report_overwrites_existing() {
        let db = test_db();

        upsert_report(
            &db,
            "acc-3",
            "account",
            "account_health",
            r#"{"score": 60}"#,
            "h-old",
        )
        .expect("first upsert");
        upsert_report(
            &db,
            "acc-3",
            "account",
            "account_health",
            r#"{"score": 90}"#,
            "h-new",
        )
        .expect("second upsert");

        let report = get_report_cached(&db, "acc-3", "account", "account_health")
            .expect("get")
            .expect("should exist");
        assert_eq!(
            report.content_json, r#"{"score": 90}"#,
            "Content should be updated"
        );
        assert_eq!(report.intel_hash, "h-new", "Hash should be updated");
    }

    #[test]
    fn test_save_report_updates_content() {
        let db = test_db();
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        upsert_report(&db, "acc-4", "account", "swot", r#"{"old": true}"#, "h1")
            .expect("initial upsert");
        save_report(
            &ctx,
            &db,
            "acc-4",
            "account",
            "swot",
            r#"{"edited": true}"#,
        )
        .expect("save_report");

        let report = get_report_cached(&db, "acc-4", "account", "swot")
            .expect("get")
            .expect("should exist");
        assert_eq!(report.content_json, r#"{"edited": true}"#);
    }

    #[test]
    fn test_get_report_returns_none_for_missing() {
        let db = test_db();

        let result = get_report_cached(&db, "nonexistent", "account", "account_health")
            .expect("should not error");
        assert!(result.is_none(), "Should return None for missing report");
    }
}
