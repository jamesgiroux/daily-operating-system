// Daily Briefing redesign IPC surface — DOS-413.
// W0 lands the contract + a single atomic command. The command returns
// `BriefingResult::Loading` until W2 services fill the assembly.

use std::sync::Arc;

use tauri::State;

use crate::services::briefing_view_model::{self, BriefingResult};
use crate::state::AppState;

/// Atomic IPC for the redesigned Daily Briefing — one read returns the full
/// `BriefingResult` envelope. Fragmenting into per-section commands is
/// rejected by ADR 0129 (decisions/0129-briefing-view-model-contract.md).
#[tauri::command]
pub async fn get_briefing_view_model(
    state: State<'_, Arc<AppState>>,
) -> Result<BriefingResult, String> {
    Ok(briefing_view_model::get_briefing_view_model(&state).await)
}
