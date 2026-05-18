use abilities_runtime::abilities::composition::{BindingRole, BlockType};
use serde_json::{json, to_value};

use crate::{
    fixture_binding, fixture_block, fixture_claim, fixture_composition, BindingExpectation,
    BlockIntegrationFixture, BlockWrapperAssertion, ProjectionDiagnostic, RendererBranchAssertion,
    ValueKind,
};

pub fn provenance_tag_fixture() -> BlockIntegrationFixture {
    let ability_name = "dailyos/provenance-tag";
    let composition_id = "dailyos/provenance-tag:primitive:provenance-tag-test-001";
    let claims = vec![fixture_claim(
        "claim-provenance-tag-source",
        "/payload/source",
    )];
    let block = fixture_block(
        "block-provenance-tag",
        BlockType::ProvenanceTag,
        json!({
            "payload": {
                "source": "glean",
                "age": "2026-05-18T00:00:00Z"
            }
        }),
        claims,
        vec![fixture_binding(
            "/payload/source",
            BindingRole::Source,
            &[0],
        )],
    );
    let composition = fixture_composition(ability_name, composition_id, 1, vec![block]);

    BlockIntegrationFixture {
        ability_name: ability_name.to_string(),
        composition_id: composition_id.to_string(),
        input_json: to_value(composition).expect("provenance-tag fixture serializes"),
        expected_bindings: vec![BindingExpectation {
            pointer: "/blocks/0/payload/payload/source".to_string(),
            value_kind: ValueKind::String,
            required: true,
        }],
        expected_diagnostics: Vec::<ProjectionDiagnostic>::new(),
        expected_renderer_branches: vec![RendererBranchAssertion {
            branch_label: "primitive-marker".to_string(),
            expected_html_pattern: "data-ds-name=\"ProvenanceTag\"".to_string(),
        }],
        expected_wrapper: BlockWrapperAssertion {
            tag: "span".to_string(),
            class: "dailyos-provenance-tag".to_string(),
            data_attrs: vec![
                ("data-ds-tier".to_string(), "primitive".to_string()),
                ("data-ds-name".to_string(), "ProvenanceTag".to_string()),
            ],
        },
    }
}

crate::integration_test_block!(provenance_tag_block_integration, provenance_tag_fixture);
