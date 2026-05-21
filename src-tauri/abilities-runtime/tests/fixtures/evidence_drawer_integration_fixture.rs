use crate::{minimal_block_kit_fixture, BlockIntegrationFixture};

pub fn evidence_drawer_fixture() -> BlockIntegrationFixture {
    minimal_block_kit_fixture(
        "evidence-drawer",
        "section",
        "wp-block-dailyos-evidence-drawer dailyos-evidence-drawer",
        &[
            ("data-ds-tier", "primitive"),
            ("data-ds-name", "EvidenceDrawer"),
        ],
        "closed-state",
        r#"data-open="false""#,
    )
}

crate::integration_test_block!(evidence_drawer_block_integration, evidence_drawer_fixture);
