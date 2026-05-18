use abilities_runtime::abilities::composition::{BindingRole, BlockType};
use serde_json::{json, to_value};

use crate::{
    fixture_binding, fixture_block, fixture_claim, fixture_composition, BindingExpectation,
    BlockIntegrationFixture, BlockWrapperAssertion, ProjectionDiagnostic, RendererBranchAssertion,
    ValueKind,
};

pub fn pill_fixture() -> BlockIntegrationFixture {
    let ability_name = "dailyos/pill";
    let composition_id = "dailyos/pill:primitive:pill-test-001";
    let claims = vec![fixture_claim("claim-pill-label", "/payload/text")];
    let block = fixture_block(
        "block-pill",
        BlockType::Pill,
        json!({
            "payload": {
                "text": "Ready"
            }
        }),
        claims,
        vec![fixture_binding("/payload/text", BindingRole::Source, &[0])],
    );
    let composition = fixture_composition(ability_name, composition_id, 1, vec![block]);

    BlockIntegrationFixture {
        ability_name: ability_name.to_string(),
        composition_id: composition_id.to_string(),
        input_json: to_value(composition).expect("pill fixture serializes"),
        expected_bindings: vec![BindingExpectation {
            pointer: "/blocks/0/payload/payload/text".to_string(),
            value_kind: ValueKind::String,
            required: true,
        }],
        expected_diagnostics: Vec::<ProjectionDiagnostic>::new(),
        expected_renderer_branches: vec![
            RendererBranchAssertion {
                branch_label: "primitive-marker".to_string(),
                expected_html_pattern: "data-ds-name=\"Pill\"".to_string(),
            },
            RendererBranchAssertion {
                branch_label: "default-tone".to_string(),
                expected_html_pattern: "data-tone=\"neutral\"".to_string(),
            },
        ],
        expected_wrapper: BlockWrapperAssertion {
            tag: "span".to_string(),
            class: "dailyos-pill".to_string(),
            data_attrs: vec![
                ("data-ds-tier".to_string(), "primitive".to_string()),
                ("data-ds-name".to_string(), "Pill".to_string()),
            ],
        },
    }
}

crate::integration_test_block!(pill_block_integration, pill_fixture);
