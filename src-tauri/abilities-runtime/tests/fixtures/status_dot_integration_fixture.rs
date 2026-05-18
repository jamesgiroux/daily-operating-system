use abilities_runtime::abilities::composition::{BindingRole, BlockType};
use serde_json::{json, to_value};

use crate::{
    fixture_binding, fixture_block, fixture_claim, fixture_composition, BindingExpectation,
    BlockIntegrationFixture, BlockWrapperAssertion, ProjectionDiagnostic, RendererBranchAssertion,
    ValueKind,
};

pub fn status_dot_fixture() -> BlockIntegrationFixture {
    let ability_name = "dailyos/status-dot";
    let composition_id = "dailyos/status-dot:primitive:status-dot-test-001";
    let claims = vec![fixture_claim("claim-status-dot-label", "/payload/text")];
    let block = fixture_block(
        "block-status-dot",
        BlockType::StatusDot,
        json!({
            "payload": {
                "text": "Connected"
            }
        }),
        claims,
        vec![fixture_binding("/payload/text", BindingRole::Source, &[0])],
    );
    let composition = fixture_composition(ability_name, composition_id, 1, vec![block]);

    BlockIntegrationFixture {
        ability_name: ability_name.to_string(),
        composition_id: composition_id.to_string(),
        input_json: to_value(composition).expect("status-dot fixture serializes"),
        expected_bindings: vec![BindingExpectation {
            pointer: "/blocks/0/payload/payload/text".to_string(),
            value_kind: ValueKind::String,
            required: true,
        }],
        expected_diagnostics: Vec::<ProjectionDiagnostic>::new(),
        expected_renderer_branches: vec![RendererBranchAssertion {
            branch_label: "primitive-marker".to_string(),
            expected_html_pattern: "data-ds-name=\"StatusDot\"".to_string(),
        }],
        expected_wrapper: BlockWrapperAssertion {
            tag: "span".to_string(),
            class: "dailyos-status-dot".to_string(),
            data_attrs: vec![
                ("data-ds-tier".to_string(), "primitive".to_string()),
                ("data-ds-name".to_string(), "StatusDot".to_string()),
            ],
        },
    }
}

crate::integration_test_block!(status_dot_block_integration, status_dot_fixture);
