use crate::{minimal_block_kit_fixture, BlockIntegrationFixture};

pub fn person_appendix_fixture() -> BlockIntegrationFixture {
    minimal_block_kit_fixture(
        "person-appendix",
        "section",
        "wp-block-dailyos-person-appendix",
        &[
            ("data-ds-tier", "pattern"),
            ("data-ds-name", "PersonAppendix"),
        ],
        "empty-state",
        r#"data-empty-reason="no_envelope""#,
    )
}

crate::integration_test_block!(person_appendix_block_integration, person_appendix_fixture);
