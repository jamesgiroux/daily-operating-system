use crate::{minimal_block_kit_fixture, BlockIntegrationFixture};

pub fn meeting_prep_status_fixture() -> BlockIntegrationFixture {
    minimal_block_kit_fixture(
        "meeting-prep-status",
        "div",
        "wp-block-dailyos-meeting-prep-status is-empty",
        &[],
        "empty-state",
        r#"data-empty-reason="missing_meeting_context""#,
    )
}

crate::integration_test_block!(
    meeting_prep_status_block_integration,
    meeting_prep_status_fixture
);
