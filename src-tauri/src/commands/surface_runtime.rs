use std::sync::Arc;

use tauri::State;

use crate::state::AppState;
use crate::surface_runtime::SurfaceEndpointPairingStatus;

#[tauri::command]
pub fn get_surface_runtime_pairing_status(
    state: State<'_, Arc<AppState>>,
) -> SurfaceEndpointPairingStatus {
    state.surface_runtime_endpoint.pairing_status()
}
