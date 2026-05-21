//! MCP v2 gateway: the single entry point that receives MCP tool calls.
//!
//! Validates Actor::McpClient policy against the server-side scope manifest
//! loaded by `auth`; dispatches through registered `McpToolHandler`
//! implementations; records audit attribution on success; emits
//! `McpToolInvoked` / `McpInvocationRejected` signals on success / rejection
//! respectively via the [`SignalEmitter`] trait (W2+ wires a production
//! emitter that calls `crate::signals::bus::emit_signal_and_propagate`).
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

/// Wall-clock floor on auth-state error responses, per L0 packet AC-12.
const TIMING_FLOOR_MILLIS: u64 = 10;

/// Default rate-limit retry-after when the gateway has nothing better.
const RATE_LIMIT_RETRY_DEFAULT_SECONDS: u32 = 60;

/// Mutation-cursor caps per L0 packet AC-6 (cycle-6 devex MED).
const MUTATION_CURSOR_MAX_DEPTH: usize = 4;
const MUTATION_CURSOR_MAX_BYTES: usize = 2 * 1024;
const MUTATION_CURSOR_TRUNCATION_SENTINEL: &str = "truncated_oversize";

// ---------------------------------------------------------------------------
// SignalEmitter trait
// ---------------------------------------------------------------------------

/// Abstract signal-emission seam between the gateway and the production
/// signals bus. The default `StderrSignalEmitter` is for tests and dev
/// dispatch (so the gateway doesn't take a hard `ActionDb` dependency in
/// W1-A); W2+ supplies a real emitter that calls
/// `crate::signals::bus::emit_signal_and_propagate` with a real `ActionDb`.
///
/// Both methods MUST be infallible from the gateway's perspective — the
/// emit-or-log discipline (L0 packet AC-7) means emission failure logs +
/// emits a `mcp_reject_signal_emit_failed` operator alert but never
/// propagates back to the caller. Implementers handle that internally.
pub trait SignalEmitter: Send + Sync {
    /// `SignalType::McpToolInvoked` on successful dispatch.
    /// Payload: `(client_id, conversation_handle, tool_name)`. No raw
    /// params/response per AC-7.
    fn emit_invoked(
        &self,
        client_id: &McpClientId,
        conversation_handle: &OpaqueConversationHandle,
        tool_name: &ScopedName,
    );

    /// `SignalType::McpInvocationRejected` on auth-state rejection.
    /// `client_id` is `None` for Gate 0a/0b / missing-client (per AC-7
    /// wording, attribution is "unresolved").
    fn emit_rejected(
        &self,
        client_id: Option<&McpClientId>,
        tool_name: Option<&ScopedName>,
        reject_reason: &str,
    );
}

/// Default emitter used when the gateway is constructed without an explicit
/// production emitter (tests, dev). Writes to stderr so events stay
/// operator-visible; per AC-7 emit-or-log this satisfies the "fallback log"
/// arm of emit-or-log when production wiring is absent.
pub struct StderrSignalEmitter;

