#![allow(
    clippy::let_underscore_must_use,
    reason = "tauri::command macro emits internal Result glue that discards generated metadata"
)]

use std::sync::Arc;

use tauri::{Emitter, State};

use crate::services::settings::AggregateTelemetryStatus;
use crate::state::AppState;

#[tauri::command]
pub fn get_aggregate_telemetry_status(state: State<'_, Arc<AppState>>) -> AggregateTelemetryStatus {
    crate::services::settings::aggregate_telemetry_status(state.inner().as_ref())
}

#[tauri::command]
pub fn set_aggregate_telemetry_enabled(
    state: State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
    enabled: bool,
) -> Result<AggregateTelemetryStatus, String> {
    let ctx = state.live_service_context();
    let status = crate::services::settings::set_aggregate_telemetry_enabled(
        &ctx,
        enabled,
        state.inner().as_ref(),
    )?;
    emit_telemetry_status_changed(&app_handle);
    Ok(status)
}

#[tauri::command]
pub fn dismiss_aggregate_telemetry_splash(
    state: State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<AggregateTelemetryStatus, String> {
    let ctx = state.live_service_context();
    let status = crate::services::settings::dismiss_aggregate_telemetry_splash(
        &ctx,
        state.inner().as_ref(),
    )?;
    emit_telemetry_status_changed(&app_handle);
    Ok(status)
}

fn emit_telemetry_status_changed(app_handle: &tauri::AppHandle) {
    #[allow(
        clippy::let_underscore_must_use,
        reason = "best-effort UI invalidation event"
    )]
    let _ = app_handle.emit("telemetry-status-updated", ());
    #[allow(
        clippy::let_underscore_must_use,
        reason = "best-effort config invalidation event"
    )]
    let _ = app_handle.emit("config-updated", ());
}
