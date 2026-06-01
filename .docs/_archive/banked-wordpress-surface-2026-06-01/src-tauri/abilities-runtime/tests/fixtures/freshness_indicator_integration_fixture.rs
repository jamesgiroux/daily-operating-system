use abilities_runtime::abilities::composition::{BindingRole, BlockType};
use serde_json::{json, to_value};

use crate::{
    fixture_binding, fixture_block, fixture_claim, fixture_composition, BindingExpectation,
    BlockIntegrationFixture, BlockWrapperAssertion, ProjectionDiagnostic, RendererBranchAssertion,
    ValueKind,
};

pub fn freshness_indicator_fixture() -> BlockIntegrationFixture {
    let ability_name = "dailyos/freshness-indicator";
    let composition_id = "dailyos/freshness-indicator:claim:claim-freshness-001";
    let claims = vec![fixture_claim("claim-freshness", "/source/asof")];
    let block = fixture_block(
        "block-freshness-indicator",
        BlockType::FreshnessIndicator,
        json!({
            "at": "2099-01-01T00:00:00Z",
            "format": "relative",
            "stalenessThreshold": 48,
            "variant": "inline"
        }),
        claims,
        vec![fixture_binding("/at", BindingRole::Source, &[0])],
    );
    let composition = fixture_composition(ability_name, composition_id, 3, vec![block]);

    BlockIntegrationFixture {
        ability_name: ability_name.to_string(),
        composition_id: composition_id.to_string(),
        input_json: to_value(composition).expect("freshness indicator fixture serializes"),
        expected_bindings: vec![
            BindingExpectation {
                pointer: "/blocks/0/payload/at".to_string(),
                value_kind: ValueKind::String,
                required: true,
            },
            BindingExpectation {
                pointer: "/blocks/0/payload/format".to_string(),
                value_kind: ValueKind::String,
                required: true,
            },
            BindingExpectation {
                pointer: "/blocks/0/payload/stalenessThreshold".to_string(),
                value_kind: ValueKind::Number,
                required: true,
            },
        ],
        expected_diagnostics: Vec::<ProjectionDiagnostic>::new(),
        expected_renderer_branches: vec![
            RendererBranchAssertion {
                branch_label: "server-relative-label".to_string(),
                expected_html_pattern: ">now<".to_string(),
            },
            RendererBranchAssertion {
                branch_label: "staleness".to_string(),
                expected_html_pattern: r#"data-staleness="fresh""#.to_string(),
            },
            RendererBranchAssertion {
                branch_label: "design-system-metadata".to_string(),
                expected_html_pattern: r#"data-ds-name="FreshnessIndicator""#.to_string(),
            },
        ],
        expected_wrapper: BlockWrapperAssertion {
            tag: "span".to_string(),
            class: "dailyos-freshness-indicator dailyos-freshness-indicator--inline".to_string(),
            data_attrs: vec![
                ("data-ds-tier".to_string(), "primitive".to_string()),
                ("data-ds-name".to_string(), "FreshnessIndicator".to_string()),
            ],
        },
    }
}

crate::integration_test_block!(
    freshness_indicator_block_integration,
    freshness_indicator_fixture
);
