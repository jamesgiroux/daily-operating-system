//! MCP v2 gateway: the single entry point that receives MCP tool calls.
//!
//! The gateway resolves handler dependencies once per request, validates the
//! granted scope set, dispatches through registered handlers, and returns the
//! stable MCP v2 response envelope.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::services::mcp_v2::contracts::{
    McpActor, McpClientId, McpHandlerContext, McpToolHandler, McpToolRequestEnvelope,
    McpToolResponseEnvelope, McpToolResult, OpaqueConversationHandle, Scope, ScopedName, ToolError,
};
use crate::services::mcp_v2::handlers::tool_account_status::AccountStatusToolHandler;
use crate::state::AppState;

pub struct McpGateway {
    state: Arc<AppState>,
    handlers: BTreeMap<String, Arc<dyn McpToolHandler>>,
}

impl McpGateway {
    pub fn new(state: Arc<AppState>) -> Self {
        Self {
            state,
            handlers: BTreeMap::new(),
        }
    }

    pub fn with_default_handlers(state: Arc<AppState>) -> Self {
        Self::new(state).with_handler(Arc::new(AccountStatusToolHandler::default()))
    }

    pub fn with_handler(mut self, handler: Arc<dyn McpToolHandler>) -> Self {
        self.register_handler(handler);
        self
    }

    pub fn register_handler(&mut self, handler: Arc<dyn McpToolHandler>) {
        self.handlers.insert(
            handler.description().name.as_str().to_string(),
            Arc::clone(&handler),
        );
    }

    pub async fn invoke(
        &self,
        client_id: McpClientId,
        granted_scopes: Vec<Scope>,
        request: McpToolRequestEnvelope,
    ) -> McpToolResponseEnvelope {
        let conversation_handle = request.conversation_handle.clone().unwrap_or_else(|| {
            OpaqueConversationHandle::new(format!("mcp-{}", uuid::Uuid::new_v4()))
        });

        let result = self
            .invoke_result(
                client_id,
                conversation_handle.clone(),
                granted_scopes,
                request,
            )
            .await;

        McpToolResponseEnvelope {
            conversation_handle,
            result: match result {
                Ok(value) => McpToolResult::Ok { value },
                Err(error) => McpToolResult::Error { error },
            },
        }
    }

    async fn invoke_result(
        &self,
        client_id: McpClientId,
        conversation_handle: OpaqueConversationHandle,
        granted_scopes: Vec<Scope>,
        request: McpToolRequestEnvelope,
    ) -> Result<serde_json::Value, ToolError> {
        let handler = self
            .handlers
            .get(request.tool_name.as_str())
            .cloned()
            .ok_or_else(|| ToolError::NotFound {
                resource: format!("tool:{}", request.tool_name.as_str()),
            })?;

        authorize(
            handler.description().scopes_required.as_slice(),
            &granted_scopes,
        )?;

        let actor = McpActor::Client {
            client_id,
            conversation_handle: Some(conversation_handle),
            tool_name: ScopedName::new(request.tool_name.as_str()),
            granted_scopes,
        };
        let ctx = McpHandlerContext::from_state(&self.state).await?;

        handler.invoke(&ctx, &actor, request.params)
    }
}

fn authorize(required: &[Scope], granted: &[Scope]) -> Result<(), ToolError> {
    for required_scope in required {
        if !granted.iter().any(|scope| scope == required_scope) {
            return Err(ToolError::Unauthorized {
                missing_scope: required_scope.clone(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::mcp_v2::contracts::{ParamSchema, ReturnSpec, Side, ToolDescription};
    use serde_json::json;

    struct FixtureHandler {
        description: ToolDescription,
    }

    impl FixtureHandler {
        fn new() -> Self {
            Self {
                description: ToolDescription {
                    name: ScopedName::new("dailyos.read.fixture"),
                    summary: "fixture".to_string(),
                    when_to_call: "fixture".to_string(),
                    when_not_to_call: "fixture".to_string(),
                    side: Side::Read,
                    parameters: Vec::new(),
                    returns: ReturnSpec {
                        schema: ParamSchema(json!({ "type": "object" })),
                        description: "fixture".to_string(),
                    },
                    examples: Vec::new(),
                    scopes_required: vec![Scope::new("dailyos.read.fixture")],
                },
            }
        }
    }

    impl McpToolHandler for FixtureHandler {
        fn description(&self) -> &ToolDescription {
            &self.description
        }

        fn invoke(
            &self,
            _ctx: &McpHandlerContext<'_>,
            _actor: &McpActor,
            _params: serde_json::Value,
        ) -> Result<serde_json::Value, ToolError> {
            Ok(json!({ "ok": true }))
        }
    }

    #[test]
    fn authorize_rejects_missing_scope() {
        let err = authorize(
            &[Scope::new("dailyos.read.fixture")],
            &[Scope::new("other")],
        )
        .unwrap_err();
        assert_eq!(
            err,
            ToolError::Unauthorized {
                missing_scope: Scope::new("dailyos.read.fixture")
            }
        );
    }

    #[test]
    fn gateway_registers_handlers_by_scoped_name() {
        let state = Arc::new(AppState::new());
        let gateway = McpGateway::new(state).with_handler(Arc::new(FixtureHandler::new()));

        assert!(gateway.handlers.contains_key("dailyos.read.fixture"));
    }
}
