//! Action parsing from workspace markdown + SQLite state merge.
//!
//! Ported from ops/action_parse.py per ADR-0049.
//! Addresses I23: pre-checks SQLite before extracting from markdown
//! to avoid re-extracting completed actions.

use std::collections::HashSet;
use std::path::Path;

use chrono::{Datelike, NaiveDate, Utc};
use regex::Regex;
use serde_json::{json, Value};

/// Result of parsing workspace actions.
pub struct ActionResult {
    pub overdue: Vec<Value>,
    pub due_today: Vec<Value>,
    pub due_this_week: Vec<Value>,
    pub waiting_on: Vec<Value>,
}

impl Default for ActionResult {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionResult {
    pub fn new() -> Self {
        Self {
            overdue: Vec::new(),
            due_today: Vec::new(),
            due_this_week: Vec::new(),
            waiting_on: Vec::new(),
        }
    }

    /// Serialize to directive-compatible JSON object.
    pub fn to_value(&self) -> Value {
        json!({
            "overdue": self.overdue,
            "due_today": self.due_today,
            "due_this_week": self.due_this_week,
            "waiting_on": self.waiting_on,
        })
    }
}

/// Parse actions from workspace markdown + merge SQLite state.
///
/// Addresses I23: pre-checks SQLite before extracting from markdown
/// to avoid re-extracting completed actions.
pub fn parse_workspace_actions(workspace: &Path, db: Option<&crate::db::ActionDb>) -> ActionResult {
    let mut result = ActionResult::new();

    // Load existing action titles from SQLite (I23 pre-check)
    let existing_titles = match db {
        Some(db) => load_existing_titles(db),
        None => HashSet::new(),
    };

    // Find actions.md
    let actions_path = workspace.join("actions.md");
    let actions_path = if actions_path.exists() {
        actions_path
    } else {
        let alt = workspace.join("_today").join("actions.md");
        if alt.exists() {
            alt
        } else {
            return result;
        }
    };

    let content = match std::fs::read_to_string(&actions_path) {
        Ok(c) => c,
        Err(_) => return result,
    };

    let today = Utc::now().date_naive();
    let monday = today - chrono::Duration::days(today.weekday().num_days_from_monday() as i64);
    let friday = monday + chrono::Duration::days(4);

    let checkbox_re = Regex::new(r"^\s*-\s*\[\s*\]\s*(.+)$").unwrap();
    let due_re = Regex::new(r"(?i)due[:\s]+(\d{4}-\d{2}-\d{2})").unwrap();
    let priority_re = Regex::new(r"(?i)\b(P[123])\b").unwrap();
    let account_re = Regex::new(r"@(\S+)").unwrap();
    let context_re = Regex::new(r"#(\S+)").unwrap();
    let waiting_re = Regex::new(r"(?i)\b(waiting|blocked|pending)\b").unwrap();
    let whitespace_re = Regex::new(r"\s+").unwrap();

    for line in content.lines() {
        let caps = match checkbox_re.captures(line) {
            Some(c) => c,
            None => continue,
        };
        let text = caps.get(1).unwrap().as_str().trim();

        // Extract metadata
        let due_date: Option<NaiveDate> = due_re
            .captures(text)
            .and_then(|c| NaiveDate::parse_from_str(c.get(1).unwrap().as_str(), "%Y-%m-%d").ok());

        let priority = priority_re
            .captures(text)
            .map(|c| c.get(1).unwrap().as_str().to_uppercase())
            .unwrap_or_else(|| "P3".to_string());

        let account = account_re
            .captures(text)
            .map(|c| c.get(1).unwrap().as_str().to_string());

        let context = context_re
            .captures(text)
            .map(|c| c.get(1).unwrap().as_str().to_string());

        // Clean the title: remove metadata markers
        let mut title = text.to_string();
        title = due_re.replace_all(&title, "").to_string();
        title = priority_re.replace_all(&title, "").to_string();
        title = account_re.replace_all(&title, "").to_string();
        title = context_re.replace_all(&title, "").to_string();
        // Collapse whitespace
        title = whitespace_re.replace_all(&title, " ").trim().to_string();

        // I23: Skip if this action already exists in SQLite
        if existing_titles.contains(&title.to_lowercase()) {
            continue;
        }

        let mut action = json!({
            "title": title,
            "account": account,
            "due_date": due_date.map(|d| d.to_string()),
            "priority": priority,
            "context": context,
            "raw": text,
        });

        // Check for "waiting on" items
        if waiting_re.is_match(text) {
            result.waiting_on.push(action);
            continue;
        }

        // Categorize by due date
        match due_date {
            Some(d) if d < today => {
                let days_overdue = (today - d).num_days();
                action["days_overdue"] = json!(days_overdue);
                result.overdue.push(action);
            }
            Some(d) if d == today => {
                result.due_today.push(action);
            }
            Some(d) if d >= monday && d <= friday => {
                result.due_this_week.push(action);
            }
            Some(_) => {
                // Future, beyond this week — skip
            }
            None => {
                // No due date: treat as due_this_week (low priority)
                result.due_this_week.push(action);
            }
        }
    }

    result
}

/// Load all existing action titles from SQLite for dedup pre-check (I23).
fn load_existing_titles(db: &crate::db::ActionDb) -> HashSet<String> {
    let mut titles = HashSet::new();
    let conn = db.conn_ref();
    let mut stmt = match conn.prepare("SELECT LOWER(TRIM(title)) FROM actions") {
        Ok(s) => s,
        Err(_) => return titles,
    };
    let rows = stmt.query_map([], |row: &rusqlite::Row| row.get::<_, Option<String>>(0));
    if let Ok(rows) = rows {
        for row in rows.flatten().flatten() {
            titles.insert(row);
        }
    }
    titles
}

/// Collect all actions for the today directive from both sources:
/// 1. Workspace markdown (`actions.md`)
/// 2. SQLite actions table (with due dates)
///
/// Deduplicates by title (lowercase). SQLite actions take precedence.
pub fn collect_all_actions(workspace: &Path, db: Option<&crate::db::ActionDb>) -> ActionResult {
    let markdown_result = parse_workspace_actions(workspace, db);

    let db_result = match db {
        Some(db) => fetch_categorized_actions(db),
        None => return markdown_result,
    };

    // Merge: start with SQLite actions, add markdown-only actions
    let mut merged = db_result;
    let seen_titles: HashSet<String> = merged
        .overdue
        .iter()
        .chain(merged.due_today.iter())
        .chain(merged.due_this_week.iter())
        .chain(merged.waiting_on.iter())
        .filter_map(|a| a.get("title").and_then(|t| t.as_str()))
        .map(|t| t.to_lowercase())
        .collect();

    for action in markdown_result.overdue {
        if let Some(title) = action.get("title").and_then(|t| t.as_str()) {
            if !seen_titles.contains(&title.to_lowercase()) {
                merged.overdue.push(action);
            }
        }
    }
    for action in markdown_result.due_today {
        if let Some(title) = action.get("title").and_then(|t| t.as_str()) {
            if !seen_titles.contains(&title.to_lowercase()) {
                merged.due_today.push(action);
            }
        }
    }
    for action in markdown_result.due_this_week {
        if let Some(title) = action.get("title").and_then(|t| t.as_str()) {
            if !seen_titles.contains(&title.to_lowercase()) {
                merged.due_this_week.push(action);
            }
        }
    }
    for action in markdown_result.waiting_on {
        if let Some(title) = action.get("title").and_then(|t| t.as_str()) {
            if !seen_titles.contains(&title.to_lowercase()) {
                merged.waiting_on.push(action);
            }
        }
    }

    merged
}

/// Fetch categorized actions from SQLite with today/this-week/overdue/waiting buckets.
fn fetch_categorized_actions(db: &crate::db::ActionDb) -> ActionResult {
    let mut result = ActionResult::new();
    let today = Utc::now().date_naive();
    let monday = today - chrono::Duration::days(today.weekday().num_days_from_monday() as i64);
    let friday = monday + chrono::Duration::days(4);

    let conn = db.conn_ref();

    // Fetch all non-completed actions with due dates
    let mut stmt = match conn.prepare(
        "SELECT id, title, priority, status, due_date, account_id, source_context
         FROM actions
         WHERE status != 'completed'
           AND due_date IS NOT NULL
         ORDER BY due_date ASC",
    ) {
        Ok(s) => s,
        Err(_) => return result,
    };

    let rows = stmt.query_map([], |row: &rusqlite::Row| {
        Ok((
            row.get::<_, Option<String>>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, Option<String>>(5)?,
            row.get::<_, Option<String>>(6)?,
        ))
    });

    if let Ok(rows) = rows {
        for row in rows.flatten() {
            let (id, title, priority, status, due_str, account_id, source_context) = row;
            let title = title.unwrap_or_default();
            if title.is_empty() {
                continue;
            }

            let due_date = due_str
                .as_ref()
                .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok());

            let priority = priority.unwrap_or_else(|| "P3".to_string());
            let status = status.unwrap_or_else(|| "open".to_string());

            // Check for waiting status
            if status == "waiting" || status == "blocked" {
                result.waiting_on.push(json!({
                    "id": id,
                    "title": title,
                    "priority": priority,
                    "due_date": due_str,
                    "account": account_id,
                    "context": source_context,
                }));
                continue;
            }

            let mut action = json!({
                "id": id,
                "title": title,
                "priority": priority,
                "due_date": due_str,
                "account": account_id,
                "context": source_context,
            });

            match due_date {
                Some(d) if d < today => {
                    let days_overdue = (today - d).num_days();
                    action["days_overdue"] = json!(days_overdue);
                    result.overdue.push(action);
                }
                Some(d) if d == today => {
                    result.due_today.push(action);
                }
                Some(d) if d >= monday && d <= friday => {
                    result.due_this_week.push(action);
                }
                _ => {
                    // Future beyond this week — skip
                }
            }
        }
    }

