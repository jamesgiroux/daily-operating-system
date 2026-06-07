//! MCP v2 transport ingress — rmcp::ServerHandler implementation.
//!
//! Bridges the official `rmcp` crate to the v2 Gateway so standard MCP
//! clients (Claude Desktop, Cursor, custom local SDK) invoke v2 tools
//! through the simplified local MCP v2 authorization contract.
//!
//! Architecture (per `.docs/plans/v1.4.7-w1-foundation/dos-mcp-transport-l0-plan.md`):
//!
//! - **Scope**: local-to-local same-machine only. Trust boundary is
//!   "same user on this machine." No remote MCP transport in v1.4.7.
//! - **Identity**: server-owned at process startup. Local stdio callers cannot
//!   override the MCP client id.
//! - **Per-call flow**: standard MCP clients send normal `{ name, arguments }`
//!   shape; public tool schemas advertise optional `_dailyos.conversationHandle`
//!   so schema-following hosts can echo continuity metadata. The transport strips
//!   that metadata, builds a [`McpToolRequestEnvelope`], and calls
//!   [`Gateway::handle_tool_call`].
//! - **Trust model**: local stdio is inside the OS-user boundary. Authorization
//!   is the server-side manifest + exposure + rate-limit path in the gateway.

use std::collections::BTreeSet;
use std::sync::Arc;

use parking_lot::Mutex;
use rmcp::model::{
    CallToolRequestParam, CallToolResult, Content, ErrorCode, ErrorData, Implementation,
    JsonObject, ListToolsResult, PaginatedRequestParam, ProtocolVersion, ServerCapabilities,
    ServerInfo, Tool,
};
use rmcp::service::RequestContext;
use rmcp::{RoleServer, ServerHandler};
use rusqlite::Connection;

use super::actor_policy::ToolGrant;
use super::auth;
use super::contracts::{
    McpClientId, McpToolRequestEnvelope, McpToolResponseEnvelope, McpToolResult,
    OpaqueConversationHandle, ScopedName, ToolDescription, ToolError,
};
use super::gateway::Gateway;
use super::taxonomy::TaxonomyCatalog;

// ---------------------------------------------------------------------------
// V2ServerHandler
// ---------------------------------------------------------------------------

/// Bridges `rmcp::ServerHandler` to the v2 `Gateway`. Constructed AFTER
/// successful env-assertion + manifest lookup in `main.rs`. Construction
/// failure exits the process before the rmcp service runs.
///
/// rmcp::ServerHandler requires Clone; all fields are Arc-wrapped so
/// cloning is shallow.
#[derive(Clone)]
pub struct V2ServerHandler {
    gateway: Arc<Gateway>,
    catalog: Arc<dyn TaxonomyCatalog>,
    conn: Option<Arc<Mutex<Connection>>>,
    local_stdio_grants: Arc<Vec<ToolGrant>>,
    verified_client_id: McpClientId,
}

impl V2ServerHandler {
    /// Construct after env-assertion + manifest lookup succeeds in `main.rs`.
    pub fn from_verified_pairing(
        gateway: Arc<Gateway>,
        catalog: Arc<dyn TaxonomyCatalog>,
        conn: Arc<Mutex<Connection>>,
        verified_client_id: McpClientId,
    ) -> Self {
        Self {
            gateway,
            catalog,
            conn: Some(conn),
            local_stdio_grants: Arc::new(Vec::new()),
            verified_client_id,
        }
    }

    /// Construct a local stdio handler whose tool exposure is held in memory.
    /// This avoids MCP startup/auth writes racing the app-owned SQLCipher DB.
    pub fn from_local_stdio(
        gateway: Arc<Gateway>,
        catalog: Arc<dyn TaxonomyCatalog>,
        local_stdio_grants: Vec<ToolGrant>,
        verified_client_id: McpClientId,
    ) -> Self {
        Self {
            gateway,
            catalog,
            conn: None,
            local_stdio_grants: Arc::new(local_stdio_grants),
            verified_client_id,
        }
    }
}

