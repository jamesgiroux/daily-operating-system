use std::collections::{HashMap, HashSet};

use crate::state::AppState;
use crate::types::EmailSignal;

/// Build an editorial prose summary for an entity's email signals.
/// Instead of "2 risks, 1 expansion" produces something like
/// "Risk signals detected across 3 emails. Expansion opportunity flagged."
pub fn build_entity_signal_prose(signals: &[EmailSignal], email_count: usize) -> String {
    let mut type_counts: HashMap<&str, usize> = HashMap::new();
    for s in signals {
        let key = s.signal_type.as_str();
        *type_counts.entry(key).or_insert(0) += 1;
    }

    if type_counts.is_empty() {
        return format!(
            "{} email{} this period.",
            email_count,
            if email_count == 1 { "" } else { "s" },
        );
    }

    let mut parts = Vec::new();

    // Risks first (most newsworthy)
    for key in &["risk", "churn", "escalation"] {
        if let Some(&count) = type_counts.get(key) {
            parts.push(if count == 1 {
                format!("One {} signal detected.", key)
            } else {
                format!("{} {} signals detected.", count, key)
            });
        }
    }

    // Positive signals
    for key in &["expansion", "positive", "success"] {
        if let Some(&count) = type_counts.get(key) {
            parts.push(if count == 1 {
                format!("{} opportunity flagged.", capitalize_first(key))
            } else {
                format!("{} {} signals.", count, key)
            });
        }
    }

    // Informational
    for key in &["question", "timeline", "sentiment", "feedback", "relationship"] {
        if let Some(&count) = type_counts.get(key) {
            if count == 1 {
                parts.push(format!("{} signal noted.", capitalize_first(key)));
            } else {
                parts.push(format!("{} {} signals.", count, key));
            }
        }
    }

    // Catch any remaining types
    for (key, &count) in &type_counts {
        let is_handled = ["risk", "churn", "escalation", "expansion", "positive",
            "success", "question", "timeline", "sentiment", "feedback", "relationship"]
            .contains(key);
        if !is_handled {
            parts.push(format!("{} {} signal{}.", count, key, if count == 1 { "" } else { "s" }));
        }
    }

    if parts.is_empty() {
        format!(
            "{} email{} this period.",
            email_count,
            if email_count == 1 { "" } else { "s" },
        )
    } else {
        parts.join(" ")
    }
}

pub fn capitalize_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().to_string() + c.as_str(),
    }
}

pub fn normalize_match_key(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect()
}

pub fn attendee_domains(attendees: &[String]) -> HashSet<String> {
    attendees
        .iter()
        .filter_map(|a| a.split('@').nth(1))
        .map(|d| d.to_lowercase())
        .collect()
}

