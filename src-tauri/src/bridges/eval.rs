use crate::abilities::AbilityRegistry;
use crate::abilities::AbilityTracer;
use crate::bridges::types::{invoke_registry_json, surface_error};
use crate::bridges::{
    AbilityResponseJson, BridgeActor, BridgeSurface, BridgeSurfaceError, InvocationContext,
};
use crate::intelligence::provider::IntelligenceProvider;
use crate::services::context::{ExecutionMode, ServiceContext};

pub trait EvalFixtureServices: Send + Sync {
    fn service_context(&self) -> ServiceContext<'_>;
}

pub struct EvalAbilityDeps<'deps> {
    pub fixture_services: &'deps dyn EvalFixtureServices,
    pub provider: &'deps dyn IntelligenceProvider,
    pub tracer: &'deps dyn AbilityTracer,
}

pub struct EvalAbilityBridge<'registry, 'deps> {
    registry: &'registry AbilityRegistry,
    deps: EvalAbilityDeps<'deps>,
}

impl<'registry, 'deps> EvalAbilityBridge<'registry, 'deps> {
    pub fn new(registry: &'registry AbilityRegistry, deps: EvalAbilityDeps<'deps>) -> Self {
        Self { registry, deps }
    }

    pub async fn invoke_json(
        &self,
        ability_name: &str,
        input_json: serde_json::Value,
        dry_run: bool,
    ) -> Result<AbilityResponseJson, BridgeSurfaceError> {
        let services = self.fixture_services().service_context();
        let invocation = InvocationContext {
            actor: BridgeActor::System,
            mode: ExecutionMode::Evaluate,
            surface: BridgeSurface::Eval,
            claim_dismissal_surface: crate::services::context::ClaimDismissalSurface::Eval,
            dry_run,
            confirmation: None,
            confirmation_store: None,
        };

        invoke_registry_json(
            self.registry,
            &services,
            self.deps.provider,
            self.deps.tracer,
            invocation,
            ability_name,
            input_json,
        )
        .await
        .map_err(surface_error)
    }

    fn fixture_services(&self) -> &dyn EvalFixtureServices {
        self.deps.fixture_services
    }
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    use async_trait::async_trait;
    use chrono::TimeZone;
    use serde_json::json;

    use super::*;
    use crate::abilities::registry::{AbilityPolicy, McpExposure, SignalPolicy};
    use crate::abilities::SpanHandle;
    use crate::abilities::{
        AbilityCategory, AbilityContext, AbilityDescriptor, AbilityError, ActorKind,
    };
    use crate::intelligence::provider::{
        Completion, ModelName, ModelTier, PromptInput, ProviderError, ProviderKind, ReplayProvider,
    };
    use crate::services::context::{FixedClock, SeedableRng};

    const SYSTEM_ACTORS: &[ActorKind] = &[ActorKind::System];
    const EVALUATE_MODES: &[ExecutionMode] = &[ExecutionMode::Evaluate];
    static ERASED_INVOCATION_COUNT: AtomicUsize = AtomicUsize::new(0);

    type ErasedFuture<'a> =
        Pin<Box<dyn Future<Output = Result<serde_json::Value, AbilityError>> + Send + 'a>>;

    struct FixtureServices {
        clock: FixedClock,
        rng: SeedableRng,
    }

