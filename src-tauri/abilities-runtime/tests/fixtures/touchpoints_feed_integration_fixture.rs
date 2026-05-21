use crate::{minimal_block_kit_fixture, BlockIntegrationFixture};

pub fn touchpoints_feed_fixture() -> BlockIntegrationFixture {
    minimal_block_kit_fixture(
        "touchpoints-feed",
        "section",
        "wp-block-dailyos-touchpoints-feed",
        &[
            ("data-ds-tier", "pattern"),
            ("data-ds-name", "TouchpointsFeed"),
        ],
        "empty-state",
        r#"data-empty-reason="no_envelope""#,
    )
}

crate::integration_test_block!(touchpoints_feed_block_integration, touchpoints_feed_fixture);
