use abilities_runtime::abilities::registry::Actor;
use abilities_runtime::abilities::{AuditIntent, ProjectedComposition};
use serde_json::json;

use crate::audit_log::{emit_surface_audit, AuditError, AuditFields, AuditLogger};
use crate::db::ActionDb;
use crate::services::context::ServiceContext;
use crate::services::projection_signing::{
    sign_projection, ProjectionBlockInput, ProjectionClaimRefInput, ProjectionKeyStore,
    ProjectionSigningError, ProjectionSurface, ProjectionWriteInput, SignedProjectionWrite,
};

#[derive(Debug, Clone)]
pub struct ProjectedCompositionSignatureRequest {
    pub projection_id: String,
    pub surface: ProjectionSurface,
    pub surface_locator: String,
    pub dailyos_canonical_id: String,
    pub dailyos_source_runtime: String,
}

pub fn projection_write_input_from_projected_composition(
    request: ProjectedCompositionSignatureRequest,
    projected: &ProjectedComposition,
) -> ProjectionWriteInput {
    ProjectionWriteInput {
        projection_id: request.projection_id,
        surface: request.surface,
        surface_locator: request.surface_locator,
        dailyos_canonical_id: request.dailyos_canonical_id,
        dailyos_source_runtime: request.dailyos_source_runtime,
        dailyos_projection_version: u64::from(projected.fallback_policy_version),
        composition_id: projected.composition_id.as_str().to_string(),
        composition_version: projected.composition_version.unwrap_or_default(),
        blocks: projected
            .blocks
            .iter()
            .enumerate()
            .map(|(index, block)| ProjectionBlockInput {
                block_id: block.block_id.as_str().to_string(),
                block_order: index as u64,
                block_type: block.selected_known_type_id.clone(),
                block_payload: json!({
                    "fallback_policy_version": projected.fallback_policy_version,
                    "original_type_id": block.original_type_id,
                    "selected_known_type_id": block.selected_known_type_id,
                    "payload": block.payload,
                    "banner": block.banner,
                    "trust_band": block.trust_band,
                    "edit_routes": block.edit_routes,
                    "diagnostics": block.diagnostics,
                }),
                claim_refs: block
                    .claim_refs
                    .iter()
                    .map(|claim_ref| ProjectionClaimRefInput {
                        claim_id: claim_ref.claim_id.clone(),
                        claim_version: claim_ref.claim_version.unwrap_or_default(),
                        field_path: claim_ref
                            .field_path
                            .as_ref()
                            .map(|field_path| field_path.as_str().to_string()),
                        provenance_invocation_id: block
                            .provenance
                            .first()
                            .map(|provenance| provenance.invocation_id.0.to_string()),
                        provenance_field_path: block
                            .provenance
                            .first()
                            .map(|provenance| provenance.field_path.as_str().to_string()),
                        scope_grant_hash: None,
                    })
                    .collect(),
            })
            .collect(),
    }
}

pub fn sign_projected_composition(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    key_store: &dyn ProjectionKeyStore,
    request: ProjectedCompositionSignatureRequest,
    projected: &ProjectedComposition,
) -> Result<SignedProjectionWrite, ProjectionSigningError> {
    let input = projection_write_input_from_projected_composition(request, projected);
    sign_projection(ctx, db, key_store, input)
}

pub fn drain_projection_audit_intents(
    logger: &mut AuditLogger,
    actor: &Actor,
    wp_user_id: Option<u64>,
    wp_user_hash: Option<&str>,
    intents: &[AuditIntent],
) -> Result<(), AuditError> {
    for intent in intents {
        let mut fields = AuditFields::new(intent.category.as_str(), intent.detail.clone());
        if let Some(wp_user_id) = wp_user_id {
            fields = fields.with_wp_user_id(wp_user_id);
        }
        if let Some(wp_user_hash) = wp_user_hash {
            fields = fields.with_wp_user_hash(wp_user_hash);
        }
        emit_surface_audit(logger, intent.event_kind, actor, fields)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use abilities_runtime::abilities::composition::{BlockId, CompositionDocId};
    use abilities_runtime::abilities::registry::{ScopeSet, SurfaceClientId, SurfaceScope};
    use abilities_runtime::abilities::trust::TrustBand;
    use abilities_runtime::abilities::{AuditCategory, ProjectedBlock};
    use serde_json::json;

    fn projected(policy_version: u32) -> ProjectedComposition {
        ProjectedComposition {
            composition_id: CompositionDocId::new("composition-projection-test"),
            composition_version: Some(3),
            fallback_policy_version: policy_version,
            blocks: vec![ProjectedBlock {
                block_id: BlockId::new("block-1"),
                block_index: 0,
                original_type_id: "dailyos/custom".to_string(),
                selected_known_type_id: "markdown_document".to_string(),
                payload: json!({"title": "Visible"}),
                banner: Some(
                    "Rendered as nearest known type — payload may be incomplete.".to_string(),
                ),
                trust_band: TrustBand::NeedsVerification,
                claim_refs: vec![],
                provenance: vec![],
                edit_routes: vec![],
                diagnostics: vec![],
            }],
            diagnostics: vec![],
            unknown_block_count: 1,
            unknown_block_cap: 5,
            dropped_unknown_block_count: 0,
        }
    }

    fn signature_request() -> ProjectedCompositionSignatureRequest {
        ProjectedCompositionSignatureRequest {
            projection_id: "projection-test".to_string(),
            surface: ProjectionSurface::WordpressDb,
            surface_locator: "wp:post:1:block:1".to_string(),
            dailyos_canonical_id: "composition:projection-test".to_string(),
            dailyos_source_runtime: "runtime-test".to_string(),
        }
    }

    #[test]
    fn policy_version_flows_into_signature_input() {
        let first =
            projection_write_input_from_projected_composition(signature_request(), &projected(3));
        let second =
            projection_write_input_from_projected_composition(signature_request(), &projected(4));
        assert_ne!(
            first.dailyos_projection_version,
            second.dailyos_projection_version
        );
        assert_ne!(
            first.blocks[0].block_payload,
            second.blocks[0].block_payload
        );
    }

    #[test]
    fn audit_intents_drain_through_surface_audit_helper() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("projection-audit.jsonl");
        let mut logger = AuditLogger::new(path.clone());
        let actor = Actor::SurfaceClient {
            instance: SurfaceClientId::new("surface-test"),
            scopes: ScopeSet::new([SurfaceScope::new("read.composition")]).unwrap(),
        };
        let intents = vec![AuditIntent {
            event_kind: "custom_block_fallback_cap_exceeded",
            category: AuditCategory::Anomaly,
            detail: json!({"schema_version": 1, "reason": "unknown_block_cap_exceeded"}),
        }];
        drain_projection_audit_intents(
            &mut logger,
            &actor,
            Some(42),
            Some("wp-user-hash-test"),
            &intents,
        )
        .unwrap();
        let written = std::fs::read_to_string(path).unwrap();
        assert!(written.contains("custom_block_fallback_cap_exceeded"));
        assert!(written.contains("unknown_block_cap_exceeded"));
    }
}
