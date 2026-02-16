// backfill_meetings.rs
//
// Historical meeting data backfill tool.
// Scans account/project directories for meeting files (transcripts, notes, summaries)
// and creates database records + entity links.

use crate::db::ActionDb;
use crate::types::Config;
use crate::util::slugify;
use chrono::{DateTime, NaiveDate, Utc};
use regex::Regex;
use rusqlite::params;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug)]
struct DiscoveredMeeting {
    file_path: PathBuf,
    date: NaiveDate,
    title: String,
    entity_id: String,
    entity_type: String, // "account" or "project"
    file_type: MeetingFileType,
}

#[derive(Debug)]
enum MeetingFileType {
    Transcript,
    Notes,
    Summary,
}

/// Backfill historical meetings from filesystem into database.
///
/// Returns (meetings_created, meetings_skipped, errors).
pub fn backfill_historical_meetings(
    db: &ActionDb,
    config: &Config,
) -> Result<(usize, usize, Vec<String>), String> {
    let workspace = PathBuf::from(&config.workspace_path);
    let mut created = 0;
    let mut skipped = 0;
    let mut errors = Vec::new();

    // Get existing meeting file paths to avoid duplicates
    let existing_paths = get_existing_meeting_paths(db)?;

    // Scan accounts
    let accounts_dir = workspace.join("Accounts");
    if accounts_dir.exists() {
        match scan_entity_directory(&accounts_dir, "account", &existing_paths) {
            Ok(meetings) => {
                for meeting in meetings {
                    match create_meeting_record(db, &meeting) {
                        Ok(_) => created += 1,
                        Err(e) => {
                            if e.contains("already exists") {
                                skipped += 1;
                            } else {
                                errors.push(format!("{}: {}", meeting.file_path.display(), e));
                            }
                        }
                    }
                }
            }
            Err(e) => errors.push(format!("Accounts scan error: {}", e)),
        }
    }

    // Scan projects
    let projects_dir = workspace.join("Projects");
    if projects_dir.exists() {
        match scan_entity_directory(&projects_dir, "project", &existing_paths) {
            Ok(meetings) => {
                for meeting in meetings {
                    match create_meeting_record(db, &meeting) {
                        Ok(_) => created += 1,
                        Err(e) => {
                            if e.contains("already exists") {
                                skipped += 1;
                            } else {
                                errors.push(format!("{}: {}", meeting.file_path.display(), e));
                            }
                        }
                    }
                }
            }
            Err(e) => errors.push(format!("Projects scan error: {}", e)),
        }
    }

    Ok((created, skipped, errors))
}

/// Get set of existing meeting file paths from database.
fn get_existing_meeting_paths(db: &ActionDb) -> Result<HashSet<String>, String> {
    let conn = db.conn_ref();
    let mut stmt = conn
        .prepare(
            "SELECT notes_path FROM meetings_history WHERE notes_path IS NOT NULL
             UNION
             SELECT transcript_path FROM meetings_history WHERE transcript_path IS NOT NULL",
        )
        .map_err(|e| e.to_string())?;

    let paths = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(paths)
}

