use std::sync::Arc;

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
use abilities_runtime::sensitivity::{ClaimDismissalSurface, ClaimVerificationState};
use abilities_runtime::services::context::{
    EntityContextClaimReadFuture, EntityContextClaimReadHandle, FixedClock, SeedableRng,
    ServiceContext,
};
use abilities_runtime::services::workspace_intake::{
    EntityRefDto, WorkspaceIntakeError, WorkspaceIntakeReceipt, WorkspaceIntakeRequest,
    WorkspaceIntakeService,
};
use abilities_runtime::types::{
    ClaimSensitivity, ClaimState, IntelligenceClaim, SurfacingState, TemporalScope,
};
use async_trait::async_trait;
use chrono::TimeZone;

struct OkIntake;

#[async_trait]
impl WorkspaceIntakeService for OkIntake {
    async fn ingest(
        &self,
        _ctx: &AbilityContext<'_>,
        _request: WorkspaceIntakeRequest,
    ) -> Result<WorkspaceIntakeReceipt, WorkspaceIntakeError> {
        Ok(WorkspaceIntakeReceipt {
            run_id: "run-w2c".to_string(),
            file_id: "file-w2c".to_string(),
            content_sha256: "sha".to_string(),
            lifecycle_state_after_slug: "ingested".to_string(),
            resolved_path: None,
        })
    }
}

struct FixtureClaimReader;

impl EntityContextClaimReadHandle for FixtureClaimReader {
    fn read_entity_context_claims<'a>(
        &'a self,
        entity_type: String,
        entity_id: String,
        _surface: ClaimDismissalSurface,
        _depth: usize,
    ) -> EntityContextClaimReadFuture<'a> {
        Box::pin(async move {
            assert_eq!(entity_type, "account");
            assert_eq!(entity_id, "acct_acme");
            Ok(vec![claim(
                "claim-1",
                "Acme budget owner approved the workspace plan",
            )])
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

fn claim(id: &str, text: &str) -> IntelligenceClaim {
    IntelligenceClaim {
        id: id.to_string(),
        claim_version: 1,
        subject_ref: r#"{"kind":"account","id":"acct_acme"}"#.to_string(),
        claim_type: "fact".to_string(),
        field_path: Some("summary".to_string()),
        topic_key: None,
        text: text.to_string(),
        dedup_key: "dedup".to_string(),
        item_hash: None,
        actor: "fixture".to_string(),
        data_source: "workspace_file".to_string(),
        source_ref: Some("accounts/acme/brief.md".to_string()),
        source_asof: Some("2026-05-21T12:00:00Z".to_string()),
        observed_at: "2026-05-21T12:00:00Z".to_string(),
        created_at: "2026-05-21T12:00:00Z".to_string(),
        provenance_json: "{}".to_string(),
        metadata_json: None,
        claim_state: ClaimState::Active,
        surfacing_state: SurfacingState::Active,
        demotion_reason: None,
        reactivated_at: None,
        retraction_reason: None,
        expires_at: None,
        superseded_by: None,
        trust_score: Some(0.92),
        trust_computed_at: Some("2026-05-21T12:00:00Z".to_string()),
        trust_version: Some(1),
        thread_id: None,
        temporal_scope: TemporalScope::State,
        sensitivity: ClaimSensitivity::Public,
        verification_state: ClaimVerificationState::Active,
        verification_reason: None,
        needs_user_decision_at: None,
    }
}

#[tokio::test]
async fn claim_display_text_is_populated_from_renderable_projection() {
    let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 21, 12, 0, 0).unwrap());
    let rng = SeedableRng::new(468);
    let services = ServiceContext::new_evaluate_default(&clock, &rng)
        .with_workspace_intake(Arc::new(OkIntake))
        .with_entity_context_claim_reader(Arc::new(FixtureClaimReader));
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
                entity_name: None,
            }),
            category: None,
        },
    )
    .await
    .expect("entity intake succeeds")
    .into_data();

    assert_eq!(output.claims.len(), 1);
    assert_eq!(
        output.claims[0].display_text,
        "Acme budget owner approved the workspace plan"
    );
}
