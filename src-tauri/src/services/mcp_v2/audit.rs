//! MCP v2 audit log writer.
//!
//! This module is intentionally a thin MCP-specific layer over
//! [`crate::audit_log::AuditLogger::append_with_actor`]. It owns the MCP
//! success-row privacy contract: read-class tools persist keyed digests for
//! request/response payloads; write-class tools drop top-level payload fields
//! before append.

use std::sync::Arc;

use chrono::{SecondsFormat, Utc};
use rusqlite::params;
use serde_json::{Map, Value};

use abilities_runtime::abilities::registry::Actor;

use crate::audit_log::{AuditError as AuditLogError, AuditFields, AuditLogger};

use super::contracts::Side;
use super::local_runtime;

pub const PARAM_PAYLOAD_KEYS: &[&str] = &["params", "parameters"];
pub const RESPONSE_PAYLOAD_KEYS: &[&str] = &["response", "response_or_error", "result"];

/// MCP-specific audit write failures.
#[derive(Debug, thiserror::Error)]
pub enum AuditError {
    #[error("audit detail serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("audit log append failed: {0}")]
    Append(String),
    #[error(
        "audit log append failed and audit outbox insert also failed: append={append_error}; outbox={outbox_error}"
    )]
    DoubleFailure {
        append_error: String,
        outbox_error: String,
    },
    #[error("audit detail digest failed: {0}")]
    Digest(String),
}

/// Write an MCP success-row audit event.
///
/// Read-side abilities replace their top-level `params` and `response` fields
/// with keyed digests. Write and submit-correction abilities remove the keys
/// named in [`PARAM_PAYLOAD_KEYS`] and [`RESPONSE_PAYLOAD_KEYS`]. Actor
/// attribution is always inserted or preserved.
pub fn write(
    actor: &Actor,
    event: &str,
    detail: Value,
    request_id: Option<String>,
    side: Side,
) -> Result<(), AuditError> {
    let detail_with_attribution = detail_with_attribution(actor, detail, side)?;

    let mut fields = AuditFields::new("security", detail_with_attribution.clone());
    if let Some(id) = request_id.clone() {
        fields = fields.with_request_id(id);
    }

    let mut logger = AuditLogger::new(crate::audit_log::default_audit_log_path());
    match logger.append_with_actor(event, actor, fields) {
        Ok(()) => Ok(()),
        Err(AuditLogError::Write(append_error)) => {
            let detail_json = serde_json::to_string(&detail_with_attribution)?;
            insert_outbox(
                event,
                &detail_json,
                actor_kind(actor),
                request_id.as_deref(),
            )
            .map_err(|outbox_error| AuditError::DoubleFailure {
                append_error,
                outbox_error,
            })
        }
        Err(error) => Err(AuditError::Append(error.to_string())),
    }
}

fn detail_with_attribution(actor: &Actor, detail: Value, side: Side) -> Result<Value, AuditError> {
    let mut detail = match side {
        Side::Read => digest_payload_detail(detail)?,
        Side::Write | Side::SubmitCorrection => sanitize_detail(detail),
    };
    add_actor_attribution(actor, &mut detail);
    Ok(Value::Object(detail))
}

fn object_detail(detail: Value) -> Map<String, Value> {
    match detail {
        Value::Object(map) => map,
        _ => Map::new(),
    }
}

fn sanitize_detail(detail: Value) -> Map<String, Value> {
    let mut map = object_detail(detail);
    for key in PARAM_PAYLOAD_KEYS
        .iter()
        .chain(RESPONSE_PAYLOAD_KEYS.iter())
        .copied()
    {
        map.remove(key);
    }
    map
}

fn digest_payload_detail(detail: Value) -> Result<Map<String, Value>, AuditError> {
    let mut map = object_detail(detail);
    for key in PARAM_PAYLOAD_KEYS
        .iter()
        .chain(RESPONSE_PAYLOAD_KEYS.iter())
        .copied()
    {
        if let Some(payload) = map.remove(key) {
            let digest = local_runtime::digest_json_value_hex(&payload)
                .map_err(|error| AuditError::Digest(error.to_string()))?;
            map.insert(format!("{key}_digest"), Value::String(digest));
            map.insert(
                format!("{key}_digest_algorithm"),
                Value::String(local_runtime::AUDIT_DIGEST_ALGORITHM.to_string()),
            );
        }
    }
    Ok(map)
}