    result
}

/// Read overdue and this-week actions directly from SQLite.
///
/// Used by the /week orchestrator which reads from SQLite rather than
/// parsing markdown.
pub fn fetch_actions_from_db(db: &crate::db::ActionDb) -> Value {
    let today = Utc::now().date_naive();
    let monday = today - chrono::Duration::days(today.weekday().num_days_from_monday() as i64);
    let friday = monday + chrono::Duration::days(4);

    let today_str = today.to_string();
    let monday_str = monday.to_string();
    let friday_str = friday.to_string();

    let mut overdue: Vec<Value> = Vec::new();
    let mut this_week: Vec<Value> = Vec::new();

    let conn = db.conn_ref();

    // Overdue
    if let Ok(mut stmt) = conn.prepare(
        "SELECT id, title, priority, status, due_date, account_id
         FROM actions
         WHERE status != 'completed'
           AND due_date IS NOT NULL
           AND due_date < ?1
         ORDER BY due_date ASC",
    ) {
        if let Ok(rows) = stmt.query_map([&today_str], |row: &rusqlite::Row| {
            let due: Option<String> = row.get(4)?;
            let days_overdue = due
                .as_ref()
                .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
                .map(|d| (today - d).num_days())
                .unwrap_or(0);

            Ok(json!({
                "id": row.get::<_, Option<String>>(0)?,
                "title": row.get::<_, Option<String>>(1)?,
                "priority": row.get::<_, Option<String>>(2)?,
                "status": row.get::<_, Option<String>>(3)?,
                "dueDate": due,
                "accountId": row.get::<_, Option<String>>(5)?,
                "daysOverdue": days_overdue,
            }))
        }) {
            for row in rows.flatten() {
                overdue.push(row);
            }
        }
    }

    // This week
    if let Ok(mut stmt) = conn.prepare(
        "SELECT id, title, priority, status, due_date, account_id
         FROM actions
         WHERE status != 'completed'
           AND due_date IS NOT NULL
           AND due_date >= ?1
           AND due_date <= ?2
         ORDER BY due_date ASC",
    ) {
        if let Ok(rows) = stmt.query_map([&monday_str, &friday_str], |row: &rusqlite::Row| {
            Ok(json!({
                "id": row.get::<_, Option<String>>(0)?,
                "title": row.get::<_, Option<String>>(1)?,
                "priority": row.get::<_, Option<String>>(2)?,
                "status": row.get::<_, Option<String>>(3)?,
                "dueDate": row.get::<_, Option<String>>(4)?,
                "accountId": row.get::<_, Option<String>>(5)?,
            }))
        }) {
            for row in rows.flatten() {
                this_week.push(row);
            }
        }
    }

    json!({
        "overdue": overdue,
        "thisWeek": this_week,
    })
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_workspace() {
        let dir = tempfile::tempdir().unwrap();
        let result = parse_workspace_actions(dir.path(), None);
        assert!(result.overdue.is_empty());
        assert!(result.due_today.is_empty());
        assert!(result.due_this_week.is_empty());
        assert!(result.waiting_on.is_empty());
    }

    #[test]
    fn test_parse_basic_action() {
        let dir = tempfile::tempdir().unwrap();
        let actions_path = dir.path().join("actions.md");
        std::fs::write(&actions_path, "- [ ] Write tests\n- [x] Done task\n").unwrap();

        let result = parse_workspace_actions(dir.path(), None);
        // "Write tests" has no due date → due_this_week
        assert_eq!(result.due_this_week.len(), 1);
        assert_eq!(result.due_this_week[0]["title"], "Write tests");
    }

    #[test]
    fn test_parse_with_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let today = Utc::now().date_naive();
        let actions_path = dir.path().join("actions.md");
        std::fs::write(
            &actions_path,
            format!("- [ ] Review PR due:{} P1 @Acme #dev\n", today),
        )
        .unwrap();

        let result = parse_workspace_actions(dir.path(), None);
        assert_eq!(result.due_today.len(), 1);
        assert_eq!(result.due_today[0]["priority"], "P1");
        assert_eq!(result.due_today[0]["account"], "Acme");
        assert_eq!(result.due_today[0]["context"], "dev");
    }

    #[test]
    fn test_parse_overdue() {
        let dir = tempfile::tempdir().unwrap();
        let yesterday = Utc::now().date_naive() - chrono::Duration::days(3);
        let actions_path = dir.path().join("actions.md");
        std::fs::write(&actions_path, format!("- [ ] Old task due:{}\n", yesterday)).unwrap();

        let result = parse_workspace_actions(dir.path(), None);
        assert_eq!(result.overdue.len(), 1);
        assert_eq!(result.overdue[0]["days_overdue"], 3);
    }

    #[test]
    fn test_parse_waiting_on() {
        let dir = tempfile::tempdir().unwrap();
        let actions_path = dir.path().join("actions.md");
        std::fs::write(&actions_path, "- [ ] Waiting on approval from legal\n").unwrap();

        let result = parse_workspace_actions(dir.path(), None);
        assert_eq!(result.waiting_on.len(), 1);
    }

    #[test]
    fn test_action_result_to_value() {
        let result = ActionResult::new();
        let val = result.to_value();
        assert!(val["overdue"].is_array());
        assert!(val["due_today"].is_array());
        assert!(val["due_this_week"].is_array());
        assert!(val["waiting_on"].is_array());
    }
}
