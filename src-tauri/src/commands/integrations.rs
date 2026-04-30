use super::*;

use crate::services::integrations::ClaudeDesktopConfigResult;

#[tauri::command]
pub fn get_claude_desktop_status() -> ClaudeDesktopConfigResult {
    crate::services::integrations::get_claude_desktop_status()
}

/// Configure Claude Desktop to use the DailyOS MCP server.
#[tauri::command]
pub fn configure_claude_desktop() -> ClaudeDesktopConfigResult {
    crate::services::integrations::configure_claude_desktop()
}

// =============================================================================
// Cowork Plugin Export
// =============================================================================

/// Result of a Cowork plugin export operation.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoworkPluginResult {
    pub success: bool,
    pub message: String,
    pub path: Option<String>,
}

/// Info about a bundled Cowork plugin.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoworkPluginInfo {
    pub name: String,
    pub description: String,
    pub filename: String,
    pub available: bool,
    pub exported: bool,
}

/// Export a bundled Cowork plugin zip to ~/Desktop.
#[tauri::command]
pub fn export_cowork_plugin(
    app_handle: tauri::AppHandle,
    plugin_name: String,
) -> CoworkPluginResult {
    let filename = match plugin_name.as_str() {
        "dailyos" => "dailyos-plugin.zip",
        "dailyos-writer" => "dailyos-writer-plugin.zip",
        _ => {
            return CoworkPluginResult {
                success: false,
                message: format!("Unknown plugin: {plugin_name}"),
                path: None,
            }
        }
    };

    let resource_path = app_handle
        .path()
        .resource_dir()
        .ok()
        .map(|d| d.join("resources/plugins").join(filename));

    // In dev mode, fall back to the source tree
    let source_path = resource_path.filter(|p| p.exists()).or_else(|| {
        let dev_path = std::env::current_dir()
            .ok()?
            .join("resources/plugins")
            .join(filename);
        dev_path.exists().then_some(dev_path)
    });

    let source = match source_path {
        Some(p) => p,
        None => {
            return CoworkPluginResult {
                success: false,
                message: format!("Bundled plugin not found: {filename}"),
                path: None,
            }
        }
    };

    let desktop = match dirs::home_dir() {
        Some(h) => h.join("Desktop").join(filename),
        None => {
            return CoworkPluginResult {
                success: false,
                message: "Could not determine home directory".to_string(),
                path: None,
            }
        }
    };

    match std::fs::copy(&source, &desktop) {
        Ok(_) => CoworkPluginResult {
            success: true,
            message: format!("Saved to Desktop/{filename}"),
            path: Some(desktop.to_string_lossy().to_string()),
        },
        Err(e) => CoworkPluginResult {
            success: false,
            message: format!("Failed to copy: {e}"),
            path: None,
        },
    }
}

/// List available bundled Cowork plugins and their export status.
#[tauri::command]
pub fn get_cowork_plugins_status(app_handle: tauri::AppHandle) -> Vec<CoworkPluginInfo> {
    let plugins = vec![
        (
            "dailyos",
            "dailyos-plugin.zip",
            "DailyOS workspace tools — briefings, accounts, meetings, actions",
        ),
        (
            "dailyos-writer",
            "dailyos-writer-plugin.zip",
            "DailyOS Writer — drafts emails, agendas, and follow-ups from your data",
        ),
    ];

    let desktop = dirs::home_dir().map(|h| h.join("Desktop"));

    let resource_dir = app_handle.path().resource_dir().ok();

    plugins
        .into_iter()
        .map(|(name, filename, description)| {
            let available = resource_dir
                .as_ref()
                .map(|d: &std::path::PathBuf| d.join("resources/plugins").join(filename).exists())
                .unwrap_or(false)
                || std::env::current_dir()
                    .ok()
                    .map(|d: std::path::PathBuf| {
                        d.join("resources/plugins").join(filename).exists()
                    })
                    .unwrap_or(false);

            let exported = desktop
                .as_ref()
                .map(|d| d.join(filename).exists())
                .unwrap_or(false);

            CoworkPluginInfo {
                name: name.to_string(),
                description: description.to_string(),
                filename: filename.to_string(),
                available,
                exported,
            }
        })
        .collect()
}

// =============================================================================
// Intelligence Field Editing (I261)
// =============================================================================

/// Update a single field in an entity's intelligence.json.
///
/// Reads the file, applies the update via JSON path navigation, records a
/// UserEdit entry (protecting the field from AI overwrite), and writes back
/// to filesystem + SQLite cache.
#[tauri::command]
pub async fn update_intelligence_field(
    entity_id: String,
    entity_type: String,
    field_path: String,
    value: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    crate::services::intelligence::update_intelligence_field(
        &entity_id,
        &entity_type,
        &field_path,
        &value,
        &state,
    )
    .await
}

/// I576: Dismiss an intelligence item, creating a tombstone to prevent re-creation.
#[tauri::command]
pub async fn dismiss_intelligence_item(
    entity_id: String,
    entity_type: String,
    field: String,
    item_text: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    crate::services::intelligence::dismiss_intelligence_item(
        &entity_id,
        &entity_type,
        &field,
        &item_text,
        &state,
    )
    .await
}

/// DOS-13: Track (accept) a recommended action — creates a real action.
#[tauri::command]
pub async fn track_recommendation(
    entity_id: String,
    entity_type: String,
    index: usize,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    crate::services::intelligence::track_recommendation(&entity_id, &entity_type, index, &state)
        .await
}

/// DOS-13: Dismiss a recommended action — removes it from intelligence.
#[tauri::command]
pub async fn dismiss_recommendation(
    entity_id: String,
    entity_type: String,
    index: usize,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    crate::services::intelligence::dismiss_recommendation(&entity_id, &entity_type, index, &state)
        .await
}

/// DOS-13 / Wave 0e: Mark an open commitment as done. Promotes the
/// commitment into value-delivered and emits `commitment_completed`.
#[tauri::command]
pub async fn mark_commitment_done(
    entity_id: String,
    entity_type: String,
    index: usize,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    crate::services::intelligence::mark_commitment_done(&entity_id, &entity_type, index, &state)
        .await
}

/// Bulk-replace the stakeholder list in an entity's intelligence.json.
#[tauri::command]
pub async fn update_stakeholders(
    entity_id: String,
    entity_type: String,
    stakeholders_json: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let stakeholders: Vec<crate::intelligence::StakeholderInsight> =
        serde_json::from_str(&stakeholders_json)
            .map_err(|e| format!("Invalid stakeholders JSON: {}", e))?;
    crate::services::intelligence::update_stakeholders(
        &entity_id,
        &entity_type,
        stakeholders,
        &state,
    )
    .await
}

/// Create a person entity from a stakeholder name (no email required).
///
/// Used when a stakeholder card references someone who doesn't yet exist as
/// a person entity. Creates with empty email, links to the parent entity.
#[tauri::command]
pub async fn create_person_from_stakeholder(
    entity_id: String,
    entity_type: String,
    name: String,
    role: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let app_state = state.inner().clone();
    let state_for_ctx = app_state.clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            crate::services::people::create_person_from_stakeholder(
                &ctx,
                db,
                &app_state,
                &entity_id,
                &entity_type,
                &name,
                role.as_deref(),
            )
        })
        .await
}

// =============================================================================
// Quill MCP Integration
// =============================================================================

/// Quill integration status for the frontend.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuillStatus {
    pub enabled: bool,
    pub bridge_exists: bool,
    pub bridge_path: String,
    pub pending_syncs: usize,
    pub failed_syncs: usize,
    pub completed_syncs: usize,
    pub last_sync_at: Option<String>,
    pub last_error: Option<String>,
    pub last_error_at: Option<String>,
    pub abandoned_syncs: usize,
    pub poll_interval_minutes: u32,
}

/// Get the current status of the Quill integration.
#[tauri::command]
pub async fn get_quill_status(state: State<'_, Arc<AppState>>) -> Result<QuillStatus, String> {
    let config = state.config.read().as_ref().map(|c| c.quill.clone());

    let quill_config = config.unwrap_or_default();
    let bridge_exists = std::path::Path::new(&quill_config.bridge_path).exists();

    // Count sync states from DB without blocking the main thread on the
    // legacy sync mutex (can beachball during wake/unlock contention).
    let (pending, failed, completed, last_sync, last_error, last_error_at, abandoned) = state
        .db_read(|db| {
            let pending = db.get_pending_quill_syncs().map(|v| v.len()).unwrap_or(0);

            // Count failed, completed, abandoned from all rows
            let (failed_count, completed_count, last, abandoned_count) = db
                .conn_ref()
                .prepare(
                    "SELECT
                        SUM(CASE WHEN state = 'failed' THEN 1 ELSE 0 END),
                        SUM(CASE WHEN state = 'completed' THEN 1 ELSE 0 END),
                        MAX(completed_at),
                        SUM(CASE WHEN state = 'abandoned' THEN 1 ELSE 0 END)
                     FROM quill_sync_state",
                )
                .and_then(|mut stmt| {
                    stmt.query_row([], |row| {
                        Ok((
                            row.get::<_, i64>(0).unwrap_or(0) as usize,
                            row.get::<_, i64>(1).unwrap_or(0) as usize,
                            row.get::<_, Option<String>>(2)?,
                            row.get::<_, i64>(3).unwrap_or(0) as usize,
                        ))
                    })
                })
                .unwrap_or((0, 0, None, 0));

            // Get last error from failed/abandoned syncs
            let (err_msg, err_at) = db
                .conn_ref()
                .prepare(
                    "SELECT error_message, updated_at FROM quill_sync_state
                     WHERE state IN ('failed', 'abandoned') AND error_message IS NOT NULL
                     ORDER BY updated_at DESC LIMIT 1",
                )
                .and_then(|mut stmt| {
                    stmt.query_row([], |row| {
                        Ok((
                            row.get::<_, Option<String>>(0)?,
                            row.get::<_, Option<String>>(1)?,
                        ))
                    })
                })
                .unwrap_or((None, None));

            Ok((
                pending,
                failed_count,
                completed_count,
                last,
                err_msg,
                err_at,
                abandoned_count,
            ))
        })
        .await
        .unwrap_or((0, 0, 0, None, None, None, 0));

    Ok(QuillStatus {
        enabled: quill_config.enabled,
        bridge_exists,
        bridge_path: quill_config.bridge_path,
        pending_syncs: pending,
        failed_syncs: failed,
        completed_syncs: completed,
        last_sync_at: last_sync,
        last_error,
        last_error_at,
        abandoned_syncs: abandoned,
        poll_interval_minutes: quill_config.poll_interval_minutes,
    })
}

/// Enable or disable Quill integration.
#[tauri::command]
pub fn set_quill_enabled(enabled: bool, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    crate::state::create_or_update_config(&state, |config| {
        config.quill.enabled = enabled;
    })?;
    Ok(())
}

/// Result of a Quill historical backfill operation.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuillBackfillResult {
    pub created: usize,
    pub eligible: usize,
}

/// Create Quill sync rows for past meetings that never had transcript sync.
#[tauri::command]
pub async fn start_quill_backfill(
    days_back: Option<u32>,
    state: State<'_, Arc<AppState>>,
) -> Result<QuillBackfillResult, String> {
    let days_back = days_back.unwrap_or(365);
    if !(1..=3650).contains(&days_back) {
        return Err("daysBack must be between 1 and 3650".to_string());
    }
    let days_back_i32 = days_back as i32;

    state
        .db_write(move |db| {
            let ids = db
                .get_backfill_eligible_meeting_ids(days_back_i32)
                .map_err(|e| e.to_string())?;
            let eligible = ids.len();
            let mut created = 0;
            for id in &ids {
                if crate::quill::sync::create_sync_for_meeting(db, id).is_ok() {
                    created += 1;
                }
            }
            Ok(QuillBackfillResult { created, eligible })
        })
        .await
}

/// Set the Quill poll interval (1–60 minutes).
#[tauri::command]
pub fn set_quill_poll_interval(
    minutes: u32,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    if !(1..=60).contains(&minutes) {
        return Err("Poll interval must be between 1 and 60 minutes".to_string());
    }
    crate::state::create_or_update_config(&state, |config| {
        config.quill.poll_interval_minutes = minutes;
    })?;
    Ok(())
}

/// Test the Quill MCP connection by spawning the bridge and verifying connectivity.
#[tauri::command]
pub async fn test_quill_connection(state: State<'_, Arc<AppState>>) -> Result<bool, String> {
    let bridge_path = state
        .config
        .read()
        .as_ref()
        .map(|c| c.quill.bridge_path.clone())
        .unwrap_or_default();

    if bridge_path.is_empty() {
        return Ok(false);
    }

    let client = crate::quill::client::QuillClient::connect(&bridge_path)
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;

    client.disconnect().await;
    Ok(true)
}

/// Trigger Quill transcript sync for a single meeting.
/// Creates a sync row if none exists, or resets a failed/stale one to pending.
#[tauri::command]
pub async fn trigger_quill_sync_for_meeting(
    meeting_id: String,
    force: Option<bool>,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let force = force.unwrap_or(false);
    state
        .db_write(move |db| {
            // Check if a sync row already exists
            match db
                .get_quill_sync_state_by_source(&meeting_id, "quill")
                .map_err(|e| e.to_string())?
            {
                Some(existing) => {
                    match existing.state.as_str() {
                        "completed" if !force => Ok("already_completed".to_string()),
                        "completed" => {
                            // Force re-sync: reset to pending so poller picks it up again.
                            // This handles the case where captures were lost due to a bug
                            // or when the user wants to re-process with updated AI.
                            crate::quill::sync::transition_state(
                                db,
                                &existing.id,
                                "pending",
                                None,
                                None,
                                None,
                                Some("Force re-sync"),
                            )
                            .map_err(|e| e.to_string())?;
                            Ok("resyncing".to_string())
                        }
                        "pending" | "polling" | "fetching" | "processing" if force => {
                            // Force-reset a stuck in-progress state back to pending.
                            // Covers the case where the app crashed or the AI pipeline
                            // failed silently mid-processing, leaving the row orphaned.
                            crate::quill::sync::transition_state(
                                db,
                                &existing.id,
                                "pending",
                                None,
                                None,
                                None,
                                Some("Force reset from stuck state"),
                            )
                            .map_err(|e| e.to_string())?;
                            Ok("resyncing".to_string())
                        }
                        "pending" | "polling" | "fetching" | "processing" => {
                            Ok("already_in_progress".to_string())
                        }
                        _ => {
                            // Failed or abandoned — reset to pending for retry
                            crate::quill::sync::transition_state(
                                db,
                                &existing.id,
                                "pending",
                                None,
                                None,
                                None,
                                Some("Manual retry"),
                            )
                            .map_err(|e| e.to_string())?;
                            Ok("retrying".to_string())
                        }
                    }
                }
                None => {
                    crate::quill::sync::create_sync_for_meeting(db, &meeting_id)
                        .map_err(|e| e.to_string())?;
                    Ok("created".to_string())
                }
            }
        })
        .await
}

