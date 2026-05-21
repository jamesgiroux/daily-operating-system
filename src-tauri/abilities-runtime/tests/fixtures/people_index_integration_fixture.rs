use crate::{minimal_block_kit_fixture, BlockIntegrationFixture};

pub fn people_index_fixture() -> BlockIntegrationFixture {
    minimal_block_kit_fixture(
        "people-index",
        "section",
        "wp-block-dailyos-people-index",
        &[("data-ds-tier", "pattern"), ("data-ds-name", "PeopleIndex")],
        "list-ability",
        r#"data-dailyos-ability="list_people""#,
    )
}

crate::integration_test_block!(people_index_block_integration, people_index_fixture);
