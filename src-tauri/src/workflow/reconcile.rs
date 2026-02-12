//! End-of-day reconciliation (ADR-0040)
//!
//! Deterministic checks that run before the archive workflow:
//! - Identify completed meetings from schedule.json
//! - Check transcript processing status for each
//! - Compute action stats for the day
//! - Produce data for day-summary.json and next-morning-flags.json

use std::fs;
use std::path::Path;

use chrono::{Local, NaiveTime, Utc};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use rusqlite::params;

use crate::db::{ActionDb, DbMeeting};
use crate::json_loader::{JsonMeeting, JsonSchedule};

/// Result of running end-of-day reconciliation
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconciliationResult {
    pub date: String,
    pub reconciled_at: String,
    pub meetings: MeetingReconciliation,
    pub actions: ActionStats,
    pub flags: Vec<MorningFlag>,
}

/// Meeting reconciliation summary
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingReconciliation {
    pub completed: usize,
    pub details: Vec<MeetingStatus>,
}

/// Status of a single completed meeting
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingStatus {
    pub title: String,
    pub meeting_type: String,
    pub time: String,
    pub end_time: Option<String>,
    pub account: Option<String>,
    pub calendar_event_id: Option<String>,
    pub transcript_status: TranscriptStatus,
}

/// Transcript processing status
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptStatus {
    /// Transcript found in canonical location (Accounts/*/02-Meetings/)
    Processed,
    /// Transcript found in _inbox/ but not yet processed
    InInbox,
    /// No transcript found (meeting may not have been recorded)
    NoTranscript,
}

/// Action stats for the day
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionStats {
    pub completed_today: usize,
    pub pending: usize,
}

/// Flag for tomorrow's briefing
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MorningFlag {
    pub flag_type: FlagType,
    pub title: String,
    pub detail: String,
}

/// Types of morning flags
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FlagType {
    MissingTranscript,
    UnprocessedInbox,
}

