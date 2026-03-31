// Projects service — extracted from commands.rs (I450)
// Business logic for project CRUD, list assembly, and workspace file management.

use std::path::Path;

use crate::commands::{MeetingSummary, ProjectChildSummary, ProjectDetailResult, ProjectListItem};
use crate::db::ActionDb;
use crate::state::AppState;

/// Get all projects with computed summary fields for the list page.
pub async fn get_projects_list(state: &AppState) -> Result<Vec<ProjectListItem>, String> {
    state
        .db_read(|db| {
            let projects = db.get_all_projects().map_err(|e| e.to_string())?;

            // Pre-compute parent names for all projects with a parent_id
            let parent_names: std::collections::HashMap<String, String> = projects
                .iter()
                .map(|p| (p.id.clone(), p.name.clone()))
                .collect();

            let items: Vec<ProjectListItem> = projects
                .into_iter()
                .map(|p| {
                    let open_action_count =
                        db.get_project_actions(&p.id).map(|a| a.len()).unwrap_or(0);
                    let days_since_last_meeting =
                        db.get_project_signals(&p.id).ok().and_then(|s| {
                            s.last_meeting.as_ref().and_then(|lm| {
                                chrono::DateTime::parse_from_rfc3339(lm).ok().map(|dt| {
                                    (chrono::Utc::now() - dt.with_timezone(&chrono::Utc)).num_days()
                                })
                            })
                        });
                    let child_count = db.get_child_projects(&p.id).map(|c| c.len()).unwrap_or(0);
                    let parent_name = p
                        .parent_id
                        .as_ref()
                        .and_then(|pid| parent_names.get(pid).cloned());
                    ProjectListItem {
                        id: p.id,
                        name: p.name,
                        status: p.status,
                        milestone: p.milestone,
                        owner: p.owner,
                        target_date: p.target_date,
                        open_action_count,
                        days_since_last_meeting,
                        is_parent: child_count > 0,
                        child_count,
                        parent_name,
                        parent_id: p.parent_id,
                        archived: p.archived,
                    }
                })
                .collect();

            Ok(items)
        })
        .await
}

/// Get child projects for a parent project (I388).
pub async fn get_child_projects_list(
    parent_id: &str,
    state: &AppState,
) -> Result<Vec<ProjectListItem>, String> {
    let parent_id = parent_id.to_string();
    state
        .db_read(move |db| {
            let children = db
                .get_child_projects(&parent_id)
                .map_err(|e| e.to_string())?;
            let parent_name = db.get_project(&parent_id).ok().flatten().map(|p| p.name);

            let items: Vec<ProjectListItem> = children
                .into_iter()
                .map(|p| {
                    let open_action_count =
                        db.get_project_actions(&p.id).map(|a| a.len()).unwrap_or(0);
                    let days_since_last_meeting =
                        db.get_project_signals(&p.id).ok().and_then(|s| {
                            s.last_meeting.as_ref().and_then(|lm| {
                                chrono::DateTime::parse_from_rfc3339(lm).ok().map(|dt| {
                                    (chrono::Utc::now() - dt.with_timezone(&chrono::Utc)).num_days()
                                })
                            })
                        });
                    let child_count = db.get_child_projects(&p.id).map(|c| c.len()).unwrap_or(0);
                    ProjectListItem {
                        id: p.id,
                        name: p.name,
                        status: p.status,
                        milestone: p.milestone,
                        owner: p.owner,
                        target_date: p.target_date,
                        open_action_count,
                        days_since_last_meeting,
                        parent_id: p.parent_id,
                        parent_name: parent_name.clone(),
                        child_count,
                        is_parent: child_count > 0,
                        archived: p.archived,
                    }
                })
                .collect();

            Ok(items)
        })
        .await
}

