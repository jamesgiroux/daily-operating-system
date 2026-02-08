//! Thin orchestrators that compose Phase 1 ops and write directive JSON.
//!
//! Ported from prepare_today.py, prepare_week.py, refresh_emails.py, deliver_week.py.
//! Each orchestrator is async (for Google API calls) and writes a directive JSON
//! that the Rust delivery pipeline (deliver.rs) consumes.

use std::collections::HashSet;
use std::path::Path;

use chrono::{Datelike, NaiveDate, Utc};
use serde_json::{json, Value};

use crate::error::ExecutionError;
use crate::google_api;
use crate::state::AppState;

use super::actions;
use super::constants::DAY_NAMES;
use super::email_classify;
use super::gaps;
use super::meeting_context;

// ============================================================================
// prepare_today
// ============================================================================

/// Replaces prepare_today.py. Writes: _today/data/today-directive.json
pub async fn prepare_today(
    state: &AppState,
    workspace: &Path,
) -> Result<(), ExecutionError> {
    let now = Utc::now();
    let today = now.date_naive();

    // Load config
    let (profile, user_domain) = get_config(state);

    log::info!(
        "prepare_today: workspace={}, profile={}, domain={}",
        workspace.display(),
        profile,
        if user_domain.is_empty() { "(unknown)" } else { &user_domain }
    );

    // Step 1: Context metadata
    let (iso_year, iso_week, _) = today.iso_week_fields();
    let context = json!({
        "date": today.to_string(),
        "day_of_week": today.format("%A").to_string(),
        "week_number": iso_week,
        "year": iso_year,
        "profile": profile,
    });

    // Step 2: Fetch calendar events + classify
    let account_hints = build_account_domain_hints(workspace);

    let (classified, events, meetings_by_type, time_status) =
        fetch_and_classify_today(today, &user_domain, &account_hints).await;

    log::info!("prepare_today: {} events fetched", events.len());

    // Step 3: Calendar gaps
    let gap_list = gaps::compute_gaps(&events, today);
    let total_gap_minutes: i64 = gap_list
        .iter()
        .filter_map(|g| g.get("duration_minutes").and_then(|v| v.as_i64()))
        .sum();
    log::info!("prepare_today: {} gaps, {} min focus time", gap_list.len(), total_gap_minutes);

    // Step 4: Fetch and classify emails
    let customer_domains = extract_customer_domains(&meetings_by_type);
    let email_result = fetch_and_classify_emails(&user_domain, &customer_domains, &account_hints).await;
    log::info!(
        "prepare_today: {} emails ({} high, {} medium, {} low)",
        email_result.all.len(),
        email_result.high.len(),
        email_result.medium_count,
        email_result.low_count,
    );

    // Step 5: Parse actions
    let db_guard = state.db.lock().ok();
    let db_ref = db_guard.as_ref().and_then(|g| g.as_ref());
    let action_result = actions::parse_workspace_actions(workspace, db_ref);
    let actions_dict = action_result.to_value();

    // Step 6: Meeting contexts
    let meeting_contexts = meeting_context::gather_all_meeting_contexts(
        &classified,
        workspace,
        db_ref,
    );
    // Drop DB guard before any further awaits
    drop(db_guard);

    // Step 7: File inventory
    let existing_today = inventory_today_files(workspace);
    let inbox_pending = count_inbox_pending(workspace);

    // Step 8: Generate AI tasks
    let ai_tasks = generate_ai_tasks(&classified, &time_status, &email_result.high);

    // Build lean events (strip attendees)
    let lean_events: Vec<Value> = events
        .iter()
        .map(|ev| {
            json!({
                "id": ev.get("id"),
                "summary": ev.get("summary").or_else(|| ev.get("title")),
                "start": ev.get("start"),
                "end": ev.get("end"),
            })
        })
        .collect();

    // Strip attendees from meetings_by_type
    let lean_meetings = lean_meetings_by_type(&meetings_by_type);

    let directive = json!({
        "command": "today",
        "generated_at": now.to_rfc3339(),
        "context": context,
        "calendar": {
            "events": lean_events,
            "past": time_status.get("past").cloned().unwrap_or_default(),
            "in_progress": time_status.get("in_progress").cloned().unwrap_or_default(),
            "upcoming": time_status.get("upcoming").cloned().unwrap_or_default(),
            "gaps": gap_list,
        },
        "meetings": lean_meetings,
        "meeting_contexts": meeting_contexts,
        "actions": actions_dict,
        "emails": {
            "high_priority": email_result.high,
            "classified": email_result.all,
            "medium_count": email_result.medium_count,
            "low_count": email_result.low_count,
        },
        "files": {
            "existing_today": existing_today,
            "inbox_pending": inbox_pending,
        },
        "ai_tasks": ai_tasks,
    });

    // Write output
    let output_path = workspace.join("_today").join("data").join("today-directive.json");
    write_directive(&output_path, &directive)?;

    log::info!("prepare_today: directive written to {}", output_path.display());
    Ok(())
}

