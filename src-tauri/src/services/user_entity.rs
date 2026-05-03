// User entity service — (ADR-0089/0090)
// Business logic for the user's professional identity and context entries.

use crate::db::ActionDb;
use crate::state::AppState;
use crate::types::{UserContextEntry, UserEntity};

/// Allowed fields for update_user_entity_field.
const ALLOWED_FIELDS: &[&str] = &[
    "name",
    "company",
    "title",
    "focus",
    "value_proposition",
    "success_definition",
    "current_priorities",
    "product_context",
    "playbooks",
    "company_bio",
    "role_description",
    "how_im_measured",
    "pricing_model",
    "differentiators",
    "objections",
    "competitive_context",
    "annual_priorities",
    "quarterly_priorities",
    "user_relevance_weight",
];

/// Fields that must contain valid JSON when set.
const JSON_FIELDS: &[&str] = &[
    "differentiators",
    "objections",
    "annual_priorities",
    "quarterly_priorities",
];

/// Read the user_entity row from DB. Returns Err if no row exists.
pub fn get_user_entity_from_db(db: &ActionDb) -> Result<UserEntity, String> {
    let conn = db.conn_ref();
    conn.query_row("SELECT * FROM user_entity WHERE id = 1", [], |row| {
        Ok(UserEntity {
            id: row.get("id")?,
            name: row.get("name")?,
            company: row.get("company")?,
            title: row.get("title")?,
            focus: row.get("focus")?,
            value_proposition: row.get("value_proposition")?,
            success_definition: row.get("success_definition")?,
            current_priorities: row.get("current_priorities")?,
            product_context: row.get("product_context")?,
            playbooks: row.get("playbooks")?,
            company_bio: row.get("company_bio")?,
            role_description: row.get("role_description")?,
            how_im_measured: row.get("how_im_measured")?,
            pricing_model: row.get("pricing_model")?,
            differentiators: row.get("differentiators")?,
            objections: row.get("objections")?,
            competitive_context: row.get("competitive_context")?,
            annual_priorities: row.get("annual_priorities")?,
            quarterly_priorities: row.get("quarterly_priorities")?,
            user_relevance_weight: row.get("user_relevance_weight")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    })
    .map_err(|e| format!("Failed to query user_entity: {}", e))
}

/// Get the user entity, seeding from config if no row exists yet.
pub async fn get_user_entity(
    ctx: &crate::services::context::ServiceContext<'_>,
    state: &AppState,
) -> Result<UserEntity, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let config = state.config.read().clone();

    let result = state
        .db_write(move |db| {
            match get_user_entity_from_db(db) {
                Ok(entity) => Ok((entity, false)),
                Err(_) => {
                    // No row yet — seed from config identity fields
                    let config = config.as_ref().ok_or("Config not initialized")?;

                    let name = config.user_name.clone();
                    let company = config.user_company.clone();
                    let title = config.user_title.clone();
                    let focus = config.user_focus.clone();

                    db.conn_ref()
                        .execute(
                            "INSERT INTO user_entity (id, name, company, title, focus)
                         VALUES (1, ?1, ?2, ?3, ?4)",
                            rusqlite::params![name, company, title, focus],
                        )
                        .map_err(|e| format!("Failed to seed user_entity: {}", e))?;

                    let entity = get_user_entity_from_db(db)?;
                    Ok((entity, true))
                }
            }
        })
        .await?;

    let (entity, needs_config_clear) = result;
    if needs_config_clear {
        let _ = crate::state::create_or_update_config(state, |config| {
            config.user_name = None;
            config.user_company = None;
            config.user_title = None;
            config.user_focus = None;
        });
    }

    Ok(entity)
}

/// Update a single field on the user entity.
pub async fn update_user_entity_field(
    ctx: &crate::services::context::ServiceContext<'_>,
    field: &str,
    value: &str,
    state: &AppState,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    if !ALLOWED_FIELDS.contains(&field) {
        return Err(format!(
            "Invalid field '{}'. Allowed: {}",
            field,
            ALLOWED_FIELDS.join(", ")
        ));
    }

    // Validate objections and differentiators as arrays of strings (AC8)
    if (field == "objections" || field == "differentiators") && !value.is_empty() {
        serde_json::from_str::<Vec<String>>(value)
            .map_err(|e| format!("Field '{}' must be a JSON array of strings: {}", field, e))?;
    } else if JSON_FIELDS.contains(&field) && !value.is_empty() {
        // General JSON validation for other JSON fields
        serde_json::from_str::<serde_json::Value>(value)
            .map_err(|e| format!("Invalid JSON for field '{}': {}", field, e))?;
    }

    let config = state.config.read().clone();

    let field = field.to_string();
    let value = value.to_string();
    state
        .db_write(move |db| {
            // Ensure row exists
            let exists: bool = db
                .conn_ref()
                .query_row(
                    "SELECT COUNT(*) > 0 FROM user_entity WHERE id = 1",
                    [],
                    |row| row.get(0),
                )
                .unwrap_or(false);

            if !exists {
                db.conn_ref()
                    .execute("INSERT INTO user_entity (id) VALUES (1)", [])
                    .map_err(|e| format!("Failed to create user_entity row: {}", e))?;
            }

            // Dynamic field update — field name is validated against ALLOWED_FIELDS above
            let sql = format!(
                "UPDATE user_entity SET {} = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = 1",
                field
            );
            let value_param: Option<&str> = if value.is_empty() { None } else { Some(&value) };
            db.conn_ref()
                .execute(&sql, rusqlite::params![value_param])
                .map_err(|e| format!("Failed to update user_entity.{}: {}", field, e))?;

            // Write context.json to workspace (inline since we can't call the method with state)
            if let Some(ref config) = config {
                if !config.workspace_path.is_empty() {
                    let workspace = std::path::Path::new(&config.workspace_path);
                    let user_dir = workspace.join("_user");
                    if !user_dir.exists() {
                        let _ = std::fs::create_dir_all(&user_dir);
                    }
                    if let Ok(entity) = get_user_entity_from_db(db) {
                        let mut obj = serde_json::Map::new();
                        macro_rules! add_field {
                            ($field:ident) => {
                                if let Some(ref val) = entity.$field {
                                    if !val.is_empty() {
                                        obj.insert(
                                            stringify!($field).to_string(),
                                            serde_json::Value::String(val.clone()),
                                        );
                                    }
                                }
                            };
                        }
                        add_field!(name);
                        add_field!(company);
                        add_field!(title);
                        add_field!(focus);
                        add_field!(value_proposition);
                        add_field!(success_definition);
                        add_field!(current_priorities);
                        add_field!(product_context);
                        add_field!(playbooks);
                        add_field!(company_bio);
                        add_field!(role_description);
                        add_field!(how_im_measured);
                        add_field!(pricing_model);
                        add_field!(competitive_context);

                        for json_field in &[
                            "differentiators",
                            "objections",
                            "annual_priorities",
                            "quarterly_priorities",
                        ] {
                            let val = match *json_field {
                                "differentiators" => &entity.differentiators,
                                "objections" => &entity.objections,
                                "annual_priorities" => &entity.annual_priorities,
                                "quarterly_priorities" => &entity.quarterly_priorities,
                                _ => continue,
                            };
                            if let Some(ref s) = val {
                                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(s) {
                                    obj.insert(json_field.to_string(), parsed);
                                }
                            }
                        }
                        if let Some(w) = entity.user_relevance_weight {
                            obj.insert(
                                "user_relevance_weight".to_string(),
                                serde_json::Value::Number(
                                    serde_json::Number::from_f64(w)
                                        .unwrap_or_else(|| serde_json::Number::from(1)),
                                ),
                            );
                        }
                        if let Ok(json) =
                            serde_json::to_string_pretty(&serde_json::Value::Object(obj))
                        {
                            let path = user_dir.join("context.json");
                            let _ = crate::util::atomic_write_str(&path, &json);
                        }
                    }
                }
            }

            Ok(())
        })
        .await
}

/// Get all user context entries.
pub async fn get_user_context_entries(state: &AppState) -> Result<Vec<UserContextEntry>, String> {
    state
        .db_read(|db| {
            let conn = db.conn_ref();
            let mut stmt = conn
                .prepare(
                    "SELECT id, title, content, embedding_id, created_at, updated_at
                 FROM user_context_entries ORDER BY created_at DESC",
                )
                .map_err(|e| format!("Failed to prepare query: {}", e))?;

            let entries = stmt
                .query_map([], |row| {
                    Ok(UserContextEntry {
                        id: row.get("id")?,
                        title: row.get("title")?,
                        content: row.get("content")?,
                        embedding_id: row.get("embedding_id")?,
                        created_at: row.get("created_at")?,
                        updated_at: row.get("updated_at")?,
                    })
                })
                .map_err(|e| format!("Failed to query context entries: {}", e))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("Failed to map context entries: {}", e))?;

            Ok(entries)
        })
        .await
}

