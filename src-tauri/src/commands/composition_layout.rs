#![allow(
    clippy::let_underscore_must_use,
    reason = "tauri::command macro emits internal Result glue that discards generated metadata"
)]

use std::sync::Arc;

use tauri::State;

use crate::services::composition_layout::{
    get_layout_overlay, reset_layout_overlay, save_layout_overlay, LayoutOverlayKey,
    LayoutOverlayResponse, SaveLayoutOverlayRequest,
};
use crate::state::AppState;

#[tauri::command]
pub async fn get_composition_layout_overlay(
    state: State<'_, Arc<AppState>>,
    key: LayoutOverlayKey,
) -> Result<LayoutOverlayResponse, String> {
    get_layout_overlay(state.inner().clone(), key).await
}

#[tauri::command]
pub async fn save_composition_layout_overlay(
    state: State<'_, Arc<AppState>>,
    request: SaveLayoutOverlayRequest,
) -> Result<LayoutOverlayResponse, String> {
    let ctx = state.live_service_context();
    save_layout_overlay(&ctx, state.inner().clone(), request).await
}

#[tauri::command]
pub async fn reset_composition_layout_overlay(
    state: State<'_, Arc<AppState>>,
    key: LayoutOverlayKey,
) -> Result<LayoutOverlayResponse, String> {
    let ctx = state.live_service_context();
    reset_layout_overlay(&ctx, state.inner().clone(), key).await
}
