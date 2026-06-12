#![allow(
    clippy::let_underscore_must_use,
    reason = "tauri::command macro emits internal Result glue that discards generated metadata"
)]

use std::sync::Arc;

use tauri::State;

use abilities_runtime::abilities::{ProjectedComposition, SurfaceKind};

use crate::abilities::provenance::{
    build_ownership_policy_for_invocation, validate_serialized_subject_ownership,
};
use crate::abilities::{AbilityRegistry, Actor};
use crate::bridges::tauri::{
    parse_tauri_claim_dismissal_surface, TauriAbilityBridge, TauriInvokeContext,
};
use crate::bridges::{
    AbilityResponseJson, BridgeSurface, BridgeSurfaceError, ConfirmationToken, RenderedProvenance,
};
use crate::observability::aggregate_metric::{MetricDimensions, MetricValue, Outcome};
use crate::services::composition_render_orchestrator::{
    hydrate_producer_projection_input, project_composition_for_surface_with_options,
    resolve_producer_ability_name, ProjectCompositionRenderOptions,
};
use crate::state::AppState;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProjectedCompositionCommandResponse {
    pub ok: bool,
    pub request_id: String,
    pub projection: ProjectedComposition,
    pub cache_hint_token: String,
    pub served_from_cache: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rendered_provenance: Option<RenderedProvenance>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingCompositionTokenResponse {
    pub meeting_token: String,
    pub composition_id: String,
}

#[allow(
    clippy::let_underscore_must_use,
    reason = "tauri::command macro emits internal Result glue that discards generated metadata"
)]
#[tauri::command]
pub async fn invoke_ability(
    state: State<'_, Arc<AppState>>,
    ability_name: String,
    input_json: serde_json::Value,
    render_surface: String,
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
    let claim_dismissal_surface = parse_tauri_claim_dismissal_surface(&render_surface)?;
    let response = match TauriAbilityBridge::new(registry)
        .invoke(
            state.inner().as_ref(),
            &ability_name,
            input_json,
            TauriInvokeContext::new(
                Actor::User,
                BridgeSurface::TauriApp,
                claim_dismissal_surface,
                dry_run,
                confirmation.as_ref(),
            ),
        )
        .await
    {
        Ok(response) => response,
        Err(err) => {
            record_ability_invocation_metric(
                state.inner().as_ref(),
                ability_meta,
                Outcome::Failure,
            );
            return Err(err);
        }
    };
    let policy = match build_ownership_policy_for_invocation(
        ability_meta,
        &input_for_policy,
        response.raw_provenance_value(),
    ) {
        Ok(policy) => policy,
        Err(err) => {
            record_ability_invocation_metric(
                state.inner().as_ref(),
                ability_meta,
                Outcome::Failure,
            );
            return Err(err.into());
        }
    };
    if let Err(err) = validate_serialized_subject_ownership(
        response.data.clone(),
        response.raw_provenance_value().clone(),
        response.diagnostics.clone(),
        &[],
        policy,
    ) {
        log::error!(
            "ability response ownership validation failed for {}: {}",
            ability_name,
            err
        );
        record_ability_invocation_metric(state.inner().as_ref(), ability_meta, Outcome::Failure);
        return Err(err.into());
    }
    record_ability_invocation_metric(state.inner().as_ref(), ability_meta, Outcome::Success);
    Ok(response)
}