/// Get Quill sync states, optionally filtered by meeting ID.
#[tauri::command]
pub async fn get_quill_sync_states(
    meeting_id: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::db::DbQuillSyncState>, String> {
    state
        .db_read(move |db| match meeting_id {
            Some(ref mid) => {
                let row = db
                    .get_quill_sync_state_by_source(mid, "quill")
                    .map_err(|e| e.to_string())?;
                Ok(row.into_iter().collect())
            }
            None => db.get_pending_quill_syncs().map_err(|e| e.to_string()),
        })
        .await
}

// =============================================================================
// Granola Integration (I226)
// =============================================================================

/// Granola integration status for the frontend.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GranolaStatus {
    pub enabled: bool,
    pub cache_exists: bool,
    pub cache_path: String,
    pub document_count: usize,
    pub pending_syncs: usize,
    pub failed_syncs: usize,
    pub completed_syncs: usize,
    pub last_sync_at: Option<String>,
    pub poll_interval_minutes: u32,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GranolaManualSyncResponse {
    pub status: String,
    pub message: String,
    pub document_title: Option<String>,
    pub content_type: Option<String>,
}

/// Get the current status of the Granola integration.
#[tauri::command]
pub async fn get_granola_status(state: State<'_, Arc<AppState>>) -> Result<GranolaStatus, String> {
    let config = state.config.read().as_ref().map(|c| c.granola.clone());

    let granola_config = config.unwrap_or_default();
    let resolved_path = crate::granola::resolve_cache_path(&granola_config);
    let cache_exists = resolved_path.is_some();

    let document_count = match &resolved_path {
        Some(p) => crate::granola::cache::count_documents(p).unwrap_or(0),
        None => 0,
    };

    // Count sync states from DB (source='granola')
    let (pending, failed, completed, last_sync) = state
        .db_read(|db| {
            let (failed_count, completed_count, last, pending_count) = db
                .conn_ref()
                .prepare(
                    "SELECT
                    SUM(CASE WHEN state = 'failed' THEN 1 ELSE 0 END),
                    SUM(CASE WHEN state = 'completed' THEN 1 ELSE 0 END),
                    MAX(completed_at),
                    SUM(CASE WHEN state IN ('pending', 'polling', 'processing') THEN 1 ELSE 0 END)
                 FROM quill_sync_state WHERE source = 'granola'",
                )
                .and_then(|mut stmt| {
                    stmt.query_row([], |row| {
                        Ok((
                            row.get::<_, i64>(0).unwrap_or(0) as usize,
                            row.get::<_, i64>(1).unwrap_or(0) as usize,
                            row.get::<_, Option<String>>(2)?,
                            row.get::<_, i64>(3).unwrap_or(0) as usize,
                        ))
                    })
                })
                .unwrap_or((0, 0, None, 0));
            Ok((pending_count, failed_count, completed_count, last))
        })
        .await
        .unwrap_or((0, 0, 0, None));

    Ok(GranolaStatus {
        enabled: granola_config.enabled,
        cache_exists,
        cache_path: resolved_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default(),
        document_count,
        pending_syncs: pending,
        failed_syncs: failed,
        completed_syncs: completed,
        last_sync_at: last_sync,
        poll_interval_minutes: granola_config.poll_interval_minutes,
    })
}

/// Attempt an immediate Granola sync for a single meeting.
#[tauri::command]
pub async fn trigger_granola_sync_for_meeting(
    meeting_id: String,
    force: Option<bool>,
    state: State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<GranolaManualSyncResponse, String> {
    let force = force.unwrap_or(false);
    let state = state.inner().clone();
    let app_handle = app_handle.clone();
    let state_for_blocking = state.clone();

    let (result, attached_event) = tauri::async_runtime::spawn_blocking(move || {
        crate::granola::poller::trigger_granola_sync_for_meeting(
            &state_for_blocking,
            &app_handle,
            &meeting_id,
            force,
        )
    })
    .await
    .map_err(|e| format!("Granola sync task failed: {}", e))??;

    // Re-run entity linking with the post-transcript context (DOS-258).
    // Best-effort — failure here never blocks the manual-sync response.
    if let Some(event) = attached_event {
        if let Err(e) = crate::services::entity_linking::calendar_adapter::evaluate_meeting(
            state,
            &event,
            crate::services::entity_linking::Trigger::TranscriptIngest,
        )
        .await
        {
            log::warn!(
                "entity_linking after manual Granola sync failed (non-fatal) for {}: {}",
                event.id,
                e
            );
        }
    }

    let status = match result.status {
        crate::granola::poller::ManualGranolaSyncStatus::Attached => "attached",
        crate::granola::poller::ManualGranolaSyncStatus::NotFound => "not_found",
        crate::granola::poller::ManualGranolaSyncStatus::AlreadyInProgress => "already_in_progress",
        crate::granola::poller::ManualGranolaSyncStatus::AlreadyCompleted => "already_completed",
    };

    let content_type = result.content_type.map(|kind| match kind {
        crate::granola::cache::GranolaContentType::Transcript => "transcript".to_string(),
        crate::granola::cache::GranolaContentType::Notes => "notes".to_string(),
    });

    Ok(GranolaManualSyncResponse {
        status: status.to_string(),
        message: result.message,
        document_title: result.document_title,
        content_type,
    })
}

/// Enable or disable Granola integration.
#[tauri::command]
pub fn set_granola_enabled(enabled: bool, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    crate::state::create_or_update_config(&state, |config| {
        config.granola.enabled = enabled;
    })?;
    Ok(())
}

/// Set the Granola poll interval (1–60 minutes).
#[tauri::command]
pub fn set_granola_poll_interval(
    minutes: u32,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    if !(1..=60).contains(&minutes) {
        return Err("Poll interval must be between 1 and 60 minutes".to_string());
    }
    crate::state::create_or_update_config(&state, |config| {
        config.granola.poll_interval_minutes = minutes;
    })?;
    Ok(())
}

/// Result of a Granola backfill operation.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GranolaBackfillResult {
    pub created: usize,
    pub eligible: usize,
}

/// Create Granola sync rows for past meetings found in the cache.
#[tauri::command]
pub fn start_granola_backfill(
    days_back: Option<u32>,
    state: State<'_, Arc<AppState>>,
) -> Result<GranolaBackfillResult, String> {
    let days_back = days_back.unwrap_or(365);
    if !(1..=3650).contains(&days_back) {
        return Err("daysBack must be between 1 and 3650".to_string());
    }
    let (created, eligible) =
        crate::granola::poller::run_granola_backfill(&state, days_back as i32)?;
    Ok(GranolaBackfillResult { created, eligible })
}

/// Test whether the Granola cache file exists and is valid.
#[tauri::command]
pub fn test_granola_cache(state: State<'_, Arc<AppState>>) -> Result<usize, String> {
    let granola_config = state
        .config
        .read()
        .as_ref()
        .map(|c| c.granola.clone())
        .unwrap_or_default();

    let path = crate::granola::resolve_cache_path(&granola_config)
        .ok_or("Granola cache file not found")?;

    crate::granola::cache::count_documents(&path)
}

// ═══════════════════════════════════════════════════════════════════════════
// I229: Gravatar MCP Integration
// ═══════════════════════════════════════════════════════════════════════════

/// Gravatar integration status for the settings UI.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GravatarStatus {
    pub enabled: bool,
    pub cached_count: i64,
    pub api_key_set: bool,
}

/// Get Gravatar integration status.
#[tauri::command]
pub fn get_gravatar_status(state: State<'_, Arc<AppState>>) -> GravatarStatus {
    let config = state.config.read().as_ref().map(|c| c.gravatar.clone());

    let gravatar_config = config.unwrap_or_default();

    let cached_count = crate::db::ActionDb::open()
        .ok()
        .map(|db| crate::gravatar::cache::count_cached(db.conn_ref()))
        .unwrap_or(0);

    GravatarStatus {
        enabled: gravatar_config.enabled,
        cached_count,
        api_key_set: crate::gravatar::keychain::get_gravatar_api_key().is_some(),
    }
}

/// Enable or disable Gravatar integration.
#[tauri::command]
pub fn set_gravatar_enabled(enabled: bool, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    crate::state::create_or_update_config(&state, |config| {
        config.gravatar.enabled = enabled;
    })?;
    Ok(())
}

/// Set or clear the Gravatar API key (stored in macOS Keychain).
#[tauri::command]
pub fn set_gravatar_api_key(
    key: Option<String>,
    _state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    match key.filter(|k| !k.is_empty()) {
        Some(k) => crate::gravatar::keychain::save_gravatar_api_key(&k),
        None => crate::gravatar::keychain::delete_gravatar_api_key(),
    }
}

/// Fetch Gravatar data for a single person on demand.
#[tauri::command]
pub async fn fetch_gravatar(
    person_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    // Look up person's email
    let pid = person_id.clone();
    let email = state
        .db_read(move |db| {
            db.conn_ref()
            .query_row(
                "SELECT email FROM person_emails WHERE person_id = ?1 AND is_primary = 1 LIMIT 1",
                [&pid],
                |row| row.get::<_, String>(0),
            )
            .map_err(|_| format!("No email found for person {}", pid))
        })
        .await?;

    let api_key = crate::gravatar::keychain::get_gravatar_api_key();

    // Connect and fetch
    let client = crate::gravatar::client::GravatarClient::connect(api_key.as_deref())
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;

    let profile = client.get_profile(&email).await.unwrap_or_default();

    let data_dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".dailyos")
        .join("avatars");
    let _ = std::fs::create_dir_all(&data_dir);

    let avatar_path = match client.get_avatar(&email, 200).await {
        Ok(Some(bytes)) => {
            use sha2::{Digest, Sha256};
            let hash = Sha256::digest(email.as_bytes());
            let hash_hex = hex::encode(&hash[..8]);
            let path = data_dir.join(format!("{}.png", hash_hex));
            if std::fs::write(&path, &bytes).is_ok() {
                Some(path.to_string_lossy().to_string())
            } else {
                None
            }
        }
        _ => None,
    };

    let interests = client.get_interests(&email).await.unwrap_or_default();

    client.disconnect().await;

    // Cache result
    let has_gravatar = profile.display_name.is_some() || avatar_path.is_some();
    let cache_entry = crate::gravatar::cache::CachedGravatar {
        email: email.clone(),
        avatar_url: avatar_path,
        display_name: profile.display_name,
        bio: profile.bio,
        location: profile.location,
        company: profile.company,
        job_title: profile.job_title,
        interests_json: if interests.is_empty() {
            None
        } else {
            serde_json::to_string(&interests).ok()
        },
        has_gravatar,
        fetched_at: chrono::Utc::now().to_rfc3339(),
        person_id: Some(person_id),
    };

    state
        .db_write(move |db| crate::gravatar::cache::upsert_cache(db.conn_ref(), &cache_entry))
        .await?;

    Ok(())
}

/// Batch fetch Gravatar data for all people with stale or missing cache.
#[tauri::command]
pub async fn bulk_fetch_gravatars(state: State<'_, Arc<AppState>>) -> Result<usize, String> {
    let api_key = crate::gravatar::keychain::get_gravatar_api_key();

    let emails_to_fetch: Vec<(String, Option<String>)> = state
        .db_read(|db| crate::gravatar::cache::get_stale_emails(db.conn_ref(), 100))
        .await?;

    if emails_to_fetch.is_empty() {
        return Ok(0);
    }

    let client = crate::gravatar::client::GravatarClient::connect(api_key.as_deref())
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;

    let data_dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".dailyos")
        .join("avatars");
    let _ = std::fs::create_dir_all(&data_dir);

    let mut fetched = 0;
    for (email, person_id) in &emails_to_fetch {
        let profile = client.get_profile(email).await.unwrap_or_default();

        let avatar_path = match client.get_avatar(email, 200).await {
            Ok(Some(bytes)) => {
                use sha2::{Digest, Sha256};
                let hash = Sha256::digest(email.as_bytes());
                let hash_hex = hex::encode(&hash[..8]);
                let path = data_dir.join(format!("{}.png", hash_hex));
                if std::fs::write(&path, &bytes).is_ok() {
                    Some(path.to_string_lossy().to_string())
                } else {
                    None
                }
            }
            _ => None,
        };

        let interests = client.get_interests(email).await.unwrap_or_default();

        let has_gravatar = profile.display_name.is_some() || avatar_path.is_some();
        let cache_entry = crate::gravatar::cache::CachedGravatar {
            email: email.clone(),
            avatar_url: avatar_path,
            display_name: profile.display_name,
            bio: profile.bio,
            location: profile.location,
            company: profile.company,
            job_title: profile.job_title,
            interests_json: if interests.is_empty() {
                None
            } else {
                serde_json::to_string(&interests).ok()
            },
            has_gravatar,
            fetched_at: chrono::Utc::now().to_rfc3339(),
            person_id: person_id.clone(),
        };

        let _ = state
            .db_write(move |db| crate::gravatar::cache::upsert_cache(db.conn_ref(), &cache_entry))
            .await;

        fetched += 1;
        // Rate limit: 1 req/sec
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }

    client.disconnect().await;
    Ok(fetched)
}

/// Get avatar for a person as a data URL (base64-encoded PNG).
/// Returns None if no cached avatar exists.
#[tauri::command]
pub async fn get_person_avatar(
    person_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Option<String>, String> {
    let path = match state
        .db_read(move |db| {
            Ok(crate::gravatar::cache::get_avatar_url_for_person(
                db.conn_ref(),
                &person_id,
            ))
        })
        .await
    {
        Ok(Some(p)) => p,
        _ => return Ok(None),
    };
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(_) => return Ok(None),
    };
    let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes);
    Ok(Some(format!("data:image/png;base64,{}", b64)))
}

