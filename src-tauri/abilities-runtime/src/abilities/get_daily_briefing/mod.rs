//! `get_daily_briefing` Read/User-only ability + composed
//! `BriefingState` (cycle-1 correctness F3).
//!
//! Read-side composition over existing meeting prep status (§5.5) +
//! per-subject entity intelligence (§5.1) + daily readiness context.
//! Per `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md`
//! §5.10.
//!
//! Consumed by the W3 Daily Briefing Gutenberg block. **Not exposed** to
//! Agent or MCP actors in v1.4.4; future Agent / MCP exposure requires
//! `/cso` re-approval per AC-507.5.

pub mod contracts;
pub mod producer;

pub use contracts::{
    AmbiguityPair, BriefingAdvisory, BriefingAvailability, BriefingEmptyReason, BriefingFreshness,
    BriefingIntegrity, BriefingSection, BriefingStaleReason, BriefingState, BriefingTrustSummary,
    DailyBriefingInput, DailyBriefingOutput, MeetingBriefRef, SourceAsofRef, WatchProposal,
    BRIEFING_SCHEMA_VERSION,
};

use dailyos_abilities_macro::ability;

use crate::abilities::{AbilityContext, AbilityResult};

#[ability(
    name = "get_daily_briefing",
    category = Read,
    version = "0.1.0",
    schema_version = 1,
    allowed_actors = [User, SurfaceClient],
    allowed_modes = [Live, Simulate, Evaluate],
    requires_confirmation = false,
    may_publish = false,
    required_scopes = ["read.daily_briefing"],
    mcp_exposure = None,
    composes = [
        { id = "get_entity_intelligence", ability = "get_entity_intelligence", optional = false },
        { id = "get_daily_readiness", ability = "get_daily_readiness", optional = false }
    ],
    experimental = false,
    signal_policy = { emits_on_output_change = [], coalesce = false }
)]
pub async fn get_daily_briefing(
    ctx: &AbilityContext<'_>,
    input: DailyBriefingInput,
) -> AbilityResult<DailyBriefingOutput> {
    producer::build_daily_briefing(ctx, input).await
}
