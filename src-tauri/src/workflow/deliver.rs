//! Rust-native delivery functions (ADR-0042: per-operation pipelines)
//!
//! Mechanical delivery (instant, no AI):
//! - `deliver_schedule()` → schedule.json
//! - `deliver_actions()` → actions.json
//! - `deliver_preps()` → preps/*.json
//! - `deliver_emails()` → emails.json
//! - `deliver_manifest()` → manifest.json
//!
//! AI enrichment (progressive, fault-tolerant):
//! - `enrich_emails()` → updates emails.json with summaries/actions/arcs
//! - `enrich_briefing()` → updates schedule.json with day narrative

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use chrono::{DateTime, Timelike, Utc};
use regex::Regex;
use serde_json::{json, Value};

use crate::json_loader::{
    Directive, DirectiveEmail, DirectiveEvent, DirectiveMeeting, DirectiveMeetingContext,
};

// ============================================================================
// Constants
// ============================================================================

/// Valid meeting type enum values (must match types.rs)
const VALID_MEETING_TYPES: &[&str] = &[
    "customer",
    "qbr",
    "training",
    "internal",
    "team_sync",
    "one_on_one",
    "partnership",
    "all_hands",
    "external",
    "personal",
];

/// Meeting types that receive prep files
const PREP_ELIGIBLE_TYPES: &[&str] = &["customer", "qbr", "partnership"];

// ============================================================================
// Shared helpers (ported from deliver_today.py)
// ============================================================================

/// Normalise a meeting type string to a valid enum value.
/// Defaults to "internal" for unrecognised values.
pub fn normalise_meeting_type(raw: &str) -> &'static str {
    let normalised = raw.to_lowercase().replace(' ', "_").replace('-', "_");
    for &valid in VALID_MEETING_TYPES {
        if normalised == valid {
            return valid;
        }
    }
    "internal"
}

/// Convert an ISO datetime string to human-readable time like "9:00 AM".
pub fn format_time_display(iso: &str) -> String {
    if iso.is_empty() || !iso.contains('T') {
        return "All day".to_string();
    }
    match DateTime::parse_from_rfc3339(&iso.replace('Z', "+00:00"))
        .or_else(|_| DateTime::parse_from_rfc3339(iso))
    {
        Ok(dt) => {
            let hour = dt.format("%I").to_string();
            let hour = hour.trim_start_matches('0');
            format!("{}:{} {}", hour, dt.format("%M"), dt.format("%p"))
        }
        Err(_) => {
            if iso.len() >= 16 {
                iso[11..16].to_string()
            } else {
                iso.to_string()
            }
        }
    }
}

/// Generate a stable meeting ID from a calendar event.
/// Format: HHMM-type-slug (e.g. "0900-customer-acme-sync").
pub fn make_meeting_id(summary: &str, start: &str, meeting_type: &str) -> String {
    let slug_re = Regex::new(r"[^a-z0-9]+").unwrap();
    let lower = summary.to_lowercase();
    let slug = slug_re.replace_all(&lower, "-");
    let slug = slug.trim_matches('-');
    let slug = if slug.len() > 40 { &slug[..40] } else { slug };

    let time_prefix = if start.contains('T') {
        DateTime::parse_from_rfc3339(&start.replace('Z', "+00:00"))
            .or_else(|_| DateTime::parse_from_rfc3339(start))
            .map(|dt| dt.format("%H%M").to_string())
            .unwrap_or_default()
    } else {
        String::new()
    };

    let time_prefix = if time_prefix.is_empty() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        format!("{}-{}", start, summary).hash(&mut hasher);
        format!("{:06x}", hasher.finish() & 0xFFFFFF)
    } else {
        time_prefix
    };

    format!("{}-{}-{}", time_prefix, meeting_type, slug)
}

/// Look up the meeting type for a calendar event by matching its id
/// against the classified meetings dict from the directive.
pub fn classify_event(
    event_id: &str,
    meetings_by_type: &HashMap<String, Vec<DirectiveMeeting>>,
) -> &'static str {
    for (mtype, meeting_list) in meetings_by_type {
        for m in meeting_list {
            let mid = m.event_id.as_deref().or(m.id.as_deref()).unwrap_or("");
            if mid == event_id {
                return normalise_meeting_type(mtype);
            }
        }
    }
    "internal"
}

/// Find the directive meeting entry for a calendar event.
fn find_meeting_entry<'a>(
    event_id: &str,
    meetings_by_type: &'a HashMap<String, Vec<DirectiveMeeting>>,
) -> (Option<&'a DirectiveMeeting>, Option<String>) {
    for (_mtype, meeting_list) in meetings_by_type {
        for m in meeting_list {
            let mid = m.event_id.as_deref().or(m.id.as_deref()).unwrap_or("");
            if mid == event_id {
                return (Some(m), m.account.clone());
            }
        }
    }
    (None, None)
}

/// Find the meeting context matching an account or event_id.
pub fn find_meeting_context<'a>(
    account: Option<&str>,
    event_id: Option<&str>,
    contexts: &'a [DirectiveMeetingContext],
) -> Option<&'a DirectiveMeetingContext> {
    if let Some(acct) = account {
        for ctx in contexts {
            if ctx.account.as_deref() == Some(acct) {
                return Some(ctx);
            }
        }
    }
    if let Some(eid) = event_id {
        for ctx in contexts {
            if ctx.event_id.as_deref() == Some(eid) {
                return Some(ctx);
            }
        }
    }
    None
}

/// Check whether a calendar event is currently in progress.
fn is_meeting_current(event: &DirectiveEvent, now: DateTime<Utc>) -> bool {
    let start = event.start.as_deref().and_then(parse_iso_dt);
    let end = event.end.as_deref().and_then(parse_iso_dt);
    match (start, end) {
        (Some(s), Some(e)) => s <= now && now <= e,
        _ => false,
    }
}

fn parse_iso_dt(s: &str) -> Option<DateTime<Utc>> {
    if s.is_empty() || !s.contains('T') {
        return None;
    }
    DateTime::parse_from_rfc3339(&s.replace('Z', "+00:00"))
        .or_else(|_| DateTime::parse_from_rfc3339(s))
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

/// Build a condensed prep summary for embedding in schedule.json.
fn build_prep_summary(ctx: &DirectiveMeetingContext) -> Option<Value> {
    let account_data = ctx.account_data.as_ref().and_then(|v| v.as_object());

    let mut at_a_glance: Vec<String> = Vec::new();
    if let Some(data) = account_data {
        for (key, label) in &[
            ("ring", "Ring"),
            ("arr", "ARR"),
            ("renewal", "Renewal"),
            ("health", "Health"),
        ] {
            if let Some(val) = data.get(*key).and_then(|v| v.as_str()) {
                if !val.is_empty() {
                    at_a_glance.push(format!("{}: {}", label, val));
                }
            }
        }
    }

    let discuss: Vec<&str> = ctx
        .talking_points
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .take(4)
        .map(|s| s.as_str())
        .collect();
    let watch: Vec<&str> = ctx
        .risks
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .take(3)
        .map(|s| s.as_str())
        .collect();
    let wins: Vec<&str> = ctx
        .wins
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .take(3)
        .map(|s| s.as_str())
        .collect();

    if at_a_glance.is_empty() && discuss.is_empty() && watch.is_empty() && wins.is_empty() {
        return None;
    }

    Some(json!({
        "atAGlance": &at_a_glance[..at_a_glance.len().min(4)],
        "discuss": discuss,
        "watch": watch,
        "wins": wins,
    }))
}

/// Return a time-appropriate greeting.
fn greeting_for_hour(hour: u32) -> &'static str {
    if hour < 12 {
        "Good morning"
    } else if hour < 17 {
        "Good afternoon"
    } else {
        "Good evening"
    }
}

/// Generate a content-stable action ID (same logic as Python _make_id).
fn make_action_id(title: &str, account: &str, due: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let key = format!(
        "{}|{}|{}",
        title.to_lowercase().trim(),
        account.to_lowercase().trim(),
        due.trim()
    );
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    format!("action-{:010x}", hasher.finish() & 0xFF_FFFF_FFFF)
}

/// Write JSON to a file with pretty printing.
pub fn write_json(path: &Path, data: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory {}: {}", parent.display(), e))?;
    }
    let content = serde_json::to_string_pretty(data)
        .map_err(|e| format!("Failed to serialize JSON: {}", e))?;
    fs::write(path, format!("{}\n", content))
        .map_err(|e| format!("Failed to write {}: {}", path.display(), e))
}

