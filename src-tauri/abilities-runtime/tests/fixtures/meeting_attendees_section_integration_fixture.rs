use crate::{minimal_block_kit_fixture, BlockIntegrationFixture};

pub fn meeting_attendees_section_fixture() -> BlockIntegrationFixture {
    minimal_block_kit_fixture(
        "meeting-attendees-section",
        "section",
        "wp-block-dailyos-meeting-attendees-section is-empty",
        &[],
        "empty-state",
        "No meeting context.",
    )
}

crate::integration_test_block!(
    meeting_attendees_section_block_integration,
    meeting_attendees_section_fixture
);
