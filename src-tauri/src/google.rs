//! Google authentication and calendar polling
//!
//! - OAuth flow via native Rust (google_api::auth)
//! - Calendar polling loop: every N minutes during work hours
//! - Events stored in AppState, frontend notified via Tauri events

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use chrono::{Timelike, Utc};
use tauri::{AppHandle, Emitter};

use crate::db::DbPerson;
use crate::google_api;
use crate::people;
use crate::state::AppState;
use crate::types::{CalendarEvent, GoogleAuthStatus, MeetingType};
use crate::util::{name_from_email, org_from_email, person_id_from_email};
#[cfg(test)]
use crate::workflow::deliver::make_meeting_id;
use crate::workflow::deliver::{meeting_primary_id, write_json};

/// Run the Google OAuth flow via native Rust.
///
/// Opens the user's browser, captures the redirect, saves the token.
/// Returns the authenticated email on success.
pub async fn start_auth(workspace: &Path) -> Result<String, String> {
    google_api::auth::run_consent_flow(Some(workspace))
        .await
        .map_err(|e| format!("Google auth failed: {}", e))
}

/// Disconnect Google by clearing stored OAuth credentials.
pub fn disconnect() -> Result<(), String> {
    google_api::token_store::delete_token()
        .map_err(|e| format!("Failed to clear token storage: {}", e))?;
    Ok(())
}

/// Poll calendar events from Google via native Rust API.
///
/// Fetches events for today, classifies them using the 10-rule algorithm,
/// and converts to CalendarEvent for AppState storage.
async fn poll_calendar(state: &AppState) -> Result<Vec<CalendarEvent>, PollError> {
    let access_token = google_api::get_valid_access_token()
        .await
        .map_err(|e| match e {
            google_api::GoogleApiError::AuthExpired => PollError::AuthExpired,
            google_api::GoogleApiError::TokenNotFound(_) => PollError::AuthExpired,
            other => PollError::ApiError(other.to_string()),
        })?;

    // Fetch today's events (same day range as the Python calendar_poll.py)
    let today = Utc::now().date_naive();
    let raw_events = google_api::calendar::fetch_events(&access_token, today, today)
        .await
        .map_err(|e| match e {
            google_api::GoogleApiError::AuthExpired => PollError::AuthExpired,
            other => PollError::ApiError(other.to_string()),
        })?;

    // Build classification inputs from config + DB
    let user_domains = state
        .config
        .read()
        .ok()
        .and_then(|g| g.as_ref().map(|c| c.resolved_user_domains()))
        .unwrap_or_default();

    let entity_hints = build_entity_hints_from_state(state);

    // Save attendee display names for hygiene name resolution (I342)
    save_attendee_display_names(&raw_events, state);

    // Classify and convert (I171 + I336: multi-domain, entity-generic)
    let events: Vec<CalendarEvent> = raw_events
        .iter()
        .map(|raw| {
            let classified =
                google_api::classify::classify_meeting_multi(raw, &user_domains, &entity_hints);
            classified.to_calendar_event()
        })
        .collect();

    Ok(events)
}

/// Build entity hints from DB for meeting classification (I336).
fn build_entity_hints_from_state(state: &AppState) -> Vec<google_api::classify::EntityHint> {
    if let Ok(db_guard) = state.db.lock() {
        if let Some(db) = db_guard.as_ref() {
            return crate::helpers::build_entity_hints(db);
        }
    }
    Vec::new()
}

