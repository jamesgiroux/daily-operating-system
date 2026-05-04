use crate::abilities::{AbilityRegistry, ConfirmationToken};
use crate::bridges::types::{invoke_registry_json, surface_error};
use crate::bridges::{
    AbilityResponseJson, BridgeActor, BridgeSurface, BridgeSurfaceError, InvocationContext,
};
use crate::services::context::ExecutionMode;
use crate::state::AppState;

pub struct TauriAbilityBridge<'registry> {
    registry: &'registry AbilityRegistry,
}

impl<'registry> TauriAbilityBridge<'registry> {
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
    ) -> Result<AbilityResponseJson, BridgeSurfaceError> {
        if state.lock_state.lock().is_locked {
            return Err(BridgeSurfaceError::AbilityUnavailable);
        }

        let services = state.live_service_context().with_actor("user");
        let invocation = InvocationContext {
            actor: BridgeActor::User,
            mode: ExecutionMode::Live,
            surface: BridgeSurface::TauriApp,
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

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::pin::Pin;
    use std::time::Duration;

    use serde_json::json;
    use tokio::sync::Notify;

    use super::*;
    use crate::abilities::registry::{AbilityPolicy, SignalPolicy};
    use crate::abilities::{
        AbilityCategory, AbilityContext, AbilityDescriptor, AbilityError, Actor,
    };
    use crate::bridges::types::{BridgeRejectReason, PRE_DISPATCH_RESOLUTION_ORDER};

    const USER_ACTORS: &[Actor] = &[Actor::User];
    const ADMIN_ACTORS: &[Actor] = &[Actor::Admin];
    const USER_SYSTEM_ACTORS: &[Actor] = &[Actor::User, Actor::System];
    const LIVE_MODES: &[ExecutionMode] = &[ExecutionMode::Live];
    const EVALUATE_MODES: &[ExecutionMode] = &[ExecutionMode::Evaluate];

    static PRE_ADMISSION_STARTED: Notify = Notify::const_new();
    static PRE_ADMISSION_RELEASE: Notify = Notify::const_new();

    type ErasedFuture<'a> =
        Pin<Box<dyn Future<Output = Result<serde_json::Value, AbilityError>> + Send + 'a>>;

    fn success_erased<'a>(
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
                }),
            ))
        })
    }

    fn pre_admission_wait_erased<'a>(
        ctx: &'a AbilityContext<'a>,
        _input: serde_json::Value,
    ) -> ErasedFuture<'a> {
        Box::pin(async move {
            PRE_ADMISSION_STARTED.notify_one();
            PRE_ADMISSION_RELEASE.notified().await;
            Ok(envelope_json(
                ctx,
                json!({
                    "completed_after_lock": true,
                    "mode": ctx.mode().as_str(),
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
                "invocation_id": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
                "ability_name": "fixture",
                "ability_version": { "major": 1, "minor": 0 },
                "ability_schema_version": 1,
                "actor": format!("{:?}", ctx.actor),
                "mode": ctx.mode().as_str(),
                "warnings": []
            }
        })
    }

    fn descriptor(
        name: &'static str,
        category: AbilityCategory,
        actors: &'static [Actor],
        modes: &'static [ExecutionMode],
        invoke_erased: for<'a> fn(&'a AbilityContext<'a>, serde_json::Value) -> ErasedFuture<'a>,
    ) -> AbilityDescriptor {
        AbilityDescriptor {
            name,
            version: "1.0.0",
            schema_version: 1,
            category,
            policy: AbilityPolicy {
                allowed_actors: actors,
                allowed_modes: modes,
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
            invoke_erased,
            input_schema: closed_object_schema,
            output_schema: closed_object_schema,
        }
    }

    fn registry(descriptors: Vec<AbilityDescriptor>) -> AbilityRegistry {
        AbilityRegistry::from_descriptors_checked(descriptors).unwrap()
    }

    fn closed_object_schema() -> serde_json::Value {
        json!({
            "type": "object",
            "additionalProperties": false
        })
    }

    async fn error_bytes_for(registry: AbilityRegistry, ability_name: &'static str) -> Vec<u8> {
        let state = AppState::new();
        let bridge = TauriAbilityBridge::new(&registry);
        let err = bridge
            .invoke(&state, ability_name, json!({}), false, None)
            .await
            .unwrap_err();
        serde_json::to_vec(&err).unwrap()
    }

    #[tokio::test]
    async fn tauri_bridge_rejects_locked_app() {
        let registry = registry(vec![]);
        let state = AppState::new();
        state.lock_state.lock().is_locked = true;

        let err = TauriAbilityBridge::new(&registry)
            .invoke(&state, "missing", json!({}), false, None)
            .await
            .unwrap_err();

        assert_eq!(err, BridgeSurfaceError::AbilityUnavailable);
    }

    #[tokio::test]
    async fn tauri_bridge_documents_lock_as_pre_admission_only() {
        let registry = registry(vec![descriptor(
            "pre_admission",
            AbilityCategory::Read,
            USER_ACTORS,
            LIVE_MODES,
            pre_admission_wait_erased,
        )]);
        let state = AppState::new();
        let bridge = TauriAbilityBridge::new(&registry);

        let invoke = bridge.invoke(&state, "pre_admission", json!({}), false, None);
        let lock_after_admission = async {
            PRE_ADMISSION_STARTED.notified().await;
            state.lock_state.lock().is_locked = true;
            PRE_ADMISSION_RELEASE.notify_one();
        };

        let (response, _) = tokio::time::timeout(Duration::from_secs(2), async {
            tokio::join!(invoke, lock_after_admission)
        })
        .await
        .unwrap();

        let response = response.unwrap();
        assert_eq!(response.data["completed_after_lock"], true);
        assert!(state.lock_state.lock().is_locked);
    }

    #[tokio::test]
    async fn bridge_pre_dispatch_resolution_order_is_fixed_and_timing_independent() {
        assert_eq!(
            PRE_DISPATCH_RESOLUTION_ORDER,
            [
                BridgeRejectReason::UnknownAbility,
                BridgeRejectReason::ActorPolicy,
                BridgeRejectReason::ModePolicy,
                BridgeRejectReason::MaintenanceGate,
                BridgeRejectReason::ExperimentalGate,
            ]
        );

        let cases = [
            (
                registry(vec![descriptor(
                    "visible",
                    AbilityCategory::Read,
                    USER_ACTORS,
                    LIVE_MODES,
                    success_erased,
                )]),
                "unknown",
            ),
            (
                registry(vec![descriptor(
                    "admin_only",
                    AbilityCategory::Read,
                    ADMIN_ACTORS,
                    LIVE_MODES,
                    success_erased,
                )]),
                "admin_only",
            ),
            (
                registry(vec![descriptor(
                    "evaluate_only",
                    AbilityCategory::Read,
                    USER_ACTORS,
                    EVALUATE_MODES,
                    success_erased,
                )]),
                "evaluate_only",
            ),
            (
                registry(vec![descriptor(
                    "maintenance",
                    AbilityCategory::Maintenance,
                    USER_SYSTEM_ACTORS,
                    LIVE_MODES,
                    success_erased,
                )]),
                "maintenance",
            ),
            (
                registry(vec![descriptor(
                    "visible",
                    AbilityCategory::Read,
                    USER_ACTORS,
                    LIVE_MODES,
                    success_erased,
                )]),
                "experimental_hidden",
            ),
        ];

        let mut serialized_errors = Vec::new();
        for (registry, ability_name) in cases {
            serialized_errors.push(error_bytes_for(registry, ability_name).await);
        }

        for serialized in &serialized_errors[1..] {
            assert_eq!(serialized, &serialized_errors[0]);
        }
    }

    #[tokio::test]
    async fn bridge_unknown_ability_unauthorized_actor_maintenance_experimental_mode_all_yield_byte_equal_error(
    ) {
        let unknown = error_bytes_for(registry(vec![]), "unknown").await;
        let unauthorized_actor = error_bytes_for(
            registry(vec![descriptor(
                "admin_only",
                AbilityCategory::Read,
                ADMIN_ACTORS,
                LIVE_MODES,
                success_erased,
            )]),
            "admin_only",
        )
        .await;
        let maintenance = error_bytes_for(
            registry(vec![descriptor(
                "maintenance",
                AbilityCategory::Maintenance,
                USER_SYSTEM_ACTORS,
                LIVE_MODES,
                success_erased,
            )]),
            "maintenance",
        )
        .await;
        let experimental = error_bytes_for(registry(vec![]), "experimental_hidden").await;
        let mode_hidden = error_bytes_for(
            registry(vec![descriptor(
                "evaluate_only",
                AbilityCategory::Read,
                USER_ACTORS,
                EVALUATE_MODES,
                success_erased,
            )]),
            "evaluate_only",
        )
        .await;

        assert_eq!(unauthorized_actor, unknown);
        assert_eq!(maintenance, unknown);
        assert_eq!(experimental, unknown);
        assert_eq!(mode_hidden, unknown);
    }
}
