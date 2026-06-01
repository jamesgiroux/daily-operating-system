use crate::{minimal_block_kit_fixture, BlockIntegrationFixture};

pub fn accounts_index_fixture() -> BlockIntegrationFixture {
    minimal_block_kit_fixture(
        "accounts-index",
        "section",
        "wp-block-dailyos-accounts-index",
        &[
            ("data-ds-tier", "pattern"),
            ("data-ds-name", "AccountsIndex"),
        ],
        "list-ability",
        r#"data-dailyos-ability="list_accounts""#,
    )
}

crate::integration_test_block!(accounts_index_block_integration, accounts_index_fixture);
