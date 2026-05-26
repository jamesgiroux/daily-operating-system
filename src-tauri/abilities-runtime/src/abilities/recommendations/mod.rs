//! Recommendation read abilities.

pub mod contracts;

pub use contracts::{
    BoundedNote, ClaimId, ConversionState, ConversionTarget, DismissReason, FactorRationale,
    FeedbackState, ListSuggestedNextStepsInput, ListSuggestedNextStepsResponse, PrimaryFactorBand,
    RecommendationFeedbackDecision, RecommendedActionView, SalienceFactor, SalienceFactorKind,
    SaliencePersistence, SalienceReadError, SalienceScore, ScoreSalienceInput,
    ScoreSalienceReadRequest, ScoreSalienceResponse, SuggestedNextStepItem,
    SuggestedNextStepsReadError, LIST_SUGGESTED_NEXT_STEPS_ABILITY_NAME,
    LIST_SUGGESTED_NEXT_STEPS_SCHEMA_VERSION, LIST_SUGGESTED_NEXT_STEPS_SCOPE,
    SCORE_SALIENCE_ABILITY_NAME, SCORE_SALIENCE_SCHEMA_VERSION, SCORE_SALIENCE_SCOPE,
};

use dailyos_abilities_macro::ability;

use crate::abilities::provenance::{
    AbilityExecutionMode, AbilityVersion, FieldAttribution, FieldPath, ProvenanceBuilder,
    ProvenanceBuilderConfig, SchemaVersion, SubjectAttribution, SubjectRef,
};
use crate::abilities::registry::Actor;
use crate::abilities::{
    AbilityCategory, AbilityContext, AbilityError, AbilityErrorKind, AbilityResult,
};

#[ability(
    name = "score_salience",
    category = Read,
    version = "1.0.0",
    schema_version = 1,
    allowed_actors = [User, System],
    allowed_modes = [Live, Evaluate],
    requires_confirmation = false,
    may_publish = false,
    required_scopes = ["read.recommendations"],
    mcp_exposure = None,
    client_side_executable = false,
    composes = [],
    experimental = false,
    signal_policy = { emits_on_output_change = [], coalesce = false }
)]
pub async fn score_salience(
    ctx: &AbilityContext<'_>,
    input: ScoreSalienceInput,
) -> AbilityResult<ScoreSalienceResponse> {
    validate_schema_version(input.schema_version)?;
    let schema_version = input.schema_version;
    let response = ctx
        .services()
        .score_salience(ScoreSalienceReadRequest {
            schema_version,
            claim_id: input.claim_id,
            actor: ctx.actor.kind(),
        })
        .await
        .map_err(read_error)?;

    finalize_output(ctx, schema_version, response)
}

#[ability(
    name = "list_suggested_next_steps",
    category = Read,
    version = "1.0.0",
    schema_version = 1,
    allowed_actors = [User, System, SurfaceClient],
    allowed_modes = [Live, Evaluate],
    requires_confirmation = false,
    may_publish = false,
    required_scopes = ["read.recommendations"],
    mcp_exposure = None,
    client_side_executable = false,
    composes = [],
    experimental = false,
    signal_policy = { emits_on_output_change = [], coalesce = false }
)]
pub async fn list_suggested_next_steps(
    ctx: &AbilityContext<'_>,
    input: ListSuggestedNextStepsInput,
) -> AbilityResult<ListSuggestedNextStepsResponse> {
    validate_list_suggested_next_steps_schema_version(input.schema_version)?;
    let schema_version = input.schema_version;
    let subject = input.subject.clone().unwrap_or(SubjectRef::Global);
    let response = ctx
        .services()
        .list_suggested_next_steps(input, ctx.actor.kind())
        .await
        .map_err(suggested_next_steps_read_error)?;

    finalize_list_suggested_next_steps_output(ctx, schema_version, subject, response)
}

fn validate_schema_version(schema_version: u32) -> Result<(), AbilityError> {
    if schema_version == SCORE_SALIENCE_SCHEMA_VERSION {
        Ok(())
    } else {
        Err(AbilityError {
            kind: AbilityErrorKind::Validation,
            message: format!(
                "unsupported schema_version `{schema_version}` for `{SCORE_SALIENCE_ABILITY_NAME}`"
            ),
        })
    }
}

