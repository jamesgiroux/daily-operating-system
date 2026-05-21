//! MCP v2 transport ingress — rmcp::ServerHandler implementation.
//!
//! Bridges the official `rmcp` crate to the v2 Gateway so standard MCP
//! clients (Claude Desktop, Cursor, custom local SDK) invoke v2 tools
//! through the W1-A trust contract.
//!
//! Architecture (per `.docs/plans/v1.4.7-w1-foundation/dos-mcp-transport-l0-plan.md`):
//!
//! - **Scope**: local-to-local same-machine only. Trust boundary is
//!   "same user on this machine." No remote MCP transport in v1.4.7.
//! - **Identity**: env-asserted at process startup
//!   (`DAILYOS_MCP_CLIENT_ID` + `DAILYOS_MCP_TRANSPORT_KEY`). Verified
//!   in `main.rs` BEFORE constructing [`V2ServerHandler`].
//! - **Per-call signing**: stdio transport self-signs envelopes
//!   internally using the process-held `transport_key`. Standard MCP
//!   clients send normal `{ name, arguments }` shape; the transport
//!   builds + signs the [`McpToolRequestEnvelope`] before calling
//!   [`Gateway::handle_tool_call`]. `_dailyos_*` keys are NOT exposed
//!   in public `tools/list` `input_schema`.
//! - **Trust model**: for stdio transport, per-envelope HMAC + nonce
//!   ledger consumption are transport-internal hygiene (the same
//!   process signs and verifies via the same key). Real client auth
//!   for stdio is the startup env-assertion. For WP adapter / SDK
//!   transports (other lanes), HMAC + nonce remain real client-to-
//!   gateway authentication.

use std::sync::Arc;

use parking_lot::Mutex;
use ring::hmac;
use rmcp::model::{
    CallToolRequestParam, CallToolResult, Content, ErrorCode, ErrorData, Implementation,
    JsonObject, ListToolsResult, PaginatedRequestParam, ProtocolVersion, ServerCapabilities,
    ServerInfo, Tool,
};
use rmcp::service::RequestContext;
use rmcp::{RoleServer, ServerHandler};
use rusqlite::Connection;
use zeroize::Zeroizing;

use super::auth;
use super::contracts::{
    McpClientId, McpToolRequestEnvelope, McpToolResponseEnvelope, McpToolResult, OpaqueNonce,
    ScopedName, ToolDescription, ToolError,
};
use super::gateway::Gateway;
use super::taxonomy::TaxonomyCatalog;

// ---------------------------------------------------------------------------
// V2ServerHandler
// ---------------------------------------------------------------------------

/// Bridges `rmcp::ServerHandler` to the v2 `Gateway`. Constructed AFTER
/// successful env-assertion + manifest lookup + keychain cross-check in
/// `main.rs`. Construction failure (any of those checks fail) exits the
/// process before the rmcp service runs.
///
/// rmcp::ServerHandler requires Clone; all fields are Arc-wrapped so
/// cloning is shallow (no transport_key bytes copied per clone).
#[derive(Clone)]
pub struct V2ServerHandler {
    gateway: Arc<Gateway>,
    catalog: Arc<dyn TaxonomyCatalog>,
    conn: Arc<Mutex<Connection>>,
    verified_client_id: McpClientId,
    /// Process-held transport key. Wrapped in `Zeroizing` so the bytes
    /// are wiped on drop. Arc-wrapped so V2ServerHandler::Clone is
    /// shallow (no per-clone copies of the secret).
    transport_key: Arc<Zeroizing<[u8; 32]>>,
    /// Most-recently preissued nonce — caller of the next `tools/call`
    /// must present this as `request_nonce`.
    next_nonce: Arc<Mutex<Option<OpaqueNonce>>>,
}

impl V2ServerHandler {
    /// Construct after env-assertion + keychain cross-check succeed in
    /// main.rs. `initial_nonce` is the seed nonce from `pair_client`
    /// (issued at pairing time, valid until first successful call).
    pub fn from_verified_pairing(
        gateway: Arc<Gateway>,
        catalog: Arc<dyn TaxonomyCatalog>,
        conn: Arc<Mutex<Connection>>,
        verified_client_id: McpClientId,
        transport_key: Zeroizing<[u8; 32]>,
        initial_nonce: OpaqueNonce,
    ) -> Self {
        Self {
            gateway,
            catalog,
            conn,
            verified_client_id,
            transport_key: Arc::new(transport_key),
            next_nonce: Arc::new(Mutex::new(Some(initial_nonce))),
        }
    }
}

