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

/// Accept a proposed action, moving it to pending (I256).
#[tauri::command]
pub async fn accept_proposed_action(
    id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let engine = state.signals.engine.clone();
    state
        .db_write(move |db| crate::services::actions::accept_proposed_action(db, &engine, &id))
        .await
}

/// Reject a proposed action by archiving it (I256).
#[tauri::command]
pub async fn reject_proposed_action(
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
            crate::services::actions::reject_proposed_action(db, &engine, &id, &source)
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
                &email_id,
                &commitment_text,
                action_title.as_deref(),
                entity_id.as_deref(),
                entity_type.as_deref(),
                owner.as_deref(),
                due_date.as_deref(),
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
    let state = services.state();
    state
        .db_write(|db| {
            let count = crate::services::mutations::reset_email_dismissals(db)?;
            log::info!(
                "reset_email_preferences: cleared {} dismissal records",
                count
            );
            Ok(format!("Cleared {} email dismissal records", count))
        })
        .await
}

/// Get all proposed (AI-suggested) actions (I256).
#[tauri::command]
pub async fn get_proposed_actions(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::db::DbAction>, String> {
    state
        .db_read(crate::services::actions::get_proposed_actions)
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

    let cached = state
        .calendar
        .google_auth
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or(GoogleAuthStatus::NotConfigured);

    // If cached state says not configured, re-check storage — token may have
    // been written by a script or the browser auth flow completing late.
    if matches!(cached, GoogleAuthStatus::NotConfigured) {
        let fresh = crate::state::detect_google_auth();
        if matches!(fresh, GoogleAuthStatus::Authenticated { .. }) {
            if let Ok(mut guard) = state.calendar.google_auth.lock() {
                *guard = fresh.clone();
            }
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
                if let Ok(mut guard) = state.calendar.google_auth.lock() {
                    *guard = status.clone();
                }
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
        .map_err(|_| "Lock poisoned")?
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
    if let Ok(mut audit) = state.audit_log.lock() {
        let _ = audit.append(
            "security",
            "oauth_connected",
            serde_json::json!({"provider": "google"}),
        );
    }

    // Update state
    if let Ok(mut guard) = state.calendar.google_auth.lock() {
        *guard = new_status.clone();
    }

    // Emit event
    let _ = app_handle.emit("google-auth-changed", &new_status);

    // Auto-extract domain from email (non-fatal, preserves manual overrides)
    if let Some(at_pos) = email.find('@') {
        let domain = email[at_pos + 1..].to_lowercase();
        if !domain.is_empty() {
            let _ = crate::state::create_or_update_config(&state, |config| {
                if config.user_domain.is_none() {
                    config.user_domain = Some(domain);
                }
            });
        }
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
    if let Ok(mut audit) = state.audit_log.lock() {
        let _ = audit.append(
            "security",
            "oauth_revoked",
            serde_json::json!({"provider": "google", "purge": purge_report}),
        );
    }

    let new_status = GoogleAuthStatus::NotConfigured;

    // Update state
    if let Ok(mut guard) = state.calendar.google_auth.lock() {
        *guard = new_status.clone();
    }

    // Clear calendar events
    if let Ok(mut guard) = state.calendar.events.write() {
        guard.clear();
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
    state
        .calendar
        .events
        .read()
        .map(|guard| guard.clone())
        .unwrap_or_default()
}

/// Get the currently active meeting (if any)
#[tauri::command]
pub fn get_current_meeting(state: State<'_, Arc<AppState>>) -> Option<CalendarEvent> {
    let now = chrono::Utc::now();
    state.calendar.events.read().ok().and_then(|guard| {
        guard
            .iter()
            .find(|e| e.start <= now && e.end > now && !e.is_all_day)
            .cloned()
    })
}

/// Get the next upcoming meeting
#[tauri::command]
pub fn get_next_meeting(state: State<'_, Arc<AppState>>) -> Option<CalendarEvent> {
    let now = chrono::Utc::now();
    state.calendar.events.read().ok().and_then(|guard| {
        guard
            .iter()
            .filter(|e| e.start > now && !e.is_all_day)
            .min_by_key(|e| e.start)
            .cloned()
    })
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
    if let Ok(mut guard) = state.capture.dismissed.lock() {
        guard.insert(meeting_id);
    }
    Ok(())
}

/// Get post-meeting capture settings
#[tauri::command]
pub fn get_capture_settings(state: State<'_, Arc<AppState>>) -> PostMeetingCaptureConfig {
    state
        .config
        .read()
        .ok()
        .and_then(|g| g.clone())
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
    let state = services.state();
    state
        .db_write(move |db| crate::services::mutations::update_capture_content(db, &id, &content))
        .await
}

/// Cycle an action's priority (P1→P2→P3→P1) — I45 interaction.
#[tauri::command]
pub async fn update_action_priority(
    id: String,
    priority: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    // Validate priority
    if !matches!(priority.as_str(), "P1" | "P2" | "P3") {
        return Err(format!(
            "Invalid priority: {}. Must be P1, P2, or P3.",
            priority
        ));
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
                .get_previous_meeting_for_entity(
                    &entity.id,
                    entity_type_str,
                    &meeting.start_time,
                )
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
/// transcript outcomes. Returns `None` when no frozen prep or no captures.
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
            let frozen_json = match meeting.prep_frozen_json {
                Some(ref json) if !json.is_empty() => json,
                _ => return Ok(None),
            };
            let captures = db
                .get_enriched_captures(&meeting_id)
                .map_err(|e| e.to_string())?;
            if captures.is_empty() {
                return Ok(None);
            }
            let prep_risks =
                crate::intelligence::predictions::extract_prep_risks(frozen_json);
            let prep_wins =
                crate::intelligence::predictions::extract_prep_wins(frozen_json);
            if prep_risks.is_empty() && prep_wins.is_empty() {
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
            crate::intelligence::predictions::emit_prediction_feedback(
                db, &scorecard, &meeting_id,
            );
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
