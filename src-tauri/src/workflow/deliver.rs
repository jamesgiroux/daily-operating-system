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
//! - `enrich_week()` → updates week-overview.json with narrative, priority, suggestions (I94/I95)

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

use chrono::{DateTime, Timelike, Utc};
use chrono_tz::Tz;
use regex::Regex;
use serde_json::{json, Value};

use crate::json_loader::{
    Directive, DirectiveEmail, DirectiveEvent, DirectiveMeeting, DirectiveMeetingContext,
};
use crate::types::{EmailSyncStage, EmailSyncState, EmailSyncStatus};
use crate::util::wrap_user_data;

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

/// Meeting types that receive prep files (account-based)
const PREP_ELIGIBLE_TYPES: &[&str] = &["customer", "qbr", "partnership"];

/// Meeting types eligible for person-focused prep (I159).
/// These get lightweight prep built from attendee data in the people DB.
const PERSON_PREP_TYPES: &[&str] = &["internal", "team_sync", "one_on_one"];
const EMAILS_FILE: &str = "emails.json";

// ============================================================================
// Shared helpers (ported from deliver_today.py)
// ============================================================================

/// Normalise a meeting type string to a valid enum value.
/// Defaults to "internal" for unrecognised values.
pub fn normalise_meeting_type(raw: &str) -> &'static str {
    let normalised = raw.to_lowercase().replace([' ', '-'], "_");
    for &valid in VALID_MEETING_TYPES {
        if normalised == valid {
            return valid;
        }
    }
    "internal"
}

/// Convert an ISO datetime string to human-readable time like "9:00 AM" in local tz.
///
/// Uses the system-local timezone by default. Callers that have a config timezone
/// should use `format_time_display_tz()` instead.
pub fn format_time_display(iso: &str) -> String {
    // Fallback: use system local offset for display
    format_time_display_tz(iso, None)
}

/// Convert an ISO datetime string to human-readable time in the given timezone.
///
/// If `tz` is None, falls back to the system local offset embedded in the timestamp,
/// or UTC if the timestamp has no offset.
pub fn format_time_display_tz(iso: &str, tz: Option<Tz>) -> String {
    if iso.is_empty() || !iso.contains('T') {
        return "All day".to_string();
    }
    match DateTime::parse_from_rfc3339(&iso.replace('Z', "+00:00"))
        .or_else(|_| DateTime::parse_from_rfc3339(iso))
    {
        Ok(dt) => {
            let local = match tz {
                Some(tz) => dt.with_timezone(&tz).format("%-I:%M %p").to_string(),
                None => {
                    // Use system local time
                    let local_dt = dt.with_timezone(&chrono::Local);
                    local_dt.format("%-I:%M %p").to_string()
                }
            };
            local
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

/// Derive the primary meeting ID from a calendar event (ADR-0061).
///
/// Prefers the Google Calendar event ID (stable across runs) with `@` sanitized
/// for filesystem safety. Falls back to `make_meeting_id()` for non-calendar
/// meetings or when the event ID is absent.
pub fn meeting_primary_id(
    calendar_event_id: Option<&str>,
    summary: &str,
    start: &str,
    meeting_type: &str,
) -> String {
    if let Some(eid) = calendar_event_id {
        if !eid.is_empty() {
            return eid.replace('@', "_at_");
        }
    }
    make_meeting_id(summary, start, meeting_type)
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
    for meeting_list in meetings_by_type.values() {
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
    // Prefer event ID first to avoid cross-meeting context bleed when account
    // hints are wrong or ambiguous.
    if let Some(eid) = event_id {
        for ctx in contexts {
            if ctx.event_id.as_deref() == Some(eid) {
                return Some(ctx);
            }
        }
    }

    if let Some(acct) = account {
        for ctx in contexts {
            if ctx.account.as_deref() == Some(acct) {
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
///
/// Pulls from both the lightweight directive fields (talking_points, risks, wins)
/// AND the entity intelligence fields (executive_assessment, entity_risks,
/// stakeholder_insights, account_data.recent_wins) so that meetings with
/// enriched intelligence show rich content on the briefing cards.
fn build_prep_summary(ctx: &DirectiveMeetingContext) -> Option<Value> {
    let account_data = ctx.account_data.as_ref().and_then(|v| v.as_object());

    let mut at_a_glance: Vec<String> = Vec::new();
    if let Some(data) = account_data {
        for (key, label) in &[
            ("lifecycle", "Lifecycle"),
            ("arr", "ARR"),
            ("renewal", "Renewal"),
            ("health", "Health"),
        ] {
            if let Some(val) = data.get(*key).and_then(|v| v.as_str()) {
                let clean = sanitize_inline_markdown(val);
                if !clean.is_empty() {
                    at_a_glance.push(format!("{}: {}", label, clean));
                }
            }
        }
    }

    // Discuss: talking_points first, fall back to account_data.recent_wins
    let discuss: Vec<String> = ctx
        .talking_points
        .as_ref()
        .filter(|v| !v.is_empty())
        .map(|v| v.iter().take(4).map(|s| sanitize_inline_markdown(s)).collect())
        .or_else(|| {
            account_data
                .and_then(|d| d.get("recent_wins"))
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .take(4)
                        .filter_map(|v| v.as_str().map(|s| sanitize_inline_markdown(s)))
                        .collect()
                })
        })
        .unwrap_or_default();

    // Watch: risks first, fall back to entity_risks[].text
    let watch: Vec<String> = ctx
        .risks
        .as_ref()
        .filter(|v| !v.is_empty())
        .map(|v| v.iter().take(3).map(|s| s.clone()).collect())
        .or_else(|| {
            ctx.entity_risks.as_ref().map(|arr| {
                arr.iter()
                    .take(3)
                    .filter_map(|v| {
                        v.get("text").and_then(|t| t.as_str()).map(|s| s.to_string())
                    })
                    .collect()
            })
        })
        .unwrap_or_default();

    // Wins: ctx.wins first, fall back to account_data.recent_wins (if not already used for discuss)
    let wins: Vec<String> = ctx
        .wins
        .as_ref()
        .filter(|v| !v.is_empty())
        .map(|v| v.iter().take(3).map(|s| sanitize_inline_markdown(s)).collect())
        .unwrap_or_default();

    // Context: executive_assessment (truncated for card display)
    let context = ctx
        .executive_assessment
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(|s| truncate_to_sentences(s, 2));

    // Stakeholders from entity intelligence (exclude internal people)
    let stakeholders: Vec<Value> = ctx
        .stakeholder_insights
        .as_ref()
        .map(|arr| {
            arr.iter()
                .filter(|v| {
                    v.get("relationship")
                        .and_then(|r| r.as_str())
                        .map_or(true, |r| r != "internal")
                })
                .take(6)
                .filter_map(|v| {
                    let name = v.get("name").and_then(|n| n.as_str())?;
                    Some(json!({
                        "name": name,
                        "role": v.get("role").and_then(|r| r.as_str()),
                        "focus": v.get("focus").and_then(|f| f.as_str()),
                    }))
                })
                .collect()
        })
        .unwrap_or_default();

    if at_a_glance.is_empty()
        && discuss.is_empty()
        && watch.is_empty()
        && wins.is_empty()
        && context.is_none()
        && stakeholders.is_empty()
    {
        return None;
    }

    let mut summary = json!({
        "atAGlance": &at_a_glance[..at_a_glance.len().min(4)],
        "discuss": discuss,
        "watch": watch,
        "wins": wins,
    });

    if let Some(obj) = summary.as_object_mut() {
        if let Some(ctx_text) = context {
            obj.insert("context".to_string(), json!(ctx_text));
        }
        if !stakeholders.is_empty() {
            obj.insert("stakeholders".to_string(), json!(stakeholders));
        }
    }

    Some(summary)
}

/// Truncate text to the first N sentences (period-delimited).
fn truncate_to_sentences(text: &str, n: usize) -> String {
    let mut count = 0;
    let mut end = 0;
    let bytes = text.as_bytes();
    let len = bytes.len();
    while end < len && count < n {
        if bytes[end] == b'.' {
            // Check it's a sentence-ending period (followed by space, newline, or end)
            let next = end + 1;
            if next >= len || bytes[next] == b' ' || bytes[next] == b'\n' {
                count += 1;
                if count >= n {
                    end += 1; // include the period
                    break;
                }
            }
        }
        end += 1;
    }
    if end >= len {
        text.to_string()
    } else {
        text[..end].trim().to_string()
    }
}

/// Fallback: build a prep summary by reading the per-meeting prep file directly.
/// Used when directive meetingContexts is empty but prep files exist.
fn build_prep_summary_from_file(data_dir: &Path, meeting_id: &str) -> Option<Value> {
    let prep_path = data_dir.join("preps").join(format!("{}.json", meeting_id));
    let content = fs::read_to_string(&prep_path).ok()?;
    let prep: Value = serde_json::from_str(&content).ok()?;

    // Map accountSnapshot → atAGlance (structured label/value pairs)
    let mut at_a_glance: Vec<String> = Vec::new();
    if let Some(snapshots) = prep.get("accountSnapshot").and_then(|v| v.as_array()) {
        for snap in snapshots.iter().take(4) {
            let label = snap.get("label").and_then(|v| v.as_str()).unwrap_or("");
            let value = snap.get("value").and_then(|v| v.as_str()).unwrap_or("");
            if !label.is_empty() && !value.is_empty() {
                at_a_glance.push(format!("{}: {}", label, value));
            }
        }
    }
    // Fallback: quickContext is a flat map written by enrich_prep_from_db
    if at_a_glance.is_empty() {
        if let Some(qc) = prep.get("quickContext").and_then(|v| v.as_object()) {
            for (key, val) in qc.iter().take(4) {
                if let Some(v) = val.as_str() {
                    at_a_glance.push(format!("{}: {}", key, v));
                }
            }
        }
    }

    // Map talkingPoints → discuss
    let discuss: Vec<&str> = prep
        .get("talkingPoints")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().take(4).filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();

    // Map entityRisks → watch (extract .text from objects, or use as string)
    // Fallback to plain "risks" array if entityRisks is absent
    let watch: Vec<String> = prep
        .get("entityRisks")
        .or_else(|| prep.get("risks"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .take(3)
                .filter_map(|v| {
                    v.get("text")
                        .and_then(|t| t.as_str())
                        .or_else(|| v.as_str())
                        .map(|s| s.to_string())
                })
                .collect()
        })
        .unwrap_or_default();

    // Map recentWins → wins
    let wins: Vec<&str> = prep
        .get("recentWins")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().take(3).filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();

    // Map intelligenceSummary → context (truncate to first 2 sentences for card display)
    // Fallback chain: intelligenceSummary > meetingContext > currentState
    let context = prep
        .get("intelligenceSummary")
        .or_else(|| prep.get("meetingContext"))
        .or_else(|| prep.get("currentState"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| truncate_to_sentences(s, 2));

    // Map stakeholderInsights → stakeholders (exclude internal people)
    let stakeholders: Vec<Value> = prep
        .get("stakeholderInsights")
        .or_else(|| prep.get("attendees"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter(|v| {
                    v.get("relationship")
                        .and_then(|r| r.as_str())
                        .map_or(true, |r| r != "internal")
                })
                .take(6)
                .filter_map(|v| {
                    let name = v.get("name").and_then(|n| n.as_str())?;
                    Some(json!({
                        "name": name,
                        "role": v.get("role").and_then(|r| r.as_str()),
                        "focus": v.get("focus").and_then(|f| f.as_str()),
                    }))
                })
                .collect()
        })
        .unwrap_or_default();

    // Map openItems
    let open_items: Vec<String> = prep
        .get("openItems")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .take(4)
                .filter_map(|v| {
                    v.get("title")
                        .and_then(|t| t.as_str())
                        .or_else(|| v.as_str())
                        .map(|s| s.to_string())
                })
                .collect()
        })
        .unwrap_or_default();

    if at_a_glance.is_empty()
        && discuss.is_empty()
        && watch.is_empty()
        && wins.is_empty()
        && context.is_none()
        && stakeholders.is_empty()
        && open_items.is_empty()
    {
        return None;
    }

    let mut summary = json!({
        "atAGlance": &at_a_glance[..at_a_glance.len().min(4)],
        "discuss": discuss,
        "watch": watch,
        "wins": wins,
    });

    if let Some(obj) = summary.as_object_mut() {
        if let Some(ctx) = context {
            obj.insert("context".to_string(), json!(ctx));
        }
        if !stakeholders.is_empty() {
            obj.insert("stakeholders".to_string(), json!(stakeholders));
        }
        if !open_items.is_empty() {
            obj.insert("openItems".to_string(), json!(open_items));
        }
    }

    Some(summary)
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

/// Write JSON to a file with pretty printing (I64: atomic write).
pub fn write_json(path: &Path, data: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory {}: {}", parent.display(), e))?;
    }
    let content = serde_json::to_string_pretty(data)
        .map_err(|e| format!("Failed to serialize JSON: {}", e))?;
    crate::util::atomic_write_str(path, &format!("{}\n", content))
        .map_err(|e| format!("Failed to write {}: {}", path.display(), e))
}

// ============================================================================
// Core delivery functions
// ============================================================================

/// Build and write schedule.json from directive data.
///
/// Returns the schedule JSON value (needed by manifest builder).
pub fn deliver_schedule(
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

    // Resolve user timezone for display (default: system local via None)
    let tz: Option<Tz> = crate::state::load_config()
        .ok()
        .and_then(|c| c.schedules.today.timezone.parse::<Tz>().ok());

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
        let meeting_id = meeting_primary_id(event.id.as_deref(), summary, start, meeting_type);

        let mc = find_meeting_context(account.as_deref(), Some(event_id), meeting_contexts);
        let resolved_account = mc.and_then(|c| c.account.clone()).or(account.clone());
        let prep_summary = mc
            .and_then(build_prep_summary)
            .or_else(|| build_prep_summary_from_file(data_dir, &meeting_id));
        let has_prep_file = data_dir
            .join("preps")
            .join(format!("{}.json", meeting_id))
            .exists();
        let has_account_prep = PREP_ELIGIBLE_TYPES.contains(&meeting_type) && (mc.is_some() || has_prep_file);
        let has_person_prep = PERSON_PREP_TYPES.contains(&meeting_type);
        let has_prep = has_account_prep || has_person_prep;
        let prep_file = if has_prep {
            Some(format!("preps/{}.json", meeting_id))
        } else {
            None
        };

        let mut meeting_obj = json!({
            "id": meeting_id,
            "calendarEventId": event.id,
            "time": format_time_display_tz(start, tz),
            "startIso": start,
            "title": summary,
            "type": meeting_type,
            "hasPrep": has_prep,
            "isCurrent": is_meeting_current(event, now),
        });

        if let Some(obj) = meeting_obj.as_object_mut() {
            if !end.is_empty() {
                obj.insert(
                    "endTime".to_string(),
                    json!(format_time_display_tz(end, tz)),
                );
            }
            if let Some(ref acct) = resolved_account {
                obj.insert("account".to_string(), json!(acct));
                // Resolve account name → slugified entity ID for intelligence lookup
                if let Some(db) = db {
                    if let Ok(Some(account_row)) = db.get_account_by_name(acct) {
                        obj.insert("accountId".to_string(), json!(account_row.id));
                    }
                }
            }
            if let Some(ref pf) = prep_file {
                obj.insert("prepFile".to_string(), json!(pf));
            }
            if let Some(ref ps) = prep_summary {
                obj.insert("prepSummary".to_string(), ps.clone());
            }

            // Embed linked entities from junction table (I52)
            if let Some(db) = db {
                if let Ok(entities) = db.get_meeting_entities(&meeting_id) {
                    if !entities.is_empty() {
                        let entity_arr: Vec<Value> = entities
                            .iter()
                            .map(|e| {
                                json!({
                                    "id": e.id,
                                    "name": e.name,
                                    "entityType": e.entity_type.as_str(),
                                })
                            })
                            .collect();
                        obj.insert("linkedEntities".to_string(), json!(entity_arr));
                    }
                }
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
    log::info!("deliver_schedule: {} meetings written", meetings_json.len());
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
            db.get_all_action_titles()
                .ok()
                .map(|titles| titles.into_iter().collect())
        })
        .unwrap_or_default();

    let mut actions_list: Vec<Value> = Vec::new();
    let mut seen_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

    let mut add_action = |title: &str,
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
        if existing_titles.contains(title.to_lowercase().trim()) {
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
            title,
            account,
            "P1",
            "pending",
            due,
            false,
            None,
            &task.context,
            &task.source,
        );
    }

    // Due this week → P2
    for task in &raw.due_this_week {
        let title = task.title.as_deref().unwrap_or("Unknown");
        let account = task.account.as_deref().unwrap_or("");
        let due = task.effective_due_date().unwrap_or("");
        add_action(
            title,
            account,
            "P2",
            "pending",
            due,
            false,
            None,
            &task.context,
            &task.source,
        );
    }

    // Waiting on → P2
    for item in &raw.waiting_on {
        let what = item.what.as_deref().unwrap_or("Unknown");
        let title = format!("Waiting: {}", what);
        let who = item.who.as_deref().unwrap_or("");
        add_action(
            &title,
            who,
            "P2",
            "waiting",
            "",
            false,
            None,
            &item.context,
            &None,
        );
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
    fs::create_dir_all(&preps_dir).map_err(|e| format!("Failed to create preps dir: {}", e))?;

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

            // Only write a prep file if there is meaningful context,
            // OR if this is a person-prep-eligible type (I159)
            let is_person_prep = PERSON_PREP_TYPES.contains(&normalised_type);
            if mc.is_none() && account.is_none() && !is_person_prep {
                continue;
            }

            // ADR-0061: Use calendar event ID as primary key, fall back to slug
            let title = meeting
                .title
                .as_deref()
                .or(meeting.summary.as_deref())
                .or(account)
                .unwrap_or("meeting");
            let start = meeting
                .start
                .as_deref()
                .or(meeting.start_display.as_deref())
                .unwrap_or("");
            let meeting_id = meeting_primary_id(event_id, title, start, normalised_type);

            let mut prep_data = build_prep_json(meeting, normalised_type, &meeting_id, mc);
            let rel_path = format!("preps/{}.json", meeting_id);

            // ADR-0065: Preserve user-authored fields from existing prep file.
            // user_agenda and user_notes are owned by the user, never overwritten by system.
            let prep_path = data_dir.join(&rel_path);
            if prep_path.exists() {
                if let Ok(existing) = fs::read_to_string(&prep_path) {
                    if let Ok(existing_json) = serde_json::from_str::<Value>(&existing) {
                        if let Some(obj) = prep_data.as_object_mut() {
                            if let Some(ua) = existing_json.get("userAgenda") {
                                if ua.is_array() && ua.as_array().is_some_and(|a| !a.is_empty()) {
                                    obj.insert("userAgenda".to_string(), ua.clone());
                                }
                            }
                            if let Some(un) = existing_json.get("userNotes") {
                                if un.is_string() && un.as_str().is_some_and(|s| !s.is_empty()) {
                                    obj.insert("userNotes".to_string(), un.clone());
                                }
                            }
                        }
                    }
                }
            }

            write_json(&prep_path, &prep_data)?;
            prep_paths.push(rel_path);
        }
    }

    // I66: Clean stale prep files AFTER new writes succeed.
    // Only remove .json files not in the new set.
    let new_filenames: std::collections::HashSet<String> = prep_paths
        .iter()
        .filter_map(|p| p.strip_prefix("preps/").map(String::from))
        .collect();
    if let Ok(entries) = fs::read_dir(&preps_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            if let Some(name_str) = name.to_str() {
                if name_str.ends_with(".json") && !new_filenames.contains(name_str) {
                    let _ = fs::remove_file(entry.path());
                }
            }
        }
    }

    log::info!("deliver_preps: {} prep files written", prep_paths.len());
    Ok(prep_paths)
}

/// Check whether a prep JSON file has substantive content beyond stub fields (I166).
///
/// A prep is "substantive" if it has any content beyond the mechanical fields
/// (meetingId, title, meetingType, timeRange). Returns false for stubs that
/// were written when meeting context gathering found no data.
pub fn is_substantive_prep(prep_path: &Path) -> bool {
    let Ok(content) = fs::read_to_string(prep_path) else {
        return false;
    };
    let Ok(json) = serde_json::from_str::<Value>(&content) else {
        return false;
    };
    // Content fields that indicate real prep data exists
    let content_keys = [
        "context",
        "quickContext",
        "attendees",
        "sinceLast",
        "strategicPrograms",
        "currentState",
        "openItems",
        "risks",
        "talkingPoints",
        "recentWins",
        "recentWinSources",
        "questions",
        "keyPrinciples",
        "proposedAgenda",
        "attendeeContext",
        "stakeholderSignals",
        "calendarNotes",
        "personPrep",
    ];
    content_keys.iter().any(|key| {
        json.get(key).is_some_and(|v| {
            !v.is_null()
                && v.as_str().is_none_or(|s| !s.is_empty())
                && v.as_array().is_none_or(|a| !a.is_empty())
                && v.as_object().is_none_or(|o| !o.is_empty())
        })
    })
}

/// Reconcile `hasPrep` flags in schedule.json to reflect actual prep content (I166).
///
/// Called after `deliver_preps` and `enrich_preps` to ensure "View Prep" buttons
/// only appear when the prep file has substantive content.
pub fn reconcile_prep_flags(data_dir: &Path) -> Result<(), String> {
    let schedule_path = data_dir.join("schedule.json");
    if !schedule_path.exists() {
        return Ok(());
    }

    let content =
        fs::read_to_string(&schedule_path).map_err(|e| format!("Read schedule.json: {}", e))?;
    let mut schedule: Value =
        serde_json::from_str(&content).map_err(|e| format!("Parse schedule.json: {}", e))?;

    let Some(meetings) = schedule.get_mut("meetings").and_then(|v| v.as_array_mut()) else {
        return Ok(());
    };

    let mut updated = false;
    for meeting in meetings.iter_mut() {
        if let Some(prep_file) = meeting.get("prepFile").and_then(|v| v.as_str()) {
            let prep_path = data_dir.join(prep_file);
            let has_substance = is_substantive_prep(&prep_path);
            let current = meeting
                .get("hasPrep")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if current != has_substance {
                meeting
                    .as_object_mut()
                    .unwrap()
                    .insert("hasPrep".to_string(), json!(has_substance));
                updated = true;
            }
        }
    }

    if updated {
        write_json(&schedule_path, &schedule)?;
        log::info!("reconcile_prep_flags: updated hasPrep flags in schedule.json");
    }

    Ok(())
}

/// Build a compact account snapshot for meeting prep (I186).
///
/// Keep this intentionally short and operational so the prep page can render
/// board-style metadata without long narrative fields.
fn build_account_snapshot(ctx: Option<&DirectiveMeetingContext>) -> Vec<Value> {
    let mut items: Vec<Value> = Vec::new();
    let ctx = match ctx {
        Some(c) => c,
        None => return items,
    };

    // Mechanical fields from account_data (dashboard.json)
    if let Some(data) = ctx.account_data.as_ref().and_then(|v| v.as_object()) {
        let fields: &[(&str, &str, &str)] = &[
            ("lifecycle", "Lifecycle", "text"),
            ("health", "Health", "status"),
            ("arr", "ARR", "currency"),
            ("renewal", "Renewal", "date"),
        ];
        for (key, label, typ) in fields {
            if let Some(val) = data.get(*key).and_then(|v| v.as_str()) {
                let clean = sanitize_inline_markdown(val);
                if !clean.is_empty() {
                    items.push(json!({"label": label, "value": clean, "type": typ}));
                }
            }
        }
    }

    // Cap small to avoid visual overload in prep report.
    items.truncate(4);
    items
}

fn sanitize_inline_markdown(value: &str) -> String {
    value
        .replace("[", "")
        .replace("]", "")
        .replace("(", "")
        .replace(")", "")
        .replace(">", "")
        .replace("**", "")
        .replace("__", "")
        .replace(['`', '*'], "")
        .replace('_', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn source_tail_regex() -> &'static Regex {
    static SOURCE_TAIL_RE: OnceLock<Regex> = OnceLock::new();
    SOURCE_TAIL_RE.get_or_init(|| {
        Regex::new(r"(?i)(?:^|\s)[_*]*\(?\s*source:\s*([^)]+?)\s*\)?[_*\s]*$")
            .expect("source tail regex must compile")
    })
}

fn recent_win_prefix_regex() -> &'static Regex {
    static RECENT_WIN_PREFIX_RE: OnceLock<Regex> = OnceLock::new();
    RECENT_WIN_PREFIX_RE.get_or_init(|| {
        Regex::new(r"(?i)^(recent\s+win|win)\s*:\s*").expect("recent win prefix regex must compile")
    })
}

fn agenda_list_item_regex() -> &'static Regex {
    static AGENDA_LIST_ITEM_RE: OnceLock<Regex> = OnceLock::new();
    AGENDA_LIST_ITEM_RE.get_or_init(|| {
        Regex::new(r"^(?:[-*•]\s+|\d+[.)]\s+)(.+)$").expect("agenda list item regex must compile")
    })
}

fn inline_numbered_agenda_regex() -> &'static Regex {
    static INLINE_NUMBERED_AGENDA_RE: OnceLock<Regex> = OnceLock::new();
    INLINE_NUMBERED_AGENDA_RE.get_or_init(|| {
        Regex::new(r"(?:^|\s)\d+[.)]\s+").expect("inline numbered agenda regex must compile")
    })
}

