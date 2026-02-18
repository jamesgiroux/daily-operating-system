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

        if pending.is_empty() {
            tokio::time::sleep(Duration::from_secs(300)).await;
            continue;
        }

        log::info!("Quill poller: processing {} pending syncs", pending.len());

        for row in pending {
            process_sync_row(&state, &app_handle, &config.bridge_path, &row).await;
            // Rate limit: 60 seconds between MCP connections
            tokio::time::sleep(Duration::from_secs(60)).await;
        }

        // Sleep 5 minutes between poll cycles
        tokio::time::sleep(Duration::from_secs(300)).await;
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

    // Step 2: Connect to Quill and search for matching meeting
    let client = QuillClient::new(bridge_path.to_string());
    if !client.bridge_exists() {
        log::warn!("Quill sync: bridge not found at {}", bridge_path);
        if let Ok(g) = state.db.lock() {
            if let Some(db) = g.as_ref() {
                let _ = sync::transition_state(
                    db, &row.id, "failed", None, None, None,
                    Some("Bridge not found"),
                );
            }
        }
        return;
    }

    let quill_meetings = match client.list_meetings().await {
        Ok(meetings) => meetings,
        Err(e) => {
            log::warn!("Quill sync: list_meetings failed: {}", e);
            if let Ok(g) = state.db.lock() {
                if let Some(db) = g.as_ref() {
                    let _ = sync::advance_attempt(db, &row.id);
                }
            }
            return;
        }
    };

    // Step 3: Match meeting using correlation algorithm
    let start_time = chrono::DateTime::parse_from_rfc3339(&meeting.start_time)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());

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

    // Step 5: Store transcript and mark completed
    {
        let db_guard = match state.db.lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        if let Some(db) = db_guard.as_ref() {
            let _ = sync::transition_state(
                db, &row.id, "processing",
                Some(&matched.quill_meeting_id),
                Some(matched.confidence),
                None,
                None,
            );

            // Store transcript metadata on the meeting row
            let now = Utc::now().to_rfc3339();
            if let Err(e) = db.update_meeting_transcript_metadata(
                &row.meeting_id,
                &format!("quill://{}", matched.quill_meeting_id),
                &now,
                None,
            ) {
                log::warn!("Quill sync: failed to store transcript metadata: {}", e);
            }

            let _ = sync::transition_state(
                db,
                &row.id,
                "completed",
                Some(&matched.quill_meeting_id),
                Some(matched.confidence),
                Some(&format!("quill://{}", matched.quill_meeting_id)),
                None,
            );

            log::info!(
                "Quill sync: completed for meeting '{}' ({} chars)",
                meeting.title,
                transcript.len()
            );
        }
    }

    // Notify frontend
    let _ = app_handle.emit("transcript-processed", &row.meeting_id);
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
