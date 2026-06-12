//! Granola polling loop — reads Granola companion IPC or local cache and syncs transcripts.
//!
//! Runs as a background task. The preferred source is Granola's companion IPC
//! bridge; the legacy plaintext JSON cache remains as a fallback for older
//! Granola installs. The state machine is mainly for tracking and retry on AI
//! pipeline failures.

use std::sync::Arc;
use std::time::Duration;

use chrono::{NaiveDateTime, Utc};
use rusqlite::params;
use tauri::{AppHandle, Emitter};

use crate::state::AppState;

use super::cache;
use super::companion;
use super::matcher;

const COMPANION_SCAN_DAYS_BACK: i32 = 90;

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

        let granola_config = state.config.read().as_ref().map(|c| c.granola.clone());

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

        match poll_once_prefer_companion(&state, &app_handle, &config) {
            Ok(events) => {
                // Re-run entity linking with the post-transcript context for
                // each meeting we successfully ingested. The calendar poller
                // already ran the engine when the meeting was created, but
                // the transcript may refine attendees/account inference.
                let clock = crate::services::context::SystemClock;
                let rng = crate::services::context::SystemRng;
                let ext = crate::services::context::ExternalClients::default();
                let svc_ctx =
                    crate::services::context::ServiceContext::new_live(&clock, &rng, &ext);
                for event in events {
                    if let Err(e) =
                        crate::services::entity_linking::calendar_adapter::evaluate_meeting(
                            &svc_ctx,
                            state.clone(),
                            &event,
                            crate::services::entity_linking::Trigger::TranscriptIngest,
                        )
                        .await
                    {
                        log::warn!(
                            "entity_linking after Granola ingest failed (non-fatal) for {}: {}",
                            event.id,
                            e
                        );
                    }
                }
            }
            Err(e) => log::warn!("Granola poller: {}", e),
        }

        tokio::select! {
            _ = tokio::time::sleep(poll_interval) => {}
            _ = state.integrations.granola_poller_wake.notified() => {
                log::info!("Granola poller: woken by signal (meeting ended)");
            }
        }
    }
}

fn poll_once_prefer_companion(
    state: &AppState,
    app_handle: &AppHandle,
    config: &super::GranolaConfig,
) -> Result<Vec<crate::types::CalendarEvent>, String> {
    if companion::CompanionClient::status().available {
        match poll_once_companion(state, app_handle, COMPANION_SCAN_DAYS_BACK) {
            Ok(events) => return Ok(events),
            Err(error) => {
                log::warn!(
                    "Granola poller: companion source failed, falling back to cache: {}",
                    error
                );
            }
        }
    }

    let cache_path = super::resolve_cache_path(config).ok_or_else(|| {
        let companion_message = companion::CompanionClient::status()
            .message
            .unwrap_or_else(|| "Granola companion bridge is unavailable".to_string());
        if super::detect_encrypted_cache_path().is_some() {
            format!(
                "{companion_message}; legacy plaintext cache is unavailable or stale while encrypted cache exists"
            )
        } else {
            format!("{companion_message}; Granola cache file not found")
        }
    })?;

    poll_once(state, app_handle, &cache_path)
}

fn poll_once_companion(
    state: &AppState,
    app_handle: &AppHandle,
    days_back: i32,
) -> Result<Vec<crate::types::CalendarEvent>, String> {
    let client = companion::CompanionClient::new().map_err(|e| e.to_string())?;
    let notes = client
        .list_recent_notes(days_back)
        .map_err(|e| e.to_string())?;
    if notes.is_empty() {
        return Ok(Vec::new());
    }

    let meetings_for_matching =
        state.with_db(|db| get_recent_meetings_for_matching(db, days_back))?;

    let mut synced = 0;
    let mut linkable_events = Vec::new();

    for note in &notes {
        let match_doc = note.as_match_document();
        let Some(matched) = matcher::match_to_meeting(&match_doc, &meetings_for_matching) else {
            continue;
        };

        let doc = match client.fetch_document(note) {
            Ok(doc) => doc,
            Err(error) => {
                log::warn!(
                    "Granola companion: failed to fetch note content for '{}': {}",
                    note.title,
                    error
                );
                continue;
            }
        };

        if let Some(event) = sync_matched_document(state, app_handle, &doc, &matched)? {
            synced += 1;
            linkable_events.push(event);
        }
    }

    if synced > 0 {
        log::info!("Granola companion poller: synced {} documents", synced);
    }

    Ok(linkable_events)
}

