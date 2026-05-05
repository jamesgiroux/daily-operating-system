use crate::abilities::AbilityRegistry;
use crate::abilities::NOOP_ABILITY_TRACER;
use crate::bridges::types::{invoke_registry_json, provider_from_context_snapshot, surface_error};
use crate::bridges::{
    AbilityResponseJson, BridgeActor, BridgeSurface, BridgeSurfaceError, ConfirmationToken,
    InvocationContext,
};
use crate::services::context::ExecutionMode;
use crate::state::AppState;

#[derive(Debug, Clone, Copy)]
pub struct ScheduledWorkerMarker {
    _private: (),
}

impl ScheduledWorkerMarker {
    pub fn scheduled_worker() -> Self {
        Self { _private: () }
    }
}

pub struct WorkerAbilityBridge<'registry> {
    registry: &'registry AbilityRegistry,
}

impl<'registry> WorkerAbilityBridge<'registry> {
    pub fn new(registry: &'registry AbilityRegistry) -> Self {
        Self { registry }
    }

    pub async fn invoke(
        &self,
        state: &AppState,
        ability_name: &str,
        input_json: serde_json::Value,
        dry_run: bool,
        confirmation: Option<&ConfirmationToken>,
        scheduled_worker: Option<&ScheduledWorkerMarker>,
    ) -> Result<AbilityResponseJson, BridgeSurfaceError> {
        if requests_global_blast_radius(&input_json) && scheduled_worker.is_none() {
            return Err(BridgeSurfaceError::AbilityUnavailable);
        }

        let snapshot = state.context_snapshot();
        let provider = provider_from_context_snapshot(&snapshot);
        let services = state.live_service_context().with_actor("scheduled_worker");
        let invocation = InvocationContext {
            actor: BridgeActor::System,
            mode: ExecutionMode::Live,
            surface: BridgeSurface::Worker,
            dry_run,
            confirmation,
            confirmation_store: None,
        };

        invoke_registry_json(
            self.registry,
            &services,
            provider,
            &NOOP_ABILITY_TRACER,
            invocation,
            ability_name,
            input_json,
        )
        .await
        .map_err(surface_error)
    }
}

fn requests_global_blast_radius(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Object(object) => {
            object
                .get("blast_radius")
                .or_else(|| object.get("blastRadius"))
                .and_then(serde_json::Value::as_str)
                .is_some_and(|blast_radius| blast_radius == "global")
                || object.values().any(requests_global_blast_radius)
        }
        serde_json::Value::Array(values) => values.iter().any(requests_global_blast_radius),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::pin::Pin;

    use serde_json::json;

    use super::*;
    use crate::abilities::registry::{AbilityPolicy, SignalPolicy};
    use crate::abilities::{
        AbilityCategory, AbilityContext, AbilityDescriptor, AbilityError, Actor,
    };

    const SYSTEM_ACTORS: &[Actor] = &[Actor::System];
    const LIVE_MODES: &[ExecutionMode] = &[ExecutionMode::Live];

    type ErasedFuture<'a> =
        Pin<Box<dyn Future<Output = Result<serde_json::Value, AbilityError>> + Send + 'a>>;

    fn success_erased<'a>(
        ctx: &'a AbilityContext<'a>,
        input: serde_json::Value,
    ) -> ErasedFuture<'a> {
        Box::pin(async move {
            Ok(json!({
                "data": {
                    "input": input,
                    "actor": format!("{:?}", ctx.actor),
                    "mode": ctx.mode().as_str(),
                    "service_actor": ctx.services().actor
                },
                "ability_version": { "major": 1, "minor": 0 },
                "diagnostics": { "warnings": [] },
                "provenance": {
                    "invocation_id": "cccccccc-cccc-4ccc-8ccc-cccccccccccc",
                    "ability_name": "worker_fixture",
                    "ability_version": { "major": 1, "minor": 0 },
                    "ability_schema_version": 1,
                    "actor": format!("{:?}", ctx.actor),
                    "mode": ctx.mode().as_str(),
                    "warnings": []
                }
            }))
        })
    }

    fn descriptor(name: &'static str) -> AbilityDescriptor {
        AbilityDescriptor {
            name,
            version: "1.0.0",
            schema_version: 1,
            category: AbilityCategory::Maintenance,
            policy: AbilityPolicy {
                allowed_actors: SYSTEM_ACTORS,
                allowed_modes: LIVE_MODES,
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
            invoke_erased: success_erased,
            input_schema: worker_schema,
            output_schema: worker_schema,
        }
    }

    fn worker_schema() -> serde_json::Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "blast_radius": { "type": "string" },
                "blastRadius": { "type": "string" }
            }
        })
    }

    fn registry() -> AbilityRegistry {
        AbilityRegistry::from_descriptors_checked(vec![descriptor("worker_maintenance")]).unwrap()
    }

    #[tokio::test]
    async fn worker_bridge_invokes_maintenance_as_system_live() {
        let registry = registry();
        let state = AppState::new();
        let bridge = WorkerAbilityBridge::new(&registry);
        let marker = ScheduledWorkerMarker::scheduled_worker();

        let response = bridge
            .invoke(
                &state,
                "worker_maintenance",
                json!({}),
                false,
                None,
                Some(&marker),
            )
            .await
            .unwrap();

        assert_eq!(response.data["actor"], "System");
        assert_eq!(response.data["mode"], "live");
        assert_eq!(response.data["service_actor"], "scheduled_worker");
        assert_eq!(response.rendered_provenance.surface, BridgeSurface::Worker);
    }

    #[tokio::test]
    async fn worker_bridge_rejects_global_without_scheduled_worker_marker() {
        let registry = registry();
        let state = AppState::new();
        let bridge = WorkerAbilityBridge::new(&registry);

        let err = bridge
            .invoke(
                &state,
                "worker_maintenance",
                json!({ "blast_radius": "global" }),
                false,
                None,
                None,
            )
            .await
            .unwrap_err();

        assert_eq!(err, BridgeSurfaceError::AbilityUnavailable);
    }
}
