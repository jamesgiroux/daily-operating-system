//! Registry-backed opaque target handles for MCP v2.
//!
//! Public MCP payloads carry `mth_...` handles. The database stores only a
//! keyed lookup hash plus AEAD-sealed target refs, so raw claim/action/entity
//! ids never appear in public responses, audit detail, or plaintext indexes.

use base64::Engine as _;
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use rand::Rng;
use ring::aead;
use rusqlite::{params, OptionalExtension};
use serde_json::Value;

use crate::db::ActionDb;

use super::contracts::{McpActor, McpClientId, OpaqueConversationHandle, ScopedName};
use super::local_runtime::{self, LocalRuntimeError};

const HANDLE_PREFIX: &str = "mth_";
const HANDLE_RANDOM_BYTES: usize = 24;
const TARGET_REF_KEY_VERSION: i64 = 1;
const TARGET_REF_MAGIC: &[u8] = b"DOSMTH1";
const NONCE_BYTES: usize = 12;
const DEFAULT_RENDER_POLICY_VERSION: &str = "mcp_target_handle_v1";
const HANDLE_TTL: Duration = Duration::hours(24);
const CLEANUP_RETENTION: Duration = Duration::days(7);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetKind {
    Claim,
    Action,
    Entity,
    SourceProvenance,
    WorkspaceSource,
}

impl TargetKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Claim => "claim",
            Self::Action => "action",
            Self::Entity => "entity",
            Self::SourceProvenance => "source_provenance",
            Self::WorkspaceSource => "workspace_source",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "claim" => Self::Claim,
            "action" => Self::Action,
            "entity" => Self::Entity,
            "source_provenance" => Self::SourceProvenance,
            "workspace_source" => Self::WorkspaceSource,
            _ => return None,
        })
    }
}

#[derive(Debug)]
pub struct MintTargetHandle<'a> {
    pub actor: &'a McpActor,
    pub originating_tool: &'a ScopedName,
    pub result_item_path: &'a str,
    pub target_kind: TargetKind,
    pub target_ref: Value,
    pub sensitivity_tier: &'a str,
    pub provenance_material: &'a str,
    pub watermark_material: &'a str,
}