/// Auto-extract distinctive title fragments as resolution keywords for an entity.
///
/// When a user manually links a meeting to an entity, this extracts words from
/// the meeting title that aren't generic meeting words (1:1, sync, review, etc.)
/// and aren't already in the entity's keyword list. This teaches the keyword
/// matcher to recognize similar titles in future meetings.
pub fn auto_extract_title_keywords(
    db: &crate::db::ActionDb,
    entity_id: &str,
    entity_type: &str,
    meeting_title: &str,
) -> Result<(), String> {
    // Words to ignore — generic meeting vocabulary
    const STOP_WORDS: &[&str] = &[
        "1:1", "1-1", "sync", "meeting", "call", "review", "check-in",
        "checkin", "catch", "up", "weekly", "daily", "monthly", "quarterly",
        "bi-weekly", "standup", "retro", "planning", "prep", "debrief",
        "follow", "kickoff", "onboarding", "training", "workshop", "session",
        "the", "a", "an", "and", "or", "of", "for", "with", "re", "fwd",
        "qbr", "ebr", "deck", "demo", "presentation",
    ];

    let stop: std::collections::HashSet<&str> = STOP_WORDS.iter().copied().collect();

    // Extract meaningful words from title (>= 3 chars, not stop words)
    let title_words: Vec<String> = meeting_title
        .split(|c: char| !c.is_alphanumeric() && c != '-' && c != '\'')
        .filter(|w| w.len() >= 3 && !stop.contains(&w.to_lowercase().as_str()))
        .map(|w| w.to_string())
        .collect();

    if title_words.is_empty() {
        return Ok(());
    }

    // Build candidate multi-word phrases from consecutive meaningful words
    let mut candidates: Vec<String> = Vec::new();
    // Individual words
    for w in &title_words {
        candidates.push(w.clone());
    }
    // Adjacent pairs (e.g., "Janus Henderson" from title words)
    for pair in title_words.windows(2) {
        candidates.push(format!("{} {}", pair[0], pair[1]));
    }

    // Load existing keywords
    let existing_json = match entity_type {
        "account" => db
            .get_account(entity_id)
            .ok()
            .flatten()
            .and_then(|a| a.keywords),
        "project" => db
            .get_project(entity_id)
            .ok()
            .flatten()
            .and_then(|p| p.keywords),
        _ => return Ok(()),
    };

    let mut keywords: Vec<String> = existing_json
        .as_deref()
        .and_then(|j| serde_json::from_str::<Vec<String>>(j).ok())
        .unwrap_or_default();

    let existing_lower: std::collections::HashSet<String> =
        keywords.iter().map(|k| k.to_lowercase()).collect();

    // Add new candidates that aren't already present
    let mut added = false;
    for candidate in candidates {
        if !existing_lower.contains(&candidate.to_lowercase()) {
            keywords.push(candidate);
            added = true;
        }
    }

    if !added {
        return Ok(());
    }

    // Persist updated keywords
    let json = serde_json::to_string(&keywords).map_err(|e| e.to_string())?;
    match entity_type {
        "account" => db
            .update_account_keywords(entity_id, &json)
            .map_err(|e| e.to_string()),
        "project" => db
            .update_project_keywords(entity_id, &json)
            .map_err(|e| e.to_string()),
        _ => Ok(()),
    }
}

/// Compute executive intelligence for the risk briefing page.
///
/// Loads config, schedule, calendar events, and skip-today signals,
/// then delegates to `crate::intelligence::compute_executive_intelligence`.
pub fn get_executive_intelligence(
    state: &AppState,
) -> Result<crate::intelligence::ExecutiveIntelligence, String> {
    // Load config for profile + workspace
    let config = state
        .config
        .read()
        .map_err(|_| "Lock poisoned")?
        .clone()
        .ok_or("No configuration loaded")?;

    let workspace = std::path::Path::new(&config.workspace_path);
    let today_dir = workspace.join("_today");

    // Load schedule meetings (merged with live calendar)
    let meetings = if today_dir.join("data").exists() {
        let briefing_meetings = crate::json_loader::load_schedule_json(&today_dir)
            .map(|(_overview, meetings)| meetings)
            .unwrap_or_default();
        let live_events = state
            .calendar_events
            .read()
            .map(|g| g.clone())
            .unwrap_or_default();
        let tz: chrono_tz::Tz = config
            .schedules
            .today
            .timezone
            .parse()
            .unwrap_or(chrono_tz::America::New_York);
        crate::calendar_merge::merge_meetings(briefing_meetings, &live_events, &tz)
    } else {
        Vec::new()
    };

    // Load cached skip-today from AI enrichment (if available)
    let skip_today = load_skip_today(&today_dir);

    // Compute intelligence from DB
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    Ok(crate::intelligence::compute_executive_intelligence(
        db,
        &meetings,
        &config.profile,
        skip_today,
    ))
}

/// Load cached SKIP TODAY results from `_today/data/intelligence.json`.
///
/// Written by AI enrichment. Returns empty vec if file doesn't exist or is
/// malformed — fault-tolerant per ADR-0042 principle.
fn load_skip_today(today_dir: &std::path::Path) -> Vec<crate::intelligence::SkipSignal> {
    let path = today_dir.join("data").join("intelligence.json");
    if !path.exists() {
        return Vec::new();
    }

    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str::<Vec<crate::intelligence::SkipSignal>>(&s).ok())
        .unwrap_or_default()
}
