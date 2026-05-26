//! Portfolio attention read ability.
//!
//! Ranks subjects that currently need attention from claim-backed evidence:
//! open loops, account health, freshness, trust, and salience where the
//! salience service is available. MCP v2 consumes this ability through
//! `dailyos.read.portfolio_attention`; it is not a legacy account-table
//! bypass.

pub mod contracts;
pub mod producer;

pub use contracts::{
    AttentionEvidence, AttentionReason, AttentionScore, PortfolioAttentionInput,
    PortfolioAttentionItem, PortfolioAttentionResult, PortfolioAttentionSubject,
    PORTFOLIO_ATTENTION_ABILITY_NAME, PORTFOLIO_ATTENTION_SCHEMA_VERSION,
    PORTFOLIO_ATTENTION_SCOPE,
};

use dailyos_abilities_macro::ability;

use crate::abilities::{AbilityContext, AbilityResult};

#[ability(
    name = "portfolio_attention",
    category = Read,
    version = "1.0.0",
    schema_version = 1,
    allowed_actors = [User, Agent, System, SurfaceClient, McpClient],
    allowed_modes = [Live, Evaluate],
    requires_confirmation = false,
    may_publish = false,
    required_scopes = ["read.portfolio_attention"],
    mcp_exposure = Invocable,
    composes = [
        { id = "list_accounts", ability = "list_accounts", optional = false },
        { id = "list_open_loops", ability = "list_open_loops", optional = false },
        { id = "score_salience", ability = "score_salience", optional = true }
    ],
    experimental = false,
    signal_policy = { emits_on_output_change = [], coalesce = false }
)]
pub async fn portfolio_attention(
    ctx: &AbilityContext<'_>,
    input: PortfolioAttentionInput,
) -> AbilityResult<PortfolioAttentionResult> {
    producer::portfolio_attention(ctx, input).await
}
