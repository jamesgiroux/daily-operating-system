use std::collections::{HashMap, HashSet};

use crate::state::AppState;
use crate::types::EmailSignal;

/// Build an editorial prose summary for an entity's email signals.
///
/// Uses the AI-generated `signal_text` to give the user a hit of what each
/// signal refers to, rather than just counting types. Follows ADR-0083
/// product vocabulary: "signal" → "update"/"change", warm chief-of-staff voice.
///
/// Priority order: risk/churn/escalation first, then positive, then informational.
/// Each type gets one representative line using the actual signal content.
pub fn build_entity_signal_prose(signals: &[EmailSignal], email_count: usize) -> String {
    if signals.is_empty() {
        return if email_count == 1 {
            "1 email — routine correspondence.".to_string()
        } else {
            format!("{} emails — routine correspondence.", email_count)
        };
    }

    // Group signals by type, keeping the signal_text for each
    let mut by_type: HashMap<&str, Vec<&str>> = HashMap::new();
    for s in signals {
        by_type
            .entry(s.signal_type.as_str())
            .or_default()
            .push(s.signal_text.as_str());
    }

    let mut parts = Vec::new();

    // Risk signals first (most newsworthy) — use signal_text for substance
    for key in &["risk", "churn", "escalation"] {
        if let Some(texts) = by_type.remove(key) {
            let label = match *key {
                "churn" => "Churn risk",
                "escalation" => "Escalation",
                _ => "Risk",
            };
            // Use the first non-empty signal_text as the detail
            let detail = texts.iter().find(|t| !t.is_empty()).copied();
            if let Some(text) = detail {
                let truncated = truncate_signal_text(text, 80);
                parts.push(format!("{}: {}", label, truncated));
            } else {
                parts.push(format!("{} flagged.", label));
            }
        }
    }

    // Positive signals — expansion, success, positive
    for key in &["expansion", "positive", "success"] {
        if let Some(texts) = by_type.remove(key) {
            let label = match *key {
                "expansion" => "Expansion",
                "success" => "Win",
                _ => "Positive",
            };
            let detail = texts.iter().find(|t| !t.is_empty()).copied();
            if let Some(text) = detail {
                let truncated = truncate_signal_text(text, 80);
                parts.push(format!("{}: {}", label, truncated));
            } else {
                parts.push(format!("{} opportunity noted.", label));
            }
        }
    }

    // Informational signals — use signal_text for context
    for key in &[
        "question",
        "timeline",
        "sentiment",
        "feedback",
        "relationship",
        "commitment",
    ] {
        if let Some(texts) = by_type.remove(key) {
            let label = match *key {
                "question" => "Open question",
                "timeline" => "Timeline",
                "sentiment" => "Sentiment shift",
                "feedback" => "Feedback",
                "relationship" => "Relationship",
                "commitment" => "Commitment",
                _ => *key,
            };
            let detail = texts.iter().find(|t| !t.is_empty()).copied();
            if let Some(text) = detail {
                let truncated = truncate_signal_text(text, 80);
                parts.push(format!("{}: {}", label, truncated));
            } else {
                parts.push(format!("{} noted.", capitalize_first(label)));
            }
        }
    }

    // Catch any remaining signal types not in the priority lists
    for (key, texts) in &by_type {
        let detail = texts.iter().find(|t| !t.is_empty()).copied();
        if let Some(text) = detail {
            let truncated = truncate_signal_text(text, 80);
            parts.push(format!("{}: {}", capitalize_first(key), truncated));
        } else {
            parts.push(format!("{} noted.", capitalize_first(key)));
        }
    }

    if parts.is_empty() {
        if email_count == 1 {
            "1 email — routine correspondence.".to_string()
        } else {
            format!("{} emails — routine correspondence.", email_count)
        }
    } else {
        parts.join(" ")
    }
}