// ============================================================================
// Core delivery functions
// ============================================================================

/// Build and write schedule.json from directive data.
///
/// Returns the schedule JSON value (needed by manifest builder).
pub fn deliver_schedule(directive: &Directive, data_dir: &Path) -> Result<Value, String> {
    let now = Utc::now();
    let date = directive
        .context
        .date
        .clone()
        .unwrap_or_else(|| now.format("%Y-%m-%d").to_string());

    let events = &directive.calendar.events;
    let meetings_by_type = &directive.meetings;
    let meeting_contexts = &directive.meeting_contexts;

    let mut meetings_json: Vec<Value> = Vec::new();

    for event in events {
        let event_id = event.id.as_deref().unwrap_or("");
        let meeting_type = classify_event(event_id, meetings_by_type);

        // Skip personal events
        if meeting_type == "personal" {
            continue;
        }

        let (_meeting_entry, account) = find_meeting_entry(event_id, meetings_by_type);
        let summary = event.summary.as_deref().unwrap_or("No title");
        let start = event.start.as_deref().unwrap_or("");
        let end = event.end.as_deref().unwrap_or("");
        let meeting_id = make_meeting_id(summary, start, meeting_type);

        let mc = find_meeting_context(account.as_deref(), Some(event_id), meeting_contexts);
        let prep_summary = mc.and_then(build_prep_summary);
        let has_prep = PREP_ELIGIBLE_TYPES.contains(&meeting_type) && mc.is_some();
        let prep_file = if has_prep {
            Some(format!("preps/{}.json", meeting_id))
        } else {
            None
        };

        let mut meeting_obj = json!({
            "id": meeting_id,
            "calendarEventId": event.id,
            "time": format_time_display(start),
            "title": summary,
            "type": meeting_type,
            "hasPrep": has_prep,
            "isCurrent": is_meeting_current(event, now),
        });

        if let Some(obj) = meeting_obj.as_object_mut() {
            if !end.is_empty() {
                obj.insert("endTime".to_string(), json!(format_time_display(end)));
            }
            if let Some(ref acct) = account {
                obj.insert("account".to_string(), json!(acct));
            }
            if let Some(ref pf) = prep_file {
                obj.insert("prepFile".to_string(), json!(pf));
            }
            if let Some(ref ps) = prep_summary {
                obj.insert("prepSummary".to_string(), ps.clone());
            }
        }

        meetings_json.push(meeting_obj);
    }

    // Build greeting and summary
    let greeting = directive
        .context
        .greeting
        .clone()
        .unwrap_or_else(|| greeting_for_hour(now.hour()).to_string());

    let summary = directive.context.summary.clone().unwrap_or_else(|| {
        let total = meetings_json.len();
        let customer_count = meetings_json
            .iter()
            .filter(|m| {
                let t = m.get("type").and_then(|v| v.as_str()).unwrap_or("");
                t == "customer" || t == "qbr"
            })
            .count();
        let mut parts = vec![format!(
            "{} meeting{} today",
            total,
            if total != 1 { "s" } else { "" }
        )];
        if customer_count > 0 {
            parts.push(format!(
                "{} customer call{}",
                customer_count,
                if customer_count != 1 { "s" } else { "" }
            ));
        }
        parts.join(" with ")
    });

    let mut schedule = json!({
        "date": date,
        "greeting": greeting,
        "summary": summary,
        "meetings": meetings_json,
    });

    if let Some(ref focus) = directive.context.focus {
        schedule
            .as_object_mut()
            .unwrap()
            .insert("focus".to_string(), json!(focus));
    }

    write_json(&data_dir.join("schedule.json"), &schedule)?;
    log::info!(
        "deliver_schedule: {} meetings written",
        meetings_json.len()
    );
    Ok(schedule)
}

/// Build and write actions.json from directive data.
///
/// Returns the actions JSON value (needed by manifest builder).
pub fn deliver_actions(
    directive: &Directive,
    data_dir: &Path,
    db: Option<&crate::db::ActionDb>,
) -> Result<Value, String> {
    let now = Utc::now();
    let date = directive
        .context
        .date
        .clone()
        .unwrap_or_else(|| now.format("%Y-%m-%d").to_string());

    let raw = &directive.actions;

    // Load existing action titles from SQLite to skip duplicates (I23)
    let existing_titles: std::collections::HashSet<String> = db
        .and_then(|db| {
            db.get_all_action_titles().ok().map(|titles| {
                titles
                    .into_iter()
                    .collect()
            })
        })
        .unwrap_or_default();

    let mut actions_list: Vec<Value> = Vec::new();
    let mut seen_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

    let mut add_action =
        |title: &str,
         account: &str,
         priority: &str,
         status: &str,
         due: &str,
         is_overdue: bool,
         days_overdue: Option<u32>,
         context: &Option<String>,
         source: &Option<String>| {
            if title.to_lowercase().trim().is_empty() {
                return;
            }
            if existing_titles.contains(&title.to_lowercase().trim().to_string()) {
                return;
            }
            let id = make_action_id(title, account, due);
            if seen_ids.contains(&id) {
                return;
            }
            seen_ids.insert(id.clone());

            let mut obj = json!({
                "id": id,
                "title": title,
                "priority": priority,
                "status": status,
                "isOverdue": is_overdue,
            });
            if let Some(map) = obj.as_object_mut() {
                if !account.is_empty() {
                    map.insert("account".to_string(), json!(account));
                }
                if !due.is_empty() {
                    map.insert("dueDate".to_string(), json!(due));
                }
                if let Some(d) = days_overdue {
                    map.insert("daysOverdue".to_string(), json!(d));
                }
                if let Some(ref c) = context {
                    map.insert("context".to_string(), json!(c));
                }
                if let Some(ref s) = source {
                    map.insert("source".to_string(), json!(s));
                }
            }
            actions_list.push(obj);
        };

    // Overdue → P1
    for task in &raw.overdue {
        let title = task.title.as_deref().unwrap_or("Unknown");
        let account = task.account.as_deref().unwrap_or("");
        let due = task.effective_due_date().unwrap_or("");
        add_action(
            title,
            account,
            "P1",
            "pending",
            due,
            true,
            task.days_overdue,
            &task.context,
            &task.source,
        );
    }

    // Due today → P1
    for task in &raw.due_today {
        let title = task.title.as_deref().unwrap_or("Unknown");
        let account = task.account.as_deref().unwrap_or("");
        let due = task.effective_due_date().unwrap_or("");
        add_action(
            title, account, "P1", "pending", due, false, None, &task.context, &task.source,
        );
    }

    // Due this week → P2
    for task in &raw.due_this_week {
        let title = task.title.as_deref().unwrap_or("Unknown");
        let account = task.account.as_deref().unwrap_or("");
        let due = task.effective_due_date().unwrap_or("");
        add_action(
            title, account, "P2", "pending", due, false, None, &task.context, &task.source,
        );
    }

    // Waiting on → P2
    for item in &raw.waiting_on {
        let what = item.what.as_deref().unwrap_or("Unknown");
        let title = format!("Waiting: {}", what);
        let who = item.who.as_deref().unwrap_or("");
        add_action(&title, who, "P2", "waiting", "", false, None, &item.context, &None);
    }

    let actions_data = json!({
        "date": date,
        "summary": {
            "overdue": raw.overdue.len(),
            "dueToday": raw.due_today.len(),
            "dueThisWeek": raw.due_this_week.len(),
            "waitingOn": raw.waiting_on.len(),
        },
        "actions": actions_list,
    });

    write_json(&data_dir.join("actions.json"), &actions_data)?;
    log::info!("deliver_actions: {} actions written", actions_list.len());
    Ok(actions_data)
}

