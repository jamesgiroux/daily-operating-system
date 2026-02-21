//! Quill polling loop — checks for new transcripts after meetings end.
//!
//! Runs as a background task, picking up pending quill_sync_state rows
//! and attempting to match + fetch transcripts from Quill via MCP.

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tauri::{AppHandle, Emitter};

use crate::db::DbQuillSyncState;
use crate::state::AppState;

use super::client::QuillClient;
use super::matcher;
use super::sync;

/// Background loop that polls for pending Quill transcript syncs.
///
/// Follows the same pattern as `google::run_calendar_poller`: startup delay,
/// then loop with config checks and work-hours gating.
///
/// The poller can be woken immediately via `AppState::quill_poller_wake` when
/// new sync rows are created (e.g. after a meeting ends), so transcripts are
/// fetched without waiting for the full poll interval.
pub async fn run_quill_poller(state: Arc<AppState>, app_handle: AppHandle) {
    // 30-second startup delay to let other subsystems initialize
    tokio::time::sleep(Duration::from_secs(30)).await;

    // Short idle interval when no work is found — allows quick pickup of new
    // sync rows without relying solely on the wake signal.
    const IDLE_CHECK_SECS: u64 = 120;

    loop {
        // Check if Quill is enabled in config
        let quill_config = state
            .config
            .read()
            .ok()
            .and_then(|g| g.as_ref().map(|c| c.quill.clone()));

        let config = match quill_config {
            Some(cfg) if cfg.enabled => cfg,
            _ => {
                // Wait for enable — but wake signal can interrupt
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_secs(60)) => {}
                    _ = state.quill_poller_wake.notified() => {
                        log::info!("Quill poller: woken by signal (checking config)");
                    }
                }
                continue;
            }
        };

        // Get pending sync rows from DB
        let pending = match get_pending_syncs(&state) {
            Some(rows) => rows,
            None => {
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_secs(IDLE_CHECK_SECS)) => {}
                    _ = state.quill_poller_wake.notified() => {
                        log::info!("Quill poller: woken by signal");
                    }
                }
                continue;
            }
        };

        if pending.is_empty() {
            // Nothing to do — wait for wake signal or short idle check
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(IDLE_CHECK_SECS)) => {}
                _ = state.quill_poller_wake.notified() => {
                    log::info!("Quill poller: woken by signal (new sync row)");
                }
            }
            continue;
        }

        log::info!("Quill poller: processing {} pending syncs", pending.len());

        for row in pending {
            process_sync_row(&state, &app_handle, &config.bridge_path, &row).await;
            // Rate limit: 60 seconds between MCP connections
            tokio::time::sleep(Duration::from_secs(60)).await;
        }

        // Brief pause after processing before checking for more work
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}