/// Scan an entity directory (Accounts or Projects) for meeting files.
fn scan_entity_directory(
    dir: &Path,
    entity_type: &str,
    existing_paths: &HashSet<String>,
) -> Result<Vec<DiscoveredMeeting>, String> {
    let mut meetings = Vec::new();

    // Regex to match date in filename: YYYY-MM-DD
    let date_re = Regex::new(r"(\d{4})-(\d{2})-(\d{2})").unwrap();

    // Meeting file indicators
    let meeting_dirs = [
        "02-Meetings",
        "03-Call-Transcripts",
        "Call-Transcripts",
        "Meeting-Notes",
    ];

    for entry in WalkDir::new(dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        // Skip if not a markdown file
        if !path.extension().map_or(false, |ext| ext == "md") {
            continue;
        }

        // Skip dashboard files
        let filename = path.file_name().unwrap().to_string_lossy();
        if filename.starts_with("dashboard") || filename.starts_with("intelligence") {
            continue;
        }

        // Check if in a meeting directory
        let path_str = path.to_string_lossy();
        let in_meeting_dir = meeting_dirs.iter().any(|d| path_str.contains(d));

        // OR check if filename has a date pattern (catch files in other dirs like 05-Projects)
        let has_date = date_re.is_match(&filename);

        if !in_meeting_dir && !has_date {
            continue;
        }

        // Skip if already in database
        let absolute_path = path.canonicalize().ok().and_then(|p| p.to_str().map(String::from));
        if let Some(abs_path) = &absolute_path {
            if existing_paths.contains(abs_path) {
                continue;
            }
        }

        // Extract date from filename
        let date = match date_re.captures(&filename) {
            Some(caps) => {
                let year: i32 = caps[1].parse().unwrap();
                let month: u32 = caps[2].parse().unwrap();
                let day: u32 = caps[3].parse().unwrap();
                match NaiveDate::from_ymd_opt(year, month, day) {
                    Some(d) => d,
                    None => continue, // Invalid date, skip
                }
            }
            None => continue, // No date in filename, skip
        };

        // Determine entity_id from directory path
        let entity_id = match extract_entity_id(path, entity_type) {
            Some(id) => id,
            None => continue, // Can't determine entity, skip
        };

        // Determine file type
        let file_type = if path_str.contains("transcript") || filename.contains("transcript") {
            MeetingFileType::Transcript
        } else if filename.contains("summary") {
            MeetingFileType::Summary
        } else {
            MeetingFileType::Notes
        };

        // Extract title from filename (remove date and type prefix)
        let title = extract_title_from_filename(&filename, &date_re);

        meetings.push(DiscoveredMeeting {
            file_path: path.to_path_buf(),
            date,
            title,
            entity_id,
            entity_type: entity_type.to_string(),
            file_type,
        });
    }

    Ok(meetings)
}

/// Extract entity ID from file path.
///
/// Path format: .../Accounts/{account-slug}/... or .../Projects/{project-slug}/...
fn extract_entity_id(path: &Path, entity_type: &str) -> Option<String> {
    let path_str = path.to_string_lossy();
    let parts: Vec<&str> = path_str.split('/').collect();

    // Find the "Accounts" or "Projects" component
    let marker = if entity_type == "account" {
        "Accounts"
    } else {
        "Projects"
    };

    for (i, part) in parts.iter().enumerate() {
        if *part == marker && i + 1 < parts.len() {
            // The next part is the entity slug
            let slug = parts[i + 1];

            // Handle nested accounts (e.g., Hilton/Corporate)
            // For now, use the immediate child as entity_id
            // This matches the directory-based entity ID convention

            return Some(slug_to_entity_id(slug, entity_type));
        }
    }

    None
}

/// Convert directory slug to entity ID using canonical slugify function.
///
/// Uses the same normalization as the rest of the codebase (util::slugify).
fn slug_to_entity_id(slug: &str, _entity_type: &str) -> String {
    slugify(slug)
}

