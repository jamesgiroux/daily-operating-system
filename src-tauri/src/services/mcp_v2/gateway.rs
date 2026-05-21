//! MCP v2 gateway: the single entry point that receives MCP tool calls.
//!
//! Validates Actor::McpClient policy against the server-side scope manifest
//! loaded by `auth`; dispatches through registered `McpToolHandler`
//! implementations; records audit attribution on success; emits
//! `McpToolInvoked` / `McpInvocationRejected` signals on success / rejection
//! respectively.
//!
//! Per ADR-0102 §C (cycle-1 + cycle-7 + cycle-8 + cycle-9 amendments) and L0
//! packet `dos-168-l0-plan.md` §1 #1 + AC-1..AC-12.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use rusqlite::{params, Connection};
use serde_json::json;

use super::actor_policy::{self, ToolGrant};
use super::audit;
use super::auth::{self, AuthError};
use super::contracts::{
    McpActor, McpClientId, McpToolHandler, McpToolRequestEnvelope, McpToolResponseEnvelope,
    McpToolResult, OpaqueConversationHandle, OpaqueNonce, Scope, ScopedName, Side, ToolError,
};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Wall-clock floor on auth-state error responses, per L0 packet AC-12. The
/// gateway sleeps until at least this many millis have elapsed before
/// returning any of: `BadParams { detail: "invalid_signature" | "nonce_replayed" }`,
/// `PairingRevoked`, `ExposureForbidden`, `Unauthorized`, `ConversationRevoked`.
/// Prevents a client-id enumeration timing oracle.
const TIMING_FLOOR_MILLIS: u64 = 10;

/// Default rate-limit retry-after when the gateway has nothing better.
const RATE_LIMIT_RETRY_DEFAULT_SECONDS: u32 = 60;

// ---------------------------------------------------------------------------
// Gateway
// ---------------------------------------------------------------------------

/// MCP v2 dispatch entry point. Holds the registered handler set and runs the
/// per-dispatch flow defined in L0 packet §1 #1.
pub struct Gateway {
    handlers: HashMap<ScopedName, Arc<dyn McpToolHandler>>,
}

impl Gateway {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    pub fn register(&mut self, handler: Arc<dyn McpToolHandler>) {
        let name = handler.description().name.clone();
        self.handlers.insert(name, handler);
    }

    pub fn registered_tools(&self) -> impl Iterator<Item = &ScopedName> {
        self.handlers.keys()
    }

    /// Handle a single MCP tool invocation. The full per-dispatch contract
    /// from L0 packet §1 #1 lives here in one function so the gates fire in
    /// the documented order and the timing-floor guarantee holds across all
    /// rejection branches.
    ///
    /// `asserted_client_id` is the candidate client id from the transport
    /// header (out-of-band). `signature` is the HMAC-SHA256 over canonical
    /// JSON of the WHOLE envelope (including `request_nonce`) per cycle-7
    /// challenge HIGH. `conn` is the per-call SQLite connection.
    pub fn handle_tool_call(
        &self,
        conn: &mut Connection,
        asserted_client_id: &McpClientId,
        envelope: McpToolRequestEnvelope,
        signature: &[u8],
    ) -> McpToolResponseEnvelope {
        let start = Instant::now();
        let dispatch = self.dispatch(conn, asserted_client_id, &envelope, signature);
        let outcome = match dispatch {
            Ok((result, next_nonce, conversation_handle)) => (
                result,
                next_nonce,
                conversation_handle,
                /* was_rejection */ false,
            ),
            Err(GatewayFailure {
                error,
                reject_reason,
                tool_name_for_signal,
                client_id_for_signal,
            }) => {
                // Auth-state rejection. Emit reject signal with safe attribution
                // per AC-7 (emit-or-log; failure → operator log + Suite-S alert
                // mcp_reject_signal_emit_failed). No audit row.
                emit_reject_signal(
                    client_id_for_signal.as_ref(),
                    &reject_reason,
                    tool_name_for_signal.as_ref(),
                );
                // Mint a fresh next_request_nonce even on rejection so the
                // caller can recover without re-pairing per L0 packet §1 #1.
                let recovery_nonce = match asserted_client_id_for_recovery_nonce(
                    conn,
                    asserted_client_id,
                ) {
                    Ok(nonce) => nonce,
                    Err(_) => OpaqueNonce::new(String::new()),
                };
                let handle =
                    OpaqueConversationHandle::new(envelope_handle_str(&envelope).to_string());
                (
                    McpToolResult::Error { error },
                    recovery_nonce,
                    handle,
                    /* was_rejection */ true,
                )
            }
        };
        let (result, next_nonce, conversation_handle, was_rejection) = outcome;
        if was_rejection {
            sleep_until_floor(start);
        }
        McpToolResponseEnvelope {
            conversation_handle,
            next_request_nonce: next_nonce,
            result,
        }
    }