// ═══════════════════════════════════════════════════════════════════════════
// I228: Clay Contact & Company Enrichment
// ═══════════════════════════════════════════════════════════════════════════

/// Clay integration status for the settings UI.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClayStatusData {
    pub enabled: bool,
    pub api_key_set: bool,
    pub auto_enrich_on_create: bool,
    pub sweep_interval_hours: u32,
    pub enriched_count: i64,
    pub pending_count: i64,
    pub last_enrichment_at: Option<String>,
}

/// Get Clay integration status.
#[tauri::command]
pub async fn get_clay_status(state: State<'_, Arc<AppState>>) -> Result<ClayStatusData, String> {
    let config = state.config.read().as_ref().map(|c| c.clay.clone());

    let clay_config = config.unwrap_or_default();

    let (enriched_count, pending_count, last_enrichment) = state
        .db_read(|db| {
            let enriched: i64 = db
                .conn_ref()
                .query_row(
                    "SELECT COUNT(*) FROM people WHERE last_enriched_at IS NOT NULL",
                    [],
                    |row| row.get(0),
                )
                .unwrap_or(0);
            let pending: i64 = db
                .conn_ref()
                .query_row(
                    "SELECT COUNT(*) FROM clay_sync_state WHERE state = 'pending'",
                    [],
                    |row| row.get(0),
                )
                .unwrap_or(0);
            let last: Option<String> = db
                .conn_ref()
                .query_row("SELECT MAX(last_enriched_at) FROM people", [], |row| {
                    row.get(0)
                })
                .unwrap_or(None);
            Ok((enriched, pending, last))
        })
        .await
        .unwrap_or((0, 0, None));

    Ok(ClayStatusData {
        enabled: clay_config.enabled,
        api_key_set: clay_config.api_key.is_some(),
        auto_enrich_on_create: clay_config.auto_enrich_on_create,
        sweep_interval_hours: clay_config.sweep_interval_hours,
        enriched_count,
        pending_count,
        last_enrichment_at: last_enrichment,
    })
}

/// Enable or disable Clay integration.
#[tauri::command]
pub fn set_clay_enabled(enabled: bool, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    crate::state::create_or_update_config(&state, |config| {
        config.clay.enabled = enabled;
    })?;
    Ok(())
}

/// Set or clear the Clay API key.
#[tauri::command]
pub fn set_clay_api_key(
    key: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    crate::state::create_or_update_config(&state, |config| {
        config.clay.api_key = key.filter(|k| !k.is_empty());
    })?;
    Ok(())
}

/// Toggle auto-enrich on person creation.
#[tauri::command]
pub fn set_clay_auto_enrich(enabled: bool, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    crate::state::create_or_update_config(&state, |config| {
        config.clay.auto_enrich_on_create = enabled;
    })?;
    Ok(())
}

/// Resolve Smithery credentials for Clay MCP: API key from keychain +
/// namespace and connection ID from config.
fn resolve_smithery_config(state: &AppState) -> Result<(String, String, String), String> {
    let api_key = crate::clay::oauth::get_smithery_api_key()
        .ok_or("No Smithery API key. Configure in Settings \u{2192} Connectors \u{2192} Clay.")?;
    let config = state.config.read().as_ref().map(|c| c.clay.clone());
    let clay = config.ok_or("Config not loaded")?;
    let ns = clay
        .smithery_namespace
        .ok_or("Smithery namespace not configured")?;
    let conn = clay
        .smithery_connection_id
        .ok_or("Smithery connection ID not configured")?;
    Ok((api_key, ns, conn))
}

/// Test Clay connection by attempting to connect via Smithery.
#[tauri::command]
pub async fn test_clay_connection(state: State<'_, Arc<AppState>>) -> Result<bool, String> {
    let (api_key, ns, conn) = resolve_smithery_config(&state)?;

    let client = crate::clay::client::ClayClient::connect(&api_key, &ns, &conn)
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;

    client.disconnect().await;
    Ok(true)
}

/// Enrichment result for the frontend.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnrichmentResultData {
    pub person_id: String,
    pub fields_updated: Vec<String>,
    pub signals: Vec<String>,
}

/// Enrich a single person from Clay on demand.
#[tauri::command]
pub async fn enrich_person_from_clay(
    person_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<EnrichmentResultData, String> {
    let (api_key, ns, conn) = resolve_smithery_config(&state)?;

    let client = crate::clay::client::ClayClient::connect(&api_key, &ns, &conn)
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;

    let result =
        crate::clay::enricher::enrich_person_from_clay_with_client(&state, &person_id, &client)
            .await?;

    client.disconnect().await;

    Ok(EnrichmentResultData {
        person_id: result.person_id,
        fields_updated: result.fields_updated,
        signals: result.signals,
    })
}

/// Enrich an account's company data from Clay (via linked people).
#[tauri::command]
pub async fn enrich_account_from_clay(
    account_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<EnrichmentResultData, String> {
    // Find a linked person for this account, enrich them, company data follows
    // I568: Use async db_read to avoid blocking Tokio threads.
    let acct_id = account_id.clone();
    let person_id: Option<String> = state
        .db_read(move |db| {
            Ok(db
                .conn_ref()
                .query_row(
                    "SELECT person_id FROM account_stakeholders WHERE account_id = ?1 LIMIT 1",
                    [&acct_id],
                    |row| row.get(0),
                )
                .ok())
        })
        .await
        .unwrap_or(None);

    let person_id = person_id.ok_or("No linked people found for this account")?;

    let (api_key, ns, conn) = resolve_smithery_config(&state)?;

    let client = crate::clay::client::ClayClient::connect(&api_key, &ns, &conn)
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;

    let result =
        crate::clay::enricher::enrich_person_from_clay_with_client(&state, &person_id, &client)
            .await?;

    client.disconnect().await;

    Ok(EnrichmentResultData {
        person_id: result.person_id,
        fields_updated: result.fields_updated,
        signals: result.signals,
    })
}

/// Bulk enrichment result.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkEnrichResult {
    pub queued: usize,
    pub total_unenriched: usize,
}

/// Start bulk Clay enrichment for all unenriched people.
#[tauri::command]
pub async fn start_clay_bulk_enrich(
    state: State<'_, Arc<AppState>>,
) -> Result<BulkEnrichResult, String> {
    let unenriched = state
        .db_read(|db| {
            let mut stmt = db
                .conn_ref()
                .prepare("SELECT id FROM people WHERE last_enriched_at IS NULL AND archived = 0")
                .map_err(|e| e.to_string())?;
            let unenriched: Vec<String> = stmt
                .query_map([], |row| row.get(0))
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .collect();

            Ok(unenriched)
        })
        .await?;
    let state_for_ctx = state.inner().clone();
    let total = state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            crate::services::mutations::queue_clay_sync_for_people(&ctx, db, &unenriched)
        })
        .await?;

    // Wake the enrichment processor immediately to process queued items
    state.integrations.enrichment_wake.notify_one();

    Ok(BulkEnrichResult {
        queued: total,
        total_unenriched: total,
    })
}

/// Enrichment log entry for the frontend.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnrichmentLogEntry {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub source: String,
    pub event_type: String,
    pub signal_type: Option<String>,
    pub fields_updated: Option<String>,
    pub created_at: String,
}

/// Get enrichment log entries for an entity.
#[tauri::command]
pub async fn get_enrichment_log(
    entity_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<EnrichmentLogEntry>, String> {
    state.db_read(move |db| {
        let mut stmt = db
            .conn_ref()
            .prepare(
                "SELECT id, entity_type, entity_id, source, event_type, signal_type, fields_updated, created_at
                 FROM enrichment_log
                 WHERE entity_id = ?1
                 ORDER BY created_at DESC
                 LIMIT 50",
            )
            .map_err(|e| e.to_string())?;

        let entries = stmt
            .query_map([&entity_id], |row| {
                Ok(EnrichmentLogEntry {
                    id: row.get(0)?,
                    entity_type: row.get(1)?,
                    entity_id: row.get(2)?,
                    source: row.get(3)?,
                    event_type: row.get(4)?,
                    signal_type: row.get(5)?,
                    fields_updated: row.get(6)?,
                    created_at: row.get(7)?,
                })
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        Ok(entries)
    }).await
}

// ---------------------------------------------------------------------------
// Clay — Smithery Connect (I422)
// ---------------------------------------------------------------------------

/// Auto-detect Smithery settings and Clay connection from CLI config + API.
#[tauri::command]
pub async fn detect_smithery_settings() -> Result<serde_json::Value, String> {
    let settings_path = dirs::home_dir()
        .ok_or("No home directory")?
        .join("Library/Application Support/smithery/settings.json");

    if !settings_path.exists() {
        return Err("Smithery CLI not configured. Run: npx @smithery/cli login".to_string());
    }

    let content = std::fs::read_to_string(&settings_path)
        .map_err(|e| format!("Failed to read Smithery settings: {}", e))?;

    let val: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse Smithery settings: {}", e))?;

    let api_key = val.get("apiKey").and_then(|v| v.as_str()).unwrap_or("");
    let namespace = val.get("namespace").and_then(|v| v.as_str()).unwrap_or("");

    if api_key.is_empty() || namespace.is_empty() {
        return Err("Smithery settings missing apiKey or namespace".to_string());
    }

    // List connections via Smithery API to find the Clay one
    let client = reqwest::Client::new();
    let connections_url = format!("https://api.smithery.ai/connect/{}", namespace);
    let clay_connection_id = match client
        .get(&connections_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            let body = resp.text().await.unwrap_or_default();
            let parsed: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
            // Find a connection whose name/mcpUrl contains "clay"
            parsed
                .get("connections")
                .and_then(|c| c.as_array())
                .and_then(|arr| {
                    arr.iter().find(|conn| {
                        let name = conn.get("name").and_then(|n| n.as_str()).unwrap_or("");
                        let url = conn.get("mcpUrl").and_then(|u| u.as_str()).unwrap_or("");
                        let status = conn
                            .get("status")
                            .and_then(|s| s.get("state"))
                            .and_then(|s| s.as_str())
                            .unwrap_or("");
                        (name.contains("clay") || url.contains("clay")) && status == "connected"
                    })
                })
                .and_then(|conn| conn.get("connectionId").and_then(|id| id.as_str()))
                .map(String::from)
        }
        _ => None,
    };

    Ok(serde_json::json!({
        "apiKey": api_key,
        "namespace": namespace,
        "connectionId": clay_connection_id,
    }))
}

/// Save Smithery API key to keychain.
#[tauri::command]
pub async fn save_smithery_api_key(key: String) -> Result<(), String> {
    let trimmed = key.trim().to_string();
    if trimmed.is_empty() {
        return Err("API key cannot be empty".to_string());
    }
    crate::clay::oauth::save_smithery_api_key(&trimmed)
}

/// Save Smithery connection config (namespace + connection ID).
#[tauri::command]
pub fn set_smithery_connection(
    namespace: String,
    connection_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let ns = if namespace.trim().is_empty() {
        None
    } else {
        Some(namespace)
    };
    let conn = if connection_id.trim().is_empty() {
        None
    } else {
        Some(connection_id)
    };
    crate::state::create_or_update_config(&state, |config| {
        config.clay.smithery_namespace = ns.clone();
        config.clay.smithery_connection_id = conn.clone();
    })?;
    Ok(())
}

/// Disconnect Smithery — remove keychain entry and clear config fields.
#[tauri::command]
pub fn disconnect_smithery(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    crate::clay::oauth::delete_smithery_api_key()?;
    crate::state::create_or_update_config(&state, |config| {
        config.clay.smithery_namespace = None;
        config.clay.smithery_connection_id = None;
    })?;

    let _ = state.with_db_write(|db| {
        crate::db::data_lifecycle::purge_source(db, crate::db::data_lifecycle::DataSource::Clay)
            .map_err(|e| e.to_string())
    })?;
    Ok(())
}

/// Get Smithery connection status.
#[tauri::command]
pub fn get_smithery_status(state: State<'_, Arc<AppState>>) -> serde_json::Value {
    let has_api_key = crate::clay::oauth::get_smithery_api_key().is_some();
    let (namespace, connection_id) = state
        .config
        .read()
        .as_ref()
        .map(|c| {
            (
                c.clay.smithery_namespace.clone(),
                c.clay.smithery_connection_id.clone(),
            )
        })
        .unwrap_or((None, None));

    let connected = has_api_key && namespace.is_some() && connection_id.is_some();

    serde_json::json!({
        "connected": connected,
        "hasApiKey": has_api_key,
        "namespace": namespace,
        "connectionId": connection_id,
    })
}

// =============================================================================
// I346: Linear Integration
// =============================================================================

/// Linear integration status for the frontend.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinearStatusData {
    pub enabled: bool,
    pub api_key_set: bool,
    pub poll_interval_minutes: u32,
    pub issue_count: i64,
    pub project_count: i64,
    pub last_sync_at: Option<String>,
}

/// Get Linear integration status.
#[tauri::command]
pub fn get_linear_status(state: State<'_, Arc<AppState>>) -> LinearStatusData {
    let config = state.config.read().as_ref().map(|c| c.linear.clone());

    let linear_config = config.unwrap_or_default();

    let (issue_count, project_count, last_sync) = crate::db::ActionDb::open()
        .ok()
        .map(|db| {
            let issues: i64 = db
                .conn_ref()
                .query_row("SELECT COUNT(*) FROM linear_issues", [], |row| row.get(0))
                .unwrap_or(0);
            let projects: i64 = db
                .conn_ref()
                .query_row("SELECT COUNT(*) FROM linear_projects", [], |row| row.get(0))
                .unwrap_or(0);
            let last: Option<String> = db
                .conn_ref()
                .query_row("SELECT MAX(synced_at) FROM linear_issues", [], |row| {
                    row.get(0)
                })
                .unwrap_or(None);
            (issues, projects, last)
        })
        .unwrap_or((0, 0, None));

    LinearStatusData {
        enabled: linear_config.enabled,
        api_key_set: linear_config.api_key.is_some(),
        poll_interval_minutes: linear_config.poll_interval_minutes,
        issue_count,
        project_count,
        last_sync_at: last_sync,
    }
}

/// Enable or disable Linear integration.
#[tauri::command]
pub fn set_linear_enabled(enabled: bool, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    crate::state::create_or_update_config(&state, |config| {
        config.linear.enabled = enabled;
    })?;
    Ok(())
}

/// Set or clear the Linear API key.
#[tauri::command]
pub fn set_linear_api_key(
    key: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    crate::state::create_or_update_config(&state, |config| {
        config.linear.api_key = key.filter(|k| !k.is_empty());
    })?;
    Ok(())
}

/// Test Linear connection by fetching the viewer.
#[tauri::command]
pub async fn test_linear_connection(state: State<'_, Arc<AppState>>) -> Result<String, String> {
    let api_key = state
        .config
        .read()
        .as_ref()
        .and_then(|c| c.linear.api_key.clone())
        .ok_or("No Linear API key configured")?;

    let client = crate::linear::client::LinearClient::new(&api_key);
    let viewer = client.test_connection().await?;
    Ok(viewer.name)
}

/// Trigger an immediate Linear sync.
#[tauri::command]
pub fn start_linear_sync(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.integrations.linear_poller_wake.notify_one();
    Ok(())
}

/// I425: Get the 5 most recently synced Linear issues.
#[tauri::command]
pub async fn get_linear_recent_issues(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<serde_json::Value>, String> {
    state.db_read(|db| {
        let mut stmt = db.conn_ref().prepare(
            "SELECT id, identifier, title, state_name, state_type, priority_label, due_date, synced_at
             FROM linear_issues
             WHERE state_type NOT IN ('completed', 'cancelled')
             ORDER BY priority ASC, synced_at DESC LIMIT 5"
        ).map_err(|e| e.to_string())?;
        let issues = stmt.query_map([], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "identifier": row.get::<_, String>(1)?,
                "title": row.get::<_, String>(2)?,
                "stateName": row.get::<_, Option<String>>(3)?,
                "stateType": row.get::<_, Option<String>>(4)?,
                "priorityLabel": row.get::<_, Option<String>>(5)?,
                "dueDate": row.get::<_, Option<String>>(6)?,
                "syncedAt": row.get::<_, Option<String>>(7)?,
            }))
        }).map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
        Ok(issues)
    }).await
}

