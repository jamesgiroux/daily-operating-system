//! JSON data loader with markdown fallback
//!
//! This module provides functions to load data from JSON files in the `_today/data/`
//! directory, falling back to markdown parsing when JSON is not available.
//!
//! Migration strategy:
//! 1. Check for `_today/data/` directory
//! 2. If JSON exists and is valid, use it (fast path)
//! 3. If JSON missing or invalid, fall back to markdown parsing (legacy path)

use std::fs;
use std::path::Path;

use crate::types::{
    Action, DayOverview, Email, EmailSyncStatus, FullMeetingPrep, LinkedEntity, Meeting,
    MeetingPrep, WeekOverview,
};

/// Check if JSON data directory exists
pub fn has_json_data(today_dir: &Path) -> bool {
    today_dir.join("data").join("manifest.json").exists()
}

/// Load manifest to check what data is available
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub date: String,
    pub generated_at: String,
    #[serde(default)]
    pub partial: bool,
    pub files: Option<ManifestFiles>,
    pub stats: Option<ManifestStats>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestFiles {
    pub schedule: Option<String>,
    pub actions: Option<String>,
    pub emails: Option<String>,
    pub preps: Option<Vec<String>>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestStats {
    pub total_meetings: Option<u32>,
    pub customer_meetings: Option<u32>,
    pub actions_due: Option<u32>,
    pub emails_flagged: Option<u32>,
}

/// Whether the data in _today/data/ is from today or stale
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "freshness", rename_all = "camelCase")]
pub enum DataFreshness {
    Fresh {
        generated_at: String,
    },
    Stale {
        data_date: String,
        generated_at: String,
    },
    Unknown,
}

/// Check if the data in _today/data/ is from today
pub fn check_data_freshness(today_dir: &Path) -> DataFreshness {
    match load_manifest(today_dir) {
        Ok(manifest) => {
            let today = chrono::Local::now().format("%Y-%m-%d").to_string();
            if manifest.date == today {
                DataFreshness::Fresh {
                    generated_at: manifest.generated_at,
                }
            } else {
                DataFreshness::Stale {
                    data_date: manifest.date,
                    generated_at: manifest.generated_at,
                }
            }
        }
        Err(_) => DataFreshness::Unknown,
    }
}

/// Load manifest from data directory
pub fn load_manifest(today_dir: &Path) -> Result<Manifest, String> {
    let manifest_path = today_dir.join("data").join("manifest.json");
    let content = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read manifest: {}", e))?;
    serde_json::from_str(&content).map_err(|e| format!("Failed to parse manifest: {}", e))
}

