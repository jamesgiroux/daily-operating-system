//! MCP v2 gateway: the single entry point that receives MCP tool calls.
//!
//! Validates Actor::McpClient policy against the server-side scope manifest
//! loaded by `auth`; dispatches through registered `McpToolHandler`
//! implementations; records audit attribution on success; emits
//! `McpToolInvoked` / `McpInvocationRejected` signals on success / rejection
//! respectively via the [`SignalEmitter`] trait (W2+ wires a production
//! service-layer signal emitter through the service signal facade).
//!
//! Per ADR-0102 §C authorization machinery and the local MCP trust model:
//! authorization and operational controls live here; local transport ceremony
//! does not.

use rusqlite::{params, Connection};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

use super::actor_policy::{self, ToolGrant};
use super::audit;
use super::auth::{self, AuthError};
use super::contracts::{
    McpActor, McpClientId, McpToolHandler, McpToolRequestEnvelope, McpToolResponseEnvelope,
    McpToolResult, OpaqueConversationHandle, Scope, ScopedName, Side, ToolError,
};
use super::handler_context::{McpHandlerContext, OwnedConnection};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

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
/// W1-A); W2+ supplies a real service-layer signal emitter with a real
/// `ActionDb`.
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
    /// `client_id` is `None` for missing-client rejection (per AC-7
    /// wording, attribution is "unresolved").
    fn emit_rejected(
        &self,
        client_id: Option<&McpClientId>,
        tool_name: Option<&ScopedName>,
        reject_reason: &str,
    );

    /// Suite-S warning for successful invocations that violated a soft
    /// contract (e.g. `mcp_write_handler_missing_cursor` per AC-6 cycle-6
    /// devex MED, or `mcp_write_handler_cursor_truncated` per AC-6
    /// cycle-2 devex MED). Distinct from [`Self::emit_rejected`] so
    /// operator dashboards can separate auth rejections from handler
    /// soft-warnings (closes L2 cycle-2 convergent 3/4 MED on
    /// warning-as-rejection pollution).
    ///
    /// Carries `conversation_handle` for per-call triage correlation with
    /// the success audit row (closes L2 cycle-3 devex MED on warning
    /// correlation).
    fn emit_warning(
        &self,
        client_id: &McpClientId,
        conversation_handle: &OpaqueConversationHandle,
        tool_name: &ScopedName,
        warning_reason: &str,
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

    fn emit_warning(
        &self,
        client_id: &McpClientId,
        conversation_handle: &OpaqueConversationHandle,
        tool_name: &ScopedName,
        warning_reason: &str,
    ) {
        eprintln!(
            "mcp.signal.warning client_id={} conversation_handle={} tool_name={} warning_reason={warning_reason}",
            client_id.as_str(),
            conversation_handle.as_str(),
            tool_name.as_str()
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
    taxonomy: Option<Arc<dyn super::taxonomy::TaxonomyCatalog>>,
    /// The sidecar's single owned DB connection. `None` in tests and any path
    /// that has not adopted owned-connection threading; handlers then fall back
    /// to their prior self-open behavior. Built once in the sidecar serve path
    /// via [`Self::set_connection`].
    connection: Option<OwnedConnection>,
}

impl Gateway {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            emitter: Arc::new(StderrSignalEmitter),
            taxonomy: None,
            connection: None,
        }
    }

    pub fn with_emitter(emitter: Arc<dyn SignalEmitter>) -> Self {
        Self {
            handlers: HashMap::new(),
            emitter,
            taxonomy: None,
            connection: None,
        }
    }

    /// Install the single process-lifetime DB connection the sidecar threads
    /// into every handler. Called once by `run_v2_server` before serving.
    /// Replaces the per-handler / per-audit self-opens.
    pub fn set_connection(&mut self, connection: OwnedConnection) {
        self.connection = Some(connection);
    }

    pub fn register(&mut self, handler: Arc<dyn McpToolHandler>) {
        let name = handler.description().name.clone();
        self.handlers.insert(name, handler);
    }

    pub fn registered_tools(&self) -> impl Iterator<Item = &ScopedName> {
        self.handlers.keys()
    }

    /// Register the taxonomy catalog. Production callers wire this with
    /// `YamlTaxonomyCatalog::load_embedded()`. Tests may use a stub.
    pub fn set_taxonomy(&mut self, catalog: Arc<dyn super::taxonomy::TaxonomyCatalog>) {
        self.taxonomy = Some(catalog);
    }

    /// Seal the gateway after all handlers are registered. Validates
    /// every registered handler has a matching catalog entry with matching
    /// `Side` per the taxonomy seal contract. Returns operator-readable error if
    /// mismatch. Also returns catalog→handler pending-tool list as
    /// operator info; callers log it.
    ///
    /// Production `main.rs` calls `gateway.seal()?` after registration.
    /// No `panic!()` — fail-fast via structured `Result`.
    pub fn seal(&self) -> Result<Vec<ScopedName>, super::taxonomy::TaxonomyError> {
        let Some(catalog) = self.taxonomy.as_ref() else {
            // No taxonomy registered: tests / dev mode skip validation.
            return Ok(Vec::new());
        };
        let handlers: Vec<&dyn McpToolHandler> =
            self.handlers.values().map(|h| h.as_ref()).collect();
        catalog.validate_against_handlers(&handlers)?;
        Ok(catalog.validate_catalog_against_handlers(&handlers))
    }

    /// Handle a single MCP tool invocation. Full per-dispatch contract per
    /// Local transport identity is the asserted `McpClientId`; the gateway
    /// enforces manifest authorization and operational controls.
    pub fn handle_tool_call(
        &self,
        conn: &mut Connection,
        asserted_client_id: &McpClientId,
        envelope: McpToolRequestEnvelope,
    ) -> McpToolResponseEnvelope {
        match self.dispatch(conn, asserted_client_id, &envelope) {
            Ok(Dispatched {
                result,
                conversation_handle,
            }) => McpToolResponseEnvelope {
                conversation_handle,
                result,
            },
            Err(failure) => {
                let GatewayFailure {
                    error,
                    reject_reason,
                    tool_name_for_signal,
                    client_id_for_signal,
                } = *failure;
                self.emitter.emit_rejected(
                    client_id_for_signal.as_ref(),
                    tool_name_for_signal.as_ref(),
                    &reject_reason,
                );
                McpToolResponseEnvelope {
                    conversation_handle: OpaqueConversationHandle::new(
                        envelope_handle_str(&envelope).to_string(),
                    ),
                    result: McpToolResult::Error { error },
                }
            }
        }
    }

    /// Handle a local stdio MCP invocation without consulting or mutating the
    /// SQLite auth tables. Local stdio runs inside the same macOS user boundary;
    /// the registered process-local grant set is the authority for exposure.
    pub fn handle_local_stdio_tool_call(
        &self,
        asserted_client_id: &McpClientId,
        envelope: McpToolRequestEnvelope,
        grants: &[ToolGrant],
    ) -> McpToolResponseEnvelope {
        match self.dispatch_local_stdio(asserted_client_id, &envelope, grants) {
            Ok(dispatched) => McpToolResponseEnvelope {
                conversation_handle: dispatched.conversation_handle,
                result: dispatched.result,
            },
            Err(failure) => {
                let GatewayFailure {
                    error,
                    reject_reason,
                    tool_name_for_signal,
                    client_id_for_signal,
                } = *failure;
                self.emitter.emit_rejected(
                    client_id_for_signal.as_ref(),
                    tool_name_for_signal.as_ref(),
                    &reject_reason,
                );
                McpToolResponseEnvelope {
                    conversation_handle: OpaqueConversationHandle::new(
                        envelope_handle_str(&envelope).to_string(),
                    ),
                    result: McpToolResult::Error { error },
                }
            }
        }
    }

    /// Inner per-dispatch flow. On success: handler result + active
    /// conversation handle. On failure: `GatewayFailure` with attribution for
    /// the rejection signal.
    fn dispatch(
        &self,
        conn: &mut Connection,
        asserted_client_id: &McpClientId,
        envelope: &McpToolRequestEnvelope,
    ) -> Result<Dispatched, Box<GatewayFailure>> {
        // Manifest gates per AC-2 decision order. Unknown clients are
        // unresolved for rejection attribution; once the manifest row loads,
        // the asserted client_id is admissible for operational attribution.
        let record = match auth::load_client_record(conn, asserted_client_id) {
            Ok(r) => r,
            Err(err) => return Err(auth_to_failure(err, envelope, None)),
        };
        if record.revoked_at.is_some() {
            return Err(Box::new(GatewayFailure {
                error: ToolError::PairingRevoked,
                reject_reason: "pairing_revoked".to_string(),
                tool_name_for_signal: Some(envelope.tool_name.clone()),
                client_id_for_signal: Some(asserted_client_id.clone()),
            }));
        }

        let grant = match auth::resolve_tool_grant(conn, asserted_client_id, &envelope.tool_name) {
            Ok(g) => g,
            Err(err) => return Err(auth_to_failure(err, envelope, Some(asserted_client_id))),
        };
        let Some(grant) = grant else {
            return Err(Box::new(GatewayFailure {
                error: ToolError::ExposureForbidden {
                    tool_name: envelope.tool_name.clone(),
                },
                reject_reason: "absent_grant".to_string(),
                tool_name_for_signal: Some(envelope.tool_name.clone()),
                client_id_for_signal: Some(asserted_client_id.clone()),
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
            Err(err) => return Err(auth_to_failure(err, envelope, Some(asserted_client_id))),
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
                }));
            }
        }

        Ok(self.invoke_authorized_handler(
            handler.as_ref(),
            asserted_client_id,
            envelope,
            &grant,
            conversation_handle,
        ))
    }

    fn dispatch_local_stdio(
        &self,
        asserted_client_id: &McpClientId,
        envelope: &McpToolRequestEnvelope,
        grants: &[ToolGrant],
    ) -> Result<Dispatched, Box<GatewayFailure>> {
        let Some(grant) = grants
            .iter()
            .find(|grant| grant.tool_name == envelope.tool_name)
        else {
            return Err(Box::new(GatewayFailure {
                error: ToolError::ExposureForbidden {
                    tool_name: envelope.tool_name.clone(),
                },
                reject_reason: "absent_local_stdio_grant".to_string(),
                tool_name_for_signal: Some(envelope.tool_name.clone()),
                client_id_for_signal: Some(asserted_client_id.clone()),
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
                reject_reason: "non_invocable_local_stdio_grant".to_string(),
                tool_name_for_signal: Some(envelope.tool_name.clone()),
                client_id_for_signal: Some(asserted_client_id.clone()),
            }));
        }

        let Some(handler) = self.handlers.get(&envelope.tool_name) else {
            return Err(Box::new(GatewayFailure {
                error: ToolError::BadParams {
                    detail: "unknown tool name".to_string(),
                },
                reject_reason: "unknown_tool".to_string(),
                tool_name_for_signal: Some(envelope.tool_name.clone()),
                client_id_for_signal: Some(asserted_client_id.clone()),
            }));
        };

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
            }));
        }

        if let Some(params_obj) = envelope.params.as_object() {
            if params_obj.contains_key("granted_scopes") {
                return Err(Box::new(GatewayFailure {
                    error: ToolError::BadParams {
                        detail: "granted_scopes not accepted in params".to_string(),
                    },
                    reject_reason: "caller_asserted_scopes".to_string(),
                    tool_name_for_signal: Some(envelope.tool_name.clone()),
                    client_id_for_signal: Some(asserted_client_id.clone()),
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
                }));
            }
        }

        let conversation_handle = envelope.conversation_handle.clone().unwrap_or_else(|| {
            OpaqueConversationHandle::new(format!("local-stdio-{}", uuid::Uuid::new_v4()))
        });

        Ok(self.invoke_authorized_handler(
            handler.as_ref(),
            asserted_client_id,
            envelope,
            grant,
            conversation_handle,
        ))
    }

    fn invoke_authorized_handler(
        &self,
        handler: &dyn McpToolHandler,
        asserted_client_id: &McpClientId,
        envelope: &McpToolRequestEnvelope,
        grant: &ToolGrant,
        conversation_handle: OpaqueConversationHandle,
    ) -> Dispatched {
        let runtime_actor =
            actor_policy::project_actor(asserted_client_id, grant, Some(&conversation_handle));
        let wire_actor = McpActor::Client {
            client_id: asserted_client_id.clone(),
            conversation_handle: Some(conversation_handle.clone()),
            tool_name: envelope.tool_name.clone(),
            granted_scopes: grant.scopes_granted.clone(),
        };
        // Hand the handler the sidecar's one owned connection when installed,
        // instead of letting it self-open. `None` preserves the prior
        // self-open fallback for tests / unadopted paths.
        let ctx = match self.connection.as_ref() {
            Some(connection) => McpHandlerContext::with_owned_connection(connection.clone()),
            None => McpHandlerContext::without_connection(),
        };
        let invocation = handler.invoke(&ctx, &wire_actor, envelope.params.clone());
        let side = handler.description().side;

        let result = match invocation {
            Ok(value) => {
                let mutation_cursor = if matches!(side, Side::Write | Side::SubmitCorrection) {
                    let extracted = value.get("mutation_cursor").cloned();
                    if extracted.is_none() {
                        self.emitter.emit_warning(
                            asserted_client_id,
                            &conversation_handle,
                            &envelope.tool_name,
                            "mcp_write_handler_missing_cursor",
                        );
                    }
                    extracted.map(|cursor| {
                        let (capped, was_truncated) = cap_mutation_cursor(cursor);
                        if was_truncated {
                            self.emitter.emit_warning(
                                asserted_client_id,
                                &conversation_handle,
                                &envelope.tool_name,
                                "mcp_write_handler_cursor_truncated",
                            );
                        }
                        capped
                    })
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
                // Route the audit-outbox fallback through the one owned
                // connection when the sidecar installed it; otherwise `None`
                // keeps the in-app try_global / self-open chain.
                let audit_result = match self.connection.as_ref() {
                    Some(owned) => {
                        let guard = owned
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner);
                        audit::write_with_conn(
                            &runtime_actor,
                            "mcp.tool_invoked",
                            audit_detail,
                            None,
                            side,
                            Some(&guard),
                        )
                    }
                    None => {
                        audit::write(&runtime_actor, "mcp.tool_invoked", audit_detail, None, side)
                    }
                };
                if let Err(err) = audit_result {
                    if matches!(err, audit::AuditError::DoubleFailure { .. }) {
                        emit_double_failure_alert();
                        return Dispatched {
                            result: McpToolResult::Error {
                                error: ToolError::Internal {
                                    trace_id: "mcp_audit_double_failure".to_string(),
                                },
                            },
                            conversation_handle,
                        };
                    }
                    eprintln!("mcp_v2 audit single-failure: {err}");
                }

                self.emitter.emit_invoked(
                    asserted_client_id,
                    &conversation_handle,
                    &envelope.tool_name,
                );
                McpToolResult::Ok { value }
            }
            Err(error) => McpToolResult::Error { error },
        };

        Dispatched {
            result,
            conversation_handle,
        }
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
    conversation_handle: OpaqueConversationHandle,
}