/// Run end-of-day reconciliation.
///
/// Reads schedule.json (must be called BEFORE archive cleans data/),
/// checks transcript status, computes action stats from SQLite.
/// Returns structured data for summary and flag files.
pub fn run_reconciliation(workspace: &Path, db: Option<&ActionDb>) -> ReconciliationResult {
    let today_dir = workspace.join("_today");
    let today_str = Local::now().format("%Y-%m-%d").to_string();
    let now = Local::now().time();

    // 1. Read schedule.json for today's meetings
    let meetings = read_completed_meetings(&today_dir, now);

    // 2. Check transcript status for each completed meeting
    let details: Vec<MeetingStatus> = meetings
        .into_iter()
        .map(|m| {
            let transcript_status = check_transcript_status(workspace, &m, &today_str);
            MeetingStatus {
                title: m.title,
                meeting_type: m.meeting_type,
                time: m.time,
                end_time: m.end_time,
                account: m.account,
                calendar_event_id: m.calendar_event_id,
                transcript_status,
            }
        })
        .collect();

    // 3. Build morning flags from gaps
    let mut flags = Vec::new();
    for ms in &details {
        // Only flag customer/external meetings with missing transcripts
        let is_trackable = matches!(
            ms.meeting_type.as_str(),
            "customer" | "qbr" | "partnership" | "external"
        );
        if is_trackable {
            match &ms.transcript_status {
                TranscriptStatus::InInbox => {
                    flags.push(MorningFlag {
                        flag_type: FlagType::UnprocessedInbox,
                        title: ms.title.clone(),
                        detail: format!(
                            "Transcript for \"{}\" is in _inbox/ but hasn't been processed",
                            ms.title
                        ),
                    });
                }
                TranscriptStatus::NoTranscript => {
                    flags.push(MorningFlag {
                        flag_type: FlagType::MissingTranscript,
                        title: ms.title.clone(),
                        detail: format!(
                            "No transcript found for \"{}\" — check if recording was saved",
                            ms.title
                        ),
                    });
                }
                TranscriptStatus::Processed => {}
            }
        }
    }

    // 4. Check for other unprocessed inbox files
    let inbox_dir = workspace.join("_inbox");
    if inbox_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&inbox_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        // Skip hidden files and already-flagged transcripts
                        if !name.starts_with('.') {
                            let already_flagged = flags
                                .iter()
                                .any(|f| matches!(f.flag_type, FlagType::UnprocessedInbox));
                            if !already_flagged {
                                flags.push(MorningFlag {
                                    flag_type: FlagType::UnprocessedInbox,
                                    title: name.to_string(),
                                    detail: format!("Unprocessed file in _inbox/: {}", name),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    // 5. Compute action stats from SQLite
    let actions = if let Some(db) = db {
        get_action_stats(db, &today_str)
    } else {
        ActionStats {
            completed_today: 0,
            pending: 0,
        }
    };

    let completed = details.len();

    ReconciliationResult {
        date: today_str,
        reconciled_at: Utc::now().to_rfc3339(),
        meetings: MeetingReconciliation { completed, details },
        actions,
        flags,
    }
}

/// Record completed meetings in SQLite meetings_history.
///
/// Also persists enriched prep context (I181) so prep data survives archival.
pub fn persist_meetings(db: &ActionDb, result: &ReconciliationResult, workspace: &Path) {
    let preps_dir = workspace.join("_today").join("data").join("preps");

    for ms in &result.meetings.details {
        let meeting_id = ms
            .calendar_event_id
            .as_deref()
            .map(sanitize_calendar_event_id)
            .unwrap_or_else(|| format!("archive-{}-{}", result.date, slug(&ms.title)));

        let prep_source =
            read_prep_source(&preps_dir, &meeting_id, ms.calendar_event_id.as_deref());
        let prep_context_json = prep_source
            .as_ref()
            .and_then(|v| serde_json::to_string(v).ok())
            .filter(|raw| is_prep_substantive(raw));
        let prep_user_agenda = prep_source.as_ref().and_then(extract_user_agenda_json);
        let prep_user_notes = prep_source.as_ref().and_then(extract_user_notes);

        let existing = db
            .get_meeting_by_id(&meeting_id)
            .ok()
            .flatten()
            .or_else(|| {
                ms.calendar_event_id
                    .as_deref()
                    .and_then(|eid| db.get_meeting_by_calendar_event_id(eid).ok().flatten())
            });

        let meeting = DbMeeting {
            id: meeting_id,
            title: ms.title.clone(),
            meeting_type: ms.meeting_type.clone(),
            start_time: format!("{} {}", result.date, ms.time),
            end_time: ms
                .end_time
                .as_ref()
                .map(|t| format!("{} {}", result.date, t)),
            account_id: ms.account.clone(),
            attendees: None,
            notes_path: match &ms.transcript_status {
                TranscriptStatus::Processed => Some("processed".to_string()),
                TranscriptStatus::InInbox => Some("in_inbox".to_string()),
                TranscriptStatus::NoTranscript => None,
            },
            summary: None,
            created_at: Utc::now().to_rfc3339(),
            calendar_event_id: ms.calendar_event_id.clone(),
            description: None,
            prep_context_json,
            user_agenda_json: existing
                .as_ref()
                .and_then(|m| m.user_agenda_json.clone())
                .or(prep_user_agenda),
            user_notes: existing
                .as_ref()
                .and_then(|m| m.user_notes.clone())
                .or(prep_user_notes),
            prep_frozen_json: None,
            prep_frozen_at: None,
            prep_snapshot_path: None,
            prep_snapshot_hash: None,
            transcript_path: None,
            transcript_processed_at: None,
        };

        if let Err(e) = db.upsert_meeting(&meeting) {
            log::warn!("Failed to persist meeting '{}': {}", ms.title, e);
            continue;
        }
        if let Err(e) = freeze_meeting_snapshot(db, workspace, &meeting, prep_source) {
            log::warn!("Failed to freeze meeting snapshot '{}': {}", ms.title, e);
        }
    }
}

fn sanitize_calendar_event_id(calendar_event_id: &str) -> String {
    calendar_event_id.replace('@', "_at_")
}

fn read_prep_source(
    preps_dir: &Path,
    meeting_id: &str,
    calendar_event_id: Option<&str>,
) -> Option<Value> {
    let direct_path = preps_dir.join(format!("{}.json", meeting_id));
    if direct_path.exists() {
        return read_prep_value(&direct_path);
    }
    let raw_calendar_id = calendar_event_id.unwrap_or_default();
    let normalized_calendar_id = sanitize_calendar_event_id(raw_calendar_id);
    let entries = fs::read_dir(preps_dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
        {
            continue;
        }
        let Some(value) = read_prep_value(&path) else {
            continue;
        };
        let event_id = value
            .get("calendarEventId")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        if event_id == meeting_id
            || event_id == raw_calendar_id
            || event_id == normalized_calendar_id
        {
            return Some(value);
        }
    }
    None
}

fn read_prep_value(path: &Path) -> Option<Value> {
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str::<Value>(&content).ok()
}

fn extract_user_agenda_json(prep: &Value) -> Option<String> {
    let agenda = prep.get("userAgenda")?.as_array()?;
    let items: Vec<String> = agenda
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.trim().to_string()))
        .filter(|s| !s.is_empty())
        .collect();
    if items.is_empty() {
        None
    } else {
        serde_json::to_string(&items).ok()
    }
}

fn extract_user_notes(prep: &Value) -> Option<String> {
    prep.get("userNotes")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn freeze_meeting_snapshot(
    db: &ActionDb,
    workspace: &Path,
    meeting: &DbMeeting,
    prep_source: Option<Value>,
) -> Result<(), String> {
    let persisted = db
        .get_meeting_by_id(&meeting.id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Meeting missing after upsert: {}", meeting.id))?;
    if persisted.prep_frozen_at.is_some() {
        return Ok(());
    }

    let mut prep = prep_source
        .or_else(|| {
            persisted
                .prep_context_json
                .as_deref()
                .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
        })
        .unwrap_or_else(|| serde_json::json!({}));
    let user_agenda = persisted
        .user_agenda_json
        .as_deref()
        .and_then(|raw| serde_json::from_str::<Vec<String>>(raw).ok());
    let user_notes = persisted.user_notes.clone();

    if prep.as_object().is_none() && user_agenda.is_none() && user_notes.is_none() {
        return Ok(());
    }
    if let Some(ref agenda) = user_agenda {
        prep["userAgenda"] = serde_json::json!(agenda);
    }
    if let Some(ref notes) = user_notes {
        prep["userNotes"] = serde_json::json!(notes);
    }

    let frozen_at = Utc::now().to_rfc3339();
    let account_name = resolve_account_name(db, persisted.account_id.as_deref());
    let base_snapshot = serde_json::json!({
        "schemaVersion": 1,
        "meetingId": persisted.id,
        "calendarEventId": persisted.calendar_event_id,
        "title": persisted.title,
        "meetingType": persisted.meeting_type,
        "startTime": persisted.start_time,
        "endTime": persisted.end_time,
        "accountId": persisted.account_id,
        "accountName": account_name,
        "frozenAt": frozen_at,
        "prep": prep,
        "userAgenda": user_agenda,
        "userNotes": user_notes,
        "outcomesSummary": persisted.summary,
    });
    let hash = hex::encode(Sha256::digest(
        &serde_json::to_vec(&base_snapshot).map_err(|e| e.to_string())?,
    ));
    let mut snapshot = base_snapshot;
    snapshot["snapshotHash"] = serde_json::json!(hash.clone());

    let snapshot_path = resolve_snapshot_path(workspace, db, &persisted)?;
    if let Some(parent) = snapshot_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create {}: {}", parent.display(), e))?;
    }
    let snapshot_str = serde_json::to_string_pretty(&snapshot).map_err(|e| e.to_string())?;
    crate::util::atomic_write_str(&snapshot_path, &snapshot_str).map_err(|e| e.to_string())?;
    let _ = db
        .freeze_meeting_prep_snapshot(
            &persisted.id,
            &snapshot_str,
            &frozen_at,
            &snapshot_path.to_string_lossy(),
            &hash,
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn resolve_account_name(db: &ActionDb, account_id: Option<&str>) -> Option<String> {
    let Some(account_id) = account_id else {
        return None;
    };
    if let Ok(Some(account)) = db.get_account(account_id) {
        return Some(account.name);
    }
    db.get_account_by_name(account_id)
        .ok()
        .flatten()
        .map(|a| a.name)
}

fn resolve_snapshot_path(
    workspace: &Path,
    db: &ActionDb,
    meeting: &DbMeeting,
) -> Result<std::path::PathBuf, String> {
    let date_prefix = meeting
        .start_time
        .get(0..10)
        .unwrap_or("unknown-date")
        .to_string();
    let file_name = format!(
        "{}-{}-prep.snapshot.json",
        date_prefix,
        crate::util::sanitize_for_filesystem(&meeting.id)
    );

    if let Ok(linked) = db.get_meeting_entities(&meeting.id) {
        if let Some(entity) = linked.first() {
            match entity.entity_type {
                crate::entity::EntityType::Account => {
                    if let Ok(Some(account)) = db.get_account(&entity.id) {
                        let dir = crate::accounts::resolve_account_dir(workspace, &account)
                            .join("Meeting-Notes");
                        return Ok(dir.join(file_name));
                    }
                }
                crate::entity::EntityType::Project => {
                    if let Ok(Some(project)) = db.get_project(&entity.id) {
                        let dir = crate::projects::project_dir(workspace, &project.name)
                            .join("Meeting-Notes");
                        return Ok(dir.join(file_name));
                    }
                }
                _ => {}
            }
        }
    }

    let (year, month) = if date_prefix.len() >= 7 {
        (&date_prefix[0..4], &date_prefix[5..7])
    } else {
        ("unknown", "00")
    };
    Ok(workspace
        .join("_archive")
        .join("meetings")
        .join(year)
        .join(month)
        .join(file_name))
}

/// Check if a prep JSON has meaningful content worth persisting.
fn is_prep_substantive(json_str: &str) -> bool {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(json_str) else {
        return false;
    };
    [
        "intelligenceSummary",
        "entityRisks",
        "entityReadiness",
        "recentWins",
        "talkingPoints",
        "recentWinSources",
        "proposedAgenda",
        "openItems",
        "questions",
        "stakeholderInsights",
    ]
    .iter()
    .any(|key| {
        v.get(key).is_some_and(|val| {
            !val.is_null()
                && (val.as_str().map_or(true, |s| !s.is_empty())
                    || val.as_array().is_some_and(|a| !a.is_empty()))
        })
    })
}

/// Write day-summary.json to the archive directory.
pub fn write_day_summary(
    archive_path: &Path,
    result: &ReconciliationResult,
    files_archived: usize,
) -> Result<(), String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct DaySummary<'a> {
        date: &'a str,
        archived_at: &'a str,
        meetings: &'a MeetingReconciliation,
        actions: &'a ActionStats,
        files_archived: usize,
    }

    let summary = DaySummary {
        date: &result.date,
        archived_at: &result.reconciled_at,
        meetings: &result.meetings,
        actions: &result.actions,
        files_archived,
    };

    let json = serde_json::to_string_pretty(&summary)
        .map_err(|e| format!("Failed to serialize day summary: {}", e))?;

    fs::write(archive_path.join("day-summary.json"), json)
        .map_err(|e| format!("Failed to write day-summary.json: {}", e))
}

/// Write next-morning-flags.json to _today/data/ for tomorrow's briefing.
pub fn write_morning_flags(today_dir: &Path, result: &ReconciliationResult) -> Result<(), String> {
    if result.flags.is_empty() {
        return Ok(()); // No flags, no file
    }

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct FlagsFile<'a> {
        generated_at: &'a str,
        flags: &'a [MorningFlag],
    }

    let flags_file = FlagsFile {
        generated_at: &result.reconciled_at,
        flags: &result.flags,
    };

    let json = serde_json::to_string_pretty(&flags_file)
        .map_err(|e| format!("Failed to serialize morning flags: {}", e))?;

    let data_dir = today_dir.join("data");
    // Ensure data/ exists (may have been cleaned, or may not exist yet)
    let _ = fs::create_dir_all(&data_dir);

    fs::write(data_dir.join("next-morning-flags.json"), json)
        .map_err(|e| format!("Failed to write next-morning-flags.json: {}", e))
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Read schedule.json and filter to completed meetings (end_time in the past).
fn read_completed_meetings(today_dir: &Path, now: NaiveTime) -> Vec<JsonMeeting> {
    let schedule_path = today_dir.join("data").join("schedule.json");
    let content = match fs::read_to_string(&schedule_path) {
        Ok(c) => c,
        Err(_) => {
            log::info!("Reconciliation: no schedule.json found, skipping meeting reconciliation");
            return Vec::new();
        }
    };

    let schedule: JsonSchedule = match serde_json::from_str(&content) {
        Ok(s) => s,
        Err(e) => {
            log::warn!("Reconciliation: failed to parse schedule.json: {}", e);
            return Vec::new();
        }
    };

    schedule
        .meetings
        .into_iter()
        .filter(|m| is_meeting_completed(m, now))
        .collect()
}

/// Check if a meeting has completed (end_time is before now).
fn is_meeting_completed(meeting: &JsonMeeting, now: NaiveTime) -> bool {
    let end_str = match &meeting.end_time {
        Some(t) => t,
        None => return false, // No end time means we can't determine completion
    };

    // Parse time strings like "10:30 AM", "2:00 PM"
    parse_display_time(end_str)
        .map(|end| end <= now)
        .unwrap_or(false)
}

/// Parse a display time string like "9:00 AM" or "2:30 PM" to NaiveTime.
fn parse_display_time(s: &str) -> Option<NaiveTime> {
    // Try common formats
    for fmt in &["%-I:%M %p", "%I:%M %p", "%-I:%M%p", "%I:%M%p"] {
        if let Ok(t) = NaiveTime::parse_from_str(s.trim(), fmt) {
            return Some(t);
        }
    }
    None
}

/// Check transcript status for a completed meeting.
fn check_transcript_status(
    workspace: &Path,
    meeting: &JsonMeeting,
    date: &str,
) -> TranscriptStatus {
    let account = match &meeting.account {
        Some(a) => a,
        None => return TranscriptStatus::NoTranscript,
    };

    // 1. Check canonical location: Accounts/{account}/02-Meetings/{date}-*
    let meetings_dir = workspace.join("Accounts").join(account).join("02-Meetings");
    if meetings_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&meetings_dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.starts_with(date) {
                        return TranscriptStatus::Processed;
                    }
                }
            }
        }
    }

    // 2. Check _inbox/ for unprocessed transcript
    // Normalize account name: "Acme Corp" → matches both "acme-corp" and "acme corp" in filenames
    let inbox_dir = workspace.join("_inbox");
    if inbox_dir.is_dir() {
        let account_lower = account.to_lowercase();
        let account_slug = account_lower.replace(' ', "-");
        if let Ok(entries) = fs::read_dir(&inbox_dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    let name_lower = name.to_lowercase();
                    if name_lower.contains(date)
                        && (name_lower.contains(&account_lower)
                            || name_lower.contains(&account_slug))
                    {
                        return TranscriptStatus::InInbox;
                    }
                }
            }
        }
    }

    TranscriptStatus::NoTranscript
}

