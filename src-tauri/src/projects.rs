//! Project workspace file I/O (I50 / ADR-0047).
//!
//! Each project gets a directory under `Projects/` in the workspace:
//!   Projects/{Name}/dashboard.json  -- canonical data (app + external tools write here)
//!   Projects/{Name}/dashboard.md    -- rich artifact (generated from JSON + SQLite)
//!
//! Three-way sync (ADR-0047):
//!   App edit -> writes dashboard.json -> syncs to SQLite -> regenerates dashboard.md
//!   External edit to JSON -> detected by startup scan -> syncs to SQLite
//!   External edit to markdown -> no auto-reconcile (markdown is generated)

use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::db::{ActionDb, DbProject};
use crate::util::slugify;

// =============================================================================
// JSON Schema
// =============================================================================

/// JSON schema for project dashboard.json files.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectJson {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default = "default_entity_type")]
    pub entity_type: String,
    pub structured: ProjectStructured,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub milestones: Vec<ProjectMilestone>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub custom_sections: Vec<serde_json::Value>,
}

fn default_version() -> u32 {
    1
}
fn default_entity_type() -> String {
    "project".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectStructured {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub milestone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectMilestone {
    pub name: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

// =============================================================================
// Filesystem I/O
// =============================================================================

/// Resolve the directory for a project's workspace files (I70: sanitized name).
pub fn project_dir(workspace: &Path, name: &str) -> PathBuf {
    crate::entity_io::entity_dir(workspace, "Projects", name)
}

/// Write `dashboard.json` for a project.
///
/// Merges structured DB fields with narrative JSON fields. If a JSON file
/// already exists, narrative fields (description, milestones, notes) are
/// preserved and only structured fields are updated from the DB.
pub fn write_project_json(
    workspace: &Path,
    project: &DbProject,
    existing_json: Option<&ProjectJson>,
    _db: &ActionDb,
) -> Result<(), String> {
    let dir = project_dir(workspace, &project.name);

    let json = ProjectJson {
        version: 1,
        entity_type: "project".to_string(),
        structured: ProjectStructured {
            status: Some(project.status.clone()),
            milestone: project.milestone.clone(),
            owner: project.owner.clone(),
            target_date: project.target_date.clone(),
        },
        description: existing_json.and_then(|j| j.description.clone()),
        milestones: existing_json
            .map(|j| j.milestones.clone())
            .unwrap_or_default(),
        notes: existing_json.and_then(|j| j.notes.clone()),
        custom_sections: existing_json
            .map(|j| j.custom_sections.clone())
            .unwrap_or_default(),
    };

    crate::entity_io::write_entity_json(&dir, "dashboard.json", &json)
}

/// Write `dashboard.md` for a project (generated artifact).
///
/// Combines structured data from SQLite with narrative data from JSON
/// and auto-generated sections from meeting/action/people history.
pub fn write_project_markdown(
    workspace: &Path,
    project: &DbProject,
    json: Option<&ProjectJson>,
    db: &ActionDb,
) -> Result<(), String> {
    let dir = project_dir(workspace, &project.name);
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create {}: {}", dir.display(), e))?;

    let mut md = String::new();

    // Header
    md.push_str(&format!("# {}\n\n", project.name));

    // Status badge
    let status_emoji = match project.status.as_str() {
        "active" => "\u{1f7e2}",   // green circle
        "on_hold" => "\u{1f7e1}",  // yellow circle
        "completed" => "\u{2705}", // check mark
        "archived" => "\u{1f4e6}", // package
        _ => "\u{26aa}",           // white circle
    };
    md.push_str(&format!(
        "**Status:** {} {}  \n",
        status_emoji, project.status
    ));

    if let Some(ref milestone) = project.milestone {
        md.push_str(&format!("**Milestone:** {}  \n", milestone));
    }
    if let Some(ref owner) = project.owner {
        md.push_str(&format!("**Owner:** {}  \n", owner));
    }
    if let Some(ref target_date) = project.target_date {
        md.push_str(&format!("**Target Date:** {}  \n", target_date));
    }
    md.push('\n');

    // Description (from JSON)
    if let Some(desc) = json.and_then(|j| j.description.as_ref()) {
        if !desc.is_empty() {
            md.push_str("## Description\n\n");
            md.push_str(desc);
            md.push_str("\n\n");
        }
    }

    // Milestones (from JSON)
    if let Some(milestones) = json.map(|j| &j.milestones) {
        if !milestones.is_empty() {
            md.push_str("## Milestones\n\n");
            for m in milestones.iter() {
                let badge = match m.status.as_str() {
                    "completed" => "\u{2705}",
                    "in_progress" => "\u{1f504}",
                    "planned" => "\u{1f4cb}",
                    _ => "\u{2022}",
                };
                md.push_str(&format!("- {} **{}** \u{2014} {}", badge, m.name, m.status));
                if let Some(ref target) = m.target_date {
                    md.push_str(&format!(" (target: {})", target));
                }
                if let Some(ref notes) = m.notes {
                    md.push_str(&format!(" \u{2014} {}", notes));
                }
                md.push('\n');
            }
            md.push('\n');
        }
    }

    // Notes (from JSON)
    if let Some(notes) = json.and_then(|j| j.notes.as_ref()) {
        if !notes.is_empty() {
            md.push_str("## Notes\n\n");
            md.push_str(notes);
            md.push_str("\n\n");
        }
    }

    // === Intelligence sections (I134 — from intelligence.json) ===

    let intel_dir = project_dir(workspace, &project.name);
    if let Ok(intel) = crate::entity_intel::read_intelligence_json(&intel_dir) {
        let intel_md = crate::entity_intel::format_intelligence_markdown(&intel);
        if !intel_md.is_empty() {
            md.push_str(&intel_md);
        }
    }

    // === Auto-generated sections below ===

    md.push_str("<!-- auto-generated -->\n");

    // Recent Meetings
    md.push_str("## Recent Meetings\n\n");
    match db.get_meetings_for_project(&project.id, 10) {
        Ok(meetings) if !meetings.is_empty() => {
            for m in &meetings {
                md.push_str(&format!(
                    "- **{}** \u{2014} {} ({})\n",
                    m.start_time.split('T').next().unwrap_or(&m.start_time),
                    m.title,
                    m.meeting_type,
                ));
            }
            md.push('\n');
        }
        _ => {
            md.push_str("_No meetings recorded yet._\n\n");
        }
    }

    // Open Actions
    md.push_str("## Open Actions\n\n");
    match db.get_project_actions(&project.id) {
        Ok(actions) if !actions.is_empty() => {
            for a in &actions {
                let due = a
                    .due_date
                    .as_deref()
                    .map(|d| format!(" (due {})", d))
                    .unwrap_or_default();
                md.push_str(&format!("- [{}] **{}**{}\n", a.priority, a.title, due,));
            }
            md.push('\n');
        }
        _ => {
            md.push_str("_No open actions._\n\n");
        }
    }

    // Team
    md.push_str("## Team\n\n");
    match db.get_people_for_entity(&project.id) {
        Ok(people) if !people.is_empty() => {
            for p in &people {
                let role_part = p
                    .role
                    .as_deref()
                    .map(|r| format!(" \u{2014} {}", r))
                    .unwrap_or_default();
                md.push_str(&format!("- **{}**{}", p.name, role_part));
                if let Some(ref org) = p.organization {
                    md.push_str(&format!(" ({})", org));
                }
                md.push('\n');
            }
            md.push('\n');
        }
        _ => {
            md.push_str("_No people linked yet._\n\n");
        }
    }

    // Activity Signals
    md.push_str("## Activity Signals\n\n");
    match db.get_project_signals(&project.id) {
        Ok(signals) => {
            md.push_str(&format!(
                "- **30-day meetings:** {}\n",
                signals.meeting_frequency_30d
            ));
            md.push_str(&format!(
                "- **90-day meetings:** {}\n",
                signals.meeting_frequency_90d
            ));
            md.push_str(&format!("- **Temperature:** {}\n", signals.temperature));
            md.push_str(&format!("- **Trend:** {}\n", signals.trend));
            if let Some(ref last) = signals.last_meeting {
                md.push_str(&format!(
                    "- **Last meeting:** {}\n",
                    last.split('T').next().unwrap_or(last)
                ));
            }
            md.push('\n');
        }
        Err(_) => {
            md.push_str("_No signal data available._\n\n");
        }
    }

    let path = dir.join("dashboard.md");
    crate::util::atomic_write_str(&path, &md).map_err(|e| format!("Write error: {}", e))?;

    Ok(())
}

// =============================================================================
// Read
// =============================================================================

/// Result of reading a project dashboard.json file.
pub struct ReadProjectResult {
    pub project: DbProject,
    pub json: ProjectJson,
}

/// Read a dashboard.json file and convert to DbProject + narrative fields.
pub fn read_project_json(path: &Path) -> Result<ReadProjectResult, String> {
    let project_dir = path.parent().ok_or("No parent dir")?;
    let json: ProjectJson = crate::entity_io::read_entity_json(
        project_dir,
        path.file_name().and_then(|n| n.to_str()).unwrap_or("dashboard.json"),
    )?;

    let name = project_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown")
        .to_string();

    let id = slugify(&name);

    let updated_at = crate::entity_io::file_updated_at(path);

    let tracker_path = path.parent().and_then(|p| {
        // Build relative path like "Projects/Widget v2"
        let projects_parent = p.parent()?;
        let dir_name = projects_parent.file_name()?.to_str()?;
        let project_name = p.file_name()?.to_str()?;
        Some(format!("{}/{}", dir_name, project_name))
    });

    let status = json
        .structured
        .status
        .clone()
        .unwrap_or_else(|| "active".to_string());

    Ok(ReadProjectResult {
        project: DbProject {
            id,
            name,
            status,
            milestone: json.structured.milestone.clone(),
            owner: json.structured.owner.clone(),
            target_date: json.structured.target_date.clone(),
            tracker_path,
            updated_at,
            archived: false,
        },
        json,
    })
}

// =============================================================================
// Sync
// =============================================================================

/// Startup scan: sync all Projects/*/dashboard.json files to SQLite.
///
/// For each file: compare file mtime against `projects.updated_at` in SQLite.
/// If file is newer: parse JSON, update SQLite, regenerate dashboard.md.
/// If SQLite is newer: regenerate dashboard.json + dashboard.md from SQLite.
///
/// Returns the number of projects synced.
pub fn sync_projects_from_workspace(workspace: &Path, db: &ActionDb) -> Result<usize, String> {
    let projects_dir = workspace.join("Projects");
    let mut synced = 0;

    // Scan existing JSON files in Projects/
    let entries = if projects_dir.exists() {
        std::fs::read_dir(&projects_dir)
            .map_err(|e| format!("Failed to read Projects/: {}", e))?
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        // Skip non-directories
        if !entry.path().is_dir() {
            continue;
        }

        // Skip system/hidden folders (e.g. _archive, .DS_Store)
        let dir_name = entry.file_name();
        let name_str = dir_name.to_string_lossy();
        if name_str.starts_with('_') || name_str.starts_with('.') {
            continue;
        }

        let json_path = entry.path().join("dashboard.json");
        if !json_path.exists() {
            // Project dir exists but no JSON file
            let name = name_str;
            if let Ok(Some(db_project)) = db.get_project_by_name(&name) {
                // Already in SQLite — generate files from DB
                let _ = write_project_json(workspace, &db_project, None, db);
                let _ = write_project_markdown(workspace, &db_project, None, db);
                synced += 1;
            } else {
                // New folder discovery — bootstrap minimal record from folder name
                let now = Utc::now().to_rfc3339();
                let id = slugify(&name);
                let new_project = DbProject {
                    id,
                    name: name.to_string(),
                    status: "active".to_string(),
                    milestone: None,
                    owner: None,
                    target_date: None,
                    tracker_path: Some(format!("Projects/{}", name)),
                    updated_at: now,
                    archived: false,
                };
                if db.upsert_project(&new_project).is_ok() {
                    let _ = write_project_json(workspace, &new_project, None, db);
                    let _ = write_project_markdown(workspace, &new_project, None, db);
                    log::info!("Bootstrapped project '{}' from existing folder", name);
                    synced += 1;
                }
            }
            continue;
        }

        match read_project_json(&json_path) {
            Ok(ReadProjectResult {
                project: file_project,
                json,
            }) => {
                match db.get_project(&file_project.id) {
                    Ok(Some(db_project)) => {
                        if file_project.updated_at > db_project.updated_at {
                            // File is newer -- update SQLite, regen markdown
                            let _ = db.upsert_project(&file_project);
                            let _ =
                                write_project_markdown(workspace, &file_project, Some(&json), db);
                            synced += 1;
                        } else if db_project.updated_at > file_project.updated_at {
                            // SQLite is newer -- regen both files
                            let _ = write_project_json(workspace, &db_project, Some(&json), db);
                            let _ = write_project_markdown(workspace, &db_project, Some(&json), db);
                            synced += 1;
                        }
                    }
                    Ok(None) => {
                        // New project from file -- insert to SQLite
                        let _ = db.upsert_project(&file_project);
                        let _ = write_project_markdown(workspace, &file_project, Some(&json), db);
                        synced += 1;
                    }
                    Err(_) => continue,
                }
            }
            Err(e) => {
                log::warn!("Failed to read {}: {}", json_path.display(), e);
                continue;
            }
        }
    }

    // Also check: SQLite projects that have no workspace dir yet
    if let Ok(all_projects) = db.get_all_projects() {
        for project in &all_projects {
            let dir = project_dir(workspace, &project.name);
            if !dir.exists() {
                let _ = write_project_json(workspace, project, None, db);
                let _ = write_project_markdown(workspace, project, None, db);
                synced += 1;
            }
        }
    }

    Ok(synced)
}

// =============================================================================
// Content Indexing (I138 — parallel to account content index)
// =============================================================================

/// Sync the content index for a single project. Compares filesystem against DB,
/// adds new files, updates changed files, removes deleted files.
///
/// Delegates to the entity-generic `sync_content_index_for_entity()`.
/// Returns `(added, updated, removed)` counts.
pub fn sync_content_index_for_project(
    workspace: &Path,
    db: &ActionDb,
    project: &DbProject,
) -> Result<(usize, usize, usize), String> {
    let dir = project_dir(workspace, &project.name);
    crate::entity_io::sync_content_index_for_entity(db, workspace, &project.id, "project", &dir)
}

/// Sync content indexes for all projects. Returns total files indexed.
pub fn sync_all_project_content_indexes(workspace: &Path, db: &ActionDb) -> Result<usize, String> {
    let projects = db
        .get_all_projects()
        .map_err(|e| format!("DB error: {}", e))?;
    let mut total = 0;

    for project in &projects {
        match sync_content_index_for_project(workspace, db, project) {
            Ok((added, updated, _removed)) => {
                total += added + updated;
            }
            Err(e) => {
                log::warn!(
                    "Content index sync failed for project '{}': {}",
                    project.name,
                    e
                );
            }
        }
    }

    Ok(total)
}

// =============================================================================
// Enrichment (I50 / ADR-0047) — LEGACY, superseded by entity_intel for I138
// =============================================================================

/// Parse Claude's enrichment response into a description string.
///
/// Expected format:
/// ```text
/// ENRICHMENT
/// DESCRIPTION: one-paragraph project description
/// END_ENRICHMENT
/// ```
pub fn parse_project_enrichment_response(response: &str) -> Option<String> {
    let mut in_block = false;
    let mut description = None;

    for line in response.lines() {
        let trimmed = line.trim();

        if trimmed == "ENRICHMENT" {
            in_block = true;
            continue;
        }
        if trimmed == "END_ENRICHMENT" {
            break;
        }

        if !in_block {
            continue;
        }

        if let Some(val) = trimmed.strip_prefix("DESCRIPTION:") {
            description = Some(val.trim().to_string());
        }
    }

    description
}

/// Build the Claude Code prompt for project enrichment.
pub fn enrichment_prompt(project_name: &str) -> String {
    format!(
        "Research the project or product \"{name}\". Use web search to find current information. \
         Return ONLY the structured block below \u{2014} no other text.\n\n\
         ENRICHMENT\n\
         DESCRIPTION: <one paragraph describing what this project/product is about>\n\
         END_ENRICHMENT",
        name = project_name
    )
}

/// Enrich a project via Claude Code websearch.
///
/// Calls Claude Code with a research prompt, parses the structured response,
/// updates dashboard.json, SQLite, and dashboard.md.
///
/// Returns the enriched description on success.
pub fn enrich_project(
    workspace: &Path,
    db: &ActionDb,
    project_id: &str,
    pty: &crate::pty::PtyManager,
) -> Result<String, String> {
    let project = db
        .get_project(project_id)
        .map_err(|e| format!("DB error: {}", e))?
        .ok_or_else(|| format!("Project {} not found", project_id))?;

    let prompt = enrichment_prompt(&project.name);
    let output = pty
        .spawn_claude(workspace, &prompt)
        .map_err(|e| format!("Claude Code error: {}", e))?;

    let description = parse_project_enrichment_response(&output.stdout)
        .ok_or("Could not parse enrichment response \u{2014} no ENRICHMENT block found")?;

    // Read existing JSON to preserve other narrative fields
    let json_path = project_dir(workspace, &project.name).join("dashboard.json");
    let mut json = if json_path.exists() {
        read_project_json(&json_path)
            .map(|r| r.json)
            .unwrap_or_else(|_| default_project_json(&project))
    } else {
        default_project_json(&project)
    };

    json.description = Some(description.clone());

    // Write JSON + markdown
    write_project_json(workspace, &project, Some(&json), db)?;
    write_project_markdown(workspace, &project, Some(&json), db)?;

    log::info!(
        "Enriched project '{}' via Claude Code websearch",
        project.name
    );
    Ok(description)
}

/// Create a minimal ProjectJson from a DbProject (no narrative fields).
pub fn default_project_json(project: &DbProject) -> ProjectJson {
    ProjectJson {
        version: 1,
        entity_type: "project".to_string(),
        structured: ProjectStructured {
            status: Some(project.status.clone()),
            milestone: project.milestone.clone(),
            owner: project.owner.clone(),
            target_date: project.target_date.clone(),
        },
        description: None,
        milestones: Vec::new(),
        notes: None,
        custom_sections: Vec::new(),
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> ActionDb {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("test.db");
        std::mem::forget(dir);
        ActionDb::open_at(path).expect("open")
    }

    fn sample_project(name: &str) -> DbProject {
        let now = Utc::now().to_rfc3339();
        DbProject {
            id: slugify(name),
            name: name.to_string(),
            status: "active".to_string(),
            milestone: Some("Beta Launch".to_string()),
            owner: Some("Alice".to_string()),
            target_date: Some("2026-06-01".to_string()),
            tracker_path: Some(format!("Projects/{}", name)),
            updated_at: now,
            archived: false,
        }
    }

    #[test]
    fn test_write_and_read_json() {
        let dir = tempfile::tempdir().expect("tempdir");
        let workspace = dir.path();
        let db = test_db();
        let project = sample_project("Widget v2");

        write_project_json(workspace, &project, None, &db).unwrap();

        let json_path = workspace.join("Projects/Widget v2/dashboard.json");
        assert!(json_path.exists());

        let result = read_project_json(&json_path).unwrap();
        assert_eq!(result.project.id, "widget-v2");
        assert_eq!(result.project.name, "Widget v2");
        assert_eq!(result.project.status, "active");
        assert_eq!(result.project.milestone, Some("Beta Launch".to_string()));
    }

    #[test]
    fn test_write_markdown() {
        let dir = tempfile::tempdir().expect("tempdir");
        let workspace = dir.path();
        let db = test_db();
        let project = sample_project("Widget v2");
        db.upsert_project(&project).unwrap();

        let json = ProjectJson {
            version: 1,
            entity_type: "project".to_string(),
            structured: ProjectStructured {
                status: Some("active".to_string()),
                milestone: Some("Beta Launch".to_string()),
                owner: Some("Alice".to_string()),
                target_date: Some("2026-06-01".to_string()),
            },
            description: Some("A next-gen widget platform.".to_string()),
            milestones: vec![ProjectMilestone {
                name: "Alpha".to_string(),
                status: "completed".to_string(),
                target_date: None,
                notes: None,
            }],
            notes: Some("High priority.".to_string()),
            custom_sections: vec![],
        };

        write_project_markdown(workspace, &project, Some(&json), &db).unwrap();

        let md_path = workspace.join("Projects/Widget v2/dashboard.md");
        assert!(md_path.exists());

        let content = std::fs::read_to_string(md_path).unwrap();
        assert!(content.contains("# Widget v2"));
        assert!(content.contains("\u{1f7e2}"));
        assert!(content.contains("A next-gen widget platform."));
        assert!(content.contains("Alpha"));
        assert!(content.contains("High priority."));
    }

    #[test]
    fn test_sync_from_workspace() {
        let dir = tempfile::tempdir().expect("tempdir");
        let workspace = dir.path();
        let db = test_db();

        let project = sample_project("Gadget");
        db.upsert_project(&project).unwrap();

        let synced = sync_projects_from_workspace(workspace, &db).unwrap();
        assert_eq!(synced, 1);

        let json_path = workspace.join("Projects/Gadget/dashboard.json");
        assert!(json_path.exists());
    }

    #[test]
    fn test_sync_picks_up_new_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let workspace = dir.path();
        let db = test_db();

        let proj_dir = workspace.join("Projects/New Proj");
        std::fs::create_dir_all(&proj_dir).unwrap();
        let json_content = serde_json::json!({
            "version": 1,
            "entityType": "project",
            "structured": {
                "status": "active",
                "milestone": "MVP"
            }
        });
        std::fs::write(
            proj_dir.join("dashboard.json"),
            serde_json::to_string_pretty(&json_content).unwrap(),
        )
        .unwrap();

        let synced = sync_projects_from_workspace(workspace, &db).unwrap();
        assert_eq!(synced, 1);

        let proj = db.get_project("new-proj").unwrap();
        assert!(proj.is_some());
        let proj = proj.unwrap();
        assert_eq!(proj.name, "New Proj");
        assert_eq!(proj.status, "active");
    }

    #[test]
    fn test_preserves_narrative_on_structured_update() {
        let dir = tempfile::tempdir().expect("tempdir");
        let workspace = dir.path();
        let db = test_db();
        let project = sample_project("Gamma Proj");

        let existing = ProjectJson {
            version: 1,
            entity_type: "project".to_string(),
            structured: ProjectStructured {
                status: Some("active".to_string()),
                milestone: None,
                owner: None,
                target_date: None,
            },
            description: Some("Important context.".to_string()),
            milestones: vec![],
            notes: Some("Don't lose these notes.".to_string()),
            custom_sections: vec![],
        };

        write_project_json(workspace, &project, Some(&existing), &db).unwrap();

        let json_path = workspace.join("Projects/Gamma Proj/dashboard.json");
        let result = read_project_json(&json_path).unwrap();
        assert_eq!(
            result.json.description,
            Some("Important context.".to_string())
        );
        assert_eq!(
            result.json.notes,
            Some("Don't lose these notes.".to_string())
        );
    }

    #[test]
    fn test_parse_enrichment_response() {
        let response = "\
Some preamble text

ENRICHMENT
DESCRIPTION: Widget v2 is a next-generation platform for enterprise widgets.
END_ENRICHMENT

Trailing text";

        let desc = parse_project_enrichment_response(response).unwrap();
        assert_eq!(
            desc,
            "Widget v2 is a next-generation platform for enterprise widgets."
        );
    }

    #[test]
    fn test_parse_enrichment_response_missing() {
        let response = "No enrichment block here.";
        assert!(parse_project_enrichment_response(response).is_none());
    }

    #[test]
    fn test_sync_bootstraps_from_folder_name() {
        let dir = tempfile::tempdir().expect("tempdir");
        let workspace = dir.path();
        let db = test_db();

        // Create project directories with NO dashboard.json and NO SQLite record
        let proj1 = workspace.join("Projects/Widget v2");
        let proj2 = workspace.join("Projects/Internal Tooling");
        std::fs::create_dir_all(&proj1).unwrap();
        std::fs::create_dir_all(&proj2).unwrap();

        // Drop existing content in one
        std::fs::write(proj1.join("spec.md"), "# Spec\nRequirements here").unwrap();

        let synced = sync_projects_from_workspace(workspace, &db).unwrap();
        assert_eq!(synced, 2);

        // Verify SQLite records were created with sensible defaults
        let widget = db.get_project("widget-v2").unwrap();
        assert!(widget.is_some());
        let widget = widget.unwrap();
        assert_eq!(widget.name, "Widget v2");
        assert_eq!(widget.status, "active");
        assert_eq!(widget.tracker_path, Some("Projects/Widget v2".to_string()));

        let tooling = db.get_project("internal-tooling").unwrap();
        assert!(tooling.is_some());

        // Verify dashboard files were created
        assert!(proj1.join("dashboard.json").exists());
        assert!(proj1.join("dashboard.md").exists());

        // Verify existing files were NOT touched
        let spec = std::fs::read_to_string(proj1.join("spec.md")).unwrap();
        assert!(spec.contains("Requirements here"));

        // Verify entity bridge fired
        let entity = db.get_entity("widget-v2").unwrap();
        assert!(entity.is_some());
    }

    #[test]
    fn test_sync_bootstrap_no_duplicates() {
        let dir = tempfile::tempdir().expect("tempdir");
        let workspace = dir.path();
        let db = test_db();

        std::fs::create_dir_all(workspace.join("Projects/New Thing")).unwrap();

        let synced1 = sync_projects_from_workspace(workspace, &db).unwrap();
        assert_eq!(synced1, 1);

        // Second sync: may re-sync due to timestamp harmonization, but must not duplicate
        let _synced2 = sync_projects_from_workspace(workspace, &db).unwrap();

        let all = db.get_all_projects().unwrap();
        let count = all.iter().filter(|p| p.name == "New Thing").count();
        assert_eq!(count, 1, "bootstrap must not create duplicates on re-sync");
    }
}
