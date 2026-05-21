use crate::{minimal_block_kit_fixture, BlockIntegrationFixture};

pub fn metadata_proposal_cue_fixture() -> BlockIntegrationFixture {
    minimal_block_kit_fixture(
        "metadata-proposal-cue",
        "div",
        "wp-block-dailyos-metadata-proposal-cue wp-block-dailyos-metadata-proposal-cue--empty",
        &[],
        "empty-state",
        r#"data-empty-reason="not_available""#,
    )
}

crate::integration_test_block!(
    metadata_proposal_cue_block_integration,
    metadata_proposal_cue_fixture
);
