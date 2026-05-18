use abilities_runtime::abilities::composition::{BindingRole, BlockType};
use serde_json::{json, to_value};

use crate::{
    fixture_binding, fixture_block, fixture_claim, fixture_composition, BindingExpectation,
    BlockIntegrationFixture, BlockWrapperAssertion, ProjectionDiagnostic, RendererBranchAssertion,
    ValueKind,
};

pub fn account_overview_fixture() -> BlockIntegrationFixture {
    let ability_name = "dailyos/account-overview";
    let composition_id = "dailyos/account-overview:account:acct-test-001";
    let claims = vec![
        fixture_claim("claim-account-summary", "/summary"),
        fixture_claim("claim-account-context", "/context/0/text"),
    ];
    let block = fixture_block(
        "block-account-overview",
        BlockType::AccountOverview,
        json!({
            "account": {
                "display_name": "Example Account"
            },
            "summary": "Example Account has a stable expansion path.",
            "health": {
                "band": "steady",
                "score": 82
            },
            "risk": {
                "title": "Renewal risk",
                "body": "No unresolved executive risk is visible."
            },
            "actions": [
                {
                    "title": "Confirm rollout owner"
                }
            ],
            "relationships": [
                {
                    "label": "Executive sponsor"
                }
            ],
            "title": "Account overview",
            "claim_count": 2,
            "counts_by_trust_band": {},
            "context": [],
            "account_id": "acct-test-001"
        }),
        claims,
        vec![
            fixture_binding("/summary", BindingRole::Source, &[0]),
            fixture_binding("/context", BindingRole::ComputedFrom, &[1]),
        ],
    );
    let composition = fixture_composition(ability_name, composition_id, 11, vec![block]);

    BlockIntegrationFixture {
        ability_name: ability_name.to_string(),
        composition_id: composition_id.to_string(),
        input_json: to_value(composition).expect("account overview fixture serializes"),
        expected_bindings: vec![
            BindingExpectation {
                pointer: "/blocks/0/payload/title".to_string(),
                value_kind: ValueKind::String,
                required: true,
            },
            BindingExpectation {
                pointer: "/blocks/0/payload/context".to_string(),
                value_kind: ValueKind::Array,
                required: true,
            },
            BindingExpectation {
                pointer: "/blocks/0/payload/health/score".to_string(),
                value_kind: ValueKind::Number,
                required: true,
            },
            BindingExpectation {
                pointer: "/blocks/0/payload/counts_by_trust_band".to_string(),
                value_kind: ValueKind::Object,
                required: true,
            },
        ],
        expected_diagnostics: Vec::<ProjectionDiagnostic>::new(),
        expected_renderer_branches: vec![
            RendererBranchAssertion {
                branch_label: "title-label".to_string(),
                expected_html_pattern: "Account overview".to_string(),
            },
            RendererBranchAssertion {
                branch_label: "trust-band-badge".to_string(),
                expected_html_pattern: "Use with caution".to_string(),
            },
            RendererBranchAssertion {
                branch_label: "finite-ending".to_string(),
                expected_html_pattern: "dailyos-finis-marker".to_string(),
            },
        ],
        expected_wrapper: BlockWrapperAssertion {
            tag: "section".to_string(),
            class: "wp-block-dailyos-account-overview".to_string(),
            data_attrs: vec![
                ("data-ds-tier".to_string(), "pattern".to_string()),
                ("data-ds-name".to_string(), "AccountOverview".to_string()),
            ],
        },
    }
}

crate::integration_test_block!(account_overview_block_integration, account_overview_fixture);
