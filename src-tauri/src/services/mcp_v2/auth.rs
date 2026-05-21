//! MCP v2 authentication + session: pairing, manifest load, transport HMAC,
//! nonce ledger, conversation handle lifecycle.
//!
//! Implements the four trust gates per ADR-0102 §C (cycle-1 + cycle-7
//! amendments) + the per-message nonce contract per cycle-8 + cycle-9
//! amendments. Mirrors the `services::surface_pairing` /
//! `services::surface_nonce` substrate pattern verbatim per L0 packet §1 #2 +
//! cycle-7 architect MED on the pairing response shape.

use std::process::Command;
use std::time::SystemTime;

use rand::Rng;
use ring::hmac;
use rusqlite::{params, Connection, OptionalExtension};
use zeroize::Zeroizing;

use abilities_runtime::abilities::registry::McpExposure;

use super::actor_policy::{ClientRecord, KeychainRef, ToolGrant, ToolRateLimit};
use super::contracts::{
    McpClientId, McpToolRequestEnvelope, OpaqueConversationHandle, OpaqueNonce, Scope, ScopedName,
};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// MCP transport / auth-state errors. Distinct from `ToolError` because these
/// fire BEFORE the gateway reaches the typed-tool layer.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("invalid HMAC signature")]
    InvalidSignature,
    #[error("nonce already consumed")]
    NonceReplayed,
    #[error("client unknown")]
    UnknownClient,
    #[error("client pairing revoked")]
    PairingRevoked,
    #[error("conversation handle unknown or revoked")]
    ConversationRevoked,
    #[error("keychain access failed: {0}")]
    Keychain(String),
    #[error("nonce preissue failed after consume (fail-closed): {0}")]
    PreissueFailed(String),
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("canonical-json encoding failed: {0}")]
    Encoding(#[from] serde_json::Error),
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

const TRANSPORT_KEY_SERVICE: &str = "ai.dailyos.mcp_v2.transport";
const NONCE_BYTES: usize = 16;
const HANDLE_BYTES: usize = 16;
const CLIENT_ID_BYTES: usize = 16;
const NONCE_EXPIRY_SECONDS: i64 = 300; // 5 min per ADR-0102 §C.bis.refresh.
const HANDLE_EXPIRY_SECONDS: i64 = 24 * 60 * 60; // 24h sliding per §D.
const TRANSPORT_KEY_LEN: usize = 32;

/// Operator-supplied pairing input that the MCP server uses to mint a new
/// `McpClientId` + seed nonce + transport key.
///
/// TODO(L2): the pairing handshake transport (stdio vs HTTP) and any
/// out-of-band proof material is W2+ when the first real MCP client lands.
/// This W1-A scope produces the server-side state needed to admit a future
/// invocation; per ADR-0102 §C the handshake itself mirrors
/// `services::surface_pairing::complete_handshake`.
#[derive(Debug, Clone)]
pub struct PairingHandshake {
    /// Caller-supplied opaque label (e.g. "claude-desktop-2026-05-20") —
    /// stored for operator visibility; never serves auth.
    pub client_label: String,
    /// Initial per-tool grant set chosen by the operator at pairing time.
    pub tool_grants: Vec<ToolGrant>,
}

/// Server-issued pairing response returned to the client. Caller MUST persist
/// `transport_key` to its own secret store (e.g., OS keychain) immediately —
/// the server retains only the `KeychainRef` and never the raw bytes.
#[derive(Debug)]
pub struct PairingResponse {
    pub client_id: McpClientId,
    pub seed_nonce: OpaqueNonce,
    /// Raw 32-byte transport key. Caller persists; this is the only time it
    /// leaves the server.
    pub transport_key: Zeroizing<[u8; TRANSPORT_KEY_LEN]>,
    pub transport_key_ref: KeychainRef,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Pair a new MCP client: mint client_id, transport key, manifest rows, and
/// seed nonce. Returns `PairingResponse` containing the raw transport key
/// (only here).
///
/// Per L0 packet §1 #2 + ADR-0102 §C cycle-1 amendment + cycle-7
/// devex+architect+CSO HIGH/MED on the pairing response shape.
pub fn pair_client(
    conn: &mut Connection,
    handshake: PairingHandshake,
) -> Result<PairingResponse, AuthError> {
    let client_id = McpClientId::new(random_hex(CLIENT_ID_BYTES));
    let transport_key_ref =
        KeychainRef(format!("{TRANSPORT_KEY_SERVICE}/{}", client_id.as_str()));
    let transport_key = Zeroizing::new(random_bytes_32());
    persist_transport_key(&transport_key_ref, &transport_key)?;

    let now = now_millis();
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO mcp_client_manifest \
         (client_id, paired_at, revoked_at, transport_key_ref) \
         VALUES (?1, ?2, NULL, ?3)",
        params![client_id.as_str(), now, &transport_key_ref.0],
    )?;
    for grant in &handshake.tool_grants {
        tx.execute(
            "INSERT INTO mcp_tool_grant \
             (client_id, tool_name, scopes_granted_json, exposure, rate_limit_max, rate_limit_window_secs) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                client_id.as_str(),
                grant.tool_name.as_str(),
                serde_json::to_string(&grant.scopes_granted)?,
                exposure_tag(grant.exposure),
                grant.rate_limit.max_calls,
                grant.rate_limit.window_seconds,
            ],
        )?;
    }
    let seed_nonce = OpaqueNonce::new(random_hex(NONCE_BYTES));
    tx.execute(
        "INSERT INTO mcp_transport_nonce_ledger \
         (nonce, client_id, issued_at, expires_at, consumed_at) \
         VALUES (?1, ?2, ?3, ?4, NULL)",
        params![
            seed_nonce.as_str(),
            client_id.as_str(),
            now,
            now + NONCE_EXPIRY_SECONDS * 1000,
        ],
    )?;
    // client_label retained only for operator-visible audit attribution at the
    // pairing audit event sink; not stored on the manifest schema for v1.4.7.
    let _ = handshake.client_label;
    tx.commit()?;

    Ok(PairingResponse {
        client_id,
        seed_nonce,
        transport_key,
        transport_key_ref,
    })
}