/// I425: Get all Linear entity links with project and entity names.
#[tauri::command]
pub async fn get_linear_entity_links(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<serde_json::Value>, String> {
    state
        .db_read(|db| {
            let mut stmt = db
                .conn_ref()
                .prepare(
                    "SELECT lel.id, lel.linear_project_id, lp.name as project_name,
                    lel.entity_id, lel.entity_type, lel.confirmed,
                    CASE lel.entity_type
                        WHEN 'account' THEN (SELECT name FROM accounts WHERE id = lel.entity_id)
                        WHEN 'project' THEN (SELECT name FROM projects WHERE id = lel.entity_id)
                        WHEN 'person' THEN (SELECT name FROM people WHERE id = lel.entity_id)
                    END as entity_name
             FROM linear_entity_links lel
             LEFT JOIN linear_projects lp ON lp.id = lel.linear_project_id
             ORDER BY lel.created_at DESC",
                )
                .map_err(|e| e.to_string())?;
            let links = stmt
                .query_map([], |row| {
                    Ok(serde_json::json!({
                        "id": row.get::<_, String>(0)?,
                        "linearProjectId": row.get::<_, String>(1)?,
                        "projectName": row.get::<_, Option<String>>(2)?,
                        "entityId": row.get::<_, String>(3)?,
                        "entityType": row.get::<_, String>(4)?,
                        "confirmed": row.get::<_, bool>(5)?,
                        "entityName": row.get::<_, Option<String>>(6)?,
                    }))
                })
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .collect();
            Ok(links)
        })
        .await
}

/// DOS-56: Jaccard word-token similarity for fuzzy name matching.
fn fuzzy_name_similarity(a: &str, b: &str) -> f64 {
    let a_lower = a.to_lowercase();
    let b_lower = b.to_lowercase();
    let a_tokens: HashSet<&str> = a_lower.split_whitespace().collect();
    let b_tokens: HashSet<&str> = b_lower.split_whitespace().collect();
    let intersection = a_tokens.intersection(&b_tokens).count();
    let union = a_tokens.union(&b_tokens).count();
    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

/// DOS-56: Auto-detect entity links by fuzzy-matching Linear project names to entity names,
/// plus domain-based suggestions from account_domains.
#[tauri::command]
pub async fn run_linear_auto_link(
    state: State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, String> {
    let state_for_ctx = state.inner().clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            let conn = db.conn_ref();
            let mut auto_linked: Vec<serde_json::Value> = Vec::new();
            let mut suggested: Vec<serde_json::Value> = Vec::new();

            // Get all Linear projects
            let projects: Vec<(String, String)> = {
                let mut stmt = conn
                    .prepare("SELECT id, name FROM linear_projects")
                    .map_err(|e| e.to_string())?;
                let rows = stmt
                    .query_map([], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                    })
                    .map_err(|e| e.to_string())?
                    .filter_map(|r| r.ok())
                    .collect();
                rows
            };

            // Get already-linked project IDs to skip
            let already_linked: HashSet<String> = {
                let mut stmt = conn
                    .prepare("SELECT linear_project_id FROM linear_entity_links")
                    .map_err(|e| e.to_string())?;
                let rows = stmt
                    .query_map([], |row| row.get::<_, String>(0))
                    .map_err(|e| e.to_string())?
                    .filter_map(|r| r.ok())
                    .collect();
                rows
            };

            // Get all accounts (id, name)
            let accounts: Vec<(String, String)> = {
                let mut stmt = conn
                    .prepare("SELECT id, name FROM accounts WHERE archived = 0")
                    .map_err(|e| e.to_string())?;
                let rows = stmt
                    .query_map([], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                    })
                    .map_err(|e| e.to_string())?
                    .filter_map(|r| r.ok())
                    .collect();
                rows
            };

            // Get all projects (id, name)
            let dos_projects: Vec<(String, String)> = {
                let mut stmt = conn
                    .prepare("SELECT id, name FROM projects WHERE archived = 0")
                    .map_err(|e| e.to_string())?;
                let rows = stmt
                    .query_map([], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                    })
                    .map_err(|e| e.to_string())?
                    .filter_map(|r| r.ok())
                    .collect();
                rows
            };

            // Get account domains for domain-based matching
            let account_domains: Vec<(String, String)> = {
                let mut stmt = conn
                    .prepare(
                        "SELECT ad.account_id, ad.domain FROM account_domains ad \
                         JOIN accounts a ON a.id = ad.account_id WHERE a.archived = 0",
                    )
                    .map_err(|e| e.to_string())?;
                let rows = stmt
                    .query_map([], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                    })
                    .map_err(|e| e.to_string())?
                    .filter_map(|r| r.ok())
                    .collect();
                rows
            };

            for (project_id, project_name) in &projects {
                if already_linked.contains(project_id) {
                    continue;
                }

                let mut best_score: f64 = 0.0;
                let mut best_entity_id: Option<String> = None;
                let mut best_entity_type: Option<&str> = None;
                let mut best_entity_name: Option<String> = None;

                // Score against accounts
                for (account_id, account_name) in &accounts {
                    let score = fuzzy_name_similarity(project_name, account_name);
                    if score > best_score {
                        best_score = score;
                        best_entity_id = Some(account_id.clone());
                        best_entity_type = Some("account");
                        best_entity_name = Some(account_name.clone());
                    }
                }

                // Score against DailyOS projects
                for (proj_id, proj_name) in &dos_projects {
                    let score = fuzzy_name_similarity(project_name, proj_name);
                    if score > best_score {
                        best_score = score;
                        best_entity_id = Some(proj_id.clone());
                        best_entity_type = Some("project");
                        best_entity_name = Some(proj_name.clone());
                    }
                }

                // Domain-based suggestion: check if project name words match any domain
                if best_score < 0.7 {
                    let proj_lower = project_name.to_lowercase();
                    for (account_id, domain) in &account_domains {
                        // Extract the domain base (e.g., "acme" from "acme.com")
                        let domain_base = domain.split('.').next().unwrap_or("").to_lowercase();
                        if domain_base.len() >= 3 && proj_lower.contains(&domain_base) {
                            // Find the account name for the suggestion
                            if let Some((_, acct_name)) =
                                accounts.iter().find(|(id, _)| id == account_id)
                            {
                                best_score = 0.75; // Domain match scores as a suggestion
                                best_entity_id = Some(account_id.clone());
                                best_entity_type = Some("account");
                                best_entity_name = Some(acct_name.clone());
                                break;
                            }
                        }
                    }
                }

                if best_score >= 0.9 {
                    // High confidence — auto-link
                    if let (Some(entity_id), Some(entity_type)) =
                        (&best_entity_id, best_entity_type)
                    {
                        crate::services::mutations::create_linear_entity_link_with_confirmed(
                            &ctx,
                            db,
                            project_id,
                            entity_id,
                            entity_type,
                            false,
                        )?;
                        auto_linked.push(serde_json::json!({
                            "linearProjectId": project_id,
                            "linearProjectName": project_name,
                            "entityId": entity_id,
                            "entityType": entity_type,
                            "entityName": best_entity_name,
                            "score": best_score,
                        }));
                    }
                } else if best_score >= 0.7 {
                    // Suggestion — needs user confirmation
                    if let (Some(entity_id), Some(entity_type)) =
                        (&best_entity_id, best_entity_type)
                    {
                        suggested.push(serde_json::json!({
                            "linearProjectId": project_id,
                            "linearProjectName": project_name,
                            "entityId": entity_id,
                            "entityType": entity_type,
                            "entityName": best_entity_name,
                            "score": best_score,
                        }));
                    }
                }
            }

            Ok(serde_json::json!({
                "autoLinked": auto_linked,
                "suggested": suggested,
            }))
        })
        .await
}

/// I425: Delete a Linear entity link.
#[tauri::command]
pub async fn delete_linear_entity_link(
    state: State<'_, Arc<AppState>>,
    link_id: String,
) -> Result<(), String> {
    let state_for_ctx = state.inner().clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            crate::services::mutations::delete_linear_entity_link(&ctx, db, &link_id)
        })
        .await
}

/// List all Linear projects for the manual link picker.
#[tauri::command]
pub async fn get_linear_projects(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<serde_json::Value>, String> {
    state
        .db_read(|db| {
            let mut stmt = db
                .conn_ref()
                .prepare("SELECT id, name FROM linear_projects ORDER BY name ASC")
                .map_err(|e| e.to_string())?;
            let projects = stmt
                .query_map([], |row| {
                    Ok(serde_json::json!({
                        "id": row.get::<_, String>(0)?,
                        "name": row.get::<_, String>(1)?,
                    }))
                })
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .collect();
            Ok(projects)
        })
        .await
}

/// Manually create a Linear entity link.
#[tauri::command]
pub async fn create_linear_entity_link(
    services: State<'_, crate::services::ServiceLayer>,
    linear_project_id: String,
    entity_id: String,
    entity_type: String,
) -> Result<(), String> {
    let state = services.state_arc();
    if !["account", "project"].contains(&entity_type.as_str()) {
        return Err("entity_type must be 'account' or 'project'".to_string());
    }
    let state_for_ctx = state.clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            crate::services::mutations::create_linear_entity_link(
                &ctx,
                db,
                &linear_project_id,
                &entity_id,
                &entity_type,
            )
        })
        .await
}

// =============================================================================
// DOS-50/51: Push Action to Linear
// =============================================================================

/// Fetch teams from Linear for the push dialog.
#[tauri::command]
pub async fn get_linear_teams(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::linear::client::LinearTeam>, String> {
    let api_key = state
        .config
        .read()
        .as_ref()
        .and_then(|c| c.linear.api_key.clone())
        .ok_or("No Linear API key configured")?;

    let client = crate::linear::client::LinearClient::new(&api_key);
    client.fetch_teams().await
}

/// Push a DailyOS action to Linear as a new issue (DOS-51).
#[tauri::command]
pub async fn push_action_to_linear(
    action_id: String,
    team_id: String,
    project_id: Option<String>,
    title: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<crate::services::linear::LinearPushResult, String> {
    crate::services::linear::push_action_to_linear(
        &state,
        &action_id,
        &team_id,
        project_id.as_deref(),
        title.as_deref(),
    )
    .await
}

// =============================================================================
// I309: Role Presets
// =============================================================================

/// Set the active role preset.
#[tauri::command]
pub async fn set_role(
    role: String,
    state: State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    let preset = crate::presets::loader::load_preset(&role)?;

    crate::state::create_or_update_config(&state, |c| {
        c.role = preset.id.clone();
        c.custom_preset_path = None;
        c.entity_mode = preset.default_entity_mode.clone();
        c.profile = crate::types::profile_for_entity_mode(&c.entity_mode);
    })?;

    // DOS-176: update active preset and recompute merged signal/email config cache.
    state.set_active_preset(preset);

    let _ = app_handle.emit("config-updated", ());
    Ok("ok".to_string())
}

/// Get the currently active role preset.
#[tauri::command]
pub async fn get_active_preset(
    state: State<'_, Arc<AppState>>,
) -> Result<Option<crate::presets::schema::RolePreset>, String> {
    Ok(state.active_preset.read().clone())
}

/// List all available role presets.
#[tauri::command]
pub async fn get_available_presets() -> Result<Vec<(String, String, String)>, String> {
    Ok(crate::presets::loader::get_available_presets())
}

// =============================================================================
// I311: Entity Metadata
// =============================================================================

/// Update JSON metadata for an entity (account or project).
#[tauri::command]
pub async fn update_entity_metadata(
    entity_type: String,
    entity_id: String,
    metadata: String,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    serde_json::from_str::<serde_json::Value>(&metadata)
        .map_err(|e| format!("Invalid JSON metadata: {}", e))?;
    let engine = state.signals.engine.clone();
    let state_for_ctx = state.inner().clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            crate::services::mutations::update_entity_metadata(
                &ctx,
                db,
                &engine,
                &entity_type,
                &entity_id,
                &metadata,
            )
        })
        .await?;
    Ok("ok".to_string())
}

