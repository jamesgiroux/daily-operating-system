pub mod prompts;
pub mod synthesis;

use dailyos_abilities_macro::ability;

pub use synthesis::{
    DetectRiskShiftInput, EvidenceSummary, RiskDirection, RiskIndicator, RiskShiftClaimDraft,
    RiskShiftContext, RiskShiftPersistError, RiskShiftResult, RiskShiftSourceVerification,
    TrustEnvelope,
};

use crate::abilities::{AbilityContext, AbilityResult};

#[ability(
    name = "detect_risk_shift",
    category = Transform,
    version = "0.1.0",
    schema_version = 1,
    allowed_actors = [User, Agent, System],
    allowed_modes = [Live, Simulate, Evaluate],
    requires_confirmation = false,
    may_publish = false,
    composes = [{ id = "get_entity_context", ability = "get_entity_context", optional = false }],
    experimental = false,
    signal_policy = { emits_on_output_change = [], coalesce = false }
)]
pub async fn detect_risk_shift(
    ctx: &AbilityContext<'_>,
    input: DetectRiskShiftInput,
) -> AbilityResult<RiskShiftResult> {
    synthesis::build_risk_shift(ctx, input).await
}
