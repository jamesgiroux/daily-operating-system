use crate::{minimal_block_kit_fixture, BlockIntegrationFixture};

pub fn unified_timeline_fixture() -> BlockIntegrationFixture {
    minimal_block_kit_fixture(
        "unified-timeline",
        "section",
        "wp-block-dailyos-unified-timeline",
        &[
            ("data-ds-tier", "pattern"),
            ("data-ds-name", "UnifiedTimeline"),
        ],
        "empty-state",
        r#"data-empty-reason="no_envelope""#,
    )
}

crate::integration_test_block!(unified_timeline_block_integration, unified_timeline_fixture);
