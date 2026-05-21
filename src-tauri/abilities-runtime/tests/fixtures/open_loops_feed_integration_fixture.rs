use crate::{minimal_block_kit_fixture, BlockIntegrationFixture};

pub fn open_loops_feed_fixture() -> BlockIntegrationFixture {
    minimal_block_kit_fixture(
        "open-loops-feed",
        "section",
        "wp-block-dailyos-open-loops-feed",
        &[
            ("data-ds-tier", "pattern"),
            ("data-ds-name", "OpenLoopsFeed"),
        ],
        "empty-state",
        r#"data-empty-reason="no_envelope""#,
    )
}

crate::integration_test_block!(open_loops_feed_block_integration, open_loops_feed_fixture);
