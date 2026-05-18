use abilities_runtime::abilities::composition::{BindingRole, BlockType};
use serde_json::{json, to_value};

use crate::{
    fixture_binding, fixture_block, fixture_claim, fixture_composition, BindingExpectation,
    BlockIntegrationFixture, BlockWrapperAssertion, ProjectionDiagnostic, RendererBranchAssertion,
    ValueKind,
};

pub fn type_badge_fixture() -> BlockIntegrationFixture {
    let ability_name = "dailyos/type-badge";
    let composition_id = "dailyos/type-badge:primitive:type-badge-test-001";
    let claims = vec![fixture_claim("claim-type-badge-label", "/payload/text")];
    let block = fixture_block(
        "block-type-badge",
        BlockType::TypeBadge,
        json!({
            "payload": {
                "text": "Customer"
            }
        }),
        claims,
        vec![fixture_binding("/payload/text", BindingRole::Source, &[0])],
    );
    let composition = fixture_composition(ability_name, composition_id, 1, vec![block]);

    BlockIntegrationFixture {
        ability_name: ability_name.to_string(),
        composition_id: composition_id.to_string(),
        input_json: to_value(composition).expect("type-badge fixture serializes"),
        expected_bindings: vec![BindingExpectation {
            pointer: "/blocks/0/payload/payload/text".to_string(),
            value_kind: ValueKind::String,
            required: true,
        }],
        expected_diagnostics: Vec::<ProjectionDiagnostic>::new(),
        expected_renderer_branches: vec![RendererBranchAssertion {
            branch_label: "primitive-marker".to_string(),
            expected_html_pattern: "data-ds-name=\"TypeBadge\"".to_string(),
        }],
        expected_wrapper: BlockWrapperAssertion {
            tag: "section".to_string(),
            class: "wp-block-dailyos-type-badge".to_string(),
            data_attrs: vec![
                ("data-ds-tier".to_string(), "pattern".to_string()),
                ("data-ds-name".to_string(), "TypeBadge".to_string()),
            ],
        },
    }
}

crate::integration_test_block!(type_badge_block_integration, type_badge_fixture);
