//! Daily impact rollup for CS Extension (I36 / ADR-0041)
//!
//! Aggregates post-meeting captures (wins/risks) from SQLite into the weekly
//! impact markdown file at:
//!   `{workspace}/Leadership/02-Performance/Weekly-Impact/{YYYY}-W{WW}-impact-capture.md`
//!
//! Called during the archive workflow, after reconciliation and before file moves.
//! Profile-gated: only runs when `config.profile == "customer-success"`.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use chrono::{Datelike, NaiveDate};

use crate::db::{ActionDb, DbCapture};

/// Result of running the daily impact rollup.
#[derive(Debug)]
pub struct RollupResult {
    pub wins_rolled_up: usize,
    pub risks_rolled_up: usize,
    pub file_path: String,
    pub skipped: bool,
}

/// Roll up today's captures into the weekly impact file.
///
/// Returns `Ok(RollupResult)` with counts. Idempotent: if the day header already
/// exists in the file, returns with `skipped: true`.
pub fn rollup_daily_impact(
    workspace: &Path,
    db: &ActionDb,
    date: &str,
) -> Result<RollupResult, String> {
    // 1. Query captures for the date
    let captures = db
        .get_captures_for_date(date)
        .map_err(|e| format!("Failed to query captures: {}", e))?;

    // Filter to wins and risks only (actions/decisions don't go to impact file)
    let wins: Vec<&DbCapture> = captures
        .iter()
        .filter(|c| c.capture_type == "win")
        .collect();
    let risks: Vec<&DbCapture> = captures
        .iter()
        .filter(|c| c.capture_type == "risk")
        .collect();

    if wins.is_empty() && risks.is_empty() {
        return Ok(RollupResult {
            wins_rolled_up: 0,
            risks_rolled_up: 0,
            file_path: String::new(),
            skipped: false,
        });
    }

    // 2. Compute ISO week and file path
    let parsed_date = NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid date '{}': {}", date, e))?;
    let iso_week = parsed_date.iso_week();
    let year = iso_week.year();
    let week = iso_week.week();

    let impact_dir = workspace
        .join("Leadership")
        .join("02-Performance")
        .join("Weekly-Impact");
    let filename = format!("{}-W{:02}-impact-capture.md", year, week);
    let file_path = impact_dir.join(&filename);

    // 3. Read or create the file
    fs::create_dir_all(&impact_dir)
        .map_err(|e| format!("Failed to create impact directory: {}", e))?;

    let existing_content = if file_path.exists() {
        fs::read_to_string(&file_path)
            .map_err(|e| format!("Failed to read impact file: {}", e))?
    } else {
        create_template(year, week)
    };

    // 4. Idempotency check: does the day header already exist?
    let day_header = format_day_header(&parsed_date);
    if existing_content.contains(&day_header) {
        return Ok(RollupResult {
            wins_rolled_up: 0,
            risks_rolled_up: 0,
            file_path: file_path.to_string_lossy().to_string(),
            skipped: true,
        });
    }

    // 5. Group captures by account (or meeting title as fallback)
    let win_entries = format_entries(&wins);
    let risk_entries = format_entries(&risks);

    // 6. Insert entries into the correct sections
    let updated = insert_into_section(&existing_content, "## Customer Outcomes", &day_header, &win_entries)
        .and_then(|content| insert_into_section(&content, "## Risk Management", &day_header, &risk_entries));

    let final_content = match updated {
        Some(content) => content,
        None => {
            // Fallback: append to end if sections not found
            let mut content = existing_content;
            if !win_entries.is_empty() {
                content.push_str(&format!("\n## Customer Outcomes\n\n{}\n{}\n", day_header, win_entries));
            }
            if !risk_entries.is_empty() {
                content.push_str(&format!("\n## Risk Management\n\n{}\n{}\n", day_header, risk_entries));
            }
            content
        }
    };

    // 7. Write file
    fs::write(&file_path, &final_content)
        .map_err(|e| format!("Failed to write impact file: {}", e))?;

    Ok(RollupResult {
        wins_rolled_up: wins.len(),
        risks_rolled_up: risks.len(),
        file_path: file_path.to_string_lossy().to_string(),
        skipped: false,
    })
}

/// Format the day header: `### Friday, Feb 7`
fn format_day_header(date: &NaiveDate) -> String {
    let day_name = date.format("%A").to_string();
    let month_day = date.format("%b %-d").to_string();
    format!("### {}, {}", day_name, month_day)
}

/// Format capture entries grouped by account/meeting.
fn format_entries(captures: &[&DbCapture]) -> String {
    if captures.is_empty() {
        return String::new();
    }

    // Group by label (account_id or meeting_title fallback)
    let mut grouped: BTreeMap<String, Vec<&DbCapture>> = BTreeMap::new();
    for cap in captures {
        let label = cap
            .account_id
            .as_deref()
            .unwrap_or(&cap.meeting_title);
        grouped.entry(label.to_string()).or_default().push(cap);
    }

    let mut lines = Vec::new();
    for (label, caps) in &grouped {
        for cap in caps {
            lines.push(format!("- **{}**: {} *(from {})*", label, cap.content, cap.meeting_title));
        }
    }

    lines.join("\n")
}

