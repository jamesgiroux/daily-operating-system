//! `source_management_ledger` read ability.

pub mod contracts;
pub mod producer;

pub use contracts::{
    SourceManagementActionInput, SourceManagementActionKind, SourceManagementActionPolicy,
    SourceManagementActionReceipt, SourceManagementActionRequest, SourceManagementEntity,
    SourceManagementIngestionRun, SourceManagementLedgerInput, SourceManagementLedgerPage,
    SourceManagementLedgerPrivacyProfile, SourceManagementLedgerReadRequest,
    SourceManagementLedgerResponse, SourceManagementSource, SourceManagementSourceActions,
    SourceManagementTrustBandSummary, SourceManagementUserOverride,
};

use dailyos_abilities_macro::ability;

use crate::abilities::{AbilityContext, AbilityResult};

pub const ABILITY_NAME: &str = "source_management_ledger";
pub const ACTION_ABILITY_NAME: &str = "source_management_action";
pub const ABILITY_SCHEMA_VERSION: u32 = 1;
pub const SOURCE_MANAGEMENT_LEDGER_SCOPE: &str = "read.workspace_sources";
pub const SOURCE_MANAGEMENT_ACTION_SCOPE: &str = "write.entity_intake";

#[ability(
    name = "source_management_ledger",
    category = Read,
    version = "1.0.0",
    schema_version = 1,
    allowed_actors = [User, System, SurfaceClient],
    allowed_modes = [Live],
    requires_confirmation = false,
    may_publish = false,
    required_scopes = ["read.workspace_sources"],
    mcp_exposure = None,
    client_side_executable = false,
    composes = [],
    experimental = false,
    signal_policy = { emits_on_output_change = [], coalesce = false }
)]
pub async fn source_management_ledger(
    ctx: &AbilityContext<'_>,
    input: SourceManagementLedgerInput,
) -> AbilityResult<SourceManagementLedgerResponse> {
    producer::source_management_ledger(ctx, input).await
}

#[ability(
    name = "source_management_action",
    category = Transform,
    version = "1.0.0",
    schema_version = 1,
    allowed_actors = [SurfaceClient],
    allowed_modes = [Live],
    requires_confirmation = false,
    may_publish = true,
    required_scopes = ["write.entity_intake"],
    mcp_exposure = None,
    client_side_executable = false,
    composes = [],
    experimental = false,
    signal_policy = { emits_on_output_change = [], coalesce = false }
)]
pub async fn source_management_action(
    ctx: &AbilityContext<'_>,
    input: SourceManagementActionInput,
) -> AbilityResult<SourceManagementActionReceipt> {
    producer::source_management_action(ctx, input).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::registry::{
        AbilityRegistry, ActorKind, McpExposure, ScopeSet, SurfaceClientId, SurfaceScope,
    };
    use crate::abilities::{Actor, NOOP_ABILITY_TRACER};
    use crate::intelligence::provider::ReplayProvider;
    use crate::sensitivity::ClaimDismissalSurface;
    use crate::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};

    #[test]
    fn descriptor_is_surface_read_only_and_not_mcp_visible() {
        let registry = AbilityRegistry::global_checked().expect("registry builds");
        let descriptor = registry
            .iter_all()
            .find(|descriptor| descriptor.name == ABILITY_NAME)
            .expect("source management ledger ability is registered");

        assert!(descriptor.policy.allowed_actors.contains(&ActorKind::User));
        assert!(descriptor
            .policy
            .allowed_actors
            .contains(&ActorKind::SurfaceClient));
        assert!(!descriptor
            .policy
            .allowed_actors
            .contains(&ActorKind::McpClient));
        assert_eq!(
            descriptor.policy.required_scopes,
            &[SOURCE_MANAGEMENT_LEDGER_SCOPE]
        );
        assert_eq!(descriptor.policy.mcp_exposure, McpExposure::None);
        assert!(!descriptor.policy.may_publish);
        assert!(!descriptor.policy.client_side_executable);
    }

    #[test]
    fn action_descriptor_is_surface_write_only_and_not_mcp_visible() {
        let registry = AbilityRegistry::global_checked().expect("registry builds");
        let descriptor = registry
            .iter_all()
            .find(|descriptor| descriptor.name == ACTION_ABILITY_NAME)
            .expect("source management action ability is registered");

        assert!(descriptor
            .policy
            .allowed_actors
            .contains(&ActorKind::SurfaceClient));
        assert!(!descriptor.policy.allowed_actors.contains(&ActorKind::User));
        assert!(!descriptor
            .policy
            .allowed_actors
            .contains(&ActorKind::McpClient));
        assert_eq!(
            descriptor.policy.required_scopes,
            &[SOURCE_MANAGEMENT_ACTION_SCOPE]
        );
        assert_eq!(descriptor.policy.mcp_exposure, McpExposure::None);
        assert!(descriptor.policy.may_publish);
        assert!(!descriptor.policy.client_side_executable);
    }

    #[tokio::test]
    async fn surface_client_without_workspace_sources_scope_is_denied_before_reader() {
        ScopeSet::set_allowlist_for_tests([
            SurfaceScope::new("read.workspace_sources"),
            SurfaceScope::new("read.account_overview"),
            SurfaceScope::new("read.composition"),
            SurfaceScope::new("submit.feedback"),
        ]);
        let scopes =
            ScopeSet::new([SurfaceScope::new("read.account_overview")]).expect("scope set");
        let registry = AbilityRegistry::global_checked().expect("registry");
        let clock = FixedClock::new(chrono::Utc::now());
        let rng = SeedableRng::new(7);
        let external = ExternalClients::default();
        let services = ServiceContext::new_live(&clock, &rng, &external).with_actor("test");
        let provider = ReplayProvider::new(std::collections::HashMap::new());
        let ctx = crate::abilities::AbilityContext::new(
            &services,
            &provider,
            &NOOP_ABILITY_TRACER,
            Actor::SurfaceClient {
                instance: SurfaceClientId::new("surface-alpha"),
                scopes,
            },
            None,
            ClaimDismissalSurface::Eval,
        );

        let error = registry
            .invoke_by_name_json(
                &ctx,
                ABILITY_NAME,
                serde_json::json!({
                    "schemaVersion": 1,
                    "entityType": "account",
                    "entityId": "acct-test-001",
                    "pageSize": 25
                }),
            )
            .await
            .expect_err("missing scope denied before reader");

        assert_eq!(error.kind, crate::abilities::AbilityErrorKind::Capability);
        assert_eq!(
            error.message,
            "permission_denied: read.workspace_sources_required"
        );
    }
}