/// Get JSON metadata for an entity (account or project).
#[tauri::command]
pub async fn get_entity_metadata(
    entity_type: String,
    entity_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    state
        .db_read(move |db| db.get_entity_metadata(&entity_type, &entity_id))
        .await
}

// =============================================================================
// I323: Email Disposition Correction
// =============================================================================

/// Correct an email disposition (I323).
/// Records a feedback signal for Thompson Sampling priority recalibration.
/// Does NOT un-archive the email (user can find it in Gmail "All Mail").
#[tauri::command]
pub async fn correct_email_disposition(
    email_id: String,
    corrected_priority: String,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let valid_priorities = ["high", "medium", "low"];
    if !valid_priorities.contains(&corrected_priority.as_str()) {
        return Err(format!(
            "Invalid priority: {}. Must be high, medium, or low.",
            corrected_priority
        ));
    }

    let state_for_ctx = state.inner().clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            let scoring_source = db
                .get_email_signal_source_for_feedback(&email_id)
                .map_err(|e| format!("Failed to resolve email signal source: {}", e))?
                .unwrap_or_else(|| "email_enrichment".to_string());

            crate::services::mutations::upsert_email_feedback_signal(
                &ctx,
                db,
                &email_id,
                &corrected_priority,
            )?;

            crate::services::mutations::upsert_signal_weight(
                &ctx,
                db,
                &scoring_source,
                "email",
                "email_priority",
                0.0,
                1.0,
            )
            .map_err(|e| format!("Failed to update signal weights: {}", e))?;

            log::info!(
                "correct_email_disposition: {} corrected to {} (penalized source: {})",
                email_id,
                corrected_priority,
                scoring_source
            );
            Ok(format!("Disposition corrected to {}", corrected_priority))
        })
        .await
}

// =============================================================================
// I330: Meeting Timeline (±7 days)
// =============================================================================

/// Return meetings for +/-N days around today with intelligence quality data.
///
/// Always-live: if no future meetings exist in `meetings`, fetches from
/// Google Calendar and upserts stubs so the timeline populates on first load
/// without waiting for scheduled workflows.
#[tauri::command]
pub async fn get_meeting_timeline(
    state: State<'_, Arc<AppState>>,
    days_before: Option<i64>,
    days_after: Option<i64>,
) -> Result<Vec<crate::types::TimelineMeeting>, String> {
    let days_after_val = days_after.unwrap_or(7);
    let result =
        crate::services::meetings::get_meeting_timeline(&state, days_before, days_after).await?;

    // Check if we have any meetings AFTER today (i.e., tomorrow or later)
    let tomorrow_str = (chrono::Local::now().date_naive() + chrono::Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();
    let has_future = result
        .iter()
        .any(|m| m.start_time.as_str() >= tomorrow_str.as_str());
    if has_future || days_after_val == 0 {
        // Enqueue future meetings that have no prep_frozen_json yet
        let ts = tomorrow_str.clone();
        let needs_prep: Vec<String> = state
            .db_read(move |db| {
                Ok(db
                    .conn_ref()
                    .prepare(
                        "SELECT m.id FROM meetings m
                     LEFT JOIN meeting_prep mp ON mp.meeting_id = m.id
                     WHERE m.start_time >= ?1
                       AND mp.prep_frozen_json IS NULL
                       AND m.meeting_type NOT IN ('personal', 'focus', 'blocked')",
                    )
                    .and_then(|mut stmt| {
                        let rows =
                            stmt.query_map(rusqlite::params![ts], |row| row.get::<_, String>(0))?;
                        Ok(rows.filter_map(|r| r.ok()).collect())
                    })
                    .unwrap_or_default())
            })
            .await
            .unwrap_or_default();
        if !needs_prep.is_empty() {
            log::info!(
                "get_meeting_timeline: enqueuing {} future meetings without prep",
                needs_prep.len()
            );
            for mid in needs_prep {
                state
                    .meeting_prep_queue
                    .enqueue(crate::meeting_prep_queue::PrepRequest::new(
                        mid,
                        crate::meeting_prep_queue::PrepPriority::PageLoad,
                    ));
            }
            state.integrations.prep_queue_wake.notify_one();
        }
        return Ok(result);
    }

    // No future meetings in DB — try live fetch from Google Calendar
    let access_token = match crate::google_api::get_valid_access_token().await {
        Ok(t) => t,
        Err(_) => return Ok(result), // No auth — return what we have
    };

    let today = chrono::Local::now().date_naive();
    let range_end = today + chrono::Duration::days(days_after_val);
    let raw_events = match crate::google_api::calendar::fetch_events(
        &access_token,
        today + chrono::Duration::days(1), // tomorrow onward (today already covered)
        range_end,
    )
    .await
    {
        Ok(events) => events,
        Err(e) => {
            log::warn!("get_meeting_timeline: live calendar fetch failed: {}", e);
            return Ok(result);
        }
    };

    if raw_events.is_empty() {
        return Ok(result);
    }

    // Classify and upsert into meetings (same pattern as prepare_today)
    let user_domains = state
        .config
        .read()
        .as_ref()
        .map(|c| c.resolved_user_domains())
        .unwrap_or_default();
    let entity_hints = state
        .db_read(|db| Ok(crate::helpers::build_entity_hints(db)))
        .await?;

    // Classify events first (no DB needed)
    let mut to_upsert: Vec<(
        crate::types::CalendarEvent,
        Vec<crate::google_api::classify::ResolvedMeetingEntity>,
    )> = Vec::new();
    for raw in &raw_events {
        let cm =
            crate::google_api::classify::classify_meeting_multi(raw, &user_domains, &entity_hints);
        let event = cm.to_calendar_event();

        // Skip personal (matches timeline query filter)
        if matches!(event.meeting_type, crate::types::MeetingType::Personal) {
            continue;
        }
        let resolved = cm.resolved_entities.clone();
        to_upsert.push((event, resolved));
    }

    // Batch upsert in a single DB write
    let state_for_ctx = state.inner().clone();
    let upserted_ids = state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            let mut ids: Vec<String> = Vec::new();
            for (event, resolved_entities) in &to_upsert {
                // Only insert if not already present
                if db
                    .get_meeting_by_calendar_event_id(&event.id)
                    .ok()
                    .flatten()
                    .is_some()
                {
                    continue;
                }

                let attendees_json = if event.attendees.is_empty() {
                    None
                } else {
                    Some(serde_json::to_string(&event.attendees).unwrap_or_default())
                };

                let db_meeting = crate::db::DbMeeting {
                    id: event.id.clone(),
                    title: event.title.clone(),
                    meeting_type: event.meeting_type.as_str().to_string(),
                    start_time: event.start.to_rfc3339(),
                    end_time: Some(event.end.to_rfc3339()),
                    attendees: attendees_json,
                    notes_path: None,
                    summary: None,
                    created_at: chrono::Utc::now().to_rfc3339(),
                    calendar_event_id: Some(event.id.clone()),
                    description: None,
                    prep_context_json: None,
                    user_agenda_json: None,
                    user_notes: None,
                    prep_frozen_json: None,
                    prep_frozen_at: None,
                    prep_snapshot_path: None,
                    prep_snapshot_hash: None,
                    transcript_path: None,
                    transcript_processed_at: None,
                    intelligence_state: None,
                    intelligence_quality: None,
                    last_enriched_at: None,
                    signal_count: None,
                    has_new_signals: None,
                    last_viewed_at: None,
                };
                let links: Vec<(String, String)> = resolved_entities
                    .iter()
                    .map(|re| (re.entity_id.clone(), re.entity_type.clone()))
                    .collect();
                if let Err(e) = crate::services::mutations::upsert_timeline_meeting_with_entities(
                    &ctx,
                    db,
                    &db_meeting,
                    &links,
                ) {
                    log::warn!(
                        "get_meeting_timeline: failed to upsert '{}': {}",
                        event.title,
                        e
                    );
                    continue;
                }

                ids.push(event.id.clone());
            }
            Ok(ids)
        })
        .await?;

    let upserted = upserted_ids.len() as u32;

    if upserted > 0 {
        log::info!(
            "get_meeting_timeline: upserted {} future meetings from Google Calendar",
            upserted
        );

        // Enqueue newly upserted meetings for prep generation
        for mid in &upserted_ids {
            state
                .meeting_prep_queue
                .enqueue(crate::meeting_prep_queue::PrepRequest::new(
                    mid.clone(),
                    crate::meeting_prep_queue::PrepPriority::PageLoad,
                ));
        }
        if !upserted_ids.is_empty() {
            state.integrations.prep_queue_wake.notify_one();
        }

        // Re-query with the newly upserted meetings
        return crate::services::meetings::get_meeting_timeline(&state, days_before, days_after)
            .await;
    }

    Ok(result)
}

// =============================================================================
// I390: Person Relationships (ADR-0088)
// =============================================================================

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationshipPayload {
    /// Pass an existing ID to update; omit for a new relationship.
    pub id: Option<String>,
    pub from_person_id: String,
    pub to_person_id: String,
    pub relationship_type: String,
    #[serde(default = "default_rel_direction")]
    pub direction: String,
    #[serde(default = "default_rel_confidence")]
    pub confidence: f64,
    pub context_entity_id: Option<String>,
    pub context_entity_type: Option<String>,
    #[serde(default = "default_rel_source")]
    pub source: String,
}

fn default_rel_direction() -> String {
    "directed".to_string()
}
fn default_rel_confidence() -> f64 {
    0.8
}
fn default_rel_source() -> String {
    "user_confirmed".to_string()
}

#[tauri::command]
pub async fn upsert_person_relationship(
    state: State<'_, Arc<AppState>>,
    payload: RelationshipPayload,
) -> Result<String, String> {
    // Validate relationship type parses
    payload
        .relationship_type
        .parse::<crate::db::person_relationships::RelationshipType>()
        .map_err(|e| format!("Invalid relationship type: {}", e))?;

    let engine = state.signals.engine.clone();
    let state_for_ctx = state.inner().clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            let id = payload
                .id
                .unwrap_or_else(|| format!("rel-{}", uuid::Uuid::new_v4()));
            crate::services::mutations::upsert_person_relationship(
                &ctx,
                db,
                &engine,
                &crate::db::person_relationships::UpsertRelationship {
                    id: &id,
                    from_person_id: &payload.from_person_id,
                    to_person_id: &payload.to_person_id,
                    relationship_type: &payload.relationship_type,
                    direction: &payload.direction,
                    confidence: payload.confidence,
                    context_entity_id: payload.context_entity_id.as_deref(),
                    context_entity_type: payload.context_entity_type.as_deref(),
                    source: &payload.source,
                    rationale: None,
                },
            )?;
            Ok(id)
        })
        .await
}

#[tauri::command]
pub async fn delete_person_relationship(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<(), String> {
    let engine = state.signals.engine.clone();
    let state_for_ctx = state.inner().clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            crate::services::mutations::delete_person_relationship(&ctx, db, &engine, &id)
        })
        .await
}

#[tauri::command]
pub async fn get_person_relationships(
    state: State<'_, Arc<AppState>>,
    person_id: String,
) -> Result<Vec<crate::db::person_relationships::PersonRelationship>, String> {
    state
        .db_read(move |db| {
            db.get_relationships_for_person(&person_id)
                .map_err(|e| format!("Failed to get relationships: {}", e))
        })
        .await
}

// =========================================================================
// Google Drive Connector (I426)
// =========================================================================

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveStatusData {
    pub enabled: bool,
    pub connected: bool,
    pub watched_count: i64,
    pub synced_count: i64,
    pub last_sync_at: Option<String>,
    pub poll_interval_minutes: u32,
}

/// Get a valid Google OAuth access token for use with Drive API and Picker.
/// Returns the token string or an error if not authenticated.
#[tauri::command]
pub async fn get_google_access_token() -> Result<String, String> {
    crate::google_api::get_valid_access_token()
        .await
        .map_err(|e| format!("Failed to get access token: {}", e))
}

/// Get Google API Client ID for use with Google Picker API.
/// Returns the numeric project ID extracted from the full client_id.
#[tauri::command]
pub fn get_google_client_id() -> String {
    // Extract numeric project ID from client_id format: "245504828099-xxx.apps.googleusercontent.com"
    "245504828099".to_string()
}

/// Get Google Drive integration status.
#[tauri::command]
pub async fn get_google_drive_status(
    state: State<'_, Arc<AppState>>,
) -> Result<crate::commands::DriveStatusData, String> {
    let config = state.config.read().as_ref().map(|c| c.drive.clone());

    let drive_config = config.unwrap_or_default();

    let connected = matches!(
        *state.calendar.google_auth.lock(),
        crate::types::GoogleAuthStatus::Authenticated { .. }
    );

    let (watched_count, synced_count, last_sync) = state
        .db_read(|db| {
            let conn = db.conn_ref();
            let watched: i64 = conn
                .query_row("SELECT COUNT(*) FROM drive_watched_sources", [], |row| {
                    row.get(0)
                })
                .unwrap_or(0);
            let synced: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM drive_watched_sources WHERE last_synced_at IS NOT NULL",
                    [],
                    |row| row.get(0),
                )
                .unwrap_or(0);
            let last: Option<String> = conn
                .query_row(
                    "SELECT MAX(last_synced_at) FROM drive_watched_sources",
                    [],
                    |row| row.get(0),
                )
                .unwrap_or(None);
            Ok((watched, synced, last))
        })
        .await
        .unwrap_or((0, 0, None));

    Ok(crate::commands::DriveStatusData {
        enabled: drive_config.enabled,
        connected,
        watched_count,
        synced_count,
        last_sync_at: last_sync,
        poll_interval_minutes: drive_config.poll_interval_minutes,
    })
}

/// Enable or disable Google Drive integration.
#[tauri::command]
pub fn set_google_drive_enabled(
    enabled: bool,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    crate::state::create_or_update_config(&state, |config| {
        config.drive.enabled = enabled;
    })?;
    Ok(())
}

/// Trigger an immediate Drive sync.
#[tauri::command]
pub fn trigger_drive_sync_now(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.integrations.drive_poller_wake.notify_one();
    Ok(())
}