/// JSON schedule format
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonSchedule {
    pub date: String,
    pub greeting: Option<String>,
    pub summary: Option<String>,
    pub focus: Option<String>,
    pub meetings: Vec<JsonMeeting>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonMeeting {
    pub id: String,
    pub calendar_event_id: Option<String>,
    pub time: String,
    pub end_time: Option<String>,
    #[serde(default)]
    pub start_iso: Option<String>,
    pub title: String,
    #[serde(rename = "type")]
    pub meeting_type: String,
    pub account: Option<String>,
    #[serde(default)]
    pub is_current: bool,
    pub has_prep: bool,
    pub prep_file: Option<String>,
    pub prep_summary: Option<JsonPrepSummary>,
    /// Entities linked via M2M junction table or entity resolution (I339)
    #[serde(default)]
    pub linked_entities: Option<Vec<LinkedEntity>>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonPrepSummary {
    pub at_a_glance: Option<Vec<String>>,
    pub discuss: Option<Vec<String>>,
    pub watch: Option<Vec<String>>,
    pub wins: Option<Vec<String>>,
    pub context: Option<String>,
    pub stakeholders: Option<Vec<JsonStakeholder>>,
    pub open_items: Option<Vec<String>>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonStakeholder {
    pub name: String,
    pub role: Option<String>,
    pub focus: Option<String>,
}

/// Load schedule from JSON
pub fn load_schedule_json(today_dir: &Path) -> Result<(DayOverview, Vec<Meeting>), String> {
    let schedule_path = today_dir.join("data").join("schedule.json");
    let content = fs::read_to_string(&schedule_path)
        .map_err(|e| format!("Failed to read schedule: {}", e))?;
    let schedule: JsonSchedule =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse schedule: {}", e))?;

    let overview = DayOverview {
        greeting: schedule
            .greeting
            .unwrap_or_else(|| "Good morning".to_string()),
        date: schedule.date,
        summary: schedule.summary.unwrap_or_default(),
        focus: schedule.focus,
    };

    let meetings: Vec<Meeting> = schedule
        .meetings
        .into_iter()
        .map(|m| {
            let meeting_type = match m.meeting_type.as_str() {
                "customer" => crate::types::MeetingType::Customer,
                "qbr" => crate::types::MeetingType::Qbr,
                "training" => crate::types::MeetingType::Training,
                "internal" => crate::types::MeetingType::Internal,
                "team_sync" => crate::types::MeetingType::TeamSync,
                "one_on_one" => crate::types::MeetingType::OneOnOne,
                "partnership" => crate::types::MeetingType::Partnership,
                "all_hands" => crate::types::MeetingType::AllHands,
                "external" => crate::types::MeetingType::External,
                "personal" => crate::types::MeetingType::Personal,
                _ => crate::types::MeetingType::Internal,
            };

            let prep = m.prep_summary.map(|ps| MeetingPrep {
                metrics: ps.at_a_glance,
                risks: ps.watch,
                wins: ps.wins,
                actions: ps.discuss,
                context: ps.context,
                stakeholders: ps.stakeholders.map(|slist| {
                    slist
                        .into_iter()
                        .map(|s| crate::types::Stakeholder {
                            name: s.name,
                            role: s.role,
                            focus: s.focus,
                        })
                        .collect()
                }),
                questions: None,
                open_items: ps.open_items,
                historical_context: None,
                source_references: None,
            });

            Meeting {
                id: m.id,
                calendar_event_id: m.calendar_event_id,
                time: m.time,
                end_time: m.end_time,
                start_iso: m.start_iso,
                title: m.title,
                meeting_type,
                prep,
                is_current: if m.is_current { Some(true) } else { None },
                prep_file: m.prep_file,
                has_prep: m.has_prep,
                overlay_status: None,
                prep_reviewed: None,
                linked_entities: None,
                suggested_unarchive_account_id: None,
                intelligence_quality: None,
            }
        })
        .collect();

    Ok((overview, meetings))
}

/// JSON actions format
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonActions {
    pub date: String,
    pub summary: Option<JsonActionsSummary>,
    pub actions: Vec<JsonAction>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonActionsSummary {
    pub overdue: Option<u32>,
    pub due_today: Option<u32>,
    pub due_this_week: Option<u32>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonAction {
    pub id: String,
    pub title: String,
    pub account: Option<String>,
    pub priority: String,
    pub status: String,
    pub due_date: Option<String>,
    #[serde(default)]
    pub is_overdue: bool,
    pub days_overdue: Option<u32>,
    pub context: Option<String>,
    pub source: Option<String>,
}

/// Load actions from JSON
pub fn load_actions_json(today_dir: &Path) -> Result<Vec<Action>, String> {
    let actions_path = today_dir.join("data").join("actions.json");
    let content =
        fs::read_to_string(&actions_path).map_err(|e| format!("Failed to read actions: {}", e))?;
    let data: JsonActions =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse actions: {}", e))?;

    let actions = data
        .actions
        .into_iter()
        .map(|a| {
            let priority = match a.priority.as_str() {
                "P1" => crate::types::Priority::P1,
                "P2" => crate::types::Priority::P2,
                _ => crate::types::Priority::P3,
            };

            let status = match a.status.as_str() {
                "completed" => crate::types::ActionStatus::Completed,
                _ => crate::types::ActionStatus::Pending,
            };

            Action {
                id: a.id,
                title: a.title,
                account: a.account,
                due_date: a.due_date,
                priority,
                status,
                is_overdue: if a.is_overdue { Some(true) } else { None },
                context: a.context,
                source: a.source,
                days_overdue: a.days_overdue.map(|d| d as i32),
            }
        })
        .collect();

    Ok(actions)
}

/// JSON emails format — matches what `deliver_emails()` writes:
/// `{ "highPriority": [...], "mediumPriority": [...], "lowPriority": [...], "stats": { ... } }`
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonEmails {
    #[serde(default)]
    pub high_priority: Vec<JsonEmail>,
    #[serde(default)]
    pub medium_priority: Vec<JsonEmail>,
    #[serde(default)]
    pub low_priority: Vec<JsonEmail>,
    pub stats: Option<JsonEmailStats>,
    #[serde(default)]
    pub sync: Option<EmailSyncStatus>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonEmailStats {
    pub high_count: Option<u32>,
    pub medium_count: Option<u32>,
    pub low_count: Option<u32>,
    pub total: Option<u32>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonEmail {
    pub id: String,
    pub sender: String,
    pub sender_email: Option<String>,
    pub subject: String,
    pub snippet: Option<String>,
    pub priority: String,
    /// AI-generated one-line summary
    pub summary: Option<String>,
    /// Suggested next action
    pub recommended_action: Option<String>,
    /// Thread history arc
    pub conversation_arc: Option<String>,
    /// Email category from AI classification
    pub email_type: Option<String>,
    /// Commitments extracted by AI enrichment (I354)
    #[serde(default)]
    pub commitments: Vec<String>,
    /// Questions extracted by AI enrichment (I354)
    #[serde(default)]
    pub questions: Vec<String>,
    /// Sentiment from AI enrichment (I354)
    pub sentiment: Option<String>,
}

/// Loaded email payload with optional sync status metadata.
#[derive(Debug)]
pub struct LoadedEmailsData {
    pub emails: Vec<Email>,
    pub sync: Option<EmailSyncStatus>,
}

/// Load emails from JSON
pub fn load_emails_json(today_dir: &Path) -> Result<Vec<Email>, String> {
    load_emails_json_with_sync(today_dir).map(|data| data.emails)
}

/// Load emails from JSON with sync metadata.
pub fn load_emails_json_with_sync(today_dir: &Path) -> Result<LoadedEmailsData, String> {
    let emails_path = today_dir.join("data").join("emails.json");
    let content =
        fs::read_to_string(&emails_path).map_err(|e| format!("Failed to read emails: {}", e))?;
    let data: JsonEmails =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse emails: {}", e))?;

    let all_emails = data
        .high_priority
        .into_iter()
        .chain(data.medium_priority)
        .chain(data.low_priority);

    let emails = all_emails
        .map(|e| {
            let priority = match e.priority.as_str() {
                "high" => crate::types::EmailPriority::High,
                "medium" => crate::types::EmailPriority::Medium,
                "low" => crate::types::EmailPriority::Low,
                // Legacy "normal" maps to medium
                "normal" => crate::types::EmailPriority::Medium,
                _ => crate::types::EmailPriority::Low,
            };

            Email {
                id: e.id,
                sender: e.sender,
                sender_email: e.sender_email.unwrap_or_default(),
                subject: e.subject,
                snippet: e.snippet,
                priority,
                avatar_url: None,
                summary: e.summary,
                recommended_action: e.recommended_action,
                conversation_arc: e.conversation_arc,
                email_type: e.email_type,
                commitments: e.commitments,
                questions: e.questions,
                sentiment: e.sentiment,
            }
        })
        .collect();

    Ok(LoadedEmailsData {
        emails,
        sync: data.sync,
    })
}

/// JSON prep format
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonPrep {
    pub meeting_id: String,
    pub calendar_event_id: Option<String>,
    pub title: String,
    pub time_range: Option<String>,
    #[serde(rename = "type")]
    pub meeting_type: String,
    pub account: Option<String>,
    pub meeting_context: Option<String>,
    pub quick_context: Option<std::collections::HashMap<String, String>>,
    pub attendees: Option<Vec<JsonStakeholder>>,
    pub since_last: Option<Vec<String>>,
    pub strategic_programs: Option<Vec<JsonProgram>>,
    pub current_state: Option<Vec<String>>,
    pub risks: Option<Vec<String>>,
    pub talking_points: Option<Vec<String>>,
    pub recent_wins: Option<Vec<String>>,
    pub recent_win_sources: Option<Vec<JsonReference>>,
    pub open_items: Option<Vec<JsonActionItem>>,
    pub questions: Option<Vec<String>>,
    pub key_principles: Option<Vec<String>>,
    pub references: Option<Vec<JsonReference>>,
    pub proposed_agenda: Option<Vec<JsonAgendaItem>>,
    /// Calendar event description from Google Calendar (I185)
    pub calendar_notes: Option<String>,
    /// Intelligence-enriched account snapshot (I186)
    pub account_snapshot: Option<Vec<crate::types::AccountSnapshotItem>>,
    /// User-authored agenda items (I194)
    pub user_agenda: Option<Vec<String>>,
    /// User-authored notes (I194)
    pub user_notes: Option<String>,
    /// Intelligence summary — executive assessment from intelligence.json (I135)
    pub intelligence_summary: Option<String>,
    /// Entity-level risks from intelligence.json (I135)
    pub entity_risks: Option<Vec<crate::entity_intel::IntelRisk>>,
    /// Entity meeting readiness items from intelligence.json (I135)
    pub entity_readiness: Option<Vec<String>>,
    /// Stakeholder insights from intelligence.json (I135)
    pub stakeholder_insights: Option<Vec<crate::entity_intel::StakeholderInsight>>,
    /// Recent email-derived signals linked to this entity (I215)
    pub recent_email_signals: Option<Vec<crate::db::DbEmailSignal>>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonProgram {
    pub name: String,
    pub status: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonActionItem {
    pub title: String,
    pub due_date: Option<String>,
    pub context: Option<String>,
    #[serde(default)]
    pub is_overdue: bool,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonAgendaItem {
    pub topic: String,
    pub why: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonReference {
    pub label: String,
    pub path: Option<String>,
    pub last_updated: Option<String>,
}

fn sanitize_inline_text(value: &str) -> String {
    value
        .replace("**", "")
        .replace("__", "")
        .replace(['`', '*'], "")
        .replace('_', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn split_inline_source_tail(value: &str) -> (String, Option<String>) {
    let raw = value.trim();
    let re = regex::Regex::new(r"(?i)(?:^|\s)[_*]*\(?\s*source:\s*([^)]+?)\s*\)?[_*\s]*$")
        .expect("valid inline source regex");
    if let Some(caps) = re.captures(raw) {
        if let Some(m) = caps.get(0) {
            let cleaned = raw[..m.start()].trim().to_string();
            let source = caps.get(1).map(|s| sanitize_inline_text(s.as_str()));
            return (
                cleaned,
                source.and_then(|s| if s.is_empty() { None } else { Some(s) }),
            );
        }
    }
    (raw.to_string(), None)
}

fn clean_recent_win(value: &str) -> Option<String> {
    let (without_source, _) = split_inline_source_tail(value);
    let cleaned = sanitize_inline_text(&without_source)
        .replace("Recent win:", "")
        .replace("recent win:", "")
        .replace("Win:", "")
        .replace("win:", "")
        .trim()
        .to_string();
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

fn derive_recent_wins_from_talking_points(
    talking_points: &[String],
) -> (Vec<String>, Vec<crate::types::SourceReference>) {
    let mut wins = Vec::new();
    let mut sources = Vec::new();
    let mut seen_wins: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut seen_sources: std::collections::HashSet<String> = std::collections::HashSet::new();

    for point in talking_points {
        let (_, source_opt) = split_inline_source_tail(point);
        if let Some(source) = source_opt {
            let source_key = source.to_lowercase();
            if !seen_sources.contains(&source_key) {
                seen_sources.insert(source_key);
                let filename = source
                    .split(['/', '\\'])
                    .rfind(|s| !s.is_empty())
                    .unwrap_or(&source)
                    .to_string();
                sources.push(crate::types::SourceReference {
                    label: filename,
                    path: Some(source),
                    last_updated: None,
                });
            }
        }

        if let Some(cleaned) = clean_recent_win(point) {
            let key = cleaned.to_lowercase();
            if !seen_wins.contains(&key) {
                seen_wins.insert(key);
                wins.push(cleaned);
            }
        }
    }

    (wins, sources)
}

/// Load meeting prep from JSON
pub fn load_prep_json(today_dir: &Path, prep_file: &str) -> Result<FullMeetingPrep, String> {
    // prep_file is like "preps/0900-acme-sync.json" or just the filename
    let prep_path = if prep_file.starts_with("preps/") {
        today_dir.join("data").join(prep_file)
    } else {
        today_dir.join("data").join("preps").join(format!(
            "{}.json",
            prep_file.trim_end_matches(".json").trim_end_matches(".md")
        ))
    };

    let content =
        fs::read_to_string(&prep_path).map_err(|e| format!("Failed to read prep: {}", e))?;
    let data: JsonPrep =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse prep: {}", e))?;

    // Convert quick_context HashMap to Vec<(String, String)>
    let quick_context = data
        .quick_context
        .map(|qc| qc.into_iter().collect::<Vec<_>>());

    // Convert strategic_programs to strings with status markers
    let strategic_programs = data.strategic_programs.map(|programs| {
        programs
            .into_iter()
            .map(|p| {
                if p.status == "completed" {
                    format!("✓ {}", p.name)
                } else {
                    p.name
                }
            })
            .collect()
    });

    let attendees = data.attendees.map(|att| {
        att.into_iter()
            .map(|a| crate::types::Stakeholder {
                name: a.name,
                role: a.role,
                focus: a.focus,
            })
            .collect()
    });

    let open_items = data.open_items.map(|items| {
        items
            .into_iter()
            .map(|i| crate::types::ActionWithContext {
                title: i.title,
                due_date: i.due_date,
                context: i.context,
                is_overdue: i.is_overdue,
            })
            .collect()
    });

    let references = data.references.map(|refs| {
        refs.into_iter()
            .map(|r| crate::types::SourceReference {
                label: r.label,
                path: r.path,
                last_updated: r.last_updated,
            })
            .collect()
    });

    let recent_win_sources = data.recent_win_sources.map(|refs| {
        refs.into_iter()
            .map(|r| crate::types::SourceReference {
                label: sanitize_inline_text(&r.label),
                path: r.path.map(|p| sanitize_inline_text(&p)),
                last_updated: r.last_updated,
            })
            .filter(|r| !r.label.is_empty())
            .collect::<Vec<_>>()
    });

    let (derived_recent_wins, derived_recent_sources) = data
        .talking_points
        .as_ref()
        .map(|points| derive_recent_wins_from_talking_points(points))
        .unwrap_or_else(|| (Vec::new(), Vec::new()));

    let explicit_recent_wins = data.recent_wins.map(|wins| {
        wins.into_iter()
            .filter_map(|w| clean_recent_win(&w))
            .collect::<Vec<_>>()
    });

    let recent_wins = explicit_recent_wins
        .and_then(|wins| if wins.is_empty() { None } else { Some(wins) })
        .or({
            if derived_recent_wins.is_empty() {
                None
            } else {
                Some(derived_recent_wins)
            }
        });
    let recent_win_sources = recent_win_sources
        .and_then(|refs| if refs.is_empty() { None } else { Some(refs) })
        .or({
            if derived_recent_sources.is_empty() {
                None
            } else {
                Some(derived_recent_sources)
            }
        });

    let proposed_agenda = data.proposed_agenda.map(|items| {
        items
            .into_iter()
            .map(|a| crate::types::AgendaItem {
                topic: a.topic,
                why: a.why,
                source: a.source,
            })
            .collect()
    });

    Ok(FullMeetingPrep {
        file_path: prep_path.to_string_lossy().to_string(),
        calendar_event_id: data.calendar_event_id,
        title: data.title,
        time_range: data.time_range.unwrap_or_default(),
        meeting_context: data.meeting_context,
        calendar_notes: data.calendar_notes,
        account_snapshot: data.account_snapshot,
        quick_context,
        user_agenda: data.user_agenda,
        user_notes: data.user_notes,
        attendees,
        since_last: data.since_last,
        strategic_programs,
        current_state: data.current_state,
        open_items,
        risks: data.risks,
        talking_points: data.talking_points,
        recent_wins,
        recent_win_sources,
        questions: data.questions,
        key_principles: data.key_principles,
        references,
        raw_markdown: None,
        stakeholder_signals: None,
        attendee_context: None,
        proposed_agenda,
        intelligence_summary: data.intelligence_summary,
        entity_risks: data.entity_risks,
        entity_readiness: data.entity_readiness,
        stakeholder_insights: data.stakeholder_insights,
        recent_email_signals: data.recent_email_signals,
    })
}

// =============================================================================
// Directive Loading (ADR-0042: per-operation pipelines)
// =============================================================================

/// The today-directive.json produced by Phase 1 (prepare_today.py).
///
/// Uses serde defaults throughout so missing keys don't cause parse failures.
/// The Rust delivery functions read what they need; unknown fields are ignored.
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct Directive {
    #[serde(default)]
    pub context: DirectiveContext,
    #[serde(default)]
    pub calendar: DirectiveCalendar,
    #[serde(default)]
    pub meetings: std::collections::HashMap<String, Vec<DirectiveMeeting>>,
    #[serde(default)]
    pub meeting_contexts: Vec<DirectiveMeetingContext>,
    #[serde(default)]
    pub actions: DirectiveActions,
    #[serde(default)]
    pub emails: DirectiveEmails,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct DirectiveContext {
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub profile: Option<String>,
    #[serde(default)]
    pub greeting: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub focus: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct DirectiveCalendar {
    #[serde(default)]
    pub events: Vec<DirectiveEvent>,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct DirectiveEvent {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub start: Option<String>,
    #[serde(default)]
    pub end: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct DirectiveMeeting {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub event_id: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub start: Option<String>,
    #[serde(default)]
    pub end: Option<String>,
    #[serde(default)]
    pub account: Option<String>,
    #[serde(default)]
    pub start_display: Option<String>,
    #[serde(default)]
    pub end_display: Option<String>,
    #[serde(rename = "type", default)]
    pub meeting_type: Option<String>,
    /// Resolved entities from I336 entity-generic classification.
    #[serde(default)]
    pub entities: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct DirectiveMeetingContext {
    #[serde(default)]
    pub event_id: Option<String>,
    #[serde(default)]
    pub account: Option<String>,
    #[serde(default)]
    pub account_data: Option<serde_json::Value>,
    /// I337: Resolved entity ID (account, project, or person).
    #[serde(default)]
    pub entity_id: Option<String>,
    /// I337: Resolved entity type ("account", "project", "person").
    #[serde(default)]
    pub entity_type: Option<String>,
    /// I337: Structured primary entity data.
    #[serde(default)]
    pub primary_entity: Option<serde_json::Value>,
    /// I337: Project-specific data when entity is a project.
    #[serde(default)]
    pub project_data: Option<serde_json::Value>,
    /// I337: Person-specific data when entity is a person.
    #[serde(default)]
    pub person_data: Option<serde_json::Value>,
    /// I337: Relationship signals when entity is a person.
    #[serde(default)]
    pub relationship_signals: Option<serde_json::Value>,
    /// I337: Shared entities (accounts/projects) when entity is a person.
    #[serde(default)]
    pub shared_entities: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub attendees: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub narrative: Option<String>,
    #[serde(default)]
    pub talking_points: Option<Vec<String>>,
    #[serde(default)]
    pub risks: Option<Vec<String>>,
    #[serde(default)]
    pub wins: Option<Vec<String>>,
    #[serde(default)]
    pub questions: Option<Vec<String>>,
    #[serde(default)]
    pub key_principles: Option<Vec<String>>,
    #[serde(default)]
    pub since_last: Option<Vec<String>>,
    #[serde(default)]
    pub current_state: Option<Vec<String>>,
    #[serde(default)]
    pub strategic_programs: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub open_items: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub references: Option<Vec<serde_json::Value>>,
    // Raw data from meeting_context.rs (SQLite queries) — used to synthesize prep content
    #[serde(default)]
    pub open_actions: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub recent_captures: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub meeting_history: Option<Vec<serde_json::Value>>,
    // I135: Entity intelligence (from intelligence.json) — persistent prep context
    #[serde(default)]
    pub executive_assessment: Option<String>,
    #[serde(default)]
    pub entity_risks: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub entity_readiness: Option<Vec<String>>,
    #[serde(default)]
    pub stakeholder_insights: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub recent_email_signals: Option<Vec<serde_json::Value>>,
    /// Calendar event description (I185).
    #[serde(default)]
    pub description: Option<String>,
    /// I317: Pre-meeting email context gathered from email signals/bridge.
    #[serde(default)]
    pub pre_meeting_email_context: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct DirectiveActions {
    #[serde(default)]
    pub overdue: Vec<DirectiveAction>,
    #[serde(default)]
    pub due_today: Vec<DirectiveAction>,
    #[serde(default)]
    pub due_this_week: Vec<DirectiveAction>,
    #[serde(default)]
    pub waiting_on: Vec<DirectiveWaiting>,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct DirectiveAction {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub account: Option<String>,
    #[serde(default)]
    pub due_date: Option<String>,
    #[serde(default, alias = "due")]
    pub due: Option<String>,
    #[serde(default)]
    pub days_overdue: Option<u32>,
    #[serde(default)]
    pub context: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
}

impl DirectiveAction {
    /// Get the due date, trying due_date first then due
    pub fn effective_due_date(&self) -> Option<&str> {
        self.due_date.as_deref().or(self.due.as_deref())
    }
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct DirectiveWaiting {
    #[serde(default)]
    pub what: Option<String>,
    #[serde(default)]
    pub who: Option<String>,
    #[serde(default)]
    pub context: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct DirectiveEmails {
    #[serde(default)]
    pub classified: Vec<DirectiveEmail>,
    #[serde(default)]
    pub high_priority: Vec<DirectiveEmail>,
    #[serde(default)]
    pub medium_count: u32,
    #[serde(default)]
    pub low_count: u32,
    #[serde(default, alias = "syncError")]
    pub sync_error: Option<DirectiveEmailSyncError>,
    /// AI-synthesized email narrative (I322)
    #[serde(default)]
    pub narrative: Option<String>,
    /// Threads awaiting user reply (I318/I355)
    #[serde(default, alias = "repliesNeeded")]
    pub replies_needed: Vec<DirectiveReplyNeeded>,
}

/// A thread awaiting the user's reply (I318 — "ball in your court").
#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectiveReplyNeeded {
    #[serde(default)]
    pub thread_id: String,
    #[serde(default)]
    pub subject: String,
    #[serde(default)]
    pub from: String,
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub wait_duration: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct DirectiveEmail {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub from_email: Option<String>,
    #[serde(default)]
    pub subject: Option<String>,
    #[serde(default)]
    pub snippet: Option<String>,
    #[serde(default)]
    pub priority: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectiveEmailSyncError {
    #[serde(default)]
    pub stage: Option<String>,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
}

/// Load the today-directive.json produced by Phase 1.
///
/// Checks `_today/data/today-directive.json` first, then falls back to
/// `_today/.today-directive.json` (legacy location).
pub fn load_directive(today_dir: &Path) -> Result<Directive, String> {
    let primary = today_dir.join("data").join("today-directive.json");
    let legacy = today_dir.join(".today-directive.json");

    let path = if primary.exists() {
        &primary
    } else if legacy.exists() {
        &legacy
    } else {
        return Err(format!(
            "Directive not found. Checked:\n  {}\n  {}",
            primary.display(),
            legacy.display()
        ));
    };

    let content =
        fs::read_to_string(path).map_err(|e| format!("Failed to read directive: {}", e))?;
    serde_json::from_str(&content).map_err(|e| format!("Failed to parse directive: {}", e))
}

// =============================================================================
// Week JSON Loading (Phase 3C)
// =============================================================================

/// Load week overview from JSON
pub fn load_week_json(today_dir: &Path) -> Result<WeekOverview, String> {
    let week_path = today_dir.join("data").join("week-overview.json");
    let content = fs::read_to_string(&week_path)
        .map_err(|e| format!("Failed to read week overview: {}", e))?;
    serde_json::from_str(&content).map_err(|e| format!("Failed to parse week overview: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn test_load_prep_json_derives_recent_wins_from_legacy_talking_points() {
        let dir = tempdir().expect("tempdir");
        let today_dir = dir.path();
        let preps_dir = today_dir.join("data").join("preps");
        fs::create_dir_all(&preps_dir).expect("create preps dir");

        let prep_path = preps_dir.join("legacy.json");
        fs::write(
            &prep_path,
            serde_json::to_string_pretty(&json!({
                "meetingId": "legacy",
                "title": "Legacy Prep",
                "type": "customer",
                "talkingPoints": [
                    "Recent win: Expansion signal from sponsor (source: notes.md)"
                ]
            }))
            .unwrap(),
        )
        .expect("write prep");

        let prep = load_prep_json(today_dir, "legacy").expect("load prep");
        assert_eq!(
            prep.recent_wins,
            Some(vec!["Expansion signal from sponsor".to_string()])
        );
        assert_eq!(
            prep.recent_win_sources
                .as_ref()
                .and_then(|s| s.first())
                .map(|s| s.label.clone()),
            Some("notes.md".to_string())
        );
    }

    #[test]
    fn test_load_prep_json_reads_structured_recent_wins() {
        let dir = tempdir().expect("tempdir");
        let today_dir = dir.path();
        let preps_dir = today_dir.join("data").join("preps");
        fs::create_dir_all(&preps_dir).expect("create preps dir");

        let prep_path = preps_dir.join("new-format.json");
        fs::write(
            &prep_path,
            serde_json::to_string_pretty(&json!({
                "meetingId": "new-format",
                "title": "New Prep",
                "type": "customer",
                "recentWins": ["Tier upgrade approved"],
                "recentWinSources": [{"label": "sync.md", "path": "notes/sync.md"}]
            }))
            .unwrap(),
        )
        .expect("write prep");

        let prep = load_prep_json(today_dir, "new-format").expect("load prep");
        assert_eq!(
            prep.recent_wins,
            Some(vec!["Tier upgrade approved".to_string()])
        );
        assert_eq!(
            prep.recent_win_sources
                .as_ref()
                .and_then(|s| s.first())
                .and_then(|s| s.path.clone()),
            Some("notes/sync.md".to_string())
        );
    }

    #[test]
    fn test_load_emails_json_with_sync_supports_legacy_without_sync() {
        let dir = tempdir().expect("tempdir");
        let today_dir = dir.path();
        let data_dir = today_dir.join("data");
        fs::create_dir_all(&data_dir).expect("create data dir");

        fs::write(
            data_dir.join("emails.json"),
            serde_json::to_string_pretty(&json!({
                "highPriority": [],
                "mediumPriority": [],
                "lowPriority": [],
                "stats": { "highCount": 0, "mediumCount": 0, "lowCount": 0, "total": 0 }
            }))
            .unwrap(),
        )
        .expect("write emails");

        let loaded = load_emails_json_with_sync(today_dir).expect("load emails");
        assert!(loaded.emails.is_empty());
        assert!(loaded.sync.is_none());
    }

    #[test]
    fn test_load_emails_json_with_sync_reads_structured_status() {
        let dir = tempdir().expect("tempdir");
        let today_dir = dir.path();
        let data_dir = today_dir.join("data");
        fs::create_dir_all(&data_dir).expect("create data dir");

        fs::write(
            data_dir.join("emails.json"),
            serde_json::to_string_pretty(&json!({
                "highPriority": [],
                "mediumPriority": [],
                "lowPriority": [],
                "stats": { "highCount": 0, "mediumCount": 0, "lowCount": 0, "total": 0 },
                "sync": {
                    "state": "error",
                    "stage": "fetch",
                    "code": "gmail_auth_failed",
                    "message": "Auth failed",
                    "usingLastKnownGood": true
                }
            }))
            .unwrap(),
        )
        .expect("write emails");

        let loaded = load_emails_json_with_sync(today_dir).expect("load emails");
        let sync = loaded.sync.expect("sync should be present");
        assert_eq!(sync.code.as_deref(), Some("gmail_auth_failed"));
        assert_eq!(sync.using_last_known_good, Some(true));
    }
}