/// Get full detail for a project by ID.
///
/// I644: All data from DB — no filesystem reads on the detail page path.
/// Fetches actions, meetings, people, signals, captures, and email signals.
/// I388: Also resolves parent/child hierarchy.
pub async fn get_project_detail(
    project_id: &str,
    state: &AppState,
) -> Result<ProjectDetailResult, String> {
    let project_id = project_id.to_string();
    state
        .db_read(move |db| {
            let project = db
                .get_project(&project_id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("Project not found: {}", project_id))?;

            // I644: Read narrative fields from DB columns (promoted from dashboard.json).
            let description = project.description.clone();
            let milestones: Vec<crate::projects::ProjectMilestone> = project
                .milestones
                .as_ref()
                .and_then(|json| serde_json::from_str(json).ok())
                .unwrap_or_default();
            let notes = project.notes.clone();
            // I644: Intelligence from DB only — no filesystem fallback.
            let intelligence = db.get_entity_intelligence(&project_id).ok().flatten();

            let open_actions = db
                .get_project_actions(&project_id)
                .map_err(|e| e.to_string())?;

            let recent_meetings = db
                .get_meetings_for_project(&project_id, 10)
                .map_err(|e| e.to_string())?
                .into_iter()
                .map(|m| MeetingSummary {
                    id: m.id,
                    title: m.title,
                    start_time: m.start_time,
                    meeting_type: m.meeting_type,
                })
                .collect();

            let linked_people = db.get_people_for_entity(&project_id).unwrap_or_default();

            let signals = db.get_project_signals(&project_id).ok();

            // Get captures linked to project meetings
            let recent_captures = db
                .get_captures_for_project(&project_id, 90)
                .unwrap_or_default();
            let recent_email_signals = db
                .list_recent_email_signals_for_entity(&project_id, 12)
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
        })
        .await
}

/// Create a new project with workspace files.
pub async fn create_project(
    name: &str,
    parent_id: Option<String>,
    state: &AppState,
) -> Result<String, String> {
    let validated_name = crate::util::validate_entity_name(name)?;
    let id = crate::util::slugify(validated_name);
    let validated_name = validated_name.to_string();

    let config = state.config.read().map_err(|_| "Lock poisoned")?.clone();

    let id_clone = id.clone();
    let validated_name_clone = validated_name.clone();
    state
        .db_write(move |db| {
            let now = chrono::Utc::now().to_rfc3339();

            if let Ok(Some(_)) = db.get_project(&id_clone) {
                return Err(format!("Project '{}' already exists", validated_name_clone));
            }

            let project = crate::db::DbProject {
                id: id_clone.clone(),
                name: validated_name_clone.clone(),
                status: "active".to_string(),
                milestone: None,
                owner: None,
                target_date: None,
                tracker_path: Some(format!("Projects/{}", validated_name_clone)),
                parent_id,
                updated_at: now,
                ..Default::default()
            };

            db.upsert_project(&project).map_err(|e| e.to_string())?;

            if let Some(ref config) = config {
                let workspace = Path::new(&config.workspace_path);
                let project_dir = crate::projects::project_dir(workspace, &validated_name_clone);
                let _ = std::fs::create_dir_all(&project_dir);
                let _ = crate::util::bootstrap_entity_directory(
                    &project_dir,
                    &validated_name_clone,
                    "project",
                );
                let _ = crate::projects::write_project_json(workspace, &project, None, db);
                let _ = crate::projects::write_project_markdown(workspace, &project, None, db);
            }

            // Self-healing: initialize quality row for new entity (I406)
            crate::self_healing::quality::ensure_quality_row(db, &id_clone, "project");

            Ok(id_clone)
        })
        .await
}

/// Update a single structured field on a project.
pub async fn update_project_field(
    project_id: &str,
    field: &str,
    value: &str,
    state: &AppState,
) -> Result<(), String> {
    let config = state.config.read().map_err(|_| "Lock poisoned")?.clone();
    let intel_queue = state.intel_queue.clone();

    let project_id = project_id.to_string();
    let field = field.to_string();
    let value = value.to_string();
    state
        .db_write(move |db| {
            db.update_project_field(&project_id, &field, &value)
                .map_err(|e| e.to_string())?;

            let _ = crate::services::signals::emit(
                db,
                "project",
                &project_id,
                "field_updated",
                "user_edit",
                Some(&format!(
                    "{{\"field\":\"{}\",\"value\":\"{}\"}}",
                    field,
                    value.replace('"', "\\\"")
                )),
                0.8,
            );

            // Self-healing: event-driven trigger evaluation (I410)
            let _ = crate::self_healing::scheduler::evaluate_on_signal(
                db,
                &project_id,
                "project",
                &intel_queue,
            );

            // Self-healing: record user correction for enrichable fields (I409)
            if matches!(
                field.as_str(),
                "status" | "milestone" | "owner" | "target_date"
            ) {
                crate::self_healing::feedback::record_enrichment_correction(
                    db,
                    &project_id,
                    "project",
                    "clay",
                );
            }

            // Regenerate workspace files
            if let Ok(Some(project)) = db.get_project(&project_id) {
                if let Some(ref config) = config {
                    let workspace = Path::new(&config.workspace_path);
                    let json_path = crate::projects::project_dir(workspace, &project.name)
                        .join("dashboard.json");
                    let existing_json = if json_path.exists() {
                        crate::projects::read_project_json(&json_path)
                            .ok()
                            .map(|r| r.json)
                    } else {
                        None
                    };
                    let _ = crate::projects::write_project_json(
                        workspace,
                        &project,
                        existing_json.as_ref(),
                        db,
                    );
                    let _ = crate::projects::write_project_markdown(
                        workspace,
                        &project,
                        existing_json.as_ref(),
                        db,
                    );
                }
            }

            Ok(())
        })
        .await
}

