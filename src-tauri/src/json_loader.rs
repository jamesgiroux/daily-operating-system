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
    Action, DayOverview, Email, FullMeetingPrep, Meeting, MeetingPrep,
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

/// Load manifest from data directory
pub fn load_manifest(today_dir: &Path) -> Result<Manifest, String> {
    let manifest_path = today_dir.join("data").join("manifest.json");
    let content = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read manifest: {}", e))?;
    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse manifest: {}", e))
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
    pub time: String,
    pub end_time: Option<String>,
    pub title: String,
    #[serde(rename = "type")]
    pub meeting_type: String,
    pub account: Option<String>,
    #[serde(default)]
    pub is_current: bool,
    pub has_prep: bool,
    pub prep_file: Option<String>,
    pub prep_summary: Option<JsonPrepSummary>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonPrepSummary {
    pub at_a_glance: Option<Vec<String>>,
    pub discuss: Option<Vec<String>>,
    pub watch: Option<Vec<String>>,
    pub wins: Option<Vec<String>>,
}

/// Load schedule from JSON
pub fn load_schedule_json(today_dir: &Path) -> Result<(DayOverview, Vec<Meeting>), String> {
    let schedule_path = today_dir.join("data").join("schedule.json");
    let content = fs::read_to_string(&schedule_path)
        .map_err(|e| format!("Failed to read schedule: {}", e))?;
    let schedule: JsonSchedule = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse schedule: {}", e))?;

    let overview = DayOverview {
        greeting: schedule.greeting.unwrap_or_else(|| "Good morning".to_string()),
        date: schedule.date,
        summary: schedule.summary.unwrap_or_default(),
        focus: schedule.focus,
    };

    let meetings: Vec<Meeting> = schedule.meetings.into_iter().map(|m| {
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
            context: None,
            stakeholders: None,
            questions: None,
            open_items: None,
            historical_context: None,
            source_references: None,
        });

        Meeting {
            id: m.id,
            time: m.time,
            end_time: m.end_time,
            title: m.title,
            meeting_type,
            account: m.account,
            prep,
            is_current: if m.is_current { Some(true) } else { None },
            prep_file: m.prep_file,
            has_prep: m.has_prep,
        }
    }).collect();

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
    let content = fs::read_to_string(&actions_path)
        .map_err(|e| format!("Failed to read actions: {}", e))?;
    let data: JsonActions = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse actions: {}", e))?;

    let actions = data.actions.into_iter().map(|a| {
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
    }).collect();

    Ok(actions)
}

/// JSON emails format
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonEmails {
    pub date: String,
    pub stats: Option<JsonEmailStats>,
    pub emails: Vec<JsonEmail>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonEmailStats {
    pub high_priority: Option<u32>,
    pub normal_priority: Option<u32>,
    pub needs_action: Option<u32>,
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
}

/// Load emails from JSON
pub fn load_emails_json(today_dir: &Path) -> Result<Vec<Email>, String> {
    let emails_path = today_dir.join("data").join("emails.json");
    let content = fs::read_to_string(&emails_path)
        .map_err(|e| format!("Failed to read emails: {}", e))?;
    let data: JsonEmails = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse emails: {}", e))?;

    let emails = data.emails.into_iter().map(|e| {
        let priority = match e.priority.as_str() {
            "high" => crate::types::EmailPriority::High,
            _ => crate::types::EmailPriority::Normal,
        };

        Email {
            id: e.id,
            sender: e.sender,
            sender_email: e.sender_email.unwrap_or_default(),
            subject: e.subject,
            snippet: e.snippet,
            priority,
            avatar_url: None,
        }
    }).collect();

    Ok(emails)
}

/// JSON prep format
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonPrep {
    pub meeting_id: String,
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
    pub open_items: Option<Vec<JsonActionItem>>,
    pub questions: Option<Vec<String>>,
    pub key_principles: Option<Vec<String>>,
    pub references: Option<Vec<JsonReference>>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonStakeholder {
    pub name: String,
    pub role: Option<String>,
    pub focus: Option<String>,
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
pub struct JsonReference {
    pub label: String,
    pub path: Option<String>,
    pub last_updated: Option<String>,
}

/// Load meeting prep from JSON
pub fn load_prep_json(today_dir: &Path, prep_file: &str) -> Result<FullMeetingPrep, String> {
    // prep_file is like "preps/0900-acme-sync.json" or just the filename
    let prep_path = if prep_file.starts_with("preps/") {
        today_dir.join("data").join(prep_file)
    } else {
        today_dir.join("data").join("preps").join(format!("{}.json", prep_file.trim_end_matches(".json").trim_end_matches(".md")))
    };

    let content = fs::read_to_string(&prep_path)
        .map_err(|e| format!("Failed to read prep: {}", e))?;
    let data: JsonPrep = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse prep: {}", e))?;

    // Convert quick_context HashMap to Vec<(String, String)>
    let quick_context = data.quick_context.map(|qc| {
        qc.into_iter().collect::<Vec<_>>()
    });

    // Convert strategic_programs to strings with status markers
    let strategic_programs = data.strategic_programs.map(|programs| {
        programs.into_iter().map(|p| {
            if p.status == "completed" {
                format!("✓ {}", p.name)
            } else {
                p.name
            }
        }).collect()
    });

    let attendees = data.attendees.map(|att| {
        att.into_iter().map(|a| crate::types::Stakeholder {
            name: a.name,
            role: a.role,
            focus: a.focus,
        }).collect()
    });

    let open_items = data.open_items.map(|items| {
        items.into_iter().map(|i| crate::types::ActionWithContext {
            title: i.title,
            due_date: i.due_date,
            context: i.context,
            is_overdue: i.is_overdue,
        }).collect()
    });

    let references = data.references.map(|refs| {
        refs.into_iter().map(|r| crate::types::SourceReference {
            label: r.label,
            path: r.path,
            last_updated: r.last_updated,
        }).collect()
    });

    Ok(FullMeetingPrep {
        file_path: prep_path.to_string_lossy().to_string(),
        title: data.title,
        time_range: data.time_range.unwrap_or_default(),
        meeting_context: data.meeting_context,
        quick_context,
        attendees,
        since_last: data.since_last,
        strategic_programs,
        current_state: data.current_state,
        open_items,
        risks: data.risks,
        talking_points: data.talking_points,
        questions: data.questions,
        key_principles: data.key_principles,
        references,
        raw_markdown: None,
    })
}
