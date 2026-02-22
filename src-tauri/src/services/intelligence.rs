// Intelligence service — extracted from commands.rs (I402)
// Business logic for entity intelligence CRUD, enrichment, and risk briefings.

use std::path::Path;

use crate::db::ActionDb;
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
    };

    // Manual refresh: clear circuit breaker so enrichment proceeds (I410)
    if let Ok(db_guard) = state.db.lock() {
        if let Some(db) = db_guard.as_ref() {
            crate::self_healing::scheduler::reset_circuit_breaker(db, &request.entity_id);
        }
    }

    let input = gather_enrichment_input(state, &request)?;

    let ai_config = state
        .config
        .read()
        .ok()
        .and_then(|g| g.as_ref().map(|c| c.ai_models.clone()))
        .unwrap_or_default();
    let intel = run_enrichment(&input, &ai_config)?;

    write_enrichment_results(state, &input, &intel)?;

    Ok(intel)
}

/// Update a single field in an entity's intelligence.json with signal emission.
pub fn update_intelligence_field(
    entity_id: &str,
    entity_type: &str,
    field_path: &str,
    value: &str,
    state: &AppState,
) -> Result<(), String> {
    let config = state.config.read().map_err(|_| "Lock poisoned")?;
    let config = config.as_ref().ok_or("No configuration loaded")?;
    let workspace = Path::new(&config.workspace_path);

    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let account = if entity_type == "account" {
        db.get_account(entity_id).map_err(|e| e.to_string())?
    } else {
        None
    };

    let entity_name = match entity_type {
        "account" => account.as_ref().map(|a| a.name.clone()),
        "project" => db
            .get_project(entity_id)
            .map_err(|e| e.to_string())?
            .map(|p| p.name),
        "person" => db
            .get_person(entity_id)
            .map_err(|e| e.to_string())?
            .map(|p| p.name),
        _ => return Err(format!("Unsupported entity type: {}", entity_type)),
    }
    .ok_or_else(|| format!("{} '{}' not found", entity_type, entity_id))?;

    let dir = crate::intelligence::resolve_entity_dir(
        workspace,
        entity_type,
        &entity_name,
        account.as_ref(),
    )?;

    let intel = crate::intelligence::apply_intelligence_field_update(&dir, field_path, value)?;

    let _ = db.upsert_entity_intelligence(&intel);

    let _ = crate::signals::bus::emit_signal(
        db,
        entity_type,
        entity_id,
        "user_correction",
        "user_edit",
        Some(&format!("{{\"field\":\"{}\"}}", field_path)),
        1.0,
    );

    // Self-healing: record user correction to lower quality score (I409)
    crate::self_healing::feedback::record_enrichment_correction(db, entity_id, entity_type, "intel_queue");

    Ok(())
}

/// Bulk-replace the stakeholder list in an entity's intelligence.json.
pub fn update_stakeholders(
    entity_id: &str,
    entity_type: &str,
    stakeholders: Vec<crate::intelligence::StakeholderInsight>,
    state: &AppState,
) -> Result<(), String> {
    let config = state.config.read().map_err(|_| "Lock poisoned")?;
    let config = config.as_ref().ok_or("No configuration loaded")?;
    let workspace = Path::new(&config.workspace_path);

    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let account = if entity_type == "account" {
        db.get_account(entity_id).map_err(|e| e.to_string())?
    } else {
        None
    };

    let entity_name = match entity_type {
        "account" => account.as_ref().map(|a| a.name.clone()),
        "project" => db
            .get_project(entity_id)
            .map_err(|e| e.to_string())?
            .map(|p| p.name),
        "person" => db
            .get_person(entity_id)
            .map_err(|e| e.to_string())?
            .map(|p| p.name),
        _ => return Err(format!("Unsupported entity type: {}", entity_type)),
    }
    .ok_or_else(|| format!("{} '{}' not found", entity_type, entity_id))?;

    let dir = crate::intelligence::resolve_entity_dir(
        workspace,
        entity_type,
        &entity_name,
        account.as_ref(),
    )?;

    let intel = crate::intelligence::apply_stakeholders_update(&dir, stakeholders)?;

    let _ = db.upsert_entity_intelligence(&intel);

    let _ = crate::signals::bus::emit_signal_and_propagate(
        db,
        &state.signal_engine,
        entity_type,
        entity_id,
        "stakeholders_updated",
        "user_edit",
        None,
        0.9,
    );

    Ok(())
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
            )?
        };

        crate::risk_briefing::run_risk_enrichment(&input)
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
