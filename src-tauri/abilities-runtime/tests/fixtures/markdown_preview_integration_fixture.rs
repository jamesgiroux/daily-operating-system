use crate::{minimal_block_kit_fixture, BlockIntegrationFixture};

pub fn markdown_preview_fixture() -> BlockIntegrationFixture {
    minimal_block_kit_fixture(
        "markdown-preview",
        "section",
        "wp-block-dailyos-markdown-preview",
        &[
            ("data-dailyos-surface", "markdown-preview"),
            ("data-dailyos-state", "not_ready"),
        ],
        "source-not-selected",
        "Preview appears when a source is selected.",
    )
}

crate::integration_test_block!(markdown_preview_block_integration, markdown_preview_fixture);
