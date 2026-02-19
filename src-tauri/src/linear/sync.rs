//! SQLite upsert functions for Linear data.

use crate::linear::client::{LinearIssue, LinearProject};
use crate::state::AppState;

/// Upsert Linear issues into the database.
pub fn upsert_issues(state: &AppState, issues: &[LinearIssue]) -> Result<(), String> {
    let db_guard = state.db.lock().map_err(|_| "DB lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database unavailable")?;
    let conn = db.conn_ref();

    for issue in issues {
        conn.execute(
            "INSERT OR REPLACE INTO linear_issues
             (id, identifier, title, state_name, state_type, priority, priority_label,
              project_id, project_name, due_date, url, synced_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, datetime('now'))",
            rusqlite::params![
                issue.id,
                issue.identifier,
                issue.title,
                issue.state_name,
                issue.state_type,
                issue.priority,
                issue.priority_label,
                issue.project_id,
                issue.project_name,
                issue.due_date,
                issue.url,
            ],
        )
        .map_err(|e| format!("Failed to upsert Linear issue: {}", e))?;
    }

    Ok(())
}

/// Upsert Linear projects into the database.
pub fn upsert_projects(state: &AppState, projects: &[LinearProject]) -> Result<(), String> {
    let db_guard = state.db.lock().map_err(|_| "DB lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database unavailable")?;
    let conn = db.conn_ref();

    for project in projects {
        conn.execute(
            "INSERT OR REPLACE INTO linear_projects
             (id, name, state, url, synced_at)
             VALUES (?1, ?2, ?3, ?4, datetime('now'))",
            rusqlite::params![project.id, project.name, project.state, project.url,],
        )
        .map_err(|e| format!("Failed to upsert Linear project: {}", e))?;
    }

    Ok(())
}
