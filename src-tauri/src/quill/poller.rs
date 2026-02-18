//! Quill polling loop — checks for new transcripts after meetings end.
//!
//! Runs as a background task, picking up pending quill_sync_state rows
//! and attempting to match + fetch transcripts from Quill via MCP.

use std::sync::Arc;
use std::time::Duration;

use chrono::{Timelike, Utc};
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
pub async fn run_quill_poller(state: Arc<AppState>, app_handle: AppHandle) {
    // 30-second startup delay to let other subsystems initialize
    tokio::time::sleep(Duration::from_secs(30)).await;

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
                tokio::time::sleep(Duration::from_secs(60)).await;
                continue;
            }
        };

        // Check work hours (reuse same pattern as google.rs)
        if !is_work_hours(&state) {
            tokio::time::sleep(Duration::from_secs(300)).await;
            continue;
        }

        // Get pending sync rows from DB
        let pending = match get_pending_syncs(&state) {
            Some(rows) => rows,
            None => {
                tokio::time::sleep(Duration::from_secs(300)).await;
                continue;
            }
        };

        let poll_interval = Duration::from_secs((config.poll_interval_minutes as u64) * 60);

        if pending.is_empty() {
            tokio::time::sleep(poll_interval).await;
            continue;
        }

        log::info!("Quill poller: processing {} pending syncs", pending.len());

        for row in pending {
            process_sync_row(&state, &app_handle, &config.bridge_path, &row).await;
            // Rate limit: 60 seconds between MCP connections
            tokio::time::sleep(Duration::from_secs(60)).await;
        }

        // Sleep between poll cycles (configurable)
        tokio::time::sleep(poll_interval).await;
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

    let search_after = (start_time - chrono::Duration::hours(12)).to_rfc3339();
    let search_before = (start_time + chrono::Duration::hours(12)).to_rfc3339();
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

    let result = {
        let db_guard = match state.db.lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        match db_guard.as_ref() {
            Some(db) => sync::process_fetched_transcript(
                db,
                &row.id,
                &calendar_event,
                &transcript,
                &workspace,
                &profile,
                ai_config.as_ref(),
            ),
            None => Err("Database not available".to_string()),
        }
    };

    match &result {
        Ok(dest) => {
            log::info!(
                "Quill sync: transcript processed for '{}' → {} ({} chars)",
                meeting.title,
                dest,
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
            meeting.account_id.as_deref(),
        );
    }
}

/// Check work hours using the same logic as google.rs.
fn is_work_hours(state: &AppState) -> bool {
    let config = state.config.read().ok().and_then(|g| g.clone());
    let (start_hour, end_hour) = match config {
        Some(cfg) => (cfg.google.work_hours_start, cfg.google.work_hours_end),
        None => (8, 18),
    };

    let now_hour = chrono::Local::now().hour();
    now_hour >= start_hour as u32 && now_hour < end_hour as u32
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
    }
}