// ============================================================================
// prepare_week
// ============================================================================

/// Replaces prepare_week.py. Writes: _today/data/week-directive.json
pub async fn prepare_week(
    state: &AppState,
    workspace: &Path,
) -> Result<(), ExecutionError> {
    let now = Utc::now();
    let today = now.date_naive();

    let (profile, user_domain) = get_config(state);

    // Week bounds
    let monday = today - chrono::Duration::days(today.weekday().num_days_from_monday() as i64);
    let friday = monday + chrono::Duration::days(4);
    let (iso_year, iso_week, _) = monday.iso_week_fields();
    let week_label = format!("W{:02}", iso_week);
    let date_range = format_date_range(monday, friday);

    log::info!("prepare_week: {} ({})", week_label, date_range);

    let context = json!({
        "weekNumber": week_label,
        "year": iso_year,
        "monday": monday.to_string(),
        "friday": friday.to_string(),
        "dateRange": date_range,
        "profile": profile,
    });

    // Fetch and classify calendar events for the week
    let account_hints = build_account_domain_hints(workspace);
    let (classified, _events, _meetings_by_type, _time_status, events_by_day) =
        fetch_and_classify_week(monday, friday, &user_domain, &account_hints).await;

    // Actions from SQLite
    let db_guard = state.db.lock().ok();
    let db_ref = db_guard.as_ref().and_then(|g| g.as_ref());
    let actions_data = match db_ref {
        Some(db) => actions::fetch_actions_from_db(db),
        None => json!({"overdue": [], "thisWeek": []}),
    };

    // Meeting contexts
    let meeting_contexts = meeting_context::gather_all_meeting_contexts(
        &classified,
        workspace,
        db_ref,
    );
    drop(db_guard);

    // Gap analysis
    let gaps_by_day = gaps::compute_all_gaps(&events_by_day, monday);
    let suggestions = gaps::suggest_focus_blocks(&gaps_by_day);

    // Build lean events by day (strip attendees)
    let mut serializable_by_day = serde_json::Map::new();
    if let Some(obj) = events_by_day.as_object() {
        for (day_name, day_events) in obj {
            let lean: Vec<Value> = day_events
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .map(|ev| {
                    json!({
                        "id": ev.get("id"),
                        "title": ev.get("title").or_else(|| ev.get("summary")),
                        "start": ev.get("start"),
                        "end": ev.get("end"),
                        "type": ev.get("type"),
                        "external_domains": ev.get("external_domains"),
                    })
                })
                .collect();
            serializable_by_day.insert(day_name.clone(), json!(lean));
        }
    }

    let directive = json!({
        "command": "week",
        "generatedAt": now.to_rfc3339(),
        "context": context,
        "meetingsByDay": Value::Object(serializable_by_day),
        "meetingContexts": meeting_contexts,
        "actions": actions_data,
        "timeBlocks": {
            "gapsByDay": gaps_by_day,
            "suggestions": suggestions,
        },
    });

    let output_path = workspace.join("_today").join("data").join("week-directive.json");
    write_directive(&output_path, &directive)?;

    log::info!("prepare_week: directive written to {}", output_path.display());
    Ok(())
}

// ============================================================================
// refresh_emails
// ============================================================================

