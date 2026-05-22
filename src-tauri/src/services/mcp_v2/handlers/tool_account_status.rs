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

use abilities_runtime::abilities::registry::{AbilityRegistry, McpExposure};
use abilities_runtime::abilities::tracer::NOOP_ABILITY_TRACER;
use serde_json::Value;

use crate::bridges::types::{
    invoke_registry_json_for_actor, AbilityInvokeError, RequestScopedInvocation,
    BRIDGE_NOOP_INTELLIGENCE_PROVIDER,
};
use crate::bridges::{BridgeActor, BridgeSurface};
use crate::services::context::{
    attach_live_workspace_readers, ClaimDismissalSurface, ExternalClients, ServiceContext,
    SystemClock, SystemRng,
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
    runtime: tokio::runtime::Handle,
}

impl AccountStatusHandler {
    pub fn new(
        description: ToolDescription,
        registry: &'static AbilityRegistry,
        runtime: tokio::runtime::Handle,
    ) -> Self {
        Self {
            description,
            registry,
            runtime,
        }
    }

    /// Convenience constructor for `mcp_v2/main.rs::run_serve`. Workspace
    /// readers are attached at invocation time via
    /// `attach_live_workspace_readers` (each reader opens its own ActionDb
    /// from LocalKeychain). Runtime handle is passed in explicitly because
    /// registration runs in the binary's synchronous startup path before
    /// `runtime.block_on(...)` begins.
    pub fn from_runtime(
        description: ToolDescription,
        runtime: tokio::runtime::Handle,
    ) -> Result<Self, &'static str> {
        let registry = AbilityRegistry::global_checked()
            .map_err(|_| "ability registry violations present at startup")?;
        Ok(Self::new(description, registry, runtime))
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
        let runtime_actor =
            project_actor(client_id, &synthetic_grant, conversation_handle.as_ref());

        // The catalog gives us `subject` (string). Pre-read the current
        // composition version for this account_id so the ability's
        // optimistic-concurrency commit doesn't trip StaleComposition.
        // Matches surface_runtime/mod.rs:2545 read-before-invoke pattern.
        let subject = extract_subject(&params)?;
        let composition_id = format!("dailyos/account-overview:account:{subject}");

        self.runtime.block_on(async {
            let clock = SystemClock;
            let rng = SystemRng;
            let external = ExternalClients::default();
            let services = attach_live_workspace_readers(
                ServiceContext::new_live(&clock, &rng, &external).with_actor(ACTOR_LABEL),
            );

            // Pre-read current version. If the composition row does not
            // exist yet, this returns 0; the ability will then commit at
            // version 1.
            let composition_id_for_read = composition_id.clone();
            let current_version = tokio::task::spawn_blocking(move || {
                let db =
                    crate::db::ActionDb::open(std::sync::Arc::new(crate::db::LocalKeychain::new()))
                        .map_err(|err| {
                            eprintln!("mcp_v2 account_status pre-read open_db failed: {err}");
                            AbilityInvokeError::Surface(
                                crate::bridges::BridgeSurfaceError::AbilityUnavailable,
                            )
                        })?;
                let clock = SystemClock;
                let rng = SystemRng;
                let external = ExternalClients::default();
                let ctx = ServiceContext::new_live(&clock, &rng, &external);
                crate::services::compositions::current_composition_version_for_composition_id(
                    &ctx,
                    &db,
                    &composition_id_for_read,
                )
                .map_err(|_| {
                    AbilityInvokeError::Surface(
                        crate::bridges::BridgeSurfaceError::AbilityUnavailable,
                    )
                })
            })
            .await
            .map_err(|_| {
                AbilityInvokeError::Surface(crate::bridges::BridgeSurfaceError::AbilityUnavailable)
            })
            .and_then(|inner| inner)
            .map_err(map_invoke_error)?;

            let resolved_input = serde_json::json!({
                "schema_version": ACCOUNT_OVERVIEW_SCHEMA_VERSION,
                "account_id": subject,
                "expected_composition_version": current_version,
            });

            // Use the request-scoped dispatch path — V2 MCP needs to pass
            // the full `Actor::McpClient { client_id, conversation_handle }`
            // variant through to the registry so account_overview's
            // `allowed_actors = [User, SurfaceClient, McpClient]` matches.
            // The legacy `invoke_registry_json` calls `BridgeActor::Agent
            // .registry_actor()` which returns `Actor::Agent` — not in the
            // allowed list, surface-rejected.
            let invocation = RequestScopedInvocation {
                registry_actor: runtime_actor,
                response_actor: BridgeActor::McpClient,
                surface: BridgeSurface::McpTool,
                claim_dismissal_surface: ClaimDismissalSurface::McpTool,
            };
            let response = invoke_registry_json_for_actor(
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

fn extract_subject(params: &Value) -> Result<String, ToolError> {
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
    Ok(subject.to_string())
}

fn map_invoke_error(err: AbilityInvokeError) -> ToolError {
    // Surface the underlying error to stderr (captured by Claude Desktop's
    // MCP log) so we can diagnose failures without losing detail to the
    // wire-shape trace_id collapse.
    eprintln!("mcp_v2 dailyos.read.account_status invoke failed: {err:?}");

    let trace_id = match &err {
        AbilityInvokeError::Surface(_) => "surface",
        AbilityInvokeError::Ability(_) => "ability",
        AbilityInvokeError::InvalidEnvelope => "invalid_envelope",
        AbilityInvokeError::ProvenanceTooLarge => "provenance_too_large",
        AbilityInvokeError::ProvenanceSerialize(_) => "provenance_serialize",
    };
    ToolError::Internal {
        trace_id: trace_id.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::mcp_v2::contracts::{
        McpClientId, OpaqueConversationHandle, Scope, ScopedName,
    };

    fn stub_actor() -> McpActor {
        McpActor::Client {
            client_id: McpClientId::new("test-client".to_string()),
            conversation_handle: Some(OpaqueConversationHandle::new("conv".to_string())),
            tool_name: ScopedName::new("dailyos.read.account_status"),
            granted_scopes: vec![Scope::new("dailyos.read.account_status")],
        }
    }

    #[test]
    fn extract_subject_returns_trimmed_value() {
        let params = serde_json::json!({ "subject": "  acme  " });
        let subject = extract_subject(&params).unwrap();
        assert_eq!(subject, "acme");
    }

    #[test]
    fn extract_subject_rejects_missing() {
        let params = serde_json::json!({});
        let err = extract_subject(&params).unwrap_err();
        match err {
            ToolError::BadParams { detail } => assert!(detail.contains("subject")),
            other => panic!("expected BadParams, got {other:?}"),
        }
    }

    #[test]
    fn extract_subject_rejects_empty() {
        let params = serde_json::json!({ "subject": "   " });
        let err = extract_subject(&params).unwrap_err();
        assert!(matches!(err, ToolError::BadParams { .. }));
    }

    #[test]
    fn extract_subject_rejects_non_string() {
        let params = serde_json::json!({ "subject": 42 });
        let err = extract_subject(&params).unwrap_err();
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
