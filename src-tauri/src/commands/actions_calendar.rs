use super::*;

#[tauri::command]
pub fn list_meeting_preps(state: State<'_, Arc<AppState>>) -> Result<Vec<String>, String> {
    crate::services::meetings::list_meeting_preps(&state)
}

// =============================================================================
// SQLite Database Commands
// =============================================================================

/// Action with resolved account name for list display.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionListItem {
    #[serde(flatten)]
    pub action: crate::db::DbAction,
    pub account_name: Option<String>,
}

/// Get actions from the SQLite database for display.
///
/// Returns pending actions (within `days_ahead` window, default 7) combined
/// with recently completed actions (last 48 hours) so the UI can show both
/// active and done states. Account names are resolved from the accounts table.
#[tauri::command]
pub async fn get_actions_from_db(
    days_ahead: Option<i32>,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<ActionListItem>, String> {
    let days = days_ahead.unwrap_or(7);
    state
        .db_read(move |db| crate::services::actions::get_actions_from_db(db, days))
        .await
}

/// Mark an action as completed in the SQLite database.
///
/// Sets `status = 'completed'` and `completed_at` to the current UTC timestamp.
#[tauri::command]
pub async fn complete_action(id: String, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let engine = state.signals.engine.clone();
    state
        .db_write(move |db| crate::services::actions::complete_action(db, &engine, &id))
        .await
}

/// Reopen a completed action, setting it back to pending.
#[tauri::command]
pub async fn reopen_action(id: String, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let engine = state.signals.engine.clone();
    state
        .db_write(move |db| crate::services::actions::reopen_action(db, &engine, &id))
        .await
}

/// Accept a suggested action, moving it to pending (I256).
#[tauri::command]
pub async fn accept_suggested_action(
    id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let engine = state.signals.engine.clone();
    state
        .db_write(move |db| crate::services::actions::accept_suggested_action(db, &engine, &id))
        .await
}

/// Reject a suggested action by archiving it (I256).
#[tauri::command]
pub async fn reject_suggested_action(
    id: String,
    source: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let source = source.unwrap_or_else(|| "unknown".to_string());
    crate::util::validate_enum_string(
        source.as_str(),
        "source",
        &[
            "unknown",
            "actions_page",
            "daily_briefing",
            "meeting_detail",
        ],
    )?;
    let engine = state.signals.engine.clone();
    state
        .db_write(move |db| {
            crate::services::actions::reject_suggested_action(db, &engine, &id, &source)
        })
        .await
}

/// Dismiss a suggested action — preference-based (no quality penalty).
///
/// Pairs with `reject_suggested_action`: same archive + tombstone behavior
/// so the suggestion isn't re-proposed on next enrichment, but skips the
/// `action_rejected` signal that penalizes Bayesian source weights.
/// Used by the Work-tab "Dismiss" affordance for "I don't want this"
/// versus "Is this accurate? No" which means "this is wrong."
#[tauri::command]
pub async fn dismiss_suggested_action(
    id: String,
    source: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let source = source.unwrap_or_else(|| "unknown".to_string());
    crate::util::validate_enum_string(
        source.as_str(),
        "source",
        &[
            "unknown",
            "actions_page",
            "daily_briefing",
            "meeting_detail",
        ],
    )?;
    let engine = state.signals.engine.clone();
    state
        .db_write(move |db| {
            crate::services::actions::dismiss_suggested_action(db, &engine, &id, &source)
        })
        .await
}

// =============================================================================
// I579: Per-email triage actions
// =============================================================================

/// Archive an email — sets resolved_at locally + archives in Gmail. Returns email ID for undo.
/// Emits `emails-updated` so all pages (dashboard, emails) refresh.
#[tauri::command]
pub async fn archive_email(
    email_id: String,
    state: State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    let result = crate::services::emails::archive_email(&state, &email_id).await?;
    let _ = app_handle.emit("emails-updated", ());
    Ok(result)
}

/// Unarchive an email — clears resolved_at + moves back to Gmail inbox (undo for archive).
/// Emits `emails-updated` so all pages refresh.
#[tauri::command]
pub async fn unarchive_email(
    email_id: String,
    state: State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    crate::services::emails::unarchive_email(&state, &email_id).await?;
    let _ = app_handle.emit("emails-updated", ());
    Ok(())
}

/// DOS-242: rescue an email previously suppressed by the noise filter.
/// Clears `is_noise = 0`, causing the email to surface in inbox/Records again.
/// Emits `emails-updated` so all pages refresh.
#[tauri::command]
pub async fn unsuppress_email(
    email_id: String,
    state: State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    state
        .db_write(move |db| crate::services::emails::unsuppress_email(db, &email_id))
        .await?;
    let _ = app_handle.emit("emails-updated", ());
    Ok(())
}

/// Toggle pin on an email. Returns the new pinned state (true = pinned).
#[tauri::command]
pub async fn pin_email(
    email_id: String,
    state: State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<bool, String> {
    let engine = state.signals.engine.clone();
    let result = state
        .db_write(move |db| crate::services::emails::pin_email(db, &engine, &email_id))
        .await?;
    let _ = app_handle.emit("emails-updated", ());
    Ok(result)
}

// =============================================================================
// I580: Commitment -> Action promotion
// =============================================================================

/// Promote an email commitment to a tracked action.
/// Returns the new action ID.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn promote_commitment_to_action(
    email_id: String,
    commitment_text: String,
    action_title: Option<String>,
    entity_id: Option<String>,
    entity_type: Option<String>,
    owner: Option<String>,
    due_date: Option<String>,
    state: State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    let engine = state.signals.engine.clone();
    let action_id = state
        .db_write(move |db| {
            crate::services::emails::promote_commitment_to_action(
                db,
                &engine,
                &crate::services::emails::PromoteCommitmentParams {
                    email_id: &email_id,
                    commitment_text: &commitment_text,
                    action_title: action_title.as_deref(),
                    entity_id: entity_id.as_deref(),
                    entity_type: entity_type.as_deref(),
                    owner: owner.as_deref(),
                    due_date: due_date.as_deref(),
                },
            )
        })
        .await?;
    let _ = app_handle.emit("emails-updated", ());
    Ok(action_id)
}

// =============================================================================
// Email dismissals
// =============================================================================

/// Dismiss an email-extracted item (commitment, question, reply_needed) from
/// The Correspondent. Records the dismissal in SQLite for relevance learning.
#[tauri::command]
pub async fn dismiss_email_item(
    item_type: String,
    email_id: String,
    item_text: String,
    sender_domain: Option<String>,
    email_type: Option<String>,
    entity_id: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    state
        .db_write(move |db| {
            crate::services::emails::dismiss_email_item(
                db,
                &item_type,
                &email_id,
                &item_text,
                sender_domain.as_deref(),
                email_type.as_deref(),
                entity_id.as_deref(),
            )
        })
        .await
}

/// Get all dismissed email item keys for frontend filtering.
#[tauri::command]
pub async fn list_dismissed_email_items(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<String>, String> {
    state
        .db_read(|db| {
            db.list_dismissed_email_items()
                .map(|set| set.into_iter().collect())
                .map_err(|e| e.to_string())
        })
        .await
}

/// Reset all email dismissal learning data (I374).
/// Truncates the email_dismissals table so classification starts fresh.
#[tauri::command]
pub async fn reset_email_preferences(
    services: State<'_, crate::services::ServiceLayer>,
) -> Result<String, String> {
    let state = services.state_arc();
    let state_for_ctx = state.clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            let count = crate::services::mutations::reset_email_dismissals(&ctx, db)?;
            log::info!(
                "reset_email_preferences: cleared {} dismissal records",
                count
            );
            Ok(format!("Cleared {} email dismissal records", count))
        })
        .await
}