fn split_inline_source_tail(value: &str) -> (String, Option<String>) {
    let raw = value.trim();
    if let Some(caps) = source_tail_regex().captures(raw) {
        if let Some(full_match) = caps.get(0) {
            let cleaned = raw[..full_match.start()].trim().to_string();
            let source = caps
                .get(1)
                .map(|m| sanitize_inline_markdown(m.as_str()))
                .and_then(|s| if s.is_empty() { None } else { Some(s) });
            return (cleaned, source);
        }
    }
    (raw.to_string(), None)
}

fn sanitize_prep_line(value: &str) -> Option<String> {
    let (without_source, _) = split_inline_source_tail(value);
    let cleaned = sanitize_inline_markdown(&without_source);
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

fn sanitize_recent_win_line(value: &str) -> Option<String> {
    let (without_source, _) = split_inline_source_tail(value);
    let stripped = recent_win_prefix_regex()
        .replace(&without_source, "")
        .trim()
        .to_string();
    let cleaned = sanitize_inline_markdown(&stripped);
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

fn source_reference_value(source: &str) -> Option<Value> {
    let cleaned = sanitize_inline_markdown(source);
    if cleaned.is_empty() {
        return None;
    }
    let label = cleaned
        .split(['/', '\\'])
        .rfind(|part| !part.trim().is_empty())
        .unwrap_or(cleaned.as_str())
        .to_string();
    Some(json!({
        "label": label,
        "path": cleaned,
    }))
}

fn append_recent_win(
    raw_win: &str,
    source_hint: Option<&str>,
    wins: &mut Vec<String>,
    win_keys: &mut HashSet<String>,
    sources: &mut Vec<Value>,
    source_keys: &mut HashSet<String>,
) {
    let (without_source, embedded_source) = split_inline_source_tail(raw_win);
    let Some(cleaned_win) = sanitize_recent_win_line(&without_source) else {
        return;
    };
    let win_key = cleaned_win.to_lowercase();
    if !win_keys.contains(&win_key) {
        win_keys.insert(win_key);
        wins.push(cleaned_win);
    }

    let source_text = source_hint.map(str::to_string).or(embedded_source);
    if let Some(source) = source_text {
        let source_key = source.to_lowercase();
        if !source_keys.contains(&source_key) {
            if let Some(src_value) = source_reference_value(&source) {
                source_keys.insert(source_key);
                sources.push(src_value);
            }
        }
    }
}

fn derive_recent_wins_and_sources(ctx: &DirectiveMeetingContext) -> (Vec<String>, Vec<Value>) {
    let mut wins: Vec<String> = Vec::new();
    let mut sources: Vec<Value> = Vec::new();
    let mut win_keys: HashSet<String> = HashSet::new();
    let mut source_keys: HashSet<String> = HashSet::new();

    if let Some(points) = &ctx.talking_points {
        for point in points {
            append_recent_win(
                point,
                None,
                &mut wins,
                &mut win_keys,
                &mut sources,
                &mut source_keys,
            );
        }
    }

    if let Some(account_wins) = ctx
        .account_data
        .as_ref()
        .and_then(|d| d.get("recent_wins"))
        .and_then(|v| v.as_array())
    {
        for win in account_wins.iter().take(6) {
            if let Some(text) = win.as_str() {
                append_recent_win(
                    text,
                    None,
                    &mut wins,
                    &mut win_keys,
                    &mut sources,
                    &mut source_keys,
                );
                continue;
            }
            if let Some(obj) = win.as_object() {
                let text = obj
                    .get("text")
                    .or_else(|| obj.get("win"))
                    .or_else(|| obj.get("summary"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let source = obj
                    .get("source")
                    .or_else(|| obj.get("path"))
                    .or_else(|| obj.get("label"))
                    .and_then(|v| v.as_str());
                append_recent_win(
                    text,
                    source,
                    &mut wins,
                    &mut win_keys,
                    &mut sources,
                    &mut source_keys,
                );
            }
        }
    }

    if let Some(capture_wins) = &ctx.wins {
        for win in capture_wins.iter().take(4) {
            append_recent_win(
                win,
                None,
                &mut wins,
                &mut win_keys,
                &mut sources,
                &mut source_keys,
            );
        }
    }

    (wins, sources)
}

/// Build a single prep JSON object (matches JsonPrep in json_loader.rs).
fn build_prep_json(
    meeting: &DirectiveMeeting,
    meeting_type: &str,
    meeting_id: &str,
    ctx: Option<&DirectiveMeetingContext>,
) -> Value {
    let account = ctx
        .and_then(|c| c.account.as_deref())
        .or(meeting.account.as_deref());

    // Account snapshot: intelligence-enriched Quick Context (I186)
    let account_snapshot = build_account_snapshot(ctx);

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
        // I159: Mark person-prep-eligible meetings so is_substantive_prep recognises them
        if PERSON_PREP_TYPES.contains(&meeting_type) {
            obj.insert("personPrep".to_string(), json!(true));
        }
        if !account_snapshot.is_empty() {
            obj.insert("accountSnapshot".to_string(), json!(account_snapshot));
        }
        if !attendees.is_empty() {
            obj.insert("attendees".to_string(), json!(attendees));
        }

        if let Some(ctx) = ctx {
            if let Some(ref desc) = ctx.description {
                if !desc.is_empty() {
                    obj.insert("calendarNotes".to_string(), json!(desc));
                }
            }
            if let Some(ref narrative) = ctx.narrative {
                obj.insert("meetingContext".to_string(), json!(narrative));
            }
            if let Some(ref v) = ctx.since_last {
                obj.insert("sinceLast".to_string(), json!(v));
            }
            if let Some(ref v) = ctx.current_state {
                obj.insert("currentState".to_string(), json!(v));
            }
            if let Some(ref v) = ctx.key_principles {
                obj.insert("keyPrinciples".to_string(), json!(v));
            }

            // --- Synthesize prep content from raw data when structured fields are empty ---

            // Risks: use ctx.risks if present, else fall back to account_data.current_risks
            let risks: Vec<Value> = ctx
                .risks
                .as_ref()
                .map(|v| v.iter().map(|s| json!(s)).collect())
                .unwrap_or_else(|| {
                    ctx.account_data
                        .as_ref()
                        .and_then(|d| d.get("current_risks"))
                        .and_then(|v| v.as_array())
                        .map(|arr| arr.iter().map(|s| json!(s)).collect())
                        .unwrap_or_default()
                });
            if !risks.is_empty() {
                obj.insert("risks".to_string(), json!(risks));
            }

            // Recent wins are canonical. Keep talkingPoints as legacy compatibility.
            let (recent_wins, recent_win_sources) = derive_recent_wins_and_sources(ctx);
            if !recent_wins.is_empty() {
                obj.insert("recentWins".to_string(), json!(recent_wins));
            }
            if !recent_win_sources.is_empty() {
                obj.insert("recentWinSources".to_string(), json!(recent_win_sources));
            }

            let mut talking_points: Vec<String> = ctx
                .talking_points
                .as_ref()
                .map(|items| {
                    let mut out: Vec<String> = Vec::new();
                    let mut seen: HashSet<String> = HashSet::new();
                    for item in items {
                        if let Some(cleaned) = sanitize_prep_line(item) {
                            let key = cleaned.to_lowercase();
                            if !seen.contains(&key) {
                                seen.insert(key);
                                out.push(cleaned);
                            }
                        }
                    }
                    out
                })
                .unwrap_or_default();
            if talking_points.is_empty() {
                if let Some(wins) = obj.get("recentWins").and_then(|v| v.as_array()).map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(ToString::to_string))
                        .collect::<Vec<_>>()
                }) {
                    talking_points = wins;
                }
            }
            if !talking_points.is_empty() {
                obj.insert("talkingPoints".to_string(), json!(talking_points));
            }

            // Questions: use ctx.questions if present
            if let Some(ref v) = ctx.questions {
                obj.insert("questions".to_string(), json!(v));
            }

            // Open items: use ctx.open_items if present, else synthesize from
            // open_actions (raw JSON array from SQLite meeting context)
            let open_items: Vec<Value> = ctx
                .open_items
                .as_ref()
                .map(|items| {
                    items
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
                        .collect()
                })
                .unwrap_or_else(|| {
                    // Fall back to raw open_actions from the directive meeting context
                    synthesize_open_items_from_raw(ctx)
                });
            if !open_items.is_empty() {
                obj.insert("openItems".to_string(), json!(open_items));
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

            // I135: Entity intelligence fields (from intelligence.json via meeting context)
            if let Some(ref assessment) = ctx.executive_assessment {
                obj.insert("intelligenceSummary".to_string(), json!(assessment));
            }
            if let Some(ref risks) = ctx.entity_risks {
                if !risks.is_empty() {
                    obj.insert("entityRisks".to_string(), json!(risks));
                }
            }
            if let Some(ref readiness) = ctx.entity_readiness {
                if !readiness.is_empty() {
                    obj.insert("entityReadiness".to_string(), json!(readiness));
                }
            }
            if let Some(ref insights) = ctx.stakeholder_insights {
                if !insights.is_empty() {
                    obj.insert("stakeholderInsights".to_string(), json!(insights));
                }
            }
            if let Some(ref signals) = ctx.recent_email_signals {
                if !signals.is_empty() {
                    obj.insert("recentEmailSignals".to_string(), json!(signals));
                }
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

/// Synthesize open items from raw meeting context data.
///
/// Converts `open_actions` (from SQLite via meeting_context.rs) into
/// the `openItems` format that `generate_mechanical_agenda()` expects.
fn synthesize_open_items_from_raw(ctx: &DirectiveMeetingContext) -> Vec<Value> {
    let mut items = Vec::new();

    if let Some(ref actions) = ctx.open_actions {
        let today_str = Utc::now().date_naive().to_string();
        for action in actions.iter().take(10) {
            let title = action.get("title").and_then(|v| v.as_str()).unwrap_or("");
            if title.is_empty() {
                continue;
            }
            let due = action
                .get("due_date")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let is_overdue = !due.is_empty() && due < today_str.as_str();
            items.push(json!({
                "title": title,
                "dueDate": if due.is_empty() { None } else { Some(due) },
                "isOverdue": is_overdue,
            }));
        }
    }

    items
}

/// Generate a mechanical agenda from existing prep data.
///
/// Synthesizes an agenda from open items (overdue first), risks,
/// talking points, and questions. Caps at 7 items. No AI needed.
fn push_unique_agenda_item(
    agenda: &mut Vec<Value>,
    seen_topics: &mut HashSet<String>,
    topic: &str,
    why: Option<&str>,
    source: &str,
    max_items: usize,
) {
    if agenda.len() >= max_items {
        return;
    }
    let Some(clean_topic) = sanitize_prep_line(topic) else {
        return;
    };
    let topic_key = clean_topic.to_lowercase();
    if seen_topics.contains(&topic_key) {
        return;
    }
    seen_topics.insert(topic_key);

    let mut item = json!({
        "topic": clean_topic,
        "source": source,
    });
    if let Some(reason) = why.and_then(sanitize_prep_line) {
        if let Some(obj) = item.as_object_mut() {
            obj.insert("why".to_string(), json!(reason));
        }
    }
    agenda.push(item);
}

fn split_inline_agenda_candidates(value: &str) -> Vec<String> {
    // Use null byte as sentinel — pipes appear in real agenda text (e.g. "Review pipeline | Discuss metrics").
    let numbered = inline_numbered_agenda_regex().replace_all(value.trim(), "\x00");
    let mut out = Vec::new();
    for segment in numbered.split('\x00') {
        for item in segment.split(';') {
            let trimmed = item.trim();
            if !trimmed.is_empty() {
                out.push(trimmed.to_string());
            }
        }
    }
    if out.is_empty() {
        out.push(value.trim().to_string());
    }
    out
}

/// Pre-sanitizes calendar agenda candidates for dedup, then collects unique items.
/// Items are sanitized here for consistent dedup keys; `push_unique_agenda_item`
/// re-sanitizes when consuming them (idempotent, harmless).
fn push_calendar_agenda_candidate(raw: &str, agenda: &mut Vec<String>, seen: &mut HashSet<String>) {
    for candidate in split_inline_agenda_candidates(raw) {
        let Some(cleaned) = sanitize_prep_line(&candidate) else {
            continue;
        };
        let key = cleaned.to_lowercase();
        if !seen.contains(&key) {
            seen.insert(key);
            agenda.push(cleaned);
        }
    }
}

fn extract_calendar_agenda_items(prep: &Value) -> Vec<String> {
    let Some(calendar_notes) = prep.get("calendarNotes").and_then(|v| v.as_str()) else {
        return Vec::new();
    };

    let mut agenda = Vec::new();
    let mut seen = HashSet::new();
    let mut in_agenda_section = false;

    for line in calendar_notes.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let lower = trimmed.to_lowercase();
        let agenda_prefix_len = if lower.starts_with("proposed agenda") {
            "proposed agenda".len()
        } else if lower.starts_with("agenda") {
            "agenda".len()
        } else {
            0
        };
        if agenda_prefix_len > 0 {
            let remainder = trimmed[agenda_prefix_len..].trim_start();
            // Require delimiter or end-of-line after keyword — reject prose like
            // "Agenda items were discussed" which is not a section header.
            if remainder.is_empty() || remainder.starts_with(':') || remainder.starts_with('-') {
                in_agenda_section = true;
                if let Some((_, rest)) = trimmed.split_once(':') {
                    push_calendar_agenda_candidate(rest, &mut agenda, &mut seen);
                }
                continue;
            }
        }

        if in_agenda_section
            && trimmed.ends_with(':')
            && !trimmed.starts_with('-')
            && !trimmed.starts_with('*')
            && !trimmed.starts_with('•')
            && !trimmed
                .chars()
                .next()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
        {
            break;
        }

        if in_agenda_section {
            if let Some(caps) = agenda_list_item_regex().captures(trimmed) {
                if let Some(item) = caps.get(1) {
                    push_calendar_agenda_candidate(item.as_str(), &mut agenda, &mut seen);
                }
            } else {
                push_calendar_agenda_candidate(trimmed, &mut agenda, &mut seen);
            }
        }
    }

    // No fallback pass needed — the first pass already matches `lower.starts_with("agenda")`
    // which covers "agenda:", "agenda -", and any other "agenda" prefix.

    agenda
}

fn generate_mechanical_agenda(prep: &Value) -> Vec<Value> {
    let mut agenda: Vec<Value> = Vec::new();
    let mut seen_topics: HashSet<String> = HashSet::new();
    const MAX_ITEMS: usize = 7;

    // 1. Calendar agenda first (I188) — enrich around user/organizer agenda.
    for item in extract_calendar_agenda_items(prep).iter().take(MAX_ITEMS) {
        push_unique_agenda_item(
            &mut agenda,
            &mut seen_topics,
            item,
            Some("From calendar agenda"),
            "calendar_note",
            MAX_ITEMS,
        );
    }

    // 2. Overdue items next (most urgent operational follow-up)
    if let Some(items) = prep.get("openItems").and_then(|v| v.as_array()) {
        for item in items {
            let is_overdue = item
                .get("isOverdue")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if is_overdue {
                let title = item
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown item");
                push_unique_agenda_item(
                    &mut agenda,
                    &mut seen_topics,
                    &format!("Follow up: {}", title),
                    Some("Overdue — needs resolution"),
                    "open_item",
                    MAX_ITEMS,
                );
            }
        }
    }

    // 3. Risks (limit 2)
    if let Some(risks) = prep.get("risks").and_then(|v| v.as_array()) {
        for risk in risks.iter().take(2) {
            if let Some(text) = risk.as_str() {
                push_unique_agenda_item(
                    &mut agenda,
                    &mut seen_topics,
                    text,
                    None,
                    "risk",
                    MAX_ITEMS,
                );
            }
        }
    }

    // 4. Questions (limit 2)
    if let Some(questions) = prep.get("questions").and_then(|v| v.as_array()) {
        for q in questions.iter().take(2) {
            if let Some(text) = q.as_str() {
                push_unique_agenda_item(
                    &mut agenda,
                    &mut seen_topics,
                    text,
                    None,
                    "question",
                    MAX_ITEMS,
                );
            }
        }
    }

    // 5. Non-overdue open items (limit 2)
    if let Some(items) = prep.get("openItems").and_then(|v| v.as_array()) {
        for item in items.iter().take(4) {
            let is_overdue = item
                .get("isOverdue")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if !is_overdue {
                let title = item
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown item");
                push_unique_agenda_item(
                    &mut agenda,
                    &mut seen_topics,
                    title,
                    None,
                    "open_item",
                    MAX_ITEMS,
                );
            }
        }
    }

    // 6. Wins are fallback only when agenda is still sparse.
    if agenda.len() < 3 {
        if let Some(wins) = prep
            .get("recentWins")
            .and_then(|v| v.as_array())
            .or_else(|| prep.get("talkingPoints").and_then(|v| v.as_array()))
        {
            for win in wins.iter().take(3) {
                if let Some(text) = win.as_str() {
                    let Some(clean_win) = sanitize_recent_win_line(text) else {
                        continue;
                    };
                    push_unique_agenda_item(
                        &mut agenda,
                        &mut seen_topics,
                        &clean_win,
                        None,
                        "talking_point",
                        MAX_ITEMS,
                    );
                }
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
fn parse_email_sync_stage(raw: Option<&str>, default: EmailSyncStage) -> EmailSyncStage {
    match raw.unwrap_or("").to_lowercase().as_str() {
        "fetch" => EmailSyncStage::Fetch,
        "deliver" => EmailSyncStage::Deliver,
        "enrich" => EmailSyncStage::Enrich,
        "refresh" => EmailSyncStage::Refresh,
        _ => default,
    }
}

fn sync_to_value(sync: &EmailSyncStatus) -> Result<Value, String> {
    serde_json::to_value(sync).map_err(|e| format!("Failed to serialize email sync status: {}", e))
}

fn existing_last_success_at(payload: &Value) -> Option<String> {
    payload
        .get("sync")
        .and_then(|v| v.get("lastSuccessAt"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn ensure_email_payload_shape(payload: &mut Value) {
    if !payload.is_object() {
        *payload = json!({});
    }
    if let Some(obj) = payload.as_object_mut() {
        if !obj.contains_key("highPriority") {
            obj.insert("highPriority".to_string(), json!([]));
        }
        if !obj.contains_key("mediumPriority") {
            obj.insert("mediumPriority".to_string(), json!([]));
        }
        if !obj.contains_key("lowPriority") {
            obj.insert("lowPriority".to_string(), json!([]));
        }
    }
    recalculate_email_stats(payload);
}

fn recalculate_email_stats(payload: &mut Value) {
    let high_count = payload
        .get("highPriority")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let medium_count = payload
        .get("mediumPriority")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let low_count = payload
        .get("lowPriority")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let total = high_count + medium_count + low_count;

    if let Some(obj) = payload.as_object_mut() {
        obj.insert(
            "stats".to_string(),
            json!({
                "highCount": high_count,
                "mediumCount": medium_count,
                "lowCount": low_count,
                "total": total,
            }),
        );
    }
}

/// Read the full emails payload from disk if available.
pub fn read_emails_payload(data_dir: &Path) -> Option<Value> {
    let path = data_dir.join(EMAILS_FILE);
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

/// Build an empty email payload with optional sync metadata.
pub fn empty_emails_payload(sync: Option<&EmailSyncStatus>) -> Value {
    let mut payload = json!({
        "highPriority": [],
        "mediumPriority": [],
        "lowPriority": [],
        "stats": {
            "highCount": 0,
            "mediumCount": 0,
            "lowCount": 0,
            "total": 0,
        }
    });
    if let Some(sync) = sync {
        if let Ok(sync_value) = sync_to_value(sync) {
            if let Some(obj) = payload.as_object_mut() {
                obj.insert("sync".to_string(), sync_value);
            }
        }
    }
    payload
}

/// Parse structured sync metadata from an emails payload.
pub fn extract_email_sync_status(payload: &Value) -> Option<EmailSyncStatus> {
    payload
        .get("sync")
        .and_then(|v| serde_json::from_value::<EmailSyncStatus>(v.clone()).ok())
}

/// Update only sync metadata in emails.json while preserving existing lists/stats.
pub fn set_email_sync_status(data_dir: &Path, sync: &EmailSyncStatus) -> Result<Value, String> {
    let mut payload = read_emails_payload(data_dir).unwrap_or_else(|| empty_emails_payload(None));
    ensure_email_payload_shape(&mut payload);

    let mut sync_to_write = sync.clone();
    if sync_to_write.last_success_at.is_none() {
        sync_to_write.last_success_at = existing_last_success_at(&payload);
    }
    if sync_to_write.last_attempt_at.is_none() {
        sync_to_write.last_attempt_at = Some(Utc::now().to_rfc3339());
    }

    if let Some(obj) = payload.as_object_mut() {
        obj.insert("sync".to_string(), sync_to_value(&sync_to_write)?);
    }
    write_json(&data_dir.join(EMAILS_FILE), &payload)?;
    Ok(payload)
}

/// Returns the emails JSON value (needed by manifest builder).
pub fn deliver_emails(directive: &Directive, data_dir: &Path) -> Result<Value, String> {
    let emails = &directive.emails;
    let now = Utc::now().to_rfc3339();

    if let Some(sync_error) = emails.sync_error.as_ref() {
        let stage = parse_email_sync_stage(sync_error.stage.as_deref(), EmailSyncStage::Fetch);
        let existing = read_emails_payload(data_dir);
        let using_last_known_good = existing.is_some();
        let last_success_at = existing.as_ref().and_then(existing_last_success_at);
        let sync = EmailSyncStatus {
            state: EmailSyncState::Error,
            stage,
            code: Some(
                sync_error
                    .code
                    .clone()
                    .unwrap_or_else(|| "email_sync_failed".to_string()),
            ),
            message: sync_error
                .message
                .clone()
                .or_else(|| Some("Email sync failed".to_string())),
            using_last_known_good: Some(using_last_known_good),
            can_retry: Some(true),
            last_attempt_at: Some(now),
            last_success_at,
        };

        if let Some(mut payload) = existing {
            ensure_email_payload_shape(&mut payload);
            if let Some(obj) = payload.as_object_mut() {
                obj.insert("sync".to_string(), sync_to_value(&sync)?);
            }
            write_json(&data_dir.join(EMAILS_FILE), &payload)?;
            return Ok(payload);
        }

        let payload = empty_emails_payload(Some(&sync));
        write_json(&data_dir.join(EMAILS_FILE), &payload)?;
        return Ok(payload);
    }

    // Build email objects from both sources, deduplicating by ID
    let mut seen_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut high_priority: Vec<Value> = Vec::new();
    let mut medium_priority: Vec<Value> = Vec::new();
    let mut low_priority: Vec<Value> = Vec::new();

    let add_email = |email: &DirectiveEmail,
                     priority: &str,
                     seen: &mut std::collections::HashSet<String>,
                     high: &mut Vec<Value>,
                     medium: &mut Vec<Value>,
                     low: &mut Vec<Value>| {
        let id = email
            .id
            .clone()
            .unwrap_or_else(|| format!("email-{}", seen.len()));
        if seen.contains(&id) {
            return;
        }
        seen.insert(id.clone());

        let obj = json!({
            "id": id,
            "sender": email.from.as_deref().unwrap_or("Unknown"),
            "senderEmail": email.from_email.as_deref().unwrap_or(""),
            "subject": email.subject.as_deref().unwrap_or("(no subject)"),
            "snippet": email.snippet,
            "priority": priority,
        });

        match priority {
            "high" => high.push(obj),
            "medium" => medium.push(obj),
            _ => low.push(obj),
        }
    };

    // High-priority emails from dedicated list
    for email in &emails.high_priority {
        add_email(
            email,
            "high",
            &mut seen_ids,
            &mut high_priority,
            &mut medium_priority,
            &mut low_priority,
        );
    }

    // All classified emails (high deduped, medium + low added)
    for email in &emails.classified {
        let prio = email.priority.as_deref().unwrap_or("medium");
        add_email(
            email,
            prio,
            &mut seen_ids,
            &mut high_priority,
            &mut medium_priority,
            &mut low_priority,
        );
    }

    let high_count = high_priority.len();
    let medium_count = medium_priority.len();
    let low_count = low_priority.len();
    let total = high_count + medium_count + low_count;

    let sync = EmailSyncStatus {
        state: EmailSyncState::Ok,
        stage: EmailSyncStage::Deliver,
        code: None,
        message: None,
        using_last_known_good: Some(false),
        can_retry: Some(true),
        last_attempt_at: Some(now.clone()),
        last_success_at: Some(now),
    };

    let emails_data = json!({
        "highPriority": high_priority,
        "mediumPriority": medium_priority,
        "lowPriority": low_priority,
        "stats": {
            "highCount": high_count,
            "mediumCount": medium_count,
            "lowCount": low_count,
            "total": total,
        },
        "sync": sync_to_value(&sync)?,
    });

    write_json(&data_dir.join(EMAILS_FILE), &emails_data)?;
    log::info!(
        "deliver_emails: {} high, {} medium, {} low ({} total)",
        high_count,
        medium_count,
        low_count,
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
    pub signals: Vec<crate::types::EmailSignal>,
}

/// Parse Claude's email enrichment response.
///
/// Expected format per email:
/// ```text
/// ENRICHMENT:email-id
/// SUMMARY: one-line summary
/// ACTION: recommended next action
/// ARC: conversation context
/// SIGNALS: [{"signalType":"timeline","signalText":"...", "confidence":0.8}]
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
            } else if let Some(val) = trimmed.strip_prefix("SIGNALS:") {
                current.signals =
                    serde_json::from_str::<Vec<crate::types::EmailSignal>>(val.trim())
                        .unwrap_or_default();
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
        let subject = email.get("subject").and_then(|v| v.as_str()).unwrap_or("?");
        let snippet = email.get("snippet").and_then(|v| v.as_str()).unwrap_or("");
        email_context.push_str(&format!(
            "ID: {}\nFrom: {}\nSubject: {}\nSnippet: {}\n\n",
            id,
            wrap_user_data(sender),
            wrap_user_data(subject),
            wrap_user_data(snippet),
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
         a recommended action, brief conversation arc context, and a JSON array \
         of structured signals. Signal types must be one of: expansion, question, timeline, \
         sentiment, feedback, relationship. Keep signalText concise.\n\n\
         Format your response as:\n\
         ENRICHMENT:email-id-here\n\
         SUMMARY: <one-line summary>\n\
         ACTION: <recommended next action>\n\
         ARC: <conversation context>\n\
         SIGNALS: <JSON array of objects with signalType, signalText, optional confidence/sentiment/urgency>\n\
         END_ENRICHMENT\n\n\
         {}",
        user_fragment, email_context
    );

    let output = pty
        .spawn_claude(workspace, &prompt)
        .map_err(|e| format!("Claude enrichment failed: {}", e))?;

    // Audit trail (I297)
    let date_id = data_dir.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");
    let _ = crate::audit::write_audit_entry(workspace, "email_batch", date_id, &output.stdout);

    let enrichments = parse_email_enrichment(&output.stdout);
    if enrichments.is_empty() {
        log::warn!("enrich_emails: no enrichments parsed from Claude output");
        // Clean up context file
        let _ = fs::remove_file(&context_path);
        return Err("No enrichments parsed from Claude output".to_string());
    }

    // Merge enrichments into emails.json
    if let Some(hp) = emails_data
        .get_mut("highPriority")
        .and_then(|v| v.as_array_mut())
    {
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
                    if !enrichment.signals.is_empty() {
                        obj.insert("emailSignals".to_string(), json!(enrichment.signals));
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

/// Parse the AI-generated daily focus statement from Claude's response.
///
/// Expects: `FOCUS:\n<single sentence>\nEND_FOCUS`
pub fn parse_briefing_focus(response: &str) -> Option<String> {
    let mut in_block = false;
    let mut lines: Vec<&str> = Vec::new();

    for line in response.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("FOCUS:") {
            in_block = true;
            let after = trimmed.strip_prefix("FOCUS:").unwrap().trim();
            if !after.is_empty() {
                lines.push(after);
            }
        } else if trimmed == "END_FOCUS" {
            break;
        } else if in_block {
            lines.push(trimmed);
        }
    }

    if lines.is_empty() {
        return None;
    }

    let focus = lines.join(" ").trim().to_string();
    if focus.is_empty() {
        None
    } else {
        Some(focus)
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

/// I137/I52: Build entity intelligence context for meetings in a schedule.
///
/// Extracts unique entity IDs from schedule meetings (via linkedEntities + accountId
/// fallback), looks up cached intelligence from the DB, returns a formatted context
/// block for the prompt. Handles both accounts and projects.
fn build_entity_intel_for_briefing(schedule: &Value, db: &crate::db::ActionDb) -> String {
    let meetings = match schedule.get("meetings").and_then(|v| v.as_array()) {
        Some(arr) => arr,
        None => return String::new(),
    };

    let mut seen = std::collections::HashSet::new();
    let mut parts = Vec::new();

    for meeting in meetings {
        // Collect entity IDs: prefer linkedEntities array, fallback to accountId
        let mut entity_ids: Vec<(String, String)> = Vec::new(); // (id, display_name)

        if let Some(linked) = meeting.get("linkedEntities").and_then(|v| v.as_array()) {
            for le in linked {
                let id = le.get("id").and_then(|v| v.as_str()).unwrap_or("");
                let name = le.get("name").and_then(|v| v.as_str()).unwrap_or(id);
                if !id.is_empty() {
                    entity_ids.push((id.to_string(), name.to_string()));
                }
            }
        }

        // Fallback: accountId (for meetings without linkedEntities)
        if entity_ids.is_empty() {
            if let Some(aid) = meeting.get("accountId").and_then(|v| v.as_str()) {
                if !aid.is_empty() {
                    let name = meeting
                        .get("account")
                        .and_then(|v| v.as_str())
                        .unwrap_or(aid);
                    entity_ids.push((aid.to_string(), name.to_string()));
                }
            }
        }

        for (entity_id, entity_name) in entity_ids {
            if !seen.insert(entity_id.clone()) {
                continue;
            }

            let intel = match db.get_entity_intelligence(&entity_id) {
                Ok(Some(intel)) => intel,
                _ => continue,
            };

            let mut block = format!("### {}\n", entity_name);

            if let Some(ref assessment) = intel.executive_assessment {
                let truncated = if assessment.len() > 300 {
                    format!("{}...", &assessment[..300])
                } else {
                    assessment.clone()
                };
                block.push_str(&truncated);
                block.push('\n');
            }

            if !intel.risks.is_empty() {
                block.push_str("Risks: ");
                let risk_texts: Vec<&str> = intel
                    .risks
                    .iter()
                    .take(3)
                    .map(|r| r.text.as_str())
                    .collect();
                block.push_str(&risk_texts.join("; "));
                block.push('\n');
            }

            if let Some(ref readiness) = intel.next_meeting_readiness {
                if !readiness.prep_items.is_empty() {
                    block.push_str("Readiness: ");
                    block.push_str(&readiness.prep_items.join("; "));
                    block.push('\n');
                }
            }

            parts.push(block);
        }
    }

    parts.join("\n")
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
    state: &crate::state::AppState,
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

    // I137: Gather entity intelligence for accounts with meetings today (brief DB lock)
    let intel_context = {
        let db_guard = state.db.lock().ok();
        match db_guard.as_ref().and_then(|g| g.as_ref()) {
            Some(db) => build_entity_intel_for_briefing(&schedule, db),
            None => String::new(),
        }
    }; // DB lock released here, before PTY call

    let user_fragment = user_ctx.prompt_fragment();
    let role_label = user_ctx.title_or_default();
    let mut prompt = format!(
        "You are writing a morning briefing narrative for {role_label}.\n\
         {user_fragment}\n\
         Today's context:\n\
         - Date: {}\n\
         - Meetings: {} ({} customer) — density: {}\n\
         - First meeting: {}\n\
         - Key meetings: {}\n\
         - Actions: {} overdue, {} due today\n\
         - Emails: {} high-priority\n\n\
         {}\n",
        date,
        meetings,
        customer_count,
        density,
        first_meeting_time,
        wrap_user_data(&top_meetings.join(", ")),
        overdue_count,
        due_today,
        high_count,
        density_guidance,
    );

    if !intel_context.is_empty() {
        prompt.push_str(&format!(
            "\n## Entity Intelligence (for today's accounts)\n\
             Use this context to make the narrative account-aware. \
             Reference specific risks, readiness, or stakeholder dynamics when relevant.\n\n\
             {}\n",
            wrap_user_data(&intel_context)
        ));
    }

    // I147: Include overnight maintenance summary if available
    let maintenance_path = data_dir.join("maintenance.json");
    if let Ok(maintenance_raw) = fs::read_to_string(&maintenance_path) {
        if let Ok(maintenance) =
            serde_json::from_str::<crate::hygiene::OvernightReport>(&maintenance_raw)
        {
            let total = maintenance.entities_refreshed
                + maintenance.names_resolved
                + maintenance.meetings_linked
                + maintenance.summaries_extracted;
            if total > 0 {
                prompt.push_str(&format!(
                    "\n## Overnight Maintenance\n\
                     DailyOS refreshed intelligence for {} accounts, resolved names for {} people, \
                     linked {} meetings, and extracted {} file summaries overnight.\n\
                     Briefly mention this at the end of the narrative (one sentence).\n",
                    maintenance.entities_refreshed,
                    maintenance.names_resolved,
                    maintenance.meetings_linked,
                    maintenance.summaries_extracted,
                ));
            }
        }
    }

    prompt.push_str(
        "\nProvide two sections in your response:\n\n\
         1. A 2-3 sentence narrative that helps them understand the shape of their day.\n\
         Focus on what matters most — customer calls, overdue items, important emails.\n\
         Be direct, not chatty.\n\n\
         2. A single-sentence focus statement — the one thing that matters most today.\n\
         This should be actionable and specific, not generic. Think: what would you tell them \
         if they only had 30 seconds? Reference the specific meeting, account, or deadline.\n\n\
         Format your response EXACTLY as:\n\n\
         NARRATIVE:\n\
         <your 2-3 sentence narrative>\n\
         END_NARRATIVE\n\n\
         FOCUS:\n\
         <your single-sentence focus>\n\
         END_FOCUS",
    );

    let output = pty
        .spawn_claude(workspace, &prompt)
        .map_err(|e| format!("Claude briefing failed: {}", e))?;

    // Audit trail (I297)
    let date_id = data_dir.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");
    let _ = crate::audit::write_audit_entry(workspace, "daily_briefing", date_id, &output.stdout);

    let response = &output.stdout;
    let narrative = parse_briefing_narrative(response);
    let focus = parse_briefing_focus(response);
    let _ = fs::remove_file(&context_path);

    let mut updated = false;

    if let Some(ref text) = narrative {
        schedule
            .as_object_mut()
            .unwrap()
            .insert("narrative".to_string(), json!(text));
        log::info!("enrich_briefing: narrative written ({} chars)", text.len());
        updated = true;
    } else {
        log::warn!("enrich_briefing: no narrative parsed from Claude output");
    }

    if let Some(ref text) = focus {
        schedule
            .as_object_mut()
            .unwrap()
            .insert("focus".to_string(), json!(text));
        log::info!("enrich_briefing: focus written ({} chars)", text.len());
        updated = true;
    } else {
        log::warn!("enrich_briefing: no focus parsed from Claude output");
    }

    if updated {
        write_json(&data_dir.join("schedule.json"), &schedule)?;
    }

    Ok(())
}

/// Parsed enrichment for a single agenda item.
#[derive(Debug, Clone, Default)]
pub struct AgendaItemEnrichment {
    pub topic: String,
    pub why: Option<String>,
    pub source: Option<String>,
}

/// Parsed enrichment for a single recent win.
#[derive(Debug, Clone, Default)]
pub struct RecentWinEnrichment {
    pub win: String,
    pub source: Option<String>,
}

/// Combined prep enrichment payload by meeting.
#[derive(Debug, Clone, Default)]
pub struct PrepEnrichment {
    pub agenda: Vec<AgendaItemEnrichment>,
    pub wins: Vec<RecentWinEnrichment>,
}

/// Parse Claude's prep enrichment response (agenda + wins).
///
/// Expected format:
/// ```text
/// AGENDA:meeting-id
/// ITEM:topic text here
/// WHY:rationale for discussing this
/// SOURCE:risk
/// END_ITEM
/// END_AGENDA
/// WINS:meeting-id
/// WIN:Executive sponsor re-engaged
/// SOURCE:2026-02-11-call-notes.md
/// END_WIN
/// END_WINS
/// ```
pub fn parse_prep_enrichment(response: &str) -> HashMap<String, PrepEnrichment> {
    enum Section {
        Agenda(String),
        Wins(String),
    }

    let mut result: HashMap<String, PrepEnrichment> = HashMap::new();
    let mut current_section: Option<Section> = None;
    let mut current_agenda_item: Option<AgendaItemEnrichment> = None;
    let mut current_win_item: Option<RecentWinEnrichment> = None;

    let commit_agenda_item =
        |meeting_id: &str,
         item: AgendaItemEnrichment,
         map: &mut HashMap<String, PrepEnrichment>| {
            let entry = map.entry(meeting_id.to_string()).or_default();
            if !item.topic.trim().is_empty() {
                entry.agenda.push(item);
            }
        };

    let commit_win_item =
        |meeting_id: &str, item: RecentWinEnrichment, map: &mut HashMap<String, PrepEnrichment>| {
            let entry = map.entry(meeting_id.to_string()).or_default();
            if !item.win.trim().is_empty() {
                entry.wins.push(item);
            }
        };

    for line in response.lines() {
        let trimmed = line.trim();

        if let Some(id) = trimmed.strip_prefix("AGENDA:") {
            if let Some(Section::Agenda(ref meeting_id)) = current_section {
                if let Some(item) = current_agenda_item.take() {
                    commit_agenda_item(meeting_id, item, &mut result);
                }
            }
            current_section = Some(Section::Agenda(id.trim().to_string()));
            continue;
        }
        if let Some(id) = trimmed.strip_prefix("WINS:") {
            if let Some(Section::Wins(ref meeting_id)) = current_section {
                if let Some(item) = current_win_item.take() {
                    commit_win_item(meeting_id, item, &mut result);
                }
            }
            current_section = Some(Section::Wins(id.trim().to_string()));
            continue;
        }

        match current_section {
            Some(Section::Agenda(ref meeting_id)) => {
                if trimmed == "END_AGENDA" {
                    if let Some(item) = current_agenda_item.take() {
                        commit_agenda_item(meeting_id, item, &mut result);
                    }
                    current_section = None;
                } else if let Some(topic) = trimmed.strip_prefix("ITEM:") {
                    if let Some(item) = current_agenda_item.take() {
                        commit_agenda_item(meeting_id, item, &mut result);
                    }
                    current_agenda_item = Some(AgendaItemEnrichment {
                        topic: topic.trim().to_string(),
                        ..Default::default()
                    });
                } else if trimmed == "END_ITEM" {
                    if let Some(item) = current_agenda_item.take() {
                        commit_agenda_item(meeting_id, item, &mut result);
                    }
                } else if let Some(ref mut item) = current_agenda_item {
                    if let Some(val) = trimmed.strip_prefix("WHY:") {
                        item.why = Some(val.trim().to_string());
                    } else if let Some(val) = trimmed.strip_prefix("SOURCE:") {
                        item.source = Some(val.trim().to_string());
                    }
                }
            }
            Some(Section::Wins(ref meeting_id)) => {
                if trimmed == "END_WINS" {
                    if let Some(item) = current_win_item.take() {
                        commit_win_item(meeting_id, item, &mut result);
                    }
                    current_section = None;
                } else if let Some(text) = trimmed.strip_prefix("WIN:") {
                    if let Some(item) = current_win_item.take() {
                        commit_win_item(meeting_id, item, &mut result);
                    }
                    current_win_item = Some(RecentWinEnrichment {
                        win: text.trim().to_string(),
                        ..Default::default()
                    });
                } else if trimmed == "END_WIN" || trimmed == "END_ITEM" {
                    if let Some(item) = current_win_item.take() {
                        commit_win_item(meeting_id, item, &mut result);
                    }
                } else if let Some(ref mut item) = current_win_item {
                    if let Some(val) = trimmed.strip_prefix("SOURCE:") {
                        item.source = Some(val.trim().to_string());
                    }
                }
            }
            None => {}
        }
    }

    // Commit trailing partial sections defensively.
    if let Some(section) = current_section {
        match section {
            Section::Agenda(meeting_id) => {
                if let Some(item) = current_agenda_item {
                    commit_agenda_item(&meeting_id, item, &mut result);
                }
            }
            Section::Wins(meeting_id) => {
                if let Some(item) = current_win_item {
                    commit_win_item(&meeting_id, item, &mut result);
                }
            }
        }
    }

    result
}

/// Backward-compatible helper for agenda-only callers.
pub fn parse_agenda_enrichment(response: &str) -> HashMap<String, Vec<AgendaItemEnrichment>> {
    parse_prep_enrichment(response)
        .into_iter()
        .filter_map(|(meeting_id, enrichment)| {
            if enrichment.agenda.is_empty() {
                None
            } else {
                Some((meeting_id, enrichment.agenda))
            }
        })
        .collect()
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

        let meeting_id = prep
            .get("meetingId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let title = prep
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Meeting");
        meeting_ids.push(meeting_id.to_string());

        prep_context.push_str(&format!(
            "--- Meeting: {} (ID: {}) ---\n",
            title, meeting_id
        ));

        if let Some(points) = prep
            .get("recentWins")
            .and_then(|v| v.as_array())
            .or_else(|| prep.get("talkingPoints").and_then(|v| v.as_array()))
        {
            prep_context.push_str("Recent Wins:\n");
            for p in points {
                if let Some(t) = p.as_str() {
                    prep_context.push_str(&format!("- {}\n", t));
                }
            }
        }
        if let Some(notes) = prep.get("calendarNotes").and_then(|v| v.as_str()) {
            if !notes.trim().is_empty() {
                prep_context.push_str("Calendar Notes:\n");
                prep_context.push_str(notes.trim());
                prep_context.push('\n');
            }
        }
        let calendar_agenda = extract_calendar_agenda_items(&prep);
        if !calendar_agenda.is_empty() {
            prep_context.push_str("Calendar Agenda:\n");
            for item in &calendar_agenda {
                prep_context.push_str(&format!("- {}\n", item));
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
                let overdue = item
                    .get("isOverdue")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                prep_context.push_str(&format!(
                    "- {}{}\n",
                    title,
                    if overdue { " [OVERDUE]" } else { "" }
                ));
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
        "You are refining meeting prep reports for a Customer Success Manager.\n\n\
         For each meeting below, review recent wins, risks, open items, questions, \
         calendar notes, and current mechanical agenda. Produce:\n\
         1) A refined agenda that:\n\
         0. Keeps calendar agenda items as primary structure when they exist (enrich around them, do not replace them)\n\
         1. Orders items by impact (highest-stakes first)\n\
         2. Adds a brief 'why' rationale for each item\n\
         3. Uses source category (calendar_note, risk, talking_point, question, open_item)\n\
         4. Avoids duplicating recent wins unless there are no other substantive topics\n\
         5. Caps at 7 items per meeting\n\
         2) A clean recent wins list (max 4) with source provenance separated.\n\n\
         Format your response as:\n\
         AGENDA:meeting-id\n\
         ITEM:topic text\n\
         WHY:rationale\n\
         SOURCE:source_category\n\
         END_ITEM\n\
         ... more items ...\n\
         END_AGENDA\n\
         WINS:meeting-id\n\
         WIN:concise win statement (no markdown, no inline source: tail)\n\
         SOURCE:path-or-label (optional, only if known)\n\
         END_WIN\n\
         ... more wins ...\n\
         END_WINS\n\n\
         {}",
        prep_context
    );

    let output = pty
        .spawn_claude(workspace, &prompt)
        .map_err(|e| format!("Claude prep enrichment failed: {}", e))?;

    // Audit trail (I297)
    let date_id = data_dir.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");
    let _ = crate::audit::write_audit_entry(workspace, "meeting_prep", date_id, &output.stdout);

    let enrichments = parse_prep_enrichment(&output.stdout);
    if enrichments.is_empty() {
        log::warn!("enrich_preps: no prep enrichments parsed from Claude output");
        return Ok(());
    }

    // Merge enriched agendas/wins back into prep files
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

        if let Some(enrichment) = enrichments.get(&meeting_id) {
            let mut agenda_seen: HashSet<String> = HashSet::new();
            let mut win_seen: HashSet<String> = HashSet::new();
            let mut source_seen: HashSet<String> = HashSet::new();

            let agenda_json: Vec<Value> = enrichment
                .agenda
                .iter()
                .filter_map(|item| {
                    let topic = sanitize_prep_line(&item.topic)?;
                    let key = topic.to_lowercase();
                    if agenda_seen.contains(&key) {
                        return None;
                    }
                    agenda_seen.insert(key);
                    let mut obj = json!({"topic": topic});
                    if let Some(m) = obj.as_object_mut() {
                        if let Some(ref why) = item.why {
                            if let Some(clean_why) = sanitize_prep_line(why) {
                                m.insert("why".to_string(), json!(clean_why));
                            }
                        }
                        if let Some(ref source) = item.source {
                            let clean_source = sanitize_inline_markdown(source).to_lowercase();
                            if !clean_source.is_empty() {
                                m.insert("source".to_string(), json!(clean_source));
                            }
                        }
                    }
                    Some(obj)
                })
                .take(7)
                .collect();

            let mut wins_json: Vec<String> = Vec::new();
            let mut win_sources_json: Vec<Value> = Vec::new();
            for win in &enrichment.wins {
                let (win_without_source, embedded_source) = split_inline_source_tail(&win.win);
                if let Some(clean_win) = sanitize_recent_win_line(&win_without_source) {
                    let key = clean_win.to_lowercase();
                    if !win_seen.contains(&key) {
                        win_seen.insert(key);
                        wins_json.push(clean_win);
                    }
                }

                let source_text = win.source.clone().or(embedded_source);
                if let Some(source_text) = source_text {
                    let source_key = source_text.to_lowercase();
                    if !source_seen.contains(&source_key) {
                        if let Some(source_ref) = source_reference_value(&source_text) {
                            source_seen.insert(source_key);
                            win_sources_json.push(source_ref);
                        }
                    }
                }
            }

            if let Some(obj) = prep.as_object_mut() {
                if !agenda_json.is_empty() {
                    obj.insert("proposedAgenda".to_string(), json!(agenda_json));
                }
                if !wins_json.is_empty() {
                    obj.insert("recentWins".to_string(), json!(wins_json.clone()));
                    // Keep legacy field in sync for older consumers.
                    obj.insert("talkingPoints".to_string(), json!(wins_json));
                }
                if !win_sources_json.is_empty() {
                    obj.insert("recentWinSources".to_string(), json!(win_sources_json));
                }
            } else {
                log::warn!("enrich_preps: prep is not a JSON object, skipping enrichment merge");
            }

            if let Err(e) = write_json(path, &prep) {
                log::warn!(
                    "enrich_preps: failed to write enriched prep {}: {}",
                    path.display(),
                    e
                );
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
// Week AI Enrichment (I94 + I95)
// ============================================================================

/// Parse a week narrative from Claude output.
///
/// Expected format:
/// ```text
/// WEEK_NARRATIVE:
/// 2-3 sentence narrative here.
/// END_WEEK_NARRATIVE
/// ```
pub fn parse_week_narrative(response: &str) -> Option<String> {
    let mut in_block = false;
    let mut lines: Vec<&str> = Vec::new();

    for line in response.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("WEEK_NARRATIVE:") {
            in_block = true;
            let after = trimmed.strip_prefix("WEEK_NARRATIVE:").unwrap().trim();
            if !after.is_empty() {
                lines.push(after);
            }
        } else if trimmed == "END_WEEK_NARRATIVE" {
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

/// Parse a top priority from Claude output.
///
/// Expected format:
/// ```text
/// TOP_PRIORITY:
/// { "title": "...", "reason": "...", "meetingId": "...", "actionId": "..." }
/// END_TOP_PRIORITY
/// ```
pub fn parse_top_priority(response: &str) -> Option<crate::types::TopPriority> {
    let mut in_block = false;
    let mut lines: Vec<&str> = Vec::new();

    for line in response.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("TOP_PRIORITY:") {
            in_block = true;
            let after = trimmed.strip_prefix("TOP_PRIORITY:").unwrap().trim();
            if !after.is_empty() {
                lines.push(after);
            }
        } else if trimmed == "END_TOP_PRIORITY" {
            break;
        } else if in_block {
            lines.push(trimmed);
        }
    }

    if lines.is_empty() {
        return None;
    }

    let json_str = lines.join(" ");
    serde_json::from_str(&json_str).ok()
}

/// A time-block suggestion from AI enrichment (I95).
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeSuggestion {
    pub block_day: String,
    pub block_start: String,
    pub suggested_use: String,
    #[serde(default)]
    pub action_id: Option<String>,
    #[serde(default)]
    pub meeting_id: Option<String>,
}

/// Parse time-block suggestions from Claude output.
///
/// Expected format:
/// ```text
/// SUGGESTIONS:
/// [{ "blockDay": "Monday", "blockStart": "11:00 AM", "suggestedUse": "..." }, ...]
/// END_SUGGESTIONS
/// ```
pub fn parse_time_suggestions(response: &str) -> Vec<TimeSuggestion> {
    let mut in_block = false;
    let mut lines: Vec<&str> = Vec::new();

    for line in response.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("SUGGESTIONS:") {
            in_block = true;
            let after = trimmed.strip_prefix("SUGGESTIONS:").unwrap().trim();
            if !after.is_empty() {
                lines.push(after);
            }
        } else if trimmed == "END_SUGGESTIONS" {
            break;
        } else if in_block {
            lines.push(trimmed);
        }
    }

    if lines.is_empty() {
        return Vec::new();
    }

    let json_str = lines.join(" ");
    serde_json::from_str(&json_str).unwrap_or_default()
}

/// I137: Build entity intelligence context for accounts with meetings this week.
///
/// Walks dayShapes[].meetings[] to find unique account IDs, looks up cached
/// intelligence from the DB. Same pattern as daily but over the whole week.
fn build_entity_intel_for_week(overview: &Value, db: &crate::db::ActionDb) -> String {
    let day_shapes = match overview.get("dayShapes").and_then(|v| v.as_array()) {
        Some(arr) => arr,
        None => return String::new(),
    };

    let mut seen = std::collections::HashSet::new();
    let mut parts = Vec::new();

    for shape in day_shapes {
        let meetings = match shape.get("meetings").and_then(|v| v.as_array()) {
            Some(arr) => arr,
            None => continue,
        };

        for meeting in meetings {
            // Collect entity IDs: prefer linkedEntities, then accountId, then resolve account name
            let mut entity_ids: Vec<(String, String)> = Vec::new();

            if let Some(linked) = meeting.get("linkedEntities").and_then(|v| v.as_array()) {
                for le in linked {
                    let id = le.get("id").and_then(|v| v.as_str()).unwrap_or("");
                    let name = le.get("name").and_then(|v| v.as_str()).unwrap_or(id);
                    if !id.is_empty() {
                        entity_ids.push((id.to_string(), name.to_string()));
                    }
                }
            }

            if entity_ids.is_empty() {
                if let Some(aid) = meeting.get("accountId").and_then(|v| v.as_str()) {
                    if !aid.is_empty() {
                        let name = meeting
                            .get("account")
                            .and_then(|v| v.as_str())
                            .unwrap_or(aid);
                        entity_ids.push((aid.to_string(), name.to_string()));
                    }
                }
            }

            // Last resort: resolve account display name → entity ID via DB
            if entity_ids.is_empty() {
                if let Some(acct_name) = meeting.get("account").and_then(|v| v.as_str()) {
                    if !acct_name.is_empty() {
                        if let Ok(Some(account)) = db.get_account_by_name(acct_name) {
                            entity_ids.push((account.id, acct_name.to_string()));
                        }
                    }
                }
            }

            for (entity_id, entity_name) in entity_ids {
                if !seen.insert(entity_id.clone()) {
                    continue;
                }

                let intel = match db.get_entity_intelligence(&entity_id) {
                    Ok(Some(intel)) => intel,
                    _ => continue,
                };

                let mut block = format!("### {}\n", entity_name);

                if let Some(ref assessment) = intel.executive_assessment {
                    let truncated = if assessment.len() > 300 {
                        format!("{}...", &assessment[..300])
                    } else {
                        assessment.clone()
                    };
                    block.push_str(&truncated);
                    block.push('\n');
                }

                if !intel.risks.is_empty() {
                    block.push_str("Risks: ");
                    let risk_texts: Vec<&str> = intel
                        .risks
                        .iter()
                        .take(3)
                        .map(|r| r.text.as_str())
                        .collect();
                    block.push_str(&risk_texts.join("; "));
                    block.push('\n');
                }

                if let Some(ref readiness) = intel.next_meeting_readiness {
                    if !readiness.prep_items.is_empty() {
                        block.push_str("Readiness: ");
                        block.push_str(&readiness.prep_items.join("; "));
                        block.push('\n');
                    }
                }

                parts.push(block);
            }
        }
    }

    parts.join("\n")
}

/// AI-generate a week narrative, top priority, and time-block suggestions (I94 + I95).
///
/// Reads week-overview.json, builds context, asks Claude for structured output,
/// patches the fields back into week-overview.json.
/// Fault-tolerant: returns Ok(()) even if parsing fails — mechanical data stays intact.
pub fn enrich_week(
    data_dir: &Path,
    pty: &crate::pty::PtyManager,
    workspace: &Path,
    user_ctx: &crate::types::UserContext,
    state: &crate::state::AppState,
) -> Result<(), String> {
    // Read week-overview.json
    let overview_path = data_dir.join("week-overview.json");
    let raw = fs::read_to_string(&overview_path)
        .map_err(|e| format!("Failed to read week-overview.json: {}", e))?;
    let mut overview: Value = serde_json::from_str(&raw)
        .map_err(|e| format!("Failed to parse week-overview.json: {}", e))?;

    // Extract context for prompt
    let week_number = overview
        .get("weekNumber")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let date_range = overview
        .get("dateRange")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let day_shapes = overview.get("dayShapes").and_then(|v| v.as_array());
    let total_meetings: usize = day_shapes
        .map(|shapes| {
            shapes
                .iter()
                .filter_map(|s| s.get("meetingCount").and_then(|v| v.as_u64()))
                .sum::<u64>() as usize
        })
        .unwrap_or(0);

    let day_summary: Vec<String> = day_shapes
        .map(|shapes| {
            shapes
                .iter()
                .map(|s| {
                    let day = s.get("dayName").and_then(|v| v.as_str()).unwrap_or("?");
                    let count = s.get("meetingCount").and_then(|v| v.as_u64()).unwrap_or(0);
                    let density = s.get("density").and_then(|v| v.as_str()).unwrap_or("?");
                    format!("{}: {} meetings ({})", day, count, density)
                })
                .collect()
        })
        .unwrap_or_default();

    let readiness_checks = overview
        .get("readinessChecks")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let hygiene_count = overview
        .get("hygieneAlerts")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);

    // Collect available time blocks for suggestion context
    let available_blocks: Vec<String> = day_shapes
        .map(|shapes| {
            shapes
                .iter()
                .flat_map(|s| {
                    let day = s.get("dayName").and_then(|v| v.as_str()).unwrap_or("?");
                    s.get("availableBlocks")
                        .and_then(|v| v.as_array())
                        .map(|blocks| {
                            blocks
                                .iter()
                                .filter_map(|b| {
                                    let start = b.get("start").and_then(|v| v.as_str())?;
                                    let mins = b.get("durationMinutes").and_then(|v| v.as_u64())?;
                                    let display = format_time_display(start);
                                    Some(format!("{} {} ({}min)", day, display, mins))
                                })
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default()
                })
                .collect()
        })
        .unwrap_or_default();

    // Collect key meetings (customer/qbr) for priority context
    let key_meetings: Vec<String> = day_shapes
        .map(|shapes| {
            shapes
                .iter()
                .flat_map(|s| {
                    let day = s.get("dayName").and_then(|v| v.as_str()).unwrap_or("?");
                    s.get("meetings")
                        .and_then(|v| v.as_array())
                        .map(|meetings| {
                            meetings
                                .iter()
                                .filter(|m| {
                                    let t = m.get("type").and_then(|v| v.as_str()).unwrap_or("");
                                    t == "customer" || t == "qbr"
                                })
                                .filter_map(|m| {
                                    let title = m.get("title").and_then(|v| v.as_str())?;
                                    let acct =
                                        m.get("account").and_then(|v| v.as_str()).unwrap_or("");
                                    Some(format!("{} — {} ({})", day, title, acct))
                                })
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default()
                })
                .collect()
        })
        .unwrap_or_default();

    // Build rich action descriptions with titles for prompt context
    let overdue_action_lines: Vec<String> = overview
        .get("actionSummary")
        .and_then(|s| s.get("overdue"))
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let id = item.get("id").and_then(|v| v.as_str())?;
                    let title = item.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled");
                    let account = item.get("account").and_then(|v| v.as_str()).unwrap_or("");
                    let priority = item.get("priority").and_then(|v| v.as_str()).unwrap_or("P3");
                    let days = item.get("daysOverdue").and_then(|v| v.as_u64()).unwrap_or(0);
                    Some(format!(
                        "{}: {} ({}, {}, {}d overdue)",
                        id, title, account, priority, days
                    ))
                })
                .collect()
        })
        .unwrap_or_default();

    let due_this_week_lines: Vec<String> = overview
        .get("actionSummary")
        .and_then(|s| s.get("dueThisWeekItems"))
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let id = item.get("id").and_then(|v| v.as_str())?;
                    let title = item.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled");
                    let account = item.get("account").and_then(|v| v.as_str()).unwrap_or("");
                    let priority = item.get("priority").and_then(|v| v.as_str()).unwrap_or("P3");
                    let due = item.get("dueDate").and_then(|v| v.as_str()).unwrap_or("");
                    Some(format!(
                        "{}: {} ({}, {}, due {})",
                        id, title, account, priority, due
                    ))
                })
                .collect()
        })
        .unwrap_or_default();

    let candidate_action_ids: Vec<String> = overdue_action_lines
        .iter()
        .chain(due_this_week_lines.iter())
        .filter_map(|line| line.split(':').next().map(|s| s.to_string()))
        .collect();

    // Build rich meeting descriptions with titles
    let meeting_lines: Vec<String> = day_shapes
        .map(|shapes| {
            shapes
                .iter()
                .flat_map(|shape| {
                    let day = shape.get("dayName").and_then(|v| v.as_str()).unwrap_or("?");
                    shape
                        .get("meetings")
                        .and_then(|v| v.as_array())
                        .map(|meetings| {
                            meetings
                                .iter()
                                .filter_map(|m| {
                                    let id = m.get("meetingId").and_then(|v| v.as_str())?;
                                    let title =
                                        m.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled");
                                    let time =
                                        m.get("time").and_then(|v| v.as_str()).unwrap_or("");
                                    let acct =
                                        m.get("account").and_then(|v| v.as_str()).unwrap_or("");
                                    Some(format!(
                                        "{}: {} ({} {} {})",
                                        id, title, day, time, acct
                                    ))
                                })
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default()
                })
                .collect()
        })
        .unwrap_or_default();

    let candidate_meeting_ids: Vec<String> = meeting_lines
        .iter()
        .filter_map(|line| line.split(':').next().map(|s| s.to_string()))
        .collect();

    // I137: Gather entity intelligence for accounts with meetings this week (brief DB lock)
    let week_intel_context = {
        let db_guard = state.db.lock().ok();
        match db_guard.as_ref().and_then(|g| g.as_ref()) {
            Some(db) => build_entity_intel_for_week(&overview, db),
            None => String::new(),
        }
    }; // DB lock released here, before PTY call

    let user_fragment = user_ctx.prompt_fragment();
    let role_label = user_ctx.title_or_default();

    let intel_section = if week_intel_context.is_empty() {
        String::new()
    } else {
        format!(
            "\n## Entity Intelligence (for this week's accounts)\n\
             Use this for cross-entity synthesis: which account needs most attention? \
             Where should time be shifted? Reference specific risks or readiness items.\n\n\
             {}\n",
            week_intel_context
        )
    };

    let overdue_lines_str = if overdue_action_lines.is_empty() {
        "none".to_string()
    } else {
        overdue_action_lines.join("\n  ")
    };
    let due_lines_str = if due_this_week_lines.is_empty() {
        "none".to_string()
    } else {
        due_this_week_lines.join("\n  ")
    };
    let meeting_lines_str = if meeting_lines.is_empty() {
        "none".to_string()
    } else {
        meeting_lines.join("\n  ")
    };

    let prompt = format!(
        "You are writing a weekly briefing for {role_label}.\n\
         {user_fragment}\n\
         Week context:\n\
         - Week: {week_number} ({date_range})\n\
         - Total meetings: {total_meetings}\n\
         - Day breakdown: {day_breakdown}\n\
         - Key customer meetings: {key_meetings}\n\
         - Readiness checks: {readiness_checks} items needing attention\n\
         - Account health alerts: {hygiene_count}\n\
         - Overdue actions:\n  {overdue_lines}\n\
         - Due this week:\n  {due_lines}\n\
         - Meetings:\n  {meeting_lines}\n\
         - Available focus blocks: {available_blocks}\n\
         - Valid action IDs: {candidate_action_ids}\n\
         - Valid meeting IDs: {candidate_meeting_ids}\n\
         {intel_section}\n\
         Respond with exactly three sections:\n\n\
         1. WEEK_NARRATIVE — Two sentences maximum. First sentence: the shape of the week \
         (e.g. \"Five customer meetings in three days, back-loaded toward Thursday.\"). \
         Second sentence: the single most important thing to know (e.g. \"Nielsen is the \
         priority — the monthly is Wednesday and the account just went yellow.\"). \
         Never use words like \"pivotal\", \"high-stakes\", \"stewardship\", or \"landscape\". \
         Write like a newsroom editor, not a consultant.\n\n\
         2. TOP_PRIORITY — Pick ONE specific deliverable, meeting, or action from the lists \
         above. Not a process (\"clear backlog\") or a strategy (\"maintain momentum\") — a \
         thing you can finish. Return as JSON: {{\"title\": \"...\", \"reason\": \"...\"}} \
         with optional \"actionId\" or \"meetingId\" ONLY when it maps to a specific item \
         from the valid IDs above. The reason should explain WHY this week, in one sentence.\n\n\
         3. SUGGESTIONS — For each available time block, suggest a specific use referencing \
         an action title or meeting title from the context above. Return as JSON array: \
         [{{\"blockDay\": \"Monday\", \"blockStart\": \"11:00 AM\", \"suggestedUse\": \"Review QBR deck ahead of Thursday's Acme meeting\"}}] \
         Write suggestedUse as a complete sentence. Never include score labels, fit \
         assessments, or reasoning metadata. Include actionId/meetingId ONLY when confident.\n\n\
         All output text (narrative, title, reason, suggestedUse) must be editorial prose \
         suitable for direct display to the user. Never include internal reasoning, score \
         labels, confidence levels, or metadata in any output field.\n\n\
         Format your response EXACTLY as:\n\n\
         WEEK_NARRATIVE:\n\
         <your narrative>\n\
         END_WEEK_NARRATIVE\n\n\
         TOP_PRIORITY:\n\
         {{\"title\": \"...\", \"reason\": \"...\"}}\n\
         END_TOP_PRIORITY\n\n\
         SUGGESTIONS:\n\
         [{{\"blockDay\": \"Monday\", \"blockStart\": \"11:00 AM\", \"suggestedUse\": \"...\", \"actionId\": \"optional\", \"meetingId\": \"optional\"}}]\n\
         END_SUGGESTIONS",
        role_label = role_label,
        user_fragment = user_fragment,
        week_number = week_number,
        date_range = date_range,
        total_meetings = total_meetings,
        day_breakdown = day_summary.join("; "),
        key_meetings = if key_meetings.is_empty() {
            "none".to_string()
        } else {
            key_meetings.join("; ")
        },
        readiness_checks = readiness_checks,
        hygiene_count = hygiene_count,
        overdue_lines = overdue_lines_str,
        due_lines = due_lines_str,
        meeting_lines = meeting_lines_str,
        available_blocks = if available_blocks.is_empty() {
            "none".to_string()
        } else {
            available_blocks.join("; ")
        },
        candidate_action_ids = if candidate_action_ids.is_empty() {
            "none".to_string()
        } else {
            candidate_action_ids.join(", ")
        },
        candidate_meeting_ids = if candidate_meeting_ids.is_empty() {
            "none".to_string()
        } else {
            candidate_meeting_ids.join(", ")
        },
    );

    let output = pty
        .spawn_claude(workspace, &prompt)
        .map_err(|e| format!("Claude week enrichment failed: {}", e))?;

    // Audit trail (I297)
    let _ = crate::audit::write_audit_entry(workspace, "week_forecast", week_number, &output.stdout);

    let response = &output.stdout;

    // Parse narrative
    let narrative = parse_week_narrative(response);
    if let Some(ref text) = narrative {
        overview
            .as_object_mut()
            .unwrap()
            .insert("weekNarrative".to_string(), json!(text));
        log::info!("enrich_week: narrative written ({} chars)", text.len());
    } else {
        log::warn!("enrich_week: no narrative parsed from Claude output");
    }

    // Parse top priority
    let priority = parse_top_priority(response);
    if let Some(ref p) = priority {
        let priority_json = serde_json::to_value(p).unwrap_or(json!(null));
        overview
            .as_object_mut()
            .unwrap()
            .insert("topPriority".to_string(), priority_json);
        log::info!("enrich_week: top priority set — {}", p.title);
    } else {
        log::warn!("enrich_week: no top priority parsed from Claude output");
    }

    // Parse time suggestions and apply to matching blocks
    let suggestions = parse_time_suggestions(response);
    if !suggestions.is_empty() {
        if let Some(shapes) = overview.get_mut("dayShapes").and_then(|v| v.as_array_mut()) {
            for suggestion in &suggestions {
                for shape in shapes.iter_mut() {
                    let day = shape.get("dayName").and_then(|v| v.as_str()).unwrap_or("");
                    if day != suggestion.block_day {
                        continue;
                    }
                    if let Some(blocks) = shape
                        .get_mut("availableBlocks")
                        .and_then(|v| v.as_array_mut())
                    {
                        for block in blocks.iter_mut() {
                            let start = block.get("start").and_then(|v| v.as_str()).unwrap_or("");
                            // Match by exact string OR by display-format time
                            // Block start is ISO ("2026-02-16T09:30:00"), suggestion is display ("9:30 AM")
                            let start_display = format_time_display(start);
                            let matches = start == suggestion.block_start
                                || start_display == suggestion.block_start
                                || start_display.to_lowercase() == suggestion.block_start.to_lowercase();
                            if matches {
                                block.as_object_mut().unwrap().insert(
                                    "suggestedUse".to_string(),
                                    json!(suggestion.suggested_use),
                                );
                                if let Some(ref action_id) = suggestion.action_id {
                                    if !action_id.trim().is_empty() {
                                        block
                                            .as_object_mut()
                                            .unwrap()
                                            .insert("actionId".to_string(), json!(action_id));
                                    }
                                }
                                if let Some(ref meeting_id) = suggestion.meeting_id {
                                    if !meeting_id.trim().is_empty() {
                                        block
                                            .as_object_mut()
                                            .unwrap()
                                            .insert("meetingId".to_string(), json!(meeting_id));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        log::info!(
            "enrich_week: applied {} time-block suggestions",
            suggestions.len()
        );
    }

    // Write back
    write_json(&overview_path, &overview)?;
    Ok(())
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
        // Test with explicit UTC timezone
        let utc_tz: Tz = "UTC".parse().unwrap();
        assert_eq!(
            format_time_display_tz("2025-02-07T09:00:00+00:00", Some(utc_tz)),
            "9:00 AM"
        );
        assert_eq!(
            format_time_display_tz("2025-02-07T14:30:00+00:00", Some(utc_tz)),
            "2:30 PM"
        );
        // Test with a known non-UTC timezone
        let est: Tz = "America/New_York".parse().unwrap();
        assert_eq!(
            format_time_display_tz("2025-02-07T14:00:00+00:00", Some(est)),
            "9:00 AM"
        );
        // Edge cases
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
    fn test_meeting_primary_id_prefers_event_id() {
        let id = meeting_primary_id(
            Some("abc123_20260210T100000Z@google.com"),
            "Acme Sync",
            "2026-02-10T10:00:00+00:00",
            "customer",
        );
        assert_eq!(id, "abc123_20260210T100000Z_at_google.com");
    }

    #[test]
    fn test_meeting_primary_id_falls_back_to_slug() {
        let id = meeting_primary_id(
            None,
            "Acme Q1 Sync",
            "2025-02-07T09:00:00+00:00",
            "customer",
        );
        assert!(id.starts_with("0900-customer-"));
        assert!(id.contains("acme"));
    }

    #[test]
    fn test_meeting_primary_id_empty_event_id_falls_back() {
        let id = meeting_primary_id(
            Some(""),
            "Acme Q1 Sync",
            "2025-02-07T09:00:00+00:00",
            "customer",
        );
        assert!(id.starts_with("0900-customer-"));
    }

    #[test]
    fn test_is_substantive_prep_with_content() {
        let dir = tempfile::tempdir().unwrap();
        let prep_path = dir.path().join("prep.json");
        let prep = json!({
            "meetingId": "test",
            "title": "Acme Call",
            "meetingType": "customer",
            "context": "Acme is a key customer with $1M ARR",
            "risks": ["Renewal at risk"],
        });
        fs::write(&prep_path, serde_json::to_string(&prep).unwrap()).unwrap();
        assert!(is_substantive_prep(&prep_path));
    }

    #[test]
    fn test_is_substantive_prep_stub_only() {
        let dir = tempfile::tempdir().unwrap();
        let prep_path = dir.path().join("prep.json");
        let prep = json!({
            "meetingId": "test",
            "title": "Weekly Sync",
            "meetingType": "internal",
        });
        fs::write(&prep_path, serde_json::to_string(&prep).unwrap()).unwrap();
        assert!(!is_substantive_prep(&prep_path));
    }

    #[test]
    fn test_is_substantive_prep_empty_arrays() {
        let dir = tempfile::tempdir().unwrap();
        let prep_path = dir.path().join("prep.json");
        let prep = json!({
            "meetingId": "test",
            "title": "Sync",
            "risks": [],
            "questions": [],
            "context": "",
        });
        fs::write(&prep_path, serde_json::to_string(&prep).unwrap()).unwrap();
        assert!(!is_substantive_prep(&prep_path));
    }

    #[test]
    fn test_reconcile_prep_flags() {
        let dir = tempfile::tempdir().unwrap();
        let data_dir = dir.path();

        // Create preps directory with one substantive and one stub
        let preps_dir = data_dir.join("preps");
        fs::create_dir_all(&preps_dir).unwrap();

        let good_prep = json!({
            "meetingId": "m1",
            "title": "Acme Call",
            "context": "Important customer",
        });
        let stub_prep = json!({
            "meetingId": "m2",
            "title": "Weekly",
        });
        fs::write(
            preps_dir.join("m1.json"),
            serde_json::to_string(&good_prep).unwrap(),
        )
        .unwrap();
        fs::write(
            preps_dir.join("m2.json"),
            serde_json::to_string(&stub_prep).unwrap(),
        )
        .unwrap();

        // Create schedule.json with both marked as hasPrep: true
        let schedule = json!({
            "date": "2026-02-10",
            "meetings": [
                {"id": "m1", "title": "Acme Call", "hasPrep": true, "prepFile": "preps/m1.json"},
                {"id": "m2", "title": "Weekly", "hasPrep": true, "prepFile": "preps/m2.json"},
            ],
        });
        write_json(&data_dir.join("schedule.json"), &schedule).unwrap();

        // Reconcile
        reconcile_prep_flags(data_dir).unwrap();

        // Check results
        let updated: Value =
            serde_json::from_str(&fs::read_to_string(data_dir.join("schedule.json")).unwrap())
                .unwrap();
        let meetings = updated["meetings"].as_array().unwrap();
        assert_eq!(meetings[0]["hasPrep"], true); // substantive
        assert_eq!(meetings[1]["hasPrep"], false); // stub
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
    fn test_find_meeting_context_prefers_event_id_over_account() {
        let contexts = vec![
            DirectiveMeetingContext {
                event_id: Some("evt-slack".to_string()),
                account: Some("Slack".to_string()),
                ..Default::default()
            },
            DirectiveMeetingContext {
                event_id: Some("evt-salesforce".to_string()),
                account: Some("Digital-Marketing-Technology".to_string()),
                ..Default::default()
            },
        ];

        let found = find_meeting_context(Some("Slack"), Some("evt-salesforce"), &contexts)
            .expect("expected a context match");

        assert_eq!(found.event_id.as_deref(), Some("evt-salesforce"));
        assert_eq!(
            found.account.as_deref(),
            Some("Digital-Marketing-Technology")
        );
    }

    #[test]
    fn test_build_prep_json_prefers_context_account() {
        let meeting = DirectiveMeeting {
            event_id: Some("evt-1".to_string()),
            title: Some("BU Sync".to_string()),
            account: Some("Slack".to_string()),
            start_display: Some("9:00 AM".to_string()),
            end_display: Some("9:30 AM".to_string()),
            ..Default::default()
        };
        let ctx = DirectiveMeetingContext {
            account: Some("Digital-Marketing-Technology".to_string()),
            ..Default::default()
        };

        let prep = build_prep_json(&meeting, "customer", "evt-1", Some(&ctx));
        assert_eq!(prep["account"], "Digital-Marketing-Technology");
    }

    #[test]
    fn test_deliver_schedule_prefers_context_account() {
        let dir = tempfile::tempdir().unwrap();
        let data_dir = dir.path().join("data");

        let directive = Directive {
            context: crate::json_loader::DirectiveContext {
                date: Some("2026-02-12".to_string()),
                ..Default::default()
            },
            calendar: crate::json_loader::DirectiveCalendar {
                events: vec![DirectiveEvent {
                    id: Some("evt-1".to_string()),
                    summary: Some("Customer Sync".to_string()),
                    start: Some("2026-02-12T14:00:00+00:00".to_string()),
                    end: Some("2026-02-12T14:30:00+00:00".to_string()),
                }],
            },
            meetings: {
                let mut m = HashMap::new();
                m.insert(
                    "customer".to_string(),
                    vec![DirectiveMeeting {
                        id: Some("evt-1".to_string()),
                        event_id: Some("evt-1".to_string()),
                        summary: Some("Customer Sync".to_string()),
                        account: Some("Slack".to_string()),
                        ..Default::default()
                    }],
                );
                m
            },
            meeting_contexts: vec![DirectiveMeetingContext {
                event_id: Some("evt-1".to_string()),
                account: Some("Digital-Marketing-Technology".to_string()),
                ..Default::default()
            }],
            actions: Default::default(),
            emails: Default::default(),
        };

        let schedule = deliver_schedule(&directive, &data_dir, None).unwrap();
        let meeting = schedule["meetings"]
            .as_array()
            .and_then(|arr| arr.first())
            .expect("expected one meeting");
        assert_eq!(meeting["account"], "Digital-Marketing-Technology");
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

        let result = deliver_schedule(&directive, &data_dir, None).unwrap();
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
        let actions =
            json!({"date": "2025-02-07", "summary": {"overdue": 0, "dueToday": 0}, "actions": []});
        let directive = Directive {
            context: crate::json_loader::DirectiveContext {
                date: Some("2025-02-07".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };

        let emails = json!({});
        let result = deliver_manifest(
            &directive,
            &schedule,
            &actions,
            &emails,
            &[],
            &data_dir,
            true,
        )
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
                high_priority: vec![crate::json_loader::DirectiveEmail {
                    id: Some("e1".to_string()),
                    from: Some("Alice".to_string()),
                    from_email: Some("alice@example.com".to_string()),
                    subject: Some("Contract renewal".to_string()),
                    snippet: Some("Please review the...".to_string()),
                    priority: Some("high".to_string()),
                }],
                classified: vec![
                    crate::json_loader::DirectiveEmail {
                        id: Some("e2".to_string()),
                        from: Some("Bob".to_string()),
                        from_email: Some("bob@example.com".to_string()),
                        subject: Some("Meeting notes".to_string()),
                        snippet: None,
                        priority: Some("medium".to_string()),
                    },
                    crate::json_loader::DirectiveEmail {
                        id: Some("e3".to_string()),
                        from: Some("Carol".to_string()),
                        from_email: Some("carol@example.com".to_string()),
                        subject: Some("Newsletter".to_string()),
                        snippet: Some("Weekly digest...".to_string()),
                        priority: Some("low".to_string()),
                    },
                ],
                medium_count: 3,
                low_count: 5,
                sync_error: None,
            },
            ..Default::default()
        };

        let result = deliver_emails(&directive, &data_dir).unwrap();
        let hp = result["highPriority"].as_array().unwrap();
        assert_eq!(hp.len(), 1);
        assert_eq!(hp[0]["sender"], "Alice");
        assert_eq!(hp[0]["priority"], "high");

        let mp = result["mediumPriority"].as_array().unwrap();
        assert_eq!(mp.len(), 1);
        assert_eq!(mp[0]["sender"], "Bob");

        let lp = result["lowPriority"].as_array().unwrap();
        assert_eq!(lp.len(), 1);
        assert_eq!(lp[0]["sender"], "Carol");

        assert_eq!(result["stats"]["highCount"], 1);
        assert_eq!(result["stats"]["mediumCount"], 1);
        assert_eq!(result["stats"]["lowCount"], 1);
        assert_eq!(result["stats"]["total"], 3);
        assert_eq!(result["sync"]["state"], "ok");
        assert!(data_dir.join("emails.json").exists());
    }

    #[test]
    fn test_deliver_emails_sync_error_preserves_last_known_good() {
        let dir = tempfile::tempdir().unwrap();
        let data_dir = dir.path().join("data");
        fs::create_dir_all(&data_dir).unwrap();

        let existing = json!({
            "highPriority": [{
                "id": "e-existing",
                "sender": "Alice",
                "senderEmail": "alice@example.com",
                "subject": "Existing",
                "priority": "high"
            }],
            "mediumPriority": [],
            "lowPriority": [],
            "stats": {
                "highCount": 1,
                "mediumCount": 0,
                "lowCount": 0,
                "total": 1
            },
            "sync": {
                "state": "ok",
                "stage": "deliver",
                "lastSuccessAt": "2026-02-11T10:00:00Z"
            }
        });
        write_json(&data_dir.join(EMAILS_FILE), &existing).unwrap();

        let directive = Directive {
            emails: crate::json_loader::DirectiveEmails {
                classified: vec![],
                high_priority: vec![],
                medium_count: 0,
                low_count: 0,
                sync_error: Some(crate::json_loader::DirectiveEmailSyncError {
                    stage: Some("fetch".to_string()),
                    code: Some("gmail_fetch_failed".to_string()),
                    message: Some("Fetch failed".to_string()),
                }),
            },
            ..Default::default()
        };

        let result = deliver_emails(&directive, &data_dir).unwrap();
        assert_eq!(result["highPriority"].as_array().unwrap().len(), 1);
        assert_eq!(result["sync"]["state"], "error");
        assert_eq!(result["sync"]["usingLastKnownGood"], true);
        assert_eq!(result["sync"]["lastSuccessAt"], "2026-02-11T10:00:00Z");
    }

    #[test]
    fn test_deliver_emails_sync_error_without_existing_writes_empty_payload() {
        let dir = tempfile::tempdir().unwrap();
        let data_dir = dir.path().join("data");

        let directive = Directive {
            emails: crate::json_loader::DirectiveEmails {
                classified: vec![],
                high_priority: vec![],
                medium_count: 0,
                low_count: 0,
                sync_error: Some(crate::json_loader::DirectiveEmailSyncError {
                    stage: Some("fetch".to_string()),
                    code: Some("gmail_auth_failed".to_string()),
                    message: Some("Auth failed".to_string()),
                }),
            },
            ..Default::default()
        };

        let result = deliver_emails(&directive, &data_dir).unwrap();
        assert_eq!(result["stats"]["total"], 0);
        assert_eq!(result["sync"]["state"], "error");
        assert_eq!(result["sync"]["usingLastKnownGood"], false);
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
    fn test_parse_briefing_focus() {
        let response = "\
NARRATIVE:
Busy day ahead with 3 customer calls.
END_NARRATIVE

FOCUS:
Nail the Acme QBR prep — the renewal decision is next month and usage is declining.
END_FOCUS
";
        let focus = parse_briefing_focus(response);
        assert!(focus.is_some());
        let text = focus.unwrap();
        assert!(text.contains("Acme QBR"));
        assert!(text.contains("renewal"));
    }

    #[test]
    fn test_parse_briefing_focus_missing() {
        let response = "NARRATIVE:\nSome narrative.\nEND_NARRATIVE";
        let focus = parse_briefing_focus(response);
        assert!(focus.is_none());
    }

    #[test]
    fn test_parse_briefing_focus_inline() {
        let response = "FOCUS: Review the SOW before the 2 PM legal call.\nEND_FOCUS";
        let focus = parse_briefing_focus(response);
        assert!(focus.is_some());
        assert!(focus.unwrap().contains("SOW"));
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
            &directive,
            &schedule,
            &actions,
            &emails,
            &[],
            &data_dir,
            false,
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
WINS:mtg-acme-weekly
WIN:Influence tier upgrade executed with engineering leader (source: 2026-02-11-sync.md)
SOURCE:2026-02-11-sync.md
END_WIN
END_WINS

AGENDA:mtg-globex-qbr
ITEM:Renewal proposal
WHY:90 days to renewal — need commitment
SOURCE:talking_point
END_ITEM
END_AGENDA
";
        let enrichments = parse_prep_enrichment(response);
        assert_eq!(enrichments.len(), 2);

        let acme = &enrichments["mtg-acme-weekly"].agenda;
        assert_eq!(acme.len(), 2);
        assert_eq!(acme[0].topic, "Address Team B usage decline");
        assert_eq!(
            acme[0].why.as_deref(),
            Some("25% drop threatens renewal — needs intervention plan")
        );
        assert_eq!(acme[0].source.as_deref(), Some("risk"));
        assert_eq!(acme[1].topic, "Celebrate Phase 1 completion");

        let acme_wins = &enrichments["mtg-acme-weekly"].wins;
        assert_eq!(acme_wins.len(), 1);
        assert!(acme_wins[0].win.contains("Influence tier upgrade executed"));
        assert_eq!(acme_wins[0].source.as_deref(), Some("2026-02-11-sync.md"));

        let globex = &enrichments["mtg-globex-qbr"].agenda;
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
    fn test_extract_calendar_agenda_items_from_notes() {
        let prep = json!({
            "calendarNotes": "Agenda:\n1. Renewal decision timeline\n- Review Team B adoption risk\nQuestions:\n- Confirm legal review owner",
        });
        let agenda = extract_calendar_agenda_items(&prep);
        assert_eq!(agenda.len(), 2);
        assert_eq!(agenda[0], "Renewal decision timeline");
        assert_eq!(agenda[1], "Review Team B adoption risk");
    }

    #[test]
    fn test_extract_calendar_agenda_numbered_inline() {
        let prep = json!({
            "calendarNotes": "Agenda: 1. Review proposal 2. Discuss timeline 3. Close action items",
        });
        let agenda = extract_calendar_agenda_items(&prep);
        assert_eq!(agenda.len(), 3);
        assert_eq!(agenda[0], "Review proposal");
        assert_eq!(agenda[1], "Discuss timeline");
        assert_eq!(agenda[2], "Close action items");
    }

    #[test]
    fn test_extract_calendar_agenda_semicolons() {
        let prep = json!({
            "calendarNotes": "Agenda: Review proposal; Discuss timeline; Close items",
        });
        let agenda = extract_calendar_agenda_items(&prep);
        assert_eq!(agenda.len(), 3);
        assert_eq!(agenda[0], "Review proposal");
        assert_eq!(agenda[1], "Discuss timeline");
        assert_eq!(agenda[2], "Close items");
    }

    #[test]
    fn test_extract_calendar_agenda_dedup() {
        let prep = json!({
            "calendarNotes": "Agenda:\n- Review proposal\n- Discuss timeline\n- Review proposal",
        });
        let agenda = extract_calendar_agenda_items(&prep);
        assert_eq!(agenda.len(), 2);
        assert_eq!(agenda[0], "Review proposal");
        assert_eq!(agenda[1], "Discuss timeline");
    }

    #[test]
    fn test_extract_calendar_agenda_empty_notes() {
        let prep = json!({});
        let agenda = extract_calendar_agenda_items(&prep);
        assert!(agenda.is_empty());

        let prep2 = json!({ "calendarNotes": null });
        let agenda2 = extract_calendar_agenda_items(&prep2);
        assert!(agenda2.is_empty());
    }

    #[test]
    fn test_extract_calendar_agenda_no_agenda_section() {
        let prep = json!({
            "calendarNotes": "Please join the call on time.\nDial-in: 555-1234",
        });
        let agenda = extract_calendar_agenda_items(&prep);
        assert!(agenda.is_empty());
    }

    #[test]
    fn test_extract_calendar_agenda_stops_at_next_header() {
        let prep = json!({
            "calendarNotes": "Agenda:\n- Review proposal\n- Discuss timeline\nQuestions:\n- Who owns legal review?\n- When is the deadline?",
        });
        let agenda = extract_calendar_agenda_items(&prep);
        assert_eq!(agenda.len(), 2);
        assert_eq!(agenda[0], "Review proposal");
        assert_eq!(agenda[1], "Discuss timeline");
    }

    #[test]
    fn test_extract_calendar_agenda_pipe_in_text() {
        // Validates Fix 1: pipes in agenda text should not cause incorrect splits.
        let prep = json!({
            "calendarNotes": "Agenda:\n- Review pipeline | Discuss metrics\n- Budget approval",
        });
        let agenda = extract_calendar_agenda_items(&prep);
        assert_eq!(agenda.len(), 2);
        assert_eq!(agenda[0], "Review pipeline | Discuss metrics");
        assert_eq!(agenda[1], "Budget approval");
    }

    #[test]
    fn test_extract_calendar_agenda_colon_variant() {
        // Validates that the first pass handles "agenda:" (with colon) — no fallback needed.
        let prep = json!({
            "calendarNotes": "agenda: Review Q1 numbers; Plan Q2 targets",
        });
        let agenda = extract_calendar_agenda_items(&prep);
        assert_eq!(agenda.len(), 2);
        assert_eq!(agenda[0], "Review Q1 numbers");
        assert_eq!(agenda[1], "Plan Q2 targets");
    }

    #[test]
    fn test_generate_mechanical_agenda_prefers_calendar_agenda() {
        let prep = json!({
            "calendarNotes": "Agenda: 1) Renewal timeline; 2) Expansion scope",
            "openItems": [
                {"title": "Send revised proposal", "isOverdue": true},
            ],
            "risks": ["Budget scrutiny from finance"],
        });
        let agenda = generate_mechanical_agenda(&prep);
        assert_eq!(agenda[0]["source"], "calendar_note");
        assert_eq!(agenda[0]["topic"], "Renewal timeline");
        assert_eq!(agenda[1]["source"], "calendar_note");
        assert_eq!(agenda[1]["topic"], "Expansion scope");
    }

    #[test]
    fn test_extract_calendar_agenda_rejects_prose_with_agenda_word() {
        // "Agenda" at the start of a sentence without a delimiter should NOT
        // trigger section mode — prevents false positive extraction.
        let prep = json!({
            "calendarNotes": "Agenda items were discussed in the last meeting.\nPlease review the notes.",
        });
        let agenda = extract_calendar_agenda_items(&prep);
        assert!(agenda.is_empty());
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

        // Should have items: 1 overdue + 2 risks + 2 questions + 1 non-overdue = 6
        assert_eq!(agenda.len(), 6);

        // First item should be the overdue follow-up
        assert!(agenda[0]["topic"]
            .as_str()
            .unwrap()
            .starts_with("Follow up:"));
        assert_eq!(agenda[0]["source"], "open_item");

        // Next 2 should be risks
        assert_eq!(agenda[1]["source"], "risk");
        assert_eq!(agenda[2]["source"], "risk");
        // Questions should be included before wins.
        assert_eq!(agenda[3]["source"], "question");
        assert_eq!(agenda[4]["source"], "question");
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
    fn test_generate_mechanical_agenda_uses_recent_wins_as_fallback() {
        let prep = json!({
            "recentWins": ["Recent win: Tier upgrade closed (source: notes.md)"],
        });
        let agenda = generate_mechanical_agenda(&prep);
        assert_eq!(agenda.len(), 1);
        assert_eq!(agenda[0]["topic"], "Tier upgrade closed");
        assert_eq!(agenda[0]["source"], "talking_point");
    }

    #[test]
    fn test_generate_mechanical_agenda_back_compat_talking_points_fallback() {
        let prep = json!({
            "talkingPoints": ["Recent win: Expansion signal from sponsor (source: call.md)"],
        });
        let agenda = generate_mechanical_agenda(&prep);
        assert_eq!(agenda.len(), 1);
        assert_eq!(agenda[0]["topic"], "Expansion signal from sponsor");
        assert_eq!(agenda[0]["source"], "talking_point");
    }

    #[test]
    fn test_sanitize_recent_win_line_removes_inline_source_tail() {
        let cleaned = sanitize_recent_win_line(
            "Recent win: Sponsor re-engaged _(source: 2026-02-11-transcript.md)_",
        );
        assert_eq!(cleaned.as_deref(), Some("Sponsor re-engaged"));
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

    // ================================================================
    // Week enrichment parser tests (I94 + I95)
    // ================================================================

    #[test]
    fn test_parse_week_narrative() {
        let response = "\
WEEK_NARRATIVE:
This is a customer-heavy week with 5 external meetings across 3 accounts.
The Globex QBR on Wednesday is the highest-stakes meeting.
END_WEEK_NARRATIVE
";
        let narrative = parse_week_narrative(response);
        assert!(narrative.is_some());
        let text = narrative.unwrap();
        assert!(text.contains("customer-heavy week"));
        assert!(text.contains("Globex QBR"));
    }

    #[test]
    fn test_parse_week_narrative_inline() {
        let response = "WEEK_NARRATIVE: Busy week ahead.\nEND_WEEK_NARRATIVE";
        let narrative = parse_week_narrative(response);
        assert_eq!(narrative, Some("Busy week ahead.".to_string()));
    }

    #[test]
    fn test_parse_week_narrative_missing() {
        let response = "Here's some random output without markers.";
        let narrative = parse_week_narrative(response);
        assert!(narrative.is_none());
    }

    #[test]
    fn test_parse_top_priority() {
        let response = r#"
TOP_PRIORITY:
{"title": "Nail the Globex QBR", "reason": "Renewal in 90 days, usage declining"}
END_TOP_PRIORITY
"#;
        let priority = parse_top_priority(response);
        assert!(priority.is_some());
        let p = priority.unwrap();
        assert_eq!(p.title, "Nail the Globex QBR");
        assert!(p.reason.contains("Renewal"));
        assert!(p.meeting_id.is_none());
        assert!(p.action_id.is_none());
    }

    #[test]
    fn test_parse_top_priority_with_ids() {
        let response = r#"
TOP_PRIORITY:
{"title": "Clear SOW", "reason": "Blocking legal", "meetingId": "mtg-1", "actionId": "act-1"}
END_TOP_PRIORITY
"#;
        let priority = parse_top_priority(response);
        assert!(priority.is_some());
        let p = priority.unwrap();
        assert_eq!(p.meeting_id, Some("mtg-1".to_string()));
        assert_eq!(p.action_id, Some("act-1".to_string()));
    }

    #[test]
    fn test_parse_top_priority_missing() {
        let response = "No priority markers here.";
        let priority = parse_top_priority(response);
        assert!(priority.is_none());
    }

    #[test]
    fn test_parse_top_priority_invalid_json() {
        let response = "TOP_PRIORITY:\nnot valid json\nEND_TOP_PRIORITY";
        let priority = parse_top_priority(response);
        assert!(priority.is_none()); // Graceful failure
    }

    #[test]
    fn test_parse_time_suggestions() {
        let response = r#"
SUGGESTIONS:
[{"blockDay": "Monday", "blockStart": "11:00 AM", "suggestedUse": "Globex QBR prep"}, {"blockDay": "Thursday", "blockStart": "2:00 PM", "suggestedUse": "Deep work"}]
END_SUGGESTIONS
"#;
        let suggestions = parse_time_suggestions(response);
        assert_eq!(suggestions.len(), 2);
        assert_eq!(suggestions[0].block_day, "Monday");
        assert_eq!(suggestions[0].block_start, "11:00 AM");
        assert_eq!(suggestions[0].suggested_use, "Globex QBR prep");
        assert_eq!(suggestions[1].block_day, "Thursday");
    }

    #[test]
    fn test_parse_time_suggestions_empty() {
        let response = "No suggestions here.";
        let suggestions = parse_time_suggestions(response);
        assert!(suggestions.is_empty());
    }

    #[test]
    fn test_parse_time_suggestions_invalid_json() {
        let response = "SUGGESTIONS:\nnot valid json\nEND_SUGGESTIONS";
        let suggestions = parse_time_suggestions(response);
        assert!(suggestions.is_empty()); // Graceful failure
    }

    #[test]
    fn test_week_overview_roundtrip_with_narrative() {
        // Verify the fixture template deserializes correctly with narrative + topPriority
        let json = r#"{
            "weekNumber": "W06",
            "dateRange": "2026-02-09 – 2026-02-13",
            "weekNarrative": "Test narrative about the week.",
            "topPriority": {
                "title": "Ship the feature",
                "reason": "Deadline is Friday."
            },
            "days": [],
            "readinessChecks": [
                { "checkType": "no_prep", "message": "Meeting X has no prep", "severity": "action_needed" }
            ],
            "dayShapes": [
                { "dayName": "Monday", "date": "2026-02-09", "meetingCount": 3, "meetingMinutes": 120, "density": "moderate", "meetings": [], "availableBlocks": [] }
            ],
            "actionSummary": {
                "overdueCount": 1,
                "dueThisWeek": 2,
                "criticalItems": ["Do the thing"],
                "overdue": [{ "id": "a1", "title": "Overdue task", "account": "Acme", "dueDate": "2026-02-07", "priority": "P1", "daysOverdue": 2 }],
                "dueThisWeekItems": [{ "id": "a2", "title": "Due task", "account": "Globex", "dueDate": "2026-02-12", "priority": "P2" }]
            },
            "hygieneAlerts": [
                { "account": "Globex", "lifecycle": "at-risk", "arr": "$800K", "issue": "Usage declining.", "severity": "warning" }
            ]
        }"#;

        let overview: crate::types::WeekOverview =
            serde_json::from_str(json).expect("Failed to deserialize WeekOverview");
        assert_eq!(
            overview.week_narrative.as_deref(),
            Some("Test narrative about the week.")
        );
        assert!(overview.top_priority.is_some());
        let tp = overview.top_priority.clone().unwrap();
        assert_eq!(tp.title, "Ship the feature");
        assert_eq!(tp.reason, "Deadline is Friday.");
        assert!(tp.meeting_id.is_none());
        assert!(tp.action_id.is_none());

        // Verify readiness checks, day shapes, actions, and hygiene also roundtrip
        assert_eq!(overview.readiness_checks.as_ref().unwrap().len(), 1);
        assert_eq!(overview.day_shapes.as_ref().unwrap().len(), 1);
        assert_eq!(overview.action_summary.as_ref().unwrap().overdue_count, 1);
        assert_eq!(overview.hygiene_alerts.as_ref().unwrap().len(), 1);

        // Re-serialize and verify fields survive
        let reserialized = serde_json::to_string(&overview).unwrap();
        assert!(reserialized.contains("weekNarrative"));
        assert!(reserialized.contains("topPriority"));
    }
}