/// Get action stats from SQLite for today.
fn get_action_stats(db: &ActionDb, today: &str) -> ActionStats {
    // Count actions completed today
    let completed_today: usize = db
        .conn_ref()
        .query_row(
            "SELECT COUNT(*) FROM actions WHERE completed_at LIKE ?1 || '%'",
            params![today],
            |row| row.get(0),
        )
        .unwrap_or(0);

    // Count pending actions
    let pending: usize = db
        .conn_ref()
        .query_row(
            "SELECT COUNT(*) FROM actions WHERE status = 'pending'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    ActionStats {
        completed_today,
        pending,
    }
}

/// Create a URL-safe slug from a title.
fn slug(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;

    #[test]
    fn test_parse_display_time() {
        assert!(parse_display_time("9:00 AM").is_some());
        assert!(parse_display_time("2:30 PM").is_some());
        assert!(parse_display_time("12:00 PM").is_some());
        assert!(parse_display_time("11:45 AM").is_some());

        let t = parse_display_time("2:30 PM").unwrap();
        assert_eq!(t.hour(), 14);
        assert_eq!(t.minute(), 30);
    }

    #[test]
    fn test_is_meeting_completed() {
        let late_now = NaiveTime::from_hms_opt(23, 0, 0).unwrap();

        let meeting = JsonMeeting {
            id: "test".to_string(),
            calendar_event_id: None,
            time: "9:00 AM".to_string(),
            end_time: Some("10:00 AM".to_string()),
            start_iso: None,
            title: "Test Meeting".to_string(),
            meeting_type: "customer".to_string(),
            account: Some("Acme".to_string()),
            is_current: false,
            has_prep: false,
            prep_file: None,
            prep_summary: None,
        };

        assert!(is_meeting_completed(&meeting, late_now));

        let early_now = NaiveTime::from_hms_opt(8, 0, 0).unwrap();
        assert!(!is_meeting_completed(&meeting, early_now));
    }

    #[test]
    fn test_is_meeting_no_end_time() {
        let now = NaiveTime::from_hms_opt(23, 0, 0).unwrap();

        let meeting = JsonMeeting {
            id: "test".to_string(),
            calendar_event_id: None,
            time: "9:00 AM".to_string(),
            end_time: None,
            start_iso: None,
            title: "Test".to_string(),
            meeting_type: "internal".to_string(),
            account: None,
            is_current: false,
            has_prep: false,
            prep_file: None,
            prep_summary: None,
        };

        assert!(!is_meeting_completed(&meeting, now));
    }

    #[test]
    fn test_slug() {
        assert_eq!(slug("Acme Corp"), "acme-corp");
        assert_eq!(
            slug("Follow-up with Engineering"),
            "follow-up-with-engineering"
        );
        assert_eq!(slug("Q1 2026 Review!"), "q1-2026-review");
    }

    #[test]
    fn test_transcript_status_no_account() {
        let temp = tempfile::TempDir::new().unwrap();
        let meeting = JsonMeeting {
            id: "test".to_string(),
            calendar_event_id: None,
            time: "9:00 AM".to_string(),
            end_time: Some("10:00 AM".to_string()),
            start_iso: None,
            title: "Internal Sync".to_string(),
            meeting_type: "internal".to_string(),
            account: None,
            is_current: false,
            has_prep: false,
            prep_file: None,
            prep_summary: None,
        };

        let status = check_transcript_status(temp.path(), &meeting, "2026-02-06");
        assert!(matches!(status, TranscriptStatus::NoTranscript));
    }

    #[test]
    fn test_transcript_status_processed() {
        let temp = tempfile::TempDir::new().unwrap();
        let meetings_dir = temp
            .path()
            .join("Accounts")
            .join("Acme Corp")
            .join("02-Meetings");
        fs::create_dir_all(&meetings_dir).unwrap();
        fs::write(
            meetings_dir.join("2026-02-06-acme-sync.md"),
            "# Meeting Summary",
        )
        .unwrap();

        let meeting = JsonMeeting {
            id: "test".to_string(),
            calendar_event_id: None,
            time: "9:00 AM".to_string(),
            end_time: Some("10:00 AM".to_string()),
            start_iso: None,
            title: "Acme Sync".to_string(),
            meeting_type: "customer".to_string(),
            account: Some("Acme Corp".to_string()),
            is_current: false,
            has_prep: false,
            prep_file: None,
            prep_summary: None,
        };

        let status = check_transcript_status(temp.path(), &meeting, "2026-02-06");
        assert!(matches!(status, TranscriptStatus::Processed));
    }

    #[test]
    fn test_transcript_status_in_inbox() {
        let temp = tempfile::TempDir::new().unwrap();
        let inbox = temp.path().join("_inbox");
        fs::create_dir_all(&inbox).unwrap();
        fs::write(
            inbox.join("2026-02-06-acme-corp-transcript.md"),
            "transcript content",
        )
        .unwrap();

        let meeting = JsonMeeting {
            id: "test".to_string(),
            calendar_event_id: None,
            time: "9:00 AM".to_string(),
            end_time: Some("10:00 AM".to_string()),
            start_iso: None,
            title: "Acme Sync".to_string(),
            meeting_type: "customer".to_string(),
            account: Some("Acme Corp".to_string()),
            is_current: false,
            has_prep: false,
            prep_file: None,
            prep_summary: None,
        };

        let status = check_transcript_status(temp.path(), &meeting, "2026-02-06");
        assert!(matches!(status, TranscriptStatus::InInbox));
    }

    #[test]
    fn test_morning_flags_empty_when_no_gaps() {
        let temp = tempfile::TempDir::new().unwrap();
        let result = run_reconciliation(temp.path(), None);
        assert!(result.flags.is_empty());
        assert_eq!(result.meetings.completed, 0);
    }

    #[test]
    fn test_persist_meetings_freezes_snapshot_and_is_idempotent() {
        let temp = tempfile::TempDir::new().expect("temp dir");
        let workspace = temp.path();
        let preps_dir = workspace.join("_today").join("data").join("preps");
        fs::create_dir_all(&preps_dir).expect("create preps dir");

        let calendar_event_id = "evt-123@google.com";
        let meeting_id = sanitize_calendar_event_id(calendar_event_id);
        let prep_json = serde_json::json!({
            "meetingId": meeting_id,
            "calendarEventId": calendar_event_id,
            "title": "Acme Weekly",
            "talkingPoints": ["Recent win: Stakeholder alignment"],
            "userAgenda": ["Confirm launch timeline"],
            "userNotes": "Need legal follow-up"
        });
        fs::write(
            preps_dir.join(format!("{}.json", meeting_id)),
            serde_json::to_string_pretty(&prep_json).unwrap(),
        )
        .expect("write prep");

        let db_dir = tempfile::TempDir::new().expect("db temp dir");
        let db = ActionDb::open_at(db_dir.path().join("actions.db")).expect("open db");

        let recon = ReconciliationResult {
            date: "2026-02-12".to_string(),
            reconciled_at: Utc::now().to_rfc3339(),
            meetings: MeetingReconciliation {
                completed: 1,
                details: vec![MeetingStatus {
                    title: "Acme Weekly".to_string(),
                    meeting_type: "customer".to_string(),
                    time: "9:00 AM".to_string(),
                    end_time: Some("10:00 AM".to_string()),
                    account: None,
                    calendar_event_id: Some(calendar_event_id.to_string()),
                    transcript_status: TranscriptStatus::NoTranscript,
                }],
            },
            actions: ActionStats {
                completed_today: 0,
                pending: 0,
            },
            flags: vec![],
        };

        persist_meetings(&db, &recon, workspace);
        let first = db
            .get_meeting_by_id(&meeting_id)
            .expect("query")
            .expect("meeting exists");
        assert!(first.user_agenda_json.is_some());
        assert_eq!(first.user_notes.as_deref(), Some("Need legal follow-up"));
        assert!(first.prep_frozen_at.is_some());
        let first_hash = first
            .prep_snapshot_hash
            .clone()
            .expect("snapshot hash populated");
        let snapshot_path = std::path::PathBuf::from(
            first
                .prep_snapshot_path
                .clone()
                .expect("snapshot path populated"),
        );
        assert!(snapshot_path.exists());
        let snapshot: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&snapshot_path).expect("read snapshot"))
                .expect("parse snapshot");
        assert_eq!(snapshot["meetingId"], meeting_id);
        assert_eq!(snapshot["schemaVersion"], 1);

        // Re-run persist to verify freeze idempotency.
        persist_meetings(&db, &recon, workspace);
        let second = db
            .get_meeting_by_id(&meeting_id)
            .expect("query second")
            .expect("meeting exists");
        assert_eq!(
            second.prep_snapshot_hash.as_deref(),
            Some(first_hash.as_str())
        );
        assert_eq!(
            second.prep_snapshot_path.as_deref(),
            first.prep_snapshot_path.as_deref()
        );
    }
}
