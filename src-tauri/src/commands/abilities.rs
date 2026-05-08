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
use crate::bridges::tauri::TauriAbilityBridge;
use crate::bridges::{AbilityResponseJson, BridgeSurfaceError, ConfirmationToken};
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
    let response = TauriAbilityBridge::new(registry)
        .invoke(
            state.inner().as_ref(),
            &ability_name,
            input_json,
            dry_run,
            confirmation.as_ref(),
        )
        .await?;
    let policy = build_ownership_policy_for_invocation(
        ability_meta,
        &input_for_policy,
        &response.rendered_provenance.value,
    )?;
    validate_serialized_subject_ownership(
        response.data.clone(),
        response.rendered_provenance.value.clone(),
        response.diagnostics.clone(),
        &[],
        policy,
    )?;
    Ok(response)
}