/// Build a gateway-owned JSON Schema wrapper around a `ToolDescription`'s
/// handler parameters. `_dailyos` is transport metadata, not handler input:
/// it is advertised so schema-following hosts can echo continuity handles,
/// then stripped before handler/ability validation.
fn build_input_schema(desc: &ToolDescription) -> JsonObject {
    let mut properties = serde_json::Map::new();
    let mut required = Vec::new();
    for param in &desc.parameters {
        properties.insert(param.name.clone(), param.schema.0.clone());
        if param.required {
            required.push(serde_json::Value::String(param.name.clone()));
        }
    }
    properties.insert("_dailyos".into(), dailyos_metadata_schema());
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

fn dailyos_metadata_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "description": "Reserved DailyOS transport metadata. Only echo dailyos.conversationHandle from a prior successful response.",
        "additionalProperties": false,
        "properties": {
            "conversationHandle": {
                "type": "string",
                "minLength": 1,
                "description": "Server-minted DailyOS continuity handle from the previous response envelope."
            }
        },
        "required": ["conversationHandle"]
    })
}

fn tool_from_description(desc: &ToolDescription) -> Tool {
    let description = format!(
        "{}\n\nWhen to call:\n{}\n\nWhen NOT to call:\n{}\n\nResponse envelope:\nSuccessful responses are JSON text with shape {{\"dailyos\":{{\"conversationHandle\":\"...\"}},\"result\":<typed tool result>}}. Echo only the returned handle as arguments._dailyos.conversationHandle on follow-up calls.",
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

fn registered_invocable_tool_names(
    grants: &[ScopedName],
    registered_tools: &BTreeSet<ScopedName>,
) -> Vec<ScopedName> {
    grants
        .iter()
        .filter(|name| registered_tools.contains(*name))
        .cloned()
        .collect()
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
                 account status, source provenance, claim feedback, notes, and actions. \
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
        let grants = if let Some(conn) = self.conn.as_ref() {
            let conn_guard = conn.lock();
            auth::list_invocable_tool_grants(&conn_guard, &self.verified_client_id).map_err(
                |e| ErrorData::internal_error(format!("mcp_v2 tool_grant lookup: {e}"), None),
            )?
        } else {
            self.local_stdio_grants
                .iter()
                .filter(|grant| {
                    matches!(
                        grant.exposure,
                        abilities_runtime::abilities::registry::McpExposure::Invocable
                    )
                })
                .map(|grant| grant.tool_name.clone())
                .collect()
        };

        let registered_tools = self
            .gateway
            .registered_tools()
            .cloned()
            .collect::<BTreeSet<_>>();
        let visible_tools = registered_invocable_tool_names(&grants, &registered_tools);

        let mut tools = Vec::with_capacity(visible_tools.len());
        for grant_name in &visible_tools {
            if let Some(desc) = self.catalog.description_for(grant_name) {
                tools.push(tool_from_description(desc));
            }
            // Catalog entries without a matching registered handler remain
            // absent even if stale local grants exist from an older binary.
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
        // transport builds the envelope consumed by the authorization gateway.
        let tool_name = ScopedName::new(request.name.to_string());
        let mut arguments = request.arguments.unwrap_or_default();
        let conversation_handle = extract_dailyos_metadata(&mut arguments)?;
        let params = serde_json::Value::Object(arguments);

        let envelope = McpToolRequestEnvelope {
            conversation_handle,
            tool_name,
            params,
        };

        // Call gateway. The simplified substrate loads the manifest,
        // dispatches handler, audits, and signal-emits.
        //
        // `handle_tool_call` is sync and the handler chain underneath it calls
        // `runtime.block_on(...)` on a captured tokio handle to dispatch async
        // abilities. Calling that directly from this async context would
        // nested-runtime-panic, so we move dispatch onto a blocking-pool thread
        // via `tokio::task::spawn_blocking`. Inside the blocking thread the
        // handler's `block_on` is safe because we are not on a worker.
        let gateway = self.gateway.clone();
        let client_id = self.verified_client_id.clone();
        let response_envelope = if let Some(conn) = self.conn.clone() {
            tokio::task::spawn_blocking(move || {
                let mut conn_guard = conn.lock();
                gateway.handle_tool_call(&mut conn_guard, &client_id, envelope)
            })
            .await
            .map_err(|join_err| {
                ErrorData::internal_error(
                    format!("mcp_v2 dispatch task join failed: {join_err}"),
                    None,
                )
            })?
        } else {
            let grants = self.local_stdio_grants.clone();
            tokio::task::spawn_blocking(move || {
                gateway.handle_local_stdio_tool_call(&client_id, envelope, grants.as_slice())
            })
            .await
            .map_err(|join_err| {
                ErrorData::internal_error(
                    format!("mcp_v2 dispatch task join failed: {join_err}"),
                    None,
                )
            })?
        };

        unwrap_response(response_envelope)
    }
}

/// Translate the gateway's response envelope into an rmcp result.
fn unwrap_response(env: McpToolResponseEnvelope) -> Result<CallToolResult, ErrorData> {
    match env.result {
        McpToolResult::Ok { value } => {
            reject_reserved_result_metadata(&value)?;
            let response = serde_json::json!({
                "dailyos": {
                    "conversationHandle": env.conversation_handle.as_str(),
                },
                "result": value,
            });
            let text = serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
            Ok(CallToolResult::success(vec![Content::text(text)]))
        }
        McpToolResult::Error { error } => Err(tool_error_to_mcp_error(error)),
    }
}

fn reject_reserved_result_metadata(value: &serde_json::Value) -> Result<(), ErrorData> {
    if contains_reserved_dailyos_key(value) {
        return Err(ErrorData::internal_error(
            "mcp_v2 handler returned reserved _dailyos result field",
            None,
        ));
    }
    Ok(())
}

fn contains_reserved_dailyos_key(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Object(object) => object
            .iter()
            .any(|(key, nested)| key == "_dailyos" || contains_reserved_dailyos_key(nested)),
        serde_json::Value::Array(items) => items.iter().any(contains_reserved_dailyos_key),
        _ => false,
    }
}

fn extract_dailyos_metadata(
    arguments: &mut JsonObject,
) -> Result<Option<OpaqueConversationHandle>, ErrorData> {
    let Some(metadata) = arguments.remove("_dailyos") else {
        return Ok(None);
    };
    let Some(metadata) = metadata.as_object() else {
        return Err(dailyos_metadata_error("metadata_object_required"));
    };
    if metadata.len() != 1 || !metadata.contains_key("conversationHandle") {
        return Err(dailyos_metadata_error("unsupported_metadata_fields"));
    }
    let Some(handle) = metadata
        .get("conversationHandle")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|handle| !handle.is_empty())
    else {
        return Err(dailyos_metadata_error("conversation_handle_required"));
    };
    Ok(Some(OpaqueConversationHandle::new(handle.to_string())))
}