fn validate_list_suggested_next_steps_schema_version(
    schema_version: u16,
) -> Result<(), AbilityError> {
    if schema_version == LIST_SUGGESTED_NEXT_STEPS_SCHEMA_VERSION {
        Ok(())
    } else {
        Err(AbilityError {
            kind: AbilityErrorKind::Validation,
            message: format!(
                "unsupported schema_version `{schema_version}` for `{LIST_SUGGESTED_NEXT_STEPS_ABILITY_NAME}`"
            ),
        })
    }
}

fn read_error(error: SalienceReadError) -> AbilityError {
    match error {
        SalienceReadError::UnsupportedSchemaVersion(schema_version) => AbilityError {
            kind: AbilityErrorKind::Validation,
            message: format!(
                "unsupported schema_version `{schema_version}` for `{SCORE_SALIENCE_ABILITY_NAME}`"
            ),
        },
        SalienceReadError::ClaimNotFound(claim_id)
        | SalienceReadError::ClaimNotVisible(claim_id) => AbilityError {
            kind: AbilityErrorKind::Validation,
            message: format!("claim_not_found: {claim_id}"),
        },
        SalienceReadError::ReadFailed(message) => AbilityError {
            kind: AbilityErrorKind::HardError(message.clone()),
            message: format!("salience_read_failed: {message}"),
        },
    }
}

fn suggested_next_steps_read_error(error: SuggestedNextStepsReadError) -> AbilityError {
    match error {
        SuggestedNextStepsReadError::UnsupportedSchemaVersion(schema_version) => AbilityError {
            kind: AbilityErrorKind::Validation,
            message: format!(
                "unsupported schema_version `{schema_version}` for `{LIST_SUGGESTED_NEXT_STEPS_ABILITY_NAME}`"
            ),
        },
        SuggestedNextStepsReadError::ClaimNotFound(claim_id)
        | SuggestedNextStepsReadError::ClaimNotVisible(claim_id) => AbilityError {
            kind: AbilityErrorKind::Validation,
            message: format!("claim_not_found: {claim_id}"),
        },
        SuggestedNextStepsReadError::ReadFailed(message) => AbilityError {
            kind: AbilityErrorKind::HardError(message.clone()),
            message: format!("suggested_next_steps_read_failed: {message}"),
        },
    }
}

fn finalize_output(
    ctx: &AbilityContext<'_>,
    schema_version: u32,
    response: ScoreSalienceResponse,
) -> AbilityResult<ScoreSalienceResponse> {
    let mut builder = ProvenanceBuilder::new(provenance_config(ctx, schema_version));
    let subject_attribution = SubjectAttribution::direct_confident(SubjectRef::Global);
    builder.set_subject(subject_attribution.clone());
    builder
        .attribute_subtree(
            FieldPath::root(),
            FieldAttribution::constant(subject_attribution),
        )
        .map_err(provenance_error)?;
    builder.finalize(response).map_err(provenance_error)
}

fn finalize_list_suggested_next_steps_output(
    ctx: &AbilityContext<'_>,
    schema_version: u16,
    subject: SubjectRef,
    response: ListSuggestedNextStepsResponse,
) -> AbilityResult<ListSuggestedNextStepsResponse> {
    let mut builder = ProvenanceBuilder::new(list_suggested_next_steps_provenance_config(
        ctx,
        schema_version,
    ));
    let subject_attribution = SubjectAttribution::direct_confident(subject);
    builder.set_subject(subject_attribution.clone());
    builder
        .attribute_subtree(
            FieldPath::root(),
            FieldAttribution::constant(subject_attribution),
        )
        .map_err(provenance_error)?;
    builder.finalize(response).map_err(provenance_error)
}

fn provenance_config(ctx: &AbilityContext<'_>, schema_version: u32) -> ProvenanceBuilderConfig {
    let mut config =
        ProvenanceBuilderConfig::new(SCORE_SALIENCE_ABILITY_NAME, ctx.services().clock.now());
    config.ability_version = AbilityVersion::new(1, 0);
    config.ability_schema_version = SchemaVersion(schema_version);
    config.actor = provenance_actor(&ctx.actor);
    config.mode = AbilityExecutionMode::from(ctx.mode());
    config.category = AbilityCategory::Read;
    config
}

