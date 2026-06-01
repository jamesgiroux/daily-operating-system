use crate::{minimal_block_kit_fixture, BlockIntegrationFixture};

pub fn recommended_actions_fixture() -> BlockIntegrationFixture {
    minimal_block_kit_fixture(
        "recommended-actions",
        "section",
        "wp-block-dailyos-recommended-actions",
        &[
            ("data-ds-tier", "pattern"),
            ("data-ds-name", "RecommendedActions"),
        ],
        "empty-state",
        r#"data-empty-reason="no_envelope""#,
    )
}

crate::integration_test_block!(
    recommended_actions_block_integration,
    recommended_actions_fixture
);
