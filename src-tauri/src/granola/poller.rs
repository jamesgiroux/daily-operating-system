//! Granola polling loop — reads local cache file and syncs transcripts.
//!
//! Runs as a background task. Unlike Quill (which connects to an MCP server),
//! Granola reads a local JSON file so there's no connection/fetch step.
//! The state machine is mainly for tracking and retry on AI pipeline failures.

use std::sync::Arc;
use std::time::Duration;

use chrono::{NaiveDateTime, Utc};
use rusqlite::params;
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
        // Dev mode isolation: pause background processing while dev sandbox is active
        if crate::db::is_dev_db_mode() {
            tokio::time::sleep(Duration::from_secs(5)).await;
            continue;
        }

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
        let db = crate::db::ActionDb::open().map_err(|e| format!("DB open failed: {e}"))?;
        get_recent_meetings_for_matching(&db, 90)?
    };

    let mut synced = 0;

    for doc in &documents {
        // Match to a meetings row
        let match_result = matcher::match_to_meeting(doc, &meetings_for_matching);
        let matched = match match_result {
            Some(m) => m,
            None => continue,
        };

        // Resolve sync row for this meeting/source. Unlike the previous behavior
        // (which skipped any existing row), we must resume non-completed rows so
        // app restarts don't strand pending Granola transcripts forever.
        let sync_id = {
            let db = crate::db::ActionDb::open().map_err(|e| format!("DB open failed: {e}"))?;

            match db
                .get_quill_sync_state_by_source(&matched.meeting_id, "granola")
                .map_err(|e| e.to_string())?
            {
                Some(existing) => {
                    if !should_process_existing_sync(&existing) {
                        continue;
                    }

                    // Reset any stale in-flight/failed row so this poll cycle can resume it.
                    if existing.state != "pending" {
                        let _ = crate::quill::sync::transition_state(
                            &db,
                            &existing.id,
                            "pending",
                            None,
                            None,
                            None,
                            Some("Granola resume/retry"),
                        );
                    }
                    existing.id
                }
                None => db
                    .insert_quill_sync_state_with_source(&matched.meeting_id, "granola")
                    .map_err(|e| e.to_string())?,
            }
        };

        // Process through the shared transcript pipeline
        let content_kind = match doc.content_type {
            cache::GranolaContentType::Transcript => {
                crate::processor::transcript::TranscriptContentKind::Transcript
            }
            cache::GranolaContentType::Notes => {
                crate::processor::transcript::TranscriptContentKind::Notes
            }
        };
        let result = process_granola_document(
            state,
            &sync_id,
            &matched.meeting_id,
            &doc.content,
            content_kind,
        );

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

        // Notify frontend with normalized payload (fallback to meeting ID if unavailable).
        emit_transcript_processed(state, app_handle, &matched.meeting_id);

        if result.is_ok() {
            let _ = crate::notification::notify_transcript_ready(app_handle, &doc.title, None, state);
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
    content_kind: crate::processor::transcript::TranscriptContentKind,
) -> Result<String, String> {
    // Phase 1: Read data with lock, then drop
    let (calendar_event, workspace, profile, ai_config) = {
        let db = crate::db::ActionDb::open().map_err(|e| format!("DB open failed: {e}"))?;

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

        // Mark as processing before leaving the DB lock. If the app exits mid-run,
        // the next poll can recover this row.
        let _ = crate::quill::sync::transition_state(
            &db,
            sync_id,
            "processing",
            None,
            None,
            None,
            None,
        );

        (calendar_event, workspace, profile, ai_config)
    }; // DB lock dropped

    // Step 2: Run AI pipeline WITHOUT holding the DB mutex
    let result = crate::quill::sync::process_fetched_transcript_without_db_with_kind(
        sync_id,
        &calendar_event,
        content,
        &workspace,
        &profile,
        ai_config.as_ref(),
        content_kind,
    );

    // Phase 3: Re-acquire lock to write results
    match result {
        Ok(tr) => {
            let db = crate::db::ActionDb::open().map_err(|e| format!("DB open failed: {e}"))?;

            let dest = tr.destination.as_deref().unwrap_or("");
            let processed_at = chrono::Utc::now().to_rfc3339();
            let _ = db.update_meeting_transcript_metadata(
                &calendar_event.id,
                dest,
                &processed_at,
                tr.summary.as_deref(),
            );

            // Write captures (wins, risks, decisions) extracted by AI
            let meeting_account_id = resolve_meeting_account_id(&db, &calendar_event.id);
            let account = calendar_event.account.as_deref();
            for win in &tr.wins {
                let _ = db.insert_capture(
                    &calendar_event.id,
                    &calendar_event.title,
                    meeting_account_id.as_deref(),
                    "win",
                    win,
                );
            }
            for risk in &tr.risks {
                let _ = db.insert_capture(
                    &calendar_event.id,
                    &calendar_event.title,
                    meeting_account_id.as_deref(),
                    "risk",
                    risk,
                );
            }
            for decision in &tr.decisions {
                let _ = db.insert_capture(
                    &calendar_event.id,
                    &calendar_event.title,
                    meeting_account_id.as_deref(),
                    "decision",
                    decision,
                );
            }

            // Write extracted actions as suggested actions
            let now = chrono::Utc::now().to_rfc3339();
            let mut written = 0usize;
            for (i, action) in tr.actions.iter().enumerate() {
                let action_account_id = action
                    .account
                    .as_deref()
                    .or(action.owner.as_deref())
                    .and_then(|candidate| {
                        db.get_account(candidate)
                            .ok()
                            .flatten()
                            .map(|account| account.id)
                            .or_else(|| {
                                db.get_account_by_name(candidate)
                                    .ok()
                                    .flatten()
                                    .map(|account| account.id)
                            })
                    })
                    .or_else(|| {
                        meeting_account_id.clone().or_else(|| {
                            account.and_then(|a| {
                                db.get_account_by_name(a).ok().flatten().map(|acc| acc.id)
                            })
                        })
                    });
                let db_action = crate::db::DbAction {
                    id: format!("granola-{}-{}", meeting_id, i),
                    title: action.title.clone(),
                    priority: action.priority.clone().unwrap_or_else(|| "P2".to_string()),
                    status: "suggested".to_string(),
                    created_at: now.clone(),
                    due_date: action.due_date.clone(),
                    completed_at: None,
                    account_id: action_account_id,
                    project_id: None,
                    source_type: Some("transcript".to_string()),
                    source_id: Some(calendar_event.id.clone()),
                    source_label: Some(calendar_event.title.clone()),
                    context: action.context.clone(),
                    waiting_on: None,
                    updated_at: now.clone(),
                    person_id: None,
                    account_name: None,
                    next_meeting_title: None,
                    next_meeting_start: None,
                };
                match db.upsert_action_if_not_completed(&db_action) {
                    Ok(()) => written += 1,
                    Err(e) => {
                        log::warn!(
                            "Granola: failed to write action '{}': {}",
                            db_action.title,
                            e
                        );
                    }
                }
            }
            if !tr.actions.is_empty() {
                log::info!(
                    "Granola: wrote {}/{} suggested actions for '{}'",
                    written,
                    tr.actions.len(),
                    calendar_event.title
                );
            }

            // Transition sync state to completed
            let _ = crate::quill::sync::transition_state(
                &db,
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
            if let Ok(db) = crate::db::ActionDb::open() {
                let _ = crate::quill::sync::transition_state(
                    &db,
                    sync_id,
                    "failed",
                    None,
                    None,
                    None,
                    Some(&error),
                );
                let _ = crate::quill::sync::advance_attempt(&db, sync_id);
            }
            Err(error)
        }
    }
}

fn should_process_existing_sync(row: &crate::db::DbQuillSyncState) -> bool {
    match row.state.as_str() {
        "completed" | "abandoned" => false,
        "pending" | "polling" | "fetching" | "processing" => true,
        "failed" => {
            if row.attempts >= row.max_attempts {
                return false;
            }
            is_retry_due(row.next_attempt_at.as_deref())
        }
        _ => false,
    }
}

fn is_retry_due(next_attempt_at: Option<&str>) -> bool {
    let Some(raw) = next_attempt_at else {
        return true;
    };

    if let Ok(dt) = NaiveDateTime::parse_from_str(raw, "%Y-%m-%d %H:%M:%S") {
        return dt <= Utc::now().naive_utc();
    }

    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(raw) {
        return dt.with_timezone(&Utc) <= Utc::now();
    }

    true
}

/// Resolve the primary account_id for a meeting.
///
/// Uses explicit account links in `meeting_entities`.
fn resolve_meeting_account_id(db: &crate::db::ActionDb, meeting_id: &str) -> Option<String> {
    db.conn_ref()
        .query_row(
            "SELECT me.entity_id
             FROM meeting_entities me
             WHERE me.meeting_id = ?1
               AND me.entity_type = 'account'
             ORDER BY me.rowid ASC
             LIMIT 1",
            params![meeting_id],
            |row| row.get::<_, String>(0),
        )
        .ok()
}

/// Get recent meetings (last 90 days) as (id, title, start_time) tuples for matching.
fn get_recent_meetings_for_matching(
    db: &crate::db::ActionDb,
    days_back: i32,
) -> Result<Vec<(String, String, String)>, String> {
    db.get_meetings_for_transcript_matching(days_back)
        .map_err(|e| e.to_string())
}

/// Emit transcript-processed event with full MeetingOutcomeData payload when available.
fn emit_transcript_processed(_state: &AppState, app_handle: &AppHandle, meeting_id: &str) {
    let payload = crate::db::ActionDb::open().ok().and_then(|db| {
        db.get_meeting_by_id(meeting_id)
            .ok()
            .flatten()
            .and_then(|meeting| {
                crate::services::meetings::collect_meeting_outcomes_from_db(&db, &meeting)
            })
    });

    match payload {
        Some(outcome) => {
            let _ = app_handle.emit("transcript-processed", &outcome);
        }
        None => {
            let _ = app_handle.emit("transcript-processed", &meeting_id.to_string());
        }
    }
}

/// Run a one-time backfill: match all Granola cache documents to meetings.
pub fn run_granola_backfill(state: &AppState, days_back: i32) -> Result<(usize, usize), String> {
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
        let db = crate::db::ActionDb::open().map_err(|e| format!("DB open failed: {e}"))?;
        get_recent_meetings_for_matching(&db, days_back)?
    };

    let mut created = 0;

    for doc in &documents {
        let match_result = matcher::match_to_meeting(doc, &meetings_for_matching);
        let matched = match match_result {
            Some(m) => m,
            None => continue,
        };

        let already_synced = {
            let db = crate::db::ActionDb::open().map_err(|e| format!("DB open failed: {e}"))?;
            db.get_quill_sync_state_by_source(&matched.meeting_id, "granola")
                .map_err(|e| e.to_string())?
                .is_some()
        };

        if already_synced {
            continue;
        }

        let db = crate::db::ActionDb::open().map_err(|e| format!("DB open failed: {e}"))?;
        if db
            .insert_quill_sync_state_with_source(&matched.meeting_id, "granola")
            .is_ok()
        {
            created += 1;
        }
    }

    Ok((created, eligible))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManualGranolaSyncStatus {
    Attached,
    NotFound,
    AlreadyInProgress,
    AlreadyCompleted,
}

#[derive(Debug, Clone)]
pub struct ManualGranolaSyncResult {
    pub status: ManualGranolaSyncStatus,
    pub message: String,
    pub document_title: Option<String>,
    pub content_type: Option<cache::GranolaContentType>,
}

/// Attempt an immediate Granola sync for a single meeting.
///
/// Unlike the background poller, this scopes matching to one meeting and returns
/// a concrete result when no Granola document is currently available.
pub fn trigger_granola_sync_for_meeting(
    state: &AppState,
    app_handle: &AppHandle,
    meeting_id: &str,
    force: bool,
) -> Result<ManualGranolaSyncResult, String> {
    let granola_config = state
        .config
        .read()
        .ok()
        .and_then(|g| g.as_ref().map(|c| c.granola.clone()))
        .unwrap_or_default();

    let cache_path =
        super::resolve_cache_path(&granola_config).ok_or("Granola cache file not found")?;
    let documents = cache::read_cache(&cache_path)?;

    // Check for existing sync state
    if !force {
        let db = crate::db::ActionDb::open().map_err(|e| format!("Database unavailable: {e}"))?;
        if let Some(existing) = db
            .get_quill_sync_state_by_source(meeting_id, "granola")
            .map_err(|e| e.to_string())?
        {
            match existing.state.as_str() {
                "completed" => {
                    return Ok(ManualGranolaSyncResult {
                        status: ManualGranolaSyncStatus::AlreadyCompleted,
                        message: "Transcript already synced".to_string(),
                        document_title: None,
                        content_type: None,
                    });
                }
                "processing" | "pending" => {
                    return Ok(ManualGranolaSyncResult {
                        status: ManualGranolaSyncStatus::AlreadyInProgress,
                        message: "Sync already in progress".to_string(),
                        document_title: None,
                        content_type: None,
                    });
                }
                _ => {} // failed/abandoned — allow retry
            }
        }
    }

    // Get meeting from DB for matching
    let db = crate::db::ActionDb::open().map_err(|e| format!("Database unavailable: {e}"))?;
    let meeting = db
        .get_meeting_by_id(meeting_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Meeting {} not found", meeting_id))?;

    let meetings_for_matching = vec![(
        meeting.id.clone(),
        meeting.title.clone(),
        meeting.start_time.clone(),
    )];

    // Try to match a Granola document
    for doc in &documents {
        let match_result = matcher::match_to_meeting(doc, &meetings_for_matching);
        if let Some(matched) = match_result {
            // Create or update sync state
            let sync_id = match db
                .get_quill_sync_state_by_source(&matched.meeting_id, "granola")
                .map_err(|e| e.to_string())?
            {
                Some(existing) => {
                    let _ = crate::quill::sync::transition_state(
                        &db,
                        &existing.id,
                        "pending",
                        None,
                        None,
                        None,
                        Some("Manual sync trigger"),
                    );
                    existing.id
                }
                None => db
                    .insert_quill_sync_state_with_source(&matched.meeting_id, "granola")
                    .map_err(|e| e.to_string())?,
            };

            let content_kind = match doc.content_type {
                cache::GranolaContentType::Transcript => {
                    crate::processor::transcript::TranscriptContentKind::Transcript
                }
                cache::GranolaContentType::Notes => {
                    crate::processor::transcript::TranscriptContentKind::Notes
                }
            };

            // Run the sync pipeline
            match process_granola_document(state, &sync_id, meeting_id, &doc.content, content_kind)
            {
                Ok(_) => {
                    emit_transcript_processed(state, app_handle, meeting_id);
                    return Ok(ManualGranolaSyncResult {
                        status: ManualGranolaSyncStatus::Attached,
                        message: "Transcript synced successfully".to_string(),
                        document_title: Some(doc.title.clone()),
                        content_type: Some(doc.content_type),
                    });
                }
                Err(e) => {
                    return Err(format!("Granola sync failed: {}", e));
                }
            }
        }
    }

    Ok(ManualGranolaSyncResult {
        status: ManualGranolaSyncStatus::NotFound,
        message: "No matching Granola document found".to_string(),
        document_title: None,
        content_type: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sync_row(
        state: &str,
        attempts: i32,
        max_attempts: i32,
        next_attempt_at: Option<&str>,
    ) -> crate::db::DbQuillSyncState {
        crate::db::DbQuillSyncState {
            id: "sync-1".to_string(),
            meeting_id: "meeting-1".to_string(),
            quill_meeting_id: None,
            state: state.to_string(),
            attempts,
            max_attempts,
            next_attempt_at: next_attempt_at.map(|s| s.to_string()),
            last_attempt_at: None,
            completed_at: None,
            error_message: None,
            match_confidence: None,
            transcript_path: None,
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
            source: "granola".to_string(),
        }
    }

    #[test]
    fn test_should_process_existing_sync_pending() {
        let row = sync_row("pending", 0, 6, None);
        assert!(should_process_existing_sync(&row));
    }

    #[test]
    fn test_should_process_existing_sync_completed_false() {
        let row = sync_row("completed", 0, 6, None);
        assert!(!should_process_existing_sync(&row));
    }

    #[test]
    fn test_should_process_existing_sync_failed_due() {
        let row = sync_row("failed", 2, 6, Some("2001-01-01 00:00:00"));
        assert!(should_process_existing_sync(&row));
    }

    #[test]
    fn test_should_process_existing_sync_failed_not_due() {
        let future = (Utc::now() + chrono::Duration::hours(1))
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();
        let row = sync_row("failed", 2, 6, Some(&future));
        assert!(!should_process_existing_sync(&row));
    }
}