fn list_suggested_next_steps_provenance_config(
    ctx: &AbilityContext<'_>,
    schema_version: u16,
) -> ProvenanceBuilderConfig {
    let mut config = ProvenanceBuilderConfig::new(
        LIST_SUGGESTED_NEXT_STEPS_ABILITY_NAME,
        ctx.services().clock.now(),
    );
    config.ability_version = AbilityVersion::new(1, 0);
    config.ability_schema_version = SchemaVersion(u32::from(schema_version));
    config.actor = provenance_actor(&ctx.actor);
    config.mode = AbilityExecutionMode::from(ctx.mode());
    config.category = AbilityCategory::Read;
    config
}

fn provenance_actor(actor: &Actor) -> crate::abilities::provenance::Actor {
    match actor {
        Actor::SurfaceClient { .. } => crate::abilities::provenance::Actor::Human {
            role: "surface_client".to_string(),
            id: "surface_client".to_string(),
        },
        Actor::User => crate::abilities::provenance::Actor::User,
        Actor::Agent => crate::abilities::provenance::Actor::Agent {
            name: "agent".to_string(),
            version: "unknown".to_string(),
        },
        Actor::Admin => crate::abilities::provenance::Actor::Human {
            role: "admin".to_string(),
            id: "admin".to_string(),
        },
        Actor::System => crate::abilities::provenance::Actor::System {
            component: "dailyos".to_string(),
        },
        Actor::McpClient { .. } => crate::abilities::provenance::Actor::Agent {
            name: "mcp_client".to_string(),
            version: "unknown".to_string(),
        },
    }
}