fn add_actor_attribution(actor: &Actor, detail: &mut Map<String, Value>) {
    if let Actor::McpClient {
        client_id,
        conversation_handle,
    } = actor
    {
        detail
            .entry("client_id".to_string())
            .or_insert_with(|| Value::String(client_id.as_str().to_string()));
        detail
            .entry("conversation_handle".to_string())
            .or_insert_with(|| {
                conversation_handle
                    .as_ref()
                    .map(|handle| Value::String(handle.as_str().to_string()))
                    .unwrap_or(Value::Null)
            });
    }
}

fn actor_kind(actor: &Actor) -> &'static str {
    match actor {
        Actor::Agent => "agent",
        Actor::User => "user",
        Actor::Admin => "admin",
        Actor::System => "system",
        Actor::SurfaceClient { .. } => "surface_client",
        Actor::McpClient { .. } => "mcp_client",
    }
}

fn insert_outbox(
    event: &str,
    detail_json: &str,
    actor_kind: &str,
    request_id: Option<&str>,
) -> Result<(), String> {
    let event = event.to_string();
    let detail_json = detail_json.to_string();
    let actor_kind = actor_kind.to_string();
    let request_id = request_id.map(str::to_string);
    let created_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);

    if let Some(service) = crate::db_service::try_global() {
        let writer = service.writer();
        return writer
            .call_sync(move |conn| {
                insert_outbox_row(
                    conn,
                    &event,
                    &detail_json,
                    &actor_kind,
                    request_id.as_deref(),
                    &created_at,
                )
            })
            .map_err(|error| error.to_string());
    }

    let db = crate::db::ActionDb::open(Arc::new(crate::db::LocalKeychain::new()))
        .map_err(|error| error.to_string())?;
    insert_outbox_row(
        db.conn_ref(),
        &event,
        &detail_json,
        &actor_kind,
        request_id.as_deref(),
        &created_at,
    )
    .map_err(|error| error.to_string())
}

fn insert_outbox_row(
    conn: &rusqlite::Connection,
    event: &str,
    detail_json: &str,
    actor_kind: &str,
    request_id: Option<&str>,
    created_at: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO mcp_audit_outbox \
         (event, detail_json, actor_kind, request_id, created_at, drained_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, NULL)",
        params![event, detail_json, actor_kind, request_id, created_at],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use abilities_runtime::abilities::registry::{
        McpClientId as RuntimeMcpClientId,
        OpaqueConversationHandle as RuntimeOpaqueConversationHandle,
    };
    use serde_json::json;

    fn actor() -> Actor {
        Actor::McpClient {
            client_id: RuntimeMcpClientId::new("client-a"),
            conversation_handle: Some(RuntimeOpaqueConversationHandle::new("conv-a")),
        }
    }

    #[test]
    fn read_detail_persists_keyed_payload_digests() {
        local_runtime::with_audit_digest_key_for_tests([9_u8; 32], || {
            let detail = json!({
                "tool_name": "dailyos.read.account_status",
                "params": { "subject": "Acme" },
                "response": { "status": "active" }
            });
            let out = detail_with_attribution(&actor(), detail, Side::Read).unwrap();
            assert!(out.get("params").is_none());
            assert!(out.get("response").is_none());
            assert_eq!(out["params_digest"].as_str().unwrap().len(), 64);
            assert_eq!(out["response_digest"].as_str().unwrap().len(), 64);
            assert_eq!(
                out["params_digest_algorithm"],
                local_runtime::AUDIT_DIGEST_ALGORITHM
            );
            assert_eq!(out["client_id"], "client-a");
        });
    }

    #[test]
    fn write_detail_masks_payload_keys() {
        let detail = json!({
            "tool_name": "dailyos.submit.note",
            "params": { "text": "sensitive note" },
            "response": { "note_id": "note-1" },
            "mutation_cursor": { "note_id": "note-1" }
        });
        let out = detail_with_attribution(&actor(), detail, Side::SubmitCorrection).unwrap();
        assert!(out.get("params").is_none());
        assert!(out.get("response").is_none());
        assert_eq!(out["mutation_cursor"]["note_id"], "note-1");
        assert_eq!(out["conversation_handle"], "conv-a");
    }
}
