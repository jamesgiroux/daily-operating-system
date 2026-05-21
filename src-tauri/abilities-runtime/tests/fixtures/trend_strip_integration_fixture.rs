use crate::{minimal_block_kit_fixture, BlockIntegrationFixture};

pub fn trend_strip_fixture() -> BlockIntegrationFixture {
    minimal_block_kit_fixture(
        "trend-strip",
        "span",
        "wp-block-dailyos-trend-strip dailyos-trend-strip",
        &[
            ("data-ds-tier", "primitive"),
            ("data-ds-name", "TrendStrip"),
        ],
        "default-direction",
        r#"data-direction="stable""#,
    )
}

crate::integration_test_block!(trend_strip_block_integration, trend_strip_fixture);
