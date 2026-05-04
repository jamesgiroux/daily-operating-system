use crate::abilities::AbilityRegistry;
use crate::bridges::types::{invoke_registry_json, surface_error};
use crate::bridges::{
    AbilityResponseJson, BridgeActor, BridgeSurface, BridgeSurfaceError, InvocationContext,
};
use crate::services::context::{ExecutionMode, ServiceContext};

pub trait EvalFixtureServices: Send + Sync {
    fn service_context(&self) -> ServiceContext<'_>;
}

pub trait EvalAbilityProvider: Send + Sync {
    fn provider_name(&self) -> &str;
}

pub trait EvalAbilityTracer: Send + Sync {
    fn record_invocation(&self, trace: EvalInvocationTrace);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalInvocationTrace {
    pub ability_name: String,
    pub actor: BridgeActor,
    pub mode: ExecutionMode,
    pub surface: BridgeSurface,
    pub dry_run: bool,
    pub provider_name: String,
}

pub struct EvalAbilityDeps<'deps> {
    pub fixture_services: &'deps dyn EvalFixtureServices,
    pub provider: &'deps dyn EvalAbilityProvider,
    pub tracer: &'deps dyn EvalAbilityTracer,
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
            dry_run,
            confirmation: None,
        };

        self.deps.tracer.record_invocation(EvalInvocationTrace {
            ability_name: ability_name.to_string(),
            actor: invocation.actor,
            mode: invocation.mode,
            surface: invocation.surface,
            dry_run,
            provider_name: self.deps.provider.provider_name().to_string(),
        });

        invoke_registry_json(
            self.registry,
            &services,
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
    use std::sync::Mutex;

    use chrono::TimeZone;
    use serde_json::json;

    use super::*;
    use crate::abilities::registry::{AbilityPolicy, SignalPolicy};
    use crate::abilities::{
        AbilityCategory, AbilityContext, AbilityDescriptor, AbilityError, Actor,
    };
    use crate::services::context::{ExternalClients, FixedClock, SeedableRng};

    const SYSTEM_ACTORS: &[Actor] = &[Actor::System];
    const EVALUATE_MODES: &[ExecutionMode] = &[ExecutionMode::Evaluate];

    type ErasedFuture<'a> =
        Pin<Box<dyn Future<Output = Result<serde_json::Value, AbilityError>> + Send + 'a>>;

    struct FixtureServices {
        clock: FixedClock,
        rng: SeedableRng,
        external: ExternalClients,
    }

    impl FixtureServices {
        fn new() -> Self {
            Self {
                clock: FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 4, 12, 0, 0).unwrap()),
                rng: SeedableRng::new(217),
                external: ExternalClients::default(),
            }
        }
    }

    impl EvalFixtureServices for FixtureServices {
        fn service_context(&self) -> ServiceContext<'_> {
            ServiceContext::new_evaluate(&self.clock, &self.rng, &self.external)
                .with_actor("eval_fixture")
        }
    }

    struct FixtureProvider;

    impl EvalAbilityProvider for FixtureProvider {
        fn provider_name(&self) -> &str {
            "fixture-provider"
        }
    }

    #[derive(Default)]
    struct FixtureTracer {
        traces: Mutex<Vec<EvalInvocationTrace>>,
    }

    impl EvalAbilityTracer for FixtureTracer {
        fn record_invocation(&self, trace: EvalInvocationTrace) {
            self.traces.lock().unwrap().push(trace);
        }
    }

    fn context_echo_erased<'a>(
        ctx: &'a AbilityContext<'a>,
        input: serde_json::Value,
    ) -> ErasedFuture<'a> {
        Box::pin(async move {
            Ok(envelope_json(
                ctx,
                json!({
                    "input": input,
                    "actor": format!("{:?}", ctx.actor),
                    "mode": ctx.mode().as_str(),
                    "service_actor": ctx.services().actor,
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

    fn registry() -> AbilityRegistry {
        AbilityRegistry::from_descriptors_checked(vec![descriptor()]).unwrap()
    }

    fn closed_object_schema() -> serde_json::Value {
        json!({
            "type": "object",
            "additionalProperties": false
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

        let traces = tracer.traces.lock().unwrap();
        assert_eq!(traces.len(), 1);
        assert_eq!(traces[0].ability_name, "eval_context_echo");
        assert_eq!(traces[0].actor, BridgeActor::System);
        assert_eq!(traces[0].mode, ExecutionMode::Evaluate);
        assert_eq!(traces[0].surface, BridgeSurface::Eval);
        assert!(traces[0].dry_run);
        assert_eq!(traces[0].provider_name, "fixture-provider");
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