/// Carries enough state to (a) build the typed `ToolError` response and
/// (b) emit the reject signal with operational attribution.
struct GatewayFailure {
    error: ToolError,
    reject_reason: String,
    tool_name_for_signal: Option<ScopedName>,
    client_id_for_signal: Option<McpClientId>,
}

/// Project an `AuthError` to a gateway failure.
fn auth_to_failure(
    err: AuthError,
    envelope: &McpToolRequestEnvelope,
    attributed_client: Option<&McpClientId>,
) -> Box<GatewayFailure> {
    let (tool_error, reason) = match err {
        AuthError::UnknownClient => (
            ToolError::BadParams {
                detail: "unknown_client".to_string(),
            },
            "unknown_client".to_string(),
        ),
        AuthError::PairingRevoked => (ToolError::PairingRevoked, "pairing_revoked".to_string()),
        AuthError::ConversationRevoked => (
            ToolError::ConversationRevoked,
            "conversation_revoked".to_string(),
        ),
        AuthError::Sqlite(detail) => (
            ToolError::Internal {
                trace_id: opaque_trace_id("auth_sqlite", &detail.to_string()),
            },
            "sqlite_error".to_string(),
        ),
        AuthError::Encoding(detail) => (
            ToolError::Internal {
                trace_id: opaque_trace_id("auth_encoding", &detail.to_string()),
            },
            "encoding_error".to_string(),
        ),
    };
    Box::new(GatewayFailure {
        error: tool_error,
        reject_reason: reason,
        tool_name_for_signal: Some(envelope.tool_name.clone()),
        client_id_for_signal: attributed_client.cloned(),
    })
}

