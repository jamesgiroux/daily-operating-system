pub mod contracts;
pub mod producer;

pub use contracts::{
    PlacementError, PlacementErrorCode, WorkspacePlaceDocumentInput, WorkspacePlaceDocumentReceipt,
    WorkspacePlaceDocumentRequest, WorkspacePlacementEntity, WorkspacePlacementMutationCursor,
};

use dailyos_abilities_macro::ability;

use crate::abilities::{AbilityContext, AbilityResult};

#[ability(
    name = "workspace_place_document",
    category = Transform,
    version = "1.0.0",
    schema_version = 1,
    allowed_actors = [McpClient],
    allowed_modes = [Live],
    requires_confirmation = false,
    may_publish = true,
    required_scopes = ["write.workspace_place_document"],
    mcp_exposure = Invocable,
    composes = [],
    experimental = false,
    signal_policy = { emits_on_output_change = [], coalesce = false }
)]
pub async fn workspace_place_document(
    ctx: &AbilityContext<'_>,
    input: WorkspacePlaceDocumentInput,
) -> AbilityResult<WorkspacePlaceDocumentReceipt> {
    producer::workspace_place_document(ctx, input).await
}

#[cfg(test)]
mod tests {
    use crate::abilities::registry::{AbilityRegistry, ActorKind, McpExposure};
    use crate::abilities::AbilityCategory;
    use crate::services::workspace_intake::WORKSPACE_PLACE_DOCUMENT_SCOPE;

    #[test]
    fn descriptor_exposes_only_the_mcp_write_surface() {
        let registry = AbilityRegistry::global_checked().expect("registry builds");
        let descriptor = registry
            .iter_all()
            .find(|descriptor| descriptor.name == "workspace_place_document")
            .expect("workspace placement ability is registered");

        assert_eq!(descriptor.category, AbilityCategory::Transform);
        assert_eq!(descriptor.policy.allowed_actors, &[ActorKind::McpClient]);
        assert_eq!(descriptor.policy.required_scopes, &[WORKSPACE_PLACE_DOCUMENT_SCOPE]);
        assert_eq!(descriptor.policy.mcp_exposure, McpExposure::Invocable);
        assert!(descriptor.policy.may_publish);
    }
}
