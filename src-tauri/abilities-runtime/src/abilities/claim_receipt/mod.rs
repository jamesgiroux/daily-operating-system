//! `claim_receipt` Read ability.
//!
//! Registers the `claim_receipt` ability in the runtime registry so WP block
//! inner-block renderers (account-detail / project-detail quote-wall,
//! value-commitments, on-track-chapter, technical-footprint, etc.) can fan out
//! per-claim receipts through
//! `runtime_client->invoke_ability('claim_receipt', $claim_ref, $scope_set)`.
//!
//! Wraps the existing
//! `services::claim_receipt::render::render_receipt_for(state, target,
//! surface)` substrate via the narrow `ClaimReceiptReadHandle` attached on
//! `ServiceContext`. The Tauri command `render_claim_receipt` is preserved
//! unchanged — the ability is an additional invocation path, not a
//! replacement.
//!
//! **Actor scope**: User + Agent + SurfaceClient (matches the actor scope of
//! the existing `render_claim_receipt` Tauri command plus the WP block
//! runtime client). `McpClient` is denied — receipt rendering is
//! sensitivity-bearing and the canonical Mcp routing is the dedicated
//! `SurfaceContext::Mcp` value supplied as an input by an *authorized* caller,
//! not an opaque MCP-originated invocation.
//!
//! **Proposal / WorkItem deferral**: only the `Claim` arm of
//! `ClaimReceiptTarget` resolves today; the other arms return a typed
//! `TargetNotFound` at the adapter boundary. Proposal-receipt render policy
//! is future W4 work, not in current scope.

pub mod contracts;
pub mod producer;

pub use crate::services::context::{
    ClaimReceiptAction, ClaimReceiptFreshness, ClaimReceiptLifecycle, ClaimReceiptProvenance,
    ClaimReceiptProvenanceSource, ClaimReceiptRedactionLevel, ClaimReceiptSnapshot,
    ClaimReceiptSurfaceContext, ClaimReceiptTarget, ClaimReceiptTrust,
};
pub use contracts::ClaimReceiptInput;

use dailyos_abilities_macro::ability;

use crate::abilities::{AbilityContext, AbilityResult};

#[ability(
    name = "claim_receipt",
    category = Read,
    version = "1.0.0",
    schema_version = 1,
    allowed_actors = [User, Agent, SurfaceClient],
    allowed_modes = [Live, Evaluate],
    requires_confirmation = false,
    may_publish = false,
    required_scopes = ["read.claim_receipt"],
    mcp_exposure = None,
    composes = [],
    experimental = false,
    signal_policy = { emits_on_output_change = [], coalesce = false }
)]
pub async fn claim_receipt(
    ctx: &AbilityContext<'_>,
    input: ClaimReceiptInput,
) -> AbilityResult<ClaimReceiptSnapshot> {
    producer::build_claim_receipt(ctx, input).await
}

#[cfg(test)]
mod tests {
    use crate::abilities::registry::{ActorKind, McpExposure};
    use crate::abilities::AbilityRegistry;

    fn registered_descriptor() -> &'static crate::abilities::AbilityDescriptor {
        let registry = AbilityRegistry::global_checked().expect("registry");
        registry
            .iter_all()
            .find(|d| d.name == "claim_receipt")
            .expect("claim_receipt ability is registered in the global inventory registry")
    }

    #[test]
    fn ability_registered_in_global_registry() {
        // The WP block invocation path
        // `invoke_ability('claim_receipt', ...)` requires this descriptor to
        // be discoverable through the global registry. If this test fails the
        // claim-bearing inner blocks fall back to empty placeholders.
        let descriptor = registered_descriptor();
        assert_eq!(descriptor.name, "claim_receipt");
        assert_eq!(descriptor.policy.required_scopes, &["read.claim_receipt"]);
    }

    #[test]
    fn allowed_actors_include_user_agent_surface_client_but_not_mcp_client() {
        // User + Agent allowed (matches the actor scope of the existing
        // `render_claim_receipt` Tauri command); SurfaceClient allowed so the
        // WP runtime client invocation works; McpClient denied
        // (sensitivity-bearing — Mcp surface must be opted into explicitly via
        // `SurfaceContext::Mcp` by an authorized caller, not entered through
        // an opaque MCP-originated invocation path).
        let descriptor = registered_descriptor();
        let actors: &[ActorKind] = descriptor.policy.allowed_actors;
        assert!(actors.contains(&ActorKind::User), "User must be allowed");
        assert!(actors.contains(&ActorKind::Agent), "Agent must be allowed");
        assert!(
            actors.contains(&ActorKind::SurfaceClient),
            "SurfaceClient must be allowed (WP block runtime client path)"
        );
        assert!(
            !actors.contains(&ActorKind::McpClient),
            "McpClient must be denied (receipt rendering is sensitivity-bearing)"
        );
        assert!(
            !actors.contains(&ActorKind::Admin),
            "Admin not declared — only the four canonical caller kinds"
        );
        assert!(
            !actors.contains(&ActorKind::System),
            "System not declared — system reads route through services::claim_receipt directly"
        );
    }

    #[test]
    fn mcp_exposure_is_none_for_conservative_surface_routing() {
        // Even though the Mcp variant of SurfaceContext exists, the ability
        // itself is hidden from MCP introspection. The Mcp render path is
        // reserved for explicit Tauri command invocation by code that has
        // already enforced its own audience policy.
        let descriptor = registered_descriptor();
        assert_eq!(descriptor.policy.mcp_exposure, McpExposure::None);
    }

    #[test]
    fn category_is_read_with_may_publish_false() {
        let descriptor = registered_descriptor();
        assert_eq!(descriptor.category, crate::abilities::AbilityCategory::Read);
        assert!(!descriptor.policy.may_publish);
        assert!(!descriptor.policy.requires_confirmation);
    }
}
