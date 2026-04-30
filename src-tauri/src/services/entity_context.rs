//! Entity context entries CRUD service.
//!
//! Structured knowledge entries for accounts, people, and projects.
//! Mirrors user_context_entries but scoped to an entity_type + entity_id pair.

use crate::state::AppState;
use crate::types::EntityContextEntry;

/// Get all context entries for an entity.
pub async fn get_entries(
    entity_type: &str,
    entity_id: &str,
    state: &AppState,
) -> Result<Vec<EntityContextEntry>, String> {
    let entity_type = entity_type.to_string();
    let entity_id = entity_id.to_string();
    state
        .db_read(move |db| {
            let conn = db.conn_ref();
            let mut stmt = conn
                .prepare(
                    "SELECT id, entity_type, entity_id, title, content, created_at, updated_at
                 FROM entity_context_entries
                 WHERE entity_type = ?1 AND entity_id = ?2
                 ORDER BY created_at DESC",
                )
                .map_err(|e| format!("Failed to prepare query: {}", e))?;

            let entries = stmt
                .query_map(rusqlite::params![entity_type, entity_id], |row| {
                    Ok(EntityContextEntry {
                        id: row.get("id")?,
                        entity_type: row.get("entity_type")?,
                        entity_id: row.get("entity_id")?,
                        title: row.get("title")?,
                        content: row.get("content")?,
                        created_at: row.get("created_at")?,
                        updated_at: row.get("updated_at")?,
                    })
                })
                .map_err(|e| format!("Failed to query entity context entries: {}", e))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("Failed to map entity context entries: {}", e))?;

            Ok(entries)
        })
        .await
}

/// Create a new entity context entry with embedding.
pub async fn create_entry(
    ctx: &crate::services::context::ServiceContext<'_>,
    entity_type: &str,
    entity_id: &str,
    title: &str,
    content: &str,
    state: &AppState,
) -> Result<EntityContextEntry, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let id = uuid::Uuid::new_v4().to_string();

    // Generate embedding before acquiring DB lock
    let embedding_blob =
        super::user_entity::embed_context_text(&state.embedding_model, title, content);

    let entity_type = entity_type.to_string();
    let entity_id = entity_id.to_string();
    let title = title.to_string();
    let content = content.to_string();
    let engine = std::sync::Arc::clone(&state.signals.engine);
    state
        .db_write(move |db| {
            db.conn_ref()
                .execute(
                    "INSERT INTO entity_context_entries (id, entity_type, entity_id, title, content, embedding)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    rusqlite::params![id, entity_type, entity_id, title, content, embedding_blob],
                )
                .map_err(|e| format!("Failed to create entity context entry: {}", e))?;

            let _ = crate::signals::bus::emit_signal_and_propagate(
                db,
                &engine,
                &entity_type,
                &entity_id,
                "context_entry_added",
                "user_entry",
                Some(&title),
                0.85,
            );

            let entry = db
                .conn_ref()
                .query_row(
                    "SELECT id, entity_type, entity_id, title, content, created_at, updated_at
                 FROM entity_context_entries WHERE id = ?1",
                    rusqlite::params![id],
                    |row| {
                        Ok(EntityContextEntry {
                            id: row.get("id")?,
                            entity_type: row.get("entity_type")?,
                            entity_id: row.get("entity_id")?,
                            title: row.get("title")?,
                            content: row.get("content")?,
                            created_at: row.get("created_at")?,
                            updated_at: row.get("updated_at")?,
                        })
                    },
                )
                .map_err(|e| format!("Failed to read created entity context entry: {}", e))?;

            Ok(entry)
        })
        .await
}

