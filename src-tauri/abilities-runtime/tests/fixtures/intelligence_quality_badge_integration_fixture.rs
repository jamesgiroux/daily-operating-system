use abilities_runtime::abilities::composition::{BindingRole, BlockType};
use serde_json::{json, to_value};

use crate::{
    fixture_binding, fixture_block, fixture_claim, fixture_composition, BindingExpectation,
    BlockIntegrationFixture, BlockWrapperAssertion, ProjectionDiagnostic, RendererBranchAssertion,
    ValueKind,
};

pub fn intelligence_quality_badge_fixture() -> BlockIntegrationFixture {
    let ability_name = "dailyos/intelligence-quality-badge";
    let composition_id = "dailyos/intelligence-quality-badge:claim:claim-quality-001";
    let claims = vec![fixture_claim("claim-quality-score", "/quality/score")];
    let block = fixture_block(
        "block-intelligence-quality-badge",
        BlockType::IntelligenceQualityBadge,
        json!({
            "qualityScore": 0.92,
            "hasNewSignals": true,
            "lastEnriched": "2099-01-01T00:00:00Z",
            "showLabel": true,
            "showTooltip": true
        }),
        claims,
        vec![fixture_binding("/qualityScore", BindingRole::Source, &[0])],
    );
    let composition = fixture_composition(ability_name, composition_id, 5, vec![block]);

    BlockIntegrationFixture {
        ability_name: ability_name.to_string(),
        composition_id: composition_id.to_string(),
        input_json: to_value(composition).expect("intelligence quality badge fixture serializes"),
        expected_bindings: vec![
            BindingExpectation {
                pointer: "/blocks/0/payload/qualityScore".to_string(),
                value_kind: ValueKind::Number,
                required: true,
            },
            BindingExpectation {
                pointer: "/blocks/0/payload/hasNewSignals".to_string(),
                value_kind: ValueKind::Bool,
                required: true,
            },
            BindingExpectation {
                pointer: "/blocks/0/payload/lastEnriched".to_string(),
                value_kind: ValueKind::String,
                required: true,
            },
            BindingExpectation {
                pointer: "/blocks/0/payload/showLabel".to_string(),
                value_kind: ValueKind::Bool,
                required: true,
            },
        ],
        expected_diagnostics: Vec::<ProjectionDiagnostic>::new(),
        expected_renderer_branches: vec![
            RendererBranchAssertion {
                branch_label: "score-derived-level".to_string(),
                expected_html_pattern: r#"data-quality-level="fresh""#.to_string(),
            },
            RendererBranchAssertion {
                branch_label: "score-derived-label".to_string(),
                expected_html_pattern: ">Fresh<".to_string(),
            },
            RendererBranchAssertion {
                branch_label: "new-signal".to_string(),
                expected_html_pattern: "dailyos-intelligence-quality-badge__newSignalDot"
                    .to_string(),
            },
            RendererBranchAssertion {
                branch_label: "display-safe-tooltip".to_string(),
                expected_html_pattern: "Last updated: Jan 1, 2099".to_string(),
            },
        ],
        expected_wrapper: BlockWrapperAssertion {
            tag: "span".to_string(),
            class: "dailyos-intelligence-quality-badge".to_string(),
            data_attrs: vec![
                ("data-ds-tier".to_string(), "primitive".to_string()),
                (
                    "data-ds-name".to_string(),
                    "IntelligenceQualityBadge".to_string(),
                ),
            ],
        },
    }
}

crate::integration_test_block!(
    intelligence_quality_badge_block_integration,
    intelligence_quality_badge_fixture
);
