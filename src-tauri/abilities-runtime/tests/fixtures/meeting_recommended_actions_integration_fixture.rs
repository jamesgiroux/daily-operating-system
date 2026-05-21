use crate::{minimal_block_kit_fixture, BlockIntegrationFixture};

pub fn meeting_recommended_actions_fixture() -> BlockIntegrationFixture {
    minimal_block_kit_fixture(
        "meeting-recommended-actions",
        "span",
        "dailyos-empty-chip",
        &[("data-empty-reason", "missing_meeting_context")],
        "empty-state",
        "No meeting context.",
    )
}

crate::integration_test_block!(
    meeting_recommended_actions_block_integration,
    meeting_recommended_actions_fixture
);
