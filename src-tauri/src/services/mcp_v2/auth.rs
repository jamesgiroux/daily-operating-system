//! MCP v2 authentication + session: pairing, manifest load, and
//! conversation handle lifecycle.
//!
//! Local MCP v2 treats the OS user as the trust boundary. This module keeps the
//! authorization machinery: per-client manifests, grant resolution, revocation,
//! and server-minted conversation handles.

use std::time::SystemTime;

use rand::Rng;
use rusqlite::{params, Connection, OptionalExtension};

use abilities_runtime::abilities::registry::McpExposure;

use super::actor_policy::{ClientRecord, KeychainRef, ToolGrant, ToolRateLimit};
use super::contracts::{McpClientId, OpaqueConversationHandle, Scope, ScopedName};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// MCP auth-state errors. Distinct from `ToolError` because these fire before
/// the gateway reaches the typed-tool layer.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("client unknown")]
    UnknownClient,
    #[error("client pairing revoked")]
    PairingRevoked,
    #[error("conversation handle unknown or revoked")]
    ConversationRevoked,
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("manifest JSON encoding failed: {0}")]
    Encoding(#[from] serde_json::Error),
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

const HANDLE_BYTES: usize = 16;
const CLIENT_ID_BYTES: usize = 16;
const HANDLE_EXPIRY_SECONDS: i64 = 24 * 60 * 60; // 24h sliding per §D.

/// Operator-supplied pairing input that the MCP server uses to mint a new
/// `McpClientId` and manifest rows.
#[derive(Debug, Clone)]
pub struct PairingHandshake {
    /// Caller-supplied opaque label (e.g. "claude-desktop-2026-05-20") —
    /// retained only for the pairing audit event sink in later wiring.
    pub client_label: String,
    /// Initial per-tool grant set chosen by the operator at pairing time.
    pub tool_grants: Vec<ToolGrant>,
}

/// Server-issued pairing response returned to the client.
#[derive(Debug)]
pub struct PairingResponse {
    pub client_id: McpClientId,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Pair a new MCP client: mint client_id and manifest rows.
pub fn pair_client(
    conn: &mut Connection,
    handshake: PairingHandshake,
) -> Result<PairingResponse, AuthError> {
    let client_id = McpClientId::new(random_hex(CLIENT_ID_BYTES));
    let now = now_millis();
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO mcp_client_manifest \
         (client_id, paired_at, revoked_at, transport_key_ref) \
         VALUES (?1, ?2, NULL, NULL)",
        params![client_id.as_str(), now],
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
    // client_label retained only for operator-visible audit attribution at the
    // pairing audit event sink; not stored on the manifest schema for v1.4.7.
    let _ = handshake.client_label;
    tx.commit()?;

    Ok(PairingResponse { client_id })
}

/// Load the manifest record for a client — indexed PK lookup; no cache so
/// revocation propagates within one in-flight call.
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
            let transport_key_ref: Option<String> = row.get(2)?;
            Ok((paired_at, revoked_at, transport_key_ref))
        })
        .optional()?;
    let (paired_at, revoked_at, transport_key_ref) = row.ok_or(AuthError::UnknownClient)?;
    Ok(ClientRecord {
        client_id: client_id.clone(),
        paired_at: millis_to_system_time(paired_at),
        revoked_at: revoked_at.map(millis_to_system_time),
        transport_key_ref: KeychainRef(transport_key_ref.unwrap_or_default()),
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

/// Resolve presented handle to a server-side record (or mint a new one if
/// absent). Composite `(handle, client_id)` binding rejects cross-client reuse.
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

/// Return every tool name from `mcp_tool_grant` for `client_id` where
/// exposure = `Invocable`. Consumed by the transport `tools/list` so only
/// Invocable-tier grants are surfaced to the host model.
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

/// Return the union of scopes granted to invocable manifest rows for
/// `client_id`. Transport-level MCP resources are not tools, but they still
/// authorize against the same server-side grant substrate.
pub fn list_invocable_granted_scopes(
    conn: &Connection,
    client_id: &McpClientId,
) -> Result<Vec<Scope>, AuthError> {
    let mut stmt = conn.prepare_cached(
        "SELECT scopes_granted_json FROM mcp_tool_grant \
         WHERE client_id = ?1 AND exposure = 'Invocable' \
         ORDER BY tool_name ASC",
    )?;
    let rows = stmt.query_map(params![client_id.as_str()], |row| row.get::<_, String>(0))?;
    let mut out = Vec::new();
    for row in rows {
        let scopes_json = row?;
        let scopes: Vec<Scope> = serde_json::from_str(&scopes_json)?;
        out.extend(scopes);
    }
    out.sort();
    out.dedup();
    Ok(out)
}

/// Ensure the built-in local stdio client exists and has grants for the
/// handlers this binary registered at boot.
///
/// This is intentionally local-only: Claude Desktop and DailyOS run as the
/// same macOS user, so the OS user boundary is the pairing boundary. The grant
/// table still stays authoritative for tool exposure, but startup no longer
/// requires a separate remote-style pairing ceremony.
pub fn ensure_local_stdio_client_grants(
    conn: &mut Connection,
    client_id: &McpClientId,
    grants: &[ToolGrant],
) -> Result<(), AuthError> {
    let now = now_millis();
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO mcp_client_manifest \
         (client_id, paired_at, revoked_at, transport_key_ref) \
         VALUES (?1, ?2, NULL, NULL) \
         ON CONFLICT(client_id) DO UPDATE SET revoked_at = NULL",
        params![client_id.as_str(), now],
    )?;

    for grant in grants {
        tx.execute(
            "INSERT INTO mcp_tool_grant \
             (client_id, tool_name, scopes_granted_json, exposure, rate_limit_max, rate_limit_window_secs) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
             ON CONFLICT(client_id, tool_name) DO UPDATE SET \
                scopes_granted_json = excluded.scopes_granted_json, \
                exposure = excluded.exposure, \
                rate_limit_max = excluded.rate_limit_max, \
                rate_limit_window_secs = excluded.rate_limit_window_secs",
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

    if grants.is_empty() {
        tx.execute(
            "DELETE FROM mcp_tool_grant WHERE client_id = ?1",
            params![client_id.as_str()],
        )?;
    } else {
        let placeholders = std::iter::repeat_n("?", grants.len())
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "DELETE FROM mcp_tool_grant \
             WHERE client_id = ? AND tool_name NOT IN ({placeholders})"
        );
        let mut values = Vec::with_capacity(grants.len() + 1);
        values.push(client_id.as_str().to_string());
        values.extend(
            grants
                .iter()
                .map(|grant| grant.tool_name.as_str().to_string()),
        );
        tx.execute(&sql, rusqlite::params_from_iter(values.iter()))?;
    }

    tx.commit()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Internals: RNG, time, encoding helpers
// ---------------------------------------------------------------------------

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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_mcp_auth_tables(conn: &Connection) {
        conn.execute_batch(
            "
            CREATE TABLE mcp_client_manifest (
                client_id TEXT PRIMARY KEY,
                paired_at INTEGER NOT NULL,
                revoked_at INTEGER NULL,
                transport_key_ref TEXT NULL
            );
            CREATE TABLE mcp_tool_grant (
                client_id TEXT,
                tool_name TEXT,
                scopes_granted_json TEXT NOT NULL,
                exposure TEXT NOT NULL CHECK (exposure IN ('None','MetadataOnly','Invocable')),
                rate_limit_max INTEGER NOT NULL,
                rate_limit_window_secs INTEGER NOT NULL,
                PRIMARY KEY (client_id, tool_name)
            );
            ",
        )
        .expect("create MCP auth tables");
    }

    fn invocable_grant(name: &str) -> ToolGrant {
        ToolGrant {
            tool_name: ScopedName::new(name),
            scopes_granted: vec![Scope::new(name)],
            exposure: McpExposure::Invocable,
            rate_limit: ToolRateLimit {
                max_calls: 600,
                window_seconds: 60,
            },
        }
    }

    #[test]
    fn local_stdio_grants_prune_stale_tools_for_client() {
        let mut conn = Connection::open_in_memory().expect("open sqlite");
        create_mcp_auth_tables(&conn);
        let client_id = McpClientId::new("local-client");

        ensure_local_stdio_client_grants(
            &mut conn,
            &client_id,
            &[
                invocable_grant("dailyos.read.account_status"),
                invocable_grant("dailyos.read.daily_briefing"),
            ],
        )
        .expect("seed initial grants");
        ensure_local_stdio_client_grants(
            &mut conn,
            &client_id,
            &[invocable_grant("dailyos.read.account_status")],
        )
        .expect("refresh grants");

        let grants = list_invocable_tool_grants(&conn, &client_id).expect("list grants");
        assert_eq!(grants, vec![ScopedName::new("dailyos.read.account_status")]);
    }
}
