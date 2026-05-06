use std::sync::Arc;

use tauri::State;

use crate::abilities::provenance::{validate_serialized_subject_ownership, OwnershipPolicy};
use crate::abilities::AbilityRegistry;
use crate::bridges::tauri::TauriAbilityBridge;
use crate::bridges::{AbilityResponseJson, BridgeSurfaceError, ConfirmationToken};
use crate::state::AppState;

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
    let response = TauriAbilityBridge::new(registry)
        .invoke(
            state.inner().as_ref(),
            &ability_name,
            input_json,
            dry_run,
            confirmation.as_ref(),
        )
        .await?;
    validate_serialized_subject_ownership(
        response.data.clone(),
        response.rendered_provenance.value.clone(),
        response.diagnostics.clone(),
        &[],
        OwnershipPolicy::confident(),
    )?;
    Ok(response)
}