/// Build an opaque `trace_id` for `ToolError::Internal`: never leak raw DB /
/// encoding error strings on the wire. Format: `<category>-<sha256-hex-12>`.
/// The detail is logged to stderr server-side so operators can grep by
/// trace_id.
fn opaque_trace_id(category: &str, detail: &str) -> String {
    use ring::digest;
    let digest = digest::digest(&digest::SHA256, detail.as_bytes());
    let hex = digest
        .as_ref()
        .iter()
        .take(6)
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let trace_id = format!("{category}-{hex}");
    eprintln!("mcp_v2 internal: trace_id={trace_id} detail={detail}");
    trace_id
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
///
/// Returns `(capped_value, was_truncated)` so the caller can emit a
/// Suite-S warning per L2 cycle-2 devex MED — silent truncation breaks
/// the taxonomy.rs rustdoc contract.
fn cap_mutation_cursor(value: serde_json::Value) -> (serde_json::Value, bool) {
    if cursor_depth(&value) > MUTATION_CURSOR_MAX_DEPTH {
        return (
            serde_json::Value::String(MUTATION_CURSOR_TRUNCATION_SENTINEL.to_string()),
            true,
        );
    }
    match serde_json::to_vec(&value) {
        Ok(bytes) if bytes.len() <= MUTATION_CURSOR_MAX_BYTES => (value, false),
        _ => (
            serde_json::Value::String(MUTATION_CURSOR_TRUNCATION_SENTINEL.to_string()),
            true,
        ),
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

    // L2 cycle-3 codex review + code-reviewer convergent MED AC-5: a
    // SELECT failure must NOT fail-open by being treated as 0. ROLLBACK
    // + reject so the call is gated correctly.
    let count: i64 = match conn.query_row(
        "SELECT COUNT(*) FROM mcp_tool_call_ledger \
         WHERE client_id = ?1 AND tool_name = ?2",
        params![client_id.as_str(), tool_name.as_str()],
        |row| row.get(0),
    ) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("mcp_v2 rate-limit SELECT COUNT failed: {err}");
            if let Err(rollback_err) = conn.execute_batch("ROLLBACK") {
                eprintln!("mcp_v2 rate-limit ROLLBACK after SELECT err: {rollback_err}");
            }
            return Err(RATE_LIMIT_RETRY_DEFAULT_SECONDS);
        }
    };
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

    // L2 cycle-2 code-reviewer NEW: COMMIT failure must roll back the
    // pending tx to avoid leaving a long-held writer lock open.
    if let Err(commit_err) = conn.execute_batch("COMMIT") {
        eprintln!("mcp_v2 rate-limit COMMIT failed; rolling back: {commit_err}");
        if let Err(rollback_err) = conn.execute_batch("ROLLBACK") {
            eprintln!("mcp_v2 rate-limit ROLLBACK after COMMIT failure: {rollback_err}");
        }
        return Err(RATE_LIMIT_RETRY_DEFAULT_SECONDS);
    }
    Ok(())
}

