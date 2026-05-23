use crate::{minimal_block_kit_fixture, BlockIntegrationFixture};

pub fn meeting_header_fixture() -> BlockIntegrationFixture {
    minimal_block_kit_fixture(
        "meeting-header",
        "header",
        "wp-block-dailyos-meeting-header is-empty",
        &[("data-empty-reason", "missing_meeting_context")],
        "empty-state",
        "No meeting context.",
    )
}

crate::integration_test_block!(meeting_header_block_integration, meeting_header_fixture);