/// Process a single quill_sync_state row through the state machine.
async fn process_sync_row(
    state: &AppState,
    app_handle: &AppHandle,
    bridge_path: &str,
    row: &DbQuillSyncState,
) {
    // Step 1: Get meeting details and attendee emails from DB
    let (meeting, attendee_emails) = {
        let db_guard = match state.db.lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        let db = match db_guard.as_ref() {
            Some(db) => db,
            None => return,
        };

        // Transition to polling state
        let _ = sync::transition_state(db, &row.id, "polling", None, None, None, None);

        let meeting = match db.get_meeting_by_id(&row.meeting_id) {
            Ok(Some(m)) => m,
            Ok(None) => {
                log::warn!("Quill sync: meeting {} not found, abandoning", row.meeting_id);
                let _ = sync::transition_state(
                    db, &row.id, "abandoned", None, None, None,
                    Some("Meeting not found in database"),
                );
                return;
            }
            Err(e) => {
                log::warn!("Quill sync: failed to get meeting {}: {}", row.meeting_id, e);
                let _ = sync::advance_attempt(db, &row.id);
                return;
            }
        };

        let emails: Vec<String> = db
            .get_meeting_attendees(&row.meeting_id)
            .unwrap_or_default()
            .into_iter()
            .map(|p| p.email)
            .collect();

        (meeting, emails)
    };

    // Parse meeting start time for search window calculation
    let start_time = chrono::DateTime::parse_from_rfc3339(&meeting.start_time)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());

    // Step 2: Connect to Quill and search for matching meeting (±12h window)
    let client = match QuillClient::connect(bridge_path).await {
        Ok(c) => c,
        Err(e) => {
            log::warn!("Quill sync: failed to connect: {}", e);
            if let Ok(g) = state.db.lock() {
                if let Some(db) = g.as_ref() {
                    let _ = sync::transition_state(
                        db, &row.id, "failed", None, None, None,
                        Some(&format!("Connection failed: {}", e)),
                    );
                }
            }
            return;
        }
    };

    let search_after = (start_time - chrono::Duration::hours(12)).format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let search_before = (start_time + chrono::Duration::hours(12)).format("%Y-%m-%dT%H:%M:%SZ").to_string();

    let quill_meetings = match client.search_meetings("", &search_after, &search_before).await {
        Ok(meetings) => meetings,
        Err(e) => {
            log::warn!("Quill sync: search_meetings failed: {}", e);
            client.disconnect().await;
            if let Ok(g) = state.db.lock() {
                if let Some(db) = g.as_ref() {
                    let _ = sync::advance_attempt(db, &row.id);
                }
            }
            return;
        }
    };

    // Step 3: Match meeting using correlation algorithm

    let match_result = matcher::match_meeting(
        &meeting.title,
        &start_time,
        &attendee_emails,
        &quill_meetings,
    );

    let matched = match match_result {
        Some(m) => m,
        None => {
            log::info!(
                "Quill sync: no match for meeting '{}', will retry",
                meeting.title
            );
            client.disconnect().await;
            if let Ok(g) = state.db.lock() {
                if let Some(db) = g.as_ref() {
                    let _ = sync::advance_attempt(db, &row.id);
                }
            }
            return;
        }
    };

    log::info!(
        "Quill sync: matched '{}' → quill:{} (confidence: {:.2})",
        meeting.title,
        matched.quill_meeting_id,
        matched.confidence
    );

    // Step 4: Fetch transcript
    {
        let db_guard = match state.db.lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        if let Some(db) = db_guard.as_ref() {
            let _ = sync::transition_state(
                db,
                &row.id,
                "fetching",
                Some(&matched.quill_meeting_id),
                Some(matched.confidence),
                None,
                None,
            );
        }
    }

    let transcript = match client.get_transcript(&matched.quill_meeting_id).await {
        Ok(t) => t,
        Err(e) => {
            log::warn!("Quill sync: get_transcript failed: {}", e);
            client.disconnect().await;
            if let Ok(g) = state.db.lock() {
                if let Some(db) = g.as_ref() {
                    let _ = sync::transition_state(
                        db, &row.id, "polling",
                        Some(&matched.quill_meeting_id),
                        Some(matched.confidence),
                        None,
                        Some(&format!("Transcript fetch failed: {}", e)),
                    );
                    let _ = sync::advance_attempt(db, &row.id);
                }
            }
            return;
        }
    };

    // Disconnect from Quill now that we have the transcript
    client.disconnect().await;

    // Step 5: Process transcript through AI pipeline
    let calendar_event = sync::db_meeting_to_calendar_event(&meeting);

    let (workspace, profile, ai_config) = {
        let config_guard = state.config.read().ok();
        match config_guard.as_ref().and_then(|g| g.as_ref()) {
            Some(cfg) => (
                std::path::PathBuf::from(&cfg.workspace_path),
                cfg.profile.clone(),
                Some(cfg.ai_models.clone()),
            ),
            None => {
                log::warn!("Quill sync: config not available for transcript processing");
                if let Ok(g) = state.db.lock() {
                    if let Some(db) = g.as_ref() {
                        let _ = sync::transition_state(
                            db, &row.id, "failed",
                            Some(&matched.quill_meeting_id),
                            Some(matched.confidence),
                            None,
                            Some("Config not available"),
                        );
                    }
                }
                return;
            }
        }
    };

    // Transition to "processing" state (brief lock, then release)
    {
        let db_guard = match state.db.lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        if let Some(db) = db_guard.as_ref() {
            let _ = sync::transition_state(db, &row.id, "processing", None, None, None, None);
        }
    }
    // DB lock released — run the AI pipeline WITHOUT holding the mutex.
    // This was the critical hang: the pipeline (AI calls, file I/O) ran
    // while holding db.lock(), blocking the entire app.
    let result = sync::process_fetched_transcript_without_db(
        &row.id,
        &calendar_event,
        &transcript,
        &workspace,
        &profile,
        ai_config.as_ref(),
    );

    // Re-acquire lock briefly to write results + captures
    {
        let db_guard = match state.db.lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        if let Some(db) = db_guard.as_ref() {
            match &result {
                Ok(tr) => {
                    let dest = tr.destination.as_deref().unwrap_or("");
                    let processed_at = chrono::Utc::now().to_rfc3339();
                    let _ = db.update_meeting_transcript_metadata(
                        &calendar_event.id,
                        dest,
                        &processed_at,
                        tr.summary.as_deref(),
                    );

                    // Write captures (wins, risks, decisions) that were extracted by AI
                    // but couldn't be written during pipeline (db was None).
                    let account = calendar_event.account.as_deref();
                    for win in &tr.wins {
                        let _ = db.insert_capture(
                            &calendar_event.id, &calendar_event.title,
                            account, "win", win,
                        );
                    }
                    for risk in &tr.risks {
                        let _ = db.insert_capture(
                            &calendar_event.id, &calendar_event.title,
                            account, "risk", risk,
                        );
                    }
                    for decision in &tr.decisions {
                        let _ = db.insert_capture(
                            &calendar_event.id, &calendar_event.title,
                            account, "decision", decision,
                        );
                    }

                    // Write extracted actions as proposed actions
                    let now = chrono::Utc::now().to_rfc3339();
                    for (i, action) in tr.actions.iter().enumerate() {
                        let db_action = crate::db::DbAction {
                            id: format!("quill-{}-{}", row.meeting_id, i),
                            title: action.title.clone(),
                            priority: "P2".to_string(),
                            status: "proposed".to_string(),
                            created_at: now.clone(),
                            due_date: action.due_date.clone(),
                            completed_at: None,
                            account_id: account.map(|a| {
                                // Try to resolve account name to ID
                                db.get_account_by_name(a)
                                    .ok()
                                    .flatten()
                                    .map(|acc| acc.id)
                                    .unwrap_or_default()
                            }).filter(|s| !s.is_empty()),
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

                    let capture_count = tr.wins.len() + tr.risks.len() + tr.decisions.len() + tr.actions.len();
                    if capture_count > 0 {
                        log::info!(
                            "Quill sync: wrote {} captures for '{}'",
                            capture_count, calendar_event.title
                        );
                    }

                    let _ = sync::transition_state(db, &row.id, "completed", None, None, Some(dest), None);
                }
                Err(error) => {
                    let _ = sync::transition_state(db, &row.id, "failed", None, None, None, Some(error));
                }
            }
        }
    }

    match &result {
        Ok(tr) => {
            log::info!(
                "Quill sync: transcript processed for '{}' → {} ({} chars)",
                meeting.title,
                tr.destination.as_deref().unwrap_or(""),
                transcript.len()
            );
        }
        Err(e) => {
            log::warn!(
                "Quill sync: transcript processing failed for '{}': {}",
                meeting.title,
                e
            );
        }
    }

    // Notify frontend
    let _ = app_handle.emit("transcript-processed", &row.meeting_id);

    // Send native notification on success
    if result.is_ok() {
        let _ = crate::notification::notify_transcript_ready(
            app_handle,
            &meeting.title,
            None,
        );
    }
}

/// Get pending quill sync rows from DB.
fn get_pending_syncs(state: &AppState) -> Option<Vec<DbQuillSyncState>> {
    let db_guard = state.db.lock().ok()?;
    let db = db_guard.as_ref()?;
    db.get_pending_quill_syncs().ok()
}

/// Check recently-ended meetings and create Quill sync rows for eligible ones.
///
/// Called from the calendar poller after each successful poll. Only acts
/// if Quill integration is enabled in config.
pub fn check_ended_meetings_for_sync(state: &AppState) {
    // Check if Quill is enabled
    let enabled = state
        .config
        .read()
        .ok()
        .and_then(|g| g.as_ref().map(|c| c.quill.enabled))
        .unwrap_or(false);

    if !enabled {
        return;
    }

    let events = match state.calendar_events.read() {
        Ok(guard) => guard.clone(),
        Err(_) => return,
    };

    let db_guard = match state.db.lock() {
        Ok(g) => g,
        Err(_) => return,
    };
    let db = match db_guard.as_ref() {
        Some(db) => db,
        None => return,
    };

    let mut created = 0;

    for event in &events {
        if !sync::should_sync_meeting(event) {
            continue;
        }
        if !sync::has_meeting_ended(event) {
            continue;
        }

        // Check transcript immutability — skip if already processed
        if let Ok(processed) = state.transcript_processed.lock() {
            if processed.contains_key(&event.id) {
                continue;
            }
        }

        let meeting_id = crate::workflow::deliver::meeting_primary_id(
            Some(&event.id),
            &event.title,
            &event.start.to_rfc3339(),
            event.meeting_type.as_str(),
        );

        if let Ok(_id) = sync::create_sync_for_meeting(db, &meeting_id) {
            created += 1;
        }
    }

    if created > 0 {
        log::info!(
            "Calendar poll: created {} Quill sync rows for ended meetings",
            created
        );
        // Wake the poller immediately so it picks up new rows without waiting
        state.quill_poller_wake.notify_one();
    }
}
