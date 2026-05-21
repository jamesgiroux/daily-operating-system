//! `dailyos.read.account_status` MCP tool handler.
//!
//! Wraps the `dailyos/account-overview` ability from `abilities-runtime`,
//! dispatched via the same `invoke_registry_json` path the Tauri UI uses.
//! Per DOS-175 cycle-2 §3: sync `McpToolHandler::invoke` is called from
//! within `tokio::task::spawn_blocking` at the transport boundary, so
//! `runtime.block_on(...)` on a captured handle is safe.
//!
//! See `.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md` for the L0
//! contract.

use std::sync::Arc;

use abilities_runtime::abilities::registry::{AbilityRegistry, McpExposure};
use parking_lot::Mutex as ParkingMutex;
use serde_json::Value;

use abilities_runtime::abilities::tracer::NOOP_ABILITY_TRACER;

use crate::bridges::mcp::McpWorkspaceReaders;
use crate::bridges::types::{
    invoke_registry_json, AbilityInvokeError, BRIDGE_NOOP_INTELLIGENCE_PROVIDER,
};
use crate::bridges::{BridgeActor, BridgeSurface, InvocationContext};
use crate::db::ActionDb;
use crate::services::context::{
    ClaimDismissalSurface, ExecutionMode, ExternalClients, ServiceContext, SystemClock, SystemRng,
};
use crate::services::mcp_v2::actor_policy::{project_actor, ToolGrant, ToolRateLimit};
use crate::services::mcp_v2::contracts::{McpActor, McpToolHandler, ToolDescription, ToolError};

const ACTOR_LABEL: &str = concat!("agent:dailyos-mcp-v2:", env!("CARGO_PKG_VERSION"));

/// Registered ability name in the abilities-runtime registry.
/// Per `account_overview.rs:84` the registered name uses `/` (not `_`).
const ABILITY_NAME: &str = "dailyos/account-overview";

/// `AccountOverviewInput` schema version pinned by `account_overview.rs:39`.
const ACCOUNT_OVERVIEW_SCHEMA_VERSION: u32 = 1;

pub struct AccountStatusHandler {
    description: ToolDescription,
    registry: &'static AbilityRegistry,
    workspace_readers: McpWorkspaceReaders,
    runtime: tokio::runtime::Handle,
}

impl AccountStatusHandler {
    pub fn new(
        description: ToolDescription,
        registry: &'static AbilityRegistry,
        workspace_readers: McpWorkspaceReaders,
        runtime: tokio::runtime::Handle,
    ) -> Self {
        Self {
            description,
            registry,
            workspace_readers,
            runtime,
        }
    }

    /// Convenience constructor for `mcp_v2/main.rs::run_serve`. Builds
    /// workspace readers from the live action DB; runtime handle is
    /// passed in explicitly because registration runs in the binary's
    /// synchronous startup path before `runtime.block_on(...)` begins.
    pub fn from_runtime(
        description: ToolDescription,
        db: Arc<ParkingMutex<ActionDb>>,
        runtime: tokio::runtime::Handle,
    ) -> Result<Self, &'static str> {
        let registry = AbilityRegistry::global_checked()
            .map_err(|_| "ability registry violations present at startup")?;
        let workspace_readers = McpWorkspaceReaders::from_action_db(db);
        Ok(Self::new(description, registry, workspace_readers, runtime))
    }
}

impl McpToolHandler for AccountStatusHandler {
    fn description(&self) -> &ToolDescription {
        &self.description
    }

    fn invoke(&self, actor: &McpActor, params: Value) -> Result<Value, ToolError> {
        let McpActor::Client {
            client_id,
            conversation_handle,
            tool_name,
            granted_scopes,
        } = actor;

        // Synthesize a ToolGrant from the already-resolved McpActor scopes.
        // The gateway validated grant before dispatch; this is purely for
        // the ADR-0102 §B "manifest resolved before runtime actor
        // construction" projection contract that project_actor preserves.
        let synthetic_grant = ToolGrant {
            tool_name: tool_name.clone(),
            scopes_granted: granted_scopes.clone(),
            exposure: McpExposure::Invocable,
            rate_limit: ToolRateLimit {
                max_calls: 0,
                window_seconds: 0,
            },
        };
        let _runtime_actor =
            project_actor(client_id, &synthetic_grant, conversation_handle.as_ref());

        let resolved_input = build_account_overview_input(&params)?;

        self.runtime.block_on(async {
            let clock = SystemClock;
            let rng = SystemRng;
            let external = ExternalClients::default();
            let services = self.workspace_readers.attach_to(
                ServiceContext::new_live(&clock, &rng, &external).with_actor(ACTOR_LABEL),
            );
            let invocation = InvocationContext {
                actor: BridgeActor::Agent,
                mode: ExecutionMode::Live,
                surface: BridgeSurface::McpTool,
                claim_dismissal_surface: ClaimDismissalSurface::McpTool,
                dry_run: false,
                confirmation: None,
                confirmation_store: None,
            };
            let response = invoke_registry_json(
                self.registry,
                &services,
                &BRIDGE_NOOP_INTELLIGENCE_PROVIDER,
                &NOOP_ABILITY_TRACER,
                invocation,
                ABILITY_NAME,
                resolved_input,
            )
            .await
            .map_err(map_invoke_error)?;
            Ok(response.data)
        })
    }
}

