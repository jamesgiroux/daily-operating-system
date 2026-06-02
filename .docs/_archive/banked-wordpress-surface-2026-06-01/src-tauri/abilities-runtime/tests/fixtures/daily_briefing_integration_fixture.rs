use crate::{minimal_block_kit_fixture, BlockIntegrationFixture};

pub fn daily_briefing_fixture() -> BlockIntegrationFixture {
    minimal_block_kit_fixture(
        "daily-briefing",
        "div",
        "wp-block-dailyos-daily-briefing is-unavailable",
        &[],
        "runtime-unavailable",
        "Runtime unavailable.",
    )
}

crate::integration_test_block!(daily_briefing_block_integration, daily_briefing_fixture);