/// Update an existing entity context entry.
pub async fn update_entry(
    ctx: &crate::services::context::ServiceContext<'_>,
    id: &str,
    title: &str,
    content: &str,
    state: &AppState,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    // Regenerate embedding before acquiring DB lock
    let embedding_blob =
        super::user_entity::embed_context_text(&state.embedding_model, title, content);

    let id = id.to_string();
    let title = title.to_string();
    let content = content.to_string();
    let engine = std::sync::Arc::clone(&state.signals.engine);
    state
        .db_write(move |db| {
            // Look up entity_type/entity_id for signal emission
            let (entity_type, entity_id): (String, String) = db
                .conn_ref()
                .query_row(
                    "SELECT entity_type, entity_id FROM entity_context_entries WHERE id = ?1",
                    rusqlite::params![id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .map_err(|e| format!("Entity context entry not found: {}", e))?;

            let updated = db
                .conn_ref()
                .execute(
                    "UPDATE entity_context_entries
                 SET title = ?1, content = ?2, embedding = ?4, updated_at = CURRENT_TIMESTAMP
                 WHERE id = ?3",
                    rusqlite::params![title, content, id, embedding_blob],
                )
                .map_err(|e| format!("Failed to update entity context entry: {}", e))?;

            if updated == 0 {
                return Err(format!("Entity context entry not found: {}", id));
            }

            let _ = crate::signals::bus::emit_signal_and_propagate(
                db,
                &engine,
                &entity_type,
                &entity_id,
                "context_entry_updated",
                "user_entry",
                Some(&title),
                0.85,
            );

            Ok(())
        })
        .await
}

/// Delete an entity context entry.
pub async fn delete_entry(
    ctx: &crate::services::context::ServiceContext<'_>,
    id: &str,
    state: &AppState,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let id = id.to_string();
    let engine = std::sync::Arc::clone(&state.signals.engine);
    state
        .db_write(move |db| {
            // Look up entity_type/entity_id for signal emission
            let (entity_type, entity_id): (String, String) = db
                .conn_ref()
                .query_row(
                    "SELECT entity_type, entity_id FROM entity_context_entries WHERE id = ?1",
                    rusqlite::params![id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .map_err(|e| format!("Entity context entry not found: {}", e))?;

            let deleted = db
                .conn_ref()
                .execute(
                    "DELETE FROM entity_context_entries WHERE id = ?1",
                    rusqlite::params![id],
                )
                .map_err(|e| format!("Failed to delete entity context entry: {}", e))?;

            if deleted == 0 {
                return Err(format!("Entity context entry not found: {}", id));
            }

            let _ = crate::signals::bus::emit_signal_and_propagate(
                db,
                &engine,
                &entity_type,
                &entity_id,
                "context_entry_deleted",
                "user_entry",
                None,
                0.85,
            );

            Ok(())
        })
        .await
}

/// Migrate legacy notes from the people table into entity_context_entries.
///
/// Called once at startup. For each person with non-empty notes, creates
/// a context entry with title "Notes". Idempotent — skips entities that
/// already have entries.
pub fn migrate_legacy_notes(
    ctx: &crate::services::context::ServiceContext<'_>,
    db: &crate::db::ActionDb,
) -> Result<usize, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let conn = db.conn_ref();
    let mut count = 0usize;

    // People notes (only table with a `notes` column)
    let mut stmt = conn
        .prepare(
            "SELECT id, notes FROM people
             WHERE notes IS NOT NULL AND notes != ''
             AND id NOT IN (
                 SELECT entity_id FROM entity_context_entries
                 WHERE entity_type = 'person'
             )",
        )
        .map_err(|e| format!("Failed to query people notes: {}", e))?;

    let people: Vec<(String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| format!("Failed to read people notes: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    for (person_id, notes) in &people {
        let id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO entity_context_entries (id, entity_type, entity_id, title, content)
             VALUES (?1, 'person', ?2, 'Notes', ?3)",
            rusqlite::params![id, person_id, notes],
        )
        .map_err(|e| format!("Failed to migrate person notes: {}", e))?;
        count += 1;
    }

    if count > 0 {
        log::info!(
            "Migrated {} legacy people notes to entity_context_entries",
            count
        );
    }

    Ok(count)
}