/// Single poll cycle: read cache, match documents, sync new ones.
///
/// Returns the calendar events for meetings whose transcripts were
/// successfully ingested in this cycle, so the async caller can re-run
/// entity linking on each.
fn poll_once(
    state: &AppState,
    app_handle: &AppHandle,
    cache_path: &std::path::Path,
) -> Result<Vec<crate::types::CalendarEvent>, String> {
    let documents = cache::read_cache(cache_path)?;
    if documents.is_empty() {
        return Ok(Vec::new());
    }

    // Get recent meetings from DB for matching (last 90 days)
    let meetings_for_matching = state.with_db(|db| get_recent_meetings_for_matching(db, 90))?;

    let mut synced = 0;
    let mut linkable_events: Vec<crate::types::CalendarEvent> = Vec::new();

    for doc in &documents {
        // Match to a meetings row
        let match_result = matcher::match_to_meeting(doc, &meetings_for_matching);
        let matched = match match_result {
            Some(m) => m,
            None => continue,
        };

        if let Some(calendar_event) = sync_matched_document(state, app_handle, doc, &matched)? {
            synced += 1;
            linkable_events.push(calendar_event);
        }
    }

    if synced > 0 {
        log::info!("Granola poller: synced {} documents", synced);
    }

    Ok(linkable_events)
}

fn sync_matched_document(
    state: &AppState,
    app_handle: &AppHandle,
    doc: &cache::GranolaDocument,
    matched: &matcher::GranolaMatchResult,
) -> Result<Option<crate::types::CalendarEvent>, String> {
    let Some(sync_id) = prepare_poll_sync_id(state, &matched.meeting_id)? else {
        return Ok(None);
    };

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
        Ok((dest, _)) => {
            log::info!(
                "Granola sync: processed '{}' → {} ({} chars, {:?})",
                doc.title,
                dest,
                doc.content.len(),
                matched.method,
            );
        }
        Err(e) => {
            log::warn!("Granola sync: processing failed for '{}': {}", doc.title, e);
        }
    }

    emit_transcript_processed(state, app_handle, &matched.meeting_id);

    if let Ok((_, calendar_event)) = result {
        #[allow(
            clippy::let_underscore_must_use,
            reason = "intentional best-effort discard; preserves existing non-blocking behavior"
        )]
        let _ = crate::notification::notify_transcript_ready(app_handle, &doc.title, None, state);
        return Ok(Some(calendar_event));
    }

    Ok(None)
}

fn prepare_poll_sync_id(state: &AppState, meeting_id: &str) -> Result<Option<String>, String> {
    // Resolve sync row for this meeting/source. Unlike the previous behavior
    // (which skipped any existing row), we must resume non-completed rows so
    // app restarts don't strand pending Granola transcripts forever.
    state.with_db(|db| {
        match db
            .get_quill_sync_state_by_source(meeting_id, "granola")
            .map_err(|e| e.to_string())?
        {
            Some(existing) => {
                if !should_process_existing_sync(&existing) {
                    return Ok(None);
                }

                // Reset any stale in-flight/failed row so this poll cycle can resume it.
                if existing.state != "pending" {
                    #[allow(
                        clippy::let_underscore_must_use,
                        reason = "intentional best-effort discard; preserves existing non-blocking behavior"
                    )]
                    // dos7-allowed: transcript-db-write - transcript sync-state write; not workspace-file ingestion
                    let _ = crate::quill::sync::transition_state(
                        db,
                        &existing.id,
                        "pending",
                        None,
                        None,
                        None,
                        Some("Granola resume/retry"),
                    );
                }
                Ok(Some(existing.id))
            }
            None => db
                .insert_quill_sync_state_with_source(meeting_id, "granola")
                .map(Some)
                .map_err(|e| e.to_string()),
        }
    })
}

