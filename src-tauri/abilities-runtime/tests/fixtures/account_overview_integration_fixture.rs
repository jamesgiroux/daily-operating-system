use crate::{minimal_block_kit_fixture, BlockIntegrationFixture};

pub fn account_overview_fixture() -> BlockIntegrationFixture {
    minimal_block_kit_fixture(
        "account-overview",
        "section",
        "wp-block-dailyos-account-overview",
        &[
            ("data-ds-tier", "pattern"),
            ("data-ds-name", "AccountOverview"),
            ("data-dailyos-surface", "account_overview"),
        ],
        "surface-marker",
        r#"data-dailyos-surface="account_overview""#,
    )
}

crate::integration_test_block!(account_overview_block_integration, account_overview_fixture);
