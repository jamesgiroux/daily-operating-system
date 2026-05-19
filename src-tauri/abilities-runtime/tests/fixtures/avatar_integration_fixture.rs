use abilities_runtime::abilities::composition::{BindingRole, BlockType};
use serde_json::{json, to_value};

use crate::{
    fixture_binding, fixture_block, fixture_claim, fixture_composition, BindingExpectation,
    BlockIntegrationFixture, BlockWrapperAssertion, ProjectionDiagnostic, RendererBranchAssertion,
    ValueKind,
};

pub fn avatar_fixture() -> BlockIntegrationFixture {
    let ability_name = "dailyos/avatar";
    let composition_id = "dailyos/avatar:person:person-avatar-001";
    let claims = vec![fixture_claim("claim-avatar-source", "/person/avatar")];
    let block = fixture_block(
        "block-avatar",
        BlockType::Avatar,
        json!({
            "name": "Example Person",
            "personId": "person-avatar-001",
            "photoUrl": "https://example.test/avatar.png",
            "size": 40
        }),
        claims,
        vec![fixture_binding("/photoUrl", BindingRole::Source, &[0])],
    );
    let composition = fixture_composition(ability_name, composition_id, 2, vec![block]);

    BlockIntegrationFixture {
        ability_name: ability_name.to_string(),
        composition_id: composition_id.to_string(),
        input_json: to_value(composition).expect("avatar fixture serializes"),
        expected_bindings: vec![
            BindingExpectation {
                pointer: "/blocks/0/payload/name".to_string(),
                value_kind: ValueKind::String,
                required: true,
            },
            BindingExpectation {
                pointer: "/blocks/0/payload/personId".to_string(),
                value_kind: ValueKind::String,
                required: true,
            },
            BindingExpectation {
                pointer: "/blocks/0/payload/photoUrl".to_string(),
                value_kind: ValueKind::String,
                required: true,
            },
            BindingExpectation {
                pointer: "/blocks/0/payload/size".to_string(),
                value_kind: ValueKind::Number,
                required: true,
            },
        ],
        expected_diagnostics: Vec::<ProjectionDiagnostic>::new(),
        expected_renderer_branches: vec![
            RendererBranchAssertion {
                branch_label: "runtime-resolved-css-url".to_string(),
                expected_html_pattern: "--dailyos-avatar-bg-url:".to_string(),
            },
            RendererBranchAssertion {
                branch_label: "source-url".to_string(),
                expected_html_pattern: "https://example.test/avatar.png".to_string(),
            },
            RendererBranchAssertion {
                branch_label: "design-system-metadata".to_string(),
                expected_html_pattern: r#"data-ds-name="Avatar""#.to_string(),
            },
        ],
        expected_wrapper: BlockWrapperAssertion {
            tag: "span".to_string(),
            class: "dailyos-avatar dailyos-avatarImage".to_string(),
            data_attrs: vec![
                ("data-ds-tier".to_string(), "primitive".to_string()),
                ("data-ds-name".to_string(), "Avatar".to_string()),
            ],
        },
    }
}

crate::integration_test_block!(avatar_block_integration, avatar_fixture);
