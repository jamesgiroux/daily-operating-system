use crate::{minimal_block_kit_fixture, BlockIntegrationFixture};

pub fn suggested_next_steps_fixture() -> BlockIntegrationFixture {
    minimal_block_kit_fixture(
        "suggested-next-steps",
        "section",
        "editorial-reveal entity-detail_chapterSection SuggestedNextSteps_section SuggestedNextSteps_section--empty is-empty",
        &[("data-dailyos-block", "suggested-next-steps")],
        "empty-state",
        "No subject context.",
    )
}

crate::integration_test_block!(
    suggested_next_steps_block_integration,
    suggested_next_steps_fixture
);
