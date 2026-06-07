//! Shared helpers for W5 MCP tool handlers.

use serde_json::{json, Value};

use crate::db::ActionDb;
use crate::services::mcp_v2::contracts::{
    McpActor, McpClientId, OpaqueConversationHandle, ScopedName, ToolError,
};
use crate::services::mcp_v2::local_runtime;
use crate::services::mcp_v2::target_handles;

const MCP_SUBMIT_REPLAY_NAMESPACE: &str = "7f8a3cf2-0c26-5879-9c04-1e1f7d43cfaa";

const RAW_ID_PARAM_KEYS: &[&str] = &[
    "claim_id",
    "claimId",
    "action_id",
    "actionId",
    "entity_id",
    "entityId",
    "account_id",
    "accountId",
    "project_id",
    "projectId",
    "person_id",
    "personId",
    "meeting_id",
    "meetingId",
    "source_id",
    "sourceId",
    "source_ref",
    "sourceRef",
    "workspace_id",
    "workspaceId",
    "file_path",
    "filePath",
    "path",
    "actor",
    "actor_id",
    "actorId",
    "surface",
    "sensitivity",
    "idempotency_key",
    "idempotencyKey",
    "granted_scopes",
    "grantedScopes",
    "conversation_handle",
    "conversationHandle",
    "tool_side",
    "toolSide",
];

pub fn required_str(params: &Value, key: &str) -> Result<String, ToolError> {
    let value = params
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ToolError::BadParams {
            detail: format!("missing `{key}` parameter"),
        })?;
    Ok(value.to_string())
}

pub fn optional_str(params: &Value, key: &str) -> Result<Option<String>, ToolError> {
    let Some(value) = params.get(key) else {
        return Ok(None);
    };
    let Some(value) = value.as_str() else {
        return Err(ToolError::BadParams {
            detail: format!("`{key}` must be a string"),
        });
    };
    let value = value.trim();
    if value.is_empty() {
        Ok(None)
    } else {
        Ok(Some(value.to_string()))
    }
}

pub fn reject_raw_id_params(params: &Value) -> Result<(), ToolError> {
    let Some(object) = params.as_object() else {
        return Err(ToolError::BadParams {
            detail: "tool parameters must be an object".to_string(),
        });
    };
    for key in RAW_ID_PARAM_KEYS {
        if object.contains_key(*key) {
            return Err(ToolError::BadParams {
                detail: format!(
                    "`{key}` is not accepted by this MCP tool; use server-issued handles"
                ),
            });
        }
    }
    Ok(())
}

pub fn validate_yyyy_mm_dd(value: &str, key: &str) -> Result<(), ToolError> {
    crate::util::validate_yyyy_mm_dd(value, key)
        .map(|_| ())
        .map_err(|detail| ToolError::BadParams { detail })
}

pub fn validate_bounded_string(
    value: &str,
    key: &str,
    min_chars: usize,
    max_chars: usize,
) -> Result<String, ToolError> {
    crate::util::validate_bounded_string(value, key, min_chars, max_chars)
        .map_err(|detail| ToolError::BadParams { detail })
}

pub fn bad_params(detail: impl Into<String>) -> ToolError {
    ToolError::BadParams {
        detail: detail.into(),
    }
}

pub fn actor_origin(
    actor: &McpActor,
) -> Result<(&McpClientId, &OpaqueConversationHandle, &ScopedName), ToolError> {
    let McpActor::Client {
        client_id,
        conversation_handle,
        tool_name,
        ..
    } = actor;
    let conversation_handle = conversation_handle
        .as_ref()
        .ok_or_else(|| bad_params("MCP target handles require a conversation handle"))?;
    Ok((client_id, conversation_handle, tool_name))
}

pub fn actor_origin_hashes(actor: &McpActor) -> Result<(String, String, String), ToolError> {
    let (client_id, conversation_handle, tool_name) = actor_origin(actor)?;
    let client_id_hash = local_runtime::target_handle_hmac_hex(client_id.as_str())
        .map_err(|error| internal_trace("mcp_client_hash", error))?;
    let conversation_handle_hash =
        local_runtime::target_handle_hmac_hex(conversation_handle.as_str())
            .map_err(|error| internal_trace("mcp_conversation_hash", error))?;
    Ok((
        client_id_hash,
        conversation_handle_hash,
        tool_name.as_str().to_string(),
    ))
}

pub fn internal_trace(prefix: &str, detail: impl std::fmt::Display) -> ToolError {
    let digest = sha256_hex(&detail.to_string());
    ToolError::Internal {
        trace_id: format!("{prefix}:{digest}"),
    }
}

pub fn no_op_mutation_cursor(tool_name: &ScopedName, status: &str, handle: Option<&str>) -> Value {
    let handle_hash = handle
        .and_then(|handle| target_handles::public_handle_hash(handle).ok())
        .unwrap_or_else(|| "none".to_string());
    json!({
        "schema_version": "mcp.mutation_cursor.v1",
        "tool_name": tool_name.as_str(),
        "status": status,
        "target_handle_hash": handle_hash,
    })
}

