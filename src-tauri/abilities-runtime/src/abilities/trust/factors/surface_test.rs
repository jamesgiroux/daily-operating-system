use crate::abilities::trust::SurfaceClass;
use crate::sensitivity::{ClaimDismissalSurface, RenderSurface};

use super::{
    surface_class_for_claim_surface, surface_class_for_render_surface,
    target_surface_for_claim_surface, target_surface_for_render_surface,
};

#[test]
fn render_surface_extractor_maps_revealable_tauri_surfaces_to_confidential() {
    assert_eq!(
        surface_class_for_render_surface(RenderSurface::TauriEntityDetail),
        Some(SurfaceClass::Confidential)
    );
    assert_eq!(
        surface_class_for_render_surface(RenderSurface::TauriReport),
        Some(SurfaceClass::Confidential)
    );
}

#[test]
fn render_surface_extractor_maps_agent_surfaces_to_internal() {
    assert_eq!(
        surface_class_for_render_surface(RenderSurface::McpTool),
        Some(SurfaceClass::Internal)
    );
    assert_eq!(
        surface_class_for_render_surface(RenderSurface::TauriChat),
        Some(SurfaceClass::Internal)
    );
}

#[test]
fn render_surface_extractor_maps_external_surfaces_to_public() {
    assert_eq!(
        surface_class_for_render_surface(RenderSurface::P2Publication),
        Some(SurfaceClass::Public)
    );
    assert_eq!(
        surface_class_for_render_surface(RenderSurface::PushNotification),
        Some(SurfaceClass::Public)
    );
}

#[test]
fn claim_surface_extractor_leaves_worker_and_eval_unscoped() {
    assert_eq!(
        surface_class_for_claim_surface(ClaimDismissalSurface::Worker),
        None
    );
    assert_eq!(
        surface_class_for_claim_surface(ClaimDismissalSurface::Eval),
        None
    );
}

#[test]
fn optional_surface_extractors_preserve_absent_surface() {
    assert_eq!(target_surface_for_render_surface(None), None);
    assert_eq!(target_surface_for_claim_surface(None), None);
}

#[test]
fn w3c_shadow_surfaces_are_representable() {
    let render_surfaces = [
        ("briefing", RenderSurface::TauriBriefingPrep),
        ("meeting_detail", RenderSurface::TauriMeetingDetail),
        ("entity_detail", RenderSurface::TauriEntityDetail),
        ("actions", RenderSurface::Action),
        ("email", RenderSurface::TauriEmailSummary),
    ];
    for (name, surface) in render_surfaces {
        assert!(
            surface_class_for_render_surface(surface).is_some(),
            "render surface {name} must map to SurfaceClass"
        );
    }

    let claim_surfaces = [
        ("briefing", ClaimDismissalSurface::Briefing),
        ("meeting_detail", ClaimDismissalSurface::TauriMeetingDetail),
        ("entity_detail", ClaimDismissalSurface::TauriEntityDetail),
        ("actions", ClaimDismissalSurface::Action),
        ("email", ClaimDismissalSurface::TauriEmailSummary),
    ];
    for (name, surface) in claim_surfaces {
        assert!(
            surface_class_for_claim_surface(surface).is_some(),
            "claim surface {name} must map to SurfaceClass"
        );
    }
}
