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
///
/// Uses `tokio::select!` to wake immediately via `granola_poller_wake`
/// (fired from the calendar poller when meetings end) instead of waiting
/// for the full poll interval.
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
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_secs(60)) => {}
                    _ = state.integrations.granola_poller_wake.notified() => {
                        log::info!("Granola poller: woken by signal (checking config)");
                    }
                }
                continue;
            }
        };

        let poll_interval = Duration::from_secs((config.poll_interval_minutes as u64) * 60);

        // Read and process the cache
        let cache_path = match super::resolve_cache_path(&config) {
            Some(p) => p,
            None => {
                log::debug!("Granola poller: no cache file found");
                tokio::select! {
                    _ = tokio::time::sleep(poll_interval) => {}
                    _ = state.integrations.granola_poller_wake.notified() => {
                        log::info!("Granola poller: woken by signal (meeting ended)");
                    }
                }
                continue;
            }
        };
        if let Err(e) = poll_once(&state, &app_handle, &cache_path) {
            log::warn!("Granola poller: {}", e);
        }

        tokio::select! {
            _ = tokio::time::sleep(poll_interval) => {}
            _ = state.integrations.granola_poller_wake.notified() => {
                log::info!("Granola poller: woken by signal (meeting ended)");
            }
        }
    }
}

/// Single poll cycle: read cache, match documents, sync new ones.
fn poll_once(
    state: &AppState,
    app_handle: &AppHandle,
    cache_path: &std::path::Path,
) -> Result<(), String> {
    let documents = cache::read_cache(cache_path)?;
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
            let _ = crate::notification::notify_transcript_ready(app_handle, &doc.title, None);
        }
    }

    if synced > 0 {
        log::info!("Granola poller: synced {} documents", synced);
    }

    Ok(())
}

/// Process a Granola document through the shared transcript pipeline.
///
/// Uses three-phase lock pattern (matching Quill's approach) to avoid
/// holding the DB mutex across AI pipeline calls:
///   Phase 1 (with lock): Read meeting data, config, build calendar_event
///   Phase 2 (no lock): Run AI pipeline via process_fetched_transcript_without_db
///   Phase 3 (with lock): Write results back to DB
fn process_granola_document(
    state: &AppState,
    sync_id: &str,
    meeting_id: &str,
    content: &str,
) -> Result<String, String> {
    // Phase 1: Read data with lock, then drop
    let (calendar_event, workspace, profile, ai_config) = {
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

        (calendar_event, workspace, profile, ai_config)
    }; // DB lock dropped

    // Phase 2: Run AI pipeline WITHOUT holding the DB mutex
    let result = crate::quill::sync::process_fetched_transcript_without_db(
        sync_id,
        &calendar_event,
        content,
        &workspace,
        &profile,
        ai_config.as_ref(),
    );

    // Phase 3: Re-acquire lock to write results
    match result {
        Ok(tr) => {
            let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
            let db = db_guard.as_ref().ok_or("Database not initialized")?;

            let dest = tr.destination.as_deref().unwrap_or("");
            let processed_at = chrono::Utc::now().to_rfc3339();
            let _ = db.update_meeting_transcript_metadata(
                &calendar_event.id,
                dest,
                &processed_at,
                tr.summary.as_deref(),
            );

            // Write captures (wins, risks, decisions) extracted by AI
            let account = calendar_event.account.as_deref();
            for win in &tr.wins {
                let _ = db.insert_capture(
                    &calendar_event.id,
                    &calendar_event.title,
                    account,
                    "win",
                    win,
                );
            }
            for risk in &tr.risks {
                let _ = db.insert_capture(
                    &calendar_event.id,
                    &calendar_event.title,
                    account,
                    "risk",
                    risk,
                );
            }
            for decision in &tr.decisions {
                let _ = db.insert_capture(
                    &calendar_event.id,
                    &calendar_event.title,
                    account,
                    "decision",
                    decision,
                );
            }

            // Write extracted actions as proposed actions
            let now = chrono::Utc::now().to_rfc3339();
            for (i, action) in tr.actions.iter().enumerate() {
                let db_action = crate::db::DbAction {
                    id: format!("granola-{}-{}", meeting_id, i),
                    title: action.title.clone(),
                    priority: "P2".to_string(),
                    status: "proposed".to_string(),
                    created_at: now.clone(),
                    due_date: action.due_date.clone(),
                    completed_at: None,
                    account_id: account
                        .map(|a| {
                            db.get_account_by_name(a)
                                .ok()
                                .flatten()
                                .map(|acc| acc.id)
                                .unwrap_or_default()
                        })
                        .filter(|s| !s.is_empty()),
                    project_id: None,
                    source_type: Some("transcript".to_string()),
                    source_id: Some(calendar_event.id.clone()),
                    source_label: Some(calendar_event.title.clone()),
                    context: None,
                    waiting_on: None,
                    updated_at: now.clone(),
                    person_id: None,
                    account_name: None,
                    next_meeting_title: None,
                    next_meeting_start: None,
                };
                let _ = db.upsert_action_if_not_completed(&db_action);
            }

            // Transition sync state to completed
            let _ = crate::quill::sync::transition_state(
                db,
                sync_id,
                "completed",
                None,
                None,
                Some(dest),
                None,
            );

            Ok(dest.to_string())
        }
        Err(error) => {
            if let Ok(db_guard) = state.db.lock() {
                if let Some(db) = db_guard.as_ref() {
                    let _ = crate::quill::sync::transition_state(
                        db,
                        sync_id,
                        "failed",
                        None,
                        None,
                        None,
                        Some(&error),
                    );
                }
            }
            Err(error)
        }
    }
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
    let granola_config = state
        .config
        .read()
        .map_err(|_| "Lock poisoned")?
        .as_ref()
        .map(|c| c.granola.clone())
        .unwrap_or_default();

    let cache_path =
        super::resolve_cache_path(&granola_config).ok_or("Granola cache file not found")?;

    let documents = cache::read_cache(&cache_path)?;
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