/// Resolve a decision: clear the needs_decision flag and emit signal (DOS-17).
#[tauri::command]
pub async fn resolve_decision(id: String, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let engine = state.signals.engine.clone();
    state
        .db_write(move |db| crate::services::actions::resolve_decision(db, &engine, &id))
        .await
}

/// Get suggested (AI-suggested) actions.
///
/// Default (`show_all` unset or false): scopes to the current user's own
/// commitments + unassigned rows based on `user_entity.name`. This is the
/// behaviour the UI should use for the main "Suggested" list — AI
/// extraction tags every speaker in a transcript as a potential owner, so
/// the unfiltered result on a real workspace is mostly other people's
/// work (observed 355 rows total, 26 actually owned by the user).
///
/// `show_all: Some(true)` returns every backlog row regardless of owner,
/// for a "Show everyone's" toggle.
#[tauri::command]
pub async fn get_suggested_actions(
    state: State<'_, Arc<AppState>>,
    show_all: Option<bool>,
) -> Result<Vec<crate::db::DbAction>, String> {
    if show_all.unwrap_or(false) {
        state
            .db_read(crate::services::actions::get_suggested_actions)
            .await
    } else {
        state
            .db_read(crate::services::actions::get_suggested_actions_for_user)
            .await
    }
}

/// DOS Work-tab Phase 3: open commitments for the Work tab Commitments chapter.
///
/// Returns rows with `action_kind = 'commitment'` AND status in
/// (backlog, unstarted, started). Sort: status ASC (backlog first), then
/// `created_at DESC`.
#[tauri::command]
pub async fn get_account_commitments(
    account_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::db::DbAction>, String> {
    state
        .db_read(move |db| crate::services::actions::get_account_commitments(db, &account_id))
        .await
}

/// DOS Work-tab Phase 3: backlog suggestions for the Work tab Suggestions chapter.
///
/// Returns `status = 'backlog'` rows for the account (any `action_kind`).
/// Backlog commitments and backlog tasks both surface as suggestions until
/// accepted (backlog → unstarted) or rejected (→ archived).
#[tauri::command]
pub async fn get_account_suggestions(
    account_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::db::DbAction>, String> {
    state
        .db_read(move |db| crate::services::actions::get_account_suggestions(db, &account_id))
        .await
}

/// DOS Work-tab Phase 3: recently landed (completed) actions for the Work
/// tab Recently landed chapter.
///
/// Returns `status = 'completed'` rows with `completed_at >= now - 30 days`
/// for the account. Sort: `completed_at DESC`. Cap 20.
#[tauri::command]
pub async fn get_account_recently_landed(
    account_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::db::DbAction>, String> {
    state
        .db_read(move |db| crate::services::actions::get_account_recently_landed(db, &account_id))
        .await
}

/// Get recent meeting history for an account from the SQLite database.
///
/// Returns meetings within `lookback_days` (default 30), limited to `limit` results (default 3).
#[tauri::command]
pub async fn get_meeting_history(
    account_id: String,
    lookback_days: Option<i32>,
    limit: Option<i32>,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::db::DbMeeting>, String> {
    let days = lookback_days.unwrap_or(30);
    let lim = limit.unwrap_or(3);
    state
        .db_read(move |db| {
            db.get_meeting_history(&account_id, days, lim)
                .map_err(|e| e.to_string())
        })
        .await
}

/// Assembled detail for a single past meeting: metadata + captures + actions.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingHistoryDetail {
    pub id: String,
    pub title: String,
    pub meeting_type: String,
    pub start_time: String,
    pub end_time: Option<String>,
    pub account_id: Option<String>,
    pub account_name: Option<String>,
    pub summary: Option<String>,
    pub attendees: Vec<String>,
    pub captures: Vec<crate::db::DbCapture>,
    pub actions: Vec<crate::db::DbAction>,
    /// Parsed prep context from enrichment (I181).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prep_context: Option<PrepContext>,
}

/// Enriched pre-meeting prep context persisted from daily briefing (I181).
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepContext {
    pub intelligence_summary: Option<String>,
    pub entity_risks: Option<Vec<serde_json::Value>>,
    pub entity_readiness: Option<Vec<String>>,
    pub talking_points: Option<Vec<String>>,
    pub recent_wins: Option<Vec<String>>,
    pub recent_win_sources: Option<Vec<SourceReference>>,
    pub proposed_agenda: Option<Vec<serde_json::Value>>,
    pub open_items: Option<Vec<serde_json::Value>>,
    pub questions: Option<Vec<String>>,
    pub stakeholder_insights: Option<Vec<serde_json::Value>>,
}

/// Get full detail for a single past meeting by ID.
///
/// Assembles the meeting row, its captures, actions, and resolves the account name.
#[tauri::command]
pub async fn get_meeting_history_detail(
    meeting_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<MeetingHistoryDetail, String> {
    crate::services::meetings::get_meeting_history_detail(&meeting_id, &state).await
}

// =============================================================================
// Meeting Search (I183)
// =============================================================================

/// A meeting search result with minimal metadata.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingSearchResult {
    pub id: String,
    pub title: String,
    pub meeting_type: String,
    pub start_time: String,
    pub account_name: Option<String>,
    pub match_snippet: Option<String>,
}

/// Search meetings by title, summary, or prep context (I183).
#[tauri::command]
pub async fn search_meetings(
    query: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<MeetingSearchResult>, String> {
    crate::services::meetings::search_meetings(&query, &state).await
}

// =============================================================================
// Action Detail
// =============================================================================

/// Enriched action with resolved account name and source meeting title.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionDetail {
    #[serde(flatten)]
    pub action: crate::db::DbAction,
    pub account_name: Option<String>,
    pub source_meeting_title: Option<String>,
}

/// Get full detail for a single action, with resolved relationships.
#[tauri::command]
pub async fn get_action_detail(
    action_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<ActionDetail, String> {
    state
        .db_read(move |db| crate::services::actions::get_action_detail(db, &action_id))
        .await
}

// =============================================================================
// Phase 3.0: Google Auth Commands
// =============================================================================

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct GoogleAuthFailedPayload {
    message: String,
}

/// Get current Google authentication status.
///
/// Re-checks persisted auth storage when cached state is NotConfigured,
/// so the UI picks up credentials written by external flows.
#[tauri::command]
pub fn get_google_auth_status(state: State<'_, Arc<AppState>>) -> GoogleAuthStatus {
    let started = std::time::Instant::now();

    // Dev override: return mocked Google auth status
    if cfg!(debug_assertions) {
        let ov = DEV_GOOGLE_OVERRIDE.load(Ordering::Relaxed);
        if ov != 0 {
            log_command_latency(
                "get_google_auth_status",
                started,
                READ_CMD_LATENCY_BUDGET_MS,
            );
            return match ov {
                1 => GoogleAuthStatus::Authenticated {
                    email: "dev@dailyos.test".to_string(),
                },
                3 => GoogleAuthStatus::TokenExpired,
                _ => GoogleAuthStatus::NotConfigured,
            };
        }
    }

    let cached = state.calendar.google_auth.lock().clone();

    // If cached state says not configured, re-check storage — token may have
    // been written by a script or the browser auth flow completing late.
    if matches!(cached, GoogleAuthStatus::NotConfigured) {
        let fresh = crate::state::detect_google_auth();
        if matches!(fresh, GoogleAuthStatus::Authenticated { .. }) {
            *state.calendar.google_auth.lock() = fresh.clone();
            log_command_latency(
                "get_google_auth_status",
                started,
                READ_CMD_LATENCY_BUDGET_MS,
            );
            return fresh;
        }
    }

    log_command_latency(
        "get_google_auth_status",
        started,
        READ_CMD_LATENCY_BUDGET_MS,
    );
    cached
}

/// Start Google OAuth flow
#[tauri::command]
pub async fn start_google_auth(
    state: State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<GoogleAuthStatus, String> {
    // Dev override: skip real OAuth flow when auth is mocked
    if cfg!(debug_assertions) {
        let ov = DEV_GOOGLE_OVERRIDE.load(Ordering::Relaxed);
        match ov {
            1 => {
                let status = GoogleAuthStatus::Authenticated {
                    email: "dev@dailyos.test".to_string(),
                };
                *state.calendar.google_auth.lock() = status.clone();
                return Ok(status);
            }
            2 => return Err("Google not configured (dev override)".into()),
            3 => return Err("Google token expired (dev override)".into()),
            _ => {} // 0 = real flow
        }
    }

    let config = state
        .config
        .read()
        .clone()
        .ok_or("No configuration loaded")?;

    let workspace_path = config.workspace_path.clone();

    // Run the native Rust OAuth flow
    let workspace = std::path::Path::new(&workspace_path);
    let email = match crate::google::start_auth(workspace).await {
        Ok(email) => email,
        Err(err) => {
            let message = err.to_string();
            let _ = app_handle.emit(
                "google-auth-failed",
                GoogleAuthFailedPayload {
                    message: message.clone(),
                },
            );
            return Err(message);
        }
    };

    let new_status = GoogleAuthStatus::Authenticated {
        email: email.clone(),
    };

    // Audit: oauth_connected
    {
        let mut audit = state.audit_log.lock();
        let _ = audit.append(
            "security",
            "oauth_connected",
            serde_json::json!({"provider": "google"}),
        );
    }

    // Update state
    *state.calendar.google_auth.lock() = new_status.clone();

    // Emit event
    let _ = app_handle.emit("google-auth-changed", &new_status);

    // Auto-extract domain from email (non-fatal, preserves manual overrides)
    let detected_domain = email
        .find('@')
        .map(|at_pos| email[at_pos + 1..].to_lowercase())
        .filter(|domain| !domain.is_empty());
    if let Err(e) = crate::state::create_or_update_config(&state, move |config| {
        config.google.enabled = true;
        if config.user_domain.is_none() {
            config.user_domain = detected_domain.clone();
        }
    }) {
        log::warn!("Google auth: failed to persist enabled config: {}", e);
    }

    Ok(new_status)
}

/// Disconnect Google account
#[tauri::command]
pub fn disconnect_google(
    state: State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    crate::google::disconnect()?;

    let purge_report = state.with_db_write(|db| {
        crate::db::data_lifecycle::purge_source(db, crate::db::data_lifecycle::DataSource::Google)
            .map_err(|e| e.to_string())
    })?;

    // Audit: oauth_revoked
    {
        let mut audit = state.audit_log.lock();
        let _ = audit.append(
            "security",
            "oauth_revoked",
            serde_json::json!({"provider": "google", "purge": purge_report}),
        );
    }

    let new_status = GoogleAuthStatus::NotConfigured;

    // Update state
    *state.calendar.google_auth.lock() = new_status.clone();

    // Clear calendar events
    state.calendar.events.write().clear();

    if let Err(e) = crate::state::create_or_update_config(&state, |config| {
        config.google.enabled = false;
    }) {
        log::warn!(
            "Google disconnect: failed to persist disabled config: {}",
            e
        );
    }

    // Emit event
    let _ = app_handle.emit("google-auth-changed", &new_status);

    Ok(())
}

// =============================================================================
// Phase 3A: Calendar Commands
// =============================================================================

/// Get calendar events from the polling cache
#[tauri::command]
pub fn get_calendar_events(state: State<'_, Arc<AppState>>) -> Vec<CalendarEvent> {
    state.calendar.events.read().clone()
}

/// Get the currently active meeting (if any)
#[tauri::command]
pub fn get_current_meeting(state: State<'_, Arc<AppState>>) -> Option<CalendarEvent> {
    let now = chrono::Utc::now();
    state
        .calendar
        .events
        .read()
        .iter()
        .find(|e| e.start <= now && e.end > now && !e.is_all_day)
        .cloned()
}

/// Get the next upcoming meeting
#[tauri::command]
pub fn get_next_meeting(state: State<'_, Arc<AppState>>) -> Option<CalendarEvent> {
    let now = chrono::Utc::now();
    state
        .calendar
        .events
        .read()
        .iter()
        .filter(|e| e.start > now && !e.is_all_day)
        .min_by_key(|e| e.start)
        .cloned()
}

// =============================================================================
// Phase 3B: Post-Meeting Capture Commands
// =============================================================================

/// Capture meeting outcomes (wins, risks, actions)
#[tauri::command]
pub async fn capture_meeting_outcome(
    outcome: CapturedOutcome,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    crate::services::meetings::capture_meeting_outcome(&outcome, &state).await
}

/// Dismiss a post-meeting capture prompt (skip)
#[tauri::command]
pub fn dismiss_meeting_prompt(
    meeting_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    state.capture.dismissed.lock().insert(meeting_id);
    Ok(())
}

/// Get post-meeting capture settings
#[tauri::command]
pub fn get_capture_settings(state: State<'_, Arc<AppState>>) -> PostMeetingCaptureConfig {
    state
        .config
        .read()
        .clone()
        .map(|c| c.post_meeting_capture)
        .unwrap_or_default()
}

/// Toggle post-meeting capture on/off
#[tauri::command]
pub fn set_capture_enabled(enabled: bool, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    crate::state::create_or_update_config(&state, |config| {
        config.post_meeting_capture.enabled = enabled;
    })?;
    Ok(())
}

/// Set post-meeting capture delay (minutes before prompt appears)
#[tauri::command]
pub fn set_capture_delay(
    delay_minutes: u32,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    crate::state::create_or_update_config(&state, |config| {
        config.post_meeting_capture.delay_minutes = delay_minutes;
    })?;
    Ok(())
}

// =============================================================================
// =============================================================================
// Transcript Intake & Meeting Outcomes (I44 / I45 / ADR-0044)
// =============================================================================

/// Attach and process a transcript for a specific meeting.
///
/// Checks immutability (one transcript per meeting), processes the transcript
/// with full meeting context via Claude, stores outcomes, and routes the file.
#[tauri::command]
pub async fn attach_meeting_transcript(
    file_path: String,
    meeting: CalendarEvent,
    state: State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<crate::types::TranscriptResult, String> {
    crate::services::meetings::attach_meeting_transcript(
        file_path,
        meeting,
        state.inner(),
        app_handle,
    )
    .await
}

/// Attach a transcript by raw text instead of a file path.
///
/// Writes the pasted content to `{app_data_dir}/transcripts/pasted/{meeting_id}_{ts}.{ext}`
/// and routes through `attach_meeting_transcript`, so processing, captures,
/// and entity-linking all match the file-upload path. `format` controls the
/// extension ("md" or "txt") so downstream parsers can decide whether to
/// strip markdown formatting; the actual transcript text is always written
/// as UTF-8.
#[tauri::command]
pub async fn attach_meeting_transcript_text(
    text: String,
    format: Option<String>,
    meeting: CalendarEvent,
    state: State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<crate::types::TranscriptResult, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("Pasted transcript is empty".to_string());
    }

    let ext = match format.as_deref() {
        Some("md") | Some("markdown") => "md",
        _ => "txt",
    };

    // Resolve {app_data_dir}/transcripts/pasted/. Ensure directory exists.
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Could not resolve app data dir: {e}"))?;
    let pasted_dir = app_data_dir.join("transcripts").join("pasted");
    std::fs::create_dir_all(&pasted_dir)
        .map_err(|e| format!("Could not create pasted-transcript dir: {e}"))?;

    let safe_meeting = meeting
        .id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>();
    let ts = chrono::Utc::now().format("%Y%m%dT%H%M%S").to_string();
    let path = pasted_dir.join(format!("{}_{}.{}", safe_meeting, ts, ext));

    std::fs::write(&path, &text).map_err(|e| format!("Could not write pasted transcript: {e}"))?;

    let path_str = path
        .to_str()
        .ok_or_else(|| "pasted transcript path is not valid UTF-8".to_string())?
        .to_string();

    crate::services::meetings::attach_meeting_transcript(
        path_str,
        meeting,
        state.inner(),
        app_handle,
    )
    .await
}

/// Reprocess an already-attached transcript: clear all extraction data then
/// re-run the full 3-phase pipeline as if the transcript were freshly attached.
#[tauri::command]
pub async fn reprocess_meeting_transcript(
    meeting_id: String,
    state: State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<crate::types::TranscriptResult, String> {
    crate::services::meetings::reprocess_meeting_transcript(&meeting_id, state.inner(), app_handle)
        .await
}

/// Get meeting outcomes (from transcript processing or manual capture).
///
/// Returns `None` only when no outcomes/transcript metadata exist in SQLite.
#[tauri::command]
pub async fn get_meeting_outcomes(
    meeting_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Option<crate::types::MeetingOutcomeData>, String> {
    state
        .db_read(move |db| {
            let meeting = db
                .get_meeting_intelligence_row(&meeting_id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("Meeting not found: {}", meeting_id))?;
            Ok(collect_meeting_outcomes_from_db(db, &meeting))
        })
        .await
}

/// Get post-meeting intelligence: interaction dynamics, champion health,
/// role changes, and enriched captures (I555/I558).
#[tauri::command]
pub async fn get_meeting_post_intelligence(
    meeting_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<crate::db::types::MeetingPostIntelligence, String> {
    state
        .db_read(move |db| {
            db.get_meeting_post_intelligence(&meeting_id)
                .map_err(|e| e.to_string())
        })
        .await
}

/// Update the content of a capture (win/risk/decision) — I45 inline editing.
#[tauri::command]
pub async fn update_capture(
    id: String,
    content: String,
    services: State<'_, crate::services::ServiceLayer>,
) -> Result<(), String> {
    let state = services.state_arc();
    let state_for_ctx = state.clone();
    state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            crate::services::mutations::update_capture_content(&ctx, db, &id, &content)
        })
        .await
}

/// Cycle an action's priority (P1→P2→P3→P1) — I45 interaction.
#[tauri::command]
pub async fn update_action_priority(
    id: String,
    priority: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    // Validate priority (0-4 integer)
    let pv: i32 = priority
        .parse()
        .map_err(|_| format!("Invalid priority: {priority}. Must be 0-4."))?;
    if !(0..=4).contains(&pv) {
        return Err(format!("Invalid priority: {pv}. Must be 0-4."));
    }
    let engine = state.signals.engine.clone();
    state
        .db_write(move |db| {
            crate::services::actions::update_action_priority(db, &engine, &id, &priority)
        })
        .await
}

// =============================================================================
// Manual Action CRUD (I127 / I128)
// =============================================================================

/// Create a new action manually (not from briefing/transcript/inbox).
///
/// Returns the new action's UUID. Priority defaults to P2 if not provided.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateActionRequest {
    pub title: String,
    pub priority: Option<String>,
    pub due_date: Option<String>,
    pub account_id: Option<String>,
    pub project_id: Option<String>,
    pub person_id: Option<String>,
    pub context: Option<String>,
    pub source_label: Option<String>,
    /// DOS Work-tab Phase 1: discriminator between generic tasks and AI-inferred
    /// commitments. Defaults to `task` when absent.
    #[serde(default)]
    pub action_kind: Option<String>,
}

#[tauri::command]
pub async fn create_action(
    request: CreateActionRequest,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    crate::services::actions::create_action(request, &state).await
}

/// Update arbitrary fields on an existing action (I128).
///
/// Only provided fields are updated; `None` means "don't touch".
/// To clear a nullable field, pass the corresponding `clear_*` flag.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateActionRequest {
    pub id: String,
    pub title: Option<String>,
    pub due_date: Option<String>,
    pub clear_due_date: Option<bool>,
    pub context: Option<String>,
    pub clear_context: Option<bool>,
    pub source_label: Option<String>,
    pub clear_source_label: Option<bool>,
    pub account_id: Option<String>,
    pub clear_account: Option<bool>,
    pub project_id: Option<String>,
    pub clear_project: Option<bool>,
    pub person_id: Option<String>,
    pub clear_person: Option<bool>,
    pub priority: Option<String>,
}

#[tauri::command]
pub async fn update_action(
    request: UpdateActionRequest,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    crate::services::actions::update_action(request, &state).await
}

// =============================================================================
// Meeting Intelligence (I635 + I637)
// =============================================================================

/// Get meeting-to-meeting continuity thread: what changed between this meeting
/// and the previous one with the same entity (I637).
#[tauri::command]
pub async fn get_meeting_continuity_thread(
    meeting_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Option<crate::db::types::ContinuityThread>, String> {
    state
        .db_read(move |db| {
            let meeting = match db
                .get_meeting_by_id(&meeting_id)
                .map_err(|e| e.to_string())?
            {
                Some(m) => m,
                None => return Ok(None),
            };
            let entities = db
                .get_meeting_entities(&meeting_id)
                .map_err(|e| e.to_string())?;
            let entity = match entities.first() {
                Some(e) => e,
                None => return Ok(None),
            };
            let entity_type_str = match entity.entity_type {
                crate::entity::EntityType::Account => "account",
                crate::entity::EntityType::Project => "project",
                crate::entity::EntityType::Person => "person",
                crate::entity::EntityType::Other => return Ok(None),
            };
            let prev = db
                .get_previous_meeting_for_entity(&entity.id, entity_type_str, &meeting.start_time)
                .map_err(|e| e.to_string())?;
            match prev {
                None => Ok(Some(crate::db::types::ContinuityThread {
                    previous_meeting_date: None,
                    previous_meeting_title: None,
                    entity_name: Some(entity.name.clone()),
                    actions_completed: vec![],
                    actions_open: vec![],
                    health_delta: None,
                    new_attendees: vec![],
                    is_first_meeting: true,
                })),
                Some(prev_meeting) => {
                    let mut thread = db
                        .get_continuity_thread(
                            &entity.id,
                            &meeting_id,
                            &prev_meeting.id,
                            &prev_meeting.start_time,
                            &meeting.start_time,
                        )
                        .map_err(|e| e.to_string())?;
                    thread.previous_meeting_title = Some(prev_meeting.title);
                    thread.entity_name = Some(entity.name.clone());
                    Ok(Some(thread))
                }
            }
        })
        .await
}

/// I635: Get prediction scorecard — compare pre-meeting prep predictions against
/// transcript outcomes. Returns `None` when no prep data or no captures.
///
/// ADR-0101: Reads prep via `load_meeting_prep_from_sources` (DB-first) instead
/// of parsing `prep_frozen_json` directly. Falls back to frozen JSON only when
/// the DB read model doesn't contain prep data.
#[tauri::command]
pub async fn get_prediction_scorecard(
    meeting_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Option<crate::intelligence::predictions::PredictionScorecard>, String> {
    state
        .db_read(move |db| {
            let meeting = match db
                .get_meeting_by_id(&meeting_id)
                .map_err(|e| e.to_string())?
            {
                Some(m) => m,
                None => return Ok(None),
            };

            // ADR-0101: Try DB-first struct extraction, fall back to frozen JSON
            let today_dir = std::path::PathBuf::new();
            let (prep_risks, prep_wins) = if let Some(ref prep) =
                crate::services::meetings::load_meeting_prep_from_sources(&today_dir, &meeting)
            {
                (
                    crate::intelligence::predictions::extract_prep_risks_from_struct(prep),
                    crate::intelligence::predictions::extract_prep_wins_from_struct(prep),
                )
            } else if let Some(ref frozen) = meeting.prep_frozen_json {
                if frozen.is_empty() {
                    return Ok(None);
                }
                (
                    crate::intelligence::predictions::extract_prep_risks(frozen),
                    crate::intelligence::predictions::extract_prep_wins(frozen),
                )
            } else {
                return Ok(None);
            };

            if prep_risks.is_empty() && prep_wins.is_empty() {
                return Ok(None);
            }
            let captures = db
                .get_enriched_captures(&meeting_id)
                .map_err(|e| e.to_string())?;
            if captures.is_empty() {
                return Ok(None);
            }
            let (outcome_risks, outcome_wins) =
                crate::intelligence::predictions::extract_outcome_items(&captures);
            let scorecard = crate::intelligence::predictions::compute_scorecard(
                &prep_risks,
                &prep_wins,
                &outcome_risks,
                &outcome_wins,
            );
            crate::intelligence::predictions::emit_prediction_feedback(db, &scorecard, &meeting_id);
            if scorecard.has_data {
                Ok(Some(scorecard))
            } else {
                Ok(None)
            }
        })
        .await
}

// =============================================================================
// Processing History (I6)
// =============================================================================
