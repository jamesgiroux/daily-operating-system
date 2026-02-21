//! Granola polling loop — reads local cache file and syncs transcripts.
//!
//! Runs as a background task. Unlike Quill (which connects to an MCP server),
//! Granola reads a local JSON file so there's no connection/fetch step.
//! The state machine is mainly for tracking and retry on AI pipeline failures.

use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Emitter};

use crate::state::AppState;

use super::cache;
use super::matcher;

/// Background loop that polls the Granola cache file for new transcripts.
pub async fn run_granola_poller(state: Arc<AppState>, app_handle: AppHandle) {
    // 45-second startup delay to let other subsystems initialize
    tokio::time::sleep(Duration::from_secs(45)).await;

    loop {
        let granola_config = state
            .config
            .read()
            .ok()
            .and_then(|g| g.as_ref().map(|c| c.granola.clone()));

        let config = match granola_config {
            Some(cfg) if cfg.enabled => cfg,
            _ => {
                tokio::time::sleep(Duration::from_secs(60)).await;
                continue;
            }
        };

        let poll_interval = Duration::from_secs((config.poll_interval_minutes as u64) * 60);

        // Read and process the cache
        if let Err(e) = poll_once(&state, &app_handle, &config.cache_path) {
            log::warn!("Granola poller: {}", e);
        }

        tokio::time::sleep(poll_interval).await;
    }
}

/// Single poll cycle: read cache, match documents, sync new ones.
fn poll_once(state: &AppState, app_handle: &AppHandle, cache_path: &str) -> Result<(), String> {
    let path = std::path::Path::new(cache_path);
    if !path.exists() {
        return Ok(()); // Cache not present — Granola may not be installed
    }

    let documents = cache::read_cache(path)?;
    if documents.is_empty() {
        return Ok(());
    }

    // Get recent meetings from DB for matching (last 90 days)
    let meetings_for_matching = {
        let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
        let db = db_guard.as_ref().ok_or("Database not initialized")?;
        get_recent_meetings_for_matching(db)?
    };

    let mut synced = 0;

    for doc in &documents {
        // Match to a meetings_history row
        let match_result = matcher::match_to_meeting(doc, &meetings_for_matching);
        let matched = match match_result {
            Some(m) => m,
            None => continue,
        };

        // Check if a sync row already exists for this meeting with source='granola'
        let already_synced = {
            let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
            let db = db_guard.as_ref().ok_or("Database not initialized")?;
            db.get_quill_sync_state_by_source(&matched.meeting_id, "granola")
                .map_err(|e| e.to_string())?
                .is_some()
        };

        if already_synced {
            continue;
        }

        // Create sync row and process
        let sync_id = {
            let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
            let db = db_guard.as_ref().ok_or("Database not initialized")?;
            db.insert_quill_sync_state_with_source(&matched.meeting_id, "granola")
                .map_err(|e| e.to_string())?
        };

        // Process through the shared transcript pipeline
        let result = process_granola_document(state, &sync_id, &matched.meeting_id, &doc.content);

        match &result {
            Ok(dest) => {
                log::info!(
                    "Granola sync: processed '{}' → {} ({} chars, {:?})",
                    doc.title,
                    dest,
                    doc.content.len(),
                    matched.method,
                );
                synced += 1;
            }
            Err(e) => {
                log::warn!("Granola sync: processing failed for '{}': {}", doc.title, e);
            }
        }

        // Notify frontend
        let _ = app_handle.emit("transcript-processed", &matched.meeting_id);

        if result.is_ok() {
            let _ = crate::notification::notify_transcript_ready(
                app_handle,
                &doc.title,
                None,
            );
        }
    }

    if synced > 0 {
        log::info!("Granola poller: synced {} documents", synced);
    }

    Ok(())
}

/// Process a Granola document through the shared transcript pipeline.
fn process_granola_document(
    state: &AppState,
    sync_id: &str,
    meeting_id: &str,
    content: &str,
) -> Result<String, String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let meeting = db
        .get_meeting_by_id(meeting_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Meeting {} not found", meeting_id))?;

    let calendar_event = crate::quill::sync::db_meeting_to_calendar_event(&meeting);

    let (workspace, profile, ai_config) = {
        let config_guard = state.config.read().map_err(|_| "Lock poisoned")?;
        match config_guard.as_ref() {
            Some(cfg) => (
                std::path::PathBuf::from(&cfg.workspace_path),
                cfg.profile.clone(),
                Some(cfg.ai_models.clone()),
            ),
            None => return Err("Config not available".to_string()),
        }
    };

    crate::quill::sync::process_fetched_transcript(
        db,
        sync_id,
        &calendar_event,
        content,
        &workspace,
        &profile,
        ai_config.as_ref(),
    )
}

/// Get recent meetings (last 90 days) as (id, title, start_time) tuples for matching.
fn get_recent_meetings_for_matching(
    db: &crate::db::ActionDb,
) -> Result<Vec<(String, String, String)>, String> {
    db.get_meetings_for_transcript_matching(90)
        .map_err(|e| e.to_string())
}

/// Run a one-time backfill: match all Granola cache documents to meetings_history.
pub fn run_granola_backfill(state: &AppState) -> Result<(usize, usize), String> {
    let cache_path = state
        .config
        .read()
        .map_err(|_| "Lock poisoned")?
        .as_ref()
        .map(|c| c.granola.cache_path.clone())
        .unwrap_or_default();

    let path = std::path::Path::new(&cache_path);
    if !path.exists() {
        return Err("Granola cache file not found".to_string());
    }

    let documents = cache::read_cache(path)?;
    let eligible = documents.len();

    let meetings_for_matching = {
        let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
        let db = db_guard.as_ref().ok_or("Database not initialized")?;
        get_recent_meetings_for_matching(db)?
    };

    let mut created = 0;

    for doc in &documents {
        let match_result = matcher::match_to_meeting(doc, &meetings_for_matching);
        let matched = match match_result {
            Some(m) => m,
            None => continue,
        };

        let already_synced = {
            let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
            let db = db_guard.as_ref().ok_or("Database not initialized")?;
            db.get_quill_sync_state_by_source(&matched.meeting_id, "granola")
                .map_err(|e| e.to_string())?
                .is_some()
        };

        if already_synced {
            continue;
        }

        let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
        let db = db_guard.as_ref().ok_or("Database not initialized")?;
        if db
            .insert_quill_sync_state_with_source(&matched.meeting_id, "granola")
            .is_ok()
        {
            created += 1;
        }
    }

    Ok((created, eligible))
}