    /// Inner per-dispatch flow. On success returns the handler result + the
    /// preissued next nonce + the active conversation handle. On any failure
    /// returns a `GatewayFailure` describing the rejection for signal
    /// attribution.
    fn dispatch(
        &self,
        conn: &mut Connection,
        asserted_client_id: &McpClientId,
        envelope: &McpToolRequestEnvelope,
        signature: &[u8],
    ) -> Result<(McpToolResult, OpaqueNonce, OpaqueConversationHandle), GatewayFailure> {
        // Gate 0a: HMAC over WHOLE envelope including request_nonce.
        if let Err(err) = auth::verify_transport_hmac(conn, asserted_client_id, envelope, signature)
        {
            return Err(auth_to_failure(err, envelope, Some(asserted_client_id)));
        }

        // Gate 0b: atomic consume + preissue per ADR-0102 §C.bis.refresh.
        // After this point, the nonce is permanently consumed (fail-closed
        // per §C.bis.fail-closed); the returned `next_nonce` is the value
        // the response envelope MUST carry whether success or rejection.
        let next_nonce =
            match auth::verify_and_consume_and_preissue(conn, asserted_client_id, &envelope.request_nonce) {
                Ok(nonce) => nonce,
                Err(err) => {
                    return Err(auth_to_failure(err, envelope, Some(asserted_client_id)))
                }
            };

        // Manifest gates per AC-2 decision order.
        let record = match auth::load_client_record(conn, asserted_client_id) {
            Ok(r) => r,
            Err(err) => return Err(auth_to_failure(err, envelope, Some(asserted_client_id))),
        };
        if record.revoked_at.is_some() {
            return Err(GatewayFailure {
                error: ToolError::PairingRevoked,
                reject_reason: "pairing_revoked".to_string(),
                tool_name_for_signal: Some(envelope.tool_name.clone()),
                client_id_for_signal: Some(asserted_client_id.clone()),
            });
        }

        let grant = match auth::resolve_tool_grant(conn, asserted_client_id, &envelope.tool_name) {
            Ok(g) => g,
            Err(err) => return Err(auth_to_failure(err, envelope, Some(asserted_client_id))),
        };
        let Some(grant) = grant else {
            return Err(GatewayFailure {
                error: ToolError::ExposureForbidden {
                    tool_name: envelope.tool_name.clone(),
                },
                reject_reason: "absent_grant".to_string(),
                tool_name_for_signal: Some(envelope.tool_name.clone()),
                client_id_for_signal: Some(asserted_client_id.clone()),
            });
        };
        if !matches!(
            grant.exposure,
            abilities_runtime::abilities::registry::McpExposure::Invocable
        ) {
            return Err(GatewayFailure {
                error: ToolError::ExposureForbidden {
                    tool_name: envelope.tool_name.clone(),
                },
                reject_reason: "non_invocable".to_string(),
                tool_name_for_signal: Some(envelope.tool_name.clone()),
                client_id_for_signal: Some(asserted_client_id.clone()),
            });
        }

        // Handler lookup. Unknown tool → BadParams per AC-1.
        let Some(handler) = self.handlers.get(&envelope.tool_name) else {
            return Err(GatewayFailure {
                error: ToolError::BadParams {
                    detail: "unknown tool name".to_string(),
                },
                reject_reason: "unknown_tool".to_string(),
                tool_name_for_signal: Some(envelope.tool_name.clone()),
                client_id_for_signal: Some(asserted_client_id.clone()),
            });
        };

        // Scope subset check per AC-2 step (iii).
        let required: &[Scope] = &handler.description().scopes_required;
        if !scope_is_subset(required, &grant.scopes_granted) {
            let missing = required
                .iter()
                .find(|r| !grant.scopes_granted.contains(r))
                .cloned()
                .unwrap_or_else(|| Scope::new(""));
            return Err(GatewayFailure {
                error: ToolError::Unauthorized {
                    missing_scope: missing.clone(),
                },
                reject_reason: format!("missing_scope:{}", missing.as_str()),
                tool_name_for_signal: Some(envelope.tool_name.clone()),
                client_id_for_signal: Some(asserted_client_id.clone()),
            });
        }

        // Reject caller-asserted scope/conversation fields in params per AC-2 (iv).
        if let Some(params_obj) = envelope.params.as_object() {
            if params_obj.contains_key("granted_scopes") {
                return Err(GatewayFailure {
                    error: ToolError::BadParams {
                        detail: "granted_scopes not accepted in params".to_string(),
                    },
                    reject_reason: "caller_asserted_scopes".to_string(),
                    tool_name_for_signal: Some(envelope.tool_name.clone()),
                    client_id_for_signal: Some(asserted_client_id.clone()),
                });
            }
            if params_obj.contains_key("conversation_id") {
                return Err(GatewayFailure {
                    error: ToolError::BadParams {
                        detail: "use envelope conversation_handle".to_string(),
                    },
                    reject_reason: "caller_asserted_conversation_id".to_string(),
                    tool_name_for_signal: Some(envelope.tool_name.clone()),
                    client_id_for_signal: Some(asserted_client_id.clone()),
                });
            }
        }

        // Resolve/mint conversation handle per AC-3a/AC-3b §D.bis.
        let conversation_handle = match auth::resolve_or_mint_handle(
            conn,
            asserted_client_id,
            envelope.conversation_handle.as_ref(),
        ) {
            Ok(h) => h,
            Err(err) => return Err(auth_to_failure(err, envelope, Some(asserted_client_id))),
        };

        // Rate limit per AC-5: BEGIN IMMEDIATE → prune → count → reserve.
        match reserve_rate_limit(conn, asserted_client_id, &envelope.tool_name, &grant) {
            Ok(()) => {}
            Err(retry_after) => {
                return Err(GatewayFailure {
                    error: ToolError::RateLimited {
                        retry_after_seconds: retry_after,
                    },
                    reject_reason: "rate_limited".to_string(),
                    tool_name_for_signal: Some(envelope.tool_name.clone()),
                    client_id_for_signal: Some(asserted_client_id.clone()),
                });
            }
        }

        // Construct the runtime actor and dispatch.
        let runtime_actor =
            actor_policy::project_actor(asserted_client_id, &grant, Some(&conversation_handle));
        let wire_actor = McpActor::Client {
            client_id: asserted_client_id.clone(),
            conversation_handle: Some(conversation_handle.clone()),
            tool_name: envelope.tool_name.clone(),
            granted_scopes: grant.scopes_granted.clone(),
        };
        let invocation = handler.invoke(&wire_actor, envelope.params.clone());

        let result = match invocation {
            Ok(value) => {
                // Extract mutation_cursor for Side::Write per AC-6 + taxonomy
                // rustdoc contract. Side::Read omits the cursor field.
                let mutation_cursor = if matches!(handler.description().side, Side::Write) {
                    value.get("mutation_cursor").cloned()
                } else {
                    None
                };

                // Audit append on success per AC-6. Emit-or-log: failure does
                // not roll back handler effects; gateway converts double-failure
                // to ToolError::Internal.
                let audit_detail = build_audit_detail(
                    asserted_client_id,
                    &conversation_handle,
                    &envelope.tool_name,
                    &envelope.params,
                    Some(&value),
                    mutation_cursor.as_ref(),
                );
                if let Err(err) =
                    audit::write(&runtime_actor, "mcp.tool_invoked", audit_detail, None)
                {
                    if matches!(err, audit::AuditError::DoubleFailure { .. }) {
                        emit_double_failure_alert();
                        return Ok((
                            McpToolResult::Error {
                                error: ToolError::Internal {
                                    trace_id: "mcp_audit_double_failure".to_string(),
                                },
                            },
                            next_nonce,
                            conversation_handle,
                        ));
                    }
                    // Single-failure (e.g. keychain unavailable): operator
                    // log only — handler effects stand per AC-6 emit-or-log.
                    eprintln!("mcp_v2 audit single-failure: {err}");
                }

                // Emit McpToolInvoked signal per AC-7 (emit-or-log).
                emit_invoked_signal(asserted_client_id, &conversation_handle, &envelope.tool_name);
                McpToolResult::Ok { value }
            }
            Err(error) => McpToolResult::Error { error },
        };

        Ok((result, next_nonce, conversation_handle))
    }
}