/// Build and write preps/*.json from directive data.
///
/// Returns list of relative prep file paths (e.g. "preps/0900-customer-acme.json").
pub fn deliver_preps(directive: &Directive, data_dir: &Path) -> Result<Vec<String>, String> {
    let preps_dir = data_dir.join("preps");
    fs::create_dir_all(&preps_dir)
        .map_err(|e| format!("Failed to create preps dir: {}", e))?;

    // Clear stale prep files
    if let Ok(entries) = fs::read_dir(&preps_dir) {
        for entry in entries.flatten() {
            if entry
                .file_name()
                .to_str()
                .is_some_and(|n| n.ends_with(".json"))
            {
                let _ = fs::remove_file(entry.path());
            }
        }
    }

    let events = &directive.calendar.events;
    let meetings_by_type = &directive.meetings;
    let meeting_contexts = &directive.meeting_contexts;
    let mut prep_paths: Vec<String> = Vec::new();

    for (mtype, meeting_list) in meetings_by_type {
        let normalised_type = normalise_meeting_type(mtype);

        if normalised_type == "personal" {
            continue;
        }

        for meeting in meeting_list {
            let account = meeting.account.as_deref();
            let event_id = meeting.event_id.as_deref().or(meeting.id.as_deref());
            let mc = find_meeting_context(account, event_id, meeting_contexts);

            // Only write a prep file if there is meaningful context
            if mc.is_none() && account.is_none() {
                continue;
            }

            // Find matching calendar event for stable ID
            let matched_event = event_id
                .and_then(|eid| events.iter().find(|e| e.id.as_deref() == Some(eid)));

            let meeting_id = if let Some(ev) = matched_event {
                make_meeting_id(
                    ev.summary.as_deref().unwrap_or("meeting"),
                    ev.start.as_deref().unwrap_or(""),
                    normalised_type,
                )
            } else {
                let title = meeting
                    .title
                    .as_deref()
                    .or(meeting.summary.as_deref())
                    .or(account)
                    .unwrap_or("meeting");
                let start = meeting
                    .start_display
                    .as_deref()
                    .or(meeting.start.as_deref())
                    .unwrap_or("");
                let time_part: String =
                    start.chars().filter(|c| c.is_ascii_digit()).take(4).collect();
                let time_part = if time_part.is_empty() {
                    "0000".to_string()
                } else {
                    time_part
                };
                let slug_re = Regex::new(r"[^a-z0-9]+").unwrap();
                let lower = title.to_lowercase();
                let slug = slug_re.replace_all(&lower, "-");
                let slug = slug.trim_matches('-');
                let slug = if slug.len() > 40 { &slug[..40] } else { slug };
                format!("{}-{}-{}", time_part, normalised_type, slug)
            };

            let prep_data = build_prep_json(meeting, normalised_type, &meeting_id, mc);
            let rel_path = format!("preps/{}.json", meeting_id);

            write_json(&data_dir.join(&rel_path), &prep_data)?;
            prep_paths.push(rel_path);
        }
    }

    log::info!("deliver_preps: {} prep files written", prep_paths.len());
    Ok(prep_paths)
}

