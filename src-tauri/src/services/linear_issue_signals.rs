//! Linear issue signal emission for entity-scoped issue state.

use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};

use crate::db::ActionDb;
use crate::linear::client::LinearIssue;
use crate::signals::propagation::PropagationEngine;

pub const CLAIM_TYPE_LINEAR_ISSUE_STATE: &str = "linear_issue_state";
pub const SIGNAL_STATE_CHANGED_TO_IN_PROGRESS: &str = "state_changed_to_in_progress";
pub const SIGNAL_STATE_CHANGED_TO_BLOCKED: &str = "state_changed_to_blocked";
pub const SIGNAL_STATE_CHANGED_TO_DONE: &str = "state_changed_to_done";
pub const SIGNAL_ASSIGNEE_CHANGED: &str = "assignee_changed";
pub const SIGNAL_PRIORITY_CHANGED_TO_URGENT: &str = "priority_changed_to_urgent";

pub const LINEAR_ISSUE_SIGNAL_TYPES: [&str; 5] = [
    SIGNAL_STATE_CHANGED_TO_IN_PROGRESS,
    SIGNAL_STATE_CHANGED_TO_BLOCKED,
    SIGNAL_STATE_CHANGED_TO_DONE,
    SIGNAL_ASSIGNEE_CHANGED,
    SIGNAL_PRIORITY_CHANGED_TO_URGENT,
];

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PreviousLinearIssueState {
    pub state_type: Option<String>,
    pub state_name: Option<String>,
    pub priority: Option<i32>,
    pub assignee_id: Option<String>,
}

