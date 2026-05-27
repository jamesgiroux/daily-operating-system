//! `markdown_preview` read ability.

pub mod contracts;
pub mod producer;

pub use contracts::{MarkdownPreviewInput, MarkdownPreviewOutput, MarkdownPreviewReadRequest};

use dailyos_abilities_macro::ability;

use crate::abilities::{AbilityContext, AbilityResult};

pub const ABILITY_NAME: &str = "markdown_preview";
pub const ABILITY_SCHEMA_VERSION: u32 = 1;
pub const MARKDOWN_PREVIEW_SCOPE: &str = "read.markdown_preview";

#[ability(
    name = "markdown_preview",
    category = Read,
    version = "1.0.0",
    schema_version = 1,
    allowed_actors = [SurfaceClient],
    allowed_modes = [Live],
    requires_confirmation = false,
    may_publish = false,
    required_scopes = ["read.markdown_preview"],
    mcp_exposure = None,
    client_side_executable = false,
    composes = [],
    experimental = false,
    signal_policy = { emits_on_output_change = [], coalesce = false }
)]
pub async fn markdown_preview(
    ctx: &AbilityContext<'_>,
    input: MarkdownPreviewInput,
) -> AbilityResult<MarkdownPreviewOutput> {
    producer::markdown_preview(ctx, input).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::registry::{
        AbilityRegistry, ActorKind, McpExposure, ScopeSet, SurfaceClientId, SurfaceScope,
    };
    use crate::abilities::AbilityCategory;
    use crate::abilities::{Actor, NOOP_ABILITY_TRACER};
    use crate::intelligence::provider::ReplayProvider;
    use crate::sensitivity::ClaimDismissalSurface;
    use crate::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};

    #[test]
    fn descriptor_is_surface_only_read_and_not_mcp_visible() {
        let registry = AbilityRegistry::global_checked().expect("registry builds");
        let descriptor = registry
            .iter_all()
            .find(|descriptor| descriptor.name == ABILITY_NAME)
            .expect("markdown preview ability is registered");

        assert_eq!(descriptor.category, AbilityCategory::Read);
        assert_eq!(descriptor.policy.allowed_actors, &[ActorKind::SurfaceClient]);
        assert_eq!(descriptor.policy.required_scopes, &[MARKDOWN_PREVIEW_SCOPE]);
        assert_eq!(descriptor.policy.mcp_exposure, McpExposure::None);
        assert!(!descriptor.policy.may_publish);
        assert!(!descriptor.policy.client_side_executable);
    }

    #[tokio::test]
    async fn surface_client_without_markdown_preview_scope_is_denied_before_reader() {
        ScopeSet::set_allowlist_for_tests([
            SurfaceScope::new("read.markdown_preview"),
            SurfaceScope::new("read.account_overview"),
            SurfaceScope::new("read.composition"),
            SurfaceScope::new("submit.feedback"),
        ]);
        let scopes = ScopeSet::new([SurfaceScope::new("read.account_overview")])
            .expect("scope set");
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
                    "sourceHandle": "source_test"
                }),
            )
            .await
            .expect_err("missing scope denied before reader");

        assert_eq!(error.kind, crate::abilities::AbilityErrorKind::Capability);
        assert_eq!(
            error.message,
            "permission_denied: read.markdown_preview_required"
        );
    }
}