/// Create a new user context entry and generate its embedding.
pub async fn create_user_context_entry(
    ctx: &crate::services::context::ServiceContext<'_>,
    title: &str,
    content: &str,
    state: &AppState,
) -> Result<UserContextEntry, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let id = uuid::Uuid::new_v4().to_string();

    // Generate embedding before acquiring DB lock (embedding is CPU-bound)
    let embedding_blob = embed_context_text(&state.embedding_model, title, content);

    let title = title.to_string();
    let content = content.to_string();
    state
        .db_write(move |db| {
            db.conn_ref()
                .execute(
                    "INSERT INTO user_context_entries (id, title, content, embedding)
                 VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![id, title, content, embedding_blob],
                )
                .map_err(|e| format!("Failed to create context entry: {}", e))?;

            let entry = db
                .conn_ref()
                .query_row(
                    "SELECT id, title, content, embedding_id, created_at, updated_at
                 FROM user_context_entries WHERE id = ?1",
                    rusqlite::params![id],
                    |row| {
                        Ok(UserContextEntry {
                            id: row.get("id")?,
                            title: row.get("title")?,
                            content: row.get("content")?,
                            embedding_id: row.get("embedding_id")?,
                            created_at: row.get("created_at")?,
                            updated_at: row.get("updated_at")?,
                        })
                    },
                )
                .map_err(|e| format!("Failed to read created entry: {}", e))?;

            Ok(entry)
        })
        .await
}

