use super::*;

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectiveUpdateRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub target_date: Option<String>,
    pub sort_order: Option<i32>,
    pub status: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MilestoneUpdateRequest {
    pub title: Option<String>,
    pub target_date: Option<String>,
    pub auto_detect_signal: Option<String>,
    pub sort_order: Option<i32>,
    pub status: Option<String>,
}

#[tauri::command]
pub async fn create_objective(
    account_id: String,
    title: String,
    description: Option<String>,
    target_date: Option<String>,
    source: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<crate::types::AccountObjective, String> {
    state
        .db_write(move |db| {
            crate::services::success_plans::create_objective(
                db,
                &account_id,
                &title,
                description.as_deref(),
                target_date.as_deref(),
                source.as_deref().unwrap_or("user"),
            )
        })
        .await
}

#[tauri::command]
pub async fn update_objective(
    id: String,
    fields: ObjectiveUpdateRequest,
    state: State<'_, Arc<AppState>>,
) -> Result<crate::types::AccountObjective, String> {
    state
        .db_write(move |db| {
            crate::services::success_plans::update_objective(
                db,
                &id,
                fields.title.as_deref(),
                fields.description.as_deref(),
                fields.target_date.as_deref(),
                fields.sort_order,
                fields.status.as_deref(),
            )
        })
        .await
}

#[tauri::command]
pub async fn complete_objective(
    id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<crate::types::AccountObjective, String> {
    let app_state = state.inner().clone();
    state
        .db_write(move |db| crate::services::success_plans::complete_objective(db, &app_state, &id))
        .await
}

#[tauri::command]
pub async fn abandon_objective(
    id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<crate::types::AccountObjective, String> {
    state
        .db_write(move |db| crate::services::success_plans::abandon_objective(db, &id))
        .await
}

#[tauri::command]
pub async fn delete_objective(
    id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    state
        .db_write(move |db| crate::services::success_plans::delete_objective(db, &id))
        .await
}

#[tauri::command]
pub async fn create_milestone(
    objective_id: String,
    title: String,
    target_date: Option<String>,
    auto_detect_signal: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<crate::types::AccountMilestone, String> {
    state
        .db_write(move |db| {
            crate::services::success_plans::create_milestone(
                db,
                &objective_id,
                &title,
                target_date.as_deref(),
                auto_detect_signal.as_deref(),
            )
        })
        .await
}

#[tauri::command]
pub async fn update_milestone(
    id: String,
    fields: MilestoneUpdateRequest,
    state: State<'_, Arc<AppState>>,
) -> Result<crate::types::AccountMilestone, String> {
    state
        .db_write(move |db| {
            crate::services::success_plans::update_milestone(
                db,
                &id,
                fields.title.as_deref(),
                fields.target_date.as_deref(),
                fields.auto_detect_signal.as_deref(),
                fields.sort_order,
                fields.status.as_deref(),
            )
        })
        .await
}

#[tauri::command]
pub async fn complete_milestone(
    id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<crate::types::AccountMilestone, String> {
    let app_state = state.inner().clone();
    state
        .db_write(move |db| crate::services::success_plans::complete_milestone(db, &app_state, &id))
        .await
}

#[tauri::command]
pub async fn skip_milestone(
    id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<crate::types::AccountMilestone, String> {
    let app_state = state.inner().clone();
    state
        .db_write(move |db| crate::services::success_plans::skip_milestone(db, &app_state, &id))
        .await
}

#[tauri::command]
pub async fn delete_milestone(
    id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    state
        .db_write(move |db| crate::services::success_plans::delete_milestone(db, &id))
        .await
}

#[tauri::command]
pub async fn link_action_to_objective(
    action_id: String,
    objective_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    state
        .db_write(move |db| crate::services::success_plans::link_action_to_objective(db, &action_id, &objective_id))
        .await
}

#[tauri::command]
pub async fn unlink_action_from_objective(
    action_id: String,
    objective_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    state
        .db_write(move |db| crate::services::success_plans::unlink_action_from_objective(db, &action_id, &objective_id))
        .await
}

#[tauri::command]
pub async fn reorder_objectives(
    account_id: String,
    ordered_ids: Vec<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    state
        .db_write(move |db| crate::services::success_plans::reorder_objectives(db, &account_id, &ordered_ids))
        .await
}

#[tauri::command]
pub async fn reorder_milestones(
    objective_id: String,
    ordered_ids: Vec<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    state
        .db_write(move |db| crate::services::success_plans::reorder_milestones(db, &objective_id, &ordered_ids))
        .await
}

#[tauri::command]
pub async fn get_objective_suggestions(
    account_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::types::SuggestedObjective>, String> {
    state
        .db_read(move |db| crate::services::success_plans::get_objective_suggestions(db, &account_id))
        .await
}

#[tauri::command]
pub async fn create_objective_from_suggestion(
    account_id: String,
    suggestion_json: String,
    state: State<'_, Arc<AppState>>,
) -> Result<crate::types::AccountObjective, String> {
    let suggestion: crate::types::SuggestedObjective =
        serde_json::from_str(&suggestion_json).map_err(|e| format!("Invalid suggestion JSON: {e}"))?;
    state
        .db_write(move |db| crate::services::success_plans::create_objective_from_suggestion(db, &account_id, &suggestion))
        .await
}

#[tauri::command]
pub fn list_success_plan_templates() -> Result<Vec<crate::types::SuccessPlanTemplate>, String> {
    Ok(crate::services::success_plans::list_templates())
}

#[tauri::command]
pub async fn apply_success_plan_template(
    account_id: String,
    template_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::types::AccountObjective>, String> {
    state
        .db_write(move |db| crate::services::success_plans::apply_success_plan_template(db, &account_id, &template_id))
        .await
}
