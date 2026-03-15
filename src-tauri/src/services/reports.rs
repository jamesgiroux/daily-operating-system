//! Report generation service — async dispatch with spawn_blocking.

use std::sync::Arc;

use crate::reports::generator::run_report_generation;
use crate::reports::swot::gather_swot_input;
use crate::reports::{get_report, get_reports_for_entity, upsert_report, ReportRow};
use crate::state::AppState;

/// Generate a report for an entity.
/// Blocks for ~60-300s (PTY call). Returns the stored report row.
pub async fn generate_report(
    state: &Arc<AppState>,
    entity_id: &str,
    entity_type: &str,
    report_type_str: &str,
    spotlight_account_ids: Option<&[String]>,
) -> Result<ReportRow, String> {
    let state = state.clone();
    let entity_id = entity_id.to_string();
    let entity_type = entity_type.to_string();
    let report_type_str = report_type_str.to_string();
    let spotlight_account_ids = spotlight_account_ids.map(|s| s.to_vec());

    let task = tauri::async_runtime::spawn_blocking(move || -> Result<ReportRow, String> {
        // Phase 1: Gather input under brief DB lock
        let mut input = {
            let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
            let db = db_guard.as_ref().ok_or("Database not initialized")?;

            let config_guard = state.config.read().map_err(|_| "Config lock poisoned")?;
            let config = config_guard.as_ref().ok_or("Config not initialized")?;
            let workspace = std::path::Path::new(&config.workspace_path);
            let ai_models = config.ai_models.clone();

            let ctx_arc = state.context_provider();
            let ctx_provider = ctx_arc.as_ref();

            match report_type_str.as_str() {
                "swot" => gather_swot_input(
                    workspace,
                    db,
                    &entity_id,
                    &entity_type,
                    ai_models,
                    ctx_provider,
                )?,
                "account_health" => {
                    let active_preset = config.role.clone();
                    crate::reports::account_health::gather_account_health_input(
                        workspace,
                        db,
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
                        db,
                        ai_models,
                        &active_preset,
                    )?
                }
                "monthly_wrapped" => {
                    let active_preset = config.role.clone();
                    crate::reports::monthly_wrapped::gather_monthly_wrapped_input(
                        workspace,
                        db,
                        ai_models,
                        &active_preset,
                    )?
                }
                "ebr_qbr" => {
                    let active_preset = config.role.clone();
                    crate::reports::ebr_qbr::gather_ebr_qbr_input(
                        workspace,
                        db,
                        &entity_id,
                        ai_models,
                        &active_preset,
                        ctx_provider,
                    )?
                }
                "book_of_business" => {
                    let active_preset = config.role.clone();
                    crate::reports::book_of_business::gather_book_of_business_input(
                        workspace,
                        db,
                        ai_models,
                        &active_preset,
                        spotlight_account_ids.as_deref(),
                    )?
                }
                _ => return Err(format!("Unknown report type: {}", report_type_str)),
            }
        };

        // Phase 1.5: Inject relevant user context into prompt (I413).
        // Opens a fresh DB connection so the Phase 1 lock is already released.
        // No-op if the embedding model is not ready or no entries exceed 0.70 similarity.
        if let Ok(db_ctx) = crate::db::ActionDb::open() {
            crate::reports::prompts::append_user_context(
                &mut input.prompt,
                &db_ctx,
                Some(state.embedding_model.as_ref()),
                &input.entity_name,
            );
        }

        // Phase 2: Run PTY (no DB lock held — this is the long-running operation)
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
            "book_of_business" => {
                let metrics: crate::reports::book_of_business::BookMetrics =
                    serde_json::from_str(
                        input.extra_data.as_deref().ok_or("Missing book metrics")?,
                    )
                    .map_err(|e| format!("Failed to parse BookMetrics: {}", e))?;
                let bob = crate::reports::book_of_business::parse_book_of_business_response(
                    &stdout, metrics,
                )?;
                serde_json::to_string(&bob)
                    .map_err(|e| format!("Failed to serialize Book of Business: {}", e))?
            }
            _ => return Err(format!("Unknown report type: {}", report_type_str)),
        };

        // Phase 3: Write to DB
        let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
        let db = db_guard.as_ref().ok_or("Database not initialized")?;

        let report_id = upsert_report(
            db,
            &input.entity_id,
            &input.entity_type,
            &input.report_type,
            &content_json,
            &input.intel_hash,
        )?;

        get_report(db, &input.entity_id, &input.entity_type, &input.report_type)?
            .ok_or_else(|| format!("Report {} not found after insert", report_id))
    });

    match task.await {
        Ok(result) => result,
        Err(e) => Err(format!("Report generation task panicked: {}", e)),
    }
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
        let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
        let db = db_guard.as_ref().ok_or("DB not initialized")?;

        let user_id: String = db
            .conn_ref()
            .query_row(
                "SELECT CAST(id AS TEXT) FROM user_entity LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap_or_else(|_| "1".to_string());

        crate::reports::get_report(db, &user_id, "user", "monthly_wrapped")?
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
    generate_report(state, "user", "user", "monthly_wrapped", None).await?;
    Ok(())
}

/// Save user edits to a report's content_json.
pub fn save_report(
    db: &crate::db::ActionDb,
    entity_id: &str,
    entity_type: &str,
    report_type: &str,
    content_json: &str,
) -> Result<(), String> {
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
        let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
        let db = db_guard.as_ref().ok_or("DB not initialized")?;

        // Get user entity ID as string
        let user_id: String = db
            .conn_ref()
            .query_row(
                "SELECT CAST(id AS TEXT) FROM user_entity LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap_or_else(|_| "1".to_string());

        crate::reports::get_report(db, &user_id, "user", "weekly_impact")?
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
    generate_report(state, "user", "user", "weekly_impact", None).await?;
    Ok(())
}