/// Import a file from Google Drive once (no ongoing sync).
///
/// Downloads the file, converts to markdown, and saves to the entity's
/// Documents/ folder. Does NOT create a watched source entry.
#[tauri::command]
pub async fn import_google_drive_file(
    google_id: String,
    name: String,
    entity_id: String,
    entity_type: String,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let content = crate::google_drive::client::download_file_as_markdown(&google_id).await?;

    let workspace = state
        .config
        .read()
        .as_ref()
        .map(|c| c.workspace_path.clone())
        .ok_or("Workspace not configured")?;

    let path = crate::google_drive::poller::save_to_entity_docs(
        &workspace,
        &entity_type,
        &entity_id,
        &name,
        &content,
    )?;

    log::info!("Drive import (once): saved {} to {}", name, path.display());
    Ok(path.display().to_string())
}

/// Add a watched Drive source linked to an entity.
#[tauri::command]
pub async fn add_google_drive_watch(
    google_id: String,
    name: String,
    file_type: String,
    google_doc_url: Option<String>,
    entity_id: String,
    entity_type: String,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let watch_id = state
        .db_write(move |db| {
            crate::google_drive::sync::upsert_watched_source(
                db,
                &google_id,
                &name,
                &file_type,
                google_doc_url.as_deref(),
                &entity_id,
                &entity_type,
            )
        })
        .await?;

    // Wake the poller so it does an initial sync
    state.integrations.drive_poller_wake.notify_one();

    Ok(watch_id)
}

/// Remove a watched Drive source.
#[tauri::command]
pub async fn remove_google_drive_watch(
    watch_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    state
        .db_write(move |db| crate::google_drive::sync::remove_watched_source(db, &watch_id))
        .await
}

/// Get all watched Drive sources.
#[tauri::command]
pub async fn get_google_drive_watches(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<DriveWatchData>, String> {
    let sources = state
        .db_read(crate::google_drive::sync::get_all_watched_sources)
        .await?;
    Ok(sources
        .into_iter()
        .map(|s| DriveWatchData {
            id: s.id,
            google_id: s.google_id,
            name: s.name,
            file_type: s.file_type,
            google_doc_url: s.google_doc_url,
            entity_id: s.entity_id,
            entity_type: s.entity_type,
            last_synced_at: s.last_synced_at,
        })
        .collect())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveWatchData {
    pub id: String,
    pub google_id: String,
    pub name: String,
    pub file_type: String,
    pub google_doc_url: Option<String>,
    pub entity_id: String,
    pub entity_type: String,
    pub last_synced_at: Option<String>,
}

// =============================================================================
// I471: Audit Log Commands
// =============================================================================

/// Get recent audit log records, optionally filtered by category.
#[tauri::command]
pub fn get_audit_log_records(
    limit: Option<usize>,
    category_filter: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Vec<crate::audit_log::AuditRecord> {
    let path = state.audit_log.lock().path().to_path_buf();

    crate::audit_log::read_records(&path, limit.unwrap_or(100), category_filter.as_deref())
}

/// Export the audit log to a user-selected path.
#[tauri::command]
pub fn export_audit_log(dest_path: String, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let src = state.audit_log.lock().path().to_path_buf();

    if !src.exists() {
        return Err("No audit log file exists yet".to_string());
    }

    std::fs::copy(&src, &dest_path).map_err(|e| format!("Failed to export audit log: {e}"))?;
    Ok(())
}

/// Verify the audit log hash chain integrity.
#[tauri::command]
pub fn verify_audit_log_integrity(state: State<'_, Arc<AppState>>) -> Result<String, String> {
    let path = state.audit_log.lock().path().to_path_buf();

    if !path.exists() {
        return Ok("No audit log file exists yet.".to_string());
    }

    match crate::audit_log::verify_audit_log(&path) {
        Ok(count) => Ok(format!(
            "Integrity verified: {} records, hash chain intact.",
            count
        )),
        Err((line, msg)) => Err(format!(
            "Integrity check failed at record {}: {}",
            line, msg
        )),
    }
}

// ---------------------------------------------------------------------------
// Context Mode (ADR-0095)
// ---------------------------------------------------------------------------

/// Get the current context mode (Local or Glean).
#[tauri::command]
pub fn get_context_mode(state: State<'_, Arc<AppState>>) -> Result<serde_json::Value, String> {
    let mode = state.with_db_read(|db| Ok(crate::context_provider::read_context_mode(db)))?;

    serde_json::to_value(&mode).map_err(|e| format!("Serialization error: {}", e))
}

/// Set the context mode and hot-swap the provider immediately.
/// In Glean mode, Clay and Gravatar enrichment are automatically disabled.
///
/// Uses the async `db_read` / `db_write` helpers (DbService pool) rather than
/// the sync `with_db_read` / `with_db_write` helpers, which open a fresh
/// `ActionDb` connection and can fail key verification under DbService's
/// held-writer contention (v1.2.1 post-migration-108 regression).
#[tauri::command]
pub async fn set_context_mode(
    mode: serde_json::Value,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let parsed: crate::context_provider::ContextMode =
        serde_json::from_value(mode).map_err(|e| format!("Invalid context mode: {}", e))?;

    // Read current mode before writing so we can log from/to
    let previous_mode = state
        .db_read(|db| Ok(crate::context_provider::read_context_mode(db)))
        .await
        .unwrap_or_default();

    let parsed_for_write = parsed.clone();
    state
        .db_write(move |db| crate::context_provider::save_context_mode(db, &parsed_for_write))
        .await?;

    // Log the mode change with from/to
    let mode_name = |m: &crate::context_provider::ContextMode| -> &str {
        match m {
            crate::context_provider::ContextMode::Local => "local",
            crate::context_provider::ContextMode::Glean { .. } => "glean",
        }
    };
    {
        let mut audit = state.audit_log.lock();
        let _ = audit.append(
            "config",
            "context_mode_changed",
            serde_json::json!({
                "from": mode_name(&previous_mode),
                "to": mode_name(&parsed),
            }),
        );
    }

    // DOS-259 (W2-B cycle 4): `build_context_provider` already installs
    // the full atomic bundle (context_provider + intelligence_provider Arc
    // + glean_intelligence_provider Arc) in one write-lock acquisition
    // via `set_context_mode_atomic`. Calling `swap_context_provider` after
    // it would reopen the L2-flagged race window where two concurrent
    // settings flips can interleave atomic-then-single-field updates and
    // leave a torn bundle. Drop the redundant single-field swap.
    let _ = state.build_context_provider(&parsed);

    if let Ok(targets) = state
        .db_read(|db| {
            let stale = db
                .get_stale_entity_intelligence(0)
                .unwrap_or_default()
                .into_iter()
                .map(|(id, typ, _)| (id, typ))
                .collect::<Vec<_>>();
            let missing = db.get_entities_without_intelligence().unwrap_or_default();
            let mut targets = stale;
            targets.extend(missing);
            Ok::<_, String>(targets)
        })
        .await
    {
        use crate::intel_queue::{IntelPriority, IntelRequest};
        for (id, typ) in &targets {
            let _ = state.intel_queue.enqueue(IntelRequest::new(
                id.clone(),
                typ.clone(),
                IntelPriority::ProactiveHygiene,
            ));
        }
        if !targets.is_empty() {
            log::info!(
                "Context mode switch: enqueued {} entities for re-enrichment",
                targets.len()
            );
        }
    }

    Ok(())
}

/// Start Glean OAuth consent flow — opens browser for SSO authentication.
///
/// Uses MCP OAuth discovery + DCR from the Glean MCP endpoint URL.
/// Returns `GleanAuthStatus::Authenticated` on success.
#[tauri::command]
pub async fn start_glean_auth(
    endpoint: String,
    state: State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<crate::glean::GleanAuthStatus, String> {
    use crate::glean;

    match glean::oauth::run_glean_consent_flow(&endpoint).await {
        Ok(result) => {
            let status = glean::GleanAuthStatus::Authenticated {
                email: result.email.unwrap_or_else(|| "connected".to_string()),
                name: result.name,
            };

            // Audit: oauth_connected
            {
                let mut audit = state.audit_log.lock();
                let _ = audit.append(
                    "security",
                    "oauth_connected",
                    serde_json::json!({"provider": "glean"}),
                );
            }

            // Auto-set context mode to Glean (Blockers 1 & 3).
            let glean_mode = crate::context_provider::ContextMode::Glean {
                endpoint: endpoint.clone(),
            };
            let glean_mode_for_write = glean_mode.clone();
            if let Err(e) = state
                .db_write(move |db| {
                    crate::context_provider::save_context_mode(db, &glean_mode_for_write)
                })
                .await
            {
                log::error!("Failed to save Glean context mode: {}", e);
            }

            // DOS-259 (W2-B cycle 4): `build_context_provider` performs
            // the full atomic transition; redundant single-field swap
            // removed (would reopen the L2 race window).
            let _ = state.build_context_provider(&glean_mode);

            // Audit: context mode auto-set
            {
                let mut audit = state.audit_log.lock();
                let _ = audit.append(
                    "config",
                    "context_mode_changed",
                    serde_json::json!({"from": "local", "to": "glean", "trigger": "glean_auth"}),
                );
            }

            // I568: Enqueue all entities for re-enrichment — use db_read to avoid blocking Tokio.
            {
                let entities_to_enqueue: Vec<(String, String)> = state
                    .db_read(|db| {
                        let mut all = Vec::new();
                        if let Ok(stale) = db.get_stale_entity_intelligence(0) {
                            for (id, typ, _) in stale {
                                all.push((id, typ));
                            }
                        }
                        if let Ok(missing) = db.get_entities_without_intelligence() {
                            all.extend(missing);
                        }
                        Ok(all)
                    })
                    .await
                    .unwrap_or_default();

                if !entities_to_enqueue.is_empty() {
                    use crate::intel_queue::{IntelPriority, IntelRequest};
                    let count = entities_to_enqueue.len();
                    for (entity_id, entity_type) in entities_to_enqueue {
                        let _ = state.intel_queue.enqueue(IntelRequest::new(
                            entity_id,
                            entity_type,
                            IntelPriority::ProactiveHygiene,
                        ));
                    }
                    log::info!("Glean auth: enqueued {} entities for re-enrichment", count);
                }
            }

            let _ = app_handle.emit("glean-auth-changed", &status);
            Ok(status)
        }
        Err(glean::GleanAuthError::FlowCancelled) => {
            Err("Glean authorization was cancelled".to_string())
        }
        Err(e) => {
            let message = format!("{}", e);
            let _ = app_handle.emit(
                "glean-auth-failed",
                serde_json::json!({ "message": message }),
            );
            Err(message)
        }
    }
}

/// Get current Glean authentication status from Keychain.
#[tauri::command]
pub fn get_glean_auth_status() -> crate::glean::GleanAuthStatus {
    crate::glean::detect_glean_auth()
}

/// Disconnect Glean — delete OAuth token from Keychain.
///
/// Uses async `db_write` (DbService pool) rather than sync `with_db_write`
/// for the same reason as `set_context_mode` — fresh-open path fails key
/// verification under DbService contention.
#[tauri::command]
pub async fn disconnect_glean(
    state: State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    crate::glean::token_store::delete_token().map_err(|e| format!("{}", e))?;

    let purge_report = state
        .db_write(|db| {
            crate::db::data_lifecycle::purge_source(
                db,
                crate::db::data_lifecycle::DataSource::Glean,
            )
            .map_err(|e| e.to_string())
        })
        .await?;

    // Audit: oauth_revoked
    {
        let mut audit = state.audit_log.lock();
        let _ = audit.append(
            "security",
            "oauth_revoked",
            serde_json::json!({"provider": "glean", "purge": purge_report}),
        );
    }

    // Revert context mode to Local and hot-swap provider.
    let local_mode = crate::context_provider::ContextMode::Local;
    let local_mode_for_write = local_mode.clone();
    if let Err(e) = state
        .db_write(move |db| crate::context_provider::save_context_mode(db, &local_mode_for_write))
        .await
    {
        log::error!("Failed to save Local context mode on disconnect: {}", e);
    }
    // DOS-259 (W2-B cycle 4): atomic transition; redundant single-field
    // swap removed (would reopen the L2 race window).
    let _ = state.build_context_provider(&local_mode);

    // Audit: context mode reverted
    {
        let mut audit = state.audit_log.lock();
        let _ = audit.append(
            "config",
            "context_mode_changed",
            serde_json::json!({"from": "glean", "to": "local", "trigger": "glean_disconnect"}),
        );
    }

    let status = crate::glean::GleanAuthStatus::NotConfigured;
    let _ = app_handle.emit("glean-auth-changed", &status);

    log::info!("Glean disconnected, context provider reverted to local");
    Ok(())
}

// ---------------------------------------------------------------------------
// I559 — Glean Agent Validation Spike (temporary exploration command)
// ---------------------------------------------------------------------------

/// Explore what tools the Glean MCP server exposes and test structured output.
///
/// This is a temporary dev command for the I559 validation spike.
/// It calls `tools/list` to discover available tools, then optionally
/// tests a structured query if an account name is provided.
///
/// Returns a JSON report of everything discovered.
#[tauri::command]
pub async fn dev_explore_glean_tools(
    account_name: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, String> {
    use crate::context_provider::{self, ContextMode};
    use std::time::Instant;

    let mode = state
        .with_db_read(|db| Ok(context_provider::read_context_mode(db)))
        .map_err(|e| format!("DB error: {e}"))?;

    let endpoint = match &mode {
        ContextMode::Glean { endpoint, .. } => endpoint.clone(),
        ContextMode::Local => {
            return Err(
                "Glean not configured. Set context mode to Glean in Settings first.".into(),
            );
        }
    };

    // Use the same token refresh path as GleanMcpClient — load_token() reads raw
    // from keychain without refreshing, which may return an expired token.
    let token = crate::glean::get_valid_access_token()
        .await
        .map_err(|e| format!("Glean token error: {e}"))?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;

    let mut report = serde_json::json!({
        "endpoint": endpoint,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "sections": {}
    });

    // -----------------------------------------------------------------------
    // 1. Tool Discovery — tools/list
    // -----------------------------------------------------------------------
    log::info!("[I559] Calling tools/list on {}", endpoint);
    let list_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });

    let start = Instant::now();
    let list_result = client
        .post(&endpoint)
        .bearer_auth(&token)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .json(&list_body)
        .send()
        .await;

    let list_latency_ms = start.elapsed().as_millis();

    match list_result {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let body: serde_json::Value = resp.json().await.unwrap_or(serde_json::json!(null));
            report["sections"]["tools_list"] = serde_json::json!({
                "status": status,
                "latency_ms": list_latency_ms,
                "response": body,
            });
            log::info!(
                "[I559] tools/list returned {} in {}ms",
                status,
                list_latency_ms
            );
        }
        Err(e) => {
            report["sections"]["tools_list"] = serde_json::json!({
                "error": format!("{e}"),
                "latency_ms": list_latency_ms,
            });
            log::warn!("[I559] tools/list failed: {e}");
        }
    }

    // -----------------------------------------------------------------------
    // 2. Structured JSON output test (if account_name provided)
    // -----------------------------------------------------------------------
    if let Some(ref acct) = account_name {
        // Test: can we get structured JSON back from a search-based query?
        let structured_query = format!(
            "Analyze the account health for {}. Return your analysis as a JSON object with these exact fields: \
            {{ \"score\": <number 0-100>, \"band\": \"green\"|\"yellow\"|\"red\", \
            \"risks\": [{{ \"text\": \"<risk description>\", \"urgency\": \"critical\"|\"watch\"|\"low\" }}], \
            \"stakeholders\": [{{ \"name\": \"<person name>\", \"role\": \"<job title>\", \"engagement\": \"high\"|\"medium\"|\"low\" }}], \
            \"competitive_mentions\": [\"<competitor name>\"], \
            \"summary\": \"<2-3 sentence executive assessment>\" }}",
            acct
        );

        // 2a. Test via search tool (what we already have)
        log::info!(
            "[I559] Testing structured query via 'search' tool for {}",
            acct
        );
        let search_body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "search",
                "arguments": {
                    "query": format!("{} account health risks stakeholders", acct),
                    "maxResults": 5
                }
            }
        });

        let start = Instant::now();
        let search_result = client
            .post(&endpoint)
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .json(&search_body)
            .send()
            .await;
        let search_latency_ms = start.elapsed().as_millis();

        match search_result {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let body: serde_json::Value = resp.json().await.unwrap_or(serde_json::json!(null));
                report["sections"]["search_test"] = serde_json::json!({
                    "status": status,
                    "latency_ms": search_latency_ms,
                    "query": format!("{} account health risks stakeholders", acct),
                    "response": body,
                });
            }
            Err(e) => {
                report["sections"]["search_test"] = serde_json::json!({
                    "error": format!("{e}"),
                    "latency_ms": search_latency_ms,
                });
            }
        }

        // 2b. Test via any discovered tools that look like they accept natural language
        // We'll try calling tools named "ask", "chat", "query", or any agent-like tool
        // if tools/list revealed them
        let interesting_tools: Vec<String> = report["sections"]["tools_list"]["response"]["result"]
            ["tools"]
            .as_array()
            .map(|tools| {
                tools
                    .iter()
                    .filter_map(|t| t["name"].as_str())
                    .filter(|name| {
                        let n = name.to_lowercase();
                        n != "search"
                            && n != "read_document"
                            && (n.contains("ask")
                                || n.contains("chat")
                                || n.contains("query")
                                || n.contains("agent")
                                || n.contains("answer")
                                || n.contains("analyze")
                                || n.contains("run"))
                    })
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        if !interesting_tools.is_empty() {
            for tool_name in interesting_tools.iter().take(3) {
                log::info!(
                    "[I559] Testing discovered tool '{}' with structured query",
                    tool_name
                );

                let tool_body = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 3,
                    "method": "tools/call",
                    "params": {
                        "name": tool_name,
                        "arguments": {
                            "query": structured_query,
                        }
                    }
                });

                let start = Instant::now();
                let tool_result = client
                    .post(&endpoint)
                    .bearer_auth(&token)
                    .header("Content-Type", "application/json")
                    .header("Accept", "application/json, text/event-stream")
                    .json(&tool_body)
                    .send()
                    .await;
                let tool_latency_ms = start.elapsed().as_millis();

                let key = format!("tool_test_{}", tool_name);
                match tool_result {
                    Ok(resp) => {
                        let status = resp.status().as_u16();
                        let body: serde_json::Value =
                            resp.json().await.unwrap_or(serde_json::json!(null));

                        // Check if the response contains parseable JSON
                        let has_json = body["result"]["content"]
                            .as_array()
                            .and_then(|arr| arr.first())
                            .and_then(|c| c["text"].as_str())
                            .map(|text| {
                                // Try to find JSON in the response text
                                if let Some(start) = text.find('{') {
                                    if let Some(end) = text.rfind('}') {
                                        serde_json::from_str::<serde_json::Value>(
                                            &text[start..=end],
                                        )
                                        .is_ok()
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                }
                            })
                            .unwrap_or(false);

                        report["sections"][&key] = serde_json::json!({
                            "tool": tool_name,
                            "status": status,
                            "latency_ms": tool_latency_ms,
                            "contains_parseable_json": has_json,
                            "response": body,
                        });
                    }
                    Err(e) => {
                        report["sections"][&key] = serde_json::json!({
                            "tool": tool_name,
                            "error": format!("{e}"),
                            "latency_ms": tool_latency_ms,
                        });
                    }
                }
            }
        }

        // 2c. People search test
        log::info!("[I559] Testing people search for {}", acct);
        let people_body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "search",
                "arguments": {
                    "query": format!("people: {}", acct),
                    "maxResults": 10
                }
            }
        });

        let start = Instant::now();
        let people_result = client
            .post(&endpoint)
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .json(&people_body)
            .send()
            .await;
        let people_latency_ms = start.elapsed().as_millis();

        match people_result {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let body: serde_json::Value = resp.json().await.unwrap_or(serde_json::json!(null));
                report["sections"]["people_search"] = serde_json::json!({
                    "status": status,
                    "latency_ms": people_latency_ms,
                    "query": format!("people: {}", acct),
                    "response": body,
                });
            }
            Err(e) => {
                report["sections"]["people_search"] = serde_json::json!({
                    "error": format!("{e}"),
                    "latency_ms": people_latency_ms,
                });
            }
        }
    }

    // -----------------------------------------------------------------------
    // 3. REST API auth probe (Agents API compatibility)
    // -----------------------------------------------------------------------
    let base_url = endpoint
        .split("/mcp")
        .next()
        .unwrap_or(&endpoint)
        .to_string();

    let agents_url = format!("{}/rest/api/v1/agents", base_url);
    log::info!("[I559] Probing Agents REST API at {}", agents_url);

    let start = Instant::now();
    let agents_result = client
        .get(&agents_url)
        .bearer_auth(&token)
        .header("Accept", "application/json")
        .send()
        .await;
    let agents_latency_ms = start.elapsed().as_millis();

    match agents_result {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let body: serde_json::Value = resp.json().await.unwrap_or(serde_json::json!(null));
            report["sections"]["agents_api_probe"] = serde_json::json!({
                "url": agents_url,
                "status": status,
                "latency_ms": agents_latency_ms,
                "auth_compatible": status != 401 && status != 403,
                "response_preview": if let Some(s) = body.as_str() {
                    serde_json::Value::String(s.chars().take(500).collect())
                } else {
                    body
                },
            });
            log::info!(
                "[I559] Agents API returned {} (auth_compatible: {})",
                status,
                status != 401 && status != 403
            );
        }
        Err(e) => {
            report["sections"]["agents_api_probe"] = serde_json::json!({
                "url": agents_url,
                "error": format!("{e}"),
                "latency_ms": agents_latency_ms,
                "auth_compatible": false,
            });
        }
    }

    // -----------------------------------------------------------------------
    // Summary
    // -----------------------------------------------------------------------
    let tool_count = report["sections"]["tools_list"]["response"]["result"]["tools"]
        .as_array()
        .map(|a| a.len())
        .unwrap_or(0);

    let tool_names: Vec<String> = report["sections"]["tools_list"]["response"]["result"]["tools"]
        .as_array()
        .map(|tools| {
            tools
                .iter()
                .filter_map(|t| t["name"].as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let agents_auth = report["sections"]["agents_api_probe"]["auth_compatible"]
        .as_bool()
        .unwrap_or(false);

    report["summary"] = serde_json::json!({
        "total_mcp_tools": tool_count,
        "mcp_tool_names": tool_names,
        "agents_api_auth_compatible": agents_auth,
        "account_tested": account_name,
    });

    log::info!(
        "[I559] Exploration complete: {} MCP tools found, agents API auth={}, tools={:?}",
        tool_count,
        agents_auth,
        tool_names
    );

    Ok(report)
}

// ---------------------------------------------------------------------------
// I495 — Ephemeral Account Query via Glean
// ---------------------------------------------------------------------------

/// A one-shot briefing about an account, produced from Glean without requiring
/// the account to exist in the local database.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EphemeralBriefing {
    pub name: String,
    pub summary: String,
    pub sections: Vec<BriefingSection>,
    pub source_count: usize,
    /// If the account already exists in DailyOS, this is its entity ID.
    pub already_exists: Option<String>,
}

/// A single section within an ephemeral briefing.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BriefingSection {
    pub title: String,
    pub content: String,
    pub source: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportAccountRequest {
    pub name: String,
    pub my_role: Option<String>,
    pub evidence: Option<String>,
    pub source: Option<String>,
    pub domain: Option<String>,
    pub industry: Option<String>,
    pub context_preview: Option<String>,
    pub summary: Option<String>,
    #[serde(default)]
    pub sections: Vec<BriefingSection>,
}

fn build_seeded_account_context(request: &ImportAccountRequest) -> Option<String> {
    let mut lines = Vec::new();

    if let Some(summary) = request
        .summary
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        lines.push(summary.trim().to_string());
    }

    if let Some(preview) = request
        .context_preview
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        if lines.iter().all(|line| line != preview.trim()) {
            lines.push(preview.trim().to_string());
        }
    }

    if let Some(role) = request
        .my_role
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        lines.push(format!("My role: {}", role.trim()));
    }

    if let Some(industry) = request
        .industry
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        lines.push(format!("Industry: {}", industry.trim()));
    }

    if let Some(domain) = request
        .domain
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        lines.push(format!("Domain: {}", domain.trim().to_lowercase()));
    }

    if let Some(evidence) = request
        .evidence
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        lines.push(format!("Evidence: {}", evidence.trim()));
    }

    if let Some(source) = request
        .source
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        lines.push(format!("Source: {}", source.trim()));
    }

    for section in request
        .sections
        .iter()
        .filter(|section| !section.title.trim().is_empty() && !section.content.trim().is_empty())
    {
        if let Some(source) = section
            .source
            .as_ref()
            .filter(|value| !value.trim().is_empty())
        {
            lines.push(format!(
                "{} ({}): {}",
                section.title.trim(),
                source.trim(),
                section.content.trim()
            ));
        } else {
            lines.push(format!(
                "{}: {}",
                section.title.trim(),
                section.content.trim()
            ));
        }
    }

    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n\n"))
    }
}

