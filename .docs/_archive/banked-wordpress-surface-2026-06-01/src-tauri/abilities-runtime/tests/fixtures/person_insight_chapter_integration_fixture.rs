use crate::{minimal_block_kit_fixture, BlockIntegrationFixture};

pub fn person_insight_chapter_fixture() -> BlockIntegrationFixture {
    minimal_block_kit_fixture(
        "person-insight-chapter",
        "section",
        "wp-block-dailyos-person-insight-chapter",
        &[
            ("data-ds-tier", "pattern"),
            ("data-ds-name", "PersonInsightChapter"),
        ],
        "empty-state",
        r#"data-empty-reason="no_envelope""#,
    )
}

crate::integration_test_block!(
    person_insight_chapter_block_integration,
    person_insight_chapter_fixture
);