/// Replaces refresh_emails.py. Writes: _today/data/email-refresh-directive.json
pub async fn refresh_emails(
    state: &AppState,
    workspace: &Path,
) -> Result<(), ExecutionError> {
    let (_profile, user_domain) = get_config(state);
    let account_hints = build_account_domain_hints(workspace);

    // Extract customer domains from morning's schedule.json if available
    let mut customer_domains = HashSet::new();
    let schedule_path = workspace.join("_today").join("data").join("schedule.json");
    if let Ok(content) = std::fs::read_to_string(&schedule_path) {
        if let Ok(schedule) = serde_json::from_str::<Value>(&content) {
            if let Some(meetings) = schedule.get("meetings").and_then(|v| v.as_array()) {
                for meeting in meetings {
                    if let Some(attendees) = meeting.get("attendees").and_then(|v| v.as_array()) {
                        for attendee in attendees {
                            if let Some(email) = attendee.as_str() {
                                if email.contains('@') {
                                    let domain = email.split('@').nth(1).unwrap_or("").to_lowercase();
                                    customer_domains.insert(domain);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let email_result = fetch_and_classify_emails(&user_domain, &customer_domains, &account_hints).await;

    // Build refresh directive matching the shape Rust expects
    let mut high_priority = Vec::new();
    let mut classified = Vec::new();
    for email in &email_result.all {
        let priority = email.get("priority").and_then(|v| v.as_str()).unwrap_or("medium");
        let entry = json!({
            "id": email.get("id"),
            "from": email.get("from"),
            "from_email": email.get("from_email"),
            "subject": email.get("subject"),
            "snippet": email.get("snippet"),
            "priority": priority,
        });
        if priority == "high" {
            high_priority.push(entry);
        } else {
            classified.push(entry);
        }
    }

    let directive = json!({
        "source": "email-refresh",
        "emails": {
            "highPriority": high_priority,
            "classified": classified,
            "mediumCount": email_result.medium_count,
            "lowCount": email_result.low_count,
        },
    });

    let output_path = workspace.join("_today").join("data").join("email-refresh-directive.json");
    write_directive(&output_path, &directive)?;

    log::info!(
        "refresh_emails: {} emails ({} high)",
        email_result.all.len(),
        email_result.high.len(),
    );
    Ok(())
}

// ============================================================================
// deliver_week (Phase 3)
// ============================================================================

/// Replaces deliver_week.py. Reads week-directive.json, writes week-overview.json.
///
/// This is Phase 3 of the week workflow — transforms the directive into the
/// format the frontend consumes (matching the Rust WeekOverview struct).
pub fn deliver_week(workspace: &Path) -> Result<(), String> {
    let data_dir = workspace.join("_today").join("data");
    let directive_path = data_dir.join("week-directive.json");

    let directive: Value = if directive_path.exists() {
        let raw = std::fs::read_to_string(&directive_path)
            .map_err(|e| format!("Failed to read week directive: {}", e))?;
        serde_json::from_str(&raw)
            .map_err(|e| format!("Failed to parse week directive: {}", e))?
    } else {
        return Err("Week directive not found".to_string());
    };

    let overview = build_week_overview(&directive, &data_dir);
    let output_path = data_dir.join("week-overview.json");
    let content = serde_json::to_string_pretty(&overview)
        .map_err(|e| format!("JSON serialization failed: {}", e))?;

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create dir: {}", e))?;
    }
    std::fs::write(&output_path, content)
        .map_err(|e| format!("Failed to write week overview: {}", e))?;

    log::info!("deliver_week: wrote {}", output_path.display());
    Ok(())
}

// ============================================================================
// Shared helpers
// ============================================================================

fn get_config(state: &AppState) -> (String, String) {
    let config_guard = state.config.read().ok();
    let config = config_guard.as_ref().and_then(|g| g.as_ref());

    let profile = config
        .map(|c| c.profile.clone())
        .unwrap_or_else(|| "general".to_string());

    let user_domain = config
        .and_then(|c| c.user_domain.clone())
        .unwrap_or_default();

    (profile, user_domain)
}

/// Build account domain hints from Accounts/ directory.
fn build_account_domain_hints(workspace: &Path) -> HashSet<String> {
    let accounts_dir = workspace.join("Accounts");
    if !accounts_dir.is_dir() {
        return HashSet::new();
    }

    let mut hints = HashSet::new();
    if let Ok(entries) = std::fs::read_dir(&accounts_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false)
                && !name_str.starts_with('.')
                && !name_str.starts_with('_')
            {
                let slug: String = name_str
                    .to_lowercase()
                    .chars()
                    .filter(|c| c.is_alphanumeric())
                    .collect();
                if slug.len() >= 3 {
                    hints.insert(slug);
                }
            }
        }
    }
    hints
}

/// Fetch calendar events for today, classify, and compute time status.
async fn fetch_and_classify_today(
    today: NaiveDate,
    user_domain: &str,
    account_hints: &HashSet<String>,
) -> (Vec<Value>, Vec<Value>, Value, serde_json::Map<String, Value>) {
    let access_token = match google_api::get_valid_access_token().await {
        Ok(t) => t,
        Err(e) => {
            log::warn!("prepare_today: Google auth failed ({}), calendar will be empty", e);
            return (Vec::new(), Vec::new(), json!({}), serde_json::Map::new());
        }
    };

    let raw_events = match google_api::calendar::fetch_events(&access_token, today, today).await {
        Ok(e) => e,
        Err(e) => {
            log::warn!("prepare_today: Calendar fetch failed ({}), calendar will be empty", e);
            return (Vec::new(), Vec::new(), json!({}), serde_json::Map::new());
        }
    };

    // Classify each event
    let mut classified = Vec::new();
    let mut events = Vec::new();
    for raw in &raw_events {
        let cm = google_api::classify::classify_meeting(raw, user_domain, account_hints);
        let ev = cm.to_calendar_event();
        classified.push(json!({
            "id": ev.id,
            "title": ev.title,
            "summary": ev.title,
            "start": ev.start,
            "end": ev.end,
            "type": ev.meeting_type,
            "attendees": raw.attendees,
            "organizer": raw.organizer,
            "external_domains": cm.external_domains,
            "is_recurring": raw.is_recurring,
        }));
        events.push(json!({
            "id": ev.id,
            "summary": ev.title,
            "start": ev.start,
            "end": ev.end,
        }));
    }

    // Bucket by type
    let mut meetings_by_type = serde_json::Map::new();
    for ev in &classified {
        let mt = ev.get("type").and_then(|v| v.as_str()).unwrap_or("external");
        meetings_by_type
            .entry(mt.to_string())
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .unwrap()
            .push(ev.clone());
    }

    // Time status
    let now = Utc::now();
    let mut time_status = serde_json::Map::new();
    time_status.insert("past".to_string(), json!([]));
    time_status.insert("in_progress".to_string(), json!([]));
    time_status.insert("upcoming".to_string(), json!([]));

    for ev in &classified {
        let event_id = ev.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let start_str = ev.get("start").and_then(|v| v.as_str()).unwrap_or("");
        let end_str = ev.get("end").and_then(|v| v.as_str()).unwrap_or("");

        let start_dt = chrono::DateTime::parse_from_rfc3339(
            &start_str.replace('Z', "+00:00"),
        ).ok().or_else(|| chrono::DateTime::parse_from_rfc3339(start_str).ok());
        let end_dt = chrono::DateTime::parse_from_rfc3339(
            &end_str.replace('Z', "+00:00"),
        ).ok().or_else(|| chrono::DateTime::parse_from_rfc3339(end_str).ok());

        let bucket = match (start_dt, end_dt) {
            (Some(_s), Some(e)) if now >= e => "past",
            (Some(s), end_opt) if s <= now && end_opt.map_or(true, |e| now < e) => "in_progress",
            _ => "upcoming",
        };

        time_status
            .get_mut(bucket)
            .unwrap()
            .as_array_mut()
            .unwrap()
            .push(json!(event_id));
    }

    (classified, events, Value::Object(meetings_by_type), time_status)
}

/// Fetch calendar events for a week, classify, and organize by day.
async fn fetch_and_classify_week(
    monday: NaiveDate,
    friday: NaiveDate,
    user_domain: &str,
    account_hints: &HashSet<String>,
) -> (Vec<Value>, Vec<Value>, Value, serde_json::Map<String, Value>, Value) {
    let access_token = match google_api::get_valid_access_token().await {
        Ok(t) => t,
        Err(e) => {
            log::warn!("prepare_week: Google auth failed ({})", e);
            let empty_days = json!({
                "Monday": [], "Tuesday": [], "Wednesday": [],
                "Thursday": [], "Friday": [],
            });
            return (Vec::new(), Vec::new(), json!({}), serde_json::Map::new(), empty_days);
        }
    };

    let raw_events = match google_api::calendar::fetch_events(&access_token, monday, friday).await {
        Ok(e) => e,
        Err(e) => {
            log::warn!("prepare_week: Calendar fetch failed ({})", e);
            let empty_days = json!({
                "Monday": [], "Tuesday": [], "Wednesday": [],
                "Thursday": [], "Friday": [],
            });
            return (Vec::new(), Vec::new(), json!({}), serde_json::Map::new(), empty_days);
        }
    };

    let mut classified = Vec::new();
    let mut events = Vec::new();
    for raw in &raw_events {
        let cm = google_api::classify::classify_meeting(raw, user_domain, account_hints);
        let ev = cm.to_calendar_event();
        classified.push(json!({
            "id": ev.id,
            "title": ev.title,
            "summary": ev.title,
            "start": ev.start,
            "end": ev.end,
            "type": ev.meeting_type,
            "attendees": raw.attendees,
            "organizer": raw.organizer,
            "external_domains": cm.external_domains,
            "is_recurring": raw.is_recurring,
        }));
        events.push(json!({
            "id": ev.id,
            "summary": ev.title,
            "start": ev.start,
            "end": ev.end,
        }));
    }

    // Bucket by type
    let mut meetings_by_type = serde_json::Map::new();
    for ev in &classified {
        let mt = ev.get("type").and_then(|v| v.as_str()).unwrap_or("external");
        meetings_by_type
            .entry(mt.to_string())
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .unwrap()
            .push(ev.clone());
    }

    // Organize by day
    let mut events_by_day = serde_json::Map::new();
    for day_name in DAY_NAMES {
        events_by_day.insert(day_name.to_string(), json!([]));
    }
    for ev in &classified {
        let start_str = ev.get("start").and_then(|v| v.as_str()).unwrap_or("");
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&start_str.replace('Z', "+00:00")) {
            let weekday = dt.weekday().num_days_from_monday() as usize;
            if weekday < 5 {
                events_by_day
                    .get_mut(DAY_NAMES[weekday])
                    .unwrap()
                    .as_array_mut()
                    .unwrap()
                    .push(ev.clone());
            }
        }
    }

    let time_status = serde_json::Map::new(); // Week doesn't need time_status

    (classified, events, Value::Object(meetings_by_type), time_status, Value::Object(events_by_day))
}

/// Fetch and classify emails (async, uses google_api).
struct EmailResult {
    all: Vec<Value>,
    high: Vec<Value>,
    medium_count: u64,
    low_count: u64,
}

async fn fetch_and_classify_emails(
    user_domain: &str,
    customer_domains: &HashSet<String>,
    account_hints: &HashSet<String>,
) -> EmailResult {
    let access_token = match google_api::get_valid_access_token().await {
        Ok(t) => t,
        Err(e) => {
            log::warn!("Email fetch: auth failed ({}), emails will be empty", e);
            return EmailResult {
                all: Vec::new(),
                high: Vec::new(),
                medium_count: 0,
                low_count: 0,
            };
        }
    };

    let raw_emails = match google_api::gmail::fetch_unread_emails(&access_token, 30).await {
        Ok(e) => e,
        Err(e) => {
            log::warn!("Email fetch failed ({}), emails will be empty", e);
            return EmailResult {
                all: Vec::new(),
                high: Vec::new(),
                medium_count: 0,
                low_count: 0,
            };
        }
    };

    let mut all = Vec::new();
    let mut high = Vec::new();
    let mut medium_count: u64 = 0;
    let mut low_count: u64 = 0;

    for email in &raw_emails {
        let priority = email_classify::classify_email_priority(
            &email.from,
            &email.subject,
            &email.list_unsubscribe,
            &email.precedence,
            customer_domains,
            user_domain,
            account_hints,
        );

        let from_email = email_classify::extract_email_address(&email.from);

        let obj = json!({
            "id": email.id,
            "thread_id": email.thread_id,
            "from": email.from,
            "from_email": from_email,
            "subject": email.subject,
            "snippet": email.snippet,
            "date": email.date,
            "priority": priority,
        });

        all.push(obj.clone());
        match priority {
            "high" => high.push(obj),
            "medium" => medium_count += 1,
            _ => low_count += 1,
        }
    }

    EmailResult {
        all,
        high,
        medium_count,
        low_count,
    }
}

/// Extract customer domains from classified meetings.
fn extract_customer_domains(meetings_by_type: &Value) -> HashSet<String> {
    let mut domains = HashSet::new();
    if let Some(customer_meetings) = meetings_by_type.get("customer").and_then(|v| v.as_array()) {
        for ev in customer_meetings {
            if let Some(ext_domains) = ev.get("external_domains").and_then(|v| v.as_array()) {
                for d in ext_domains {
                    if let Some(s) = d.as_str() {
                        domains.insert(s.to_string());
                    }
                }
            }
        }
    }
    domains
}

/// List existing files in {workspace}/_today/.
fn inventory_today_files(workspace: &Path) -> Vec<String> {
    let today_dir = workspace.join("_today");
    if !today_dir.is_dir() {
        return Vec::new();
    }
    let mut files: Vec<String> = std::fs::read_dir(&today_dir)
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| {
            e.file_type().map(|ft| ft.is_file()).unwrap_or(false)
                && !e.file_name().to_string_lossy().starts_with('.')
        })
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    files.sort();
    files
}

/// Count files pending in {workspace}/_inbox/.
fn count_inbox_pending(workspace: &Path) -> usize {
    let inbox_dir = workspace.join("_inbox");
    if !inbox_dir.is_dir() {
        return 0;
    }
    std::fs::read_dir(&inbox_dir)
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| {
            e.file_type().map(|ft| ft.is_file()).unwrap_or(false)
                && !e.file_name().to_string_lossy().starts_with('.')
        })
        .count()
}

/// Generate AI task list for Phase 2 enrichment.
fn generate_ai_tasks(
    classified: &[Value],
    time_status: &serde_json::Map<String, Value>,
    emails_high: &[Value],
) -> Vec<Value> {
    let mut tasks = Vec::new();
    let past_ids: HashSet<String> = time_status
        .get("past")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    for meeting in classified {
        let event_id = meeting.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let meeting_type = meeting.get("type").and_then(|v| v.as_str()).unwrap_or("");

        if past_ids.contains(event_id) {
            continue;
        }
        if meeting_type == "personal" || meeting_type == "all_hands" {
            continue;
        }

        match meeting_type {
            "customer" | "qbr" => {
                tasks.push(json!({
                    "type": "generate_meeting_prep",
                    "event_id": event_id,
                    "meeting_type": meeting_type,
                    "priority": "high",
                }));
            }
            "training" => {
                tasks.push(json!({
                    "type": "generate_meeting_prep",
                    "event_id": event_id,
                    "meeting_type": meeting_type,
                    "priority": "medium",
                }));
            }
            "external" => {
                let has_unknown = meeting
                    .get("external_domains")
                    .and_then(|v| v.as_array())
                    .map(|a| !a.is_empty())
                    .unwrap_or(false);
                tasks.push(json!({
                    "type": if has_unknown { "research_unknown_meeting" } else { "generate_meeting_prep" },
                    "event_id": event_id,
                    "meeting_type": meeting_type,
                    "priority": if has_unknown { "medium" } else { "low" },
                }));
            }
            "internal" | "team_sync" | "one_on_one" => {
                tasks.push(json!({
                    "type": "generate_meeting_prep",
                    "event_id": event_id,
                    "meeting_type": meeting_type,
                    "priority": "low",
                }));
            }
            _ => {}
        }
    }

    // Email summaries for high-priority emails
    for email in emails_high {
        tasks.push(json!({
            "type": "summarize_email",
            "email_id": email.get("id"),
            "thread_id": email.get("thread_id"),
            "priority": "medium",
        }));
    }

    // Generate daily briefing narrative
    tasks.push(json!({
        "type": "generate_briefing_narrative",
        "priority": "high",
    }));

    tasks
}

/// Strip attendees from meetings_by_type for lean directive output.
fn lean_meetings_by_type(meetings_by_type: &Value) -> Value {
    let mut lean = serde_json::Map::new();
    if let Some(obj) = meetings_by_type.as_object() {
        for (mt, meetings) in obj {
            let lean_meetings: Vec<Value> = meetings
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .map(|m| {
                    let mut cloned = m.clone();
                    if let Some(obj) = cloned.as_object_mut() {
                        obj.remove("attendees");
                    }
                    cloned
                })
                .collect();
            lean.insert(mt.clone(), json!(lean_meetings));
        }
    }
    Value::Object(lean)
}

/// Write directive JSON to disk, creating parent dirs as needed.
fn write_directive(path: &Path, data: &Value) -> Result<(), ExecutionError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            ExecutionError::ScriptFailed {
                code: 1,
                stderr: format!("Failed to create dir {}: {}", parent.display(), e),
            }
        })?;
    }
    let content = serde_json::to_string_pretty(data).map_err(|e| {
        ExecutionError::ParseError(format!("JSON serialization failed: {}", e))
    })?;
    std::fs::write(path, content).map_err(|e| {
        ExecutionError::ScriptFailed {
            code: 1,
            stderr: format!("Failed to write {}: {}", path.display(), e),
        }
    })?;
    Ok(())
}

