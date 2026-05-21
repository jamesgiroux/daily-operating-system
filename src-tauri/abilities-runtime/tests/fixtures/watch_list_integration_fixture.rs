use crate::{minimal_block_kit_fixture, BlockIntegrationFixture};

pub fn watch_list_fixture() -> BlockIntegrationFixture {
    minimal_block_kit_fixture(
        "watch-list",
        "section",
        "wp-block-dailyos-watch-list",
        &[("data-ds-tier", "pattern"), ("data-ds-name", "WatchList")],
        "empty-state",
        r#"data-empty-reason="no_envelope""#,
    )
}

crate::integration_test_block!(watch_list_block_integration, watch_list_fixture);
