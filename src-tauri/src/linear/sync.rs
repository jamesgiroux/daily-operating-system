//! SQLite upsert functions for Linear data.

use crate::db::ActionDb;
use crate::linear::client::{LinearIssue, LinearProject};
use crate::state::AppState;

/// Upsert Linear issues into the database and emit signals for state changes.
pub fn upsert_issues(_state: &AppState, issues: &[LinearIssue]) -> Result<(), String> {
    let db = ActionDb::open().map_err(|e| format!("DB open failed: {e}"))?;
    let conn = db.conn_ref();

    for issue in issues {
        // Capture old state for signal comparison
        let old_state_type: Option<String> = conn
            .query_row(
                "SELECT state_type FROM linear_issues WHERE id = ?1",
                [&issue.id],
                |row| row.get(0),
            )
            .ok();

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

        // Look up entity link for this issue's project
        let entity_link: Option<(String, String)> = issue.project_id.as_ref().and_then(|pid| {
            conn.query_row(
                "SELECT entity_id, entity_type FROM linear_entity_links WHERE linear_project_id = ?1 LIMIT 1",
                [pid],
                |row| Ok((row.get(0)?, row.get(1)?)),
            ).ok()
        });

        if let Some((entity_id, entity_type)) = &entity_link {
            let new_state = issue.state_type.as_deref();
            let value = serde_json::json!({
                "identifier": issue.identifier,
                "title": issue.title,
            })
            .to_string();

            // Signal: issue completed
            if new_state == Some("completed") && old_state_type.as_deref() != Some("completed") {
                let _ = crate::signals::bus::emit_signal(
                    &db,
                    entity_type,
                    entity_id,
                    "linear_issue_completed",
                    "linear",
                    Some(&value),
                    0.7,
                );
            }

            // Signal: issue blocked
            if let Some(state_name) = &issue.state_name {
                if state_name.to_lowercase().contains("blocked")
                    && old_state_type.as_deref() != new_state
                {
                    let _ = crate::signals::bus::emit_signal(
                        &db,
                        entity_type,
                        entity_id,
                        "linear_issue_blocked",
                        "linear",
                        Some(&value),
                        0.6,
                    );
                }
            }

            // Signal: issue overdue
            if let Some(due) = &issue.due_date {
                let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                if due < &today && new_state != Some("completed") && new_state != Some("cancelled")
                {
                    let _ = crate::signals::bus::emit_signal(
                        &db,
                        entity_type,
                        entity_id,
                        "linear_issue_overdue",
                        "linear",
                        Some(&value),
                        0.5,
                    );
                }
            }
        }
    }

    Ok(())
}

/// Upsert Linear projects into the database.
pub fn upsert_projects(_state: &AppState, projects: &[LinearProject]) -> Result<(), String> {
    let db = ActionDb::open().map_err(|e| format!("DB open failed: {e}"))?;
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
