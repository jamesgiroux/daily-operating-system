use crate::{minimal_block_kit_fixture, BlockIntegrationFixture};

pub fn vitals_strip_fixture() -> BlockIntegrationFixture {
    minimal_block_kit_fixture(
        "vitals-strip",
        "section",
        "wp-block-dailyos-vitals-strip",
        &[("data-ds-tier", "pattern"), ("data-ds-name", "VitalsStrip")],
        "empty-state",
        r#"data-empty-reason="no_envelope""#,
    )
}

crate::integration_test_block!(vitals_strip_block_integration, vitals_strip_fixture);