/// Build a single prep JSON object (matches JsonPrep in json_loader.rs).
fn build_prep_json(
    meeting: &DirectiveMeeting,
    meeting_type: &str,
    meeting_id: &str,
    ctx: Option<&DirectiveMeetingContext>,
) -> Value {
    let account = meeting.account.as_deref();

    // Quick context from account data
    let mut quick_context: serde_json::Map<String, Value> = serde_json::Map::new();
    if let Some(ctx) = ctx {
        if let Some(data) = ctx.account_data.as_ref().and_then(|v| v.as_object()) {
            let labels: &[(&str, &str)] = &[
                ("ring", "Ring"),
                ("arr", "ARR"),
                ("renewal", "Renewal"),
                ("health", "Health"),
                ("tier", "Tier"),
                ("csm", "CSM"),
                ("stage", "Stage"),
            ];
            for (key, label) in labels {
                if let Some(val) = data.get(*key).and_then(|v| v.as_str()) {
                    if !val.is_empty() {
                        quick_context.insert(label.to_string(), json!(val));
                    }
                }
            }
        }
    }

    // Attendees
    let attendees: Vec<Value> = ctx
        .and_then(|c| c.attendees.as_ref())
        .map(|att_list| {
            att_list
                .iter()
                .map(|a| {
                    let name = a
                        .get("name")
                        .or_else(|| a.get("email"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown");
                    let mut entry = json!({"name": name});
                    if let Some(obj) = entry.as_object_mut() {
                        if let Some(role) = a.get("role").and_then(|v| v.as_str()) {
                            obj.insert("role".to_string(), json!(role));
                        }
                        if let Some(focus) = a.get("focus").and_then(|v| v.as_str()) {
                            obj.insert("focus".to_string(), json!(focus));
                        }
                    }
                    entry
                })
                .collect()
        })
        .unwrap_or_default();

    // Time range
    let start_display = meeting.start_display.as_deref().unwrap_or("");
    let end_display = meeting.end_display.as_deref().unwrap_or("");
    let time_range = if !start_display.is_empty() && !end_display.is_empty() {
        format!("{} - {}", start_display, end_display)
    } else {
        start_display.to_string()
    };

    let mut prep = json!({
        "meetingId": meeting_id,
        "calendarEventId": meeting.event_id.as_deref().or(meeting.id.as_deref()),
        "title": meeting.title.as_deref().or(meeting.summary.as_deref()).unwrap_or("Meeting"),
        "type": meeting_type,
    });
    if let Some(obj) = prep.as_object_mut() {
        if !time_range.is_empty() {
            obj.insert("timeRange".to_string(), json!(time_range));
        }
        if let Some(acct) = account {
            obj.insert("account".to_string(), json!(acct));
        }
        if !quick_context.is_empty() {
            obj.insert("quickContext".to_string(), Value::Object(quick_context));
        }
        if !attendees.is_empty() {
            obj.insert("attendees".to_string(), json!(attendees));
        }

        if let Some(ctx) = ctx {
            if let Some(ref narrative) = ctx.narrative {
                obj.insert("meetingContext".to_string(), json!(narrative));
            }
            if let Some(ref v) = ctx.since_last {
                obj.insert("sinceLast".to_string(), json!(v));
            }
            if let Some(ref v) = ctx.current_state {
                obj.insert("currentState".to_string(), json!(v));
            }
            if let Some(ref v) = ctx.risks {
                obj.insert("risks".to_string(), json!(v));
            }
            if let Some(ref v) = ctx.talking_points {
                obj.insert("talkingPoints".to_string(), json!(v));
            }
            if let Some(ref v) = ctx.questions {
                obj.insert("questions".to_string(), json!(v));
            }
            if let Some(ref v) = ctx.key_principles {
                obj.insert("keyPrinciples".to_string(), json!(v));
            }

            // Strategic programs → array of {name, status}
            if let Some(ref programs) = ctx.strategic_programs {
                let prog_json: Vec<Value> = programs
                    .iter()
                    .map(|p| {
                        if let Some(pobj) = p.as_object() {
                            json!({
                                "name": pobj.get("name").and_then(|v| v.as_str()).unwrap_or(&p.to_string()),
                                "status": pobj.get("status").and_then(|v| v.as_str()).unwrap_or("in_progress"),
                            })
                        } else {
                            json!({"name": p.to_string().trim_matches('"'), "status": "in_progress"})
                        }
                    })
                    .collect();
                obj.insert("strategicPrograms".to_string(), json!(prog_json));
            }

            // Open items → array of {title, dueDate, context, isOverdue}
            if let Some(ref items) = ctx.open_items {
                let items_json: Vec<Value> = items
                    .iter()
                    .map(|item| {
                        if let Some(o) = item.as_object() {
                            json!({
                                "title": o.get("title").and_then(|v| v.as_str()).unwrap_or(&item.to_string()),
                                "dueDate": o.get("due_date").and_then(|v| v.as_str()),
                                "context": o.get("context").and_then(|v| v.as_str()),
                                "isOverdue": o.get("is_overdue").and_then(|v| v.as_bool()).unwrap_or(false),
                            })
                        } else {
                            json!({"title": item.to_string().trim_matches('"'), "isOverdue": false})
                        }
                    })
                    .collect();
                obj.insert("openItems".to_string(), json!(items_json));
            }

            // References → array of {label, path, lastUpdated}
            if let Some(ref refs) = ctx.references {
                let refs_json: Vec<Value> = refs
                    .iter()
                    .map(|r| {
                        if let Some(o) = r.as_object() {
                            json!({
                                "label": o.get("label").and_then(|v| v.as_str()).unwrap_or(&r.to_string()),
                                "path": o.get("path").and_then(|v| v.as_str()),
                                "lastUpdated": o.get("last_updated").and_then(|v| v.as_str()),
                            })
                        } else {
                            json!({"label": r.to_string().trim_matches('"')})
                        }
                    })
                    .collect();
                obj.insert("references".to_string(), json!(refs_json));
            }
        }
    }

    // Proposed agenda (mechanical synthesis from prep ingredients)
    // Done after all other fields are inserted so we can read them back.
    let agenda = generate_mechanical_agenda(&prep);
    if !agenda.is_empty() {
        if let Some(obj) = prep.as_object_mut() {
            obj.insert("proposedAgenda".to_string(), json!(agenda));
        }
    }

    prep
}

/// Generate a mechanical agenda from existing prep data.
///
/// Synthesizes an agenda from open items (overdue first), risks,
/// talking points, and questions. Caps at 7 items. No AI needed.
fn generate_mechanical_agenda(prep: &Value) -> Vec<Value> {
    let mut agenda: Vec<Value> = Vec::new();
    const MAX_ITEMS: usize = 7;

    // 1. Overdue items first (most urgent)
    if let Some(items) = prep.get("openItems").and_then(|v| v.as_array()) {
        for item in items {
            if agenda.len() >= MAX_ITEMS {
                break;
            }
            let is_overdue = item.get("isOverdue").and_then(|v| v.as_bool()).unwrap_or(false);
            if is_overdue {
                let title = item.get("title").and_then(|v| v.as_str()).unwrap_or("Unknown item");
                agenda.push(json!({
                    "topic": format!("Follow up: {}", title),
                    "why": "Overdue — needs resolution",
                    "source": "open_item",
                }));
            }
        }
    }

    // 2. Risks (limit 2)
    if let Some(risks) = prep.get("risks").and_then(|v| v.as_array()) {
        for risk in risks.iter().take(2) {
            if agenda.len() >= MAX_ITEMS {
                break;
            }
            if let Some(text) = risk.as_str() {
                agenda.push(json!({
                    "topic": text,
                    "source": "risk",
                }));
            }
        }
    }

    // 3. Talking points (limit 3)
    if let Some(points) = prep.get("talkingPoints").and_then(|v| v.as_array()) {
        for point in points.iter().take(3) {
            if agenda.len() >= MAX_ITEMS {
                break;
            }
            if let Some(text) = point.as_str() {
                agenda.push(json!({
                    "topic": text,
                    "source": "talking_point",
                }));
            }
        }
    }

    // 4. Questions (limit 2)
    if let Some(questions) = prep.get("questions").and_then(|v| v.as_array()) {
        for q in questions.iter().take(2) {
            if agenda.len() >= MAX_ITEMS {
                break;
            }
            if let Some(text) = q.as_str() {
                agenda.push(json!({
                    "topic": text,
                    "source": "question",
                }));
            }
        }
    }

    // 5. Non-overdue open items (limit 2)
    if let Some(items) = prep.get("openItems").and_then(|v| v.as_array()) {
        for item in items.iter().take(4) {
            if agenda.len() >= MAX_ITEMS {
                break;
            }
            let is_overdue = item.get("isOverdue").and_then(|v| v.as_bool()).unwrap_or(false);
            if !is_overdue {
                let title = item.get("title").and_then(|v| v.as_str()).unwrap_or("Unknown item");
                agenda.push(json!({
                    "topic": title,
                    "source": "open_item",
                }));
            }
        }
    }

    // Truncate to max
    agenda.truncate(MAX_ITEMS);
    agenda
}

/// Build and write emails.json from directive data.
///
/// Maps `directive.emails` (classified + high_priority) to the frontend
/// `Email` type. This is a mechanical op — no AI needed.
///
/// Returns the emails JSON value (needed by manifest builder).
pub fn deliver_emails(directive: &Directive, data_dir: &Path) -> Result<Value, String> {
    let emails = &directive.emails;

    // Build high-priority email objects from both sources, deduplicating by ID
    let mut seen_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut high_priority: Vec<Value> = Vec::new();

    let mut add_email = |email: &DirectiveEmail, priority: &str| {
        let id = email
            .id
            .clone()
            .unwrap_or_else(|| format!("email-{}", seen_ids.len()));
        if seen_ids.contains(&id) {
            return;
        }
        seen_ids.insert(id.clone());

        high_priority.push(json!({
            "id": id,
            "sender": email.from.as_deref().unwrap_or("Unknown"),
            "senderEmail": email.from_email.as_deref().unwrap_or(""),
            "subject": email.subject.as_deref().unwrap_or("(no subject)"),
            "snippet": email.snippet,
            "priority": priority,
        }));
    };

    // High-priority emails first
    for email in &emails.high_priority {
        add_email(email, "high");
    }

    // Classified emails that are high priority (avoid duplicating)
    for email in &emails.classified {
        let prio = email.priority.as_deref().unwrap_or("medium");
        if prio == "high" {
            add_email(email, "high");
        }
    }

    let high_count = high_priority.len();

    // Count medium emails from classified list
    let medium_from_classified = emails
        .classified
        .iter()
        .filter(|e| e.priority.as_deref() == Some("medium"))
        .count() as u32;
    let medium_count = emails.medium_count.max(medium_from_classified);

    let low_count = emails.low_count;
    let total = high_count as u32 + medium_count + low_count;

    let emails_data = json!({
        "highPriority": high_priority,
        "stats": {
            "highCount": high_count,
            "mediumCount": medium_count,
            "lowCount": low_count,
            "total": total,
        },
    });

    write_json(&data_dir.join("emails.json"), &emails_data)?;
    log::info!(
        "deliver_emails: {} high-priority, {} total",
        high_count,
        total
    );
    Ok(emails_data)
}

// ============================================================================
// AI enrichment (progressive, fault-tolerant)
// ============================================================================

/// Parsed enrichment for a single email.
#[derive(Debug, Clone, Default)]
pub struct EmailEnrichment {
    pub summary: Option<String>,
    pub action: Option<String>,
    pub arc: Option<String>,
}

/// Parse Claude's email enrichment response.
///
/// Expected format per email:
/// ```text
/// ENRICHMENT:email-id
/// SUMMARY: one-line summary
/// ACTION: recommended next action
/// ARC: conversation context
/// END_ENRICHMENT
/// ```
pub fn parse_email_enrichment(response: &str) -> HashMap<String, EmailEnrichment> {
    let mut result: HashMap<String, EmailEnrichment> = HashMap::new();
    let mut current_id: Option<String> = None;
    let mut current = EmailEnrichment::default();

    for line in response.lines() {
        let trimmed = line.trim();

        if let Some(id) = trimmed.strip_prefix("ENRICHMENT:") {
            // Start a new enrichment block
            current_id = Some(id.trim().to_string());
            current = EmailEnrichment::default();
        } else if trimmed == "END_ENRICHMENT" {
            // Close the current block
            if let Some(ref id) = current_id {
                result.insert(id.clone(), current.clone());
            }
            current_id = None;
            current = EmailEnrichment::default();
        } else if current_id.is_some() {
            // Inside a block — parse fields
            if let Some(val) = trimmed.strip_prefix("SUMMARY:") {
                current.summary = Some(val.trim().to_string());
            } else if let Some(val) = trimmed.strip_prefix("ACTION:") {
                current.action = Some(val.trim().to_string());
            } else if let Some(val) = trimmed.strip_prefix("ARC:") {
                current.arc = Some(val.trim().to_string());
            }
        }
    }

    result
}

/// AI-enrich high-priority emails via PTY-spawned Claude.
///
/// Reads `emails.json`, asks Claude for summaries/actions/arcs,
/// merges enrichments back. If AI fails, emails.json stays unenriched.
pub fn enrich_emails(
    data_dir: &Path,
    pty: &crate::pty::PtyManager,
    workspace: &Path,
    user_ctx: &crate::types::UserContext,
) -> Result<(), String> {
    let emails_path = data_dir.join("emails.json");
    let raw = fs::read_to_string(&emails_path)
        .map_err(|e| format!("Failed to read emails.json: {}", e))?;
    let mut emails_data: Value =
        serde_json::from_str(&raw).map_err(|e| format!("Failed to parse emails.json: {}", e))?;

    let high_priority = emails_data
        .get("highPriority")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    if high_priority.is_empty() {
        log::info!("enrich_emails: no high-priority emails to enrich");
        return Ok(());
    }

    // Build context for Claude
    let mut email_context = String::new();
    for email in &high_priority {
        let id = email.get("id").and_then(|v| v.as_str()).unwrap_or("?");
        let sender = email.get("sender").and_then(|v| v.as_str()).unwrap_or("?");
        let subject = email
            .get("subject")
            .and_then(|v| v.as_str())
            .unwrap_or("?");
        let snippet = email.get("snippet").and_then(|v| v.as_str()).unwrap_or("");
        email_context.push_str(&format!(
            "ID: {}\nFrom: {}\nSubject: {}\nSnippet: {}\n\n",
            id, sender, subject, snippet
        ));
    }

    // Write context file
    let context_path = data_dir.join(".email-context.json");
    let context_json = json!({ "emails": high_priority });
    write_json(&context_path, &context_json)?;

    let user_fragment = user_ctx.prompt_fragment();
    let prompt = format!(
        "You are enriching email briefing data. {}\
         For each email below, provide a one-line summary, \
         a recommended action, and brief conversation arc context.\n\n\
         Format your response as:\n\
         ENRICHMENT:email-id-here\n\
         SUMMARY: <one-line summary>\n\
         ACTION: <recommended next action>\n\
         ARC: <conversation context>\n\
         END_ENRICHMENT\n\n\
         {}",
        user_fragment, email_context
    );

    let output = pty
        .spawn_claude(workspace, &prompt)
        .map_err(|e| format!("Claude enrichment failed: {}", e))?;

    let enrichments = parse_email_enrichment(&output.stdout);
    if enrichments.is_empty() {
        log::warn!("enrich_emails: no enrichments parsed from Claude output");
        // Clean up context file
        let _ = fs::remove_file(&context_path);
        return Ok(());
    }

    // Merge enrichments into emails.json
    if let Some(hp) = emails_data.get_mut("highPriority").and_then(|v| v.as_array_mut()) {
        for email in hp.iter_mut() {
            let id = email.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if let Some(enrichment) = enrichments.get(id) {
                if let Some(obj) = email.as_object_mut() {
                    if let Some(ref s) = enrichment.summary {
                        obj.insert("summary".to_string(), json!(s));
                    }
                    if let Some(ref a) = enrichment.action {
                        obj.insert("recommendedAction".to_string(), json!(a));
                    }
                    if let Some(ref arc) = enrichment.arc {
                        obj.insert("conversationArc".to_string(), json!(arc));
                    }
                }
            }
        }
    }

    write_json(&emails_path, &emails_data)?;
    let _ = fs::remove_file(&context_path);
    log::info!(
        "enrich_emails: enriched {}/{} emails",
        enrichments.len(),
        high_priority.len()
    );
    Ok(())
}

/// Parse Claude's briefing narrative response.
///
/// Expected format:
/// ```text
/// NARRATIVE:
/// 2-3 sentence narrative here.
/// END_NARRATIVE
/// ```
pub fn parse_briefing_narrative(response: &str) -> Option<String> {
    let mut in_block = false;
    let mut lines: Vec<&str> = Vec::new();

    for line in response.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("NARRATIVE:") {
            in_block = true;
            // Check if there's content on the same line after the marker
            let after = trimmed.strip_prefix("NARRATIVE:").unwrap().trim();
            if !after.is_empty() {
                lines.push(after);
            }
        } else if trimmed == "END_NARRATIVE" {
            break;
        } else if in_block {
            lines.push(trimmed);
        }
    }

    if lines.is_empty() {
        return None;
    }

    let narrative = lines.join(" ").trim().to_string();
    if narrative.is_empty() {
        None
    } else {
        Some(narrative)
    }
}

/// Classify meeting density for briefing tone adaptation (I37).
///
/// Returns a density label based on meeting count:
/// - 0–2: "light"
/// - 3–5: "moderate"
/// - 6–8: "busy"
/// - 9+:  "packed"
fn classify_meeting_density(count: usize) -> &'static str {
    match count {
        0..=2 => "light",
        3..=5 => "moderate",
        6..=8 => "busy",
        _ => "packed",
    }
}

