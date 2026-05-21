use crate::{minimal_block_kit_fixture, BlockIntegrationFixture};

pub fn meeting_related_entities_fixture() -> BlockIntegrationFixture {
    minimal_block_kit_fixture(
        "meeting-related-entities",
        "span",
        "dailyos-empty-chip",
        &[("data-empty-reason", "missing_meeting_context")],
        "empty-state",
        "No meeting context.",
    )
}

crate::integration_test_block!(
    meeting_related_entities_block_integration,
    meeting_related_entities_fixture
);