/// Build a JSON Schema fragment from a `ToolDescription`'s parameter
/// list. Per L0 AC-6, transport self-signs and `_dailyos_*` keys are
/// NOT advertised — clients send normal handler params only.
fn build_input_schema(desc: &ToolDescription) -> JsonObject {
    let mut properties = serde_json::Map::new();
    let mut required = Vec::new();
    for param in &desc.parameters {
        properties.insert(param.name.clone(), param.schema.0.clone());
        if param.required {
            required.push(serde_json::Value::String(param.name.clone()));
        }
    }
    let mut schema = serde_json::Map::new();
    schema.insert("type".into(), serde_json::Value::String("object".into()));
    schema.insert("properties".into(), serde_json::Value::Object(properties));
    schema.insert("required".into(), serde_json::Value::Array(required));
    schema.insert(
        "additionalProperties".into(),
        serde_json::Value::Bool(false),
    );
    schema
}

fn tool_from_description(desc: &ToolDescription) -> Tool {
    let description = format!(
        "{}\n\nWhen to call:\n{}\n\nWhen NOT to call:\n{}",
        desc.summary.trim(),
        desc.when_to_call.trim(),
        desc.when_not_to_call.trim(),
    );
    Tool {
        name: desc.name.as_str().to_string().into(),
        description: description.into(),
        input_schema: Arc::new(build_input_schema(desc)),
    }
}

// ---------------------------------------------------------------------------
// rmcp::ServerHandler impl
// ---------------------------------------------------------------------------

