use abilities_runtime::abilities::composition::{BindingRole, BlockType};
use serde_json::{json, to_value};

use crate::{
    fixture_binding, fixture_block, fixture_claim, fixture_composition, BindingExpectation,
    BlockIntegrationFixture, BlockWrapperAssertion, ProjectionDiagnostic, RendererBranchAssertion,
    ValueKind,
};

pub fn entity_chip_fixture() -> BlockIntegrationFixture {
    let ability_name = "dailyos/entity-chip";
    let composition_id = "dailyos/entity-chip:primitive:entity-chip-test-001";
    let claims = vec![fixture_claim("claim-entity-chip-label", "/payload/text")];
    let block = fixture_block(
        "block-entity-chip",
        BlockType::EntityChip,
        json!({
            "payload": {
                "text": "Acme Corp"
            }
        }),
        claims,
        vec![fixture_binding("/payload/text", BindingRole::Source, &[0])],
    );
    let composition = fixture_composition(ability_name, composition_id, 1, vec![block]);

    BlockIntegrationFixture {
        ability_name: ability_name.to_string(),
        composition_id: composition_id.to_string(),
        input_json: to_value(composition).expect("entity-chip fixture serializes"),
        expected_bindings: vec![BindingExpectation {
            pointer: "/blocks/0/payload/payload/text".to_string(),
            value_kind: ValueKind::String,
            required: true,
        }],
        expected_diagnostics: Vec::<ProjectionDiagnostic>::new(),
        expected_renderer_branches: vec![RendererBranchAssertion {
            branch_label: "primitive-marker".to_string(),
            expected_html_pattern: "data-ds-name=\"EntityChip\"".to_string(),
        }],
        expected_wrapper: BlockWrapperAssertion {
            tag: "span".to_string(),
            class: "wp-block-dailyos-entity-chip".to_string(),
            data_attrs: vec![
                ("data-ds-tier".to_string(), "primitive".to_string()),
                ("data-ds-name".to_string(), "EntityChip".to_string()),
            ],
        },
    }
}

crate::integration_test_block!(entity_chip_block_integration, entity_chip_fixture);