/// Update an existing user context entry and regenerate its embedding.
pub async fn update_user_context_entry(
    ctx: &crate::services::context::ServiceContext<'_>,
    id: &str,
    title: &str,
    content: &str,
    state: &AppState,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    // Regenerate embedding before acquiring DB lock
    let embedding_blob = embed_context_text(&state.embedding_model, title, content);

    let id = id.to_string();
    let title = title.to_string();
    let content = content.to_string();
    state.db_write(move |db| {
        let updated = db
            .conn_ref()
            .execute(
                "UPDATE user_context_entries
                 SET title = ?1, content = ?2, embedding = ?4, embedding_id = NULL, updated_at = CURRENT_TIMESTAMP
                 WHERE id = ?3",
                rusqlite::params![title, content, id, embedding_blob],
            )
            .map_err(|e| format!("Failed to update context entry: {}", e))?;

        if updated == 0 {
            return Err(format!("Context entry not found: {}", id));
        }

        Ok(())
    }).await
}

/// Delete a user context entry and its associated embedding.
pub async fn delete_user_context_entry(
    ctx: &crate::services::context::ServiceContext<'_>,
    id: &str,
    state: &AppState,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let id = id.to_string();
    state
        .db_write(move |db| {
            // Get embedding_id before deletion for cleanup
            let embedding_id: Option<String> = db
                .conn_ref()
                .query_row(
                    "SELECT embedding_id FROM user_context_entries WHERE id = ?1",
                    rusqlite::params![id],
                    |row| row.get::<_, Option<String>>(0),
                )
                .ok()
                .flatten();

            let deleted = db
                .conn_ref()
                .execute(
                    "DELETE FROM user_context_entries WHERE id = ?1",
                    rusqlite::params![id],
                )
                .map_err(|e| format!("Failed to delete context entry: {}", e))?;

            if deleted == 0 {
                return Err(format!("Context entry not found: {}", id));
            }

            // Clean up associated embedding if it exists
            if let Some(ref emb_id) = embedding_id {
                let _ = db.conn_ref().execute(
                    "DELETE FROM content_embeddings WHERE id = ?1",
                    rusqlite::params![emb_id],
                );
            }

            Ok(())
        })
        .await
}

/// Embed context entry text using the document prefix for asymmetric retrieval.
///
/// Returns `None` when the embedding model is unavailable (entry saves without
/// an embedding — the background sweep or next update will retry).
pub(crate) fn embed_context_text(
    model: &crate::embeddings::EmbeddingModel,
    title: &str,
    content: &str,
) -> Option<Vec<u8>> {
    if !model.is_ready() {
        return None;
    }
    let text = format!(
        "{}{}: {}",
        crate::embeddings::DOCUMENT_PREFIX,
        title,
        content
    );
    model
        .embed(&text)
        .ok()
        .map(|v| crate::embeddings::f32_vec_to_blob(&v))
}

