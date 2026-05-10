use crate::sensitivity::{ClaimDismissalSurface, RenderSurface};

use super::super::types::SurfaceClass;

pub fn target_surface_for_render_surface(surface: Option<RenderSurface>) -> Option<SurfaceClass> {
    surface.and_then(surface_class_for_render_surface)
}

pub fn target_surface_for_claim_surface(
    surface: Option<ClaimDismissalSurface>,
) -> Option<SurfaceClass> {
    surface.and_then(surface_class_for_claim_surface)
}

pub fn surface_class_for_render_surface(surface: RenderSurface) -> Option<SurfaceClass> {
    match surface {
        RenderSurface::TauriEntityDetail
        | RenderSurface::TauriMeetingDetail
        | RenderSurface::TauriEmailSummary
        | RenderSurface::Action
        | RenderSurface::TauriProvenance
        | RenderSurface::TauriReport => Some(SurfaceClass::Confidential),
        RenderSurface::TauriBriefingPrep
        | RenderSurface::TauriChat
        | RenderSurface::McpTool
        | RenderSurface::McpToolDetail => Some(SurfaceClass::Internal),
        RenderSurface::P2Publication
        | RenderSurface::LogStructured
        | RenderSurface::PushNotification => Some(SurfaceClass::Public),
    }
}

pub fn surface_class_for_claim_surface(surface: ClaimDismissalSurface) -> Option<SurfaceClass> {
    match surface {
        ClaimDismissalSurface::TauriEntityDetail
        | ClaimDismissalSurface::TauriMeetingDetail
        | ClaimDismissalSurface::TauriEmailSummary
        | ClaimDismissalSurface::Action
        | ClaimDismissalSurface::TauriProvenance
        | ClaimDismissalSurface::TauriReport => Some(SurfaceClass::Confidential),
        ClaimDismissalSurface::Briefing
        | ClaimDismissalSurface::TauriChat
        | ClaimDismissalSurface::McpTool
        | ClaimDismissalSurface::McpToolDetail => Some(SurfaceClass::Internal),
        ClaimDismissalSurface::P2Publication
        | ClaimDismissalSurface::LogStructured
        | ClaimDismissalSurface::PushNotification => Some(SurfaceClass::Public),
        ClaimDismissalSurface::Worker | ClaimDismissalSurface::Eval => None,
    }
}

#[cfg(test)]
#[path = "surface_test.rs"]
mod surface_test;
