//! `workspace_graph` read ability.

pub mod contracts;
pub mod producer;

use dailyos_abilities_macro::ability;

use crate::abilities::workspace_graph::contracts::{WorkspaceGraphInput, WorkspaceGraphResponse};
use crate::abilities::{AbilityContext, AbilityResult};

pub const ABILITY_NAME: &str = "workspace_graph";
pub const ABILITY_SCHEMA_VERSION: u32 = 1;

#[ability(
    name = "workspace_graph",
    category = Read,
    version = "1.0.0",
    schema_version = 1,
    allowed_actors = [User, Agent, System, SurfaceClient],
    allowed_modes = [Live, Simulate, Evaluate],
    requires_confirmation = false,
    may_publish = false,
    required_scopes = ["read.workspace_graph"],
    mcp_exposure = None,
    composes = [],
    experimental = false,
    signal_policy = { emits_on_output_change = [], coalesce = false }
)]
pub async fn workspace_graph(
    ctx: &AbilityContext<'_>,
    input: WorkspaceGraphInput,
) -> AbilityResult<WorkspaceGraphResponse> {
    producer::workspace_graph(ctx, input).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use serde_json::json;

    use crate::abilities::registry::{
        ActorKind, McpExposure, ScopeSet, SurfaceClientId, SurfaceScope,
    };
    use crate::abilities::workspace_graph::contracts::{
        WorkspaceGraphAudit, WorkspaceGraphPage, WorkspaceGraphPrivacyProfile,
        WorkspaceGraphProjection, WorkspaceGraphProjectionBody, WorkspaceGraphReadRequest,
        WorkspaceGraphResponse,
    };
    use crate::abilities::AbilityRegistry;
    use crate::abilities::{Actor, NOOP_ABILITY_TRACER};
    use crate::intelligence::provider::ReplayProvider;
    use crate::sensitivity::ClaimDismissalSurface;
    use crate::services::context::{
        FixedClock, SeedableRng, ServiceContext, WorkspaceGraphReadFuture, WorkspaceGraphReadHandle,
    };

    fn registered_descriptor() -> &'static crate::abilities::AbilityDescriptor {
        let registry = AbilityRegistry::global_checked().expect("registry");
        registry
            .iter_all()
            .find(|d| d.name == ABILITY_NAME)
            .expect("workspace_graph ability is registered")
    }

    #[test]
    fn descriptor_is_runtime_surface_only() {
        let descriptor = registered_descriptor();
        assert_eq!(descriptor.name, ABILITY_NAME);
        assert!(descriptor.policy.allowed_actors.contains(&ActorKind::User));
        assert!(descriptor.policy.allowed_actors.contains(&ActorKind::Agent));
        assert!(descriptor
            .policy
            .allowed_actors
            .contains(&ActorKind::System));
        assert!(descriptor
            .policy
            .allowed_actors
            .contains(&ActorKind::SurfaceClient));
        assert!(!descriptor
            .policy
            .allowed_actors
            .contains(&ActorKind::McpClient));
        assert_eq!(descriptor.policy.required_scopes, &["read.workspace_graph"]);
        assert_eq!(descriptor.policy.mcp_exposure, McpExposure::None);
    }

    #[tokio::test]
    async fn surface_client_without_workspace_graph_scope_is_denied_before_reader() {
        ScopeSet::set_allowlist_for_tests([
            SurfaceScope::new("read.workspace_graph"),
            SurfaceScope::new("read.entity_names"),
            SurfaceScope::new("read.account_overview"),
            SurfaceScope::new("read.composition"),
            SurfaceScope::new("submit.feedback"),
        ]);
        let scopes = ScopeSet::new([SurfaceScope::new("read.entity_names")]).expect("scope set");
        let error = invoke_for_surface(scopes, false)
            .await
            .expect_err("missing graph scope denied");
        assert_eq!(error.kind, crate::abilities::AbilityErrorKind::Capability);
        assert_eq!(
            error.message,
            "permission_denied: read.workspace_graph_required"
        );
    }

    #[tokio::test]
    async fn surface_client_explicit_name_request_without_name_scope_is_denied() {
        ScopeSet::set_allowlist_for_tests([
            SurfaceScope::new("read.workspace_graph"),
            SurfaceScope::new("read.entity_names"),
            SurfaceScope::new("read.account_overview"),
            SurfaceScope::new("read.composition"),
            SurfaceScope::new("submit.feedback"),
        ]);
        let scopes = ScopeSet::new([SurfaceScope::new("read.workspace_graph")]).expect("scope set");
        let error = invoke_for_surface(scopes, true)
            .await
            .expect_err("missing name scope denied");
        assert_eq!(error.kind, crate::abilities::AbilityErrorKind::Capability);
        assert_eq!(
            error.message,
            "permission_denied: read.entity_names_required"
        );
    }

    #[tokio::test]
    async fn agent_reads_use_prompt_safe_privacy_profile() {
        let registry = AbilityRegistry::global_checked().expect("registry");
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 24, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(7);
        let reader = std::sync::Arc::new(CapturingWorkspaceGraphReader::default());
        let services = ServiceContext::new_evaluate_default(&clock, &rng)
            .with_actor("test")
            .with_workspace_graph_reader(reader.clone());
        let provider = ReplayProvider::new(std::collections::HashMap::new());
        let ctx = crate::abilities::AbilityContext::new(
            &services,
            &provider,
            &NOOP_ABILITY_TRACER,
            Actor::Agent,
            None,
            ClaimDismissalSurface::Eval,
        );

        registry
            .invoke_by_name_json(
                &ctx,
                ABILITY_NAME,
                json!({
                    "schemaVersion": 1,
                    "includeEntityNames": false,
                    "pageSize": 25
                }),
            )
            .await
            .expect("agent graph read succeeds");

        let requests = reader.requests.lock().expect("requests lock");
        assert_eq!(requests.len(), 1);
        assert_eq!(
            requests[0].privacy_profile,
            WorkspaceGraphPrivacyProfile::SurfaceClient
        );
    }

    async fn invoke_for_surface(
        scopes: ScopeSet,
        include_entity_names: bool,
    ) -> Result<serde_json::Value, crate::abilities::AbilityError> {
        let registry = AbilityRegistry::global_checked().expect("registry");
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 24, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(7);
        let services = ServiceContext::new_evaluate_default(&clock, &rng).with_actor("test");
        let provider = ReplayProvider::new(std::collections::HashMap::new());
        let actor = Actor::SurfaceClient {
            instance: SurfaceClientId::new("surface-alpha"),
            scopes,
        };
        let ctx = crate::abilities::AbilityContext::new(
            &services,
            &provider,
            &NOOP_ABILITY_TRACER,
            actor,
            None,
            ClaimDismissalSurface::Eval,
        );
        registry
            .invoke_by_name_json(
                &ctx,
                ABILITY_NAME,
                json!({
                    "schemaVersion": 1,
                    "includeEntityNames": include_entity_names,
                    "pageSize": 25
                }),
            )
            .await
    }

    #[derive(Default)]
    struct CapturingWorkspaceGraphReader {
        requests: std::sync::Mutex<Vec<WorkspaceGraphReadRequest>>,
    }

    impl WorkspaceGraphReadHandle for CapturingWorkspaceGraphReader {
        fn read_workspace_graph<'a>(
            &'a self,
            request: WorkspaceGraphReadRequest,
        ) -> WorkspaceGraphReadFuture<'a> {
            self.requests.lock().expect("requests lock").push(request);
            Box::pin(async { Ok(empty_workspace_graph_response()) })
        }
    }

    fn empty_workspace_graph_response() -> WorkspaceGraphResponse {
        WorkspaceGraphResponse::Projection(WorkspaceGraphProjection {
            schema_version: 1,
            graph_version: "v1:test".to_string(),
            page: WorkspaceGraphPage {
                next_cursor: None,
                has_more: false,
            },
            projection: WorkspaceGraphProjectionBody {
                entities: Vec::new(),
            },
            audit: WorkspaceGraphAudit {
                schema_version: 1,
                graph_version: "v1:test".to_string(),
                gap_counts: std::collections::BTreeMap::new(),
                gaps: Vec::new(),
            },
        })
    }
}