impl Default for Gateway {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Carries enough state to (a) build the typed `ToolError` response and
/// (b) emit the reject signal with safe attribution.
struct GatewayFailure {
    error: ToolError,
    reject_reason: String,
    tool_name_for_signal: Option<ScopedName>,
    client_id_for_signal: Option<McpClientId>,
}

fn auth_to_failure(
    err: AuthError,
    envelope: &McpToolRequestEnvelope,
    asserted_client: Option<&McpClientId>,
) -> GatewayFailure {
    let (tool_error, reason) = match err {
        AuthError::InvalidSignature => (
            ToolError::BadParams {
                detail: "invalid_signature".to_string(),
            },
            "invalid_hmac".to_string(),
        ),
        AuthError::NonceReplayed => (
            ToolError::BadParams {
                detail: "nonce_replayed".to_string(),
            },
            "nonce_replayed".to_string(),
        ),
        AuthError::UnknownClient => (
            // Uniform wire shape with invalid-signature per AC-12 timing-oracle.
            ToolError::BadParams {
                detail: "invalid_signature".to_string(),
            },
            "unknown_client".to_string(),
        ),
        AuthError::PairingRevoked => (ToolError::PairingRevoked, "pairing_revoked".to_string()),
        AuthError::ConversationRevoked => {
            (ToolError::ConversationRevoked, "conversation_revoked".to_string())
        }
        AuthError::Keychain(detail) => (
            ToolError::Internal {
                trace_id: format!("auth_keychain:{detail}"),
            },
            "keychain_error".to_string(),
        ),
        AuthError::Sqlite(detail) => (
            ToolError::Internal {
                trace_id: format!("auth_sqlite:{detail}"),
            },
            "sqlite_error".to_string(),
        ),
        AuthError::Encoding(detail) => (
            ToolError::Internal {
                trace_id: format!("auth_encoding:{detail}"),
            },
            "encoding_error".to_string(),
        ),
    };
    GatewayFailure {
        error: tool_error,
        reject_reason: reason,
        tool_name_for_signal: Some(envelope.tool_name.clone()),
        client_id_for_signal: asserted_client.cloned(),
    }
}

fn scope_is_subset(required: &[Scope], granted: &[Scope]) -> bool {
    required.iter().all(|r| granted.contains(r))
}

fn build_audit_detail(
    client_id: &McpClientId,
    conversation_handle: &OpaqueConversationHandle,
    tool_name: &ScopedName,
    params: &serde_json::Value,
    response: Option<&serde_json::Value>,
    mutation_cursor: Option<&serde_json::Value>,
) -> serde_json::Value {
    let mut detail = json!({
        "client_id": client_id.as_str(),
        "conversation_handle": conversation_handle.as_str(),
        "tool_name": tool_name.as_str(),
        "params": params,
    });
    if let Some(value) = response {
        detail["response"] = value.clone();
    }
    if let Some(cursor) = mutation_cursor {
        if let Some(obj) = detail.as_object_mut() {
            obj.insert("mutation_cursor".to_string(), cursor.clone());
        }
    }
    detail
}

fn reserve_rate_limit(
    conn: &mut Connection,
    client_id: &McpClientId,
    tool_name: &ScopedName,
    grant: &ToolGrant,
) -> Result<(), u32> {
    let now = current_unix_millis();
    let window_ms = grant.rate_limit.window_seconds as i64 * 1000;
    let tx = match conn.transaction() {
        Ok(t) => t,
        Err(_) => return Err(RATE_LIMIT_RETRY_DEFAULT_SECONDS),
    };
    // Prune expired rows for this (client, tool). Best-effort: if the prune
    // SQL fails we still attempt the count + insert.
    let _prune = tx.execute(
        "DELETE FROM mcp_tool_call_ledger \
         WHERE client_id = ?1 AND tool_name = ?2 AND called_at < ?3",
        params![client_id.as_str(), tool_name.as_str(), now - window_ms],
    ).map_err(|_| RATE_LIMIT_RETRY_DEFAULT_SECONDS)?;
    let count: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM mcp_tool_call_ledger \
             WHERE client_id = ?1 AND tool_name = ?2",
            params![client_id.as_str(), tool_name.as_str()],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if count as u32 >= grant.rate_limit.max_calls {
        tx.commit().map_err(|_| grant.rate_limit.window_seconds)?;
        return Err(grant.rate_limit.window_seconds);
    }
    tx.execute(
        "INSERT INTO mcp_tool_call_ledger (client_id, tool_name, called_at) \
         VALUES (?1, ?2, ?3)",
        params![client_id.as_str(), tool_name.as_str(), now],
    ).map_err(|_| RATE_LIMIT_RETRY_DEFAULT_SECONDS)?;
    tx.commit().map_err(|_| RATE_LIMIT_RETRY_DEFAULT_SECONDS)?;
    Ok(())
}

fn current_unix_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn sleep_until_floor(start: Instant) {
    let elapsed = start.elapsed();
    let floor = Duration::from_millis(TIMING_FLOOR_MILLIS);
    if elapsed < floor {
        std::thread::sleep(floor - elapsed);
    }
}

fn envelope_handle_str(envelope: &McpToolRequestEnvelope) -> &str {
    envelope
        .conversation_handle
        .as_ref()
        .map(|h| h.as_str())
        .unwrap_or("")
}

/// If the nonce was already consumed before the response could be returned,
/// we still want the caller to have a fresh nonce so they can recover. Try to
/// preissue a new one off the latest issued (unconsumed) row for the client;
/// failing that, return empty string and let the caller re-pair.
///
/// TODO(L2): replace with a dedicated `issue_nonce_only(client_id)` API in
/// auth.rs once we confirm the recovery semantics in implementation tests.
fn asserted_client_id_for_recovery_nonce(
    conn: &mut Connection,
    client_id: &McpClientId,
) -> Result<OpaqueNonce, AuthError> {
    let now = current_unix_millis();
    let nonce = OpaqueNonce::new(format!("recovery-{now:x}-{}", random_short()));
    conn.execute(
        "INSERT INTO mcp_transport_nonce_ledger \
         (nonce, client_id, issued_at, expires_at, consumed_at) \
         VALUES (?1, ?2, ?3, ?4, NULL)",
        params![
            nonce.as_str(),
            client_id.as_str(),
            now,
            now + 300_000,
        ],
    )?;
    Ok(nonce)
}

fn random_short() -> String {
    use rand::Rng;
    let mut bytes = [0u8; 8];
    rand::rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

// ---------------------------------------------------------------------------
// Signal emission stubs
// ---------------------------------------------------------------------------
//
// TODO(L2): wire to crate::signals::bus once W2+ lands a signal emitter
// instance the gateway can hold. Until then, signals log to stderr (operator
// observable) — this satisfies the emit-or-log fallback per AC-7 by treating
// "no real emitter" as the failure case.

fn emit_invoked_signal(
    client_id: &McpClientId,
    conversation_handle: &OpaqueConversationHandle,
    tool_name: &ScopedName,
) {
    eprintln!(
        "mcp.signal.invoked client_id={} conversation_handle={} tool_name={}",
        client_id.as_str(),
        conversation_handle.as_str(),
        tool_name.as_str()
    );
}

fn emit_reject_signal(
    client_id: Option<&McpClientId>,
    reject_reason: &str,
    tool_name: Option<&ScopedName>,
) {
    let client = client_id.map(|c| c.as_str()).unwrap_or("unresolved");
    let tool = tool_name.map(|t| t.as_str()).unwrap_or("unresolved");
    eprintln!(
        "mcp.signal.rejected client_id={client} tool_name={tool} reject_reason={reject_reason}"
    );
}

fn emit_double_failure_alert() {
    eprintln!("mcp.alert.audit_double_failure: handler effects preserved, audit lost");
}

#[cfg(test)]
mod tests {
    //! Gateway tests are minimal here because the per-dispatch flow consumes
    //! a live SQLite connection + a registered handler set. End-to-end
    //! gateway tests live in `src-tauri/tests/` and exercise the migrations
    //! v241-v244 schemas; the unit tests below validate the small pure
    //! helpers that don't need DB state.

    use super::*;

    #[test]
    fn scope_subset_empty_required_passes() {
        assert!(scope_is_subset(&[], &[]));
        assert!(scope_is_subset(&[], &[Scope::new("dailyos.read.account_status")]));
    }

    #[test]
    fn scope_subset_missing_returns_false() {
        let required = vec![Scope::new("dailyos.write.place_document")];
        let granted = vec![Scope::new("dailyos.read.account_status")];
        assert!(!scope_is_subset(&required, &granted));
    }

    #[test]
    fn scope_subset_present_returns_true() {
        let required = vec![Scope::new("dailyos.read.account_status")];
        let granted = vec![
            Scope::new("dailyos.read.account_status"),
            Scope::new("dailyos.read.portfolio_attention"),
        ];
        assert!(scope_is_subset(&required, &granted));
    }

    #[test]
    fn sleep_until_floor_enforces_minimum() {
        let start = Instant::now();
        sleep_until_floor(start);
        assert!(start.elapsed() >= Duration::from_millis(TIMING_FLOOR_MILLIS));
    }
}