fn current_unix_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
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

    use super::super::contracts::{ParamSchema, ReturnSpec, ToolDescription};
    use super::*;
    use crate::db::ActionDb;
    use crate::services::mcp_v2::actor_policy::ToolRateLimit;
    use std::sync::Mutex;

    struct RecordingContextHandler {
        description: ToolDescription,
        connection_pointers: Arc<Mutex<Vec<usize>>>,
    }

    impl RecordingContextHandler {
        fn new(tool_name: &str, connection_pointers: Arc<Mutex<Vec<usize>>>) -> Self {
            Self {
                description: ToolDescription {
                    name: ScopedName::new(tool_name),
                    summary: "record context connection".to_string(),
                    when_to_call: "test only".to_string(),
                    when_not_to_call: "never outside tests".to_string(),
                    side: Side::Read,
                    parameters: Vec::new(),
                    returns: ReturnSpec {
                        schema: ParamSchema(json!({ "type": "object" })),
                        description: "test payload".to_string(),
                    },
                    examples: Vec::new(),
                    scopes_required: vec![Scope::new(tool_name)],
                },
                connection_pointers,
            }
        }
    }

    impl McpToolHandler for RecordingContextHandler {
        fn description(&self) -> &ToolDescription {
            &self.description
        }

        fn invoke(
            &self,
            ctx: &McpHandlerContext,
            _actor: &McpActor,
            _params: serde_json::Value,
        ) -> Result<serde_json::Value, ToolError> {
            let pointer = ctx
                .with_conn(|db| db.conn_ref() as *const rusqlite::Connection as usize)
                .ok_or_else(|| ToolError::Internal {
                    trace_id: "missing_owned_context_connection".to_string(),
                })?;
            self.connection_pointers
                .lock()
                .expect("connection pointer lock")
                .push(pointer);
            Ok(json!({ "connectionPointer": pointer }))
        }
    }

    fn recording_grant(tool_name: &str) -> ToolGrant {
        ToolGrant {
            tool_name: ScopedName::new(tool_name),
            scopes_granted: vec![Scope::new(tool_name)],
            exposure: abilities_runtime::abilities::registry::McpExposure::Invocable,
            rate_limit: ToolRateLimit {
                max_calls: 0,
                window_seconds: 0,
            },
        }
    }

    fn envelope(tool_name: &str) -> McpToolRequestEnvelope {
        McpToolRequestEnvelope {
            conversation_handle: None,
            tool_name: ScopedName::new(tool_name),
            params: json!({}),
        }
    }

    #[test]
    fn scope_subset_empty_required_passes() {
        assert!(scope_is_subset(&[], &[]));
        assert!(scope_is_subset(
            &[],
            &[Scope::new("dailyos.read.account_status")]
        ));
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
    fn local_stdio_dispatch_reuses_gateway_owned_connection_across_calls() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("gateway-owned.db");
        let owned = Arc::new(Mutex::new(
            ActionDb::open_at_unencrypted(path).expect("owned gateway db"),
        ));
        let connection_pointers = Arc::new(Mutex::new(Vec::new()));
        let tool_name = "dailyos.read.gateway_context_probe";

        let mut gateway = Gateway::new();
        gateway.set_connection(Arc::clone(&owned));
        gateway.register(Arc::new(RecordingContextHandler::new(
            tool_name,
            Arc::clone(&connection_pointers),
        )));

        let client_id = McpClientId::new("local-stdio-test");
        let grants = vec![recording_grant(tool_name)];

        for _ in 0..2 {
            let response = gateway.handle_local_stdio_tool_call(
                &client_id,
                envelope(tool_name),
                grants.as_slice(),
            );
            assert!(
                matches!(response.result, McpToolResult::Ok { .. }),
                "probe handler must dispatch through the owned context"
            );
        }

        let pointers = connection_pointers.lock().expect("connection pointer lock");
        assert_eq!(
            pointers.len(),
            2,
            "two dispatches should invoke the handler twice"
        );
        assert_eq!(
            pointers[0], pointers[1],
            "gateway dispatch must thread the same owned DB connection into every call"
        );
        assert_eq!(
            Arc::strong_count(&owned),
            2,
            "gateway plus test should share one owned connection, not clone per dispatch"
        );
    }

    #[test]
    fn cap_mutation_cursor_passes_small_payload() {
        let cursor = json!({ "claim_id": 42, "signal_id": "abc" });
        let (capped, was_truncated) = cap_mutation_cursor(cursor.clone());
        assert_eq!(capped, cursor);
        assert!(!was_truncated);
    }

    #[test]
    fn cap_mutation_cursor_truncates_oversize_bytes() {
        let huge: Vec<i64> = (0..1000).collect();
        let cursor = json!({ "ids": huge });
        let (capped, was_truncated) = cap_mutation_cursor(cursor);
        assert_eq!(capped, json!("truncated_oversize"));
        assert!(was_truncated);
    }

    #[test]
    fn cap_mutation_cursor_truncates_deep_nesting() {
        // Build object {a:{a:{a:{a:{a:{a:1}}}}}} — depth 6, > MAX_DEPTH 4.
        let mut value = json!({ "a": 1 });
        for _ in 0..6 {
            value = json!({ "a": value });
        }
        let (capped, was_truncated) = cap_mutation_cursor(value);
        assert_eq!(capped, json!("truncated_oversize"));
        assert!(was_truncated);
    }
}
