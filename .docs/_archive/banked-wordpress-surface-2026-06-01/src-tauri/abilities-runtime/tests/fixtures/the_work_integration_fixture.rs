use crate::{minimal_block_kit_fixture, BlockIntegrationFixture};

pub fn the_work_fixture() -> BlockIntegrationFixture {
    minimal_block_kit_fixture(
        "the-work",
        "section",
        "wp-block-dailyos-the-work",
        &[("data-ds-tier", "pattern"), ("data-ds-name", "TheWork")],
        "empty-state",
        r#"data-empty-reason="no_envelope""#,
    )
}

crate::integration_test_block!(the_work_block_integration, the_work_fixture);