fn prepare_manual_sync_id(state: &AppState, meeting_id: &str) -> Result<String, String> {
    let meeting_id = meeting_id.to_string();
    state.with_db(move |db| {
        match db
            .get_quill_sync_state_by_source(&meeting_id, "granola")
            .map_err(|e| e.to_string())?
        {
            Some(existing) => {
                #[allow(
                    clippy::let_underscore_must_use,
                    reason = "intentional best-effort discard; preserves existing non-blocking behavior"
                )]
                // dos7-allowed: transcript-db-write - transcript sync-state write; not workspace-file ingestion
                let _ = crate::quill::sync::transition_state(
                    db,
                    &existing.id,
                    "pending",
                    None,
                    None,
                    None,
                    Some("Manual sync trigger"),
                );
                Ok(existing.id)
            }
            None => db
                .insert_quill_sync_state_with_source(&meeting_id, "granola")
                .map_err(|e| e.to_string()),
        }
    })
}

/// Process a Granola document through the shared transcript pipeline.
///
/// Uses three-phase lock pattern (matching Quill's approach) to avoid
/// holding the DB mutex across AI pipeline calls:
///   Phase 1 (with lock): Read meeting data, config, build calendar_event
///   Phase 2 (no lock): Run AI pipeline via process_fetched_transcript_without_db
///   Phase 3 (with lock): Write results back to DB
/// Returns (destination_path, calendar_event) on success so the async
/// caller can re-run entity linking with the post-transcript context.
fn process_granola_document(
    state: &AppState,
    sync_id: &str,
    meeting_id: &str,
    content: &str,
    content_kind: crate::processor::transcript::TranscriptContentKind,
) -> Result<(String, crate::types::CalendarEvent), String> {
    // Phase 1: Read data with lock, then drop
    let (calendar_event, workspace, profile, ai_config) = state.with_db(|db| {
        let meeting = db
            .get_meeting_by_id(meeting_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Meeting {} not found", meeting_id))?;

        let mut calendar_event = crate::quill::sync::db_meeting_to_calendar_event(&meeting);
        // Hydrate linked entities + attendees so the processor routes the
        // markdown to the right account dir and emits entity IDs in the
        // YAML frontmatter.
        crate::processor::transcript::enrich_meeting_from_db(&mut calendar_event, db);

        let (workspace, profile, ai_config) = {
            let config_guard = state.config.read();
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
        #[allow(
            clippy::let_underscore_must_use,
            reason = "intentional best-effort discard; preserves existing non-blocking behavior"
        )]
        // dos7-allowed: transcript-db-write - transcript sync-state write; not workspace-file ingestion
        let _ =
            crate::quill::sync::transition_state(db, sync_id, "processing", None, None, None, None);

        Ok((calendar_event, workspace, profile, ai_config))
    })?; // DB lock dropped

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
            let dest = state.with_db(|db| {
                let dest = tr.destination.as_deref().unwrap_or("").to_string();
                let processed_at = chrono::Utc::now().to_rfc3339();
                #[allow(
                    clippy::let_underscore_must_use,
                    reason = "intentional best-effort discard; preserves existing non-blocking behavior"
                )]
                // dos7-allowed: transcript-db-write - transcript metadata write; not workspace-file ingestion
                let _ = db.update_meeting_transcript_metadata(
                    &calendar_event.id,
                    &dest,
                    &processed_at,
                    tr.summary.as_deref(),
                );

                // Write captures (wins, risks, decisions) extracted by AI
                let meeting_account_id = resolve_meeting_account_id(db, &calendar_event.id);
                let account = calendar_event.account.as_deref();
                for win in &tr.wins {
                #[allow(
                    clippy::let_underscore_must_use,
                    reason = "intentional best-effort discard; preserves existing non-blocking behavior"
                )]
                // dos7-allowed: transcript-db-write - transcript capture write; not workspace-file ingestion
                let _ = db.insert_capture(
                    &calendar_event.id,
                    &calendar_event.title,
                    meeting_account_id.as_deref(),
                    "win",
                    win,
                );
            }
            for risk in &tr.risks {
                #[allow(
                    clippy::let_underscore_must_use,
                    reason = "intentional best-effort discard; preserves existing non-blocking behavior"
                )]
                // dos7-allowed: transcript-db-write - transcript capture write; not workspace-file ingestion
                let _ = db.insert_capture(
                    &calendar_event.id,
                    &calendar_event.title,
                    meeting_account_id.as_deref(),
                    "risk",
                    risk,
                );
            }
            for decision in &tr.decisions {
                #[allow(
                    clippy::let_underscore_must_use,
                    reason = "intentional best-effort discard; preserves existing non-blocking behavior"
                )]
                // dos7-allowed: transcript-db-write - transcript capture write; not workspace-file ingestion
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
                    priority: action
                        .priority
                        .as_deref()
                        .map(crate::action_status::migrate_priority)
                        .unwrap_or(crate::action_status::PRIORITY_MEDIUM),
                    status: crate::action_status::BACKLOG.to_string(),
                    created_at: now.clone(),
                    due_date: action.due_date.clone(),
                    completed_at: None,
                    account_id: action_account_id,
                    project_id: None,
                    source_type: Some("transcript".to_string()),
                    source_id: Some(calendar_event.id.clone()),
                    source_label: Some(calendar_event.title.clone()),
                    action_kind: crate::action_status::KIND_TASK.to_string(),
                    commitment_id: None,
                    owner_raw: None,
                    owner_entity_id: None,
                    owner_confidence: None,
                    owner_source: None,
                    trust_score: None,
                    trust_band: None,
                    commitment_source_count: None,
                    context: action.context.clone(),
                    waiting_on: None,
                    updated_at: now.clone(),
                    person_id: None,
                    account_name: None,
                    next_meeting_title: None,
                    next_meeting_start: None,
                    needs_decision: false,
                    decision_owner: None,
                    decision_stakes: None,
                    linear_identifier: None,
                    linear_url: None,
                };
                // dos7-allowed: transcript-db-write - transcript action write; not workspace-file ingestion
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
                #[allow(
                    clippy::let_underscore_must_use,
                    reason = "intentional best-effort discard; preserves existing non-blocking behavior"
                )]
                // dos7-allowed: transcript-db-write - transcript sync-state write; not workspace-file ingestion
                let _ = crate::quill::sync::transition_state(
                    db,
                    sync_id,
                    "completed",
                    None,
                    None,
                    Some(&dest),
                    None,
                );

                Ok(dest)
            })?;

            Ok((dest, calendar_event))
        }
        Err(error) => {
            #[allow(
                clippy::let_underscore_must_use,
                reason = "intentional best-effort discard; preserves existing non-blocking behavior"
            )]
            let _ = state.with_db(|db| {
                #[allow(
                    clippy::let_underscore_must_use,
                    reason = "intentional best-effort discard; preserves existing non-blocking behavior"
                )]
                // dos7-allowed: transcript-db-write - transcript sync-state write; not workspace-file ingestion
                let _ = crate::quill::sync::transition_state(
                    db,
                    sync_id,
                    "failed",
                    None,
                    None,
                    None,
                    Some(&error),
                );
                #[allow(
                    clippy::let_underscore_must_use,
                    reason = "intentional best-effort discard; preserves existing non-blocking behavior"
                )]
                // dos7-allowed: transcript-db-write - transcript sync-state write; not workspace-file ingestion
                let _ = crate::quill::sync::advance_attempt(db, sync_id);
                Ok(())
            });
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
/// Uses explicit account links through the graph-compatible meeting link view.
fn resolve_meeting_account_id(db: &crate::db::ActionDb, meeting_id: &str) -> Option<String> {
    db.conn_ref()
        .query_row(
            "SELECT me.entity_id
             FROM effective_meeting_entities me
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
fn emit_transcript_processed(state: &AppState, app_handle: &AppHandle, meeting_id: &str) {
    let meeting_id = meeting_id.to_string();
    let payload = state
        .with_db(|db| {
            Ok(db
                .get_meeting_by_id(&meeting_id)
                .ok()
                .flatten()
                .and_then(|meeting| {
                    crate::services::meetings::collect_meeting_outcomes_from_db(db, &meeting)
                }))
        })
        .ok()
        .flatten();

    match payload {
        Some(outcome) => {
            #[allow(
                clippy::let_underscore_must_use,
                reason = "intentional best-effort discard; preserves existing non-blocking behavior"
            )]
            let _ = app_handle.emit("transcript-processed", &outcome);
        }
        None => {
            #[allow(
                clippy::let_underscore_must_use,
                reason = "intentional best-effort discard; preserves existing non-blocking behavior"
            )]
            let _ = app_handle.emit("transcript-processed", &meeting_id);
        }
    }
}

