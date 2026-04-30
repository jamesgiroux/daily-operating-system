// People service — extracted from commands.rs
// Business logic for person merge and delete with filesystem cleanup.

use std::path::Path;

use crate::commands::{EntitySummary, MeetingSummary, PersonDetailResult};
use crate::db::ActionDb;
use crate::services::context::ServiceContext;
use crate::state::AppState;

/// Merge two people: transfer all references from `remove_id` to `keep_id`,
/// then delete the removed person. Also cleans up filesystem directories
/// and regenerates the kept person's files.
pub fn merge_people(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    state: &AppState,
    keep_id: &str,
    remove_id: &str,
) -> Result<String, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    // Get removed person's info before merge (for filesystem cleanup)
    let removed = db
        .get_person(remove_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Person not found: {}", remove_id))?;

    // Perform DB merge
    db.merge_people(keep_id, remove_id)
        .map_err(|e| e.to_string())?;

    // Filesystem cleanup
    let config = state.config.read();
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
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    state: &AppState,
    person_id: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    // Get person info before delete (for filesystem cleanup)
    let person = db
        .get_person(person_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Person not found: {}", person_id))?;

    // Perform DB delete
    db.delete_person(person_id).map_err(|e| e.to_string())?;

    // Emit deletion signal (I308)
    let _ = crate::services::signals::emit_and_propagate(
        db,
        &state.signals.engine,
        "person",
        person_id,
        "entity_deleted",
        "user_action",
        Some(&format!(
            "{{\"name\":\"{}\"}}",
            person.name.replace('"', "\\\"")
        )),
        1.0,
    );

    // Filesystem cleanup
    let config = state.config.read();
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
pub async fn get_person_detail(
    person_id: &str,
    state: &AppState,
) -> Result<PersonDetailResult, String> {
    let _config = state.config.read().clone();

    let person_id = person_id.to_string();
    state
        .db_read(move |db| {
            let person = db
                .get_person(&person_id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("Person not found: {}", person_id))?;

            let signals = db.get_person_signals(&person_id).ok();

            let entities = db
                .get_entities_for_person(&person_id)
                .map_err(|e| e.to_string())?
                .into_iter()
                .map(|e| EntitySummary {
                    id: e.id,
                    name: e.name,
                    entity_type: e.entity_type.as_str().to_string(),
                })
                .collect();

            let recent_meetings = db
                .get_person_meetings(&person_id, 10)
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
                .get_captures_for_person(&person_id, 90)
                .unwrap_or_default();
            let recent_email_signals = db
                .list_recent_email_signals_for_entity(&person_id, 12)
                .unwrap_or_default();

            // Load intelligence from DB (I513)
            let intelligence = db.get_entity_intelligence(&person_id).ok().flatten();

            let open_actions = db
                .get_person_actions(&person_id)
                .map_err(|e| e.to_string())?;

            let upcoming_meetings: Vec<MeetingSummary> = db
                .get_upcoming_meetings_for_person(&person_id, 5)
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
        })
        .await
}

/// Update a single field on a person, emit signal, and regenerate workspace files.
pub fn update_person_field(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    state: &AppState,
    person_id: &str,
    field: &str,
    value: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let prior_source = db
        .get_person(person_id)
        .ok()
        .flatten()
        .and_then(|person| person.enrichment_sources)
        .and_then(|sources_json| serde_json::from_str::<serde_json::Value>(&sources_json).ok())
        .and_then(|sources| {
            sources[field]
                .get("source")
                .and_then(|s| s.as_str())
                .map(|s| s.to_string())
        });

    db.update_person_field(person_id, field, value)
        .map_err(|e| e.to_string())?;

    // Emit field update signal + self-healing evaluation (I377, I410)
    let _ = crate::services::signals::emit_propagate_and_evaluate(
        db,
        &state.signals.engine,
        "person",
        person_id,
        "field_updated",
        "user_edit",
        Some(&format!(
            "{{\"field\":\"{}\",\"value\":\"{}\"}}",
            field,
            value.replace('"', "\\\"")
        )),
        0.8,
        &state.intel_queue,
    );

    // Self-healing: record user correction for Clay-enrichable fields (I409)
    if matches!(field, "linkedin_url" | "role" | "organization" | "name") {
        crate::self_healing::feedback::record_enrichment_correction(
            db, person_id, "person", "clay",
        );
    }

    // Mark this field as user-owned so lower-priority enrichment sources cannot overwrite it.
    if let Err(e) = db.set_person_field_source(person_id, field, "user") {
        log::warn!(
            "I507: Failed to stamp user provenance for person {} field {}: {}",
            person_id,
            field,
            e
        );
    }

    // I507: Source-attributed correction feedback — penalize the prior source that
    // wrote the field, then mark user as the new owner in provenance.
    if let Some(prior_source) = prior_source.as_deref() {
        if prior_source != "user" {
            let _ = db.upsert_signal_weight(prior_source, "person", "profile_enrichment", 0.0, 1.0);
        }
    }

    // Regenerate workspace files
    if let Ok(Some(person)) = db.get_person(person_id) {
        let config = state.config.read();
        if let Some(ref config) = *config {
            let workspace = Path::new(&config.workspace_path);
            let _ = crate::people::write_person_json(workspace, &person, db);
            let _ = crate::people::write_person_markdown(workspace, &person, db);
        }
    }

    Ok(())
}

/// Link a person to an entity and regenerate workspace files.
pub fn link_person_entity(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    state: &AppState,
    person_id: &str,
    entity_id: &str,
    relationship_type: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    db.link_person_to_entity(person_id, entity_id, relationship_type)
        .map_err(|e| e.to_string())?;

    // Emit person linked signal (I308)
    let _ = crate::services::signals::emit_and_propagate(
        db,
        &state.signals.engine,
        relationship_type,
        entity_id,
        "person_linked",
        "user_action",
        Some(&format!("{{\"person_id\":\"{}\"}}", person_id)),
        0.9,
    );

    // Regenerate person.json so linked_entities persists in filesystem (ADR-0048)
    if let Ok(Some(person)) = db.get_person(person_id) {
        let config = state.config.read();
        if let Some(ref config) = *config {
            let workspace = Path::new(&config.workspace_path);
            let _ = crate::people::write_person_json(workspace, &person, db);
            let _ = crate::people::write_person_markdown(workspace, &person, db);
        }
    }

    Ok(())
}

/// Unlink a person from an entity and regenerate workspace files.
pub fn unlink_person_entity(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    state: &AppState,
    person_id: &str,
    entity_id: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    db.unlink_person_from_entity(person_id, entity_id)
        .map_err(|e| e.to_string())?;

    // Emit person unlinked signal (I308)
    let _ = crate::services::signals::emit_and_propagate(
        db,
        &state.signals.engine,
        "entity",
        entity_id,
        "person_unlinked",
        "user_action",
        Some(&format!("{{\"person_id\":\"{}\"}}", person_id)),
        0.7,
    );

    // Regenerate person.json so linked_entities reflects removal (ADR-0048)
    if let Ok(Some(person)) = db.get_person(person_id) {
        let config = state.config.read();
        if let Some(ref config) = *config {
            let workspace = Path::new(&config.workspace_path);
            let _ = crate::people::write_person_json(workspace, &person, db);
            let _ = crate::people::write_person_markdown(workspace, &person, db);
        }
    }

    Ok(())
}

/// Create a new person manually. Returns the generated person ID.
// DOS-209 adds ServiceContext to this existing command-shaped API during W2-A.
#[allow(clippy::too_many_arguments)]
pub fn create_person(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    state: &AppState,
    email: &str,
    name: &str,
    organization: Option<&str>,
    role: Option<&str>,
    relationship: Option<&str>,
) -> Result<String, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let id = crate::util::slugify(email);
    let now = ctx.clock.now().to_rfc3339();

    let person = crate::db::DbPerson {
        id: id.clone(),
        email: email.to_string(),
        name: name.to_string(),
        organization: organization.map(|s| s.to_string()),
        role: role.map(|s| s.to_string()),
        relationship: relationship.unwrap_or("unknown").to_string(),
        notes: None,
        tracker_path: None,
        last_seen: None,
        first_seen: Some(now.clone()),
        meeting_count: 0,
        updated_at: now,
        archived: false,
        linkedin_url: None,
        twitter_handle: None,
        phone: None,
        photo_url: None,
        bio: None,
        title_history: None,
        company_industry: None,
        company_size: None,
        company_hq: None,
        last_enriched_at: None,
        enrichment_sources: None,
    };

    db.upsert_person(&person).map_err(|e| e.to_string())?;

    // Self-healing: initialize quality row for new entity (I406)
    crate::self_healing::quality::ensure_quality_row(db, &id, "person");

    // Write workspace files (write-only exports for external systems)
    let config = state.config.read();
    if let Some(ref config) = *config {
        let workspace = Path::new(&config.workspace_path);
        let person_dir = crate::people::person_dir(workspace, &person.name);
        let _ = std::fs::create_dir_all(&person_dir);
        let _ = crate::util::bootstrap_entity_directory(&person_dir, &person.name, "person");
        let _ = crate::people::write_person_json(workspace, &person, db);
        let _ = crate::people::write_person_markdown(workspace, &person, db);
        let _ = crate::people::write_person_dashboard_json(workspace, &person, db);
    }

    Ok(id)
}

/// Archive or unarchive a person with signal emission.
///
/// DOS-286: when archiving, drops any pending intel queue entries for this
/// person so already-queued enrichments don't run against a now-archived target.
pub fn archive_person(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    state: &AppState,
    id: &str,
    archived: bool,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    db.archive_person(id, archived).map_err(|e| e.to_string())?;

    let signal_type = if archived {
        "entity_archived"
    } else {
        "entity_unarchived"
    };
    let _ = crate::services::signals::emit_and_propagate(
        db,
        &state.signals.engine,
        "person",
        id,
        signal_type,
        "user_action",
        None,
        0.9,
    );

    // DOS-286: drop any in-flight enrichments for the archived person.
    if archived {
        state.intel_queue.remove_by_entity_id(id);
    }

    Ok(())
}

/// Create a person entity from a stakeholder name (no email required).
/// Links to the parent entity and writes workspace files.
pub fn create_person_from_stakeholder(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    state: &AppState,
    entity_id: &str,
    entity_type: &str,
    name: &str,
    role: Option<&str>,
) -> Result<String, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("Name is required".to_string());
    }

    let id = crate::util::slugify(&name);
    let now = ctx.clock.now().to_rfc3339();

    let person = crate::db::DbPerson {
        id: id.clone(),
        email: String::new(),
        name: name.clone(),
        organization: None,
        role: role.map(|s| s.to_string()),
        relationship: "external".to_string(),
        notes: None,
        tracker_path: None,
        last_seen: None,
        first_seen: Some(now.clone()),
        meeting_count: 0,
        updated_at: now,
        archived: false,
        linkedin_url: None,
        twitter_handle: None,
        phone: None,
        photo_url: None,
        bio: None,
        title_history: None,
        company_industry: None,
        company_size: None,
        company_hq: None,
        last_enriched_at: None,
        enrichment_sources: None,
    };

    db.upsert_person(&person).map_err(|e| e.to_string())?;

    // Link to the parent entity
    db.link_person_to_entity(&id, entity_id, "associated")
        .map_err(|e| e.to_string())?;

    // Write person files to workspace
    let config = state.config.read();
    if let Some(ref config) = *config {
        let workspace = Path::new(&config.workspace_path);
        let person_dir = crate::people::person_dir(workspace, &person.name);
        let _ = std::fs::create_dir_all(&person_dir);
        let _ = crate::util::bootstrap_entity_directory(&person_dir, &person.name, "person");
        let _ = crate::people::write_person_json(workspace, &person, db);
        let _ = crate::people::write_person_markdown(workspace, &person, db);
        let _ = crate::people::write_person_dashboard_json(workspace, &person, db);
    }

    log::info!(
        "Created person '{}' (id={}) from stakeholder, linked to {} '{}'",
        name,
        id,
        entity_type,
        entity_id,
    );

    Ok(id)
}
