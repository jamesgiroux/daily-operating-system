use crate::{minimal_block_kit_fixture, BlockIntegrationFixture};

pub fn account_detail_fixture() -> BlockIntegrationFixture {
    minimal_block_kit_fixture(
        "account-detail",
        "div",
        "wp-block-dailyos-account-detail wp-block-dailyos-account-detail--empty",
        &[],
        "empty-state",
        r#"data-empty-reason="no_account_id""#,
    )
}

crate::integration_test_block!(account_detail_block_integration, account_detail_fixture);
