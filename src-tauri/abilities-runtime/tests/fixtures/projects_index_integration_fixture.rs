use crate::{minimal_block_kit_fixture, BlockIntegrationFixture};

pub fn projects_index_fixture() -> BlockIntegrationFixture {
    minimal_block_kit_fixture(
        "projects-index",
        "section",
        "wp-block-dailyos-projects-index",
        &[
            ("data-ds-tier", "pattern"),
            ("data-ds-name", "ProjectsIndex"),
        ],
        "list-ability",
        r#"data-dailyos-ability="list_projects""#,
    )
}

crate::integration_test_block!(projects_index_block_integration, projects_index_fixture);
