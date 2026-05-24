use std::sync::Arc;

use abilities_runtime::abilities::entity_intake::producer::entity_intake;
use abilities_runtime::abilities::entity_intake::EntityIntakeInput;
use abilities_runtime::abilities::registry::{
    AbilityContext, AbilityErrorKind, Actor, ScopeSet, SurfaceClientId, SurfaceScope,
};
use abilities_runtime::abilities::NOOP_ABILITY_TRACER;
use abilities_runtime::intelligence::provider::{
    Completion, FingerprintMetadata, IntelligenceProvider, ModelName, ModelTier, PromptInput,
    ProviderError, ProviderKind,
};
use abilities_runtime::sensitivity::ClaimDismissalSurface;
use abilities_runtime::services::context::{FixedClock, SeedableRng, ServiceContext};
use abilities_runtime::services::workspace_intake::{
    EntityRefDto, WorkspaceIntakeError, WorkspaceIntakeReceipt, WorkspaceIntakeRequest,
    WorkspaceIntakeService,
};
use async_trait::async_trait;
use chrono::TimeZone;

struct RejectingIntake;

#[async_trait]
impl WorkspaceIntakeService for RejectingIntake {
    async fn ingest(
        &self,
        _ctx: &AbilityContext<'_>,
        _request: WorkspaceIntakeRequest,
    ) -> Result<WorkspaceIntakeReceipt, WorkspaceIntakeError> {
        Err(WorkspaceIntakeError::InvalidEntityTypeSlug(
            "invalid kind".to_string(),
        ))
    }
}

struct StaticProvider;

#[async_trait]
impl IntelligenceProvider for StaticProvider {
    async fn complete(
        &self,
        _prompt: PromptInput,
        _tier: ModelTier,
    ) -> Result<Completion, ProviderError> {
        Ok(Completion {
            text: String::new(),
            fingerprint_metadata: FingerprintMetadata {
                provider: ProviderKind::Other("test"),
                model: ModelName::new("unused"),
                temperature: 0.0,
                top_p: None,
                seed: None,
                tokens_input: None,
                tokens_output: None,
                provider_completion_id: None,
            },
        })
    }

    fn provider_kind(&self) -> ProviderKind {
        ProviderKind::Other("test")
    }

    fn current_model(&self, _tier: ModelTier) -> ModelName {
        ModelName::new("unused")
    }
}

fn surface_actor() -> Actor {
    Actor::SurfaceClient {
        instance: SurfaceClientId::new("wp-test"),
        scopes: ScopeSet::new([SurfaceScope::new("write.entity_intake")]).expect("scope set"),
    }
}

#[tokio::test]
async fn invalid_entity_type_slug_maps_to_typed_error() {
    let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 21, 12, 0, 0).unwrap());
    let rng = SeedableRng::new(468);
    let services = ServiceContext::new_evaluate_default(&clock, &rng)
        .with_workspace_intake(Arc::new(RejectingIntake));
    let provider = StaticProvider;
    let ctx = AbilityContext::new(
        &services,
        &provider,
        &NOOP_ABILITY_TRACER,
        surface_actor(),
        None,
        ClaimDismissalSurface::Eval,
    );

    let err = entity_intake(
        &ctx,
        EntityIntakeInput {
            file_ref: "accounts/acme/brief.md".to_string(),
            entity_seed: Some(EntityRefDto {
                entity_type_slug: "invalid kind".to_string(),
                entity_id: "acct_acme".to_string(),
                entity_name: None,
            }),
            category: None,
        },
    )
    .await
    .expect_err("invalid slug should fail");

    assert_eq!(
        err.kind,
        AbilityErrorKind::HardError("InvalidEntityType".to_string())
    );
}