pub fn mutation_cursor_for_target(
    tool_name: &ScopedName,
    status: &str,
    target_kind: &str,
    target_handle: &str,
) -> Value {
    let target_handle_hash = target_handles::public_handle_hash(target_handle)
        .unwrap_or_else(|_| sha256_hex(target_handle));
    json!({
        "schema_version": "mcp.mutation_cursor.v1",
        "tool_name": tool_name.as_str(),
        "status": status,
        "target_kind": target_kind,
        "target_handle_hash": target_handle_hash,
    })
}

pub fn unavailable_payload(
    schema_version: &str,
    tool_name: &ScopedName,
    refresh_required: bool,
    include_cursor_for_handle: Option<&str>,
) -> Value {
    let mut payload = json!({
        "schema_version": schema_version,
        "tool_name": tool_name.as_str(),
        "status": "unavailable",
        "reason": "target_unavailable",
        "refresh_required": refresh_required,
    });
    if let Some(handle) = include_cursor_for_handle {
        payload["mutation_cursor"] = no_op_mutation_cursor(tool_name, "unavailable", Some(handle));
    }
    payload
}

pub fn canonical_json_string(value: &Value) -> String {
    serde_json::to_string(&canonical_json_value(value)).unwrap_or_else(|_| "null".to_string())
}

pub fn source_provenance_watermark(target_ref: &Value) -> String {
    format!("source_provenance:{}", canonical_json_string(target_ref))
}

pub fn entity_watermark_from_parts(
    entity_type: &str,
    entity_id: &str,
    updated_at: &str,
    archived: bool,
) -> String {
    format!("entity:{entity_type}:{entity_id}:{updated_at}:archived={archived}")
}

pub fn current_entity_watermark(
    db: &ActionDb,
    target_ref: &Value,
) -> Result<Option<String>, ToolError> {
    let entity_type = target_ref
        .get("entity_type")
        .and_then(Value::as_str)
        .ok_or_else(|| bad_params("entity handle is unavailable"))?;
    let entity_id = target_ref
        .get("entity_id")
        .and_then(Value::as_str)
        .ok_or_else(|| bad_params("entity handle is unavailable"))?;
    match entity_type {
        "account" => db
            .get_account(entity_id)
            .map_err(|error| ToolError::UpstreamFailure {
                detail: error.to_string(),
            })
            .map(|account| {
                account.and_then(|account| {
                    (!account.archived).then(|| {
                        entity_watermark_from_parts(
                            "account",
                            &account.id,
                            &account.updated_at,
                            account.archived,
                        )
                    })
                })
            }),
        "project" => db
            .get_project(entity_id)
            .map_err(|error| ToolError::UpstreamFailure {
                detail: error.to_string(),
            })
            .map(|project| {
                project.and_then(|project| {
                    (!project.archived).then(|| {
                        entity_watermark_from_parts(
                            "project",
                            &project.id,
                            &project.updated_at,
                            project.archived,
                        )
                    })
                })
            }),
        "person" => db
            .get_person(entity_id)
            .map_err(|error| ToolError::UpstreamFailure {
                detail: error.to_string(),
            })
            .map(|person| {
                person.and_then(|person| {
                    (!person.archived).then(|| {
                        entity_watermark_from_parts(
                            "person",
                            &person.id,
                            &person.updated_at,
                            person.archived,
                        )
                    })
                })
            }),
        _ => Err(bad_params("entity handle cannot target this MCP tool")),
    }
}

pub fn mcp_submit_replay_key(
    actor: &McpActor,
    tool_name: &ScopedName,
    purpose: &str,
    material: &Value,
) -> Result<String, ToolError> {
    use sha2::{Digest, Sha256};

    let (client_id_hash, conversation_handle_hash, actor_tool_name) = actor_origin_hashes(actor)?;
    let canonical_material = canonical_json_string(material);
    let mut hasher = Sha256::new();
    for part in [
        "dailyos.mcp.submit.replay.v1",
        purpose,
        tool_name.as_str(),
        &actor_tool_name,
        &client_id_hash,
        &conversation_handle_hash,
        &canonical_material,
    ] {
        hasher.update(part.len().to_string().as_bytes());
        hasher.update(b":");
        hasher.update(part.as_bytes());
        hasher.update(b"\0");
    }
    Ok(format!("mcp_submit_{:x}", hasher.finalize()))
}

pub fn deterministic_uuid_from_replay_key(
    kind: &str,
    replay_key: &str,
) -> Result<String, ToolError> {
    let namespace = uuid::Uuid::parse_str(MCP_SUBMIT_REPLAY_NAMESPACE)
        .map_err(|error| internal_trace("mcp_submit_replay_namespace", error))?;
    Ok(uuid::Uuid::new_v5(&namespace, format!("{kind}:{replay_key}").as_bytes()).to_string())
}

fn canonical_json_value(value: &Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.iter().map(canonical_json_value).collect()),
        Value::Object(object) => {
            let mut entries = object.iter().collect::<Vec<_>>();
            entries.sort_by_key(|(key, _)| *key);
            let mut sorted = serde_json::Map::new();
            for (key, value) in entries {
                sorted.insert(key.clone(), canonical_json_value(value));
            }
            Value::Object(sorted)
        }
        _ => value.clone(),
    }
}

fn sha256_hex(value: &str) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    format!("{:x}", hasher.finalize())
}
