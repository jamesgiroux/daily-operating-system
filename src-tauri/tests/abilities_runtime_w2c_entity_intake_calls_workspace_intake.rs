use std::sync::{Arc, Mutex};

use abilities_runtime::abilities::entity_intake::producer::entity_intake;
use abilities_runtime::abilities::entity_intake::EntityIntakeInput;
use abilities_runtime::abilities::registry::{
    AbilityContext, Actor, ScopeSet, SurfaceClientId, SurfaceScope,
};
use abilities_runtime::abilities::NOOP_ABILITY_TRACER;
use abilities_runtime::intelligence::provider::{
    Completion, FingerprintMetadata, IntelligenceProvider, ModelName, ModelTier, PromptInput,
    ProviderError, ProviderKind,
};
use abilities_runtime::sensitivity::ClaimDismissalSurface;
use abilities_runtime::services::context::{
    EntityContextClaimReadFuture, EntityContextClaimReadHandle, FixedClock, SeedableRng,
    ServiceContext,
};
use abilities_runtime::services::workspace_intake::{
    EntityRefDto, WorkspaceIntakeError, WorkspaceIntakeReceipt, WorkspaceIntakeRequest,
    WorkspaceIntakeService,
};
use async_trait::async_trait;
use chrono::TimeZone;

#[derive(Default)]
struct SpyIntake {
    request: Mutex<Option<WorkspaceIntakeRequest>>,
}

struct EmptyClaimReader;

impl EntityContextClaimReadHandle for EmptyClaimReader {
    fn read_entity_context_claims<'a>(
        &'a self,
        _entity_type: String,
        _entity_id: String,
        _surface: ClaimDismissalSurface,
        _depth: usize,
    ) -> EntityContextClaimReadFuture<'a> {
        Box::pin(async { Ok(Vec::new()) })
    }
}

#[async_trait]
impl WorkspaceIntakeService for SpyIntake {
    async fn ingest(
        &self,
        _ctx: &AbilityContext<'_>,
        request: WorkspaceIntakeRequest,
    ) -> Result<WorkspaceIntakeReceipt, WorkspaceIntakeError> {
        *self.request.lock().expect("request lock") = Some(request);
        Ok(WorkspaceIntakeReceipt {
            run_id: "run-w2c".to_string(),
            file_id: "file-w2c".to_string(),
            content_sha256: "sha".to_string(),
            lifecycle_state_after_slug: "ingested".to_string(),
            resolved_path: Some("accounts/acme/brief.md".to_string()),
        })
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
async fn calls_workspace_intake_with_entity_seeded_dto() {
    let intake = Arc::new(SpyIntake::default());
    let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 21, 12, 0, 0).unwrap());
    let rng = SeedableRng::new(468);
    let services = ServiceContext::new_evaluate_default(&clock, &rng)
        .with_workspace_intake(intake.clone())
        .with_entity_context_claim_reader(Arc::new(EmptyClaimReader));
    let provider = StaticProvider;
    let ctx = AbilityContext::new(
        &services,
        &provider,
        &NOOP_ABILITY_TRACER,
        surface_actor(),
        None,
        ClaimDismissalSurface::Eval,
    );

    let output = entity_intake(
        &ctx,
        EntityIntakeInput {
            file_ref: "accounts/acme/brief.md".to_string(),
            entity_seed: Some(EntityRefDto {
                entity_type_slug: "account".to_string(),
                entity_id: "acct_acme".to_string(),
                entity_name: Some("Acme".to_string()),
            }),
            category: Some("briefs".to_string()),
        },
    )
    .await
    .expect("entity intake succeeds")
    .into_data();

    assert_eq!(output.run_id, "run-w2c");
    let request = intake
        .request
        .lock()
        .expect("request lock")
        .clone()
        .expect("workspace intake called");
    assert_eq!(request.file_ref, "accounts/acme/brief.md");
    assert_eq!(request.source_type_slug, "entity_doc");
    assert_eq!(request.mode_slug, "entity_seeded");
    assert_eq!(request.category_slug.as_deref(), Some("briefs"));
    assert_eq!(
        request.entity.as_ref().map(|entity| entity.entity_type_slug.as_str()),
        Some("account")
    );
    assert_eq!(
        request.entity.as_ref().map(|entity| entity.entity_id.as_str()),
        Some("acct_acme")
    );
}