/// Truncate signal text to a max length, breaking at a word boundary.
/// Ensures the text ends with a period if it doesn't already.
fn truncate_signal_text(text: &str, max_len: usize) -> String {
    let trimmed = text.trim().trim_end_matches('.');
    if trimmed.len() <= max_len {
        return format!("{}.", trimmed);
    }
    // Break at the last space before max_len
    let truncated = &trimmed[..max_len];
    let break_at = truncated.rfind(' ').unwrap_or(max_len);
    format!("{}.", trimmed[..break_at].trim_end_matches(',').trim())
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
    ctx: &crate::services::context::ServiceContext<'_>,
    db: &crate::db::ActionDb,
    entity_id: &str,
    entity_type: &str,
    meeting_title: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    // Words to ignore — generic meeting vocabulary
    const STOP_WORDS: &[&str] = &[
        "1:1",
        "1-1",
        "sync",
        "meeting",
        "call",
        "review",
        "check-in",
        "checkin",
        "catch",
        "up",
        "weekly",
        "daily",
        "monthly",
        "quarterly",
        "bi-weekly",
        "standup",
        "retro",
        "planning",
        "prep",
        "debrief",
        "follow",
        "kickoff",
        "onboarding",
        "training",
        "workshop",
        "session",
        "the",
        "a",
        "an",
        "and",
        "or",
        "of",
        "for",
        "with",
        "re",
        "fwd",
        "qbr",
        "ebr",
        "deck",
        "demo",
        "presentation",
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
    // Adjacent pairs (e.g., "Acme Corp" from title words)
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
pub async fn get_executive_intelligence(
    state: &AppState,
) -> Result<crate::intelligence::ExecutiveIntelligence, String> {
    // Load config for profile + workspace
    let config = state
        .config
        .read()
        .clone()
        .ok_or("No configuration loaded")?;

    let workspace = std::path::Path::new(&config.workspace_path);
    let today_dir = workspace.join("_today");

    // Load today's meetings from DB, merge with live calendar (I513)
    let meetings = {
        let tz_ent: chrono_tz::Tz = config
            .schedules
            .today
            .timezone
            .parse()
            .unwrap_or(chrono_tz::America::New_York);
        let tf_ent = crate::helpers::today_meeting_filter(&tz_ent);
        let today = tf_ent.date;
        let tomorrow = tf_ent.next_date;
        let db_meetings: Vec<crate::types::Meeting> = state
            .db_read(move |db| {
                let conn = db.conn_ref();
                let mut stmt = conn
                    .prepare(
                        "SELECT id, title, meeting_type, start_time, end_time, calendar_event_id
                         FROM meetings
                         WHERE start_time >= ?1 AND start_time < ?2
                         ORDER BY start_time ASC",
                    )
                    .map_err(|e| e.to_string())?;
                let rows = stmt
                    .query_map(rusqlite::params![today, tomorrow], |row| {
                        let id: String = row.get(0)?;
                        let title: String = row.get(1)?;
                        let mt: String = row.get(2)?;
                        let start: String = row.get(3)?;
                        let end: Option<String> = row.get(4)?;
                        let cal_id: Option<String> = row.get(5)?;
                        Ok((id, title, mt, start, end, cal_id))
                    })
                    .map_err(|e| e.to_string())?;
                let meetings = rows
                    .filter_map(|r| r.ok())
                    .map(|(id, title, mt, start, end, cal_id)| {
                        let meeting_type = crate::parser::parse_meeting_type(&mt);
                        let time =
                            chrono::NaiveDateTime::parse_from_str(&start, "%Y-%m-%dT%H:%M:%S")
                                .map(|dt| dt.format("%-I:%M %p").to_string())
                                .or_else(|_| {
                                    chrono::DateTime::parse_from_rfc3339(&start)
                                        .map(|dt| dt.format("%-I:%M %p").to_string())
                                })
                                .unwrap_or_else(|_| start.clone());
                        let end_time = end.as_ref().and_then(|et| {
                            chrono::NaiveDateTime::parse_from_str(et, "%Y-%m-%dT%H:%M:%S")
                                .map(|dt| dt.format("%-I:%M %p").to_string())
                                .or_else(|_| {
                                    chrono::DateTime::parse_from_rfc3339(et)
                                        .map(|dt| dt.format("%-I:%M %p").to_string())
                                })
                                .ok()
                        });
                        crate::types::Meeting {
                            id,
                            calendar_event_id: cal_id,
                            time,
                            end_time,
                            start_iso: Some(start),
                            title,
                            meeting_type,
                            prep: None,
                            is_current: None,
                            prep_file: None,
                            has_prep: false,
                            overlay_status: None,
                            prep_reviewed: None,
                            linked_entities: None,
                            suggested_unarchive_account_id: None,
                            intelligence_quality: None,
                            calendar_attendees: None,
                            calendar_description: None,
                        }
                    })
                    .collect();
                Ok(meetings)
            })
            .await
            .unwrap_or_default();

        let live_events = state
            .calendar
            .events
            .read()
            .clone();
        let tz: chrono_tz::Tz = config
            .schedules
            .today
            .timezone
            .parse()
            .unwrap_or(chrono_tz::America::New_York);
        crate::calendar_merge::merge_meetings(db_meetings, &live_events, &tz)
    };

    // Load cached skip-today from AI enrichment (if available)
    let skip_today = load_skip_today(&today_dir);

    let profile = config.profile.clone();
    // Compute intelligence from DB
    state
        .db_read(move |db| {
            Ok(crate::intelligence::compute_executive_intelligence(
                db, &meetings, &profile, skip_today,
            ))
        })
        .await
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
