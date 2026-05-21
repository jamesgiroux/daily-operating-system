use crate::{minimal_block_kit_fixture, BlockIntegrationFixture};

pub fn meeting_post_meeting_capture_fixture() -> BlockIntegrationFixture {
    minimal_block_kit_fixture(
        "meeting-post-meeting-capture",
        "span",
        "dailyos-empty-chip",
        &[("data-empty-reason", "missing_meeting_context")],
        "empty-state",
        "No meeting context.",
    )
}

crate::integration_test_block!(
    meeting_post_meeting_capture_block_integration,
    meeting_post_meeting_capture_fixture
);
