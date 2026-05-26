use crate::{minimal_block_kit_fixture, BlockIntegrationFixture};

pub fn source_management_fixture() -> BlockIntegrationFixture {
    minimal_block_kit_fixture(
        "source-management",
        "section",
        "wp-block-dailyos-source-management dailyos-source-management",
        &[
            ("data-dailyos-surface", "source-management"),
            ("data-dailyos-state", "not_ready"),
        ],
        "entity-not-selected",
        "Sources appear when an entity is selected.",
    )
}

crate::integration_test_block!(
    source_management_block_integration,
    source_management_fixture
);
