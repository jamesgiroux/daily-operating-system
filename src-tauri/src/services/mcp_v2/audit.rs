//! MCP v2 audit log writer.
//!
//! This module is intentionally a thin MCP-specific layer over
//! [`crate::audit_log::AuditLogger::append_with_actor`]. It owns the MCP
//! success-row privacy contract: raw request/response payloads are converted
//! to keyed HMAC-SHA256 hashes before anything reaches JSONL audit storage.

use std::process::Command;
use std::sync::Arc;

use chrono::{SecondsFormat, Utc};
use rand::Rng;
use ring::hmac;
use rusqlite::params;
use serde_json::{Map, Value};
use zeroize::Zeroizing;

use abilities_runtime::abilities::registry::Actor;

use crate::audit_log::{AuditError as AuditLogError, AuditFields, AuditLogger};

const AUDIT_KEY_SERVICE: &str = "com.dailyos.desktop.mcp-v2-audit";
const AUDIT_KEY_ACCOUNT: &str = "mcp-v2-audit-hmac-key";
const AUDIT_KEY_BYTES: usize = 32;
const PARAM_PAYLOAD_KEYS: &[&str] = &["params", "parameters"];
const RESPONSE_PAYLOAD_KEYS: &[&str] = &["response", "response_or_error", "result"];

/// MCP-specific audit write failures.
#[derive(Debug, thiserror::Error)]
pub enum AuditError {
    #[error("audit key unavailable: {0}")]
    Keychain(String),
    #[error("audit detail serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("audit log append failed: {0}")]
    Append(String),
    #[error("audit log append failed and audit outbox insert also failed: append={append_error}; outbox={outbox_error}")]
    DoubleFailure {
        append_error: String,
        outbox_error: String,
    },
}

/// Write an MCP success-row audit event.
///
/// `detail` may include raw top-level `params` and `response` /
/// `response_or_error` / `result` fields. Those fields are never persisted:
/// this helper replaces them with `params_hash` and `response_hash`, computed
/// with a per-install key stored in Keychain.
pub fn write(
    actor: &Actor,
    event: &str,
    detail: Value,
    request_id: Option<String>,
) -> Result<(), AuditError> {
    let audit_key = load_or_create_audit_key()?;
    let detail_with_hashes = detail_with_hashes(actor, detail, &audit_key)?;

    let mut fields = AuditFields::new("security", detail_with_hashes.clone());
    if let Some(id) = request_id.clone() {
        fields = fields.with_request_id(id);
    }

    let mut logger = AuditLogger::new(crate::audit_log::default_audit_log_path());
    match logger.append_with_actor(event, actor, fields) {
        Ok(()) => Ok(()),
        Err(AuditLogError::Write(append_error)) => {
            let detail_json = serde_json::to_string(&detail_with_hashes)?;
            insert_outbox(event, &detail_json, actor_kind(actor), request_id.as_deref()).map_err(
                |outbox_error| AuditError::DoubleFailure {
                    append_error,
                    outbox_error,
                },
            )
        }
        Err(error) => Err(AuditError::Append(error.to_string())),
    }
}

fn detail_with_hashes(
    actor: &Actor,
    detail: Value,
    audit_key: &[u8; AUDIT_KEY_BYTES],
) -> Result<Value, serde_json::Error> {
    let params = payload_for_keys(&detail, PARAM_PAYLOAD_KEYS);
    let response = payload_for_keys(&detail, RESPONSE_PAYLOAD_KEYS);

    let mut sanitized = sanitize_detail(detail);
    sanitized.insert(
        "params_hash".to_string(),
        Value::String(hmac_json(audit_key, &params)?),
    );
    sanitized.insert(
        "response_hash".to_string(),
        Value::String(hmac_json(audit_key, &response)?),
    );
    add_actor_attribution(actor, &mut sanitized);

    Ok(Value::Object(sanitized))
}

fn payload_for_keys(detail: &Value, keys: &[&str]) -> Value {
    let Some(map) = detail.as_object() else {
        return Value::Null;
    };
    keys.iter()
        .find_map(|key| map.get(*key).cloned())
        .unwrap_or(Value::Null)
}