async fn import_account_from_glean_internal(
    request: ImportAccountRequest,
    state: Arc<AppState>,
    priority: crate::intel_queue::IntelPriority,
) -> Result<String, String> {
    let request_for_db = request.clone();
    let state_for_db = state.clone();
    let state_for_ctx = state.clone();
    let (account_id, created_new) = state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            if let Some(existing) = db
                .get_account_by_name(&request_for_db.name)
                .map_err(|e| e.to_string())?
            {
                return Ok((existing.id, false));
            }

            let account_id = crate::services::accounts::create_account(
                &ctx,
                db,
                &state_for_db,
                &request_for_db.name,
                None,
                Some(crate::db::AccountType::Customer),
            )?;

            if let Some(domain) = request_for_db
                .domain
                .as_ref()
                .map(|value| value.trim().to_lowercase())
                .filter(|value| !value.is_empty())
            {
                crate::services::accounts::set_account_domains(&ctx, db, &account_id, &[domain])?;
            }

            let signal_payload = serde_json::json!({
                "source": request_for_db.source,
                "evidence": request_for_db.evidence,
                "myRole": request_for_db.my_role,
                "domain": request_for_db.domain,
                "industry": request_for_db.industry,
            })
            .to_string();

            crate::services::signals::emit_and_propagate(
                db,
                &state_for_db.signals.engine,
                "account",
                &account_id,
                "entity_created",
                "glean_chat",
                Some(&signal_payload),
                0.9,
            )
            .map_err(|e| format!("signal emit failed: {e}"))?;

            Ok((account_id, true))
        })
        .await?;

    if created_new {
        if let Some(seed_context) = build_seeded_account_context(&request) {
            let title = if request
                .summary
                .as_ref()
                .is_some_and(|value| !value.trim().is_empty())
            {
                "Glean briefing"
            } else {
                "Glean discovery"
            };
            crate::services::entity_context::create_entry(
                "account",
                &account_id,
                title,
                &seed_context,
                &state,
            )
            .await
            .map_err(|e| format!("Failed to seed account context: {e}"))?;
        }
    }

    // DOS-311: this path runs at Manual or Onboarding priority (see callers
    // at lines 3627 and 3918). User-facing Glean import; surface
    // EnqueueError::Paused as a retry message rather than silently dropping.
    crate::intel_queue::enqueue_user_facing(
        &state.intel_queue,
        crate::intel_queue::IntelRequest::new(account_id.clone(), "account".to_string(), priority),
    )?;

    Ok(account_id)
}

/// Query Glean for an ephemeral briefing about a named account.
///
/// Does not require the account to exist in the local database. If it does
/// exist, `already_exists` is set to the entity ID so the frontend can link
/// to the detail page instead.
#[tauri::command]
pub async fn query_ephemeral_account(
    name: String,
    state: State<'_, Arc<AppState>>,
) -> Result<EphemeralBriefing, String> {
    use crate::context_provider::glean::GleanMcpClient;
    use crate::intelligence::glean_prompts::build_ephemeral_query_prompt;

    // 1. Require a Glean endpoint
    let provider = state.context_provider();
    let endpoint = provider
        .remote_endpoint()
        .map(|s| s.to_string())
        .ok_or_else(|| "Glean not connected".to_string())?;

    // 2. Check if account already exists
    let already_exists: Option<String> =
        state.with_db_read(|db| Ok(db.get_account_by_name(&name).ok().flatten().map(|a| a.id)))?;

    // 3. Build prompt and call Glean
    let prompt = build_ephemeral_query_prompt(&name);
    let client = GleanMcpClient::new(&endpoint);
    let response_text = client
        .chat(&prompt, None)
        .await
        .map_err(|e| format!("Glean query failed for {}: {}", name, e))?;

    log::info!(
        "[I495] Ephemeral query for '{}' — {} chars response",
        name,
        response_text.len()
    );

    // 4. Parse response — try JSON first, fall back to wrapping prose
    let briefing = parse_ephemeral_response(&name, &response_text, already_exists)?;

    Ok(briefing)
}