impl PreviousLinearIssueState {
    pub fn load(db: &ActionDb, issue_id: &str) -> Result<Option<Self>, String> {
        db.conn_ref()
            .query_row(
                "SELECT state_type, state_name, priority, assignee_id
                 FROM linear_issues
                 WHERE id = ?1",
                [issue_id],
                |row| {
                    Ok(Self {
                        state_type: row.get(0)?,
                        state_name: row.get(1)?,
                        priority: row.get(2)?,
                        assignee_id: row.get(3)?,
                    })
                },
            )
            .optional()
            .map_err(|error| error.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct LinearIssueSignalPayload {
    pub source_ref: String,
    pub subject_ref: SubjectSignalRef,
    pub claim_type: String,
    pub source_asof: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubjectSignalRef {
    pub kind: String,
    pub id: String,
}

pub fn emit_issue_change_signals(
    db: &ActionDb,
    engine: &PropagationEngine,
    issue: &LinearIssue,
    previous: Option<&PreviousLinearIssueState>,
    synced_at: &str,
) {
    let Some(project_id) = issue.project_id.as_deref() else {
        return;
    };
    let links = linked_entities_for_linear_project(db, project_id);
    if links.is_empty() {
        return;
    }

    let signal_types = changed_signal_types(issue, previous);
    if signal_types.is_empty() {
        return;
    }

    for (entity_id, entity_type) in links {
        let payload = LinearIssueSignalPayload {
            source_ref: issue.id.clone(),
            subject_ref: SubjectSignalRef {
                kind: entity_type.clone(),
                id: entity_id.clone(),
            },
            claim_type: CLAIM_TYPE_LINEAR_ISSUE_STATE.to_string(),
            source_asof: issue
                .updated_at
                .clone()
                .unwrap_or_else(|| synced_at.to_string()),
            from_state: previous.and_then(|state| {
                state
                    .state_name
                    .clone()
                    .or_else(|| state.state_type.clone())
            }),
            to_state: issue
                .state_name
                .clone()
                .or_else(|| issue.state_type.clone()),
            identifier: Some(issue.identifier.clone()),
            url: Some(issue.url.clone()),
        };
        let Ok(value) = serde_json::to_string(&payload) else {
            continue;
        };

        for signal_type in &signal_types {
            if let Err(error) = crate::signals::bus::emit_signal_and_propagate(
                db,
                engine,
                &entity_type,
                &entity_id,
                signal_type,
                "linear",
                Some(&value),
                confidence_for_signal(signal_type),
            ) {
                log::warn!(
                    "Linear issue signal emit failed for {entity_type}:{entity_id} {signal_type}: {error}"
                );
            }
        }
    }
}

pub fn changed_signal_types(
    issue: &LinearIssue,
    previous: Option<&PreviousLinearIssueState>,
) -> Vec<&'static str> {
    let Some(previous) = previous else {
        return Vec::new();
    };

    let state_changed =
        !same_optional_text(previous.state_type.as_deref(), issue.state_type.as_deref())
            || !same_optional_text(previous.state_name.as_deref(), issue.state_name.as_deref());
    let mut signals = Vec::new();
    if state_changed {
        if is_in_progress(issue.state_type.as_deref(), issue.state_name.as_deref()) {
            signals.push(SIGNAL_STATE_CHANGED_TO_IN_PROGRESS);
        }
        if is_blocked(issue.state_name.as_deref()) {
            signals.push(SIGNAL_STATE_CHANGED_TO_BLOCKED);
        }
        if is_done(issue.state_type.as_deref(), issue.state_name.as_deref()) {
            signals.push(SIGNAL_STATE_CHANGED_TO_DONE);
        }
    }

    if previous.assignee_id.as_deref() != issue.assignee_id.as_deref() {
        signals.push(SIGNAL_ASSIGNEE_CHANGED);
    }

    if !is_urgent(previous.priority, None)
        && is_urgent(issue.priority, issue.priority_label.as_deref())
    {
        signals.push(SIGNAL_PRIORITY_CHANGED_TO_URGENT);
    }

    signals
}

fn linked_entities_for_linear_project(db: &ActionDb, project_id: &str) -> Vec<(String, String)> {
    let Ok(mut statement) = db.conn_ref().prepare(
        "SELECT entity_id, entity_type
         FROM linear_entity_links
         WHERE linear_project_id = ?1",
    ) else {
        return Vec::new();
    };
    let Ok(rows) = statement.query_map([project_id], |row| Ok((row.get(0)?, row.get(1)?))) else {
        return Vec::new();
    };
    rows.flatten().collect()
}

fn same_optional_text(left: Option<&str>, right: Option<&str>) -> bool {
    left.map(normalized) == right.map(normalized)
}

fn normalized(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn is_in_progress(state_type: Option<&str>, state_name: Option<&str>) -> bool {
    state_type.is_some_and(|value| normalized(value) == "started")
        || state_name
            .is_some_and(|value| matches!(normalized(value).as_str(), "started" | "in progress"))
}

fn is_blocked(state_name: Option<&str>) -> bool {
    state_name.is_some_and(|value| normalized(value).contains("blocked"))
}

fn is_done(state_type: Option<&str>, state_name: Option<&str>) -> bool {
    state_type.is_some_and(|value| normalized(value) == "completed")
        || state_name
            .is_some_and(|value| matches!(normalized(value).as_str(), "done" | "completed"))
}

fn is_urgent(priority: Option<i32>, priority_label: Option<&str>) -> bool {
    priority == Some(1) || priority_label.is_some_and(|label| normalized(label) == "urgent")
}

fn confidence_for_signal(signal_type: &str) -> f64 {
    match signal_type {
        SIGNAL_STATE_CHANGED_TO_BLOCKED | SIGNAL_PRIORITY_CHANGED_TO_URGENT => 0.85,
        SIGNAL_STATE_CHANGED_TO_DONE | SIGNAL_STATE_CHANGED_TO_IN_PROGRESS => 0.8,
        SIGNAL_ASSIGNEE_CHANGED => 0.65,
        _ => 0.6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn issue(state_type: &str, state_name: &str) -> LinearIssue {
        LinearIssue {
            id: "issue-1".into(),
            identifier: "DOS-1".into(),
            title: "Synthetic issue".into(),
            state_name: Some(state_name.into()),
            state_type: Some(state_type.into()),
            priority: Some(3),
            priority_label: Some("Normal".into()),
            project_id: Some("project-1".into()),
            project_name: Some("Synthetic Project".into()),
            assignee_id: Some("person-1".into()),
            assignee_name: Some("Example Owner".into()),
            due_date: None,
            updated_at: Some("2026-05-15T12:00:00Z".into()),
            url: "https://linear.app/example/issue/DOS-1".into(),
        }
    }

    #[test]
    fn detects_requested_linear_issue_signal_types() {
        let previous = PreviousLinearIssueState {
            state_type: Some("unstarted".into()),
            state_name: Some("Todo".into()),
            priority: Some(3),
            assignee_id: Some("person-old".into()),
        };
        let mut current = issue("started", "Blocked");
        current.priority = Some(1);

        let signals = changed_signal_types(&current, Some(&previous));

        assert_eq!(
            signals,
            vec![
                SIGNAL_STATE_CHANGED_TO_IN_PROGRESS,
                SIGNAL_STATE_CHANGED_TO_BLOCKED,
                SIGNAL_ASSIGNEE_CHANGED,
                SIGNAL_PRIORITY_CHANGED_TO_URGENT,
            ]
        );
    }
}