/// Run a one-time backfill: match all Granola cache documents to meetings.
pub fn run_granola_backfill(state: &AppState, days_back: i32) -> Result<(usize, usize), String> {
    let granola_config = state
        .config
        .read()
        .as_ref()
        .map(|c| c.granola.clone())
        .unwrap_or_default();

    if companion::CompanionClient::status().available {
        match run_granola_companion_backfill(state, days_back) {
            Ok(result) => return Ok(result),
            Err(error) => {
                log::warn!(
                    "Granola backfill: companion source failed, falling back to cache: {}",
                    error
                );
            }
        }
    }

    let cache_path =
        super::resolve_cache_path(&granola_config).ok_or("Granola cache file not found")?;

    let documents = cache::read_cache(&cache_path)?;
    let eligible = documents.len();

    let meetings_for_matching =
        state.with_db(|db| get_recent_meetings_for_matching(db, days_back))?;

    let mut created = 0;

    for doc in &documents {
        let match_result = matcher::match_to_meeting(doc, &meetings_for_matching);
        let matched = match match_result {
            Some(m) => m,
            None => continue,
        };

        if insert_backfill_sync_state_if_missing(state, &matched.meeting_id)? {
            created += 1;
        }
    }

    Ok((created, eligible))
}

fn run_granola_companion_backfill(
    state: &AppState,
    days_back: i32,
) -> Result<(usize, usize), String> {
    let client = companion::CompanionClient::new().map_err(|e| e.to_string())?;
    let notes = client
        .list_recent_notes(days_back)
        .map_err(|e| e.to_string())?;
    let eligible = notes.len();

    let meetings_for_matching =
        state.with_db(|db| get_recent_meetings_for_matching(db, days_back))?;

    let mut created = 0;

    for note in &notes {
        let doc = note.as_match_document();
        let match_result = matcher::match_to_meeting(&doc, &meetings_for_matching);
        let matched = match match_result {
            Some(m) => m,
            None => continue,
        };

        if insert_backfill_sync_state_if_missing(state, &matched.meeting_id)? {
            created += 1;
        }
    }

    Ok((created, eligible))
}