impl ServerHandler for V2ServerHandler {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "dailyos-mcp-v2".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
            instructions: Some(
                "DailyOS MCP v2. Personal working intelligence for the paired user — \
                 accounts, briefings, attention, working memory, notes, actions. \
                 Not for broad enterprise or web corpus search."
                    .to_string(),
            ),
        }
    }

    async fn list_tools(
        &self,
        _request: PaginatedRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        // Per L0 AC-6: filter by `mcp_tool_grant` rows for
        // `verified_client_id` where exposure = Invocable.
        let grants = {
            let conn_guard = self.conn.lock();
            auth::list_invocable_tool_grants(&conn_guard, &self.verified_client_id).map_err(
                |e| {
                    ErrorData::internal_error(
                        format!("mcp_v2 tool_grant lookup: {e}"),
                        None,
                    )
                },
            )?
        };

        let mut tools = Vec::with_capacity(grants.len());
        for grant_name in &grants {
            if let Some(desc) = self.catalog.description_for(grant_name) {
                tools.push(tool_from_description(desc));
            }
            // Catalog entries without a matching registered handler are
            // silently absent — pending in catalog, not invocable yet.
        }
        Ok(ListToolsResult {
            next_cursor: None,
            tools,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        // Per L0 AC-2: receive normal MCP `{ name, arguments }` shape;
        // transport builds + signs envelope.
        let tool_name = ScopedName::new(request.name.to_string());
        let params = serde_json::Value::Object(request.arguments.unwrap_or_default());

        // Pull the preissued nonce (initial or from prior response).
        let request_nonce = {
            let mut guard = self.next_nonce.lock();
            guard.take().ok_or_else(|| {
                ErrorData::internal_error(
                    "mcp_v2 transport: no preissued nonce; re-pair to recover".to_string(),
                    None,
                )
            })?
        };

        let envelope = McpToolRequestEnvelope {
            request_nonce,
            conversation_handle: None,
            tool_name,
            params,
        };

        // Sign envelope with process-held transport_key. Matches W1-A
        // `auth::verify_transport_hmac` canonicalization (serde_json::to_vec);
        // see auth.rs:verify_transport_hmac.
        //
        // KNOWN (path-α): L0 AC-14 specified RFC 8785 JCS; W1-A shipped
        // serde_json::to_vec. Flipping either side requires flipping both
        // atomically. Filed as separate ticket.
        let canonical = serde_json::to_vec(&envelope).map_err(|e| {
            ErrorData::internal_error(format!("mcp_v2 envelope canonicalize: {e}"), None)
        })?;
        let mac_key = hmac::Key::new(hmac::HMAC_SHA256, self.transport_key.as_ref().as_ref());
        let signature = hmac::sign(&mac_key, &canonical);

        // Call gateway. W1-A substrate verifies HMAC, consumes nonce,
        // dispatches handler, audits, signal-emits.
        let response_envelope = {
            let mut conn_guard = self.conn.lock();
            self.gateway.handle_tool_call(
                &mut conn_guard,
                &self.verified_client_id,
                envelope,
                signature.as_ref(),
            )
        };

        // Store the gateway's preissued next_nonce for the NEXT call.
        // (Even on error responses the gateway returns a preissued nonce
        // so the caller can recover.)
        *self.next_nonce.lock() = Some(response_envelope.next_request_nonce.clone());

        unwrap_response(response_envelope)
    }
}

/// Translate the gateway's response envelope into an rmcp result.
fn unwrap_response(env: McpToolResponseEnvelope) -> Result<CallToolResult, ErrorData> {
    match env.result {
        McpToolResult::Ok { value } => {
            let text = serde_json::to_string(&value).unwrap_or_else(|_| "{}".to_string());
            Ok(CallToolResult::success(vec![Content::text(text)]))
        }
        McpToolResult::Error { error } => Err(tool_error_to_mcp_error(error)),
    }
}

/// Per L0 AC-3 closed-matrix mapping from `ToolError` to `rmcp::ErrorData`.
fn tool_error_to_mcp_error(err: ToolError) -> ErrorData {
    use serde_json::json;
    match err {
        ToolError::Unauthorized { missing_scope } => ErrorData::invalid_request(
            "tool requires scope your pairing lacks".to_string(),
            Some(json!({
                "kind": "unauthorized",
                "missing_scope": missing_scope.as_str(),
            })),
        ),
        ToolError::BadParams { detail: _ } => {
            // detail intentionally NOT included in public payload per AC-3;
            // logged server-side only by the gateway audit path.
            ErrorData::invalid_params(
                "tool params are malformed".to_string(),
                Some(json!({
                    "kind": "bad_params",
                    "detail_logged_server_side": true,
                })),
            )
        }
        ToolError::RateLimited { retry_after_seconds } => ErrorData::new(
            ErrorCode(-32099),
            "too many calls; retry later".to_string(),
            Some(json!({
                "kind": "rate_limited",
                "retry_after_seconds": retry_after_seconds,
            })),
        ),
        ToolError::ExposureForbidden { tool_name } => ErrorData::new(
            ErrorCode::METHOD_NOT_FOUND,
            "tool is not exposed to your pairing".to_string(),
            Some(json!({
                "kind": "exposure_forbidden",
                "tool_name": tool_name.as_str(),
            })),
        ),
        ToolError::PairingRevoked => ErrorData::invalid_request(
            "pairing has been revoked".to_string(),
            Some(json!({ "kind": "pairing_revoked" })),
        ),
        ToolError::ConversationRevoked => ErrorData::invalid_request(
            "conversation has been revoked".to_string(),
            Some(json!({ "kind": "conversation_revoked" })),
        ),
        ToolError::Internal { trace_id } => ErrorData::internal_error(
            "internal error".to_string(),
            Some(json!({
                "kind": "internal",
                "trace_id": trace_id,
            })),
        ),
        ToolError::NotFound { resource } => ErrorData::new(
            ErrorCode::METHOD_NOT_FOUND,
            "requested resource not found".to_string(),
            Some(json!({
                "kind": "not_found",
                // resource is an opaque ID per L0 AC-3 (never PII).
                "resource": resource,
            })),
        ),
        ToolError::UpstreamFailure { detail: _ } => {
            // detail NOT included on wire per AC-3; logged server-side.
            ErrorData::internal_error(
                "upstream system failure; try again later".to_string(),
                Some(json!({
                    "kind": "upstream_failure",
                    "detail_logged_server_side": true,
                })),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::mcp_v2::contracts::{ParamSchema, ParamSpec, ReturnSpec, Scope, Side};

    fn fake_desc(name: &str) -> ToolDescription {
        ToolDescription {
            name: ScopedName::new(name),
            summary: "test summary".into(),
            when_to_call: "test when".into(),
            when_not_to_call: "test when not".into(),
            side: Side::Read,
            parameters: vec![ParamSpec {
                name: "subject".into(),
                schema: ParamSchema(serde_json::json!({ "type": "string" })),
                required: true,
                description: "an account name".into(),
            }],
            returns: ReturnSpec {
                schema: ParamSchema(serde_json::json!({})),
                description: "test return".into(),
            },
            examples: vec![],
            scopes_required: vec![],
        }
    }

    #[test]
    fn input_schema_excludes_dailyos_keys_per_l0_ac6() {
        let desc = fake_desc("dailyos.read.account_status");
        let schema = build_input_schema(&desc);
        let props = schema
            .get("properties")
            .and_then(|v| v.as_object())
            .expect("properties present");
        assert!(props.contains_key("subject"), "handler params present");
        assert!(
            !props.keys().any(|k| k.starts_with("_dailyos_")),
            "no `_dailyos_*` in public input_schema: keys={:?}",
            props.keys().collect::<Vec<_>>(),
        );
        assert_eq!(
            schema.get("additionalProperties"),
            Some(&serde_json::Value::Bool(false)),
        );
    }

    #[test]
    fn tool_description_composes_summary_when_to_call_when_not() {
        let desc = fake_desc("dailyos.read.account_status");
        let tool = tool_from_description(&desc);
        let description = tool.description.to_string();
        assert!(description.contains("test summary"));
        assert!(description.contains("When to call:\ntest when"));
        assert!(description.contains("When NOT to call:\ntest when not"));
    }

    #[test]
    fn tool_error_unauthorized_maps_to_invalid_request_with_kind() {
        let err = tool_error_to_mcp_error(ToolError::Unauthorized {
            missing_scope: Scope::new("dailyos.read.account_status"),
        });
        assert_eq!(err.code, ErrorCode::INVALID_REQUEST);
        let data = err.data.as_ref().expect("data present");
        assert_eq!(data["kind"], "unauthorized");
        assert_eq!(data["missing_scope"], "dailyos.read.account_status");
    }

    #[test]
    fn tool_error_rate_limited_maps_with_retry_after() {
        let err = tool_error_to_mcp_error(ToolError::RateLimited {
            retry_after_seconds: 60,
        });
        assert_eq!(err.code, ErrorCode(-32099));
        let data = err.data.as_ref().expect("data present");
        assert_eq!(data["kind"], "rate_limited");
        assert_eq!(data["retry_after_seconds"], 60);
    }

    #[test]
    fn tool_error_exposure_forbidden_maps_to_method_not_found() {
        let err = tool_error_to_mcp_error(ToolError::ExposureForbidden {
            tool_name: ScopedName::new("dailyos.read.account_status"),
        });
        assert_eq!(err.code, ErrorCode::METHOD_NOT_FOUND);
        let data = err.data.as_ref().expect("data present");
        assert_eq!(data["kind"], "exposure_forbidden");
    }

    #[test]
    fn tool_error_bad_params_does_not_leak_detail() {
        let err = tool_error_to_mcp_error(ToolError::BadParams {
            detail: "secret-server-side-detail".into(),
        });
        let data_str = serde_json::to_string(&err.data).unwrap_or_default();
        assert!(
            !data_str.contains("secret-server-side-detail"),
            "BadParams detail must NOT leak to wire per AC-3: data={data_str}",
        );
        assert!(data_str.contains("detail_logged_server_side"));
    }

    #[test]
    fn tool_error_internal_includes_opaque_trace_id() {
        let err = tool_error_to_mcp_error(ToolError::Internal {
            trace_id: "trace-abc123".into(),
        });
        assert_eq!(err.code, ErrorCode::INTERNAL_ERROR);
        let data = err.data.as_ref().expect("data present");
        assert_eq!(data["kind"], "internal");
        assert_eq!(data["trace_id"], "trace-abc123");
    }
}
