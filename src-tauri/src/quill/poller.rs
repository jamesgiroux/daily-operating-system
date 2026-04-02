//! Quill polling loop — checks for new transcripts after meetings end.
//!
//! Runs as a background task, picking up pending quill_sync_state rows
//! and attempting to match + fetch transcripts from Quill via MCP.

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use rusqlite::params;
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
        // Dev mode isolation: pause background processing while dev sandbox is active
        if crate::db::is_dev_db_mode() {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            continue;
        }

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
                    _ = state.integrations.quill_poller_wake.notified() => {
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
                    _ = state.integrations.quill_poller_wake.notified() => {
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
                _ = state.integrations.quill_poller_wake.notified() => {
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
        let db = match crate::db::ActionDb::open() {
            Ok(d) => d,
            Err(_) => return,
        };

        // Transition to polling state
        let _ = sync::transition_state(&db, &row.id, "polling", None, None, None, None);

        let meeting = match db.get_meeting_by_id(&row.meeting_id) {
            Ok(Some(m)) => m,
            Ok(None) => {
                log::warn!(
                    "Quill sync: meeting {} not found, abandoning",
                    row.meeting_id
                );
                let _ = sync::transition_state(
                    &db,
                    &row.id,
                    "abandoned",
                    None,
                    None,
                    None,
                    Some("Meeting not found in database"),
                );
                return;
            }
            Err(e) => {
                log::warn!(
                    "Quill sync: failed to get meeting {}: {}",
                    row.meeting_id,
                    e
                );
                let _ = sync::advance_attempt(&db, &row.id);
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
            if let Ok(db) = crate::db::ActionDb::open() {
                let _ = sync::transition_state(
                    &db,
                    &row.id,
                    "failed",
                    None,
                    None,
                    None,
                    Some(&format!("Connection failed: {}", e)),
                );
            }
            return;
        }
    };

    let search_after = (start_time - chrono::Duration::hours(12))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    let search_before = (start_time + chrono::Duration::hours(12))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();

    let quill_meetings = match client
        .search_meetings("", &search_after, &search_before)
        .await
    {
        Ok(meetings) => meetings,
        Err(e) => {
            log::warn!("Quill sync: search_meetings failed: {}", e);
            client.disconnect().await;
            if let Ok(db) = crate::db::ActionDb::open() {
                let _ = sync::advance_attempt(&db, &row.id);
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
            if let Ok(db) = crate::db::ActionDb::open() {
                let _ = sync::advance_attempt(&db, &row.id);
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
        if let Ok(db) = crate::db::ActionDb::open() {
            let _ = sync::transition_state(
                &db,
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
            if let Ok(db) = crate::db::ActionDb::open() {
                let _ = sync::transition_state(
                    &db,
                    &row.id,
                    "polling",
                    Some(&matched.quill_meeting_id),
                    Some(matched.confidence),
                    None,
                    Some(&format!("Transcript fetch failed: {}", e)),
                );
                let _ = sync::advance_attempt(&db, &row.id);
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
                if let Ok(db) = crate::db::ActionDb::open() {
                    let _ = sync::transition_state(
                        &db,
                        &row.id,
                        "failed",
                        Some(&matched.quill_meeting_id),
                        Some(matched.confidence),
                        None,
                        Some("Config not available"),
                    );
                }
                return;
            }
        }
    };

    // Transition to "processing" state
    {
        if let Ok(db) = crate::db::ActionDb::open() {
            let _ = sync::transition_state(&db, &row.id, "processing", None, None, None, None);
        }
    }
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

    // Write results + captures
    {
        if let Ok(db) = crate::db::ActionDb::open() {
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
                        let db_action = crate::db::DbAction {
                            id: format!("quill-{}-{}", row.meeting_id, i),
                            title: action.title.clone(),
                            priority: "P2".to_string(),
                            status: "suggested".to_string(),
                            created_at: now.clone(),
                            due_date: action.due_date.clone(),
                            completed_at: None,
                            account_id: meeting_account_id.clone().or_else(|| {
                                account.and_then(|a| {
                                    db.get_account_by_name(a).ok().flatten().map(|acc| acc.id)
                                })
                            }),
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
                        match db.upsert_action_if_not_completed(&db_action) {
                            Ok(()) => written += 1,
                            Err(e) => {
                                log::warn!(
                                    "Quill: failed to write action '{}': {}",
                                    db_action.title,
                                    e
                                );
                            }
                        }
                    }
                    if !tr.actions.is_empty() {
                        log::info!(
                            "Quill: wrote {}/{} suggested actions for '{}'",
                            written,
                            tr.actions.len(),
                            calendar_event.title
                        );
                    }

                    let capture_count =
                        tr.wins.len() + tr.risks.len() + tr.decisions.len() + tr.actions.len();
                    if capture_count > 0 {
                        log::info!(
                            "Quill sync: wrote {} captures for '{}'",
                            capture_count,
                            calendar_event.title
                        );
                    }

                    let _ = sync::transition_state(
                        &db,
                        &row.id,
                        "completed",
                        None,
                        None,
                        Some(dest),
                        None,
                    );
                }
                Err(error) => {
                    let _ = sync::transition_state(
                        &db,
                        &row.id,
                        "failed",
                        None,
                        None,
                        None,
                        Some(error),
                    );
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

    // Notify frontend with normalized payload (fallback to meeting ID if unavailable).
    emit_transcript_processed(state, app_handle, &row.meeting_id);

    // Send native notification on success
    if result.is_ok() {
        let _ = crate::notification::notify_transcript_ready(app_handle, &meeting.title, None, state);
    }
}

/// Get pending quill sync rows from DB.
fn get_pending_syncs(_state: &AppState) -> Option<Vec<DbQuillSyncState>> {
    let db = crate::db::ActionDb::open().ok()?;
    db.get_pending_quill_syncs().ok()
}

/// Emit transcript-processed event with full MeetingOutcomeData payload when available.
fn emit_transcript_processed(_state: &AppState, app_handle: &AppHandle, meeting_id: &str) {
    let payload = crate::db::ActionDb::open().ok().and_then(|db| {
        let meeting = db.get_meeting_by_id(meeting_id).ok()??;
        crate::services::meetings::collect_meeting_outcomes_from_db(&db, &meeting)
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

    let events = match state.calendar.events.read() {
        Ok(guard) => guard.clone(),
        Err(_) => return,
    };

    let db = match crate::db::ActionDb::open() {
        Ok(d) => d,
        Err(_) => return,
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
        if let Ok(processed) = state.capture.transcript_processed.lock() {
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

        if let Ok(_id) = sync::create_sync_for_meeting(&db, &meeting_id) {
            created += 1;
        }
    }

    if created > 0 {
        log::info!(
            "Calendar poll: created {} Quill sync rows for ended meetings",
            created
        );
        // Wake the poller immediately so it picks up new rows without waiting
        state.integrations.quill_poller_wake.notify_one();
    }
}