/// Write the user entity to `_user/context.json` in the workspace.
fn write_user_context_json(
    ctx: &crate::services::context::ServiceContext<'_>,
    db: &ActionDb,
    state: &AppState,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let config_guard = state.config.read();
    let config = config_guard.as_ref().ok_or("Config not initialized")?;

    if config.workspace_path.is_empty() {
        return Ok(());
    }

    let workspace = std::path::Path::new(&config.workspace_path);
    let user_dir = workspace.join("_user");
    if !user_dir.exists() {
        std::fs::create_dir_all(&user_dir)
            .map_err(|e| format!("Failed to create _user dir: {}", e))?;
    }

    let entity = get_user_entity_from_db(db)?;

    // Build a JSON object with only non-null fields
    let mut obj = serde_json::Map::new();
    macro_rules! add_field {
        ($field:ident) => {
            if let Some(ref val) = entity.$field {
                if !val.is_empty() {
                    obj.insert(
                        stringify!($field).to_string(),
                        serde_json::Value::String(val.clone()),
                    );
                }
            }
        };
    }

    add_field!(name);
    add_field!(company);
    add_field!(title);
    add_field!(focus);
    add_field!(value_proposition);
    add_field!(success_definition);
    add_field!(current_priorities);
    add_field!(product_context);
    add_field!(playbooks);
    add_field!(company_bio);
    add_field!(role_description);
    add_field!(how_im_measured);
    add_field!(pricing_model);
    add_field!(competitive_context);

    // JSON fields: parse and insert as structured values
    for json_field in &[
        "differentiators",
        "objections",
        "annual_priorities",
        "quarterly_priorities",
    ] {
        let val = match *json_field {
            "differentiators" => &entity.differentiators,
            "objections" => &entity.objections,
            "annual_priorities" => &entity.annual_priorities,
            "quarterly_priorities" => &entity.quarterly_priorities,
            _ => continue,
        };
        if let Some(ref s) = val {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(s) {
                obj.insert(json_field.to_string(), parsed);
            }
        }
    }

    if let Some(w) = entity.user_relevance_weight {
        obj.insert(
            "user_relevance_weight".to_string(),
            serde_json::Value::Number(
                serde_json::Number::from_f64(w).unwrap_or_else(|| serde_json::Number::from(1)),
            ),
        );
    }

    let json = serde_json::to_string_pretty(&serde_json::Value::Object(obj))
        .map_err(|e| format!("Failed to serialize context.json: {}", e))?;

    let path = user_dir.join("context.json");
    crate::util::atomic_write_str(&path, &json)
        .map_err(|e| format!("Failed to write context.json: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::test_db;

    #[test]
    fn test_get_user_entity_creates_row() {
        let db = test_db();
        // Initially no row
        assert!(get_user_entity_from_db(&db).is_err());

        // Insert directly
        db.conn_ref()
            .execute(
                "INSERT INTO user_entity (id, name, company) VALUES (1, 'Alice', 'Acme')",
                [],
            )
            .unwrap();

        let entity = get_user_entity_from_db(&db).unwrap();
        assert_eq!(entity.name.as_deref(), Some("Alice"));
        assert_eq!(entity.company.as_deref(), Some("Acme"));
        assert!(entity.title.is_none());
    }

    #[test]
    fn test_update_field_validates() {
        // Field validation (no state needed)
        assert!(!ALLOWED_FIELDS.contains(&"nonexistent"));
        assert!(ALLOWED_FIELDS.contains(&"name"));
        assert!(ALLOWED_FIELDS.contains(&"differentiators"));
    }

    #[test]
    fn test_json_field_validation() {
        assert!(JSON_FIELDS.contains(&"differentiators"));
        assert!(JSON_FIELDS.contains(&"annual_priorities"));
        assert!(!JSON_FIELDS.contains(&"name"));
    }

    #[test]
    fn test_context_entries_crud() {
        let db = test_db();

        // Insert
        db.conn_ref()
            .execute(
                "INSERT INTO user_context_entries (id, title, content) VALUES ('e1', 'Test', 'Content here')",
                [],
            )
            .unwrap();

        // Read
        let mut stmt = db
            .conn_ref()
            .prepare("SELECT id, title, content FROM user_context_entries WHERE id = 'e1'")
            .unwrap();
        let entry: (String, String, String) = stmt
            .query_row([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .unwrap();
        assert_eq!(entry.0, "e1");
        assert_eq!(entry.1, "Test");

        // Update
        db.conn_ref()
            .execute(
                "UPDATE user_context_entries SET title = 'Updated', updated_at = CURRENT_TIMESTAMP WHERE id = 'e1'",
                [],
            )
            .unwrap();

        let title: String = db
            .conn_ref()
            .query_row(
                "SELECT title FROM user_context_entries WHERE id = 'e1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(title, "Updated");

        // Delete
        db.conn_ref()
            .execute("DELETE FROM user_context_entries WHERE id = 'e1'", [])
            .unwrap();
        let count: i32 = db
            .conn_ref()
            .query_row("SELECT COUNT(*) FROM user_context_entries", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 0);
    }
}
