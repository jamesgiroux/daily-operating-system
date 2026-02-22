// Projects service — extracted from commands.rs
// Business logic for project detail assembly.

use std::path::Path;

use crate::commands::{MeetingSummary, ProjectChildSummary, ProjectDetailResult};
use crate::state::AppState;

/// Get full detail for a project by ID.
///
/// Loads project from DB, reads dashboard.json + intelligence.json,
/// fetches actions, meetings, people, signals, captures, and email signals.
/// I388: Also resolves parent/child hierarchy.
pub fn get_project_detail(
    project_id: &str,
    state: &AppState,
) -> Result<ProjectDetailResult, String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let project = db
        .get_project(project_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Project not found: {}", project_id))?;

    // Read narrative fields from dashboard.json + intelligence.json if they exist
    let config = state.config.read().map_err(|_| "Lock poisoned")?;
    let (description, milestones, notes, intelligence) = if let Some(ref config) = *config {
        let workspace = Path::new(&config.workspace_path);
        let project_dir = crate::projects::project_dir(workspace, &project.name);
        let json_path = project_dir.join("dashboard.json");
        let (desc, ms, nt) = if json_path.exists() {
            match crate::projects::read_project_json(&json_path) {
                Ok(result) => (
                    result.json.description,
                    result.json.milestones,
                    result.json.notes,
                ),
                Err(_) => (None, Vec::new(), None),
            }
        } else {
            (None, Vec::new(), None)
        };
        let intel = crate::intelligence::read_intelligence_json(&project_dir).ok();
        (desc, ms, nt, intel)
    } else {
        (None, Vec::new(), None, None)
    };
    drop(config);

    let open_actions = db
        .get_project_actions(project_id)
        .map_err(|e| e.to_string())?;

    let recent_meetings = db
        .get_meetings_for_project(project_id, 10)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|m| MeetingSummary {
            id: m.id,
            title: m.title,
            start_time: m.start_time,
            meeting_type: m.meeting_type,
        })
        .collect();

    let linked_people = db.get_people_for_entity(project_id).unwrap_or_default();

    let signals = db.get_project_signals(project_id).ok();

    // Get captures linked to project meetings
    let recent_captures = db
        .get_captures_for_project(project_id, 90)
        .unwrap_or_default();
    let recent_email_signals = db
        .list_recent_email_signals_for_entity(project_id, 12)
        .unwrap_or_default();

    // I388: Resolve parent name for child projects, children for parent projects
    let parent_name = project
        .parent_id
        .as_ref()
        .and_then(|pid| db.get_project(pid).ok().flatten().map(|p| p.name));

    let child_projects = db.get_child_projects(&project.id).unwrap_or_default();
    let parent_aggregate = if !child_projects.is_empty() {
        db.get_project_parent_aggregate(&project.id).ok()
    } else {
        None
    };
    let children: Vec<ProjectChildSummary> = child_projects
        .iter()
        .map(|child| {
            let open_action_count = db
                .get_project_actions(&child.id)
                .map(|a| a.len())
                .unwrap_or(0);
            ProjectChildSummary {
                id: child.id.clone(),
                name: child.name.clone(),
                status: child.status.clone(),
                milestone: child.milestone.clone(),
                open_action_count,
            }
        })
        .collect();

    Ok(ProjectDetailResult {
        id: project.id,
        name: project.name,
        status: project.status,
        milestone: project.milestone,
        owner: project.owner,
        target_date: project.target_date,
        description,
        milestones,
        notes,
        open_actions,
        recent_meetings,
        linked_people,
        signals,
        recent_captures,
        recent_email_signals,
        archived: project.archived,
        intelligence,
        parent_id: project.parent_id,
        parent_name,
        children,
        parent_aggregate,
    })
}
