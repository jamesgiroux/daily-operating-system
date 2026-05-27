use crate::{minimal_block_kit_fixture, BlockIntegrationFixture};

pub fn meeting_context_bundle_fixture() -> BlockIntegrationFixture {
    minimal_block_kit_fixture(
        "meeting-context-bundle",
        "span",
        "dailyos-empty-chip wp-block-dailyos-meeting-context-bundle is-empty",
        &[("data-empty-reason", "missing_meeting_context")],
        "empty-state",
        "No meeting context.",
    )
}

crate::integration_test_block!(
    meeting_context_bundle_block_integration,
    meeting_context_bundle_fixture
);
