use crate::{minimal_block_kit_fixture, BlockIntegrationFixture};

pub fn project_detail_fixture() -> BlockIntegrationFixture {
    minimal_block_kit_fixture(
        "project-detail",
        "div",
        "wp-block-dailyos-project-detail is-empty",
        &[],
        "empty-state",
        "No project to show here.",
    )
}

crate::integration_test_block!(project_detail_block_integration, project_detail_fixture);
