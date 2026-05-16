//! Push-to-Linear service.
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinearActorScope {
    User,
    Agent,
    System,
}

impl LinearActorScope {
    pub fn from_wire(value: Option<&str>) -> Self {
        match value.unwrap_or("user").trim().to_ascii_lowercase().as_str() {
            "agent" | "mcp" => Self::Agent,
            "system" => Self::System,
            _ => Self::User,
        }
    }

    fn drops_restricted(self) -> bool {
        matches!(self, Self::Agent)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LinearEntityRef {
    pub kind: String,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LinearEntityIssue {
    pub id: String,
    pub identifier: Option<String>,
    pub title: String,
    pub state_name: Option<String>,
    pub state_type: Option<String>,
    pub state_group: String,
    pub priority: Option<i32>,
    pub priority_label: Option<String>,
    pub project_id: Option<String>,
    pub project_name: Option<String>,
    pub assignee_name: Option<String>,
    pub due_date: Option<String>,
    pub url: Option<String>,
    pub source_ref: String,
    pub subject_ref: LinearEntityRef,
    pub claim_type: String,
    pub source_asof: Option<String>,
    pub trust_band: String,
    pub source_lifecycle_state: String,
    pub redacted: bool,
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
    ctx: &crate::services::context::ServiceContext<'_>,
    state: &AppState,
    action_id: &str,
    team_id: &str,
    project_id: Option<&str>,
    title_override: Option<&str>,
) -> Result<LinearPushResult, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    // 1. Read action from DB
    let action = state.with_db(|db| {
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
    let already_pushed = state.with_db(|db| {
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
        state.with_db(|db| {
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

    // DailyOS priority is already Linear-compatible (0-4 integers)
    let linear_priority = if action.priority > 0 {
        Some(action.priority)
    } else {
        None
    };

    // 6. Create issue in Linear
    let created = client
        .create_issue(
            title_override.unwrap_or(&action.title),
            team_id,
            Some(&description),
            resolved_project_id.as_deref(),
            linear_priority,
            action.due_date.as_deref(),
        )
        .await?;

    // 7. Store mapping + set status to 'started'
    let link_id = Uuid::new_v4().to_string();
    let now = ctx.clock.now().to_rfc3339();
    let created_id = created.id.clone();
    let created_identifier = created.identifier.clone();
    let created_url = created.url.clone();
    let action_id_owned = action_id.to_string();
    let now_clone = now.clone();

    state.with_db(|db| {
        db.conn_ref()
            .execute(
                "INSERT INTO action_linear_links (id, action_id, linear_issue_id, linear_identifier, linear_url, pushed_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![link_id, action_id_owned, created_id, created_identifier, created_url, now_clone],
            )
            .map_err(|e| e.to_string())?;
        // Auto-set action status to 'started' (pushing = actively working)
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

    // Check if this was an AI-suggested action for Bayesian feedback
    let is_ai_suggested = action
        .source_type
        .as_deref()
        .is_some_and(|s| s.starts_with("ai") || s == "intelligence");

    state.with_db(|db| {
        if let Err(e) = crate::services::signals::emit_and_propagate(
            ctx,
            db,
            &state.signals.engine,
            entity_type,
            entity_id,
            "action_pushed_to_linear",
            "user_action",
            Some(&signal_value),
            0.9,
        ) {
            log::warn!("emit Linear push signal failed for {entity_type}:{entity_id}: {e}");
        }

        // Positive Bayesian feedback when an AI-suggested action is
        // pushed to Linear — validates the suggestion quality.
        if is_ai_suggested {
            if let Err(e) = crate::services::signals::emit_and_propagate(
                ctx,
                db,
                &state.signals.engine,
                entity_type,
                entity_id,
                "ai_suggestion_validated",
                "user_action",
                Some(&format!("{{\"action_id\":\"{}\"}}", action_id)),
                0.9,
            ) {
                log::warn!(
                    "emit AI suggestion validation signal failed for {entity_type}:{entity_id}: {e}"
                );
            }
        }

        Ok(())
    })?;

    Ok(LinearPushResult {
        identifier: created.identifier,
        url: created.url,
    })
}

pub fn get_entity_linear_issues(
    db: &crate::db::ActionDb,
    entity_type: &str,
    entity_id: &str,
    actor_scope: LinearActorScope,
) -> Result<Vec<LinearEntityIssue>, String> {
    if !matches!(entity_type, "account" | "project") {
        return Ok(Vec::new());
    }

    let mut stmt = db
        .conn_ref()
        .prepare(
            "SELECT li.id, li.identifier, li.title, li.state_name, li.state_type,
                    li.priority, li.priority_label, li.project_id, li.project_name,
                    li.assignee_name, li.due_date, li.url,
                    COALESCE(li.linear_updated_at, li.synced_at) AS source_asof,
                    EXISTS (
                        SELECT 1
                        FROM linear_entity_links restricted_link
                        JOIN accounts restricted_account
                          ON restricted_account.id = restricted_link.entity_id
                         AND restricted_link.entity_type = 'account'
                        WHERE restricted_link.linear_project_id = li.project_id
                          AND restricted_account.account_type = 'internal'
                    ) AS restricted_source
             FROM linear_issues li
             JOIN linear_entity_links lel ON lel.linear_project_id = li.project_id
             WHERE lel.entity_id = ?1
               AND lel.entity_type = ?2
               AND lower(COALESCE(li.state_type, '')) NOT IN ('cancelled', 'canceled')
             ORDER BY
               CASE
                 WHEN lower(COALESCE(li.state_name, '')) LIKE '%blocked%' THEN 2
                 WHEN COALESCE(li.state_type, '') = 'started' THEN 1
                 WHEN COALESCE(li.state_type, '') = 'completed' THEN 3
                 ELSE 0
               END ASC,
               COALESCE(li.priority, 99) ASC,
               COALESCE(li.linear_updated_at, li.synced_at) DESC
             LIMIT 50",
        )
        .map_err(|error| error.to_string())?;

    let rows = stmt
        .query_map(params![entity_id, entity_type], |row| {
            let id: String = row.get(0)?;
            let identifier: String = row.get(1)?;
            let title: String = row.get(2)?;
            let state_name: Option<String> = row.get(3)?;
            let state_type: Option<String> = row.get(4)?;
            let source_asof: Option<String> = row.get(12)?;
            let restricted_source: bool = row.get::<_, i64>(13)? != 0;
            let state_group =
                linear_issue_state_group(state_type.as_deref(), state_name.as_deref());
            let trust_band = trust_band_for_linear_issue(source_asof.as_deref());
            let redacted = restricted_source && !actor_scope.drops_restricted();

            Ok(LinearEntityIssue {
                id: if redacted {
                    format!("restricted-{id}")
                } else {
                    id.clone()
                },
                identifier: (!redacted).then_some(identifier),
                title: if redacted {
                    "Restricted Linear issue".to_string()
                } else {
                    title
                },
                state_name: if redacted { None } else { state_name },
                state_type,
                state_group,
                priority: if redacted { None } else { row.get(5)? },
                priority_label: if redacted { None } else { row.get(6)? },
                project_id: if redacted { None } else { row.get(7)? },
                project_name: if redacted { None } else { row.get(8)? },
                assignee_name: if redacted { None } else { row.get(9)? },
                due_date: if redacted { None } else { row.get(10)? },
                url: if redacted { None } else { row.get(11)? },
                source_ref: id,
                subject_ref: LinearEntityRef {
                    kind: entity_type.to_string(),
                    id: entity_id.to_string(),
                },
                claim_type: crate::services::linear_issue_signals::CLAIM_TYPE_LINEAR_ISSUE_STATE
                    .to_string(),
                source_asof,
                trust_band,
                source_lifecycle_state: if restricted_source {
                    "restricted".to_string()
                } else {
                    "active".to_string()
                },
                redacted,
            })
        })
        .map_err(|error| error.to_string())?;

    let mut issues = Vec::new();
    for row in rows {
        let issue = row.map_err(|error| error.to_string())?;
        if issue.source_lifecycle_state == "restricted" && actor_scope.drops_restricted() {
            continue;
        }
        issues.push(issue);
    }
    Ok(issues)
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

fn linear_issue_state_group(state_type: Option<&str>, state_name: Option<&str>) -> String {
    let state_name = state_name.unwrap_or("").trim().to_ascii_lowercase();
    let state_type = state_type.unwrap_or("").trim().to_ascii_lowercase();
    if state_name.contains("blocked") {
        "blocked"
    } else if state_type == "completed" || matches!(state_name.as_str(), "done" | "completed") {
        "done"
    } else if state_type == "started" || matches!(state_name.as_str(), "started" | "in progress") {
        "in_progress"
    } else {
        "open"
    }
    .to_string()
}

fn trust_band_for_linear_issue(source_asof: Option<&str>) -> String {
    let Some(source_asof) = source_asof else {
        return "needs_verification".to_string();
    };
    let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(source_asof) else {
        return "use_with_caution".to_string();
    };
    let age_days = chrono::Utc::now()
        .signed_duration_since(parsed.with_timezone(&chrono::Utc))
        .num_days();
    if age_days <= 14 {
        "likely_current"
    } else if age_days <= 45 {
        "use_with_caution"
    } else {
        "needs_verification"
    }
    .to_string()
}
