//! `dailyos.write.place_document` MCP tool handler.
//!
//! Wraps the claim-producing `workspace_place_document` ability from
//! `abilities-runtime`. The handler stays at the v2 MCP bridge edge:
//! gateway/auth/rate-limit semantics remain owned by `gateway.rs`, while
//! workspace mutation behavior remains owned by `WorkspaceIntakeService`.
//! Sync `McpToolHandler::invoke` is called from within
//! `tokio::task::spawn_blocking` at the transport boundary, so
//! `runtime.block_on(...)` on a captured handle is safe.

use std::sync::Arc;

use abilities_runtime::abilities::registry::{AbilityRegistry, McpExposure};
use abilities_runtime::abilities::tracer::NOOP_ABILITY_TRACER;
use abilities_runtime::services::workspace_intake::WORKSPACE_PLACE_DOCUMENT_TOOL_NAME;
use serde_json::Value;

use crate::bridges::types::{
    invoke_registry_json_for_actor, AbilityInvokeError, BridgeSurfaceError,
    RequestScopedInvocation, BRIDGE_NOOP_INTELLIGENCE_PROVIDER,
};
use crate::bridges::{BridgeActor, BridgeSurface};
use crate::services::context::{
    attach_live_workspace_readers_with_signal_engine, ClaimDismissalSurface, ExternalClients,
    ServiceContext, SystemClock, SystemRng,
};
use crate::services::mcp_v2::actor_policy::{project_actor, ToolGrant, ToolRateLimit};
use crate::services::mcp_v2::contracts::{McpActor, McpToolHandler, ToolDescription, ToolError};
use crate::signals::propagation::PropagationEngine;

const ABILITY_NAME: &str = "workspace_place_document";
const TOOL_NAME: &str = WORKSPACE_PLACE_DOCUMENT_TOOL_NAME;
const ACTOR_LABEL: &str = concat!("agent:dailyos-mcp-v2:", env!("CARGO_PKG_VERSION"));

pub struct PlacementHandler {
    description: ToolDescription,
    registry: &'static AbilityRegistry,
    runtime: tokio::runtime::Handle,
    signal_engine: Arc<PropagationEngine>,
}

impl PlacementHandler {
    pub fn new(
        description: ToolDescription,
        registry: &'static AbilityRegistry,
        runtime: tokio::runtime::Handle,
        signal_engine: Arc<PropagationEngine>,
    ) -> Self {
        Self {
            description,
            registry,
            runtime,
            signal_engine,
        }
    }

    pub fn from_runtime(
        description: ToolDescription,
        runtime: tokio::runtime::Handle,
        signal_engine: Arc<PropagationEngine>,
    ) -> Result<Self, &'static str> {
        let registry = AbilityRegistry::global_checked()
            .map_err(|_| "ability registry violations present at startup")?;
        Ok(Self::new(description, registry, runtime, signal_engine))
    }

    fn invoke_with_services(
        &self,
        actor: &McpActor,
        params: Value,
        services: &ServiceContext<'_>,
    ) -> Result<Value, ToolError> {
        let McpActor::Client {
            client_id,
            conversation_handle,
            tool_name,
            granted_scopes,
        } = actor;

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

        self.runtime.block_on(async {
            let invocation = RequestScopedInvocation {
                registry_actor: runtime_actor,
                response_actor: BridgeActor::McpClient,
                surface: BridgeSurface::McpTool,
                claim_dismissal_surface: ClaimDismissalSurface::McpTool,
                dry_run: false,
                confirmation: None,
                confirmation_store: None,
            };
            let response = invoke_registry_json_for_actor(
                self.registry,
                services,
                &BRIDGE_NOOP_INTELLIGENCE_PROVIDER,
                &NOOP_ABILITY_TRACER,
                invocation,
                ABILITY_NAME,
                params,
            )
            .await
            .map_err(map_invoke_error)?;

            Ok(mcp_safe_response(response.data))
        })
    }
}

impl McpToolHandler for PlacementHandler {
    fn description(&self) -> &ToolDescription {
        &self.description
    }

    fn invoke(&self, actor: &McpActor, params: Value) -> Result<Value, ToolError> {
        let clock = SystemClock;
        let rng = SystemRng;
        let external = ExternalClients::default();
        let services = attach_live_workspace_readers_with_signal_engine(
            ServiceContext::new_live(&clock, &rng, &external).with_actor(ACTOR_LABEL),
            Some(Arc::clone(&self.signal_engine)),
        );
        self.invoke_with_services(actor, params, &services)
    }
}