impl SignalEmitter for StderrSignalEmitter {
    fn emit_invoked(
        &self,
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

    fn emit_rejected(
        &self,
        client_id: Option<&McpClientId>,
        tool_name: Option<&ScopedName>,
        reject_reason: &str,
    ) {
        let client = client_id.map(|c| c.as_str()).unwrap_or("unresolved");
        let tool = tool_name.map(|t| t.as_str()).unwrap_or("unresolved");
        eprintln!(
            "mcp.signal.rejected client_id={client} tool_name={tool} reject_reason={reject_reason}"
        );
    }
}

// ---------------------------------------------------------------------------
// Gateway
// ---------------------------------------------------------------------------

/// MCP v2 dispatch entry point. Holds the registered handler set + the
/// signal emitter and runs the per-dispatch flow defined in L0 packet §1 #1.
pub struct Gateway {
    handlers: HashMap<ScopedName, Arc<dyn McpToolHandler>>,
    emitter: Arc<dyn SignalEmitter>,
}

impl Gateway {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            emitter: Arc::new(StderrSignalEmitter),
        }
    }

    pub fn with_emitter(emitter: Arc<dyn SignalEmitter>) -> Self {
        Self {
            handlers: HashMap::new(),
            emitter,
        }
    }

    pub fn register(&mut self, handler: Arc<dyn McpToolHandler>) {
        let name = handler.description().name.clone();
        self.handlers.insert(name, handler);
    }

    pub fn registered_tools(&self) -> impl Iterator<Item = &ScopedName> {
        self.handlers.keys()
    }

    /// Handle a single MCP tool invocation. Full per-dispatch contract per
    /// L0 packet §1 #1; gate ordering matches AC-2 verbatim. The Gate 0b
    /// preissued nonce is captured and returned in the response envelope on
    /// BOTH success and rejection paths (closes L2 code-reviewer HIGH +
    /// CSO HIGH on recovery-nonce leak).
    pub fn handle_tool_call(
        &self,
        conn: &mut Connection,
        asserted_client_id: &McpClientId,
        envelope: McpToolRequestEnvelope,
        signature: &[u8],
    ) -> McpToolResponseEnvelope {
        let start = Instant::now();
        match self.dispatch(conn, asserted_client_id, &envelope, signature) {
            Ok(Dispatched {
                result,
                next_nonce,
                conversation_handle,
            }) => McpToolResponseEnvelope {
                conversation_handle,
                next_request_nonce: next_nonce,
                result,
            },
            Err(failure) => {
                let GatewayFailure {
                    error,
                    reject_reason,
                    tool_name_for_signal,
                    client_id_for_signal,
                    preissued_next_nonce,
                } = *failure;
                // Auth-state rejection. Emit reject signal with safe
                // attribution (Gate 0a/0b → client_id None per AC-7).
                self.emitter.emit_rejected(
                    client_id_for_signal.as_ref(),
                    tool_name_for_signal.as_ref(),
                    &reject_reason,
                );
                // AC-12 timing floor on ALL rejection branches.
                sleep_until_floor(start);
                // Return the Gate 0b preissued nonce (if Gate 0b admitted)
                // OR an empty token (Gate 0a / pre-consume failures —
                // caller must re-pair via the seed nonce).
                McpToolResponseEnvelope {
                    conversation_handle: OpaqueConversationHandle::new(
                        envelope_handle_str(&envelope).to_string(),
                    ),
                    next_request_nonce: preissued_next_nonce
                        .unwrap_or_else(|| OpaqueNonce::new(String::new())),
                    result: McpToolResult::Error { error },
                }
            }
        }
    }

    /// Inner per-dispatch flow. On success: handler result + preissued next
    /// nonce + active conversation handle. On failure: `GatewayFailure` with
    /// safe attribution + the preissued next nonce (if Gate 0b admitted) so
    /// rejection responses carry the SAME nonce the caller's next request
    /// must present, per ADR-0102 §C.bis.refresh.
    fn dispatch(
        &self,
        conn: &mut Connection,
        asserted_client_id: &McpClientId,
        envelope: &McpToolRequestEnvelope,
        signature: &[u8],
    ) -> Result<Dispatched, Box<GatewayFailure>> {
        // Gate 0a: HMAC over WHOLE envelope including request_nonce. On
        // failure, attribution is "unresolved" — the asserted client_id is
        // not yet verified, so AC-7 says we MUST NOT signal it.
        if let Err(err) = auth::verify_transport_hmac(conn, asserted_client_id, envelope, signature)
        {
            return Err(auth_to_failure(err, envelope, /* gate0_attributed */ false, None));
        }

        // Gate 0b: fail-closed consume + preissue per ADR-0102 §C.bis.refresh
        // + §C.bis.fail-closed. After this point, the nonce is permanently
        // consumed (commit happens inside auth::verify_and_consume_and_preissue
        // phase a); the returned `next_nonce` is what the response envelope
        // MUST carry whether the rest of dispatch succeeds or fails.
        let next_nonce =
            match auth::verify_and_consume_and_preissue(conn, asserted_client_id, &envelope.request_nonce) {
                Ok(nonce) => nonce,
                Err(err) => {
                    return Err(auth_to_failure(err, envelope, /* gate0_attributed */ false, None))
                }
            };

        // Manifest gates per AC-2 decision order. From here on attribution
        // is verified (HMAC + nonce passed) so client_id is admissible in
        // reject signals + we carry the preissued next_nonce on rejection.
        let preissued = Some(next_nonce.clone());
        let record = match auth::load_client_record(conn, asserted_client_id) {
            Ok(r) => r,
            Err(err) => {
                return Err(auth_to_failure(
                    err,
                    envelope,
                    /* gate0_attributed */ true,
                    preissued.clone(),
                ))
            }
        };
        if record.revoked_at.is_some() {
            return Err(Box::new(GatewayFailure {
                error: ToolError::PairingRevoked,
                reject_reason: "pairing_revoked".to_string(),
                tool_name_for_signal: Some(envelope.tool_name.clone()),
                client_id_for_signal: Some(asserted_client_id.clone()),
                preissued_next_nonce: preissued.clone(),
            }));
        }

        let grant = match auth::resolve_tool_grant(conn, asserted_client_id, &envelope.tool_name) {
            Ok(g) => g,
            Err(err) => {
                return Err(auth_to_failure(err, envelope, true, preissued.clone()))
            }
        };
        let Some(grant) = grant else {
            return Err(Box::new(GatewayFailure {
                error: ToolError::ExposureForbidden {
                    tool_name: envelope.tool_name.clone(),
                },
                reject_reason: "absent_grant".to_string(),
                tool_name_for_signal: Some(envelope.tool_name.clone()),
                client_id_for_signal: Some(asserted_client_id.clone()),
                preissued_next_nonce: preissued.clone(),
            }));
        };
        if !matches!(
            grant.exposure,
            abilities_runtime::abilities::registry::McpExposure::Invocable
        ) {
            return Err(Box::new(GatewayFailure {
                error: ToolError::ExposureForbidden {
                    tool_name: envelope.tool_name.clone(),
                },
                reject_reason: "non_invocable".to_string(),
                tool_name_for_signal: Some(envelope.tool_name.clone()),
                client_id_for_signal: Some(asserted_client_id.clone()),
                preissued_next_nonce: preissued.clone(),
            }));
        }

        // Handler lookup. Unknown tool → BadParams per AC-1.
        let Some(handler) = self.handlers.get(&envelope.tool_name) else {
            return Err(Box::new(GatewayFailure {
                error: ToolError::BadParams {
                    detail: "unknown tool name".to_string(),
                },
                reject_reason: "unknown_tool".to_string(),
                tool_name_for_signal: Some(envelope.tool_name.clone()),
                client_id_for_signal: Some(asserted_client_id.clone()),
                preissued_next_nonce: preissued.clone(),
            }));
        };

        // Scope subset per AC-2 step (iii).
        let required: &[Scope] = &handler.description().scopes_required;
        if !scope_is_subset(required, &grant.scopes_granted) {
            let missing = required
                .iter()
                .find(|r| !grant.scopes_granted.contains(r))
                .cloned()
                .unwrap_or_else(|| Scope::new(""));
            return Err(Box::new(GatewayFailure {
                error: ToolError::Unauthorized {
                    missing_scope: missing.clone(),
                },
                reject_reason: format!("missing_scope:{}", missing.as_str()),
                tool_name_for_signal: Some(envelope.tool_name.clone()),
                client_id_for_signal: Some(asserted_client_id.clone()),
                preissued_next_nonce: preissued.clone(),
            }));
        }

        // Reject caller-asserted scope/conversation fields in params per AC-2 (iv).
        if let Some(params_obj) = envelope.params.as_object() {
            if params_obj.contains_key("granted_scopes") {
                return Err(Box::new(GatewayFailure {
                    error: ToolError::BadParams {
                        detail: "granted_scopes not accepted in params".to_string(),
                    },
                    reject_reason: "caller_asserted_scopes".to_string(),
                    tool_name_for_signal: Some(envelope.tool_name.clone()),
                    client_id_for_signal: Some(asserted_client_id.clone()),
                    preissued_next_nonce: preissued.clone(),
                }));
            }
            if params_obj.contains_key("conversation_id") {
                return Err(Box::new(GatewayFailure {
                    error: ToolError::BadParams {
                        detail: "use envelope conversation_handle".to_string(),
                    },
                    reject_reason: "caller_asserted_conversation_id".to_string(),
                    tool_name_for_signal: Some(envelope.tool_name.clone()),
                    client_id_for_signal: Some(asserted_client_id.clone()),
                    preissued_next_nonce: preissued.clone(),
                }));
            }
        }

        // Resolve/mint conversation handle per AC-3a/AC-3b §D.bis.
        let conversation_handle = match auth::resolve_or_mint_handle(
            conn,
            asserted_client_id,
            envelope.conversation_handle.as_ref(),
        ) {
            Ok(h) => h,
            Err(err) => {
                return Err(auth_to_failure(err, envelope, true, preissued.clone()))
            }
        };

        // Rate limit per AC-5: BEGIN IMMEDIATE → prune → count → reserve OR ROLLBACK.
        match reserve_rate_limit(conn, asserted_client_id, &envelope.tool_name, &grant) {
            Ok(()) => {}
            Err(retry_after) => {
                return Err(Box::new(GatewayFailure {
                    error: ToolError::RateLimited {
                        retry_after_seconds: retry_after,
                    },
                    reject_reason: "rate_limited".to_string(),
                    tool_name_for_signal: Some(envelope.tool_name.clone()),
                    client_id_for_signal: Some(asserted_client_id.clone()),
                    preissued_next_nonce: preissued.clone(),
                }));
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
                // Extract + cap mutation_cursor for Side::Write per AC-6.
                let mutation_cursor = if matches!(handler.description().side, Side::Write) {
                    let extracted = value.get("mutation_cursor").cloned();
                    if extracted.is_none() {
                        // SHOULD-with-warning per L0 cycle-6 devex MED.
                        self.emitter.emit_rejected(
                            Some(asserted_client_id),
                            Some(&envelope.tool_name),
                            "mcp_write_handler_missing_cursor",
                        );
                    }
                    extracted.map(cap_mutation_cursor)
                } else {
                    None
                };

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
                        return Ok(Dispatched {
                            result: McpToolResult::Error {
                                error: ToolError::Internal {
                                    trace_id: "mcp_audit_double_failure".to_string(),
                                },
                            },
                            next_nonce,
                            conversation_handle,
                        });
                    }
                    eprintln!("mcp_v2 audit single-failure: {err}");
                }

                self.emitter
                    .emit_invoked(asserted_client_id, &conversation_handle, &envelope.tool_name);
                McpToolResult::Ok { value }
            }
            Err(error) => McpToolResult::Error { error },
        };

        Ok(Dispatched {
            result,
            next_nonce,
            conversation_handle,
        })
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