/// Load the manifest record for a client — indexed PK lookup; no cache (per
/// ADR-0102 §C cycle-1: revocation propagates within one in-flight call).
pub fn load_client_record(
    conn: &Connection,
    client_id: &McpClientId,
) -> Result<ClientRecord, AuthError> {
    let mut stmt = conn.prepare_cached(
        "SELECT paired_at, revoked_at, transport_key_ref \
         FROM mcp_client_manifest WHERE client_id = ?1",
    )?;
    let row = stmt
        .query_row(params![client_id.as_str()], |row| {
            let paired_at: i64 = row.get(0)?;
            let revoked_at: Option<i64> = row.get(1)?;
            let transport_key_ref: String = row.get(2)?;
            Ok((paired_at, revoked_at, transport_key_ref))
        })
        .optional()?;
    let (paired_at, revoked_at, transport_key_ref) = row.ok_or(AuthError::UnknownClient)?;
    Ok(ClientRecord {
        client_id: client_id.clone(),
        paired_at: millis_to_system_time(paired_at),
        revoked_at: revoked_at.map(millis_to_system_time),
        transport_key_ref: KeychainRef(transport_key_ref),
    })
}

/// Indexed `(client_id, tool_name)` lookup per L0 packet AC-2 step (ii).
pub fn resolve_tool_grant(
    conn: &Connection,
    client_id: &McpClientId,
    tool_name: &ScopedName,
) -> Result<Option<ToolGrant>, AuthError> {
    let mut stmt = conn.prepare_cached(
        "SELECT scopes_granted_json, exposure, rate_limit_max, rate_limit_window_secs \
         FROM mcp_tool_grant WHERE client_id = ?1 AND tool_name = ?2",
    )?;
    let row = stmt
        .query_row(params![client_id.as_str(), tool_name.as_str()], |row| {
            let scopes_json: String = row.get(0)?;
            let exposure: String = row.get(1)?;
            let max_calls: u32 = row.get(2)?;
            let window_seconds: u32 = row.get(3)?;
            Ok((scopes_json, exposure, max_calls, window_seconds))
        })
        .optional()?;
    let Some((scopes_json, exposure, max_calls, window_seconds)) = row else {
        return Ok(None);
    };
    let scopes_granted: Vec<Scope> = serde_json::from_str(&scopes_json)?;
    Ok(Some(ToolGrant {
        tool_name: tool_name.clone(),
        scopes_granted,
        exposure: parse_exposure(&exposure),
        rate_limit: ToolRateLimit {
            max_calls,
            window_seconds,
        },
    }))
}