/// AI-generate a briefing narrative via PTY-spawned Claude.
///
/// Reads schedule.json + actions.json + emails.json to build context,
/// asks Claude for a 2-3 sentence narrative, patches schedule.json.
/// If AI fails, schedule.json keeps its mechanical greeting/summary.
pub fn enrich_briefing(
    data_dir: &Path,
    pty: &crate::pty::PtyManager,
    workspace: &Path,
    user_ctx: &crate::types::UserContext,
) -> Result<(), String> {
    // Read context files
    let schedule_raw = fs::read_to_string(data_dir.join("schedule.json"))
        .map_err(|e| format!("Failed to read schedule.json: {}", e))?;
    let mut schedule: Value = serde_json::from_str(&schedule_raw)
        .map_err(|e| format!("Failed to parse schedule.json: {}", e))?;

    let actions_raw = fs::read_to_string(data_dir.join("actions.json")).unwrap_or_default();
    let actions: Value = serde_json::from_str(&actions_raw).unwrap_or(json!({}));

    let emails_raw = fs::read_to_string(data_dir.join("emails.json")).unwrap_or_default();
    let emails: Value = serde_json::from_str(&emails_raw).unwrap_or(json!({}));

    // Extract context for prompt
    let date = schedule
        .get("date")
        .and_then(|v| v.as_str())
        .unwrap_or("today");
    let meetings = schedule
        .get("meetings")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let customer_count = schedule
        .get("meetings")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter(|m| {
                    let t = m.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    t == "customer" || t == "qbr"
                })
                .count()
        })
        .unwrap_or(0);

    let top_meetings: Vec<String> = schedule
        .get("meetings")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .take(3)
                .filter_map(|m| m.get("title").and_then(|v| v.as_str()).map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let overdue_count = actions
        .get("summary")
        .and_then(|s| s.get("overdue"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let due_today = actions
        .get("summary")
        .and_then(|s| s.get("dueToday"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let high_count = emails
        .get("stats")
        .and_then(|s| s.get("highCount"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    // Density classification (I37)
    let density = classify_meeting_density(meetings);

    // First meeting time for context
    let first_meeting_time = schedule
        .get("meetings")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|m| m.get("time").and_then(|v| v.as_str()))
        .unwrap_or("N/A");

    // Write context file
    let context = json!({
        "date": date,
        "meetings": meetings,
        "customerMeetings": customer_count,
        "topMeetings": top_meetings,
        "overdueActions": overdue_count,
        "dueToday": due_today,
        "highPriorityEmails": high_count,
        "density": density,
        "firstMeetingTime": first_meeting_time,
    });
    let context_path = data_dir.join(".briefing-context.json");
    write_json(&context_path, &context)?;

    let density_guidance = match density {
        "light" => "This is a light day. Highlight available open time and suggest tackling overdue items or deep work.",
        "moderate" => "This is a balanced day. Note customer commitments and any gaps worth protecting.",
        "busy" => "This is a busy day. Focus on the 1-2 highest-stakes meetings and what to prioritize.",
        "packed" => "This is a packed day. Triage mode — identify what can be skipped, delegated, or deferred.",
        _ => "",
    };

    let user_fragment = user_ctx.prompt_fragment();
    let role_label = user_ctx.title_or_default();
    let prompt = format!(
        "You are writing a morning briefing narrative for {role_label}.\n\
         {user_fragment}\n\
         Today's context:\n\
         - Date: {}\n\
         - Meetings: {} ({} customer) — density: {}\n\
         - First meeting: {}\n\
         - Key meetings: {}\n\
         - Actions: {} overdue, {} due today\n\
         - Emails: {} high-priority\n\n\
         {}\n\n\
         Write a 2-3 sentence narrative that helps them understand the shape of their day.\n\
         Focus on what matters most — customer calls, overdue items, important emails.\n\
         Be direct, not chatty.\n\n\
         NARRATIVE:\n\
         <your narrative here>\n\
         END_NARRATIVE",
        date,
        meetings,
        customer_count,
        density,
        first_meeting_time,
        top_meetings.join(", "),
        overdue_count,
        due_today,
        high_count,
        density_guidance,
    );

    let output = pty
        .spawn_claude(workspace, &prompt)
        .map_err(|e| format!("Claude briefing failed: {}", e))?;

    let narrative = parse_briefing_narrative(&output.stdout);
    let _ = fs::remove_file(&context_path);

    match narrative {
        Some(text) => {
            schedule
                .as_object_mut()
                .unwrap()
                .insert("narrative".to_string(), json!(text));
            write_json(&data_dir.join("schedule.json"), &schedule)?;
            log::info!("enrich_briefing: narrative written ({} chars)", text.len());
            Ok(())
        }
        None => {
            log::warn!("enrich_briefing: no narrative parsed from Claude output");
            Ok(())
        }
    }
}

/// Parsed enrichment for a single agenda item.
#[derive(Debug, Clone, Default)]
pub struct AgendaItemEnrichment {
    pub topic: String,
    pub why: Option<String>,
    pub source: Option<String>,
}

/// Parse Claude's agenda enrichment response.
///
/// Expected format:
/// ```text
/// AGENDA:meeting-id
/// ITEM:topic text here
/// WHY:rationale for discussing this
/// SOURCE:risk
/// END_ITEM
/// ITEM:another topic
/// WHY:another rationale
/// SOURCE:talking_point
/// END_ITEM
/// END_AGENDA
/// ```
pub fn parse_agenda_enrichment(response: &str) -> HashMap<String, Vec<AgendaItemEnrichment>> {
    let mut result: HashMap<String, Vec<AgendaItemEnrichment>> = HashMap::new();
    let mut current_meeting: Option<String> = None;
    let mut current_items: Vec<AgendaItemEnrichment> = Vec::new();
    let mut current_item: Option<AgendaItemEnrichment> = None;

    for line in response.lines() {
        let trimmed = line.trim();

        if let Some(id) = trimmed.strip_prefix("AGENDA:") {
            current_meeting = Some(id.trim().to_string());
            current_items = Vec::new();
        } else if trimmed == "END_AGENDA" {
            if let Some(ref id) = current_meeting {
                if !current_items.is_empty() {
                    result.insert(id.clone(), current_items.clone());
                }
            }
            current_meeting = None;
            current_items = Vec::new();
        } else if current_meeting.is_some() {
            if let Some(topic) = trimmed.strip_prefix("ITEM:") {
                // Start a new item
                current_item = Some(AgendaItemEnrichment {
                    topic: topic.trim().to_string(),
                    ..Default::default()
                });
            } else if trimmed == "END_ITEM" {
                if let Some(item) = current_item.take() {
                    if !item.topic.is_empty() {
                        current_items.push(item);
                    }
                }
            } else if let Some(ref mut item) = current_item {
                if let Some(val) = trimmed.strip_prefix("WHY:") {
                    item.why = Some(val.trim().to_string());
                } else if let Some(val) = trimmed.strip_prefix("SOURCE:") {
                    item.source = Some(val.trim().to_string());
                }
            }
        }
    }

    result
}

/// AI-enrich prep agendas via PTY-spawned Claude.
///
/// Reads each prep JSON in data_dir/preps/, builds context from the prep
/// ingredients + current mechanical agenda, asks Claude to refine agenda
/// ordering and add rationale. If AI fails, mechanical agenda stays intact.
pub fn enrich_preps(
    data_dir: &Path,
    pty: &crate::pty::PtyManager,
    workspace: &Path,
) -> Result<(), String> {
    let preps_dir = data_dir.join("preps");
    if !preps_dir.exists() {
        log::info!("enrich_preps: no preps directory, skipping");
        return Ok(());
    }

    // Collect prep files
    let prep_files: Vec<std::path::PathBuf> = fs::read_dir(&preps_dir)
        .map_err(|e| format!("Failed to read preps dir: {}", e))?
        .flatten()
        .filter(|e| e.file_name().to_str().is_some_and(|n| n.ends_with(".json")))
        .map(|e| e.path())
        .collect();

    if prep_files.is_empty() {
        log::info!("enrich_preps: no prep files to enrich");
        return Ok(());
    }

    // Build combined context for all preps
    let mut prep_context = String::new();
    let mut meeting_ids: Vec<String> = Vec::new();

    for path in &prep_files {
        let raw = match fs::read_to_string(path) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let prep: Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let meeting_id = prep.get("meetingId").and_then(|v| v.as_str()).unwrap_or("unknown");
        let title = prep.get("title").and_then(|v| v.as_str()).unwrap_or("Meeting");
        meeting_ids.push(meeting_id.to_string());

        prep_context.push_str(&format!("--- Meeting: {} (ID: {}) ---\n", title, meeting_id));

        if let Some(points) = prep.get("talkingPoints").and_then(|v| v.as_array()) {
            prep_context.push_str("Talking Points:\n");
            for p in points {
                if let Some(t) = p.as_str() {
                    prep_context.push_str(&format!("- {}\n", t));
                }
            }
        }
        if let Some(risks) = prep.get("risks").and_then(|v| v.as_array()) {
            prep_context.push_str("Risks:\n");
            for r in risks {
                if let Some(t) = r.as_str() {
                    prep_context.push_str(&format!("- {}\n", t));
                }
            }
        }
        if let Some(items) = prep.get("openItems").and_then(|v| v.as_array()) {
            prep_context.push_str("Open Items:\n");
            for item in items {
                let title = item.get("title").and_then(|v| v.as_str()).unwrap_or("?");
                let overdue = item.get("isOverdue").and_then(|v| v.as_bool()).unwrap_or(false);
                prep_context.push_str(&format!("- {}{}\n", title, if overdue { " [OVERDUE]" } else { "" }));
            }
        }
        if let Some(questions) = prep.get("questions").and_then(|v| v.as_array()) {
            prep_context.push_str("Questions:\n");
            for q in questions {
                if let Some(t) = q.as_str() {
                    prep_context.push_str(&format!("- {}\n", t));
                }
            }
        }
        if let Some(agenda) = prep.get("proposedAgenda").and_then(|v| v.as_array()) {
            prep_context.push_str("Current Mechanical Agenda:\n");
            for (i, item) in agenda.iter().enumerate() {
                let topic = item.get("topic").and_then(|v| v.as_str()).unwrap_or("?");
                prep_context.push_str(&format!("{}. {}\n", i + 1, topic));
            }
        }
        prep_context.push('\n');
    }

    let prompt = format!(
        "You are refining meeting agendas for a Customer Success Manager.\n\n\
         For each meeting below, review the talking points, risks, open items, questions, \
         and current mechanical agenda. Produce a refined agenda that:\n\
         1. Orders items by impact (highest-stakes first)\n\
         2. Adds a brief 'why' rationale for each item\n\
         3. Keeps the source category (risk, talking_point, question, open_item)\n\
         4. Caps at 7 items per meeting\n\n\
         Format your response as:\n\
         AGENDA:meeting-id\n\
         ITEM:topic text\n\
         WHY:rationale\n\
         SOURCE:source_category\n\
         END_ITEM\n\
         ... more items ...\n\
         END_AGENDA\n\n\
         {}",
        prep_context
    );

    let output = pty
        .spawn_claude(workspace, &prompt)
        .map_err(|e| format!("Claude prep enrichment failed: {}", e))?;

    let enrichments = parse_agenda_enrichment(&output.stdout);
    if enrichments.is_empty() {
        log::warn!("enrich_preps: no agenda enrichments parsed from Claude output");
        return Ok(());
    }

    // Merge enriched agendas back into prep files
    let mut enriched_count = 0;
    for path in &prep_files {
        let raw = match fs::read_to_string(path) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let mut prep: Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let meeting_id = prep
            .get("meetingId")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if let Some(items) = enrichments.get(&meeting_id) {
            let agenda_json: Vec<Value> = items
                .iter()
                .map(|item| {
                    let mut obj = json!({"topic": item.topic});
                    if let Some(m) = obj.as_object_mut() {
                        if let Some(ref why) = item.why {
                            m.insert("why".to_string(), json!(why));
                        }
                        if let Some(ref source) = item.source {
                            m.insert("source".to_string(), json!(source));
                        }
                    }
                    obj
                })
                .collect();

            if let Some(obj) = prep.as_object_mut() {
                obj.insert("proposedAgenda".to_string(), json!(agenda_json));
            } else {
                log::warn!("enrich_preps: prep is not a JSON object, skipping agenda insertion");
            }

            if let Err(e) = write_json(path, &prep) {
                log::warn!("enrich_preps: failed to write enriched prep {}: {}", path.display(), e);
            } else {
                enriched_count += 1;
            }
        }
    }

    log::info!(
        "enrich_preps: enriched {}/{} prep files",
        enriched_count,
        prep_files.len()
    );
    Ok(())
}

/// Build and write manifest.json.
///
/// When `partial` is true, the manifest indicates that AI enrichment
/// hasn't completed yet (schedule + actions + preps are ready, but
/// emails and briefing narrative may still be pending).
pub fn deliver_manifest(
    directive: &Directive,
    schedule_data: &Value,
    actions_data: &Value,
    emails_data: &Value,
    prep_paths: &[String],
    data_dir: &Path,
    partial: bool,
) -> Result<Value, String> {
    let now = Utc::now();
    let date = directive
        .context
        .date
        .clone()
        .unwrap_or_else(|| now.format("%Y-%m-%d").to_string());
    let profile = directive.context.profile.as_deref();

    let meetings = schedule_data
        .get("meetings")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let customer_count = schedule_data
        .get("meetings")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter(|m| {
                    let t = m.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    t == "customer" || t == "qbr"
                })
                .count()
        })
        .unwrap_or(0);

    let actions_summary = actions_data.get("summary");
    let actions_due = actions_summary
        .and_then(|s| s.get("dueToday"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let actions_overdue = actions_summary
        .and_then(|s| s.get("overdue"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let email_stats = emails_data.get("stats");
    let emails_high = email_stats
        .and_then(|s| s.get("highCount"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let emails_total = email_stats
        .and_then(|s| s.get("total"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let mut manifest = json!({
        "schemaVersion": "1.0.0",
        "date": date,
        "generatedAt": now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        "partial": partial,
        "files": {
            "schedule": "schedule.json",
            "actions": "actions.json",
            "emails": "emails.json",
            "preps": prep_paths,
        },
        "stats": {
            "totalMeetings": meetings,
            "customerMeetings": customer_count,
            "actionsDue": actions_due,
            "actionsOverdue": actions_overdue,
            "emailsHighPriority": emails_high,
            "emailsTotal": emails_total,
        },
    });

    if let Some(p) = profile {
        manifest
            .as_object_mut()
            .unwrap()
            .insert("profile".to_string(), json!(p));
    }

    write_json(&data_dir.join("manifest.json"), &manifest)?;
    log::info!(
        "deliver_manifest: partial={}, {} meetings, {} actions due, {} emails",
        partial,
        meetings,
        actions_due,
        emails_total,
    );
    Ok(manifest)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json_loader::DirectiveAction;

    #[test]
    fn test_normalise_meeting_type() {
        assert_eq!(normalise_meeting_type("customer"), "customer");
        assert_eq!(normalise_meeting_type("Customer"), "customer");
        assert_eq!(normalise_meeting_type("TEAM_SYNC"), "team_sync");
        assert_eq!(normalise_meeting_type("team-sync"), "team_sync");
        assert_eq!(normalise_meeting_type("unknown"), "internal");
        assert_eq!(normalise_meeting_type("all hands"), "all_hands");
    }

    #[test]
    fn test_format_time_display() {
        assert_eq!(format_time_display("2025-02-07T09:00:00+00:00"), "9:00 AM");
        assert_eq!(
            format_time_display("2025-02-07T14:30:00+00:00"),
            "2:30 PM"
        );
        assert_eq!(format_time_display(""), "All day");
        assert_eq!(format_time_display("2025-02-07"), "All day");
    }

    #[test]
    fn test_make_meeting_id() {
        let id = make_meeting_id("Acme Q1 Sync", "2025-02-07T09:00:00+00:00", "customer");
        assert!(id.starts_with("0900-customer-"));
        assert!(id.contains("acme"));
    }

    #[test]
    fn test_make_action_id_stable() {
        let id1 = make_action_id("Send proposal", "Acme", "2025-02-10");
        let id2 = make_action_id("Send proposal", "Acme", "2025-02-10");
        assert_eq!(id1, id2);
        assert!(id1.starts_with("action-"));
    }

    #[test]
    fn test_make_action_id_different() {
        let id1 = make_action_id("Send proposal", "Acme", "2025-02-10");
        let id2 = make_action_id("Send proposal", "Beta", "2025-02-10");
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_greeting_for_hour() {
        assert_eq!(greeting_for_hour(6), "Good morning");
        assert_eq!(greeting_for_hour(11), "Good morning");
        assert_eq!(greeting_for_hour(12), "Good afternoon");
        assert_eq!(greeting_for_hour(16), "Good afternoon");
        assert_eq!(greeting_for_hour(17), "Good evening");
        assert_eq!(greeting_for_hour(23), "Good evening");
    }

    #[test]
    fn test_deliver_schedule_minimal() {
        let dir = tempfile::tempdir().unwrap();
        let data_dir = dir.path().join("data");

        let directive = Directive {
            context: crate::json_loader::DirectiveContext {
                date: Some("2025-02-07".to_string()),
                ..Default::default()
            },
            calendar: crate::json_loader::DirectiveCalendar {
                events: vec![DirectiveEvent {
                    id: Some("evt-1".to_string()),
                    summary: Some("Team Standup".to_string()),
                    start: Some("2025-02-07T09:00:00+00:00".to_string()),
                    end: Some("2025-02-07T09:30:00+00:00".to_string()),
                }],
            },
            meetings: {
                let mut m = HashMap::new();
                m.insert(
                    "internal".to_string(),
                    vec![DirectiveMeeting {
                        id: Some("evt-1".to_string()),
                        event_id: Some("evt-1".to_string()),
                        summary: Some("Team Standup".to_string()),
                        ..Default::default()
                    }],
                );
                m
            },
            meeting_contexts: vec![],
            actions: Default::default(),
            emails: Default::default(),
        };

        let result = deliver_schedule(&directive, &data_dir).unwrap();
        assert_eq!(result["date"], "2025-02-07");
        assert_eq!(result["meetings"].as_array().unwrap().len(), 1);
        assert!(data_dir.join("schedule.json").exists());
    }

    #[test]
    fn test_deliver_actions_minimal() {
        let dir = tempfile::tempdir().unwrap();
        let data_dir = dir.path().join("data");

        let directive = Directive {
            context: crate::json_loader::DirectiveContext {
                date: Some("2025-02-07".to_string()),
                ..Default::default()
            },
            actions: crate::json_loader::DirectiveActions {
                overdue: vec![DirectiveAction {
                    title: Some("Renew contract".to_string()),
                    account: Some("Acme".to_string()),
                    due_date: Some("2025-02-01".to_string()),
                    days_overdue: Some(6),
                    ..Default::default()
                }],
                due_today: vec![DirectiveAction {
                    title: Some("Send agenda".to_string()),
                    ..Default::default()
                }],
                ..Default::default()
            },
            ..Default::default()
        };

        let result = deliver_actions(&directive, &data_dir, None).unwrap();
        let actions = result["actions"].as_array().unwrap();
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0]["priority"], "P1");
        assert_eq!(actions[0]["isOverdue"], true);
        assert!(data_dir.join("actions.json").exists());
    }

    #[test]
    fn test_deliver_manifest_partial() {
        let dir = tempfile::tempdir().unwrap();
        let data_dir = dir.path().join("data");

        let schedule = json!({"date": "2025-02-07", "meetings": []});
        let actions = json!({"date": "2025-02-07", "summary": {"overdue": 0, "dueToday": 0}, "actions": []});
        let directive = Directive {
            context: crate::json_loader::DirectiveContext {
                date: Some("2025-02-07".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };

        let emails = json!({});
        let result =
            deliver_manifest(&directive, &schedule, &actions, &emails, &[], &data_dir, true)
                .unwrap();
        assert_eq!(result["partial"], true);
        assert!(data_dir.join("manifest.json").exists());
    }

    #[test]
    fn test_deliver_emails_minimal() {
        let dir = tempfile::tempdir().unwrap();
        let data_dir = dir.path().join("data");

        let directive = Directive {
            emails: crate::json_loader::DirectiveEmails {
                high_priority: vec![
                    crate::json_loader::DirectiveEmail {
                        id: Some("e1".to_string()),
                        from: Some("Alice".to_string()),
                        from_email: Some("alice@example.com".to_string()),
                        subject: Some("Contract renewal".to_string()),
                        snippet: Some("Please review the...".to_string()),
                        priority: Some("high".to_string()),
                    },
                ],
                classified: vec![
                    crate::json_loader::DirectiveEmail {
                        id: Some("e2".to_string()),
                        from: Some("Bob".to_string()),
                        from_email: Some("bob@example.com".to_string()),
                        subject: Some("Meeting notes".to_string()),
                        snippet: None,
                        priority: Some("medium".to_string()),
                    },
                ],
                medium_count: 3,
                low_count: 5,
            },
            ..Default::default()
        };

        let result = deliver_emails(&directive, &data_dir).unwrap();
        let hp = result["highPriority"].as_array().unwrap();
        assert_eq!(hp.len(), 1);
        assert_eq!(hp[0]["sender"], "Alice");
        assert_eq!(hp[0]["priority"], "high");
        assert_eq!(result["stats"]["highCount"], 1);
        assert_eq!(result["stats"]["mediumCount"], 3);
        assert_eq!(result["stats"]["lowCount"], 5);
        assert_eq!(result["stats"]["total"], 9);
        assert!(data_dir.join("emails.json").exists());
    }

    #[test]
    fn test_deliver_emails_empty() {
        let dir = tempfile::tempdir().unwrap();
        let data_dir = dir.path().join("data");

        let directive = Directive {
            emails: Default::default(),
            ..Default::default()
        };

        let result = deliver_emails(&directive, &data_dir).unwrap();
        let hp = result["highPriority"].as_array().unwrap();
        assert_eq!(hp.len(), 0);
        assert_eq!(result["stats"]["total"], 0);
    }

    #[test]
    fn test_parse_email_enrichment() {
        let response = "\
ENRICHMENT:e1
SUMMARY: Customer requesting contract extension
ACTION: Reply with proposed terms
ARC: Initial outreach → negotiation → this follow-up
END_ENRICHMENT

ENRICHMENT:e2
SUMMARY: QBR scheduling request
ACTION: Confirm date and send agenda
ARC: First contact about Q2 QBR
END_ENRICHMENT
";
        let enrichments = parse_email_enrichment(response);
        assert_eq!(enrichments.len(), 2);

        let e1 = &enrichments["e1"];
        assert_eq!(
            e1.summary.as_deref(),
            Some("Customer requesting contract extension")
        );
        assert_eq!(e1.action.as_deref(), Some("Reply with proposed terms"));
        assert!(e1.arc.as_deref().unwrap().contains("follow-up"));

        let e2 = &enrichments["e2"];
        assert_eq!(e2.summary.as_deref(), Some("QBR scheduling request"));
    }

    #[test]
    fn test_parse_email_enrichment_partial() {
        let response = "\
ENRICHMENT:e1
SUMMARY: Important update
END_ENRICHMENT
";
        let enrichments = parse_email_enrichment(response);
        assert_eq!(enrichments.len(), 1);

        let e1 = &enrichments["e1"];
        assert_eq!(e1.summary.as_deref(), Some("Important update"));
        assert!(e1.action.is_none());
        assert!(e1.arc.is_none());
    }

    #[test]
    fn test_parse_briefing_narrative() {
        let response = "\
NARRATIVE:
You have a busy day with 3 customer calls. Two overdue actions need attention before your 10 AM call with Acme. One high-priority email from the VP requires a response.
END_NARRATIVE
";
        let narrative = parse_briefing_narrative(response);
        assert!(narrative.is_some());
        let text = narrative.unwrap();
        assert!(text.contains("busy day"));
        assert!(text.contains("overdue actions"));
    }

    #[test]
    fn test_parse_briefing_narrative_missing() {
        let response = "Here's some random output without markers.";
        let narrative = parse_briefing_narrative(response);
        assert!(narrative.is_none());
    }

    #[test]
    fn test_deliver_manifest_with_emails() {
        let dir = tempfile::tempdir().unwrap();
        let data_dir = dir.path().join("data");

        let schedule = json!({"date": "2025-02-07", "meetings": []});
        let actions = json!({"summary": {"overdue": 1, "dueToday": 2}, "actions": []});
        let emails = json!({
            "highPriority": [],
            "stats": {"highCount": 3, "mediumCount": 5, "lowCount": 10, "total": 18}
        });
        let directive = Directive {
            context: crate::json_loader::DirectiveContext {
                date: Some("2025-02-07".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };

        let result = deliver_manifest(
            &directive, &schedule, &actions, &emails, &[], &data_dir, false,
        )
        .unwrap();
        assert_eq!(result["partial"], false);
        assert_eq!(result["files"]["emails"], "emails.json");
        assert_eq!(result["stats"]["emailsHighPriority"], 3);
        assert_eq!(result["stats"]["emailsTotal"], 18);
        assert_eq!(result["stats"]["actionsOverdue"], 1);
    }

    #[test]
    fn test_parse_agenda_enrichment() {
        let response = "\
AGENDA:mtg-acme-weekly
ITEM:Address Team B usage decline
WHY:25% drop threatens renewal — needs intervention plan
SOURCE:risk
END_ITEM
ITEM:Celebrate Phase 1 completion
WHY:Position as proof of execution before Phase 2 ask
SOURCE:talking_point
END_ITEM
END_AGENDA

AGENDA:mtg-globex-qbr
ITEM:Renewal proposal
WHY:90 days to renewal — need commitment
SOURCE:talking_point
END_ITEM
END_AGENDA
";
        let enrichments = parse_agenda_enrichment(response);
        assert_eq!(enrichments.len(), 2);

        let acme = &enrichments["mtg-acme-weekly"];
        assert_eq!(acme.len(), 2);
        assert_eq!(acme[0].topic, "Address Team B usage decline");
        assert_eq!(acme[0].why.as_deref(), Some("25% drop threatens renewal — needs intervention plan"));
        assert_eq!(acme[0].source.as_deref(), Some("risk"));
        assert_eq!(acme[1].topic, "Celebrate Phase 1 completion");

        let globex = &enrichments["mtg-globex-qbr"];
        assert_eq!(globex.len(), 1);
        assert_eq!(globex[0].topic, "Renewal proposal");
    }

    #[test]
    fn test_parse_agenda_enrichment_empty() {
        let response = "Here's some random output without markers.";
        let enrichments = parse_agenda_enrichment(response);
        assert!(enrichments.is_empty());
    }

    #[test]
    fn test_generate_mechanical_agenda_basic() {
        let prep = json!({
            "openItems": [
                {"title": "Send SOW", "isOverdue": true},
                {"title": "Update docs", "isOverdue": false},
            ],
            "risks": ["Budget risk", "Timeline risk", "Staffing risk"],
            "talkingPoints": ["Win 1", "Win 2", "Win 3", "Win 4"],
            "questions": ["Q1?", "Q2?", "Q3?"],
        });
        let agenda = generate_mechanical_agenda(&prep);

        // Should have items: 1 overdue + 2 risks + 3 talking points + 1 non-overdue = 7
        assert_eq!(agenda.len(), 7);

        // First item should be the overdue follow-up
        assert!(agenda[0]["topic"].as_str().unwrap().starts_with("Follow up:"));
        assert_eq!(agenda[0]["source"], "open_item");

        // Next 2 should be risks
        assert_eq!(agenda[1]["source"], "risk");
        assert_eq!(agenda[2]["source"], "risk");

        // Next 3 should be talking points
        assert_eq!(agenda[3]["source"], "talking_point");
    }

    #[test]
    fn test_generate_mechanical_agenda_empty() {
        let prep = json!({});
        let agenda = generate_mechanical_agenda(&prep);
        assert!(agenda.is_empty());
    }

    #[test]
    fn test_generate_mechanical_agenda_caps_at_seven() {
        let prep = json!({
            "openItems": [
                {"title": "A", "isOverdue": true},
                {"title": "B", "isOverdue": true},
                {"title": "C", "isOverdue": true},
                {"title": "D", "isOverdue": true},
                {"title": "E", "isOverdue": true},
                {"title": "F", "isOverdue": true},
                {"title": "G", "isOverdue": true},
                {"title": "H", "isOverdue": true},
            ],
            "risks": ["Risk 1"],
            "talkingPoints": ["Point 1"],
        });
        let agenda = generate_mechanical_agenda(&prep);
        assert_eq!(agenda.len(), 7);
    }

    #[test]
    fn test_classify_meeting_density() {
        assert_eq!(classify_meeting_density(0), "light");
        assert_eq!(classify_meeting_density(1), "light");
        assert_eq!(classify_meeting_density(2), "light");
        assert_eq!(classify_meeting_density(3), "moderate");
        assert_eq!(classify_meeting_density(5), "moderate");
        assert_eq!(classify_meeting_density(6), "busy");
        assert_eq!(classify_meeting_density(7), "busy");
        assert_eq!(classify_meeting_density(8), "busy");
        assert_eq!(classify_meeting_density(9), "packed");
        assert_eq!(classify_meeting_density(10), "packed");
        assert_eq!(classify_meeting_density(15), "packed");
    }
}
