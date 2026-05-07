use super::*;

#[tauri::command]
pub async fn reveal_sensitive_claim_text(
    claim_id: String,
    surface: Option<String>,
    reveal_session_id: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<crate::services::sensitivity::RenderableClaimText, String> {
    let surface = match surface.as_deref() {
        Some(name) => crate::services::sensitivity::RenderSurface::from_name(name)
            .ok_or_else(|| format!("Unknown render surface: {name}"))?,
        None => crate::services::sensitivity::RenderSurface::TauriEntityDetail,
    };
    let actor = crate::services::sensitivity::RenderActor::user("user", Some("user"));
    state
        .db_write(move |db| {
            crate::services::sensitivity::reveal_claim_text_for_tauri(
                db,
                &claim_id,
                surface,
                &actor,
                reveal_session_id.as_deref(),
            )
        })
        .await
}