/// Update the notes field on a project.
pub async fn update_project_notes(
    project_id: &str,
    notes: &str,
    state: &AppState,
) -> Result<(), String> {
    let config = state.config.read().map_err(|_| "Lock poisoned")?.clone();

    let project_id = project_id.to_string();
    let notes = notes.to_string();
    state
        .db_write(move |db| {
            let project = db
                .get_project(&project_id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("Project not found: {}", project_id))?;

            if let Some(ref config) = config {
                let workspace = Path::new(&config.workspace_path);
                let json_path =
                    crate::projects::project_dir(workspace, &project.name).join("dashboard.json");

                let mut json = if json_path.exists() {
                    crate::projects::read_project_json(&json_path)
                        .map(|r| r.json)
                        .unwrap_or_else(|_| crate::projects::default_project_json(&project))
                } else {
                    crate::projects::default_project_json(&project)
                };

                json.notes = if notes.is_empty() {
                    None
                } else {
                    Some(notes.clone())
                };

                crate::projects::write_project_json(workspace, &project, Some(&json), db)?;
                crate::projects::write_project_markdown(workspace, &project, Some(&json), db)?;

                let _ = crate::services::signals::emit(
                    db,
                    "project",
                    &project_id,
                    "field_updated",
                    "user_edit",
                    Some(&format!(
                        "{{\"field\":\"notes\",\"value\":\"{}\"}}",
                        notes
                            .chars()
                            .take(100)
                            .collect::<String>()
                            .replace('"', "\\\"")
                    )),
                    0.8,
                );
            }

            Ok(())
        })
        .await
}

/// Bulk-create projects from a list of names.
pub fn bulk_create_projects(
    db: &ActionDb,
    workspace: &Path,
    names: &[String],
) -> Result<Vec<String>, String> {
    let mut created_ids = Vec::with_capacity(names.len());

    for raw_name in names {
        let name = crate::util::validate_entity_name(raw_name)?;
        let id = crate::util::slugify(name);

        if let Ok(Some(_)) = db.get_project(&id) {
            continue;
        }

        let now = chrono::Utc::now().to_rfc3339();
        let project = crate::db::DbProject {
            id: id.clone(),
            name: name.to_string(),
            status: "active".to_string(),
            milestone: None,
            owner: None,
            target_date: None,
            tracker_path: Some(format!("Projects/{}", name)),
            updated_at: now,
            ..Default::default()
        };

        db.upsert_project(&project).map_err(|e| e.to_string())?;

        let project_dir = crate::projects::project_dir(workspace, name);
        let _ = std::fs::create_dir_all(&project_dir);
        let _ = crate::util::bootstrap_entity_directory(&project_dir, name, "project");
        let _ = crate::projects::write_project_json(workspace, &project, None, db);
        let _ = crate::projects::write_project_markdown(workspace, &project, None, db);

        created_ids.push(id);
    }

    Ok(created_ids)
}

/// Archive or restore a project with signal emission.
pub fn archive_project(db: &ActionDb, id: &str, archived: bool) -> Result<(), String> {
    db.archive_project(id, archived)
        .map_err(|e| e.to_string())?;

    let signal_type = if archived {
        "entity_archived"
    } else {
        "entity_unarchived"
    };
    let _ =
        crate::services::signals::emit(db, "project", id, signal_type, "user_action", None, 0.9);

    Ok(())
}
