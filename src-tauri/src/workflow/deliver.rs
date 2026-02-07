//! Rust-native delivery functions (ADR-0042: per-operation pipelines)
//!
//! Ports the mechanical delivery logic from deliver_today.py into Rust.
//! These functions take the Phase 1 directive and write JSON output files
//! that the frontend consumes — no AI enrichment needed.
//!
//! Functions:
//! - `deliver_schedule()` → schedule.json
//! - `deliver_actions()` → actions.json
//! - `deliver_preps()` → preps/*.json
//! - `deliver_manifest()` → manifest.json

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use chrono::{DateTime, Timelike, Utc};
use regex::Regex;
use serde_json::{json, Value};

use crate::json_loader::{
    Directive, DirectiveEvent, DirectiveMeeting, DirectiveMeetingContext,
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
fn write_json(path: &Path, data: &Value) -> Result<(), String> {
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

        let obj = meeting_obj.as_object_mut().unwrap();
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
            let map = obj.as_object_mut().unwrap();
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
                    let obj = entry.as_object_mut().unwrap();
                    if let Some(role) = a.get("role").and_then(|v| v.as_str()) {
                        obj.insert("role".to_string(), json!(role));
                    }
                    if let Some(focus) = a.get("focus").and_then(|v| v.as_str()) {
                        obj.insert("focus".to_string(), json!(focus));
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
    let obj = prep.as_object_mut().unwrap();

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

    prep
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

    let mut manifest = json!({
        "schemaVersion": "1.0.0",
        "date": date,
        "generatedAt": now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        "partial": partial,
        "files": {
            "schedule": "schedule.json",
            "actions": "actions.json",
            "preps": prep_paths,
        },
        "stats": {
            "totalMeetings": meetings,
            "customerMeetings": customer_count,
            "actionsDue": actions_due,
            "actionsOverdue": actions_overdue,
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
        "deliver_manifest: partial={}, {} meetings, {} actions due",
        partial,
        meetings,
        actions_due,
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

        let result =
            deliver_manifest(&directive, &schedule, &actions, &[], &data_dir, true).unwrap();
        assert_eq!(result["partial"], true);
        assert!(data_dir.join("manifest.json").exists());
    }
}
