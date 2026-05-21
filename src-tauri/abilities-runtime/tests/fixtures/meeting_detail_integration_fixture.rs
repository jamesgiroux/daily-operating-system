use crate::{minimal_block_kit_fixture, BlockIntegrationFixture};

pub fn meeting_detail_fixture() -> BlockIntegrationFixture {
    minimal_block_kit_fixture(
        "meeting-detail",
        "div",
        "wp-block-dailyos-meeting-detail is-empty",
        &[("data-empty-reason", "missing_meeting_id")],
        "empty-state",
        "No meeting to show here.",
    )
}

crate::integration_test_block!(meeting_detail_block_integration, meeting_detail_fixture);
