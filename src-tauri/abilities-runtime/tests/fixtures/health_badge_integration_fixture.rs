use abilities_runtime::abilities::composition::{BindingRole, BlockType};
use serde_json::{json, to_value};

use crate::{
    fixture_binding, fixture_block, fixture_claim, fixture_composition, BindingExpectation,
    BlockIntegrationFixture, BlockWrapperAssertion, ProjectionDiagnostic, RendererBranchAssertion,
    ValueKind,
};

pub fn health_badge_fixture() -> BlockIntegrationFixture {
    let ability_name = "dailyos/health-badge";
    let composition_id = "dailyos/health-badge:account:acct-health-001";
    let claims = vec![fixture_claim("claim-health-score", "/health/score")];
    let block = fixture_block(
        "block-health-badge",
        BlockType::HealthBadge,
        json!({
            "score": 84,
            "band": "green",
            "trend": {
                "direction": "improving",
                "rationale": "Coverage improved across active dimensions."
            },
            "confidence": 0.82,
            "sufficientData": true,
            "showScore": true,
            "size": "standard",
            "source": "Health model"
        }),
        claims,
        vec![fixture_binding("/score", BindingRole::Source, &[0])],
    );
    let composition = fixture_composition(ability_name, composition_id, 1, vec![block]);

    BlockIntegrationFixture {
        ability_name: ability_name.to_string(),
        composition_id: composition_id.to_string(),
        input_json: to_value(composition).expect("health badge fixture serializes"),
        expected_bindings: vec![
            BindingExpectation {
                pointer: "/blocks/0/payload/score".to_string(),
                value_kind: ValueKind::Number,
                required: true,
            },
            BindingExpectation {
                pointer: "/blocks/0/payload/band".to_string(),
                value_kind: ValueKind::String,
                required: true,
            },
            BindingExpectation {
                pointer: "/blocks/0/payload/trend".to_string(),
                value_kind: ValueKind::Object,
                required: true,
            },
            BindingExpectation {
                pointer: "/blocks/0/payload/sufficientData".to_string(),
                value_kind: ValueKind::Bool,
                required: true,
            },
            BindingExpectation {
                pointer: "/blocks/0/payload/size".to_string(),
                value_kind: ValueKind::String,
                required: true,
            },
        ],
        expected_diagnostics: Vec::<ProjectionDiagnostic>::new(),
        expected_renderer_branches: vec![
            RendererBranchAssertion {
                branch_label: "score".to_string(),
                expected_html_pattern: ">84<".to_string(),
            },
            RendererBranchAssertion {
                branch_label: "band".to_string(),
                expected_html_pattern: r#"data-band="green""#.to_string(),
            },
            RendererBranchAssertion {
                branch_label: "trend".to_string(),
                expected_html_pattern: r#"data-trend="improving""#.to_string(),
            },
            RendererBranchAssertion {
                branch_label: "design-system-metadata".to_string(),
                expected_html_pattern: r#"data-ds-name="HealthBadge""#.to_string(),
            },
        ],
        expected_wrapper: BlockWrapperAssertion {
            tag: "span".to_string(),
            class: "dailyos-health-badge".to_string(),
            data_attrs: vec![
                ("data-ds-tier".to_string(), "primitive".to_string()),
                ("data-ds-name".to_string(), "HealthBadge".to_string()),
            ],
        },
    }
}

crate::integration_test_block!(health_badge_block_integration, health_badge_fixture);