fn insert_backfill_sync_state_if_missing(
    state: &AppState,
    meeting_id: &str,
) -> Result<bool, String> {
    let meeting_id = meeting_id.to_string();
    state.with_db(move |db| {
        if db
            .get_quill_sync_state_by_source(&meeting_id, "granola")
            .map_err(|e| e.to_string())?
            .is_some()
        {
            return Ok(false);
        }
        db.insert_quill_sync_state_with_source(&meeting_id, "granola")
            .map(|_| true)
            .map_err(|e| e.to_string())
    })
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
/// Returns the manual-sync result and, when a transcript was attached, the
/// calendar event so the async caller can re-run entity linking.
pub fn trigger_granola_sync_for_meeting(
    state: &AppState,
    app_handle: &AppHandle,
    meeting_id: &str,
    force: bool,
) -> Result<(ManualGranolaSyncResult, Option<crate::types::CalendarEvent>), String> {
    let granola_config = state
        .config
        .read()
        .as_ref()
        .map(|c| c.granola.clone())
        .unwrap_or_default();

    // Check for existing sync state
    if !force {
        let existing_sync_state = {
            let meeting_id = meeting_id.to_string();
            state.with_db(move |db| {
                db.get_quill_sync_state_by_source(&meeting_id, "granola")
                    .map_err(|e| e.to_string())
            })?
        };
        if let Some(existing) = existing_sync_state {
            match existing.state.as_str() {
                "completed" => {
                    return Ok((
                        ManualGranolaSyncResult {
                            status: ManualGranolaSyncStatus::AlreadyCompleted,
                            message: "Transcript already synced".to_string(),
                            document_title: None,
                            content_type: None,
                        },
                        None,
                    ));
                }
                "processing" | "pending" => {
                    return Ok((
                        ManualGranolaSyncResult {
                            status: ManualGranolaSyncStatus::AlreadyInProgress,
                            message: "Sync already in progress".to_string(),
                            document_title: None,
                            content_type: None,
                        },
                        None,
                    ));
                }
                _ => {} // failed/abandoned — allow retry
            }
        }
    }

    // Get meeting from DB for matching
    let meeting = {
        let meeting_id = meeting_id.to_string();
        state.with_db(move |db| {
            db.get_meeting_by_id(&meeting_id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("Meeting {} not found", meeting_id))
        })?
    };

    let meetings_for_matching = vec![(
        meeting.id.clone(),
        meeting.title.clone(),
        meeting.start_time.clone(),
    )];

    match trigger_companion_sync_for_meeting(
        state,
        app_handle,
        meeting_id,
        &meeting,
        &meetings_for_matching,
    ) {
        Ok(Some(result)) => return Ok(result),
        Ok(None) => {}
        Err(error) => {
            log::warn!(
                "Granola manual sync: companion source failed, falling back to cache: {}",
                error
            );
        }
    }

    let cache_path = super::resolve_cache_path(&granola_config).ok_or_else(|| {
        if super::detect_encrypted_cache_path().is_some() {
            "Granola companion bridge is unavailable and the legacy plaintext cache has no usable data; Granola is writing an encrypted cache that DailyOS does not read directly".to_string()
        } else {
            "Granola cache file not found".to_string()
        }
    })?;
    let documents = cache::read_cache(&cache_path)?;

    // Try to match a Granola document
    for doc in &documents {
        let match_result = matcher::match_to_meeting(doc, &meetings_for_matching);
        if let Some(matched) = match_result {
            let sync_id = prepare_manual_sync_id(state, &matched.meeting_id)?;

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
                Ok((_, calendar_event)) => {
                    emit_transcript_processed(state, app_handle, meeting_id);
                    return Ok((
                        ManualGranolaSyncResult {
                            status: ManualGranolaSyncStatus::Attached,
                            message: "Transcript synced successfully".to_string(),
                            document_title: Some(doc.title.clone()),
                            content_type: Some(doc.content_type),
                        },
                        Some(calendar_event),
                    ));
                }
                Err(e) => {
                    return Err(format!("Granola sync failed: {}", e));
                }
            }
        }
    }

    Ok((
        ManualGranolaSyncResult {
            status: ManualGranolaSyncStatus::NotFound,
            message: no_matching_granola_document_message(&documents),
            document_title: None,
            content_type: None,
        },
        None,
    ))
}