fn mcp_safe_response(mut value: Value) -> Value {
    if let Value::Object(object) = &mut value {
        object.insert("resolved_path".to_string(), Value::Null);
        object.insert("resolvedPath".to_string(), Value::Null);
    }
    value
}

fn map_invoke_error(err: AbilityInvokeError) -> ToolError {
    eprintln!("mcp_v2 {TOOL_NAME} invoke failed: {err:?}");

    match err {
        AbilityInvokeError::Surface(BridgeSurfaceError::InputSchemaInvalid)
        | AbilityInvokeError::Surface(BridgeSurfaceError::InputReservedField)
        | AbilityInvokeError::Surface(BridgeSurfaceError::Validation(_)) => ToolError::BadParams {
            detail: "workspace placement request failed bridge validation".to_string(),
        },
        AbilityInvokeError::Surface(BridgeSurfaceError::ProducerUnavailable) => {
            ToolError::UpstreamFailure {
                detail: "workspace placement service unavailable".to_string(),
            }
        }
        AbilityInvokeError::Ability(error) => map_placement_ability_error(&error.message),
        AbilityInvokeError::InvalidEnvelope => ToolError::Internal {
            trace_id: "invalid_envelope".to_string(),
        },
        AbilityInvokeError::ProvenanceTooLarge => ToolError::Internal {
            trace_id: "provenance_too_large".to_string(),
        },
        AbilityInvokeError::ProvenanceSerialize(_) => ToolError::Internal {
            trace_id: "provenance_serialize".to_string(),
        },
        AbilityInvokeError::Surface(_) => ToolError::Internal {
            trace_id: "bridge_surface".to_string(),
        },
    }
}

