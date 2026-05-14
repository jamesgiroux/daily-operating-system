use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::abilities::registry::ScopeSet;
use crate::abilities::Actor;
use crate::db::claims::{ClaimState, SurfacingState};
use crate::db::ActionDb;
use crate::services::claims::load_claim_by_id;
use crate::services::sensitivity::{
    render_policy_for_surface, RenderActor, RenderDecision, RenderSurface,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CorrectionPayload {
    pub claim: Option<Value>,
    pub scope_redacted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl CorrectionPayload {
    pub fn out_of_scope() -> Self {
        Self {
            claim: None,
            scope_redacted: true,
            reason: Some("out_of_scope".to_string()),
        }
    }
}

pub fn project_claim_for_scope(
    db: &ActionDb,
    claim_id: &str,
    actor: &Actor,
) -> Option<CorrectionPayload> {
    let claim = match load_claim_by_id(db.conn_ref(), claim_id) {
        Ok(Some(claim)) => claim,
        Ok(None) => return None,
        Err(error) => {
            log::warn!(
                target: "dailyos_lib::bridges::correction_payload",
                "correction projection failed to load claim_id={claim_id}: {error}"
            );
            return None;
        }
    };

    let Actor::SurfaceClient { scopes, .. } = actor else {
        return Some(CorrectionPayload::out_of_scope());
    };

    if !scope_permits_claim_read(scopes) || !claim_renderable_for_surface_client(&claim) {
        return Some(CorrectionPayload::out_of_scope());
    }

    let claim = match serde_json::to_value(&claim) {
        Ok(value) => value,
        Err(error) => {
            log::warn!(
                target: "dailyos_lib::bridges::correction_payload",
                "correction projection failed to serialize claim_id={claim_id}: {error}"
            );
            return Some(CorrectionPayload::out_of_scope());
        }
    };

    Some(CorrectionPayload {
        claim: Some(claim),
        scope_redacted: false,
        reason: None,
    })
}

fn scope_permits_claim_read(scopes: &ScopeSet) -> bool {
    scopes.iter().any(|scope| {
        let scope = scope.as_str();
        scope == "read.account_overview"
            || scope == "read.composition"
            || scope.starts_with("read.")
            || scope.starts_with("admin.")
            || scope.starts_with("manage.")
    })
}

fn claim_renderable_for_surface_client(claim: &crate::db::claims::IntelligenceClaim) -> bool {
    if claim.claim_state != ClaimState::Active || claim.surfacing_state != SurfacingState::Active {
        return false;
    }

    matches!(
        render_policy_for_surface(
            claim,
            RenderSurface::McpTool,
            &RenderActor::agent("surface_client"),
        ),
        RenderDecision::Render
    )
}