/// Save attendee display names from raw Google Calendar events into the DB
/// for hygiene name resolution. Upserts into `attendee_display_names`.
fn save_attendee_display_names(
    raw_events: &[google_api::calendar::GoogleCalendarEvent],
    state: &AppState,
) {
    let db_guard = match state.db.lock().ok() {
        Some(g) => g,
        None => return,
    };
    let db = match db_guard.as_ref() {
        Some(db) => db,
        None => return,
    };

    let mut saved = 0;
    for event in raw_events {
        for (email, name) in &event.attendee_names {
            // Only store names that look like real display names (contain a space, no @)
            if !name.contains(' ') || name.contains('@') {
                continue;
            }
            if db
                .conn_ref()
                .execute(
                    "INSERT INTO attendee_display_names (email, display_name, last_seen)
                     VALUES (?1, ?2, datetime('now'))
                     ON CONFLICT(email) DO UPDATE SET
                         display_name = excluded.display_name,
                         last_seen = excluded.last_seen",
                    rusqlite::params![email, name],
                )
                .is_ok()
            {
                saved += 1;
            }
        }
    }
    if saved > 0 {
        log::debug!("Calendar sync: saved {} attendee display names", saved);
    }
}

/// Calendar polling errors
enum PollError {
    AuthExpired,
    ApiError(String),
}

/// Start the calendar polling loop.
///
/// Runs as an async task — polls every N minutes during work hours.
/// Updates AppState with events and emits `calendar-updated` to the frontend.
pub async fn run_calendar_poller(state: Arc<AppState>, app_handle: AppHandle) {
    // Brief startup delay to let Google auth settle before first poll
    tokio::time::sleep(Duration::from_secs(5)).await;

    loop {
        // Check if we should poll
        if !should_poll(&state) {
            tokio::time::sleep(Duration::from_secs(30)).await;
            continue;
        }

        // Get workspace path
        let workspace = match get_workspace(&state) {
            Some(p) => p,
            None => {
                tokio::time::sleep(Duration::from_secs(30)).await;
                continue;
            }
        };

        // Poll calendar
        match poll_calendar(&state).await {
            Ok(events) => {
                // Check for new prep-eligible meetings before storing (I41)
                let new_preps = generate_preps_for_new_meetings(&events, &state, &workspace);
                if new_preps > 0 {
                    log::info!("Calendar poll: generated {} new preps", new_preps);
                }

                // Populate people from calendar attendees (I51)
                let sync_intel = populate_people_from_events(&events, &state, &workspace);

                // Trigger intelligence lifecycle for new/changed meetings (ADR-0081)
                if !sync_intel.new_meetings.is_empty() || !sync_intel.changed_meetings.is_empty() {
                    let state_clone = state.clone();
                    let new_ids = sync_intel.new_meetings;
                    let changed_ids = sync_intel.changed_meetings;
                    tokio::spawn(async move {
                        for mid in &new_ids {
                            match crate::intelligence_lifecycle::generate_meeting_intelligence(
                                &state_clone, mid, false,
                            )
                            .await
                            {
                                Ok(q) => log::info!(
                                    "Calendar poll: generated intelligence for new meeting '{}' (quality={:?})",
                                    mid, q.level
                                ),
                                Err(e) => log::warn!(
                                    "Calendar poll: intelligence generation failed for '{}': {}",
                                    mid, e
                                ),
                            }
                        }
                        for mid in &changed_ids {
                            match crate::intelligence_lifecycle::generate_meeting_intelligence(
                                &state_clone, mid, true,
                            )
                            .await
                            {
                                Ok(q) => log::info!(
                                    "Calendar poll: refreshed intelligence for changed meeting '{}' (quality={:?})",
                                    mid, q.level
                                ),
                                Err(e) => log::warn!(
                                    "Calendar poll: intelligence refresh failed for '{}': {}",
                                    mid, e
                                ),
                            }
                        }
                    });
                }

                // Detect cancelled meetings: today's DB meetings not in current poll (ADR-0081)
                detect_cancelled_meetings(&events, &state);

                if let Ok(mut guard) = state.calendar_events.write() {
                    *guard = events;
                }

                // Pre-meeting intelligence refresh (I147 — ADR-0058)
                let cfg_for_hygiene = state.config.read().ok().and_then(|g| g.clone());
                if let Ok(db_guard) = state.db.lock() {
                    if let Some(db) = db_guard.as_ref() {
                        let refreshed = crate::hygiene::check_upcoming_meeting_readiness(
                            db,
                            &state.intel_queue,
                            cfg_for_hygiene.as_ref(),
                        );
                        if !refreshed.is_empty() {
                            log::info!(
                                "Calendar poll: enqueued {} pre-meeting intelligence refreshes",
                                refreshed.len()
                            );
                        }
                    }
                }

                let _ = app_handle.emit("calendar-updated", ());

                // Check for recently-ended meetings needing Quill transcript sync
                crate::quill::poller::check_ended_meetings_for_sync(&state);

                // Notify frontend about new preps
                for _ in 0..new_preps {
                    let _ = app_handle.emit("prep-ready", ());
                }
            }
            Err(PollError::AuthExpired) => {
                log::warn!("Calendar poll: token expired");
                if let Ok(mut guard) = state.google_auth.lock() {
                    *guard = GoogleAuthStatus::TokenExpired;
                }
                let _ = app_handle.emit("google-auth-changed", GoogleAuthStatus::TokenExpired);
            }
            Err(PollError::ApiError(e)) => {
                log::warn!("Calendar poll error: {}", e);
            }
        }

        // Sleep between polls
        let interval = get_poll_interval(&state);
        tokio::time::sleep(Duration::from_secs(interval * 60)).await;
    }
}