/// Insert a day header and entries after a section header in the markdown content.
///
/// Finds the section (e.g. `## Customer Outcomes`) and inserts the new day entry
/// after the section header (or after the last existing day entry in that section).
/// Returns `None` if the section was not found.
fn insert_into_section(
    content: &str,
    section_header: &str,
    day_header: &str,
    entries: &str,
) -> Option<String> {
    if entries.is_empty() {
        return Some(content.to_string());
    }

    let section_pos = content.find(section_header)?;
    let after_header = section_pos + section_header.len();

    // Find the end of this section (next ## heading or end of file)
    let section_end = content[after_header..]
        .find("\n## ")
        .map(|pos| after_header + pos)
        .unwrap_or(content.len());

    // Insert the new day entry right after the section header line
    // Find the end of the header line (the newline after `## Customer Outcomes`)
    let insert_pos = content[after_header..section_end]
        .find('\n')
        .map(|pos| after_header + pos + 1)
        .unwrap_or(section_end);

    let mut result = String::with_capacity(content.len() + day_header.len() + entries.len() + 4);
    result.push_str(&content[..insert_pos]);
    result.push('\n');
    result.push_str(day_header);
    result.push('\n');
    result.push_str(entries);
    result.push('\n');
    result.push_str(&content[insert_pos..]);

    Some(result)
}