fn trigger_companion_sync_for_meeting(
    state: &AppState,
    app_handle: &AppHandle,
    meeting_id: &str,
    meeting: &crate::db::DbMeeting,
    meetings_for_matching: &[(String, String, String)],
) -> Result<Option<(ManualGranolaSyncResult, Option<crate::types::CalendarEvent>)>, String> {
    let client = match companion::CompanionClient::new() {
        Ok(client) => client,
        Err(error) if error.is_unavailable() => return Ok(None),
        Err(error) => return Err(error.to_string()),
    };

    let notes = client
        .list_notes_near(&meeting.start_time, meeting.end_time.as_deref())
        .map_err(|e| e.to_string())?;

    for note in &notes {
        let match_doc = note.as_match_document();
        let Some(matched) = matcher::match_to_meeting(&match_doc, meetings_for_matching) else {
            continue;
        };

        let doc = client.fetch_document(note).map_err(|e| e.to_string())?;
        let sync_id = prepare_manual_sync_id(state, &matched.meeting_id)?;

        let content_kind = match doc.content_type {
            cache::GranolaContentType::Transcript => {
                crate::processor::transcript::TranscriptContentKind::Transcript
            }
            cache::GranolaContentType::Notes => {
                crate::processor::transcript::TranscriptContentKind::Notes
            }
        };

        return match process_granola_document(
            state,
            &sync_id,
            meeting_id,
            &doc.content,
            content_kind,
        ) {
            Ok((_, calendar_event)) => {
                emit_transcript_processed(state, app_handle, meeting_id);
                Ok(Some((
                    ManualGranolaSyncResult {
                        status: ManualGranolaSyncStatus::Attached,
                        message: "Transcript synced successfully".to_string(),
                        document_title: Some(doc.title),
                        content_type: Some(doc.content_type),
                    },
                    Some(calendar_event),
                )))
            }
            Err(e) => Err(format!("Granola sync failed: {}", e)),
        };
    }

    Ok(None)
}

fn no_matching_granola_document_message(documents: &[cache::GranolaDocument]) -> String {
    if documents.is_empty() && super::detect_encrypted_cache_path().is_some() {
        "No matching Granola document found. Granola's plaintext cache is empty while an encrypted cache exists; enable Granola companion access so DailyOS can read current notes.".to_string()
    } else {
        "No matching Granola document found".to_string()
    }
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