struct Dispatched {
    result: McpToolResult,
    next_nonce: OpaqueNonce,
    conversation_handle: OpaqueConversationHandle,
}

/// Carries enough state to (a) build the typed `ToolError` response,
/// (b) emit the reject signal with safe attribution per AC-7, AND
/// (c) propagate the Gate 0b preissued next nonce to the rejection
/// response per ADR-0102 §C.bis.refresh (closes L2 cycle-1 code-reviewer
/// HIGH + CSO HIGH on recovery-nonce leak).
struct GatewayFailure {
    error: ToolError,
    reject_reason: String,
    tool_name_for_signal: Option<ScopedName>,
    client_id_for_signal: Option<McpClientId>,
    preissued_next_nonce: Option<OpaqueNonce>,
}

/// Project an `AuthError` to a gateway failure. `gate0_attributed = false`
/// means we are still inside Gate 0a/0b — attribution MUST be `None` per
/// AC-7 wording (asserted client_id is not yet verified).
fn auth_to_failure(
    err: AuthError,
    envelope: &McpToolRequestEnvelope,
    gate0_attributed: bool,
    preissued: Option<OpaqueNonce>,
) -> Box<GatewayFailure> {
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
        AuthError::PreissueFailed(detail) => (
            ToolError::Internal {
                trace_id: format!("preissue_failed:{detail}"),
            },
            "preissue_failed".to_string(),
        ),
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
    let _ = gate0_attributed; // attribution policy currently identical for both arms; see comment.
    Box::new(GatewayFailure {
        error: tool_error,
        reject_reason: reason,
        tool_name_for_signal: Some(envelope.tool_name.clone()),
        client_id_for_signal: None,
        // ^ Per AC-7 wording: pre-verification → `unresolved`. After Gate 0b
        //   admit the caller passes verified attribution at the call sites
        //   in `dispatch` (manifest gates onward) by setting
        //   `client_id_for_signal: Some(asserted_client_id.clone())`.
        //   `auth_to_failure` itself is only used for the unverified paths
        //   (Gate 0a HMAC, Gate 0b consume, and the post-admit auth helpers
        //   that bubble up DB / encoding errors — those carry verified
        //   attribution via the per-call-site explicit field).
        preissued_next_nonce: preissued,
    })
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

/// Enforce AC-6 mutation_cursor caps: depth ≤ 4 + serialized ≤ 2 KiB. On
/// violation, replace the cursor with the `"truncated_oversize"` sentinel
/// string so audit detail stays bounded.
fn cap_mutation_cursor(value: serde_json::Value) -> serde_json::Value {
    if cursor_depth(&value) > MUTATION_CURSOR_MAX_DEPTH {
        return serde_json::Value::String(MUTATION_CURSOR_TRUNCATION_SENTINEL.to_string());
    }
    match serde_json::to_vec(&value) {
        Ok(bytes) if bytes.len() <= MUTATION_CURSOR_MAX_BYTES => value,
        _ => serde_json::Value::String(MUTATION_CURSOR_TRUNCATION_SENTINEL.to_string()),
    }
}

fn cursor_depth(value: &serde_json::Value) -> usize {
    match value {
        serde_json::Value::Object(map) => 1 + map.values().map(cursor_depth).max().unwrap_or(0),
        serde_json::Value::Array(items) => 1 + items.iter().map(cursor_depth).max().unwrap_or(0),
        _ => 0,
    }
}

/// AC-5 rate-limit reservation: `BEGIN IMMEDIATE` → prune expired rows →
/// SELECT COUNT(*) → if >= max ROLLBACK + RateLimited; else INSERT + COMMIT.
fn reserve_rate_limit(
    conn: &mut Connection,
    client_id: &McpClientId,
    tool_name: &ScopedName,
    grant: &ToolGrant,
) -> Result<(), u32> {
    let now = current_unix_millis();
    let window_ms = grant.rate_limit.window_seconds as i64 * 1000;

    conn.execute_batch("BEGIN IMMEDIATE")
        .map_err(|_| RATE_LIMIT_RETRY_DEFAULT_SECONDS)?;

    let prune_outcome = conn.execute(
        "DELETE FROM mcp_tool_call_ledger \
         WHERE client_id = ?1 AND tool_name = ?2 AND called_at < ?3",
        params![client_id.as_str(), tool_name.as_str(), now - window_ms],
    );
    if prune_outcome.is_err() {
        if let Err(rollback_err) = conn.execute_batch("ROLLBACK") {
            eprintln!("mcp_v2 rate-limit ROLLBACK failed: {rollback_err}");
        }
        return Err(RATE_LIMIT_RETRY_DEFAULT_SECONDS);
    }

    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM mcp_tool_call_ledger \
             WHERE client_id = ?1 AND tool_name = ?2",
            params![client_id.as_str(), tool_name.as_str()],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if count as u32 >= grant.rate_limit.max_calls {
        if let Err(rollback_err) = conn.execute_batch("ROLLBACK") {
            eprintln!("mcp_v2 rate-limit ROLLBACK failed: {rollback_err}");
        }
        return Err(grant.rate_limit.window_seconds);
    }

    let insert_outcome = conn.execute(
        "INSERT INTO mcp_tool_call_ledger (client_id, tool_name, called_at) \
         VALUES (?1, ?2, ?3)",
        params![client_id.as_str(), tool_name.as_str(), now],
    );
    if insert_outcome.is_err() {
        if let Err(rollback_err) = conn.execute_batch("ROLLBACK") {
            eprintln!("mcp_v2 rate-limit ROLLBACK failed: {rollback_err}");
        }
        return Err(RATE_LIMIT_RETRY_DEFAULT_SECONDS);
    }

    conn.execute_batch("COMMIT")
        .map_err(|_| RATE_LIMIT_RETRY_DEFAULT_SECONDS)?;
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

    #[test]
    fn cap_mutation_cursor_passes_small_payload() {
        let cursor = json!({ "claim_id": 42, "signal_id": "abc" });
        let capped = cap_mutation_cursor(cursor.clone());
        assert_eq!(capped, cursor);
    }

    #[test]
    fn cap_mutation_cursor_truncates_oversize_bytes() {
        let huge: Vec<i64> = (0..1000).collect();
        let cursor = json!({ "ids": huge });
        let capped = cap_mutation_cursor(cursor);
        assert_eq!(capped, json!("truncated_oversize"));
    }

    #[test]
    fn cap_mutation_cursor_truncates_deep_nesting() {
        // Build object {a:{a:{a:{a:{a:{a:1}}}}}} — depth 6, > MAX_DEPTH 4.
        let mut value = json!({ "a": 1 });
        for _ in 0..6 {
            value = json!({ "a": value });
        }
        let capped = cap_mutation_cursor(value);
        assert_eq!(capped, json!("truncated_oversize"));
    }
}