fn sanitize_detail(detail: Value) -> Map<String, Value> {
    let mut map = match detail {
        Value::Object(map) => map,
        _ => Map::new(),
    };
    for key in PARAM_PAYLOAD_KEYS
        .iter()
        .chain(RESPONSE_PAYLOAD_KEYS.iter())
        .copied()
    {
        map.remove(key);
    }
    map
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

fn hmac_json(key_bytes: &[u8; AUDIT_KEY_BYTES], value: &Value) -> Result<String, serde_json::Error> {
    let canonical = serde_json::to_vec(&canonical_value(value))?;
    let key = hmac::Key::new(hmac::HMAC_SHA256, key_bytes);
    Ok(hex::encode(hmac::sign(&key, &canonical).as_ref()))
}

fn canonical_value(value: &Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.iter().map(canonical_value).collect()),
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let mut ordered = Map::new();
            for key in keys {
                if let Some(value) = map.get(key) {
                    ordered.insert(key.clone(), canonical_value(value));
                }
            }
            Value::Object(ordered)
        }
        _ => value.clone(),
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

fn load_or_create_audit_key() -> Result<Zeroizing<[u8; AUDIT_KEY_BYTES]>, AuditError> {
    match read_audit_key() {
        Ok(Some(key)) => Ok(Zeroizing::new(key)),
        Ok(None) => {
            let key = generate_audit_key();
            persist_audit_key(&key)?;
            Ok(Zeroizing::new(key))
        }
        Err(error) => Err(AuditError::Keychain(error)),
    }
}

fn read_audit_key() -> Result<Option<[u8; AUDIT_KEY_BYTES]>, String> {
    let output = run_security(&[
        "find-generic-password",
        "-s",
        AUDIT_KEY_SERVICE,
        "-a",
        AUDIT_KEY_ACCOUNT,
        "-w",
    ])?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if is_keychain_item_not_found(&stderr) {
            return Ok(None);
        }
        return Err(format!("keychain read failed: {}", stderr.trim()));
    }

    // L2 cycle-4 codex review class-pattern: audit HMAC key intermediates
    // must wrap in Zeroizing to match the AC-11 contract sweep applied to
    // auth.rs (cycle-3 + cycle-4 fixes).
    let stdout: Zeroizing<Vec<u8>> = Zeroizing::new(output.stdout);
    // L2 cycle-5 codex review NEW: use borrowing str::from_utf8 to avoid
    // a clone-into-FromUtf8Error path that would drop unzeroized on UTF-8
    // failure. Mirrors auth.rs::load_transport_key.
    let hex_key: Zeroizing<String> = Zeroizing::new(
        std::str::from_utf8(&stdout)
            .map_err(|error| format!("keychain returned non-UTF-8 key: {error}"))?
            .to_owned(),
    );
    decode_key(hex_key.trim()).map(Some)
}

fn persist_audit_key(key: &[u8; AUDIT_KEY_BYTES]) -> Result<(), AuditError> {
    // L2 cycle-4 codex review class-pattern: see read_audit_key comment.
    let key_hex: Zeroizing<String> = Zeroizing::new(hex::encode(key));
    let output = run_security(&[
        "add-generic-password",
        "-s",
        AUDIT_KEY_SERVICE,
        "-a",
        AUDIT_KEY_ACCOUNT,
        "-w",
        key_hex.as_str(),
        "-U",
    ])
    .map_err(AuditError::Keychain)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AuditError::Keychain(format!(
            "keychain write failed: {}",
            stderr.trim()
        )));
    }
    Ok(())
}

fn run_security(args: &[&str]) -> Result<std::process::Output, String> {
    Command::new("security")
        .args(args)
        .output()
        .map_err(|error| format!("failed to run security CLI: {error}"))
}

fn is_keychain_item_not_found(stderr: &str) -> bool {
    let stderr = stderr.to_ascii_lowercase();
    stderr.contains("could not be found")
        || stderr.contains("item not found")
        || stderr.contains("-25300")
}

fn decode_key(hex_key: &str) -> Result<[u8; AUDIT_KEY_BYTES], String> {
    // L2 cycle-4 codex review class-pattern: decoded key bytes must wrap
    // in Zeroizing before the copy_from_slice into the fixed-size array,
    // matching the AC-11 sweep applied to auth.rs::load_transport_key.
    let bytes: Zeroizing<Vec<u8>> = Zeroizing::new(
        hex::decode(hex_key).map_err(|error| format!("invalid audit key hex: {error}"))?,
    );
    if bytes.len() != AUDIT_KEY_BYTES {
        return Err(format!(
            "audit key length mismatch: expected {AUDIT_KEY_BYTES}, got {}",
            bytes.len()
        ));
    }
    let mut key = [0u8; AUDIT_KEY_BYTES];
    key.copy_from_slice(&bytes);
    Ok(key)
}

fn generate_audit_key() -> [u8; AUDIT_KEY_BYTES] {
    let mut key = [0u8; AUDIT_KEY_BYTES];
    rand::rng().fill_bytes(&mut key);
    key
}
