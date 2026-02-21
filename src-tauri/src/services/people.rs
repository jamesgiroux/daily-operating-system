// People service — extracted from commands.rs
// Business logic for person merge and delete with filesystem cleanup.

use std::path::Path;

use crate::commands::{EntitySummary, MeetingSummary, PersonDetailResult};
use crate::db::ActionDb;
use crate::state::AppState;

/// Merge two people: transfer all references from `remove_id` to `keep_id`,
/// then delete the removed person. Also cleans up filesystem directories
/// and regenerates the kept person's files.
pub fn merge_people(
    db: &ActionDb,
    state: &AppState,
    keep_id: &str,
    remove_id: &str,
) -> Result<String, String> {
    // Get removed person's info before merge (for filesystem cleanup)
    let removed = db
        .get_person(remove_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Person not found: {}", remove_id))?;

    // Perform DB merge
    db.merge_people(keep_id, remove_id)
        .map_err(|e| e.to_string())?;

    // Filesystem cleanup
    let config = state.config.read().map_err(|_| "Lock poisoned")?;
    if let Some(ref config) = *config {
        let workspace = Path::new(&config.workspace_path);

        // Remove the merged-away person's directory
        let remove_dir = if let Some(ref tp) = removed.tracker_path {
            workspace.join(tp)
        } else {
            crate::people::person_dir(workspace, &removed.name)
        };
        if remove_dir.exists() {
            let _ = std::fs::remove_dir_all(&remove_dir);
        }

        // Regenerate kept person's files
        if let Ok(Some(kept)) = db.get_person(keep_id) {
            let _ = crate::people::write_person_json(workspace, &kept, db);
            let _ = crate::people::write_person_markdown(workspace, &kept, db);
        }
    }

    Ok(keep_id.to_string())
}

/// Delete a person and all their references. Also removes their filesystem directory.
pub fn delete_person(
    db: &ActionDb,
    state: &AppState,
    person_id: &str,
) -> Result<(), String> {
    // Get person info before delete (for filesystem cleanup)
    let person = db
        .get_person(person_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Person not found: {}", person_id))?;

    // Perform DB delete
    db.delete_person(person_id).map_err(|e| e.to_string())?;

    // Emit deletion signal (I308)
    let _ = crate::signals::bus::emit_signal(db, "person", person_id, "entity_deleted", "user_action",
        Some(&format!("{{\"name\":\"{}\"}}", person.name.replace('"', "\\\""))), 1.0);

    // Filesystem cleanup
    let config = state.config.read().map_err(|_| "Lock poisoned")?;
    if let Some(ref config) = *config {
        let workspace = Path::new(&config.workspace_path);
        let person_dir = if let Some(ref tp) = person.tracker_path {
            workspace.join(tp)
        } else {
            crate::people::person_dir(workspace, &person.name)
        };
        if person_dir.exists() {
            let _ = std::fs::remove_dir_all(&person_dir);
        }
    }

    Ok(())
}

/// Get full detail for a person (person + signals + entities + recent meetings).
pub fn get_person_detail(
    person_id: &str,
    state: &AppState,
) -> Result<PersonDetailResult, String> {
    let db_guard = state.db.lock().map_err(|_| "Lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let person = db
        .get_person(person_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Person not found: {}", person_id))?;

    let signals = db.get_person_signals(person_id).ok();

    let entities = db
        .get_entities_for_person(person_id)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|e| EntitySummary {
            id: e.id,
            name: e.name,
            entity_type: e.entity_type.as_str().to_string(),
        })
        .collect();

    let recent_meetings = db
        .get_person_meetings(person_id, 10)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|m| MeetingSummary {
            id: m.id,
            title: m.title,
            start_time: m.start_time,
            meeting_type: m.meeting_type,
        })
        .collect();

    let recent_captures = db
        .get_captures_for_person(person_id, 90)
        .unwrap_or_default();
    let recent_email_signals = db
        .list_recent_email_signals_for_entity(person_id, 12)
        .unwrap_or_default();

    // Load intelligence from person dir (if exists)
    let intelligence = {
        let config = state.config.read().map_err(|_| "Lock poisoned")?;
        if let Some(ref config) = *config {
            let person_dir =
                crate::people::person_dir(Path::new(&config.workspace_path), &person.name);
            crate::intelligence::read_intelligence_json(&person_dir).ok()
        } else {
            None
        }
    };

    let open_actions = db
        .get_person_actions(person_id)
        .map_err(|e| e.to_string())?;

    let upcoming_meetings: Vec<MeetingSummary> = db
        .get_upcoming_meetings_for_person(person_id, 5)
        .unwrap_or_default()
        .into_iter()
        .map(|m| MeetingSummary {
            id: m.id,
            title: m.title,
            start_time: m.start_time,
            meeting_type: m.meeting_type,
        })
        .collect();

    Ok(PersonDetailResult {
        person,
        signals,
        entities,
        recent_meetings,
        recent_captures,
        recent_email_signals,
        intelligence,
        open_actions,
        upcoming_meetings,
    })
}
