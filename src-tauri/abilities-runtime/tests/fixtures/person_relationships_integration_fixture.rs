use crate::{minimal_block_kit_fixture, BlockIntegrationFixture};

pub fn person_relationships_fixture() -> BlockIntegrationFixture {
    minimal_block_kit_fixture(
        "person-relationships",
        "section",
        "wp-block-dailyos-person-relationships",
        &[
            ("data-ds-tier", "pattern"),
            ("data-ds-name", "PersonRelationships"),
        ],
        "empty-state",
        r#"data-empty-reason="no_envelope""#,
    )
}

crate::integration_test_block!(
    person_relationships_block_integration,
    person_relationships_fixture
);
