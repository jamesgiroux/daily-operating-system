use crate::{minimal_block_kit_fixture, BlockIntegrationFixture};

pub fn meeting_attendees_section_fixture() -> BlockIntegrationFixture {
    minimal_block_kit_fixture(
        "meeting-attendees-section",
        "span",
        "dailyos-empty-chip",
        &[("data-empty-reason", "missing_meeting_context")],
        "empty-state",
        "No meeting context.",
    )
}

crate::integration_test_block!(
    meeting_attendees_section_block_integration,
    meeting_attendees_section_fixture
);
