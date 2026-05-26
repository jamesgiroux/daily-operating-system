//! `get_daily_briefing` Read/User-only ability + composed
//! `BriefingState` (cycle-1 correctness F3).
//!
//! Read-side composition over existing meeting prep status (§5.5) +
//! per-subject entity intelligence (§5.1) + daily readiness context.
//! Per `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md`
//! §5.10.
//!
//! Consumed by the W3 Daily Briefing Gutenberg block and v1.4.7 MCP v2 read
//! surface. MCP exposure is read-only and routes through the request-scoped
//! abilities bridge so the runtime render policy still owns sensitivity and
//! provenance handling.

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
    allowed_actors = [User, McpClient],
    allowed_modes = [Live, Simulate, Evaluate],
    requires_confirmation = false,
    may_publish = false,
    required_scopes = ["read.daily_briefing"],
    mcp_exposure = Invocable,
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