#[derive(Debug)]
pub struct ResolveTargetHandle<'a> {
    pub actor: &'a McpActor,
    pub handle: &'a str,
    pub expected_kind: TargetKind,
    pub current_watermark_material: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedTargetHandle {
    pub target_ref: Value,
    pub result_item_path: String,
    pub originating_tool: String,
    pub render_policy_version: String,
    pub sensitivity_tier: String,
    target_watermark_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetHandleResolutionError {
    Unavailable { refresh_required: bool },
    Internal(String),
}

impl TargetHandleResolutionError {
    pub fn unavailable() -> Self {
        Self::Unavailable {
            refresh_required: false,
        }
    }

    pub fn refresh_required() -> Self {
        Self::Unavailable {
            refresh_required: true,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TargetHandleError {
    #[error("target handle actor is not an MCP client")]
    Actor,
    #[error("target handle requires an MCP conversation handle")]
    MissingConversation,
    #[error("target handle key unavailable: {0}")]
    LocalRuntime(#[from] LocalRuntimeError),
    #[error("target handle serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("target handle encryption failed")]
    Crypto,
    #[error("target handle database write failed: {0}")]
    Db(#[from] rusqlite::Error),
}

pub fn public_handle_hash(handle: &str) -> Result<String, LocalRuntimeError> {
    local_runtime::target_handle_hmac_hex(handle)
}

pub fn mint_target_handle(
    db: &ActionDb,
    input: MintTargetHandle<'_>,
) -> Result<String, TargetHandleError> {
    mint_target_handle_with_mode(db, input, false)
}

pub fn mint_replacing_target_handle(
    db: &ActionDb,
    input: MintTargetHandle<'_>,
) -> Result<String, TargetHandleError> {
    mint_target_handle_with_mode(db, input, true)
}

fn mint_target_handle_with_mode(
    db: &ActionDb,
    input: MintTargetHandle<'_>,
    replace_existing: bool,
) -> Result<String, TargetHandleError> {
    let (client_id, conversation_handle) = actor_parts(input.actor)?;
    let handle = mint_public_handle();
    let handle_lookup_hash = local_runtime::target_handle_hmac_hex(&handle)?;
    let conversation_handle_hash =
        local_runtime::target_handle_hmac_hex(conversation_handle.as_str())?;
    let provenance_hash = local_runtime::target_handle_hmac_hex(input.provenance_material)?;
    let target_watermark_hash = local_runtime::target_handle_hmac_hex(input.watermark_material)?;
    let render_policy_version = DEFAULT_RENDER_POLICY_VERSION;
    let ciphertext = seal_target_ref(
        &input.target_ref,
        &target_ref_aad(
            client_id,
            &conversation_handle_hash,
            input.originating_tool,
            input.target_kind,
            render_policy_version,
        ),
    )?;
    let now = Utc::now();
    let created_at = rfc3339(now);
    let expires_at = rfc3339(now + HANDLE_TTL);

    if replace_existing {
        db.conn_ref().execute(
            "DELETE FROM mcp_target_handles
              WHERE client_id = ?1
                AND conversation_handle_hash = ?2
                AND originating_tool = ?3
                AND result_item_path = ?4
                AND target_kind = ?5
                AND provenance_hash = ?6
                AND target_watermark_hash = ?7",
            params![
                client_id.as_str(),
                conversation_handle_hash,
                input.originating_tool.as_str(),
                input.result_item_path,
                input.target_kind.as_str(),
                provenance_hash,
                target_watermark_hash,
            ],
        )?;
    }

    db.conn_ref().execute(
        "INSERT INTO mcp_target_handles (
            handle_lookup_hash, client_id, conversation_handle_hash, originating_tool,
            result_item_path, target_kind, target_ref_ciphertext, target_ref_key_version,
            render_policy_version, sensitivity_tier, provenance_hash, target_watermark_hash,
            created_at, last_used_at, expires_at, revoked_at, revoked_reason_code
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13, ?14, NULL, NULL)",
        params![
            handle_lookup_hash,
            client_id.as_str(),
            conversation_handle_hash,
            input.originating_tool.as_str(),
            input.result_item_path,
            input.target_kind.as_str(),
            ciphertext,
            TARGET_REF_KEY_VERSION,
            render_policy_version,
            input.sensitivity_tier,
            provenance_hash,
            target_watermark_hash,
            created_at,
            expires_at,
        ],
    )?;

    Ok(handle)
}

pub fn resolve_target_handle(
    db: &ActionDb,
    input: ResolveTargetHandle<'_>,
) -> Result<ResolvedTargetHandle, TargetHandleResolutionError> {
    if !input.handle.starts_with(HANDLE_PREFIX) {
        return Err(TargetHandleResolutionError::unavailable());
    }

    let (client_id, conversation_handle) = actor_parts(input.actor)
        .map_err(|error| TargetHandleResolutionError::Internal(error.to_string()))?;
    let handle_lookup_hash = public_handle_hash(input.handle)
        .map_err(|error| TargetHandleResolutionError::Internal(error.to_string()))?;
    let expected_conversation_hash =
        local_runtime::target_handle_hmac_hex(conversation_handle.as_str())
            .map_err(|error| TargetHandleResolutionError::Internal(error.to_string()))?;
    let row = load_row(db, &handle_lookup_hash)
        .map_err(|error| TargetHandleResolutionError::Internal(error.to_string()))?
        .ok_or_else(TargetHandleResolutionError::unavailable)?;

    if row.client_id != client_id.as_str()
        || row.conversation_handle_hash != expected_conversation_hash
        || row.revoked_at.is_some()
    {
        return Err(TargetHandleResolutionError::unavailable());
    }
    if row.target_kind != input.expected_kind {
        return Err(TargetHandleResolutionError::unavailable());
    }
    if parse_rfc3339(&row.expires_at)
        .map(|expires_at| expires_at < Utc::now())
        .unwrap_or(true)
    {
        return Err(TargetHandleResolutionError::unavailable());
    }
    if let Some(material) = input.current_watermark_material {
        let current = local_runtime::target_handle_hmac_hex(material)
            .map_err(|error| TargetHandleResolutionError::Internal(error.to_string()))?;
        if current != row.target_watermark_hash {
            return Err(TargetHandleResolutionError::refresh_required());
        }
    }

    let target_ref = open_target_ref(
        &row.target_ref_ciphertext,
        &target_ref_aad(
            client_id,
            &row.conversation_handle_hash,
            &ScopedName::new(row.originating_tool.clone()),
            row.target_kind,
            &row.render_policy_version,
        ),
    )
    .map_err(|error| TargetHandleResolutionError::Internal(error.to_string()))?;

    let now = Utc::now();
    let new_expires_at = rfc3339(now + HANDLE_TTL);
    db.conn_ref()
        .execute(
            "UPDATE mcp_target_handles
                SET last_used_at = ?2, expires_at = ?3
              WHERE handle_lookup_hash = ?1",
            params![handle_lookup_hash, rfc3339(now), new_expires_at],
        )
        .map_err(|error| TargetHandleResolutionError::Internal(error.to_string()))?;

    Ok(ResolvedTargetHandle {
        target_ref,
        result_item_path: row.result_item_path,
        originating_tool: row.originating_tool,
        render_policy_version: row.render_policy_version,
        sensitivity_tier: row.sensitivity_tier,
        target_watermark_hash: row.target_watermark_hash,
    })
}

pub fn resolved_target_watermark_matches(
    resolved: &ResolvedTargetHandle,
    current_watermark_material: &str,
) -> Result<bool, LocalRuntimeError> {
    let current = local_runtime::target_handle_hmac_hex(current_watermark_material)?;
    Ok(current == resolved.target_watermark_hash)
}

pub fn cleanup_target_handles(db: &ActionDb) -> Result<usize, rusqlite::Error> {
    let cutoff = rfc3339(Utc::now() - CLEANUP_RETENTION);
    db.conn_ref().execute(
        "DELETE FROM mcp_target_handles
          WHERE expires_at < ?1
             OR (revoked_at IS NOT NULL AND revoked_at < ?1)",
        params![cutoff],
    )
}

fn actor_parts(
    actor: &McpActor,
) -> Result<(&McpClientId, &OpaqueConversationHandle), TargetHandleError> {
    let McpActor::Client {
        client_id,
        conversation_handle,
        ..
    } = actor;
    let conversation_handle = conversation_handle
        .as_ref()
        .ok_or(TargetHandleError::MissingConversation)?;
    Ok((client_id, conversation_handle))
}

#[derive(Debug)]
struct TargetHandleRow {
    client_id: String,
    conversation_handle_hash: String,
    originating_tool: String,
    result_item_path: String,
    target_kind: TargetKind,
    target_ref_ciphertext: Vec<u8>,
    render_policy_version: String,
    sensitivity_tier: String,
    target_watermark_hash: String,
    expires_at: String,
    revoked_at: Option<String>,
}

fn load_row(
    db: &ActionDb,
    handle_lookup_hash: &str,
) -> Result<Option<TargetHandleRow>, rusqlite::Error> {
    db.conn_ref()
        .query_row(
            "SELECT client_id, conversation_handle_hash, originating_tool,
                    result_item_path, target_kind, target_ref_ciphertext,
                    render_policy_version, sensitivity_tier, target_watermark_hash,
                    expires_at, revoked_at
               FROM mcp_target_handles
              WHERE handle_lookup_hash = ?1",
            params![handle_lookup_hash],
            |row| {
                let kind_raw: String = row.get(4)?;
                let target_kind = TargetKind::parse(&kind_raw).ok_or_else(|| {
                    rusqlite::Error::InvalidColumnType(
                        4,
                        "target_kind".to_string(),
                        rusqlite::types::Type::Text,
                    )
                })?;
                Ok(TargetHandleRow {
                    client_id: row.get(0)?,
                    conversation_handle_hash: row.get(1)?,
                    originating_tool: row.get(2)?,
                    result_item_path: row.get(3)?,
                    target_kind,
                    target_ref_ciphertext: row.get(5)?,
                    render_policy_version: row.get(6)?,
                    sensitivity_tier: row.get(7)?,
                    target_watermark_hash: row.get(8)?,
                    expires_at: row.get(9)?,
                    revoked_at: row.get(10)?,
                })
            },
        )
        .optional()
}

fn mint_public_handle() -> String {
    let mut bytes = [0_u8; HANDLE_RANDOM_BYTES];
    rand::rng().fill_bytes(&mut bytes);
    format!(
        "{HANDLE_PREFIX}{}",
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
    )
}

fn seal_target_ref(target_ref: &Value, aad: &str) -> Result<Vec<u8>, TargetHandleError> {
    let key_bytes = local_runtime::target_handle_key_v1()?;
    let unbound = aead::UnboundKey::new(&aead::AES_256_GCM, &key_bytes)
        .map_err(|_| TargetHandleError::Crypto)?;
    let key = aead::LessSafeKey::new(unbound);
    let mut nonce_bytes = [0_u8; NONCE_BYTES];
    rand::rng().fill_bytes(&mut nonce_bytes);
    let nonce = aead::Nonce::assume_unique_for_key(nonce_bytes);
    let mut in_out = serde_json::to_vec(target_ref)?;
    key.seal_in_place_append_tag(nonce, aead::Aad::from(aad.as_bytes()), &mut in_out)
        .map_err(|_| TargetHandleError::Crypto)?;

    let mut blob = Vec::with_capacity(TARGET_REF_MAGIC.len() + NONCE_BYTES + in_out.len());
    blob.extend_from_slice(TARGET_REF_MAGIC);
    blob.extend_from_slice(&nonce_bytes);
    blob.extend_from_slice(&in_out);
    Ok(blob)
}

fn open_target_ref(ciphertext: &[u8], aad: &str) -> Result<Value, TargetHandleError> {
    if ciphertext.len() <= TARGET_REF_MAGIC.len() + NONCE_BYTES
        || !ciphertext.starts_with(TARGET_REF_MAGIC)
    {
        return Err(TargetHandleError::Crypto);
    }
    let mut nonce_bytes = [0_u8; NONCE_BYTES];
    nonce_bytes
        .copy_from_slice(&ciphertext[TARGET_REF_MAGIC.len()..TARGET_REF_MAGIC.len() + NONCE_BYTES]);
    let mut in_out = ciphertext[TARGET_REF_MAGIC.len() + NONCE_BYTES..].to_vec();
    let key_bytes = local_runtime::target_handle_key_v1()?;
    let unbound = aead::UnboundKey::new(&aead::AES_256_GCM, &key_bytes)
        .map_err(|_| TargetHandleError::Crypto)?;
    let key = aead::LessSafeKey::new(unbound);
    let plaintext = key
        .open_in_place(
            aead::Nonce::assume_unique_for_key(nonce_bytes),
            aead::Aad::from(aad.as_bytes()),
            &mut in_out,
        )
        .map_err(|_| TargetHandleError::Crypto)?;
    serde_json::from_slice(plaintext).map_err(TargetHandleError::from)
}

fn target_ref_aad(
    client_id: &McpClientId,
    conversation_handle_hash: &str,
    originating_tool: &ScopedName,
    target_kind: TargetKind,
    render_policy_version: &str,
) -> String {
    format!(
        "dailyos.mcp.target-ref.v1\0{}\0{}\0{}\0{}\0{}",
        client_id.as_str(),
        conversation_handle_hash,
        originating_tool.as_str(),
        target_kind.as_str(),
        render_policy_version
    )
}

fn parse_rfc3339(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|value| value.with_timezone(&Utc))
}

fn rfc3339(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Millis, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::mcp_v2::contracts::Scope;

    fn actor(client: &str, conversation: &str) -> McpActor {
        McpActor::Client {
            client_id: McpClientId::new(client),
            conversation_handle: Some(OpaqueConversationHandle::new(conversation)),
            tool_name: ScopedName::new("dailyos.read.account_status"),
            granted_scopes: vec![Scope::new("dailyos.read.account_status")],
        }
    }

    fn db() -> ActionDb {
        let dir = tempfile::tempdir().expect("tempdir");
        ActionDb::open_at_unencrypted(dir.path().join("target-handles.db")).expect("open db")
    }

    fn with_key<R>(f: impl FnOnce() -> R) -> R {
        local_runtime::with_target_handle_key_for_tests([7_u8; 32], f)
    }

    #[test]
    fn mint_stores_only_hash_and_ciphertext_then_resolves_for_same_actor() {
        with_key(|| {
            let db = db();
            let primary_actor = actor("client-a", "conversation-a");
            let handle = mint_target_handle(
                &db,
                MintTargetHandle {
                    actor: &primary_actor,
                    originating_tool: &ScopedName::new("dailyos.read.account_status"),
                    result_item_path: "/assessment/facts/0",
                    target_kind: TargetKind::Claim,
                    target_ref: serde_json::json!({ "claim_id": "claim-secret-1" }),
                    sensitivity_tier: "internal",
                    provenance_material: "source-a",
                    watermark_material: "claim-secret-1:v1:active",
                },
            )
            .expect("mint handle");
            assert!(handle.starts_with(HANDLE_PREFIX));

            let plaintext_columns: String = db
                .conn_ref()
                .query_row(
                    "SELECT handle_lookup_hash || ' ' || client_id || ' ' ||
                            conversation_handle_hash || ' ' || originating_tool || ' ' ||
                            result_item_path || ' ' || target_kind || ' ' ||
                            render_policy_version || ' ' || sensitivity_tier || ' ' ||
                            provenance_hash || ' ' || target_watermark_hash
                       FROM mcp_target_handles",
                    [],
                    |row| row.get(0),
                )
                .expect("row");
            assert!(!plaintext_columns.contains(&handle));
            assert!(!plaintext_columns.contains("claim-secret-1"));
            assert!(!plaintext_columns.contains("conversation-a"));

            let resolved = resolve_target_handle(
                &db,
                ResolveTargetHandle {
                    actor: &primary_actor,
                    handle: &handle,
                    expected_kind: TargetKind::Claim,
                    current_watermark_material: Some("claim-secret-1:v1:active"),
                },
            )
            .expect("resolve handle");
            assert_eq!(resolved.target_ref["claim_id"], "claim-secret-1");
        });
    }

    #[test]
    fn replacing_mint_keeps_one_deterministic_row_and_revokes_old_public_handle() {
        with_key(|| {
            let db = db();
            let actor = actor("client-a", "conversation-a");
            let tool = ScopedName::new("dailyos.submit.action");
            let input = |path: &'static str| MintTargetHandle {
                actor: &actor,
                originating_tool: &tool,
                result_item_path: path,
                target_kind: TargetKind::Action,
                target_ref: serde_json::json!({ "action_id": "action-1" }),
                sensitivity_tier: "internal",
                provenance_material: "mcp_submit_action:action-1",
                watermark_material: "action:action-1:updated:pending",
            };

            let first = mint_replacing_target_handle(&db, input("/action")).expect("first handle");
            let second =
                mint_replacing_target_handle(&db, input("/action")).expect("second handle");
            assert_ne!(
                first, second,
                "replacement still returns a fresh public handle"
            );

            let row_count: i64 = db
                .conn_ref()
                .query_row("SELECT COUNT(*) FROM mcp_target_handles", [], |row| {
                    row.get(0)
                })
                .expect("row count");
            assert_eq!(
                row_count, 1,
                "deterministic replay must not accumulate handle rows"
            );

            assert_eq!(
                resolve_target_handle(
                    &db,
                    ResolveTargetHandle {
                        actor: &actor,
                        handle: &first,
                        expected_kind: TargetKind::Action,
                        current_watermark_material: Some("action:action-1:updated:pending"),
                    },
                ),
                Err(TargetHandleResolutionError::unavailable()),
                "the old public handle must not remain live after replacement"
            );
            let resolved = resolve_target_handle(
                &db,
                ResolveTargetHandle {
                    actor: &actor,
                    handle: &second,
                    expected_kind: TargetKind::Action,
                    current_watermark_material: Some("action:action-1:updated:pending"),
                },
            )
            .expect("replacement handle resolves");
            assert_eq!(resolved.target_ref["action_id"], "action-1");
        });
    }

    #[test]
    fn resolve_rejects_cross_client_and_stale_watermark_without_existence_hint() {
        with_key(|| {
            let db = db();
            let primary_actor = actor("client-a", "conversation-a");
            let handle = mint_target_handle(
                &db,
                MintTargetHandle {
                    actor: &primary_actor,
                    originating_tool: &ScopedName::new("dailyos.read.account_status"),
                    result_item_path: "/subject",
                    target_kind: TargetKind::Entity,
                    target_ref: serde_json::json!({ "entity_type": "account", "entity_id": "acct-1" }),
                    sensitivity_tier: "internal",
                    provenance_material: "entity:acct-1",
                    watermark_material: "account:acct-1:v1",
                },
            )
            .expect("mint handle");

            let wrong_actor = actor("client-b", "conversation-a");
            assert_eq!(
                resolve_target_handle(
                    &db,
                    ResolveTargetHandle {
                        actor: &wrong_actor,
                        handle: &handle,
                        expected_kind: TargetKind::Entity,
                        current_watermark_material: Some("account:acct-1:v1"),
                    },
                ),
                Err(TargetHandleResolutionError::unavailable())
            );

            assert_eq!(
                resolve_target_handle(
                    &db,
                    ResolveTargetHandle {
                        actor: &primary_actor,
                        handle: &handle,
                        expected_kind: TargetKind::Entity,
                        current_watermark_material: Some("account:acct-1:v2"),
                    },
                ),
                Err(TargetHandleResolutionError::refresh_required())
            );
        });
    }

    #[test]
    fn cleanup_keeps_fresh_expired_rows_and_drops_old_rows() {
        with_key(|| {
            let db = db();
            let actor = actor("client-a", "conversation-a");
            let fresh = mint_target_handle(
                &db,
                MintTargetHandle {
                    actor: &actor,
                    originating_tool: &ScopedName::new("dailyos.read.account_status"),
                    result_item_path: "/fresh",
                    target_kind: TargetKind::Entity,
                    target_ref: serde_json::json!({ "entity_id": "fresh" }),
                    sensitivity_tier: "internal",
                    provenance_material: "fresh",
                    watermark_material: "fresh",
                },
            )
            .expect("fresh handle");
            let old = mint_target_handle(
                &db,
                MintTargetHandle {
                    actor: &actor,
                    originating_tool: &ScopedName::new("dailyos.read.account_status"),
                    result_item_path: "/old",
                    target_kind: TargetKind::Entity,
                    target_ref: serde_json::json!({ "entity_id": "old" }),
                    sensitivity_tier: "internal",
                    provenance_material: "old",
                    watermark_material: "old",
                },
            )
            .expect("old handle");
            let fresh_hash = public_handle_hash(&fresh).expect("fresh hash");
            let old_hash = public_handle_hash(&old).expect("old hash");
            let old_expiry = rfc3339(Utc::now() - Duration::days(8));
            db.conn_ref()
                .execute(
                    "UPDATE mcp_target_handles SET expires_at = ?1 WHERE handle_lookup_hash = ?2",
                    params![old_expiry, old_hash],
                )
                .expect("age old row");

            assert_eq!(cleanup_target_handles(&db).expect("cleanup"), 1);
            let count: i64 = db
                .conn_ref()
                .query_row(
                    "SELECT COUNT(*) FROM mcp_target_handles WHERE handle_lookup_hash = ?1",
                    params![fresh_hash],
                    |row| row.get(0),
                )
                .expect("count");
            assert_eq!(count, 1);
        });
    }
}
