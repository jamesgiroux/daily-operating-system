use crate::{minimal_block_kit_fixture, BlockIntegrationFixture};

pub fn person_hero_fixture() -> BlockIntegrationFixture {
    minimal_block_kit_fixture(
        "person-hero",
        "section",
        "wp-block-dailyos-person-hero",
        &[("data-ds-tier", "pattern"), ("data-ds-name", "PersonHero")],
        "empty-state",
        r#"data-empty-reason="no_envelope""#,
    )
}

crate::integration_test_block!(person_hero_block_integration, person_hero_fixture);
