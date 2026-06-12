#![allow(
    clippy::let_underscore_must_use,
    reason = "tauri::command macro emits internal Result glue that discards generated metadata"
)]

use std::sync::Arc;

use tauri::State;

use crate::services::intelligence::{
    cleanup_over_cap_generated_projection_claims_for_subject_via_db_service,
    OverCapGeneratedProjectionCleanupInput, OverCapGeneratedProjectionCleanupReport,
};
use crate::state::AppState;

#[tauri::command]
pub async fn cleanup_over_cap_generated_intelligence(
    state: State<'_, Arc<AppState>>,
    entity_type: String,
    entity_id: String,
    projection_producer: Option<String>,
    dry_run: bool,
) -> Result<OverCapGeneratedProjectionCleanupReport, String> {
    cleanup_over_cap_generated_projection_claims_for_subject_via_db_service(
        state.inner(),
        OverCapGeneratedProjectionCleanupInput {
            entity_type,
            entity_id,
            projection_producer,
            dry_run,
        },
    )
    .await
}
