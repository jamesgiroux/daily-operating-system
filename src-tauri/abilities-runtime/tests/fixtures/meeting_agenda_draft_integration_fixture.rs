use crate::{minimal_block_kit_fixture, BlockIntegrationFixture};

pub fn meeting_agenda_draft_fixture() -> BlockIntegrationFixture {
    minimal_block_kit_fixture(
        "meeting-agenda-draft",
        "span",
        "dailyos-empty-chip",
        &[("data-empty-reason", "missing_meeting_context")],
        "empty-state",
        "No meeting context.",
    )
}

crate::integration_test_block!(
    meeting_agenda_draft_block_integration,
    meeting_agenda_draft_fixture
);