    impl FixtureServices {
        fn new() -> Self {
            Self {
                clock: FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 4, 12, 0, 0).unwrap()),
                rng: SeedableRng::new(217),
            }
        }
    }

    impl EvalFixtureServices for FixtureServices {
        fn service_context(&self) -> ServiceContext<'_> {
            ServiceContext::new_evaluate_default(&self.clock, &self.rng).with_actor("eval_fixture")
        }
    }

    struct FixtureProvider;

    #[async_trait]
    impl IntelligenceProvider for FixtureProvider {
        async fn complete(
            &self,
            _prompt: PromptInput,
            _tier: ModelTier,
        ) -> Result<Completion, ProviderError> {
            Ok(Completion::default())
        }

        fn provider_kind(&self) -> ProviderKind {
            ProviderKind::Other("fixture-provider")
        }

        fn current_model(&self, _tier: ModelTier) -> ModelName {
            ModelName::new("fixture-model")
        }
    }

    #[derive(Default)]
    struct FixtureTracer {
        events: Mutex<Vec<String>>,
    }

    impl AbilityTracer for FixtureTracer {
        fn start_span(&self, name: &str) -> SpanHandle {
            self.events.lock().unwrap().push(format!("span:{name}"));
            SpanHandle { id: 217 }
        }

        fn record_event(&self, span: &SpanHandle, name: &str, _fields: serde_json::Value) {
            self.events
                .lock()
                .unwrap()
                .push(format!("event:{}:{name}", span.id));
        }
    }

    fn context_echo_erased<'a>(
        ctx: &'a AbilityContext<'a>,
        input: serde_json::Value,
    ) -> ErasedFuture<'a> {
        Box::pin(async move {
            let span = ctx.tracer.start_span("eval_context_echo");
            ctx.tracer
                .record_event(&span, "ability_context_seen", json!({ "surface": "eval" }));
            Ok(envelope_json(
                ctx,
                json!({
                    "input": input,
                    "actor": format!("{:?}", ctx.actor),
                    "mode": ctx.mode().as_str(),
                    "service_actor": ctx.services().actor,
                    "provider_kind": ctx.provider.provider_kind().as_str(),
                    "provider_model": ctx.provider.current_model(ModelTier::Synthesis).as_str(),
                }),
            ))
        })
    }

    fn replay_provider_erased<'a>(
        ctx: &'a AbilityContext<'a>,
        _input: serde_json::Value,
    ) -> ErasedFuture<'a> {
        Box::pin(async move {
            let completion = ctx
                .provider
                .complete(PromptInput::new("eval replay prompt"), ModelTier::Synthesis)
                .await
                .map_err(|error| AbilityError {
                    kind: crate::abilities::AbilityErrorKind::HardError(error.to_string()),
                    message: "replay provider fixture missing".to_string(),
                })?;
            Ok(envelope_json(
                ctx,
                json!({
                    "completion": completion.text,
                    "provider_kind": ctx.provider.provider_kind().as_str(),
                    "provider_model": ctx.provider.current_model(ModelTier::Synthesis).as_str(),
                    "mode": ctx.mode().as_str(),
                }),
            ))
        })
    }

    fn counted_erased_registry_path<'a>(
        ctx: &'a AbilityContext<'a>,
        input: serde_json::Value,
    ) -> ErasedFuture<'a> {
        Box::pin(async move {
            ERASED_INVOCATION_COUNT.fetch_add(1, Ordering::SeqCst);
            Ok(envelope_json(
                ctx,
                json!({
                    "input": input,
                    "count": ERASED_INVOCATION_COUNT.load(Ordering::SeqCst),
                }),
            ))
        })
    }

    fn envelope_json(ctx: &AbilityContext<'_>, data: serde_json::Value) -> serde_json::Value {
        json!({
            "data": data,
            "ability_version": { "major": 1, "minor": 0 },
            "diagnostics": { "warnings": [] },
            "provenance": {
                "invocation_id": "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb",
                "ability_name": "eval_fixture",
                "ability_version": { "major": 1, "minor": 0 },
                "ability_schema_version": 1,
                "actor": format!("{:?}", ctx.actor),
                "mode": ctx.mode().as_str(),
                "warnings": []
            }
        })
    }

    fn descriptor() -> AbilityDescriptor {
        AbilityDescriptor {
            name: "eval_context_echo",
            version: "1.0.0",
            schema_version: 1,
            category: AbilityCategory::Read,
            policy: AbilityPolicy {
                allowed_actors: SYSTEM_ACTORS,
                allowed_modes: EVALUATE_MODES,
                requires_confirmation: false,
                may_publish: false,
                required_scopes: &[],
                mcp_exposure: McpExposure::None,
                client_side_executable: false,
                rate_limit: None,
            },
            composes: &[],
            mutates: &[],
            experimental: false,
            registered_at: None,
            signal_policy: SignalPolicy {
                emits_on_output_change: &[],
                coalesce: false,
            },
            invoke_erased: context_echo_erased,
            input_schema: closed_object_schema,
            output_schema: closed_object_schema,
        }
    }

    fn descriptor_with_invoke(
        mut descriptor: AbilityDescriptor,
        invoke_erased: for<'a> fn(&'a AbilityContext<'a>, serde_json::Value) -> ErasedFuture<'a>,
    ) -> AbilityDescriptor {
        descriptor.invoke_erased = invoke_erased;
        descriptor
    }

    fn registry() -> AbilityRegistry {
        AbilityRegistry::from_descriptors_checked(vec![descriptor()]).unwrap()
    }

    fn closed_object_schema() -> serde_json::Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "value": {},
                "subject": { "type": "string" }
            }
        })
    }

    #[tokio::test]
    async fn eval_bridge_constructs_evaluate_context_with_fixture_services_provider_tracer() {
        let registry = registry();
        let fixture_services = FixtureServices::new();
        let provider = FixtureProvider;
        let tracer = FixtureTracer::default();
        let bridge = EvalAbilityBridge::new(
            &registry,
            EvalAbilityDeps {
                fixture_services: &fixture_services,
                provider: &provider,
                tracer: &tracer,
            },
        );

        let response = bridge
            .invoke_json("eval_context_echo", json!({ "value": 217 }), true)
            .await
            .unwrap();

        assert_eq!(response.data["mode"], "evaluate");
        assert_eq!(response.data["actor"], "System");
        assert_eq!(response.data["service_actor"], "eval_fixture");
        assert_eq!(response.data["provider_kind"], "fixture-provider");
        assert_eq!(response.data["provider_model"], "fixture-model");

        let events = tracer.events.lock().unwrap();
        assert_eq!(
            events.as_slice(),
            ["span:eval_context_echo", "event:217:ability_context_seen"]
        );
    }

    #[tokio::test]
    async fn eval_bridge_uses_replay_provider_and_never_live_provider() {
        let registry = AbilityRegistry::from_descriptors_checked(vec![descriptor_with_invoke(
            descriptor(),
            replay_provider_erased,
        )])
        .unwrap();
        let fixture_services = FixtureServices::new();
        let provider =
            ReplayProvider::from_prompt_pairs([("eval replay prompt", "fixture completion")]);
        let tracer = FixtureTracer::default();
        let bridge = EvalAbilityBridge::new(
            &registry,
            EvalAbilityDeps {
                fixture_services: &fixture_services,
                provider: &provider,
                tracer: &tracer,
            },
        );

        let response = bridge
            .invoke_json("eval_context_echo", json!({}), false)
            .await
            .unwrap();

        assert_eq!(response.data["completion"], "fixture completion");
        assert_eq!(response.data["provider_kind"], "replay");
        assert_eq!(response.data["provider_model"], "replay");
        assert_eq!(response.data["mode"], "evaluate");
    }

    #[tokio::test]
    async fn eval_bridge_invokes_same_erased_registry_path_as_runtime_bridges() {
        ERASED_INVOCATION_COUNT.store(0, Ordering::SeqCst);
        let registry = AbilityRegistry::from_descriptors_checked(vec![descriptor_with_invoke(
            descriptor(),
            counted_erased_registry_path,
        )])
        .unwrap();
        let fixture_services = FixtureServices::new();
        let provider = FixtureProvider;
        let tracer = FixtureTracer::default();
        let bridge = EvalAbilityBridge::new(
            &registry,
            EvalAbilityDeps {
                fixture_services: &fixture_services,
                provider: &provider,
                tracer: &tracer,
            },
        );

        let response = bridge
            .invoke_json("eval_context_echo", json!({ "value": 217 }), false)
            .await
            .unwrap();

        assert_eq!(ERASED_INVOCATION_COUNT.load(Ordering::SeqCst), 1);
        assert_eq!(response.data["count"], 1);
        assert_eq!(response.data["input"]["value"], 217);
    }

    #[tokio::test]
    async fn eval_bridge_runs_without_tauri_runtime() {
        let registry = registry();
        let fixture_services = FixtureServices::new();
        let provider = FixtureProvider;
        let tracer = FixtureTracer::default();
        let bridge = EvalAbilityBridge::new(
            &registry,
            EvalAbilityDeps {
                fixture_services: &fixture_services,
                provider: &provider,
                tracer: &tracer,
            },
        );

        let response = bridge
            .invoke_json("eval_context_echo", json!({}), false)
            .await
            .unwrap();

        assert_eq!(response.ability_name, "eval_context_echo");
        assert_eq!(response.rendered_provenance.surface, BridgeSurface::Eval);
    }
}
