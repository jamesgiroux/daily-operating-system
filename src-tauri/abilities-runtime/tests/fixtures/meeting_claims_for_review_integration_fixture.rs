use crate::{minimal_block_kit_fixture, BlockIntegrationFixture};

pub fn meeting_claims_for_review_fixture() -> BlockIntegrationFixture {
    minimal_block_kit_fixture(
        "meeting-claims-for-review",
        "section",
        "wp-block-dailyos-meeting-claims-for-review is-empty",
        &[("data-empty-reason", "missing_meeting_context")],
        "empty-state",
        "No meeting context.",
    )
}

crate::integration_test_block!(
    meeting_claims_for_review_block_integration,
    meeting_claims_for_review_fixture
);