/// Extract human-readable title from filename.
fn extract_title_from_filename(filename: &str, date_re: &Regex) -> String {
    let mut title = filename.to_string();

    // Remove extension
    if let Some(pos) = title.rfind(".md") {
        title.truncate(pos);
    }

    // Remove date prefix
    title = date_re.replace(&title, "").to_string();

    // Remove type prefixes (summary-, meeting-, strategy-, transcript-)
    let prefixes = ["summary-", "meeting-", "strategy-", "transcript-", "call-"];
    for prefix in &prefixes {
        if title.starts_with(prefix) {
            title = title[prefix.len()..].to_string();
            break;
        }
    }

    // Clean up: trim hyphens, replace remaining hyphens with spaces, title case
    title = title.trim_matches('-').replace('-', " ");

    // Basic title case (capitalize first letter of each word)
    title
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Create meeting record in database and link to entity.
fn create_meeting_record(db: &ActionDb, meeting: &DiscoveredMeeting) -> Result<(), String> {
    let conn = db.conn_ref();

    // Generate meeting ID
    let meeting_id = format!(
        "{}-{}",
        meeting.date.format("%Y%m%d"),
        meeting
            .title
            .to_lowercase()
            .replace(' ', "-")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-')
            .collect::<String>()
    );

    // Check if meeting ID already exists
    let exists: bool = conn
        .query_row(
            "SELECT 1 FROM meetings_history WHERE id = ?1",
            params![&meeting_id],
            |_| Ok(true),
        )
        .unwrap_or(false);

    if exists {
        return Err(format!("Meeting {} already exists", meeting_id));
    }

    // Determine meeting type (customer vs internal)
    // For now, all historical meetings assumed to be customer meetings
    // TODO: Could parse file content or use account metadata
    let meeting_type = "customer";

    // ISO 8601 timestamp for start_time (using date at noon)
    let start_time = meeting
        .date
        .and_hms_opt(12, 0, 0)
        .unwrap()
        .and_utc()
        .to_rfc3339();

    let created_at: DateTime<Utc> = Utc::now();
    let created_at_str = created_at.to_rfc3339();

    // Get absolute path for notes_path / transcript_path
    let absolute_path = meeting
        .file_path
        .canonicalize()
        .ok()
        .and_then(|p| p.to_str().map(String::from));

    let (notes_path, transcript_path) = match meeting.file_type {
        MeetingFileType::Transcript => (None, absolute_path),
        _ => (absolute_path, None),
    };

    // Insert meeting record
    conn.execute(
        "INSERT INTO meetings_history (
            id, title, meeting_type, start_time, created_at,
            notes_path, transcript_path
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            &meeting_id,
            &meeting.title,
            meeting_type,
            &start_time,
            &created_at_str,
            &notes_path,
            &transcript_path,
        ],
    )
    .map_err(|e| e.to_string())?;

    // Link to entity via meeting_entities table
    conn.execute(
        "INSERT OR IGNORE INTO meeting_entities (meeting_id, entity_id, entity_type)
         VALUES (?1, ?2, ?3)",
        params![&meeting_id, &meeting.entity_id, &meeting.entity_type],
    )
    .map_err(|e| e.to_string())?;

    log::info!(
        "Created meeting: {} ({}) linked to {} {}",
        meeting_id,
        meeting.title,
        meeting.entity_type,
        meeting.entity_id
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_title_from_filename() {
        let date_re = Regex::new(r"(\d{4})-(\d{2})-(\d{2})").unwrap();

        assert_eq!(
            extract_title_from_filename(
                "2025-07-23-summary-airbnb-wordpress-vip-introduction.md",
                &date_re
            ),
            "Airbnb Wordpress Vip Introduction"
        );

        assert_eq!(
            extract_title_from_filename("2025-06-23-strategy-shobha-airbnb-account-strategy-discussion.md", &date_re),
            "Shobha Airbnb Account Strategy Discussion"
        );

        assert_eq!(
            extract_title_from_filename(
                "2026-02-05 SlackWPVIP-sync_-transcript.md",
                &date_re
            ),
            "Slackwpvip Sync  Transcript"
        );
    }

    #[test]
    fn test_slug_to_entity_id() {
        assert_eq!(slug_to_entity_id("Bring-a-Trailer", "account"), "bring-a-trailer");
        assert_eq!(slug_to_entity_id("Bring-A-Trailer-(bat)", "account"), "bring-a-trailer-bat");
        assert_eq!(slug_to_entity_id("Hilton", "account"), "hilton");
        assert_eq!(slug_to_entity_id("Jane-Software", "account"), "jane-software");
        // Parentheses and special chars become hyphens, then deduplicated
        assert_eq!(slug_to_entity_id("Salesforce (Digital Marketing)", "account"), "salesforce-digital-marketing");
    }
}