/// Verify HMAC-SHA256 signature over canonical-JSON bytes of the WHOLE
/// request envelope (including `request_nonce`) per cycle-7 challenge HIGH.
/// Loads key into a `Zeroizing` wrapper for one verify then drops per AC-11.
///
/// Returns the asserted `McpClientId` on verify success. Caller passes the
/// candidate `client_id` (out-of-band: transport-layer header). The HMAC
/// inclusion of the WHOLE envelope means an attacker cannot swap nonces on
/// captured signed payloads (closes cycle-7 challenge HIGH).
pub fn verify_transport_hmac(
    conn: &Connection,
    asserted_client_id: &McpClientId,
    envelope: &McpToolRequestEnvelope,
    signature: &[u8],
) -> Result<McpClientId, AuthError> {
    let record = load_client_record(conn, asserted_client_id)?;
    let key_bytes = Zeroizing::new(load_transport_key(&record.transport_key_ref)?);
    let mac_key = hmac::Key::new(hmac::HMAC_SHA256, key_bytes.as_ref());
    let canonical = serde_json::to_vec(envelope)?;
    hmac::verify(&mac_key, &canonical, signature).map_err(|_| AuthError::InvalidSignature)?;
    Ok(asserted_client_id.clone())
}

/// Fail-closed consume + preissue per ADR-0102 §C.bis.fail-closed.
///
/// Implemented as TWO separate phases per L2 cycle-1 code-reviewer HIGH:
/// (a) commit the consume in its own transaction — once committed the nonce
///     is permanently consumed regardless of downstream failure;
/// (b) insert the next nonce in a second statement (autocommit). If (b)
///     fails the consume from (a) is NOT rolled back — caller receives
///     `AuthError::PreissueFailed`, must re-pair.
///
/// Cycle-7 CSO HIGH + architect MED on issue-next failure path is closed
/// this way: consume is fail-closed, preissue failure is surfaced explicitly.
pub fn verify_and_consume_and_preissue(
    conn: &mut Connection,
    client_id: &McpClientId,
    presented: &OpaqueNonce,
) -> Result<OpaqueNonce, AuthError> {
    let now = now_millis();

    // Phase (a): consume-only transaction. Either consumes successfully (and
    // commits — fail-closed point) or returns the typed AuthError.
    {
        let tx = conn.transaction()?;
        let consumed = tx.execute(
            "UPDATE mcp_transport_nonce_ledger SET consumed_at = ?3 \
             WHERE client_id = ?1 AND nonce = ?2 \
               AND consumed_at IS NULL AND expires_at >= ?3",
            params![client_id.as_str(), presented.as_str(), now],
        )?;
        if consumed != 1 {
            // Disambiguate replayed vs absent/expired for caller-error shape
            // per §C.bis.schema. Same wire shape; only `detail` differs.
            let already_consumed: Option<i64> = tx
                .query_row(
                    "SELECT consumed_at FROM mcp_transport_nonce_ledger \
                     WHERE client_id = ?1 AND nonce = ?2",
                    params![client_id.as_str(), presented.as_str()],
                    |row| row.get(0),
                )
                .optional()?
                .flatten();
            // tx auto-rolls back on drop — fine here, nothing has changed.
            return Err(if already_consumed.is_some() {
                AuthError::NonceReplayed
            } else {
                AuthError::InvalidSignature
            });
        }
        tx.commit()?;
        // Fail-closed point: consume is now permanent in the ledger.
    }

    // Phase (b): preissue next nonce. If this fails, consume from (a) stays
    // committed; caller learns the failure via `AuthError::PreissueFailed`
    // and must re-pair (or use the recovery path the gateway provides if
    // any). Per ADR-0102 §C.bis.fail-closed.
    let next_nonce = OpaqueNonce::new(random_hex(NONCE_BYTES));
    conn.execute(
        "INSERT INTO mcp_transport_nonce_ledger \
         (nonce, client_id, issued_at, expires_at, consumed_at) \
         VALUES (?1, ?2, ?3, ?4, NULL)",
        params![
            next_nonce.as_str(),
            client_id.as_str(),
            now,
            now + NONCE_EXPIRY_SECONDS * 1000,
        ],
    )
    .map_err(|err| AuthError::PreissueFailed(err.to_string()))?;
    Ok(next_nonce)
}

