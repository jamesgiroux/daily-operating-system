pub mod contracts;
pub mod producer;

pub use contracts::{
    EntityIntakeClaim, EntityIntakeError, EntityIntakeInput, EntityIntakeOutput,
    EntityIntakeRenderInput,
};

use dailyos_abilities_macro::ability;

use crate::abilities::{AbilityContext, AbilityResult};

#[ability(
    name = "entity_intake",
    category = Transform,
    version = "1.0.0",
    schema_version = 1,
    allowed_actors = [SurfaceClient],
    allowed_modes = [Live, Simulate, Evaluate],
    requires_confirmation = false,
    may_publish = true,
    required_scopes = ["write.entity_intake"],
    mcp_exposure = None,
    composes = [],
    experimental = false,
    signal_policy = { emits_on_output_change = [], coalesce = false }
)]
pub async fn entity_intake(
    ctx: &AbilityContext<'_>,
    input: EntityIntakeInput,
) -> AbilityResult<EntityIntakeOutput> {
    producer::entity_intake(ctx, input).await
}

#[ability(
    name = "entity_intake_render",
    category = Read,
    version = "1.0.0",
    schema_version = 1,
    allowed_actors = [SurfaceClient],
    allowed_modes = [Live, Simulate, Evaluate],
    requires_confirmation = false,
    may_publish = false,
    required_scopes = ["read.entity_intelligence"],
    mcp_exposure = None,
    composes = [],
    experimental = false,
    signal_policy = { emits_on_output_change = [], coalesce = false }
)]
pub async fn entity_intake_render(
    ctx: &AbilityContext<'_>,
    input: EntityIntakeRenderInput,
) -> AbilityResult<EntityIntakeOutput> {
    producer::entity_intake_render(ctx, input).await
}
