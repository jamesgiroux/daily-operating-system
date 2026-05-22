use crate::{minimal_block_kit_fixture, BlockIntegrationFixture};

pub fn entity_intake_fixture() -> BlockIntegrationFixture {
    minimal_block_kit_fixture(
        "entity-intake",
        "div",
        "dailyos-block-error-panel",
        &[
            ("data-block", "dailyos/entity-intake"),
            ("data-error-code", "InvalidEntityType"),
        ],
        "error-panel",
        r#"data-error-code="InvalidEntityType""#,
    )
}

crate::integration_test_block!(entity_intake_block_integration, entity_intake_fixture);
