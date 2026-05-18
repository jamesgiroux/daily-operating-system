#![allow(
    clippy::let_underscore_must_use,
    reason = "tauri::command macro emits internal Result glue that discards generated metadata"
)]

use std::sync::Arc;

use chrono::Utc;
use tauri::State;

use crate::services::surface_pairing::{
    self, PairingCodeIssue, PairingCodeIssueInput, RevokePairingInput, SurfaceClientPairingSummary,
};
use crate::state::AppState;
use crate::surface_runtime::SurfaceEndpointPairingStatus;

#[tauri::command]
pub fn get_surface_runtime_pairing_status(
    state: State<'_, Arc<AppState>>,
) -> SurfaceEndpointPairingStatus {
    state.surface_runtime_endpoint.pairing_status()
}

#[tauri::command]
pub async fn list_surface_client_pairings(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<SurfaceClientPairingSummary>, String> {
    state
        .db_read(|db| surface_pairing::list_pairings(db).map_err(|error| error.to_string()))
        .await
}

#[tauri::command]
pub async fn create_surface_runtime_pairing_string(
    state: State<'_, Arc<AppState>>,
) -> Result<PairingCodeIssue, String> {
    let context = state.surface_runtime_endpoint.runtime_pairing_context()?;
    let input = PairingCodeIssueInput {
        runtime_anchor_id: context.runtime_anchor_id,
        endpoint_startup_id: context.startup_id.to_string(),
        bound_port: context.bound_port,
        now: Utc::now(),
    };
    let issued = state
        .db_write(move |db| {
            let clock = crate::services::context::SystemClock;
            let rng = crate::services::context::SystemRng;
            let external = crate::services::context::ExternalClients::default();
            let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &external);
            surface_pairing::issue_pairing_code(&ctx, db, input).map_err(|error| error.to_string())
        })
        .await?;
    Ok(issued)
}

#[tauri::command]
pub async fn revoke_surface_client_pairing(
    surface_client_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let revoked_surface_client_id = surface_client_id.clone();
    let input = RevokePairingInput {
        surface_client_id,
        reason: "user_revoked".to_string(),
        now: Utc::now(),
    };
    let (event, cleanup_target) = state
        .db_write(move |db| {
            let clock = crate::services::context::SystemClock;
            let rng = crate::services::context::SystemRng;
            let external = crate::services::context::ExternalClients::default();
            let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &external);
            surface_pairing::revoke_pairing(&ctx, db, input).map_err(|error| error.to_string())
        })
        .await?;
    state
        .surface_runtime_endpoint
        .forget_surface_client_sessions(&revoked_surface_client_id);
    let mut audit = state.audit_log.lock();
    for cleanup_event in
        surface_pairing::cleanup_session_keychain_entries(&cleanup_target, "user_revoked")
    {
        if let Err(error) = surface_pairing::emit_pairing_audit(&mut audit, &cleanup_event) {
            log::warn!("surface pairing key cleanup audit write failed: {error}");
        }
    }
    if let Err(error) = surface_pairing::emit_pairing_audit(&mut audit, &event) {
        log::warn!("surface pairing revoke audit write failed: {error}");
    }
    Ok(())
}
