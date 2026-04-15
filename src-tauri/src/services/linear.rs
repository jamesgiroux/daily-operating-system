//! Push-to-Linear service (DOS-51).
//!
//! Pushes a DailyOS action to Linear as a new issue, records the link,
//! updates action status, and emits a signal.

use rusqlite::params;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::linear::client::LinearClient;
use crate::state::AppState;

/// Result returned to the frontend after a successful push.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinearPushResult {
    pub identifier: String,
    pub url: String,
}

/// Push a DailyOS action to Linear as a new issue.
///
/// 1. Validates the action exists and is in a pushable status (pending/suggested).
/// 2. Checks it hasn't already been pushed.
/// 3. Builds description from action context and entity info.
/// 4. Creates the issue in Linear.
/// 5. Records the link in action_linear_links.
/// 6. Emits `action_pushed_to_linear` signal.
pub async fn push_action_to_linear(
    state: &AppState,
    action_id: &str,
    team_id: &str,
    project_id: Option<&str>,
) -> Result<LinearPushResult, String> {
    // 1. Read action from DB
    let action = state.with_db_read(|db| {
        db.get_action_by_id(action_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Action not found: {}", action_id))
    })?;

    // Validate pushable status
    match action.status.as_str() {
        "backlog" | "unstarted" => {}
        other => {
            return Err(format!(
                "Action cannot be pushed: status is '{}' (must be pending or suggested)",
                other
            ));
        }
    }

    // 2. Check if already pushed
    let already_pushed = state.with_db_read(|db| {
        let exists: bool = db
            .conn_ref()
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM action_linear_links WHERE action_id = ?1)",
                params![action_id],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        Ok(exists)
    })?;

    if already_pushed {
        return Err("Action has already been pushed to Linear".to_string());
    }

    // 3. Get Linear API key and build client
    let api_key = state
        .config
        .read()
        .as_ref()
        .and_then(|c| c.linear.api_key.clone())
        .ok_or("No Linear API key configured")?;

    let client = LinearClient::new(&api_key);

    // 4. Resolve project_id: if not provided, check if action's account has a linked Linear project
    let resolved_project_id: Option<String> = if project_id.is_some() {
        project_id.map(|s| s.to_string())
    } else if let Some(ref acct_id) = action.account_id {
        state.with_db_read(|db| {
            let pid: Option<String> = db
                .conn_ref()
                .query_row(
                    "SELECT linear_project_id FROM linear_entity_links
                     WHERE entity_id = ?1 AND entity_type = 'account' LIMIT 1",
                    params![acct_id],
                    |row| row.get(0),
                )
                .ok();
            Ok(pid)
        })?
    } else {
        None
    };

    // 5. Build description from action context
    let description = build_description(&action);

    // DailyOS priority is already Linear-compatible (0-4 integers, DOS-55)
    let linear_priority = if action.priority > 0 {
        Some(action.priority)
    } else {
        None
    };

    // 6. Create issue in Linear
    let created = client
        .create_issue(
            &action.title,
            team_id,
            Some(&description),
            resolved_project_id.as_deref(),
            linear_priority,
            action.due_date.as_deref(),
        )
        .await?;

    // 7. Store mapping + set status to 'started'
    let link_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let created_id = created.id.clone();
    let created_identifier = created.identifier.clone();
    let created_url = created.url.clone();
    let action_id_owned = action_id.to_string();
    let now_clone = now.clone();

    state.with_db_write(|db| {
        db.conn_ref()
            .execute(
                "INSERT INTO action_linear_links (id, action_id, linear_issue_id, linear_identifier, linear_url, pushed_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![link_id, action_id_owned, created_id, created_identifier, created_url, now_clone],
            )
            .map_err(|e| e.to_string())?;
        // Auto-set action status to 'started' (DOS-51: pushing = actively working)
        db.conn_ref()
            .execute(
                "UPDATE actions SET status = 'started', updated_at = ?1 WHERE id = ?2",
                params![now_clone, action_id_owned],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    })?;

    // 8. Emit signal
    let (entity_type, entity_id) = if let Some(ref acct_id) = action.account_id {
        ("account", acct_id.as_str())
    } else if let Some(ref proj_id) = action.project_id {
        ("project", proj_id.as_str())
    } else {
        ("action", action_id)
    };

    let signal_value = serde_json::json!({
        "action_id": action_id,
        "linear_identifier": created.identifier,
        "linear_url": created.url,
    })
    .to_string();

    state.with_db_write(|db| {
        let _ = crate::services::signals::emit_and_propagate(
            db,
            &state.signals.engine,
            entity_type,
            entity_id,
            "action_pushed_to_linear",
            "user_action",
            Some(&signal_value),
            0.9,
        );
        Ok(())
    })?;

    Ok(LinearPushResult {
        identifier: created.identifier,
        url: created.url,
    })
}

/// Build a markdown description for the Linear issue from action context.
fn build_description(action: &crate::db::DbAction) -> String {
    let mut parts = Vec::new();

    if let Some(ref ctx) = action.context {
        if !ctx.is_empty() {
            parts.push(ctx.clone());
        }
    }

    if let Some(ref account_name) = action.account_name {
        parts.push(format!("**Account:** {}", account_name));
    }

    if let Some(ref source_label) = action.source_label {
        parts.push(format!("**Source:** {}", source_label));
    }

    parts.push("*Pushed from DailyOS*".to_string());

    parts.join("\n\n")
}