fn map_placement_ability_error(message: &str) -> ToolError {
    let placement_error = serde_json::from_str::<Value>(message).ok();
    let code = placement_error
        .as_ref()
        .and_then(|error| error.get("code"))
        .and_then(Value::as_str)
        .unwrap_or("");

    match code {
        "rate_limited" => ToolError::RateLimited {
            retry_after_seconds: placement_error
                .as_ref()
                .and_then(|error| error.get("retry_after_seconds"))
                .and_then(Value::as_u64)
                .and_then(|seconds| u32::try_from(seconds).ok())
                .unwrap_or(60),
        },
        "target_not_found_or_unauthorized" | "entity_not_routable" => ToolError::NotFound {
            resource: "placement_target".to_string(),
        },
        "invalid_request_shape"
        | "invalid_entity_type"
        | "invalid_entity_id"
        | "invalid_category"
        | "category_not_allowed"
        | "invalid_content_encoding"
        | "invalid_content_type"
        | "content_too_large"
        | "invalid_filename_hint"
        | "invalid_client_dedup_key"
        | "unsupported_schema_version"
        | "idempotency_in_progress"
        | "previous_attempt_failed"
        | "placement_path_rejected" => ToolError::BadParams {
            detail: format!("workspace placement failed with code {code}"),
        },
        "ingestion_failed" => ToolError::UpstreamFailure {
            detail: "workspace placement ingestion failed".to_string(),
        },
        "placement_internal" => ToolError::Internal {
            trace_id: placement_error
                .as_ref()
                .and_then(|error| error.get("trace_id"))
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| "workspace_placement_internal".to_string()),
        },
        _ => ToolError::Internal {
            trace_id: "workspace_placement_ability".to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use abilities_runtime::services::workspace_intake::{
        PlacementError, PlacementErrorCode, PlacementInvocationContext, WorkspaceIntakeError,
        WorkspaceIntakeReceipt, WorkspaceIntakeRequest, WorkspaceIntakeService,
        WorkspacePlaceDocumentReceipt, WorkspacePlaceDocumentRequest,
        WorkspacePlacementMutationCursor,
    };
    use async_trait::async_trait;
    use chrono::Utc;
    use parking_lot::Mutex;
    use rusqlite::{params, Connection};
    use serde_json::json;

    use super::*;
    use crate::abilities::workspace_graph::contracts::{
        WorkspaceGraphInput, WorkspaceGraphPrivacyProfile, WorkspaceGraphReadRequest,
        WorkspaceGraphResponse,
    };
    use crate::db::{ActionDb, DbAccount};
    use crate::services::mcp_v2::contracts::{
        McpClientId, OpaqueConversationHandle, Scope, ScopedName, Side,
    };
    use crate::services::workspace_ingestion::graph::{
        diagnostic_key_for_tests, read_workspace_graph,
    };
    use crate::services::workspace_ingestion::workspace_intake_impl::place_document_after_rate_for_tests;

    struct StubWorkspaceIntake;

    #[async_trait]
    impl WorkspaceIntakeService for StubWorkspaceIntake {
        async fn ingest(
            &self,
            _ctx: &abilities_runtime::abilities::registry::AbilityContext<'_>,
            _request: WorkspaceIntakeRequest,
        ) -> Result<WorkspaceIntakeReceipt, WorkspaceIntakeError> {
            Err(WorkspaceIntakeError::DbError(
                "stub does not implement ingest".to_string(),
            ))
        }

        async fn place_document(
            &self,
            _ctx: &abilities_runtime::abilities::registry::AbilityContext<'_>,
            _invocation: PlacementInvocationContext,
            request: WorkspacePlaceDocumentRequest,
        ) -> Result<WorkspacePlaceDocumentReceipt, PlacementError> {
            Ok(WorkspacePlaceDocumentReceipt {
                schema_version: 1,
                document_handle: Some("document_opaque".to_string()),
                source_handle: Some("source_opaque".to_string()),
                entity_type: request.entity.entity_type,
                entity_id: request.entity.entity_id,
                category: request.category,
                workspace_file_kind: "mcp_placement".to_string(),
                source_asof: Some("2026-05-26T00:00:00.000Z".to_string()),
                lifecycle_state: "ingested".to_string(),
                claim_count_produced: 1,
                idempotent_replay: false,
                dry_run: false,
                resolved_path: Some("Accounts/example/private-note.md".to_string()),
                mutation_cursor: WorkspacePlacementMutationCursor::WorkspacePlacement {
                    document_handle: "document_opaque".to_string(),
                    source_handle: "source_opaque".to_string(),
                    idempotency_id: "placement_opaque".to_string(),
                },
            })
        }
    }

    struct HermeticPlacementIntake {
        conn: Arc<Mutex<Connection>>,
        workspace_root: PathBuf,
        signal_engine: Arc<PropagationEngine>,
    }

    #[async_trait]
    impl WorkspaceIntakeService for HermeticPlacementIntake {
        async fn ingest(
            &self,
            _ctx: &abilities_runtime::abilities::registry::AbilityContext<'_>,
            _request: WorkspaceIntakeRequest,
        ) -> Result<WorkspaceIntakeReceipt, WorkspaceIntakeError> {
            Err(WorkspaceIntakeError::DbError(
                "fixture only implements placement".to_string(),
            ))
        }

        async fn place_document(
            &self,
            ctx: &abilities_runtime::abilities::registry::AbilityContext<'_>,
            invocation: PlacementInvocationContext,
            request: WorkspacePlaceDocumentRequest,
        ) -> Result<WorkspacePlaceDocumentReceipt, PlacementError> {
            let guard = self.conn.lock();
            place_document_after_rate_for_tests(
                ctx.services(),
                ActionDb::from_conn(&guard),
                &self.workspace_root,
                Some(&self.signal_engine),
                &invocation,
                &request,
                "target_opaque",
            )
        }
    }

    fn description() -> ToolDescription {
        ToolDescription {
            name: ScopedName::new(TOOL_NAME),
            summary: "place document".to_string(),
            when_to_call: "when placing a document".to_string(),
            when_not_to_call: "when uploading elsewhere".to_string(),
            side: Side::Write,
            parameters: vec![],
            returns: crate::services::mcp_v2::contracts::ReturnSpec {
                schema: crate::services::mcp_v2::contracts::ParamSchema(json!({})),
                description: "receipt".to_string(),
            },
            examples: vec![],
            scopes_required: vec![Scope::new("write.workspace_place_document")],
        }
    }

    fn actor() -> McpActor {
        McpActor::Client {
            client_id: McpClientId::new("test-client"),
            conversation_handle: Some(OpaqueConversationHandle::new("conv")),
            tool_name: ScopedName::new(TOOL_NAME),
            granted_scopes: vec![Scope::new("write.workspace_place_document")],
        }
    }

    fn placement_params() -> Value {
        json!({
            "schema_version": 1,
            "entity": {
                "entity_type": "account",
                "entity_id": "acct_opaque"
            },
            "content_b64": "aGVsbG8=",
            "content_type": "text/markdown",
            "category": "notes",
            "dry_run": false
        })
    }

    fn placement_claim_params() -> Value {
        json!({
            "schema_version": 1,
            "entity": {
                "entity_type": "account",
                "entity_id": "acct_placement"
            },
            "content_b64": "UGxhY2VtZW50IGdyYXBoIHZhbGlkYXRpb24gbm90ZS4=",
            "content_type": "text/markdown",
            "category": "notes",
            "client_dedup_key": "placement-handler-claim-fixture",
            "dry_run": false
        })
    }

    fn seed_account(conn: &Connection) {
        ActionDb::from_conn(conn)
            .upsert_account(&DbAccount {
                id: "acct_placement".to_string(),
                name: "Placement Account".to_string(),
                tracker_path: Some("Accounts/Placement Account".to_string()),
                updated_at: Utc::now().to_rfc3339(),
                ..Default::default()
            })
            .expect("account seed");
    }

    fn count<P>(conn: &Connection, sql: &str, params: P) -> i64
    where
        P: rusqlite::Params,
    {
        conn.query_row(sql, params, |row| row.get(0))
            .expect("count query")
    }

    #[test]
    fn placement_handler_invokes_ability_and_scrubs_paths() {
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let handler = PlacementHandler::new(
            description(),
            AbilityRegistry::global_checked().expect("registry"),
            runtime.handle().clone(),
            Arc::new(PropagationEngine::new()),
        );
        let clock = SystemClock;
        let rng = SystemRng;
        let external = ExternalClients::default();
        let services = ServiceContext::new_live(&clock, &rng, &external)
            .with_workspace_intake(Arc::new(StubWorkspaceIntake));

        let result = handler
            .invoke_with_services(&actor(), placement_params(), &services)
            .expect("placement succeeds");

        assert_eq!(result["document_handle"], "document_opaque");
        assert_eq!(result["source_handle"], "source_opaque");
        assert_eq!(result["resolved_path"], Value::Null);
        assert_eq!(result["resolvedPath"], Value::Null);
        assert_eq!(
            result["mutation_cursor"]["kind"], "workspace_placement",
            "write responses must expose a mutation cursor for gateway audit"
        );
    }

    #[cfg(unix)]
    #[test]
    fn placement_handler_commits_claim_and_graph_without_path_leak() {
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let conn = Connection::open_in_memory().expect("in-memory db");
        crate::migrations::run_migrations(&conn).expect("migrations");
        seed_account(&conn);
        let conn = Arc::new(Mutex::new(conn));
        let workspace = tempfile::tempdir().expect("workspace");
        let workspace_root = workspace.path().canonicalize().expect("workspace root");
        let signal_engine = Arc::new(PropagationEngine::new());
        let handler = PlacementHandler::new(
            description(),
            AbilityRegistry::global_checked().expect("registry"),
            runtime.handle().clone(),
            Arc::clone(&signal_engine),
        );
        let clock = SystemClock;
        let rng = SystemRng;
        let external = ExternalClients::default();
        let services = ServiceContext::new_live(&clock, &rng, &external).with_workspace_intake(
            Arc::new(HermeticPlacementIntake {
                conn: Arc::clone(&conn),
                workspace_root: workspace_root.clone(),
                signal_engine,
            }),
        );

        let result = handler
            .invoke_with_services(&actor(), placement_claim_params(), &services)
            .expect("placement succeeds");

        assert_eq!(result["entity_type"], "account");
        assert_eq!(result["entity_id"], "acct_placement");
        assert_eq!(result["workspace_file_kind"], "mcp_placement");
        assert_eq!(result["claim_count_produced"], 1);
        assert_eq!(result["lifecycle_state"], "ingested");
        assert_eq!(result["resolved_path"], Value::Null);
        assert_eq!(result["resolvedPath"], Value::Null);
        let idempotency_id = result["mutation_cursor"]["idempotency_id"]
            .as_str()
            .expect("idempotency cursor");
        assert!(idempotency_id.starts_with("placement_"));

        let guard = conn.lock();
        let file_id = guard
            .query_row(
                "SELECT file_id
                 FROM workspace_placement_idempotency
                 WHERE idempotency_id = ?1
                   AND status = 'succeeded'
                   AND run_id IS NOT NULL
                   AND claim_count_produced = 1",
                params![idempotency_id],
                |row| row.get::<_, String>(0),
            )
            .expect("placement idempotency success row");
        assert_eq!(
            count(
                &guard,
                "SELECT COUNT(*)
                 FROM workspace_file_lifecycle
                 WHERE file_id = ?1
                   AND lifecycle_state = 'ingested'
                   AND source_type = 'mcp_placement'",
                params![&file_id],
            ),
            1
        );
        assert_eq!(
            count(
                &guard,
                "SELECT COUNT(*)
                 FROM document_entity_links
                 WHERE file_id = ?1
                   AND entity_type = 'account'
                   AND entity_id = 'acct_placement'
                   AND attribution_source = 'mcp_placement'
                   AND rejected = 0",
                params![&file_id],
            ),
            1
        );

        let source_ref = format!("workspace_file:{file_id}");
        let (data_source, metadata_json, provenance_json, sensitivity): (
            String,
            String,
            String,
            String,
        ) = guard
            .query_row(
                "SELECT data_source, metadata_json, provenance_json, sensitivity
                 FROM intelligence_claims
                 WHERE source_ref = ?1",
                params![&source_ref],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("workspace placement claim");
        assert_eq!(data_source, "workspace_file:mcp_placement");
        assert_eq!(sensitivity, "user_only");
        let metadata: serde_json::Value =
            serde_json::from_str(&metadata_json).expect("metadata json");
        assert_eq!(metadata["producer"], "workspace_ingestion");
        assert_eq!(metadata["workspace_file_id"], file_id);
        assert_eq!(metadata["workspace_file_kind"], "mcp_placement");
        assert_eq!(metadata["resolved_category"], "notes");
        assert!(metadata["ingestion_run_id"]
            .as_str()
            .is_some_and(|value| !value.is_empty()));

        for forbidden in [
            "Placement Account",
            "Placement graph validation note",
            workspace_root.to_string_lossy().as_ref(),
        ] {
            assert!(
                !metadata_json.contains(forbidden),
                "claim metadata leaked raw fixture detail `{forbidden}`"
            );
            assert!(
                !provenance_json.contains(forbidden),
                "claim provenance leaked raw fixture detail `{forbidden}`"
            );
            assert!(
                !result.to_string().contains(forbidden),
                "MCP handler response leaked raw fixture detail `{forbidden}`"
            );
        }

        let graph = read_workspace_graph(
            &guard,
            WorkspaceGraphReadRequest {
                input: WorkspaceGraphInput {
                    schema_version: 1,
                    entity_filter: None,
                    category_filter: None,
                    cursor: None,
                    if_none_match: None,
                    include_entity_names: false,
                    page_size: 50,
                },
                privacy_profile: WorkspaceGraphPrivacyProfile::FirstParty,
            },
            &diagnostic_key_for_tests("workspace-placement-handler-success"),
        )
        .expect("workspace graph read");
        let WorkspaceGraphResponse::Projection(projection) = graph else {
            panic!("expected workspace graph projection");
        };
        assert!(
            projection.audit.gaps.is_empty(),
            "workspace graph audit gaps: {:?}",
            projection.audit.gaps
        );
        let entity = projection
            .projection
            .entities
            .iter()
            .find(|entity| entity.entity_type == "account" && entity.entity_id == "acct_placement")
            .expect("placement graph entity");
        assert_eq!(entity.file_links.len(), 1);
        assert_eq!(entity.claim_summary.total, 1);
    }

    #[test]
    fn mcp_safe_response_scrubs_path_key_variants() {
        let result = mcp_safe_response(json!({
            "document_handle": "document_opaque",
            "resolved_path": "Accounts/example/private-note.md",
            "resolvedPath": "Accounts/example/private-note.md",
            "mutation_cursor": {
                "kind": "workspace_placement",
                "document_handle": "document_opaque",
                "source_handle": "source_opaque",
                "idempotency_id": "placement_opaque"
            }
        }));

        assert_eq!(result["resolved_path"], Value::Null);
        assert_eq!(result["resolvedPath"], Value::Null);
    }

    #[test]
    fn placement_rate_limit_maps_to_tool_error_retry() {
        let error = PlacementError::new(PlacementErrorCode::RateLimited, "rate limited")
            .with_retry_after(42);
        let tool_error = map_placement_ability_error(
            &serde_json::to_string(&error).expect("placement error serializes"),
        );

        assert_eq!(
            tool_error,
            ToolError::RateLimited {
                retry_after_seconds: 42
            }
        );
    }
}