#[allow(
    clippy::let_underscore_must_use,
    reason = "tauri::command macro emits internal Result glue that discards generated metadata"
)]
#[tauri::command]
pub async fn get_projected_composition(
    state: State<'_, Arc<AppState>>,
    composition_id: String,
    composition_version: Option<i64>,
    cache_hint_token: Option<String>,
    force_refresh: Option<bool>,
) -> Result<ProjectedCompositionCommandResponse, BridgeSurfaceError> {
    let _ = composition_version;
    let _ = cache_hint_token;

    if state.lock_state.lock().is_locked {
        return Err(BridgeSurfaceError::AbilityUnavailable);
    }

    let request_id = uuid::Uuid::new_v4().to_string();
    let Some(ability_name) = resolve_producer_ability_name(&composition_id) else {
        return Err(BridgeSurfaceError::Validation(
            "project_composition_unknown_producer".to_string(),
        ));
    };

    let registry =
        AbilityRegistry::global_checked().map_err(|_| BridgeSurfaceError::AbilityUnavailable)?;
    let ability_meta = registry
        .iter_for(Actor::User)
        .find(|descriptor| descriptor.name == ability_name)
        .ok_or(BridgeSurfaceError::AbilityUnavailable)?;

    let app_state = state.inner().clone();
    let render = match project_composition_for_surface_with_options(
        app_state.as_ref(),
        Actor::User,
        SurfaceKind::TauriApp,
        &composition_id,
        ProjectCompositionRenderOptions {
            force_refresh: force_refresh.unwrap_or(false),
        },
        |producer_input| {
            let app_state = app_state.clone();
            async move {
                let input =
                    hydrate_producer_projection_input(app_state.as_ref(), &producer_input).await?;
                let response = TauriAbilityBridge::new(registry)
                    .invoke_tauri_app(
                        app_state.as_ref(),
                        producer_input.ability_name,
                        input.clone(),
                        crate::services::context::ClaimDismissalSurface::TauriEntityDetail,
                        false,
                        None,
                    )
                    .await?;

                let policy = build_ownership_policy_for_invocation(
                    ability_meta,
                    &input,
                    response.raw_provenance_value(),
                )?;
                validate_serialized_subject_ownership(
                    response.data.clone(),
                    response.raw_provenance_value().clone(),
                    response.diagnostics.clone(),
                    &[],
                    policy,
                )?;
                Ok(response)
            }
        },
    )
    .await
    {
        Ok(render) => render,
        Err(err) => {
            record_ability_invocation_metric(app_state.as_ref(), ability_meta, Outcome::Failure);
            return Err(err);
        }
    };

    if let Some(outcome) = projected_composition_producer_metric_outcome(render.served_from_cache) {
        record_ability_invocation_metric(app_state.as_ref(), ability_meta, outcome);
    }
    Ok(ProjectedCompositionCommandResponse {
        ok: true,
        request_id,
        projection: render.projection,
        cache_hint_token: render.cache_hint_token,
        served_from_cache: render.served_from_cache,
        rendered_provenance: render.rendered_provenance,
    })
}

#[allow(
    clippy::let_underscore_must_use,
    reason = "tauri::command macro emits internal Result glue that discards generated metadata"
)]
#[tauri::command]
pub async fn get_meeting_composition_token(
    state: State<'_, Arc<AppState>>,
    meeting_id: String,
) -> Result<MeetingCompositionTokenResponse, String> {
    if state.lock_state.lock().is_locked {
        return Err("app locked".to_string());
    }

    let meeting_token = state
        .db_read(move |db| {
            crate::services::meetings::issue_meeting_composition_token(db, &meeting_id)
        })
        .await
        .map_err(String::from)?;
    Ok(MeetingCompositionTokenResponse {
        composition_id: format!("dailyos/meeting-detail:meeting:{meeting_token}"),
        meeting_token,
    })
}

fn projected_composition_producer_metric_outcome(served_from_cache: bool) -> Option<Outcome> {
    (!served_from_cache).then_some(Outcome::Success)
}

fn record_ability_invocation_metric(
    state: &AppState,
    ability_meta: &crate::abilities::AbilityDescriptor,
    outcome: Outcome,
) {
    crate::observability::aggregate_metric::emit_aggregate_metric(
        state,
        crate::aggregate_metric_name!("ability_invocation_count"),
        MetricValue::Count(1),
        MetricDimensions::default()
            .ability(ability_meta.name, ability_meta.version)
            .outcome(outcome),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projected_composition_metrics_only_count_producer_runs() {
        assert_eq!(
            projected_composition_producer_metric_outcome(false),
            Some(Outcome::Success)
        );
        assert_eq!(projected_composition_producer_metric_outcome(true), None);
    }
}
