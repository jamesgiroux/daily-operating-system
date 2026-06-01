use crate::{minimal_block_kit_fixture, BlockIntegrationFixture};

pub fn metadata_proposal_drawer_fixture() -> BlockIntegrationFixture {
    minimal_block_kit_fixture(
        "metadata-proposal-drawer",
        "div",
        "wp-block-dailyos-metadata-proposal-drawer wp-block-dailyos-metadata-proposal-drawer--empty",
        &[],
        "empty-state",
        r#"data-empty-reason="not_available""#,
    )
}

crate::integration_test_block!(
    metadata_proposal_drawer_block_integration,
    metadata_proposal_drawer_fixture
);