fn build_account_overview_input(params: &Value) -> Result<Value, ToolError> {
    let subject = params
        .get("subject")
        .and_then(Value::as_str)
        .ok_or_else(|| ToolError::BadParams {
            detail: "missing 'subject' parameter (expected non-empty string)".into(),
        })?
        .trim();
    if subject.is_empty() {
        return Err(ToolError::BadParams {
            detail: "'subject' must be a non-empty string".into(),
        });
    }
    // Cycle-1 passthrough: treat `subject` as the account_id directly per
    // DOS-175 §3.4. Phase-A.1 sub-ticket replaces this with a real
    // subject-to-account_id resolver.
    Ok(serde_json::json!({
        "schema_version": ACCOUNT_OVERVIEW_SCHEMA_VERSION,
        "account_id": subject,
    }))
}

fn map_invoke_error(err: AbilityInvokeError) -> ToolError {
    // For W2-A cycle-1 we map non-fatal ability errors uniformly to
    // ToolError::Internal with a synthesized trace id. Phase-B refines
    // the mapping (e.g., NotFound for missing account, BadParams for
    // input validation surfaced from the ability's normalize step).
    let trace_id = match &err {
        AbilityInvokeError::Surface(_) => "surface",
        AbilityInvokeError::Ability(_) => "ability",
        AbilityInvokeError::InvalidEnvelope => "invalid_envelope",
        AbilityInvokeError::ProvenanceTooLarge => "provenance_too_large",
        AbilityInvokeError::ProvenanceSerialize(_) => "provenance_serialize",
    };
    log::warn!(
        target: "dailyos_lib::services::mcp_v2::handlers::account_status",
        "ability invoke failed: {err:?}"
    );
    let _ = (err, trace_id);
    ToolError::Internal {
        trace_id: trace_id.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::mcp_v2::contracts::{McpClientId, OpaqueConversationHandle, Scope, ScopedName};

    fn stub_actor() -> McpActor {
        McpActor::Client {
            client_id: McpClientId::new("test-client".to_string()),
            conversation_handle: Some(OpaqueConversationHandle::new("conv".to_string())),
            tool_name: ScopedName::new("dailyos.read.account_status"),
            granted_scopes: vec![Scope::new("dailyos.read.account_status")],
        }
    }

    #[test]
    fn build_input_translates_subject_to_account_id() {
        let params = serde_json::json!({ "subject": "acme" });
        let out = build_account_overview_input(&params).unwrap();
        assert_eq!(out["schema_version"], serde_json::json!(1));
        assert_eq!(out["account_id"], serde_json::json!("acme"));
    }

    #[test]
    fn build_input_trims_whitespace() {
        let params = serde_json::json!({ "subject": "  acme  " });
        let out = build_account_overview_input(&params).unwrap();
        assert_eq!(out["account_id"], serde_json::json!("acme"));
    }

    #[test]
    fn build_input_rejects_missing_subject() {
        let params = serde_json::json!({});
        let err = build_account_overview_input(&params).unwrap_err();
        match err {
            ToolError::BadParams { detail } => assert!(detail.contains("subject")),
            other => panic!("expected BadParams, got {other:?}"),
        }
    }

    #[test]
    fn build_input_rejects_empty_subject() {
        let params = serde_json::json!({ "subject": "   " });
        let err = build_account_overview_input(&params).unwrap_err();
        assert!(matches!(err, ToolError::BadParams { .. }));
    }

    #[test]
    fn build_input_rejects_non_string_subject() {
        let params = serde_json::json!({ "subject": 42 });
        let err = build_account_overview_input(&params).unwrap_err();
        assert!(matches!(err, ToolError::BadParams { .. }));
    }

    #[test]
    fn stub_actor_smoke() {
        // Ensures the actor projection compiles against the current
        // McpActor enum shape (a tripwire if the variant fields change).
        let actor = stub_actor();
        match &actor {
            McpActor::Client {
                client_id,
                tool_name,
                granted_scopes,
                ..
            } => {
                assert_eq!(client_id.as_str(), "test-client");
                assert_eq!(tool_name.as_str(), "dailyos.read.account_status");
                assert_eq!(granted_scopes.len(), 1);
            }
        }
    }
}