#[tauri::command]
pub async fn import_account_from_glean(
    request: ImportAccountRequest,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    import_account_from_glean_internal(
        request,
        state.inner().clone(),
        crate::intel_queue::IntelPriority::Manual,
    )
    .await
}

/// Parse the Glean response into an EphemeralBriefing.
///
/// Tries to extract a JSON object first. If the response is prose rather than
/// JSON, wraps it in a single section rather than failing.
fn parse_ephemeral_response(
    name: &str,
    response_text: &str,
    already_exists: Option<String>,
) -> Result<EphemeralBriefing, String> {
    // Try to extract JSON object
    if let Some(json_start) = response_text.find('{') {
        // Use brace-counting extraction
        let mut depth = 0i32;
        let mut in_string = false;
        let mut escape_next = false;

        for (i, ch) in response_text[json_start..].char_indices() {
            if escape_next {
                escape_next = false;
                continue;
            }
            match ch {
                '\\' if in_string => escape_next = true,
                '"' => in_string = !in_string,
                '{' if !in_string => depth += 1,
                '}' if !in_string => {
                    depth -= 1;
                    if depth == 0 {
                        let json_text = &response_text[json_start..=json_start + i];
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_text) {
                            let summary = parsed
                                .get("summary")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();

                            let sections: Vec<BriefingSection> = parsed
                                .get("sections")
                                .and_then(|v| v.as_array())
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|s| {
                                            Some(BriefingSection {
                                                title: s.get("title")?.as_str()?.to_string(),
                                                content: s.get("content")?.as_str()?.to_string(),
                                                source: s
                                                    .get("source")
                                                    .and_then(|v| v.as_str())
                                                    .map(|s| s.to_string()),
                                            })
                                        })
                                        .collect()
                                })
                                .unwrap_or_default();

                            let source_count = parsed
                                .get("sourceCount")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(sections.len() as u64)
                                as usize;

                            return Ok(EphemeralBriefing {
                                name: name.to_string(),
                                summary,
                                sections,
                                source_count,
                                already_exists,
                            });
                        }
                        break;
                    }
                }
                _ => {}
            }
        }
    }

    // Fallback: wrap prose in a single section
    let trimmed = response_text.trim();
    if trimmed.is_empty() {
        return Err(format!("Glean returned empty response for '{}'", name));
    }

    Ok(EphemeralBriefing {
        name: name.to_string(),
        summary: trimmed.to_string(),
        sections: vec![BriefingSection {
            title: "Overview".to_string(),
            content: trimmed.to_string(),
            source: None,
        }],
        source_count: 1,
        already_exists,
    })
}

// I535 Step 9 — Discover accounts from Glean
// ---------------------------------------------------------------------------

/// Use Glean's MCP chat tool to discover accounts the user is involved with.
///
/// Returns a list of `DiscoveredAccount` items with `already_in_dailyos` set
/// to `true` for any account whose name (case-insensitive) already exists in
/// the local database.
#[tauri::command]
pub async fn discover_accounts_from_glean(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::intelligence::glean_provider::DiscoveredAccount>, String> {
    use crate::intelligence::glean_provider::GleanIntelligenceProvider;

    // 1. Require a Glean endpoint
    let provider = state.context_provider();
    let endpoint = provider
        .remote_endpoint()
        .map(|s| s.to_string())
        .ok_or_else(|| "Glean not connected".to_string())?;

    // 2. Resolve user identity from config + Google token
    let user_name = state
        .config
        .read()
        .as_ref()
        .and_then(|cfg| cfg.user_name.clone())
        .unwrap_or_default();
    let user_email = crate::google_api::token_store::peek_account_email().unwrap_or_default();

    if user_email.is_empty() {
        return Err("No user email available — sign in to Google first".to_string());
    }

    // 3. Call Glean discovery
    let glean = GleanIntelligenceProvider::new(&endpoint);
    let mut accounts = glean
        .discover_accounts(&user_email, &user_name)
        .await
        .map_err(|e| format!("Glean discovery failed: {e}"))?;

    // 4. Check each discovered account against existing DB accounts
    let existing_accounts: Vec<(crate::db::DbAccount, Vec<String>)> = state.with_db_read(|db| {
        db.get_all_accounts_with_domains(false)
            .map_err(|e| e.to_string())
    })?;

    for account in &mut accounts {
        let discovered_name = account.name.to_lowercase();
        let discovered_domain = account.domain.as_ref().map(|value| value.to_lowercase());

        account.already_in_dailyos = existing_accounts.iter().any(|(existing, domains)| {
            let name_matches = existing.name.eq_ignore_ascii_case(&account.name);
            let domain_matches = discovered_domain.as_ref().is_some_and(|domain| {
                domains
                    .iter()
                    .any(|existing_domain| existing_domain.eq_ignore_ascii_case(domain))
            });

            domain_matches
                || (name_matches && discovered_domain.is_none())
                || (name_matches && domains.is_empty())
                || (name_matches
                    && domains.iter().any(|existing_domain| {
                        existing_domain
                            .eq_ignore_ascii_case(discovered_domain.as_deref().unwrap_or_default())
                    }))
        }) || existing_accounts.iter().any(|(_, domains)| {
            discovered_domain.as_ref().is_some_and(|domain| {
                !discovered_name.is_empty()
                    && domains
                        .iter()
                        .any(|existing_domain| existing_domain.eq_ignore_ascii_case(domain))
            })
        });
    }

    log::info!(
        "[I535] Discovered {} accounts from Glean ({} already in DailyOS)",
        accounts.len(),
        accounts.iter().filter(|a| a.already_in_dailyos).count()
    );

    Ok(accounts)
}

// =============================================================================
// I561 — Onboarding: Three Connectors
// =============================================================================

/// Result of batch account import during onboarding.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingImportResult {
    pub created: usize,
    pub failed: Vec<String>,
}

fn completed_onboarding_dimensions(intel: &crate::intelligence::IntelligenceJson) -> u32 {
    let mut completed = 0u32;

    if intel.company_context.is_some()
        || !intel.competitive_context.is_empty()
        || !intel.strategic_priorities.is_empty()
    {
        completed += 1;
    }

    if !intel.stakeholder_insights.is_empty()
        || intel.coverage_assessment.is_some()
        || !intel.organizational_changes.is_empty()
        || !intel.internal_team.is_empty()
    {
        completed += 1;
    }

    if intel.meeting_cadence.is_some()
        || intel.email_responsiveness.is_some()
        || intel.next_meeting_readiness.is_some()
    {
        completed += 1;
    }

    if !intel.value_delivered.is_empty()
        || intel
            .success_metrics
            .as_ref()
            .is_some_and(|items| !items.is_empty())
        || intel
            .open_commitments
            .as_ref()
            .is_some_and(|items| !items.is_empty())
        || !intel.blockers.is_empty()
    {
        completed += 1;
    }

    if intel.contract_context.is_some()
        || !intel.expansion_signals.is_empty()
        || intel.agreement_outlook.is_some()
        || intel.org_health.is_some()
    {
        completed += 1;
    }

    if intel.health.is_some()
        || intel.support_health.is_some()
        || intel.product_adoption.is_some()
        || intel.nps_csat.is_some()
        || !intel.gong_call_summaries.is_empty()
    {
        completed += 1;
    }

    completed.min(6)
}

/// Batch-create accounts from onboarding discovery. Emits entity_created signal
/// for each and enqueues Glean enrichment at Onboarding priority.
#[tauri::command]
pub async fn onboarding_import_accounts(
    #[allow(unused_variables)] account_names: Option<Vec<String>>,
    accounts: Option<Vec<ImportAccountRequest>>,
    state: State<'_, Arc<AppState>>,
) -> Result<OnboardingImportResult, String> {
    let requests = accounts.unwrap_or_else(|| {
        account_names
            .unwrap_or_default()
            .into_iter()
            .map(|name| ImportAccountRequest {
                name,
                my_role: None,
                evidence: None,
                source: None,
                domain: None,
                industry: None,
                context_preview: None,
                summary: None,
                sections: Vec::new(),
            })
            .collect()
    });
    let mut created = 0usize;
    let mut failed = Vec::new();

    for request in requests {
        let display_name = request.name.clone();
        match import_account_from_glean_internal(
            request,
            state.inner().clone(),
            crate::intel_queue::IntelPriority::Onboarding,
        )
        .await
        {
            Ok(_) => {
                created += 1;
            }
            Err(e) => {
                log::warn!("[I561] Failed to create account '{}': {}", display_name, e);
                failed.push(display_name);
            }
        }
    }

    Ok(OnboardingImportResult { created, failed })
}

/// Pre-fill user profile from Glean org directory during onboarding.
/// Returns None if Glean is not connected or lookup fails.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserProfileSuggestion {
    pub name: Option<String>,
    pub title: Option<String>,
    pub department: Option<String>,
    pub company: Option<String>,
}

#[tauri::command]
pub async fn onboarding_prefill_profile(
    state: State<'_, Arc<AppState>>,
) -> Result<Option<UserProfileSuggestion>, String> {
    use crate::context_provider::glean::GleanMcpClient;

    // Require Glean endpoint
    let provider = state.context_provider();
    let endpoint = match provider.remote_endpoint() {
        Some(e) => e.to_string(),
        None => return Ok(None),
    };

    // Get user email
    let user_email = crate::google_api::token_store::peek_account_email().unwrap_or_default();
    if user_email.is_empty() {
        return Ok(None);
    }

    let client = GleanMcpClient::new(&endpoint);
    let prompt = format!(
        "Look up the employee profile for {} in the company directory. \
         Return ONLY a JSON object with these fields: \
         {{\"name\": \"...\", \"title\": \"...\", \"department\": \"...\", \"company\": \"...\"}}. \
         If a field is unknown, set it to null.",
        user_email
    );

    let response = match client.chat(&prompt, None).await {
        Ok(r) => r,
        Err(e) => {
            log::warn!("[I561] Glean profile prefill failed: {}", e);
            return Ok(None);
        }
    };

    // Extract JSON from response
    let json_text = match crate::intelligence::glean_provider::extract_json_object(&response) {
        Some(j) => j.to_string(),
        None => {
            log::warn!("[I561] No JSON in Glean profile response");
            return Ok(None);
        }
    };

    #[derive(serde::Deserialize)]
    struct ProfileResponse {
        name: Option<String>,
        title: Option<String>,
        department: Option<String>,
        company: Option<String>,
    }

    match serde_json::from_str::<ProfileResponse>(&json_text) {
        Ok(p) => Ok(Some(UserProfileSuggestion {
            name: p.name,
            title: p.title,
            department: p.department,
            company: p.company,
        })),
        Err(e) => {
            log::warn!("[I561] Failed to parse profile response: {}", e);
            Ok(None)
        }
    }
}

/// Enrichment progress for a single account during onboarding.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnrichmentProgress {
    pub entity_id: String,
    pub name: String,
    pub status: String, // "queued" | "analyzing" | "complete" | "failed"
    pub completed: u32,
    pub total: u32,
    pub stakeholder_count: usize,
    pub risk_count: usize,
}

/// Query enrichment status for recently created accounts (onboarding polling).
#[tauri::command]
pub async fn onboarding_enrichment_status(
    account_names: Vec<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<EnrichmentProgress>, String> {
    state
        .db_read(move |db| {
            let all_accounts = db.get_all_accounts().map_err(|e| e.to_string())?;
            let mut results = Vec::new();

            for account in all_accounts {
                let name_lower = account.name.to_lowercase();
                if !account_names.iter().any(|n| n.to_lowercase() == name_lower) {
                    continue;
                }

                let intel = db.get_entity_intelligence(&account.id).ok().flatten();

                let (status, completed, stakeholder_count, risk_count) = if let Some(intel) = intel
                {
                    let completed = completed_onboarding_dimensions(&intel);
                    let status = if completed >= 6 || !intel.enriched_at.trim().is_empty() {
                        "complete"
                    } else if completed > 0 {
                        "analyzing"
                    } else {
                        "queued"
                    };
                    (
                        status.to_string(),
                        completed,
                        intel.stakeholder_insights.len(),
                        intel.risks.len(),
                    )
                } else {
                    ("queued".to_string(), 0, 0, 0)
                };

                results.push(EnrichmentProgress {
                    entity_id: account.id.clone(),
                    name: account.name.clone(),
                    status,
                    completed,
                    total: 6,
                    stakeholder_count,
                    risk_count,
                });
            }

            Ok(results)
        })
        .await
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GleanTokenHealth {
    pub connected: bool,
    pub status: String,
    pub expires_at: Option<String>,
    pub expires_in_hours: Option<i64>,
}

#[tauri::command]
pub fn get_glean_token_health() -> GleanTokenHealth {
    match crate::glean::token_store::load_token() {
        Ok(token) => {
            let expiry = token
                .expiry
                .as_ref()
                .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
                .map(|value| value.with_timezone(&chrono::Utc));

            match expiry {
                Some(expiry) => {
                    let hours = (expiry - chrono::Utc::now()).num_hours();
                    let status = if hours < 0 {
                        "expired"
                    } else if hours < 24 {
                        "expiring"
                    } else {
                        "healthy"
                    };

                    GleanTokenHealth {
                        connected: true,
                        status: status.to_string(),
                        expires_at: Some(expiry.to_rfc3339()),
                        expires_in_hours: Some(hours),
                    }
                }
                None => GleanTokenHealth {
                    connected: true,
                    status: "healthy".to_string(),
                    expires_at: None,
                    expires_in_hours: None,
                },
            }
        }
        Err(_) => GleanTokenHealth {
            connected: false,
            status: "not_connected".to_string(),
            expires_at: None,
            expires_in_hours: None,
        },
    }
}
