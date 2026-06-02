use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::services::context::ServiceContext;
use crate::state::AppState;

const OVERLAY_SCHEMA_VERSION: i64 = 1;
const MAX_OVERLAY_BYTES: usize = 32 * 1024;
const MAX_ORDER_ITEMS: usize = 128;
const MAX_LABEL_CHARS: usize = 120;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompositionLayoutOverlay {
    pub schema_version: i64,
    #[serde(default)]
    pub section_order: Vec<String>,
    #[serde(default)]
    pub block_order: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub hidden_section_ids: Vec<String>,
    #[serde(default)]
    pub hidden_block_ids: Vec<String>,
    #[serde(default)]
    pub block_variants: BTreeMap<String, String>,
    #[serde(default)]
    pub section_label_overrides: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutOverlayResponse {
    pub entity_type: String,
    pub surface_key: String,
    pub overlay_schema_version: i64,
    pub layout_revision: i64,
    pub overlay: Option<CompositionLayoutOverlay>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveLayoutOverlayRequest {
    pub entity_type: String,
    pub surface_key: String,
    pub overlay: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutOverlayKey {
    pub entity_type: String,
    pub surface_key: String,
}

pub async fn get_layout_overlay(
    state: Arc<AppState>,
    key: LayoutOverlayKey,
) -> Result<LayoutOverlayResponse, String> {
    let entity_type = validate_entity_type(&key.entity_type)?.to_string();
    let surface_key = validate_surface_key(&key.surface_key)?.to_string();
    state
        .db_read(move |db| read_overlay_response(db.conn_ref(), &entity_type, &surface_key))
        .await
        .map_err(|error| error.to_string())
}

pub async fn save_layout_overlay(
    ctx: &ServiceContext<'_>,
    state: Arc<AppState>,
    request: SaveLayoutOverlayRequest,
) -> Result<LayoutOverlayResponse, String> {
    ctx.check_mutation_allowed()
        .map_err(|error| error.to_string())?;

    let entity_type = validate_entity_type(&request.entity_type)?.to_string();
    let surface_key = validate_surface_key(&request.surface_key)?.to_string();
    let overlay = validate_overlay_payload(request.overlay)?;

    state
        .db_write(move |db| {
            save_overlay_response(db.conn_ref(), &entity_type, &surface_key, overlay)
        })
        .await
        .map_err(|error| error.to_string())
}

pub async fn reset_layout_overlay(
    ctx: &ServiceContext<'_>,
    state: Arc<AppState>,
    key: LayoutOverlayKey,
) -> Result<LayoutOverlayResponse, String> {
    ctx.check_mutation_allowed()
        .map_err(|error| error.to_string())?;

    let entity_type = validate_entity_type(&key.entity_type)?.to_string();
    let surface_key = validate_surface_key(&key.surface_key)?.to_string();

    state
        .db_write(move |db| reset_overlay_response(db.conn_ref(), &entity_type, &surface_key))
        .await
        .map_err(|error| error.to_string())
}

fn read_overlay_response(
    conn: &Connection,
    entity_type: &str,
    surface_key: &str,
) -> Result<LayoutOverlayResponse, String> {
    validate_entity_type(entity_type)?;
    validate_surface_key(surface_key)?;

    let row = conn
        .query_row(
            "SELECT overlay_schema_version, layout_revision, overlay_json, updated_at \
             FROM composition_layout_overlays \
             WHERE entity_type = ?1 AND surface_key = ?2",
            params![entity_type, surface_key],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .optional()
        .map_err(|error| format!("read layout overlay failed: {error}"))?;

    match row {
        Some((overlay_schema_version, layout_revision, overlay_json, updated_at)) => {
            let overlay = serde_json::from_str::<CompositionLayoutOverlay>(&overlay_json)
                .map_err(|error| format!("stored layout overlay is invalid: {error}"))?;
            Ok(LayoutOverlayResponse {
                entity_type: entity_type.to_string(),
                surface_key: surface_key.to_string(),
                overlay_schema_version,
                layout_revision,
                overlay: Some(overlay),
                updated_at: Some(updated_at),
            })
        }
        None => Ok(LayoutOverlayResponse {
            entity_type: entity_type.to_string(),
            surface_key: surface_key.to_string(),
            overlay_schema_version: OVERLAY_SCHEMA_VERSION,
            layout_revision: 0,
            overlay: None,
            updated_at: None,
        }),
    }
}

fn save_overlay_response(
    conn: &Connection,
    entity_type: &str,
    surface_key: &str,
    mut overlay: CompositionLayoutOverlay,
) -> Result<LayoutOverlayResponse, String> {
    validate_entity_type(entity_type)?;
    validate_surface_key(surface_key)?;

    let now = Utc::now().to_rfc3339();
    overlay.updated_at = Some(now.clone());
    let overlay_json = serde_json::to_string(&overlay)
        .map_err(|error| format!("serialize layout overlay failed: {error}"))?;
    if overlay_json.len() > MAX_OVERLAY_BYTES {
        return Err("layout overlay payload exceeds 32768 bytes".to_string());
    }

    let next_revision = current_layout_revision(conn, entity_type, surface_key)? + 1;
    conn.execute(
        "INSERT INTO composition_layout_overlays (
            entity_type,
            surface_key,
            overlay_schema_version,
            layout_revision,
            overlay_json,
            created_at,
            updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)
         ON CONFLICT(entity_type, surface_key) DO UPDATE SET
            overlay_schema_version = excluded.overlay_schema_version,
            layout_revision = excluded.layout_revision,
            overlay_json = excluded.overlay_json,
            updated_at = excluded.updated_at",
        params![
            entity_type,
            surface_key,
            overlay.schema_version,
            next_revision,
            overlay_json,
            now,
        ],
    )
    .map_err(|error| format!("save layout overlay failed: {error}"))?;

    read_overlay_response(conn, entity_type, surface_key)
}

fn reset_overlay_response(
    conn: &Connection,
    entity_type: &str,
    surface_key: &str,
) -> Result<LayoutOverlayResponse, String> {
    validate_entity_type(entity_type)?;
    validate_surface_key(surface_key)?;

    let next_revision = current_layout_revision(conn, entity_type, surface_key)? + 1;
    conn.execute(
        "DELETE FROM composition_layout_overlays WHERE entity_type = ?1 AND surface_key = ?2",
        params![entity_type, surface_key],
    )
    .map_err(|error| format!("reset layout overlay failed: {error}"))?;

    Ok(LayoutOverlayResponse {
        entity_type: entity_type.to_string(),
        surface_key: surface_key.to_string(),
        overlay_schema_version: OVERLAY_SCHEMA_VERSION,
        layout_revision: next_revision,
        overlay: None,
        updated_at: None,
    })
}

fn current_layout_revision(
    conn: &Connection,
    entity_type: &str,
    surface_key: &str,
) -> Result<i64, String> {
    conn.query_row(
        "SELECT layout_revision FROM composition_layout_overlays \
         WHERE entity_type = ?1 AND surface_key = ?2",
        params![entity_type, surface_key],
        |row| row.get::<_, i64>(0),
    )
    .optional()
    .map_err(|error| format!("read layout revision failed: {error}"))
    .map(|revision| revision.unwrap_or(0))
}

fn validate_entity_type(value: &str) -> Result<&str, String> {
    let value = value.trim();
    match value {
        "account" | "project" | "person" => Ok(value),
        _ => Err("layout overlay entity_type must be account, project, or person".to_string()),
    }
}

fn validate_surface_key(value: &str) -> Result<&str, String> {
    let value = value.trim();
    match value {
        "entity_page" => Ok(value),
        _ => Err("layout overlay surface_key must be entity_page".to_string()),
    }
}

fn validate_overlay_payload(value: serde_json::Value) -> Result<CompositionLayoutOverlay, String> {
    let raw = serde_json::to_string(&value)
        .map_err(|error| format!("serialize incoming layout overlay failed: {error}"))?;
    if raw.len() > MAX_OVERLAY_BYTES {
        return Err("layout overlay payload exceeds 32768 bytes".to_string());
    }
    if !value.is_object() {
        return Err("layout overlay payload must be an object".to_string());
    }

    let overlay: CompositionLayoutOverlay = serde_json::from_value(value)
        .map_err(|error| format!("layout overlay payload shape is invalid: {error}"))?;
    if overlay.schema_version != OVERLAY_SCHEMA_VERSION {
        return Err("layout overlay schemaVersion must be 1".to_string());
    }
    validate_id_list("sectionOrder", &overlay.section_order)?;
    validate_id_map("blockOrder", &overlay.block_order)?;
    validate_id_list("hiddenSectionIds", &overlay.hidden_section_ids)?;
    validate_id_list("hiddenBlockIds", &overlay.hidden_block_ids)?;
    validate_variant_map(&overlay.block_variants)?;
    validate_label_map(&overlay.section_label_overrides)?;
    Ok(overlay)
}

fn validate_id_map(name: &str, value: &BTreeMap<String, Vec<String>>) -> Result<(), String> {
    if value.len() > MAX_ORDER_ITEMS {
        return Err(format!("{name} exceeds {MAX_ORDER_ITEMS} entries"));
    }
    for (key, ids) in value {
        validate_identifier(name, key)?;
        validate_id_list(name, ids)?;
    }
    Ok(())
}

fn validate_id_list(name: &str, ids: &[String]) -> Result<(), String> {
    if ids.len() > MAX_ORDER_ITEMS {
        return Err(format!("{name} exceeds {MAX_ORDER_ITEMS} entries"));
    }
    for id in ids {
        validate_identifier(name, id)?;
    }
    Ok(())
}

fn validate_identifier(name: &str, id: &str) -> Result<(), String> {
    let trimmed = id.trim();
    if trimmed.is_empty() || trimmed.len() > 128 {
        return Err(format!("{name} contains an invalid id"));
    }
    let allowed = trimmed.chars().all(|ch| {
        ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':' | '/' | '%' | '@')
    });
    if !allowed {
        return Err(format!("{name} contains an unsupported id character"));
    }
    Ok(())
}

fn validate_variant_map(value: &BTreeMap<String, String>) -> Result<(), String> {
    if value.len() > MAX_ORDER_ITEMS {
        return Err(format!("blockVariants exceeds {MAX_ORDER_ITEMS} entries"));
    }
    for (block_id, variant) in value {
        validate_identifier("blockVariants", block_id)?;
        match variant.as_str() {
            "default" | "compact" | "spotlight" => {}
            _ => return Err("blockVariants contains an unsupported variant".to_string()),
        }
    }
    Ok(())
}

fn validate_label_map(value: &BTreeMap<String, String>) -> Result<(), String> {
    if value.len() > MAX_ORDER_ITEMS {
        return Err(format!(
            "sectionLabelOverrides exceeds {MAX_ORDER_ITEMS} entries"
        ));
    }
    for (section_id, label) in value {
        validate_identifier("sectionLabelOverrides", section_id)?;
        let label = label.trim();
        if label.chars().count() > MAX_LABEL_CHARS || label.chars().any(char::is_control) {
            return Err("sectionLabelOverrides contains an invalid label".to_string());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use serde_json::json;

    fn conn() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory db");
        conn.execute_batch(include_str!(
            "../migrations/275_composition_layout_overlays.sql"
        ))
        .expect("migration 275");
        conn
    }

    fn valid_overlay() -> serde_json::Value {
        json!({
            "schemaVersion": 1,
            "sectionOrder": ["headline", "state-of-play"],
            "blockOrder": {
                "state-of-play": ["block:one", "block:two"]
            },
            "hiddenSectionIds": ["reports"],
            "hiddenBlockIds": ["block:three"],
            "blockVariants": {
                "block:one": "compact"
            },
            "sectionLabelOverrides": {
                "state-of-play": "State of play"
            }
        })
    }

    #[test]
    fn migration_275_is_idempotent() {
        let conn = Connection::open_in_memory().expect("in-memory db");
        let sql = include_str!("../migrations/275_composition_layout_overlays.sql");
        conn.execute_batch(sql).expect("first migration");
        conn.execute_batch(sql).expect("second migration");
    }

    #[test]
    fn create_read_update_and_reset_overlay() {
        let conn = conn();
        let missing = read_overlay_response(&conn, "account", "entity_page").expect("missing");
        assert!(missing.overlay.is_none());
        assert_eq!(missing.layout_revision, 0);

        let saved = save_overlay_response(
            &conn,
            "account",
            "entity_page",
            validate_overlay_payload(valid_overlay()).expect("valid overlay"),
        )
        .expect("save");
        assert_eq!(saved.layout_revision, 1);
        assert_eq!(
            saved.overlay.as_ref().expect("overlay").hidden_section_ids,
            vec!["reports"]
        );

        let mut changed = valid_overlay();
        changed["hiddenBlockIds"] = json!(["block:four"]);
        let updated = save_overlay_response(
            &conn,
            "account",
            "entity_page",
            validate_overlay_payload(changed).expect("changed overlay"),
        )
        .expect("update");
        assert_eq!(updated.layout_revision, 2);
        assert_eq!(
            updated.overlay.expect("overlay").hidden_block_ids,
            vec!["block:four"]
        );

        let reset = reset_overlay_response(&conn, "account", "entity_page").expect("reset");
        assert!(reset.overlay.is_none());
        assert_eq!(reset.layout_revision, 3);
        let after = read_overlay_response(&conn, "account", "entity_page").expect("after");
        assert!(after.overlay.is_none());
        assert_eq!(after.layout_revision, 0);
    }

    #[test]
    fn rejects_invalid_payloads_and_keys() {
        assert!(validate_entity_type("workspace").is_err());
        assert!(validate_surface_key("settings").is_err());

        let mut invalid_variant = valid_overlay();
        invalid_variant["blockVariants"] = json!({ "block:one": "raw" });
        assert!(validate_overlay_payload(invalid_variant).is_err());

        let mut invalid_id = valid_overlay();
        invalid_id["hiddenBlockIds"] = json!(["bad id"]);
        assert!(validate_overlay_payload(invalid_id).is_err());

        let mut wrong_schema = valid_overlay();
        wrong_schema["schemaVersion"] = json!(2);
        assert!(validate_overlay_payload(wrong_schema).is_err());
    }

    #[test]
    fn enforces_payload_bounds() {
        let mut oversized = valid_overlay();
        let ids = (0..200)
            .map(|index| format!("block:{index}"))
            .collect::<Vec<_>>();
        oversized["hiddenBlockIds"] = json!(ids);
        assert!(validate_overlay_payload(oversized).is_err());

        let mut long_label = valid_overlay();
        long_label["sectionLabelOverrides"] = json!({ "reports": "x".repeat(121) });
        assert!(validate_overlay_payload(long_label).is_err());
    }
}