/// Create a minimal weekly impact file template.
fn create_template(year: i32, week: u32) -> String {
    format!(
        "---\nweek: {}-W{:02}\n---\n\n## Customer Outcomes\n\n## Risk Management\n\n## Summary Stats\n",
        year, week
    )
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::ActionDb;
    use rusqlite::params;

    fn test_db() -> ActionDb {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("test.db");
        std::mem::forget(dir);
        ActionDb::open_at(path).expect("open db")
    }

    fn insert_capture(
        db: &ActionDb,
        id: &str,
        meeting_id: &str,
        meeting_title: &str,
        account_id: Option<&str>,
        capture_type: &str,
        content: &str,
        date: &str,
    ) {
        let ts = format!("{}T14:00:00+00:00", date);
        db.conn_ref()
            .execute(
                "INSERT INTO captures (id, meeting_id, meeting_title, account_id, capture_type, content, captured_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![id, meeting_id, meeting_title, account_id, capture_type, content, ts],
            )
            .expect("insert capture");
    }

    #[test]
    fn test_empty_captures_noop() {
        let db = test_db();
        let temp = tempfile::tempdir().unwrap();

        let result = rollup_daily_impact(temp.path(), &db, "2026-02-07").unwrap();
        assert_eq!(result.wins_rolled_up, 0);
        assert_eq!(result.risks_rolled_up, 0);
        assert!(!result.skipped);
        assert!(result.file_path.is_empty());
    }

    #[test]
    fn test_wins_rollup_to_customer_outcomes() {
        let db = test_db();
        let temp = tempfile::tempdir().unwrap();

        insert_capture(&db, "c1", "mtg-1", "Acme QBR", Some("Acme"), "win", "Expanded deployment to 500 users", "2026-02-07");
        insert_capture(&db, "c2", "mtg-2", "Beta Sync", Some("Beta"), "win", "New champion identified", "2026-02-07");

        let result = rollup_daily_impact(temp.path(), &db, "2026-02-07").unwrap();
        assert_eq!(result.wins_rolled_up, 2);
        assert_eq!(result.risks_rolled_up, 0);
        assert!(!result.skipped);

        let content = fs::read_to_string(&result.file_path).unwrap();
        assert!(content.contains("### Saturday, Feb 7"));
        assert!(content.contains("**Acme**: Expanded deployment to 500 users *(from Acme QBR)*"));
        assert!(content.contains("**Beta**: New champion identified *(from Beta Sync)*"));
        assert!(content.contains("## Customer Outcomes"));
    }

    #[test]
    fn test_risks_rollup_to_risk_management() {
        let db = test_db();
        let temp = tempfile::tempdir().unwrap();

        insert_capture(&db, "c1", "mtg-1", "Acme QBR", Some("Acme"), "risk", "Budget freeze in Q2", "2026-02-07");

        let result = rollup_daily_impact(temp.path(), &db, "2026-02-07").unwrap();
        assert_eq!(result.wins_rolled_up, 0);
        assert_eq!(result.risks_rolled_up, 1);

        let content = fs::read_to_string(&result.file_path).unwrap();
        assert!(content.contains("## Risk Management"));
        assert!(content.contains("**Acme**: Budget freeze in Q2 *(from Acme QBR)*"));
    }

    #[test]
    fn test_idempotency_skips_duplicate() {
        let db = test_db();
        let temp = tempfile::tempdir().unwrap();

        insert_capture(&db, "c1", "mtg-1", "Acme QBR", Some("Acme"), "win", "Big win", "2026-02-07");

        // First run writes
        let r1 = rollup_daily_impact(temp.path(), &db, "2026-02-07").unwrap();
        assert!(!r1.skipped);
        assert_eq!(r1.wins_rolled_up, 1);

        let content_after_first = fs::read_to_string(&r1.file_path).unwrap();

        // Second run skips
        let r2 = rollup_daily_impact(temp.path(), &db, "2026-02-07").unwrap();
        assert!(r2.skipped);
        assert_eq!(r2.wins_rolled_up, 0);

        // Content should be unchanged
        let content_after_second = fs::read_to_string(&r2.file_path).unwrap();
        assert_eq!(content_after_first, content_after_second);
    }

    #[test]
    fn test_file_creation_with_template() {
        let db = test_db();
        let temp = tempfile::tempdir().unwrap();

        insert_capture(&db, "c1", "mtg-1", "Acme QBR", Some("Acme"), "win", "Win", "2026-02-07");

        let result = rollup_daily_impact(temp.path(), &db, "2026-02-07").unwrap();

        let content = fs::read_to_string(&result.file_path).unwrap();
        // Should have frontmatter
        assert!(content.starts_with("---\n"));
        assert!(content.contains("week: 2026-W06"));
        // Should have sections
        assert!(content.contains("## Customer Outcomes"));
        assert!(content.contains("## Risk Management"));
        assert!(content.contains("## Summary Stats"));
    }

    #[test]
    fn test_no_account_uses_meeting_title() {
        let db = test_db();
        let temp = tempfile::tempdir().unwrap();

        insert_capture(&db, "c1", "mtg-1", "Team Retro", None, "win", "Improved velocity", "2026-02-07");

        let result = rollup_daily_impact(temp.path(), &db, "2026-02-07").unwrap();
        assert_eq!(result.wins_rolled_up, 1);

        let content = fs::read_to_string(&result.file_path).unwrap();
        assert!(content.contains("**Team Retro**: Improved velocity *(from Team Retro)*"));
    }

    #[test]
    fn test_mixed_wins_and_risks() {
        let db = test_db();
        let temp = tempfile::tempdir().unwrap();

        insert_capture(&db, "c1", "mtg-1", "Acme QBR", Some("Acme"), "win", "Expanded deployment", "2026-02-07");
        insert_capture(&db, "c2", "mtg-1", "Acme QBR", Some("Acme"), "risk", "Budget freeze", "2026-02-07");
        insert_capture(&db, "c3", "mtg-2", "Beta Sync", Some("Beta"), "win", "New champion", "2026-02-07");

        let result = rollup_daily_impact(temp.path(), &db, "2026-02-07").unwrap();
        assert_eq!(result.wins_rolled_up, 2);
        assert_eq!(result.risks_rolled_up, 1);

        let content = fs::read_to_string(&result.file_path).unwrap();

        // Wins in Customer Outcomes
        assert!(content.contains("**Acme**: Expanded deployment *(from Acme QBR)*"));
        assert!(content.contains("**Beta**: New champion *(from Beta Sync)*"));

        // Risk in Risk Management
        assert!(content.contains("**Acme**: Budget freeze *(from Acme QBR)*"));

        // Day header appears in both sections
        let day_header_count = content.matches("### Saturday, Feb 7").count();
        assert_eq!(day_header_count, 2, "Day header should appear in both sections");
    }

    #[test]
    fn test_actions_and_decisions_excluded() {
        let db = test_db();
        let temp = tempfile::tempdir().unwrap();

        insert_capture(&db, "c1", "mtg-1", "Acme QBR", Some("Acme"), "action", "Follow up on X", "2026-02-07");
        insert_capture(&db, "c2", "mtg-1", "Acme QBR", Some("Acme"), "decision", "Go with option B", "2026-02-07");

        let result = rollup_daily_impact(temp.path(), &db, "2026-02-07").unwrap();
        // Actions and decisions should not be rolled up
        assert_eq!(result.wins_rolled_up, 0);
        assert_eq!(result.risks_rolled_up, 0);
        assert!(result.file_path.is_empty());
    }

    #[test]
    fn test_format_day_header() {
        let date = NaiveDate::from_ymd_opt(2026, 2, 7).unwrap();
        assert_eq!(format_day_header(&date), "### Saturday, Feb 7");

        let date2 = NaiveDate::from_ymd_opt(2026, 1, 5).unwrap();
        assert_eq!(format_day_header(&date2), "### Monday, Jan 5");
    }

    #[test]
    fn test_insert_into_section() {
        let content = "---\nweek: 2026-W06\n---\n\n## Customer Outcomes\n\n## Risk Management\n\n## Summary Stats\n";
        let result = insert_into_section(content, "## Customer Outcomes", "### Friday, Feb 6", "- **Acme**: Win").unwrap();
        assert!(result.contains("### Friday, Feb 6\n- **Acme**: Win"));
        // Other sections preserved
        assert!(result.contains("## Risk Management"));
        assert!(result.contains("## Summary Stats"));
    }
}
