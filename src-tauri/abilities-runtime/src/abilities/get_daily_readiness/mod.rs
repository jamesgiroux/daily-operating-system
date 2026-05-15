pub mod prompts;
pub mod synthesis;

use dailyos_abilities_macro::ability;

pub use synthesis::{
    build_daily_readiness, DailyReadiness, DailyReadinessContext, DailyReadinessInput,
};

use crate::abilities::{AbilityContext, AbilityResult};

pub const OPERATIONS: &[&str] = &["IntelligenceComplete"];

#[ability(
    name = "get_daily_readiness",
    category = Read,
    version = "0.1.0",
    schema_version = 1,
    allowed_actors = [User, Agent, System],
    allowed_modes = [Live, Simulate, Evaluate],
    requires_confirmation = false,
    may_publish = false,
    required_scopes = ["read.daily_readiness"],
    mcp_exposure = Invocable,
    composes = [
        { id = "prepare_meeting", ability = "prepare_meeting", optional = false },
        { id = "get_entity_context", ability = "get_entity_context", optional = false }
    ],
    experimental = false,
    signal_policy = { emits_on_output_change = [], coalesce = false }
)]
pub async fn get_daily_readiness(
    ctx: &AbilityContext<'_>,
    input: DailyReadinessInput,
) -> AbilityResult<DailyReadiness> {
    build_daily_readiness(ctx, input).await
}