fn provenance_error(error: impl std::fmt::Display) -> AbilityError {
    AbilityError {
        kind: AbilityErrorKind::Validation,
        message: format!("provenance construction failed: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::{Arc, Mutex};

    use chrono::{TimeZone, Utc};
    use serde_json::json;

    use crate::abilities::registry::{
        AbilityRegistry, ActorKind, McpClientId, McpExposure, OpaqueConversationHandle, ScopeSet,
        SurfaceClientId, SurfaceScope,
    };
    use crate::abilities::trust::types::TrustBand;
    use crate::abilities::NOOP_ABILITY_TRACER;
    use crate::intelligence::provider::ReplayProvider;
    use crate::sensitivity::ClaimDismissalSurface;
    use crate::services::context::{
        ExternalClients, FixedClock, SalienceReadFuture, SalienceReadHandle, SeedableRng,
        ServiceContext, SuggestedNextStepsReadFuture, SuggestedNextStepsReadHandle,
    };

    #[derive(Default)]
    struct CapturingSalienceReader {
        requests: Mutex<Vec<ScoreSalienceReadRequest>>,
    }

    impl SalienceReadHandle for CapturingSalienceReader {
        fn score_salience<'a>(
            &'a self,
            request: ScoreSalienceReadRequest,
        ) -> SalienceReadFuture<'a> {
            self.requests.lock().expect("requests lock").push(request);
            Box::pin(async { Ok(response_fixture()) })
        }
    }

    struct ErroringSalienceReader {
        error: SalienceReadError,
    }

    impl SalienceReadHandle for ErroringSalienceReader {
        fn score_salience<'a>(
            &'a self,
            _request: ScoreSalienceReadRequest,
        ) -> SalienceReadFuture<'a> {
            let error = self.error.clone();
            Box::pin(async move { Err(error) })
        }
    }

    #[derive(Default)]
    struct CapturingSuggestedNextStepsReader {
        requests: Mutex<Vec<(ListSuggestedNextStepsInput, ActorKind)>>,
    }

    impl SuggestedNextStepsReadHandle for CapturingSuggestedNextStepsReader {
        fn list_suggested_next_steps<'a>(
            &'a self,
            input: ListSuggestedNextStepsInput,
            actor: ActorKind,
        ) -> SuggestedNextStepsReadFuture<'a> {
            self.requests
                .lock()
                .expect("requests lock")
                .push((input, actor));
            Box::pin(async { Ok(suggested_next_steps_response_fixture()) })
        }
    }

    #[test]
    fn descriptor_is_user_system_read_only_and_hidden_from_clients() {
        let registry = AbilityRegistry::global_checked().expect("registry builds");
        let descriptor = registry
            .iter_all()
            .find(|descriptor| descriptor.name == SCORE_SALIENCE_ABILITY_NAME)
            .expect("score_salience ability is registered");

        assert_eq!(descriptor.category, AbilityCategory::Read);
        assert!(descriptor.policy.allowed_actors.contains(&ActorKind::User));
        assert!(descriptor
            .policy
            .allowed_actors
            .contains(&ActorKind::System));
        assert!(!descriptor
            .policy
            .allowed_actors
            .contains(&ActorKind::SurfaceClient));
        assert!(!descriptor
            .policy
            .allowed_actors
            .contains(&ActorKind::McpClient));
        assert_eq!(descriptor.policy.required_scopes, &[SCORE_SALIENCE_SCOPE]);
        assert_eq!(descriptor.policy.mcp_exposure, McpExposure::None);
        assert!(!descriptor.policy.may_publish);
        assert!(!descriptor.policy.client_side_executable);
        assert!(descriptor.mutates.is_empty());
    }

    #[test]
    fn registry_does_not_admit_surface_or_mcp_actors() {
        let registry = AbilityRegistry::global_checked().expect("registry builds");
        ScopeSet::set_allowlist_for_tests([SurfaceScope::new(SCORE_SALIENCE_SCOPE)]);
        let surface_scopes =
            ScopeSet::new([SurfaceScope::new(SCORE_SALIENCE_SCOPE)]).expect("surface scope set");
        assert!(registry
            .iter_for(Actor::SurfaceClient {
                instance: SurfaceClientId::new("surface-1"),
                scopes: surface_scopes,
            })
            .all(|descriptor| descriptor.name != SCORE_SALIENCE_ABILITY_NAME));
        assert!(registry
            .iter_for(Actor::McpClient {
                client_id: McpClientId::new("client-1"),
                conversation_handle: Some(OpaqueConversationHandle::new("conv-1")),
            })
            .all(|descriptor| descriptor.name != SCORE_SALIENCE_ABILITY_NAME));
    }

    #[test]
    fn list_suggested_next_steps_descriptor_is_surface_read_and_hidden_from_mcp() {
        let registry = AbilityRegistry::global_checked().expect("registry builds");
        let descriptor = registry
            .iter_all()
            .find(|descriptor| descriptor.name == LIST_SUGGESTED_NEXT_STEPS_ABILITY_NAME)
            .expect("list_suggested_next_steps ability is registered");

        assert_eq!(descriptor.category, AbilityCategory::Read);
        assert!(descriptor.policy.allowed_actors.contains(&ActorKind::User));
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
        assert_eq!(
            descriptor.policy.required_scopes,
            &[LIST_SUGGESTED_NEXT_STEPS_SCOPE]
        );
        assert_eq!(descriptor.policy.mcp_exposure, McpExposure::None);
        assert!(!descriptor.policy.may_publish);
        assert!(!descriptor.policy.client_side_executable);
        assert!(descriptor.mutates.is_empty());
    }

    #[tokio::test]
    async fn list_suggested_next_steps_per_actor_dry_run_matches_allowlist() {
        ScopeSet::set_allowlist_for_tests([SurfaceScope::new(LIST_SUGGESTED_NEXT_STEPS_SCOPE)]);
        let reader = Arc::new(CapturingSuggestedNextStepsReader::default());

        for actor in [Actor::User, Actor::System, surface_actor()] {
            invoke_list_suggested_next_steps(reader.clone(), actor)
                .await
                .expect("allowed actor succeeds");
        }

        for actor in [Actor::Agent, Actor::Admin, mcp_actor()] {
            let error = invoke_list_suggested_next_steps(reader.clone(), actor)
                .await
                .expect_err("disallowed actor is denied");
            assert_eq!(error.kind, AbilityErrorKind::Capability);
        }

        let requests = reader.requests.lock().expect("requests lock");
        assert_eq!(requests.len(), 3);
        assert_eq!(requests[0].1, ActorKind::User);
        assert_eq!(requests[1].1, ActorKind::System);
        assert_eq!(requests[2].1, ActorKind::SurfaceClient);
    }

    #[tokio::test]
    async fn list_suggested_next_steps_forwards_subject_filter_placeholder() {
        ScopeSet::set_allowlist_for_tests([SurfaceScope::new(LIST_SUGGESTED_NEXT_STEPS_SCOPE)]);
        let reader = Arc::new(CapturingSuggestedNextStepsReader::default());
        invoke_list_suggested_next_steps_with_payload(
            reader.clone(),
            Actor::User,
            json!({
                "schemaVersion": LIST_SUGGESTED_NEXT_STEPS_SCHEMA_VERSION,
                "subject": { "account": "acct-1" },
                "surface": "entity_detail",
                "maxItems": 3
            }),
        )
        .await
        .expect("read succeeds");

        let requests = reader.requests.lock().expect("requests lock");
        assert_eq!(requests.len(), 1);
        assert_eq!(
            requests[0].0.subject,
            Some(SubjectRef::Account("acct-1".to_string()))
        );
        assert_eq!(requests[0].0.max_items, Some(3));
    }

    #[tokio::test]
    async fn list_suggested_next_steps_schema_mismatch_is_validation_error() {
        let reader = Arc::new(CapturingSuggestedNextStepsReader::default());
        let error = invoke_list_suggested_next_steps_with_payload(
            reader.clone(),
            Actor::User,
            json!({
                "schemaVersion": 99,
                "subject": null,
                "surface": "entity_detail"
            }),
        )
        .await
        .expect_err("schema mismatch rejected");

        assert_eq!(error.kind, AbilityErrorKind::Validation);
        assert!(reader.requests.lock().expect("requests lock").is_empty());
    }

    #[tokio::test]
    async fn user_invocation_uses_reader_and_returns_privacy_safe_payload() {
        let reader = Arc::new(CapturingSalienceReader::default());
        let response = invoke(reader.clone(), Actor::User)
            .await
            .expect("user read succeeds");

        assert_eq!(response["data"]["claimId"], "claim-1");
        assert_eq!(response["data"]["persistence"]["kind"], "preview");
        assert_eq!(
            response["data"]["salience"]["factors"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            response["data"]["salience"]["factors"][0]["rationale"]["kind"],
            "trust"
        );
        let requests = reader.requests.lock().expect("requests lock");
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].actor, ActorKind::User);
    }

    #[tokio::test]
    async fn user_cannot_distinguish_hidden_claims_from_missing_claims() {
        let hidden = invoke(
            Arc::new(ErroringSalienceReader {
                error: SalienceReadError::ClaimNotVisible("claim-1".to_string()),
            }),
            Actor::User,
        )
        .await
        .expect_err("hidden claim returns an external not-found error");
        let missing = invoke(
            Arc::new(ErroringSalienceReader {
                error: SalienceReadError::ClaimNotFound("claim-1".to_string()),
            }),
            Actor::User,
        )
        .await
        .expect_err("missing claim returns not-found");

        assert_eq!(hidden.kind, missing.kind);
        assert_eq!(hidden.message, missing.message);
        assert_eq!(hidden.message, "claim_not_found: claim-1");
    }

    #[tokio::test]
    async fn mcp_actor_is_denied_before_reader() {
        let reader = Arc::new(CapturingSalienceReader::default());
        let err = invoke(
            reader.clone(),
            Actor::McpClient {
                client_id: McpClientId::new("client-1"),
                conversation_handle: Some(OpaqueConversationHandle::new("conv-1")),
            },
        )
        .await
        .expect_err("mcp actor is denied");

        assert_eq!(err.kind, AbilityErrorKind::Capability);
        assert!(reader.requests.lock().expect("requests lock").is_empty());
    }

    #[tokio::test]
    async fn surface_actor_is_denied_before_reader() {
        ScopeSet::set_allowlist_for_tests([SurfaceScope::new(SCORE_SALIENCE_SCOPE)]);
        let scopes = ScopeSet::new([SurfaceScope::new(SCORE_SALIENCE_SCOPE)]).expect("scope set");
        let reader = Arc::new(CapturingSalienceReader::default());
        let err = invoke(
            reader.clone(),
            Actor::SurfaceClient {
                instance: SurfaceClientId::new("surface-1"),
                scopes,
            },
        )
        .await
        .expect_err("surface actor is denied");

        assert_eq!(err.kind, AbilityErrorKind::Capability);
        assert!(reader.requests.lock().expect("requests lock").is_empty());
    }

    async fn invoke<R>(reader: Arc<R>, actor: Actor) -> Result<serde_json::Value, AbilityError>
    where
        R: SalienceReadHandle + 'static,
    {
        let registry = AbilityRegistry::global_checked().expect("registry builds");
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 26, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(7);
        let external = ExternalClients::default();
        let services = ServiceContext::new_live(&clock, &rng, &external)
            .with_actor("test")
            .with_salience_reader(reader);
        let provider = ReplayProvider::new(std::collections::HashMap::new());
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
                SCORE_SALIENCE_ABILITY_NAME,
                json!({
                    "schemaVersion": SCORE_SALIENCE_SCHEMA_VERSION,
                    "claimId": "claim-1"
                }),
            )
            .await
    }

    async fn invoke_list_suggested_next_steps<R>(
        reader: Arc<R>,
        actor: Actor,
    ) -> Result<serde_json::Value, AbilityError>
    where
        R: SuggestedNextStepsReadHandle + 'static,
    {
        invoke_list_suggested_next_steps_with_payload(
            reader,
            actor,
            json!({
                "schemaVersion": LIST_SUGGESTED_NEXT_STEPS_SCHEMA_VERSION,
                "subject": null,
                "surface": "entity_detail",
                "maxItems": 5
            }),
        )
        .await
    }

    async fn invoke_list_suggested_next_steps_with_payload<R>(
        reader: Arc<R>,
        actor: Actor,
        payload: serde_json::Value,
    ) -> Result<serde_json::Value, AbilityError>
    where
        R: SuggestedNextStepsReadHandle + 'static,
    {
        let registry = AbilityRegistry::global_checked().expect("registry builds");
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 26, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(7);
        let external = ExternalClients::default();
        let services = ServiceContext::new_live(&clock, &rng, &external)
            .with_actor("test")
            .with_suggested_next_steps_reader(reader);
        let provider = ReplayProvider::new(std::collections::HashMap::new());
        let ctx = crate::abilities::AbilityContext::new(
            &services,
            &provider,
            &NOOP_ABILITY_TRACER,
            actor,
            None,
            ClaimDismissalSurface::Eval,
        );
        registry
            .invoke_by_name_json(&ctx, LIST_SUGGESTED_NEXT_STEPS_ABILITY_NAME, payload)
            .await
    }

    fn surface_actor() -> Actor {
        let scopes =
            ScopeSet::new([SurfaceScope::new(LIST_SUGGESTED_NEXT_STEPS_SCOPE)]).expect("scope set");
        Actor::SurfaceClient {
            instance: SurfaceClientId::new("surface-1"),
            scopes,
        }
    }

    fn mcp_actor() -> Actor {
        Actor::McpClient {
            client_id: McpClientId::new("client-1"),
            conversation_handle: Some(OpaqueConversationHandle::new("conv-1")),
        }
    }

    fn response_fixture() -> ScoreSalienceResponse {
        ScoreSalienceResponse {
            schema_version: SCORE_SALIENCE_SCHEMA_VERSION,
            claim_id: ClaimId("claim-1".to_string()),
            computed_at: "2026-05-26T12:00:00Z".to_string(),
            persistence: SaliencePersistence::Preview,
            salience: SalienceScore {
                total: 0.9,
                factors: vec![SalienceFactor {
                    kind: SalienceFactorKind::Trust,
                    value: Some(0.9),
                    weight: 0.1,
                    rationale: FactorRationale::Trust {
                        trust_band: TrustBand::LikelyCurrent,
                    },
                }],
            },
        }
    }

    fn suggested_next_steps_response_fixture() -> ListSuggestedNextStepsResponse {
        ListSuggestedNextStepsResponse {
            schema_version: LIST_SUGGESTED_NEXT_STEPS_SCHEMA_VERSION,
            items: Vec::new(),
            generated_at: Utc.with_ymd_and_hms(2026, 5, 26, 12, 0, 0).unwrap(),
        }
    }
}