/// Resolve presented handle to a server-side record (or mint a new one if
/// absent). Composite `(handle, client_id)` per ADR-0102 §D.bis cross-client
/// binding: a handle issued to client A presented by client B →
/// `ConversationRevoked`.
pub fn resolve_or_mint_handle(
    conn: &mut Connection,
    client_id: &McpClientId,
    presented: Option<&OpaqueConversationHandle>,
) -> Result<OpaqueConversationHandle, AuthError> {
    let now = now_millis();
    let Some(handle) = presented else {
        return mint_handle(conn, client_id, now);
    };
    let mut stmt = conn.prepare_cached(
        "SELECT revoked_at, last_touched_at FROM mcp_conversation_handle \
         WHERE handle = ?1 AND client_id = ?2",
    )?;
    let row: Option<(Option<i64>, i64)> = stmt
        .query_row(params![handle.as_str(), client_id.as_str()], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .optional()?;
    drop(stmt);
    let Some((revoked_at, last_touched_at)) = row else {
        return Err(AuthError::ConversationRevoked);
    };
    if revoked_at.is_some() {
        return Err(AuthError::ConversationRevoked);
    }
    if now - last_touched_at > HANDLE_EXPIRY_SECONDS * 1000 {
        // Expired (sliding 24h): silent mint replacement per ADR-0102 §D.
        return mint_handle(conn, client_id, now);
    }
    conn.execute(
        "UPDATE mcp_conversation_handle SET last_touched_at = ?3 \
         WHERE handle = ?1 AND client_id = ?2",
        params![handle.as_str(), client_id.as_str(), now],
    )?;
    Ok(handle.clone())
}

fn mint_handle(
    conn: &mut Connection,
    client_id: &McpClientId,
    now: i64,
) -> Result<OpaqueConversationHandle, AuthError> {
    let handle = OpaqueConversationHandle::new(random_hex(HANDLE_BYTES));
    conn.execute(
        "INSERT INTO mcp_conversation_handle \
         (handle, client_id, mint_at, last_touched_at, revoked_at) \
         VALUES (?1, ?2, ?3, ?3, NULL)",
        params![handle.as_str(), client_id.as_str(), now],
    )?;
    Ok(handle)
}

/// Admin: revoke a conversation handle. Subsequent presentations →
/// `ConversationRevoked`.
pub fn revoke_handle(
    conn: &Connection,
    handle: &OpaqueConversationHandle,
) -> Result<(), AuthError> {
    conn.execute(
        "UPDATE mcp_conversation_handle SET revoked_at = ?2 WHERE handle = ?1",
        params![handle.as_str(), now_millis()],
    )?;
    Ok(())
}

/// Admin: revoke a paired client. Subsequent invocations → `PairingRevoked`.
pub fn revoke_client(conn: &Connection, client_id: &McpClientId) -> Result<(), AuthError> {
    conn.execute(
        "UPDATE mcp_client_manifest SET revoked_at = ?2 WHERE client_id = ?1",
        params![client_id.as_str(), now_millis()],
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Internals: keychain custody, RNG, time, encoding helpers
// ---------------------------------------------------------------------------

fn persist_transport_key(
    key_ref: &KeychainRef,
    key: &[u8; TRANSPORT_KEY_LEN],
) -> Result<(), AuthError> {
    // L2 cycle-3 codex review HIGH AC-11: transport key intermediates must
    // not linger in non-Zeroizing buffers. Wrap the hex String in
    // `Zeroizing` so its backing allocation is wiped when this function
    // returns, regardless of success/error path.
    let key_hex: Zeroizing<String> = Zeroizing::new(hex::encode(key));
    let output = Command::new("security")
        .args([
            "add-generic-password",
            "-s",
            TRANSPORT_KEY_SERVICE,
            "-a",
            &key_ref.0,
            "-w",
            key_hex.as_str(),
            "-U",
        ])
        .output()
        .map_err(|error| AuthError::Keychain(format!("security CLI invocation failed: {error}")))?;
    if !output.status.success() {
        return Err(AuthError::Keychain(format!(
            "keychain write failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(())
}

fn load_transport_key(key_ref: &KeychainRef) -> Result<[u8; TRANSPORT_KEY_LEN], AuthError> {
    // L2 cycle-3 codex review HIGH AC-11: wrap every intermediate that
    // briefly holds key material in `Zeroizing` so backing allocations are
    // wiped on drop. The returned `[u8; 32]` is caller-managed and is
    // expected to be wrapped in `Zeroizing` at the call site (gateway).
    let output = Command::new("security")
        .args([
            "find-generic-password",
            "-s",
            TRANSPORT_KEY_SERVICE,
            "-a",
            &key_ref.0,
            "-w",
        ])
        .output()
        .map_err(|error| AuthError::Keychain(format!("security CLI invocation failed: {error}")))?;
    if !output.status.success() {
        return Err(AuthError::Keychain(format!(
            "keychain read failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let stdout: Zeroizing<Vec<u8>> = Zeroizing::new(output.stdout);
    // L2 cycle-5 codex review NEW: use borrowing str::from_utf8 to avoid
    // a clone-into-FromUtf8Error path that would drop unzeroized on UTF-8
    // failure. The borrowed &str is then explicitly Zeroizing-owned.
    let hex_key: Zeroizing<String> = Zeroizing::new(
        std::str::from_utf8(&stdout)
            .map_err(|error| AuthError::Keychain(format!("non-UTF-8 key: {error}")))?
            .to_owned(),
    );
    let bytes: Zeroizing<Vec<u8>> = Zeroizing::new(
        hex::decode(hex_key.trim())
            .map_err(|error| AuthError::Keychain(format!("invalid hex key: {error}")))?,
    );
    if bytes.len() != TRANSPORT_KEY_LEN {
        return Err(AuthError::Keychain(format!(
            "transport key length mismatch: expected {TRANSPORT_KEY_LEN}, got {}",
            bytes.len()
        )));
    }
    let mut key = [0u8; TRANSPORT_KEY_LEN];
    key.copy_from_slice(&bytes);
    Ok(key)
}

fn random_bytes_32() -> [u8; TRANSPORT_KEY_LEN] {
    let mut bytes = [0u8; TRANSPORT_KEY_LEN];
    rand::rng().fill_bytes(&mut bytes);
    bytes
}

fn random_hex(byte_len: usize) -> String {
    let mut bytes = vec![0u8; byte_len];
    rand::rng().fill_bytes(&mut bytes);
    hex::encode(&bytes)
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn millis_to_system_time(ms: i64) -> SystemTime {
    SystemTime::UNIX_EPOCH + std::time::Duration::from_millis(ms.max(0) as u64)
}

fn exposure_tag(exposure: McpExposure) -> &'static str {
    match exposure {
        McpExposure::None => "None",
        McpExposure::MetadataOnly => "MetadataOnly",
        McpExposure::Invocable => "Invocable",
    }
}

fn parse_exposure(tag: &str) -> McpExposure {
    match tag {
        "Invocable" => McpExposure::Invocable,
        "MetadataOnly" => McpExposure::MetadataOnly,
        _ => McpExposure::None,
    }
}

/// Return every tool name from `mcp_tool_grant` for `client_id` where
/// exposure = `Invocable`. Consumed by the W1.5 transport `tools/list`
/// per the DOS-MCP-Transport L0 AC-6: only Invocable-tier grants are
/// surfaced to the host model.
pub fn list_invocable_tool_grants(
    conn: &Connection,
    client_id: &McpClientId,
) -> Result<Vec<ScopedName>, AuthError> {
    let mut stmt = conn.prepare_cached(
        "SELECT tool_name FROM mcp_tool_grant \
         WHERE client_id = ?1 AND exposure = 'Invocable' \
         ORDER BY tool_name ASC",
    )?;
    let rows = stmt.query_map(params![client_id.as_str()], |row| {
        let name: String = row.get(0)?;
        Ok(ScopedName::new(name))
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}