/// Check if we should poll now (authenticated + within work hours)
fn should_poll(state: &AppState) -> bool {
    // Check auth status
    let is_authenticated = state
        .google_auth
        .lock()
        .map(|guard| matches!(*guard, GoogleAuthStatus::Authenticated { .. }))
        .unwrap_or(false);

    if !is_authenticated {
        return false;
    }

    // Check work hours
    let config = state.config.read().ok().and_then(|g| g.clone());
    let (start_hour, end_hour) = match config {
        Some(cfg) => (cfg.google.work_hours_start, cfg.google.work_hours_end),
        None => (8, 18),
    };

    let now_hour = chrono::Local::now().hour() as u8;
    now_hour >= start_hour && now_hour < end_hour
}

/// Get the poll interval in minutes from config
fn get_poll_interval(state: &AppState) -> u64 {
    state
        .config
        .read()
        .ok()
        .and_then(|g| g.clone())
        .map(|cfg| cfg.google.calendar_poll_interval_minutes as u64)
        .unwrap_or(5)
}

/// Prep-eligible meeting types (same as PREP_ELIGIBLE_TYPES + PERSON_PREP_TYPES in deliver.rs)
const PREP_ELIGIBLE_TYPES: &[MeetingType] = &[
    MeetingType::Customer,
    MeetingType::Qbr,
    MeetingType::Partnership,
    MeetingType::Internal,
    MeetingType::TeamSync,
    MeetingType::OneOnOne,
];

/// Generate lightweight prep files for new calendar events that don't already have preps.
///
/// Called after each calendar poll. Checks if prep-eligible meetings (customer, qbr, partnership)
/// have a prep JSON in `_today/data/preps/`. If not, generates a lightweight prep from
/// account data in SQLite.
fn generate_preps_for_new_meetings(
    events: &[CalendarEvent],
    state: &AppState,
    workspace: &Path,
) -> usize {
    let preps_dir = workspace.join("_today").join("data").join("preps");
    if !preps_dir.exists() {
        // No _today/data/preps/ means briefing hasn't run yet — nothing to do
        return 0;
    }

    let mut generated = 0;

    for event in events {
        // Skip non-prep-eligible types, all-day events, personal events
        if event.is_all_day || !PREP_ELIGIBLE_TYPES.contains(&event.meeting_type) {
            continue;
        }

        let meeting_type_str = event.meeting_type.as_str();

        let meeting_id = meeting_primary_id(
            Some(&event.id),
            &event.title,
            &event.start.to_rfc3339(),
            meeting_type_str,
        );

        let prep_path = preps_dir.join(format!("{}.json", meeting_id));
        if prep_path.exists() {
            continue; // Already has prep
        }

        // Also check by event ID (different meeting_id but same event)
        let already_prepped = has_existing_prep_for_event(&preps_dir, &meeting_id);
        if already_prepped {
            continue;
        }

        // Generate lightweight prep from account data in SQLite
        let mut prep = serde_json::json!({
            "meetingId": meeting_id,
            "calendarEventId": event.id,
            "title": event.title,
            "type": meeting_type_str,
            "timeRange": format!(
                "{} - {}",
                event.start.format("%-I:%M %p"),
                event.end.format("%-I:%M %p")
            ),
        });

        if let Some(ref account) = event.account {
            if let Some(obj) = prep.as_object_mut() {
                obj.insert("account".to_string(), serde_json::json!(account));
            }

            // Try to pull account data from SQLite
            if let Ok(db_guard) = state.db.lock() {
                if let Some(db) = db_guard.as_ref() {
                    enrich_prep_from_db(&mut prep, account, db);
                }
            }
        }

        match write_json(&prep_path, &prep) {
            Ok(()) => {
                log::info!(
                    "Generated reactive prep for '{}' ({})",
                    event.title,
                    meeting_id
                );
                generated += 1;
            }
            Err(e) => {
                log::warn!("Failed to write reactive prep for '{}': {}", event.title, e);
            }
        }
    }

    generated
}