/// Human-friendly date range string, e.g. "February 2-6, 2026".
fn format_date_range(monday: NaiveDate, friday: NaiveDate) -> String {
    if monday.month() == friday.month() {
        format!(
            "{} {}-{}, {}",
            monday.format("%B"),
            monday.day(),
            friday.day(),
            friday.year(),
        )
    } else {
        format!(
            "{} {} - {} {}, {}",
            monday.format("%B"),
            monday.day(),
            friday.format("%B"),
            friday.day(),
            friday.year(),
        )
    }
}

// ============================================================================
// deliver_week helpers (ported from deliver_week.py)
// ============================================================================

fn build_week_overview(directive: &Value, data_dir: &Path) -> Value {
    let context = directive.get("context").cloned().unwrap_or(json!({}));

    let week_number_raw = context
        .get("weekNumber")
        .or_else(|| context.get("week_number"))
        .cloned()
        .unwrap_or(json!(0));
    let week_number = if let Some(s) = week_number_raw.as_str() {
        if s.starts_with('W') {
            s.to_string()
        } else {
            format!("W{:02}", s.parse::<u32>().unwrap_or(0))
        }
    } else {
        format!("W{:02}", week_number_raw.as_u64().unwrap_or(0))
    };

    let date_range = context
        .get("dateRange")
        .or_else(|| context.get("date_range_display"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let monday_str = context
        .get("monday")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    // Build days array
    let meetings_by_day = directive
        .get("meetingsByDay")
        .cloned()
        .unwrap_or(json!({}));

    let mut days = Vec::new();
    for (i, day_name) in DAY_NAMES.iter().enumerate() {
        let day_date = if !monday_str.is_empty() {
            NaiveDate::parse_from_str(monday_str, "%Y-%m-%d")
                .ok()
                .map(|m| (m + chrono::Duration::days(i as i64)).to_string())
                .unwrap_or_default()
        } else {
            String::new()
        };

        let day_meetings = meetings_by_day
            .get(day_name)
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        days.push(build_week_day(&day_date, day_name, &day_meetings));
    }

    let action_summary = build_action_summary(directive, data_dir);
    let focus_areas = build_focus_areas(directive);
    let time_blocks = build_time_blocks(directive);

    json!({
        "weekNumber": week_number,
        "dateRange": date_range,
        "days": days,
        "actionSummary": action_summary,
        "hygieneAlerts": [],
        "focusAreas": focus_areas,
        "availableTimeBlocks": time_blocks,
    })
}

fn build_week_day(date: &str, day_name: &str, meetings_raw: &[Value]) -> Value {
    let mut meetings = Vec::new();
    for m in meetings_raw {
        let meeting_type = normalise_meeting_type(
            m.get("type").and_then(|v| v.as_str()).unwrap_or("internal"),
        );
        if meeting_type == "personal" {
            continue;
        }

        let start = m.get("start").and_then(|v| v.as_str()).unwrap_or("");
        let time_display = format_time_display(start);

        meetings.push(json!({
            "time": if time_display.is_empty() { "TBD".to_string() } else { time_display },
            "title": m.get("title").or_else(|| m.get("summary")).and_then(|v| v.as_str()).unwrap_or("Meeting"),
            "account": m.get("account"),
            "type": meeting_type,
            "prepStatus": "prep_needed",
        }));
    }

    json!({
        "date": date,
        "dayName": day_name,
        "meetings": meetings,
    })
}

fn build_action_summary(directive: &Value, data_dir: &Path) -> Value {
    let actions = directive.get("actions").cloned().unwrap_or(json!({}));
    let overdue = actions.get("overdue").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let mut this_week = actions
        .get("thisWeek")
        .or_else(|| actions.get("this_week"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    // Fallback to actions.json
    if overdue.is_empty() && this_week.is_empty() {
        let actions_path = data_dir.join("actions.json");
        if let Ok(content) = std::fs::read_to_string(&actions_path) {
            if let Ok(today_actions) = serde_json::from_str::<Value>(&content) {
                if let Some(all) = today_actions.get("actions").and_then(|v| v.as_array()) {
                    this_week = all
                        .iter()
                        .filter(|a| !a.get("isOverdue").and_then(|v| v.as_bool()).unwrap_or(false))
                        .cloned()
                        .collect();
                }
            }
        }
    }

    let critical_items: Vec<String> = overdue
        .iter()
        .take(10)
        .filter_map(|task| {
            let title = task.get("title").and_then(|v| v.as_str()).unwrap_or("");
            let account = task.get("account").and_then(|v| v.as_str()).unwrap_or("");
            if title.is_empty() {
                None
            } else if account.is_empty() {
                Some(title.to_string())
            } else {
                Some(format!("{} - {}", title, account))
            }
        })
        .collect();

    json!({
        "overdueCount": overdue.len(),
        "dueThisWeek": this_week.len(),
        "criticalItems": critical_items,
    })
}

fn build_focus_areas(directive: &Value) -> Vec<String> {
    let mut areas = Vec::new();

    let meetings_by_day = directive.get("meetingsByDay").cloned().unwrap_or(json!({}));
    let empty_arr = Vec::new();
    let customer_count: usize = meetings_by_day
        .as_object()
        .map(|obj| {
            obj.values()
                .flat_map(|day| day.as_array().unwrap_or(&empty_arr).iter())
                .filter(|m| m.get("type").and_then(|v| v.as_str()) == Some("customer"))
                .count()
        })
        .unwrap_or(0);
    if customer_count > 0 {
        areas.push(format!("Customer meetings ({})", customer_count));
    }

    let actions = directive.get("actions").cloned().unwrap_or(json!({}));
    let overdue = actions.get("overdue").and_then(|v| v.as_array());
    if let Some(o) = overdue {
        if !o.is_empty() {
            areas.push(format!("Overdue items ({})", o.len()));
        }
    }

    let this_week = actions
        .get("thisWeek")
        .or_else(|| actions.get("this_week"))
        .and_then(|v| v.as_array());
    if let Some(tw) = this_week {
        if !tw.is_empty() {
            areas.push(format!("Due this week ({})", tw.len()));
        }
    }

    if areas.is_empty() {
        areas.push("Review weekly overview".to_string());
    }

    areas
}

fn build_time_blocks(directive: &Value) -> Vec<Value> {
    let mut blocks = Vec::new();
    let time_blocks_raw = directive
        .get("timeBlocks")
        .or_else(|| directive.get("time_blocks"))
        .cloned()
        .unwrap_or(json!({}));

    let suggestions = time_blocks_raw
        .get("suggestions")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    for s in &suggestions {
        let day = s.get("day").and_then(|v| v.as_str()).unwrap_or("");
        let mut start = s.get("start").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let mut end = s.get("end").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let duration = s
            .get("duration_minutes")
            .or_else(|| s.get("duration"))
            .and_then(|v| v.as_i64())
            .unwrap_or(30);

        // Extract HH:MM from ISO datetime
        if start.contains('T') {
            start = start.split('T').nth(1).unwrap_or("")[..5].to_string();
        }
        if end.contains('T') {
            end = end.split('T').nth(1).unwrap_or("")[..5].to_string();
        }

        let suggested_use = s
            .get("suggested_use")
            .or_else(|| s.get("block_type"))
            .and_then(|v| v.as_str())
            .unwrap_or("Focus")
            .to_string();

        if !day.is_empty() && !start.is_empty() && !end.is_empty() {
            blocks.push(json!({
                "day": day,
                "start": start,
                "end": end,
                "durationMinutes": duration,
                "suggestedUse": suggested_use,
            }));
        }
    }

    // If no suggestions, use raw gaps
    if blocks.is_empty() {
        let gaps_by_day = time_blocks_raw
            .get("gapsByDay")
            .or_else(|| time_blocks_raw.get("gaps_by_day"))
            .cloned()
            .unwrap_or(json!({}));

        for day_name in DAY_NAMES {
            if let Some(day_gaps) = gaps_by_day.get(day_name).and_then(|v| v.as_array()) {
                for gap in day_gaps {
                    let duration = gap.get("duration_minutes").and_then(|v| v.as_i64()).unwrap_or(0);
                    if duration < 30 {
                        continue;
                    }
                    let mut start = gap.get("start").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let mut end = gap.get("end").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    if start.contains('T') {
                        start = start.split('T').nth(1).unwrap_or("")[..5].to_string();
                    }
                    if end.contains('T') {
                        end = end.split('T').nth(1).unwrap_or("")[..5].to_string();
                    }
                    if !start.is_empty() && !end.is_empty() {
                        blocks.push(json!({
                            "day": day_name,
                            "start": start,
                            "end": end,
                            "durationMinutes": duration,
                            "suggestedUse": "Deep work",
                        }));
                    }
                }
            }
        }
    }

    blocks.truncate(10);
    blocks
}

fn normalise_meeting_type(raw: &str) -> String {
    let normalised = raw.to_lowercase().replace(' ', "_").replace('-', "_");
    let valid = [
        "customer", "qbr", "training", "internal", "team_sync",
        "one_on_one", "partnership", "all_hands", "external", "personal",
    ];
    if valid.contains(&normalised.as_str()) {
        normalised
    } else {
        "internal".to_string()
    }
}

fn format_time_display(iso_string: &str) -> String {
    if iso_string.is_empty() || !iso_string.contains('T') {
        return String::new();
    }
    chrono::DateTime::parse_from_rfc3339(&iso_string.replace('Z', "+00:00"))
        .ok()
        .or_else(|| chrono::DateTime::parse_from_rfc3339(iso_string).ok())
        .map(|dt| {
            let h = dt.format("%I:%M %p").to_string();
            h.trim_start_matches('0').to_string()
        })
        .unwrap_or_default()
}

/// Helper trait for NaiveDate ISO week access.
trait IsoWeekFields {
    fn iso_week_fields(&self) -> (i32, u32, u32);
}

impl IsoWeekFields for NaiveDate {
    fn iso_week_fields(&self) -> (i32, u32, u32) {
        let iso = self.iso_week();
        (iso.year(), iso.week(), self.weekday().num_days_from_monday() + 1)
    }
}
