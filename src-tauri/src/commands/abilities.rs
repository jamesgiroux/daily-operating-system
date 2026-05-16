#![allow(
    clippy::let_underscore_must_use,
    reason = "tauri::command macro emits internal Result glue that discards generated metadata"
)]

use std::sync::Arc;

use tauri::State;

use crate::abilities::provenance::{
    build_ownership_policy_for_invocation, validate_serialized_subject_ownership,
};
use crate::abilities::{AbilityRegistry, Actor};
use crate::bridges::tauri::{
    parse_tauri_claim_dismissal_surface, TauriAbilityBridge, TauriInvokeContext,
};
use crate::bridges::{AbilityResponseJson, BridgeSurface, BridgeSurfaceError, ConfirmationToken};
use crate::observability::aggregate_metric::{MetricDimensions, MetricValue, Outcome};
use crate::state::AppState;

#[allow(
    clippy::let_underscore_must_use,
    reason = "tauri::command macro emits internal Result glue that discards generated metadata"
)]
#[tauri::command]
pub async fn invoke_ability(
    state: State<'_, Arc<AppState>>,
    ability_name: String,
    input_json: serde_json::Value,
    render_surface: String,
    dry_run: bool,
    confirmation: Option<ConfirmationToken>,
) -> Result<AbilityResponseJson, BridgeSurfaceError> {
    if state.lock_state.lock().is_locked {
        return Err(BridgeSurfaceError::AbilityUnavailable);
    }

    let registry =
        AbilityRegistry::global_checked().map_err(|_| BridgeSurfaceError::AbilityUnavailable)?;
    let ability_meta = registry
        .iter_for(Actor::User)
        .find(|descriptor| descriptor.name == ability_name)
        .ok_or(BridgeSurfaceError::AbilityUnavailable)?;
    let input_for_policy = input_json.clone();
    let claim_dismissal_surface = parse_tauri_claim_dismissal_surface(&render_surface)?;
    let response = match TauriAbilityBridge::new(registry)
        .invoke(
            state.inner().as_ref(),
            &ability_name,
            input_json,
            TauriInvokeContext::new(
                Actor::User,
                BridgeSurface::TauriApp,
                claim_dismissal_surface,
                dry_run,
                confirmation.as_ref(),
            ),
        )
        .await
    {
        Ok(response) => response,
        Err(err) => {
            record_ability_invocation_metric(
                state.inner().as_ref(),
                ability_meta,
                Outcome::Failure,
            );
            return Err(err);
        }
    };
    let policy = match build_ownership_policy_for_invocation(
        ability_meta,
        &input_for_policy,
        response.raw_provenance_value(),
    ) {
        Ok(policy) => policy,
        Err(err) => {
            record_ability_invocation_metric(
                state.inner().as_ref(),
                ability_meta,
                Outcome::Failure,
            );
            return Err(err.into());
        }
    };
    if let Err(err) = validate_serialized_subject_ownership(
        response.data.clone(),
        response.raw_provenance_value().clone(),
        response.diagnostics.clone(),
        &[],
        policy,
    ) {
        record_ability_invocation_metric(state.inner().as_ref(), ability_meta, Outcome::Failure);
        return Err(err.into());
    }
    record_ability_invocation_metric(state.inner().as_ref(), ability_meta, Outcome::Success);
    Ok(response)
}

fn record_ability_invocation_metric(
    state: &AppState,
    ability_meta: &crate::abilities::AbilityDescriptor,
    outcome: Outcome,
) {
    crate::observability::aggregate_metric::emit_aggregate_metric(
        state,
        crate::aggregate_metric_name!("ability_invocation_count"),
        MetricValue::Count(1),
        MetricDimensions::default()
            .ability(ability_meta.name, ability_meta.version)
            .outcome(outcome),
    );
}
