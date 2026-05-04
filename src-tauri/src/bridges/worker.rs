use crate::abilities::{AbilityRegistry, ConfirmationToken};
use crate::bridges::types::{invoke_registry_json, surface_error};
use crate::bridges::{
    AbilityResponseJson, BridgeActor, BridgeSurface, BridgeSurfaceError, InvocationContext,
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

        let services = state.live_service_context().with_actor("scheduled_worker");
        let invocation = InvocationContext {
            actor: BridgeActor::System,
            mode: ExecutionMode::Live,
            surface: BridgeSurface::Worker,
            dry_run,
            confirmation,
        };

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
