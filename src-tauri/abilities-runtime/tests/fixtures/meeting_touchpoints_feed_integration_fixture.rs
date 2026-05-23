use crate::{minimal_block_kit_fixture, BlockIntegrationFixture};

pub fn meeting_touchpoints_feed_fixture() -> BlockIntegrationFixture {
    minimal_block_kit_fixture(
        "meeting-touchpoints-feed",
        "section",
        "meeting-intel_chapterSection is-empty",
        &[
            ("data-ds-name", "MeetingTouchpointsFeed"),
            ("data-empty-reason", "missing_meeting_context"),
        ],
        "empty-state",
        "No meeting context.",
    )
}

crate::integration_test_block!(
    meeting_touchpoints_feed_block_integration,
    meeting_touchpoints_feed_fixture
);