/// Check if any existing prep file already covers this calendar event ID.
fn has_existing_prep_for_event(preps_dir: &Path, event_id: &str) -> bool {
    let entries = match std::fs::read_dir(preps_dir) {
        Ok(e) => e,
        Err(_) => return false,
    };

    for entry in entries.flatten() {
        if !entry
            .file_name()
            .to_str()
            .is_some_and(|n| n.ends_with(".json"))
        {
            continue;
        }
        if let Ok(content) = std::fs::read_to_string(entry.path()) {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
                let prep_event_id = data.get("calendarEventId").and_then(|v| v.as_str());
                if prep_event_id == Some(event_id)
                    || prep_event_id == Some(&event_id.replace("_at_", "@"))
                {
                    return true;
                }
            }
        }
    }
    false
}

/// Enrich a prep JSON with account data from SQLite (quick context + open actions).
fn enrich_prep_from_db(prep: &mut serde_json::Value, account_id: &str, db: &crate::db::ActionDb) {
    // Quick context from account data
    if let Ok(Some(account)) = db.get_account(account_id) {
        let mut qc = serde_json::Map::new();
        if let Some(ref lifecycle) = account.lifecycle {
            qc.insert("Lifecycle".to_string(), serde_json::json!(lifecycle));
        }
        if let Some(arr) = account.arr {
            qc.insert(
                "ARR".to_string(),
                serde_json::json!(format!("${:.0}k", arr / 1000.0)),
            );
        }
        if let Some(ref health) = account.health {
            qc.insert("Health".to_string(), serde_json::json!(health));
        }
        if let Some(ref contract_end) = account.contract_end {
            qc.insert("Renewal".to_string(), serde_json::json!(contract_end));
        }
        if !qc.is_empty() {
            if let Some(obj) = prep.as_object_mut() {
                obj.insert("quickContext".to_string(), serde_json::Value::Object(qc));
            }
        }
    }

    // Open actions for this account
    if let Ok(actions) = db.get_account_actions(account_id) {
        if !actions.is_empty() {
            let today = Utc::now().format("%Y-%m-%d").to_string();
            let items: Vec<serde_json::Value> = actions
                .iter()
                .take(5)
                .map(|a| {
                    let is_overdue = a.due_date.as_deref().is_some_and(|d| d < today.as_str());
                    serde_json::json!({
                        "title": a.title,
                        "dueDate": a.due_date,
                        "isOverdue": is_overdue,
                    })
                })
                .collect();
            if !items.is_empty() {
                if let Some(obj) = prep.as_object_mut() {
                    obj.insert("openItems".to_string(), serde_json::json!(items));
                }
            }
        }
    }
}

/// Result of syncing calendar events: meeting IDs that need intelligence triggers.
struct CalendarSyncIntelligence {
    /// Meeting IDs that are newly detected (need initial intelligence generation).
    new_meetings: Vec<String>,
    /// Meeting IDs that changed (title/time shifted — need force-full refresh).
    changed_meetings: Vec<String>,
}

