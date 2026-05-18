use abilities_runtime::abilities::composition::{BindingRole, BlockType};
use serde_json::{json, to_value};

use crate::{
    fixture_binding, fixture_block, fixture_claim, fixture_composition, BindingExpectation,
    BlockIntegrationFixture, BlockWrapperAssertion, ProjectionDiagnostic, RendererBranchAssertion,
    ValueKind,
};

pub fn trust_band_badge_fixture() -> BlockIntegrationFixture {
    let ability_name = "dailyos/trust-band-badge";
    let composition_id = "dailyos/trust-band-badge:claim:claim-trust-001";
    let claims = vec![fixture_claim("claim-trust-band", "/trust/band")];
    let block = fixture_block(
        "block-trust-band-badge",
        BlockType::TrustBandBadge,
        json!({
            "band": "needs_verification",
            "compact": true
        }),
        claims,
        vec![fixture_binding("/band", BindingRole::Source, &[0])],
    );
    let composition = fixture_composition(ability_name, composition_id, 4, vec![block]);

    BlockIntegrationFixture {
        ability_name: ability_name.to_string(),
        composition_id: composition_id.to_string(),
        input_json: to_value(composition).expect("trust band badge fixture serializes"),
        expected_bindings: vec![
            BindingExpectation {
                pointer: "/blocks/0/payload/band".to_string(),
                value_kind: ValueKind::String,
                required: true,
            },
            BindingExpectation {
                pointer: "/blocks/0/payload/compact".to_string(),
                value_kind: ValueKind::Bool,
                required: true,
            },
        ],
        expected_diagnostics: Vec::<ProjectionDiagnostic>::new(),
        expected_renderer_branches: vec![
            RendererBranchAssertion {
                branch_label: "label".to_string(),
                expected_html_pattern: "Needs verification".to_string(),
            },
            RendererBranchAssertion {
                branch_label: "band".to_string(),
                expected_html_pattern: r#"data-band="needs_verification""#.to_string(),
            },
            RendererBranchAssertion {
                branch_label: "compact".to_string(),
                expected_html_pattern: "dailyos-trust-band-badge--compact".to_string(),
            },
        ],
        expected_wrapper: BlockWrapperAssertion {
            tag: "span".to_string(),
            class: "dailyos-trust-band-badge dailyos-trust-band-badge--compact".to_string(),
            data_attrs: vec![
                ("data-ds-tier".to_string(), "primitive".to_string()),
                ("data-ds-name".to_string(), "TrustBandBadge".to_string()),
            ],
        },
    }
}

crate::integration_test_block!(trust_band_badge_block_integration, trust_band_badge_fixture);
