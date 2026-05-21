use crate::{minimal_block_kit_fixture, BlockIntegrationFixture};

pub fn person_network_fixture() -> BlockIntegrationFixture {
    minimal_block_kit_fixture(
        "person-network",
        "section",
        "wp-block-dailyos-person-network",
        &[
            ("data-ds-tier", "pattern"),
            ("data-ds-name", "PersonNetwork"),
        ],
        "empty-state",
        r#"data-empty-reason="no_envelope""#,
    )
}

crate::integration_test_block!(person_network_block_integration, person_network_fixture);