/// Populate people table from calendar event attendees (I51).
///
/// For each event, for each attendee email:
/// - Skip self (match against user's Google email)
/// - Skip all-hands (>50 attendees)
/// - Classify internal/external using user_domain config
/// - Upsert into people table (idempotent)
/// - Write People/{name}/person.json + person.md if new
/// - Auto-link to entity if meeting has an account field
///
/// Returns sync intelligence for new/changed meetings that need intelligence triggers.
fn populate_people_from_events(events: &[CalendarEvent], state: &AppState, workspace: &Path) -> CalendarSyncIntelligence {
    // Acquire config/auth locks first (short-lived), then DB lock
    let self_email = state.google_auth.lock().ok().and_then(|g| match &*g {
        GoogleAuthStatus::Authenticated { email } => Some(email.to_lowercase()),
        _ => None,
    });

    let user_domains = state
        .config
        .read()
        .ok()
        .and_then(|g| g.as_ref().map(|c| c.resolved_user_domains()))
        .unwrap_or_default();

    let empty_result = CalendarSyncIntelligence {
        new_meetings: Vec::new(),
        changed_meetings: Vec::new(),
    };

    let db_guard = match state.db.lock().ok() {
        Some(g) => g,
        None => return empty_result,
    };
    let db = match db_guard.as_ref() {
        Some(db) => db,
        None => return empty_result,
    };

    let mut new_people = 0;
    let mut new_meetings = Vec::new();
    let mut changed_meetings = Vec::new();

    for event in events {
        // Skip all-hands (>50 attendees)
        if event.attendees.len() > 50 {
            continue;
        }

        // Ensure meeting exists in DB so record_meeting_attendance can query start_time
        let meeting_id = meeting_primary_id(
            Some(&event.id),
            &event.title,
            &event.start.to_rfc3339(),
            event.meeting_type.as_str(),
        );
        let attendees_json = serde_json::to_string(&event.attendees).unwrap_or_default();
        match db.ensure_meeting_in_history(crate::db::EnsureMeetingHistoryInput {
            id: &meeting_id,
            title: &event.title,
            meeting_type: event.meeting_type.as_str(),
            start_time: &event.start.to_rfc3339(),
            end_time: Some(&event.end.to_rfc3339()),
            calendar_event_id: Some(&event.id),
            attendees: Some(&attendees_json),
            description: None, // Description flows through directive path
        }) {
            Ok(crate::db::MeetingSyncOutcome::New) => {
                new_meetings.push(meeting_id.clone());
            }
            Ok(crate::db::MeetingSyncOutcome::Changed) => {
                // Mark as having new signals so intelligence refreshes
                let _ = db.mark_meeting_new_signals(&meeting_id);
                changed_meetings.push(meeting_id.clone());
            }
            Ok(crate::db::MeetingSyncOutcome::Unchanged) => {}
            Err(e) => {
                log::warn!(
                    "Failed to ensure meeting '{}' in history: {}",
                    event.title,
                    e
                );
            }
        }

        for email in &event.attendees {
            let email_lower = email.to_lowercase();

            // Skip self
            if self_email.as_deref() == Some(&email_lower) {
                continue;
            }

            // Check if person already exists in DB (exact email or known alias)
            let existing = db.get_person_by_email_or_alias(&email_lower).ok().flatten();
            // If no exact/alias match, try domain-alias resolution
            let existing = match existing {
                Some(p) => Some(p),
                None => {
                    match db.get_sibling_domains_for_email(&email_lower, &user_domains) {
                        Ok(siblings) if !siblings.is_empty() => {
                            match db.find_person_by_domain_alias(&email_lower, &siblings) {
                                Ok(Some(person)) => {
                                    // Record this new email as an alias
                                    let _ = db.add_person_email(&person.id, &email_lower, false);
                                    Some(person)
                                }
                                _ => None,
                            }
                        }
                        _ => None,
                    }
                }
            };
            if let Some(ref person) = existing {
                // Person already tracked — auto-link to entity if applicable
                if let Some(ref account) = event.account {
                    let _ = db.link_person_to_entity(&person.id, account, "associated");
                }
                // Record attendance (idempotent — safe across repeated polls)
                let _ = db.record_meeting_attendance(&meeting_id, &person.id);
                continue;
            }

            // New person — create
            let id = person_id_from_email(&email_lower);
            let name = name_from_email(&email_lower);
            let org = org_from_email(&email_lower);
            let relationship =
                crate::util::classify_relationship_multi(&email_lower, &user_domains);

            let person = DbPerson {
                id: id.clone(),
                email: email_lower,
                name,
                organization: Some(org),
                role: None,
                relationship,
                notes: None,
                tracker_path: None,
                last_seen: Some(event.start.to_rfc3339()),
                first_seen: Some(Utc::now().to_rfc3339()),
                meeting_count: 0,
                updated_at: Utc::now().to_rfc3339(),
                archived: false,
                linkedin_url: None,
                twitter_handle: None,
                phone: None,
                photo_url: None,
                bio: None,
                title_history: None,
                company_industry: None,
                company_size: None,
                company_hq: None,
                last_enriched_at: None,
                enrichment_sources: None,
            };

            if let Ok(is_new) = db.upsert_person(&person) {
                if let Err(e) = people::write_person_json(workspace, &person, db) {
                    log::warn!("Failed to write person.json for '{}': {}", person.name, e);
                }
                if let Err(e) = people::write_person_markdown(workspace, &person, db) {
                    log::warn!("Failed to write person.md for '{}': {}", person.name, e);
                }
                new_people += 1;

                // I353 Phase 2: Emit person_created signal for hygiene feedback loop
                if is_new {
                    let _ = crate::signals::bus::emit_signal_and_propagate(
                        db,
                        &state.signal_engine,
                        "person",
                        &person.id,
                        "person_created",
                        "calendar_sync",
                        None,
                        0.95,
                    );
                }

                // Auto-link to entity if meeting has an account
                if let Some(ref account) = event.account {
                    let _ = db.link_person_to_entity(&id, account, "associated");
                }

                // Record attendance for the new person
                let _ = db.record_meeting_attendance(&meeting_id, &id);
            }
        }
    }

    if new_people > 0 {
        log::info!("People: discovered {} new people from calendar", new_people);
    }

    if !new_meetings.is_empty() || !changed_meetings.is_empty() {
        log::info!(
            "Calendar sync intelligence: {} new, {} changed meetings",
            new_meetings.len(),
            changed_meetings.len()
        );
    }

    CalendarSyncIntelligence {
        new_meetings,
        changed_meetings,
    }
}

