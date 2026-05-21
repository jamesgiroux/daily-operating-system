use crate::{minimal_block_kit_fixture, BlockIntegrationFixture};

pub fn person_detail_fixture() -> BlockIntegrationFixture {
    minimal_block_kit_fixture(
        "person-detail",
        "div",
        "wp-block-dailyos-person-detail is-empty",
        &[],
        "empty-state",
        "No person to show here.",
    )
}

crate::integration_test_block!(person_detail_block_integration, person_detail_fixture);
