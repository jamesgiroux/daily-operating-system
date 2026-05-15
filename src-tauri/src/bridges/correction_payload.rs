use rusqlite::{params, OptionalExtension};
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

/// Project a composition-bound version-event row for the given actor's scope
/// per packet §16. Compositions store only version metadata in the substrate
/// (`composition_versions`); there is no per-composition `subject_ref` /
/// `sensitivity` to test against. The scope gate therefore mirrors the
/// claim-read gate: out-of-scope actors get a redacted envelope, in-scope
/// actors get a non-redacted payload. The endpoint composes the
/// `composition_id`-bearing fields onto the wire only when `scope_redacted`
/// is false.
///
/// Returns `None` when the substrate has no `composition_versions` row for
/// `composition_id` (404), matching `project_claim_for_scope` semantics.
pub fn project_composition_for_scope(
    db: &ActionDb,
    composition_id: &str,
    actor: &Actor,
) -> Option<CorrectionPayload> {
    let exists: bool = match db
        .conn_ref()
        .query_row(
            "SELECT 1 FROM composition_versions WHERE composition_id = ?1",
            params![composition_id],
            |row| row.get::<_, i64>(0).map(|_| true),
        )
        .optional()
    {
        Ok(Some(true)) => true,
        Ok(_) => false,
        Err(error) => {
            log::warn!(
                target: "dailyos_lib::bridges::correction_payload",
                "composition projection failed to look up composition_id={composition_id}: {error}"
            );
            return None;
        }
    };
    if !exists {
        return None;
    }

    let Actor::SurfaceClient { scopes, .. } = actor else {
        return Some(CorrectionPayload::out_of_scope());
    };

    if !scope_permits_composition_read(scopes) {
        return Some(CorrectionPayload::out_of_scope());
    }

    // Compositions have no claim body to embed; the endpoint shapes the full
    // event envelope itself. We return a non-redacted CorrectionPayload with
    // claim=None as the signal "scope passed; emit the full event".
    Some(CorrectionPayload {
        claim: None,
        scope_redacted: false,
        reason: None,
    })
}

fn scope_permits_composition_read(scopes: &ScopeSet) -> bool {
    scopes.iter().any(|scope| {
        let scope = scope.as_str();
        scope == "read.composition"
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