/// Detect meetings that were in the DB for today but disappeared from the calendar poll.
///
/// These are likely cancelled meetings. Updates their intelligence_state to "archived".
fn detect_cancelled_meetings(current_events: &[CalendarEvent], state: &AppState) {
    let today = Utc::now().date_naive().to_string(); // "YYYY-MM-DD"

    let db_guard = match state.db.lock().ok() {
        Some(g) => g,
        None => return,
    };
    let db = match db_guard.as_ref() {
        Some(db) => db,
        None => return,
    };

    // Build set of current calendar event IDs from this poll
    let current_ids: std::collections::HashSet<&str> = current_events
        .iter()
        .map(|e| e.id.as_str())
        .collect();

    // Query today's meetings from DB that have a calendar_event_id
    let mut stmt = match db.conn_ref().prepare(
        "SELECT id, calendar_event_id FROM meetings_history
         WHERE start_time LIKE ?1 || '%'
         AND calendar_event_id IS NOT NULL
         AND (intelligence_state IS NULL OR intelligence_state != 'archived')",
    ) {
        Ok(s) => s,
        Err(_) => return,
    };

    let cancelled: Vec<String> = stmt
        .query_map(rusqlite::params![today], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
            ))
        })
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|r| r.ok())
        .filter(|(_id, cal_id)| !current_ids.contains(cal_id.as_str()))
        .map(|(id, _)| id)
        .collect();

    for meeting_id in &cancelled {
        if let Err(e) = db.update_intelligence_state(meeting_id, "archived", None, None) {
            log::warn!(
                "Failed to archive cancelled meeting '{}': {}",
                meeting_id,
                e
            );
        } else {
            log::info!(
                "Calendar poll: meeting '{}' cancelled, intelligence archived",
                meeting_id
            );
        }
        // Emit cancellation signal (I308) with propagation
        let _ = crate::signals::bus::emit_signal_and_propagate(
            db, &state.signal_engine, "meeting", meeting_id, "meeting_cancelled", "calendar",
            None, 0.9,
        );
    }
}

