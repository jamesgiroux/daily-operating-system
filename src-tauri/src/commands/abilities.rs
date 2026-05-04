use std::sync::Arc;

use tauri::State;

use crate::abilities::{AbilityRegistry, ConfirmationToken};
use crate::bridges::tauri::TauriAbilityBridge;
use crate::bridges::{AbilityResponseJson, BridgeSurfaceError};
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
    TauriAbilityBridge::new(registry)
        .invoke(
            state.inner().as_ref(),
            &ability_name,
            input_json,
            dry_run,
            confirmation.as_ref(),
        )
        .await
}