fn dailyos_metadata_error(reason: &str) -> ErrorData {
    ErrorData::invalid_params(
        "DailyOS metadata is malformed".to_string(),
        Some(serde_json::json!({
            "kind": "bad_dailyos_metadata",
            "reason": reason,
        })),
    )
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
        ToolError::RateLimited {
            retry_after_seconds,
        } => ErrorData::new(
            ErrorCode(-32099),
            "too many calls; retry later".to_string(),
            Some(json!({
                "kind": "rate_limited",
                "retry_after_seconds": retry_after_seconds,
            })),
        ),
        ToolError::ExposureForbidden { tool_name: _ } => ErrorData::new(
            ErrorCode::METHOD_NOT_FOUND,
            "tool is not exposed to your pairing".to_string(),
            Some(json!({
                "kind": "exposure_forbidden",
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
    use crate::services::mcp_v2::actor_policy::ToolRateLimit;
    use crate::services::mcp_v2::contracts::{ParamSchema, ParamSpec, ReturnSpec, Scope, Side};
    use crate::services::mcp_v2::handlers::tool_account_status::present_account_status_response;
    use crate::services::mcp_v2::local_runtime::LocalConversationStore;
    use crate::services::mcp_v2::taxonomy::TaxonomyError;
    use abilities_runtime::abilities::registry::McpExposure;
    use serde_json::json;
    use std::borrow::Cow;

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

    #[derive(Clone)]
    struct SingleToolCatalog {
        description: ToolDescription,
    }

    impl TaxonomyCatalog for SingleToolCatalog {
        fn validate_against_handlers(
            &self,
            _handlers: &[&dyn crate::services::mcp_v2::contracts::McpToolHandler],
        ) -> Result<(), TaxonomyError> {
            Ok(())
        }

        fn validate_catalog_against_handlers(
            &self,
            _handlers: &[&dyn crate::services::mcp_v2::contracts::McpToolHandler],
        ) -> Vec<ScopedName> {
            Vec::new()
        }

        fn side_for(&self, tool_name: &ScopedName) -> Option<Side> {
            (tool_name == &self.description.name).then_some(self.description.side)
        }

        fn description_for(&self, tool_name: &ScopedName) -> Option<&ToolDescription> {
            (tool_name == &self.description.name).then_some(&self.description)
        }

        fn iter_names(&self) -> Box<dyn Iterator<Item = &ScopedName> + '_> {
            Box::new(std::iter::once(&self.description.name))
        }
    }

    struct RecordingHandler {
        description: ToolDescription,
        invocations: Arc<Mutex<Vec<serde_json::Value>>>,
    }

    impl crate::services::mcp_v2::contracts::McpToolHandler for RecordingHandler {
        fn description(&self) -> &ToolDescription {
            &self.description
        }

        fn invoke(
            &self,
            _ctx: &crate::services::mcp_v2::handler_context::McpHandlerContext,
            actor: &crate::services::mcp_v2::contracts::McpActor,
            params: serde_json::Value,
        ) -> Result<serde_json::Value, ToolError> {
            self.invocations.lock().push(params);
            let crate::services::mcp_v2::contracts::McpActor::Client {
                conversation_handle,
                ..
            } = actor;
            Ok(json!({
                "status": "ok",
                "actorConversationHandle": conversation_handle.as_ref().map(|handle| handle.as_str()),
            }))
        }
    }

    fn local_stdio_grant(tool_name: &ScopedName) -> ToolGrant {
        ToolGrant {
            tool_name: tool_name.clone(),
            scopes_granted: Vec::new(),
            exposure: McpExposure::Invocable,
            rate_limit: ToolRateLimit {
                max_calls: 600,
                window_seconds: 60,
            },
        }
    }

    fn request_context() -> RequestContext<RoleServer> {
        let (peer, _rx) = rmcp::service::Peer::<RoleServer>::new(
            Arc::new(rmcp::service::AtomicU32RequestIdProvider::default()),
            rmcp::model::ClientInfo::default(),
        );
        RequestContext {
            ct: Default::default(),
            id: rmcp::model::RequestId::Number(1),
            peer,
        }
    }

    fn assert_schema_accepts(schema: &JsonObject, value: &serde_json::Value) {
        validate_schema_subset(&serde_json::Value::Object(schema.clone()), value)
            .expect("schema accepts value");
    }

    fn assert_schema_rejects(schema: &JsonObject, value: &serde_json::Value) {
        validate_schema_subset(&serde_json::Value::Object(schema.clone()), value)
            .expect_err("schema rejects value");
    }

    fn validate_schema_subset(
        schema: &serde_json::Value,
        value: &serde_json::Value,
    ) -> Result<(), String> {
        match schema.get("type").and_then(serde_json::Value::as_str) {
            Some("object") => validate_object_schema_subset(schema, value),
            Some("string") => validate_string_schema_subset(schema, value),
            Some(other) => Err(format!("unsupported schema type: {other}")),
            None => Ok(()),
        }
    }

    fn validate_object_schema_subset(
        schema: &serde_json::Value,
        value: &serde_json::Value,
    ) -> Result<(), String> {
        let object = value
            .as_object()
            .ok_or_else(|| "expected object value".to_string())?;
        let properties = schema
            .get("properties")
            .and_then(serde_json::Value::as_object)
            .ok_or_else(|| "object schema missing properties".to_string())?;

        for required in schema
            .get("required")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
        {
            if !object.contains_key(required) {
                return Err(format!("missing required property: {required}"));
            }
        }

        if schema
            .get("additionalProperties")
            .and_then(serde_json::Value::as_bool)
            == Some(false)
        {
            for key in object.keys() {
                if !properties.contains_key(key) {
                    return Err(format!("unexpected property: {key}"));
                }
            }
        }

        for (key, nested_value) in object {
            if let Some(nested_schema) = properties.get(key) {
                validate_schema_subset(nested_schema, nested_value)?;
            }
        }
        Ok(())
    }

    fn validate_string_schema_subset(
        schema: &serde_json::Value,
        value: &serde_json::Value,
    ) -> Result<(), String> {
        let text = value
            .as_str()
            .ok_or_else(|| "expected string value".to_string())?;
        if let Some(min_length) = schema.get("minLength").and_then(serde_json::Value::as_u64) {
            if text.len() < min_length as usize {
                return Err(format!("string shorter than minLength {min_length}"));
            }
        }
        Ok(())
    }

    async fn call_tool_json(
        handler: &V2ServerHandler,
        name: &'static str,
        arguments: serde_json::Value,
    ) -> serde_json::Value {
        let arguments = arguments
            .as_object()
            .cloned()
            .expect("test arguments are object");
        let result = handler
            .call_tool(
                CallToolRequestParam {
                    name: Cow::Borrowed(name),
                    arguments: Some(arguments),
                },
                request_context(),
            )
            .await
            .expect("call_tool succeeds");

        serde_json::from_str(result.content[0].as_text().unwrap().text.as_str())
            .expect("response content is JSON text")
    }

    #[test]
    fn input_schema_advertises_reserved_dailyos_conversation_handle() {
        let desc = fake_desc("dailyos.read.account_status");
        let schema = build_input_schema(&desc);
        let props = schema
            .get("properties")
            .and_then(|v| v.as_object())
            .expect("properties present");
        assert!(props.contains_key("subject"), "handler params present");
        let dailyos = props
            .get("_dailyos")
            .and_then(|value| value.as_object())
            .expect("reserved metadata schema present");
        assert_eq!(dailyos.get("additionalProperties"), Some(&json!(false)));
        let dailyos_props = dailyos
            .get("properties")
            .and_then(|value| value.as_object())
            .expect("metadata properties present");
        assert!(dailyos_props.contains_key("conversationHandle"));
        assert_eq!(
            dailyos.get("required"),
            Some(&json!(["conversationHandle"]))
        );
        assert_eq!(
            schema.get("additionalProperties"),
            Some(&serde_json::Value::Bool(false)),
        );
    }

    #[tokio::test]
    async fn tools_list_schema_validates_echo_then_call_tool_strips_metadata() {
        let tool_name = "dailyos.read.account_status";
        let description = fake_desc(tool_name);
        let invocations = Arc::new(Mutex::new(Vec::new()));
        let conversation_dir = tempfile::tempdir().expect("conversation tempdir");
        let mut gateway = Gateway::new().with_local_conversation_store_for_tests(
            LocalConversationStore::new(conversation_dir.path().join("handles.json")),
        );
        gateway.register(Arc::new(RecordingHandler {
            description: description.clone(),
            invocations: invocations.clone(),
        }));
        let handler = V2ServerHandler::from_local_stdio(
            Arc::new(gateway),
            Arc::new(SingleToolCatalog {
                description: description.clone(),
            }),
            vec![local_stdio_grant(&description.name)],
            McpClientId::new("local-client-a"),
        );

        let tools = handler
            .list_tools(None, request_context())
            .await
            .expect("tools/list succeeds");
        let tool = tools
            .tools
            .iter()
            .find(|tool| tool.name.as_ref() == tool_name)
            .expect("registered tool listed");
        let schema = tool.input_schema.as_ref();

        let first_arguments = json!({ "subject": "Example Account" });
        assert_schema_accepts(schema, &first_arguments);
        assert_schema_rejects(
            schema,
            &json!({
                "subject": "Example Account",
                "conversation_id": "caller-controlled"
            }),
        );
        let first_payload = call_tool_json(&handler, tool_name, first_arguments).await;
        let minted_handle = first_payload["dailyos"]["conversationHandle"]
            .as_str()
            .expect("first response mints handle")
            .to_string();

        let follow_up_arguments = json!({
            "subject": "Example Account",
            "_dailyos": { "conversationHandle": minted_handle }
        });
        assert_schema_accepts(schema, &follow_up_arguments);
        assert_schema_rejects(
            schema,
            &json!({
                "subject": "Example Account",
                "_dailyos": {
                    "conversationHandle": first_payload["dailyos"]["conversationHandle"],
                    "clientId": "caller-controlled"
                }
            }),
        );
        let second_payload = call_tool_json(&handler, tool_name, follow_up_arguments).await;

        assert_eq!(
            second_payload["dailyos"]["conversationHandle"],
            first_payload["dailyos"]["conversationHandle"]
        );
        assert_eq!(
            second_payload["result"]["actorConversationHandle"],
            first_payload["dailyos"]["conversationHandle"]
        );
        assert_eq!(
            *invocations.lock(),
            vec![
                json!({ "subject": "Example Account" }),
                json!({ "subject": "Example Account" }),
            ],
            "`_dailyos` must be stripped before handler/ability params"
        );
    }

    #[tokio::test]
    async fn call_tool_exposure_error_does_not_echo_hidden_requested_name() {
        let visible_tool_name = "dailyos.read.account_status";
        let hidden_tool_name = "dailyos.read.daily_briefing";
        let description = fake_desc(visible_tool_name);
        let invocations = Arc::new(Mutex::new(Vec::new()));
        let conversation_dir = tempfile::tempdir().expect("conversation tempdir");
        let mut gateway = Gateway::new().with_local_conversation_store_for_tests(
            LocalConversationStore::new(conversation_dir.path().join("handles.json")),
        );
        gateway.register(Arc::new(RecordingHandler {
            description: description.clone(),
            invocations,
        }));
        let handler = V2ServerHandler::from_local_stdio(
            Arc::new(gateway),
            Arc::new(SingleToolCatalog {
                description: description.clone(),
            }),
            vec![local_stdio_grant(&description.name)],
            McpClientId::new("local-client-a"),
        );

        let err = handler
            .call_tool(
                CallToolRequestParam {
                    name: Cow::Borrowed(hidden_tool_name),
                    arguments: Some(JsonObject::new()),
                },
                request_context(),
            )
            .await
            .expect_err("hidden tool call must be rejected");
        assert_eq!(err.code, ErrorCode::METHOD_NOT_FOUND);
        let wire = serde_json::to_string(&err.data).unwrap_or_default();
        assert!(
            !wire.contains(hidden_tool_name) && !err.message.contains(hidden_tool_name),
            "hidden tool name must not be echoed on the public wire error: data={wire}"
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
        assert!(description.contains("Response envelope:"));
        assert!(description.contains("arguments._dailyos.conversationHandle"));
    }

    #[test]
    fn mcp_v2_tools_list_registered_handlers_only() {
        let grants = vec![
            ScopedName::new("dailyos.read.account_status"),
            ScopedName::new("dailyos.read.daily_briefing"),
        ];
        let registered_tools = [ScopedName::new("dailyos.read.account_status")]
            .into_iter()
            .collect::<BTreeSet<_>>();

        let visible = registered_invocable_tool_names(&grants, &registered_tools);

        assert_eq!(
            visible,
            vec![ScopedName::new("dailyos.read.account_status")]
        );
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
        assert!(
            data.get("tool_name").is_none(),
            "exposure errors must not echo probed tool names"
        );
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

    #[test]
    fn extract_dailyos_metadata_strips_conversation_handle() {
        let mut arguments = serde_json::json!({
            "subject": "Example Account",
            "_dailyos": {
                "conversationHandle": "local-stdio-abc"
            }
        })
        .as_object()
        .cloned()
        .unwrap();

        let handle = extract_dailyos_metadata(&mut arguments)
            .expect("metadata valid")
            .expect("handle present");

        assert_eq!(handle.as_str(), "local-stdio-abc");
        assert!(!arguments.contains_key("_dailyos"));
        assert_eq!(arguments["subject"], "Example Account");
    }

    #[test]
    fn extract_dailyos_metadata_rejects_actor_assertions() {
        let mut arguments = serde_json::json!({
            "_dailyos": {
                "conversationHandle": "local-stdio-abc",
                "clientId": "caller-controlled"
            }
        })
        .as_object()
        .cloned()
        .unwrap();

        let err = extract_dailyos_metadata(&mut arguments).expect_err("metadata rejected");
        let data = err.data.as_ref().expect("error data");
        assert_eq!(data["kind"], "bad_dailyos_metadata");
        assert_eq!(data["reason"], "unsupported_metadata_fields");
    }

    #[test]
    fn unwrap_response_preserves_account_status_projection_envelope() {
        let account_status = present_account_status_response(
            "account_01",
            json!({
                "schemaVersion": 2,
                "subject": {
                    "kind": "account",
                    "id": "account_01",
                    "displayLabel": "Account 01"
                },
                "facts": { "items": [{
                    "claimId": "claim_01",
                    "text": "Account 01 has a current onboarding plan.",
                    "claimType": "account_status",
                    "trustBand": "likely_current",
                    "sensitivity": "internal",
                    "provenance": { "sourceIds": ["source_01"] }
                }] },
                "openLoops": { "items": [] },
                "relationships": { "items": [] },
                "touchpoints": { "items": [] },
                "recordEntries": { "items": [] },
                "trust": {
                    "aggregateBand": "likely_current",
                    "sectionCaveats": {}
                },
                "sensitivity": "internal",
                "provenance": {
                    "sources": [{
                        "id": "source_01",
                        "label": "Workspace note",
                        "sourceType": "workspace_file",
                        "redacted": false
                    }],
                    "redactionApplied": false
                },
                "sections": {}
            }),
        );

        let result = unwrap_response(McpToolResponseEnvelope {
            conversation_handle: OpaqueConversationHandle::new("local-stdio-abc"),
            result: McpToolResult::Ok {
                value: account_status,
            },
        })
        .expect("success result");

        let payload: serde_json::Value =
            serde_json::from_str(result.content[0].as_text().unwrap().text.as_str()).unwrap();
        assert_eq!(payload["dailyos"]["conversationHandle"], "local-stdio-abc");
        assert_eq!(payload["result"]["toolName"], "dailyos.read.account_status");
        assert_eq!(
            payload["result"]["trust"]["aggregateBand"],
            "likely_current"
        );
        assert_eq!(payload["result"]["sensitivity"], "internal");
        assert_eq!(
            payload["result"]["provenance"]["rawClaimIdsIncluded"],
            false
        );
        assert!(payload["result"].get("_dailyos").is_none());
    }

    #[test]
    fn unwrap_response_rejects_reserved_dailyos_result_metadata() {
        let err = unwrap_response(McpToolResponseEnvelope {
            conversation_handle: OpaqueConversationHandle::new("local-stdio-abc"),
            result: McpToolResult::Ok {
                value: serde_json::json!({
                    "status": "active",
                    "nested": [{
                        "_dailyos": { "conversationHandle": "forged" }
                    }]
                }),
            },
        })
        .expect_err("reserved result metadata rejected");

        assert_eq!(err.code, ErrorCode::INTERNAL_ERROR);
    }
}