/// Get workspace path from config
fn get_workspace(state: &AppState) -> Option<PathBuf> {
    state
        .config
        .read()
        .ok()
        .and_then(|g| g.clone())
        .map(|cfg| std::path::PathBuf::from(cfg.workspace_path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{DbAccount, test_utils::test_db};

    fn sample_event(
        id: &str,
        title: &str,
        mt: MeetingType,
        account: Option<&str>,
    ) -> CalendarEvent {
        CalendarEvent {
            id: id.to_string(),
            title: title.to_string(),
            start: Utc::now(),
            end: Utc::now() + chrono::Duration::hours(1),
            meeting_type: mt,
            account: account.map(|a| a.to_string()),
            attendees: vec![],
            is_all_day: false,
            linked_entities: None,
        }
    }

    #[test]
    fn test_has_existing_prep_for_event_no_match() {
        let dir = tempfile::tempdir().expect("temp dir");
        let preps_dir = dir.path();

        // Write a prep file with a different event ID
        let prep = serde_json::json!({
            "meetingId": "test-meeting",
            "calendarEventId": "cal-event-999",
            "title": "Some meeting"
        });
        std::fs::write(
            preps_dir.join("test-meeting.json"),
            serde_json::to_string_pretty(&prep).unwrap(),
        )
        .unwrap();

        assert!(!has_existing_prep_for_event(preps_dir, "cal-event-123"));
    }

    #[test]
    fn test_has_existing_prep_for_event_match() {
        let dir = tempfile::tempdir().expect("temp dir");
        let preps_dir = dir.path();

        let prep = serde_json::json!({
            "meetingId": "test-meeting",
            "calendarEventId": "cal-event-123",
            "title": "Acme QBR"
        });
        std::fs::write(
            preps_dir.join("test-meeting.json"),
            serde_json::to_string_pretty(&prep).unwrap(),
        )
        .unwrap();

        assert!(has_existing_prep_for_event(preps_dir, "cal-event-123"));
    }

    #[test]
    fn test_has_existing_prep_ignores_non_json() {
        let dir = tempfile::tempdir().expect("temp dir");
        let preps_dir = dir.path();

        std::fs::write(preps_dir.join("notes.txt"), "cal-event-123").unwrap();
        assert!(!has_existing_prep_for_event(preps_dir, "cal-event-123"));
    }

    #[test]
    fn test_enrich_prep_from_db_adds_quick_context() {
        let db = test_db();

        // Insert an account
        let account = DbAccount {
            id: "acme".to_string(),
            name: "Acme Corp".to_string(),
            lifecycle: Some("ramping".to_string()),
            arr: Some(150_000.0),
            health: Some("green".to_string()),
            contract_start: None,
            contract_end: Some("2026-06-15".to_string()),
            nps: None,
            tracker_path: None,
            parent_id: None,
            is_internal: false,
            updated_at: Utc::now().to_rfc3339(),
            archived: false,
            keywords: None,
            keywords_extracted_at: None,
        metadata: None,
        };
        db.upsert_account(&account).unwrap();

        let mut prep = serde_json::json!({
            "meetingId": "test",
            "title": "Acme sync"
        });

        enrich_prep_from_db(&mut prep, "acme", &db);

        let qc = prep.get("quickContext").expect("quickContext should exist");
        assert_eq!(qc.get("Lifecycle").unwrap(), "ramping");
        assert_eq!(qc.get("ARR").unwrap(), "$150k");
        assert_eq!(qc.get("Health").unwrap(), "green");
        assert_eq!(qc.get("Renewal").unwrap(), "2026-06-15");
    }

    #[test]
    fn test_enrich_prep_from_db_no_account() {
        let db = test_db();

        let mut prep = serde_json::json!({
            "meetingId": "test",
            "title": "Unknown sync"
        });

        enrich_prep_from_db(&mut prep, "nonexistent", &db);

        // Should not add quickContext if account not found
        assert!(prep.get("quickContext").is_none());
    }

    #[test]
    fn test_prep_eligible_types_filter() {
        // Account-based prep types
        assert!(PREP_ELIGIBLE_TYPES.contains(&MeetingType::Customer));
        assert!(PREP_ELIGIBLE_TYPES.contains(&MeetingType::Qbr));
        assert!(PREP_ELIGIBLE_TYPES.contains(&MeetingType::Partnership));
        // I159: Person-prep types
        assert!(PREP_ELIGIBLE_TYPES.contains(&MeetingType::Internal));
        assert!(PREP_ELIGIBLE_TYPES.contains(&MeetingType::TeamSync));
        assert!(PREP_ELIGIBLE_TYPES.contains(&MeetingType::OneOnOne));
        // Not eligible
        assert!(!PREP_ELIGIBLE_TYPES.contains(&MeetingType::Personal));
        assert!(!PREP_ELIGIBLE_TYPES.contains(&MeetingType::AllHands));
    }

    #[test]
    fn test_all_day_events_skipped() {
        // All-day events should never get preps, even if they're customer type
        let event = CalendarEvent {
            id: "cal-1".to_string(),
            title: "Acme offsite".to_string(),
            start: Utc::now(),
            end: Utc::now() + chrono::Duration::hours(8),
            meeting_type: MeetingType::Customer,
            account: Some("acme".to_string()),
            attendees: vec![],
            is_all_day: true,
            linked_entities: None,
        };

        assert!(event.is_all_day || !PREP_ELIGIBLE_TYPES.contains(&event.meeting_type));
        // Since is_all_day is true, the condition short-circuits and skips
        assert!(event.is_all_day);
    }

    #[test]
    fn test_make_meeting_id_deterministic() {
        let id1 = make_meeting_id("Acme QBR", "2026-02-07T09:00:00Z", "customer");
        let id2 = make_meeting_id("Acme QBR", "2026-02-07T09:00:00Z", "customer");
        assert_eq!(id1, id2);

        // Different inputs produce different IDs
        let id3 = make_meeting_id("Acme QBR", "2026-02-07T10:00:00Z", "customer");
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_meeting_primary_id_stable_across_rename_reschedule() {
        let id1 = meeting_primary_id(
            Some("evt-123@google.com"),
            "Acme Weekly Sync",
            "2026-02-12T09:00:00Z",
            "customer",
        );
        let id2 = meeting_primary_id(
            Some("evt-123@google.com"),
            "Acme Strategy Review",
            "2026-02-12T11:00:00Z",
            "customer",
        );
        assert_eq!(id1, id2);
        assert_eq!(id1, "evt-123_at_google.com");
    }

    #[test]
    fn test_has_existing_prep_for_event_match_sanitized_id() {
        let dir = tempfile::tempdir().expect("temp dir");
        let preps_dir = dir.path();

        let prep = serde_json::json!({
            "meetingId": "evt-123_at_google.com",
            "calendarEventId": "evt-123@google.com",
            "title": "Acme QBR"
        });
        std::fs::write(
            preps_dir.join("evt-123_at_google.com.json"),
            serde_json::to_string_pretty(&prep).unwrap(),
        )
        .unwrap();

        assert!(has_existing_prep_for_event(
            preps_dir,
            "evt-123_at_google.com"
        ));
    }
}
