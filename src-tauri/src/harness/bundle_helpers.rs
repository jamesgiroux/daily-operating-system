#![allow(dead_code)]

use std::{future::Future, path::PathBuf, pin::Pin, sync::Arc};

use crate::abilities::registry::{AbilityPolicy, McpExposure, SignalPolicy};
use crate::abilities::{
    AbilityCategory, AbilityContext, AbilityDescriptor, AbilityError, AbilityRegistry, ActorKind,
};
use crate::db::ActionDb;
use crate::services::context::ExecutionMode;
use serde_json::{json, Value};

use super::{load_fixture, run_fixture, EvalFixture, RunError, RunResult, RunnerDeps};

type ErasedFuture<'a> =
    Pin<Box<dyn Future<Output = Result<serde_json::Value, AbilityError>> + Send + 'a>>;

const SYSTEM_ACTORS: &[ActorKind] = &[ActorKind::System];
const EVALUATE_MODES: &[ExecutionMode] = &[ExecutionMode::Evaluate];

pub fn bundle_fixture_path(bundle: u32) -> PathBuf {
    let repo_relative = PathBuf::from(format!("src-tauri/tests/fixtures/bundle-{bundle}"));
    if repo_relative.is_dir() {
        return repo_relative;
    }

    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(format!("bundle-{bundle}"))
}

pub fn load_bundle_fixture(bundle: u32) -> EvalFixture {
    load_fixture(&bundle_fixture_path(bundle)).expect("bundle fixture loads")
}

pub fn synthetic_runner_deps() -> RunnerDeps {
    RunnerDeps {
        registry: Arc::new(
            AbilityRegistry::from_descriptors_checked(vec![
                enrich_account_intelligence_descriptor(),
            ])
            .expect("synthetic registry builds"),
        ),
    }
}

pub fn run_with_synthetic_enrich_stub(fixture: &EvalFixture) -> Result<RunResult, RunError> {
    let deps = synthetic_runner_deps();
    run_fixture(&deps, fixture)
}

pub fn refresh_prepare_meeting_context_from_db(
    prepared: &mut super::runner::PreparedFixtureRun,
    meeting_id: &str,
) -> Result<(), RunError> {
    let db = ActionDb::from_conn(&prepared.conn);
    prepared.prepare_meeting_context = Some(
        crate::services::meetings::load_prepare_meeting_context_snapshot(db, meeting_id)
            .map_err(RunError::StateSqlFailed)?,
    );
    Ok(())
}

pub fn assert_eval_bridge_stub_invoked(result: &RunResult) {
    assert_eq!(
        result.actual_output["stub_kind"], "enrich_account_intelligence_test_stub",
        "fixture should invoke the registered synthetic enrich_account_intelligence stub"
    );
    assert_eq!(result.actual_output["entity_id"], "acct-test-1");
    assert_eq!(result.actual_provenance["surface"], "eval");
}

pub fn assert_fixture_metadata(
    fixture: &EvalFixture,
    bundle: u32,
    dominant_factors: &[&str],
    pass_fail_snippet: &str,
) {
    assert_eq!(fixture.metadata.bundle, Some(bundle));
    assert_eq!(
        fixture.metadata.trust_factors_dominant,
        dominant_factors
            .iter()
            .map(|factor| (*factor).to_string())
            .collect::<Vec<_>>()
    );
    assert!(
        fixture
            .metadata
            .pass_fail_definition
            .contains(pass_fail_snippet),
        "pass/fail definition should mention `{pass_fail_snippet}`"
    );
    assert!(
        fixture.metadata.fixture_design_notes.is_some(),
        "bundle metadata should carry fixture design notes"
    );
}

pub fn expected_post_action_state(fixture: &EvalFixture) -> &Value {
    fixture
        .expected
        .state
        .as_ref()
        .and_then(|state| state.get("post_action_state"))
        .expect("expected_state.json post_action_state is present")
}

pub fn actual_post_action_state(result: &RunResult) -> &Value {
    result
        .actual_state
        .as_ref()
        .and_then(|state| state.get("post_action_state"))
        .expect("run_fixture captured post_action_state")
}

pub fn claims(state: &Value) -> &Vec<Value> {
    state["intelligence_claims"]
        .as_array()
        .expect("intelligence_claims array")
}

pub fn claim_by_id<'a>(state: &'a Value, claim_id: &str) -> &'a Value {
    claims(state)
        .iter()
        .find(|claim| claim["claim_id"] == claim_id)
        .unwrap_or_else(|| panic!("missing claim `{claim_id}`"))
}

pub fn optional_claim_by_id<'a>(state: &'a Value, claim_id: &str) -> Option<&'a Value> {
    claims(state)
        .iter()
        .find(|claim| claim["claim_id"] == claim_id)
}

pub fn trust_score(claim: &Value) -> f64 {
    claim["trust_score"]
        .as_f64()
        .unwrap_or_else(|| panic!("claim {} has numeric trust_score", claim["claim_id"]))
}

pub fn assert_trust_score_below(claim: &Value, boundary: f64) {
    let score = trust_score(claim);
    assert!(
        score < boundary,
        "claim {} expected trust_score < {boundary}, got {score}",
        claim["claim_id"]
    );
}

pub fn assert_trust_score_at_least(claim: &Value, boundary: f64) {
    let score = trust_score(claim);
    assert!(
        score >= boundary,
        "claim {} expected trust_score >= {boundary}, got {score}",
        claim["claim_id"]
    );
}

pub fn assert_string_array_contains_all(value: &Value, expected: &[&str]) {
    let actual = value.as_array().expect("string array");
    for needle in expected {
        assert!(
            actual.iter().any(|item| item.as_str() == Some(*needle)),
            "expected array {actual:?} to contain `{needle}`"
        );
    }
}

pub fn assert_state_string_array_contains(state: &Value, field: &str, needle: &str) {
    let actual = state[field].as_array().expect("state string array");
    assert!(
        actual
            .iter()
            .any(|item| item.as_str().is_some_and(|text| text.contains(needle))),
        "expected state.{field} to contain `{needle}`"
    );
}

pub fn confidence_evidence_for<'a>(
    state: &'a Value,
    claim_id: &str,
    factor_name: &str,
) -> &'a Value {
    state["confidence_evidence"]
        .as_array()
        .expect("confidence_evidence array")
        .iter()
        .find(|evidence| {
            evidence["claim_id"] == claim_id
                && (evidence["factor"]["name"].as_str() == Some(factor_name)
                    || evidence["factor"].as_str() == Some(factor_name))
        })
        .unwrap_or_else(|| panic!("missing ConfidenceEvidence `{factor_name}` for `{claim_id}`"))
}

pub fn warning_present(provenance: &Value, warning_name: &str) -> bool {
    provenance["provenance"]["warnings"]
        .as_array()
        .is_some_and(|warnings| {
            warnings
                .iter()
                .any(|warning| warning.get(warning_name).is_some())
        })
}

pub fn assert_warning_present(fixture: &EvalFixture, warning_name: &str) {
    assert!(
        warning_present(&fixture.expected.provenance, warning_name),
        "expected provenance warning `{warning_name}`"
    );
}

fn enrich_account_intelligence_descriptor() -> AbilityDescriptor {
    AbilityDescriptor {
        name: "enrich_account_intelligence",
        version: "0.0.1-test",
        schema_version: 1,
        category: AbilityCategory::Read,
        policy: AbilityPolicy {
            allowed_actors: SYSTEM_ACTORS,
            allowed_modes: EVALUATE_MODES,
            requires_confirmation: false,
            may_publish: false,
            required_scopes: &[],
            mcp_exposure: McpExposure::None,
            client_side_executable: false,
        },
        composes: &[],
        mutates: &[],
        experimental: false,
        registered_at: None,
        signal_policy: SignalPolicy::default(),
        invoke_erased: enrich_account_intelligence_test_stub,
        input_schema: enrich_input_schema,
        output_schema: closed_object_schema,
    }
}

fn enrich_account_intelligence_test_stub<'a>(
    ctx: &'a AbilityContext<'a>,
    input: serde_json::Value,
) -> ErasedFuture<'a> {
    Box::pin(async move {
        Ok(json!({
            "data": {
                "stub_kind": "enrich_account_intelligence_test_stub",
                "entity_type": input["entity_type"],
                "entity_id": input["entity_id"],
                "schema_version": input["schema_version"],
                "mode": ctx.mode().as_str(),
                "actor": format!("{:?}", ctx.actor)
            },
            "ability_version": { "major": 0, "minor": 1 },
            "diagnostics": { "warnings": [] },
            "provenance": {
                "invocation_id": "cccccccc-cccc-4ccc-8ccc-cccccccccccc",
                "ability_name": "enrich_account_intelligence",
                "ability_version": { "major": 0, "minor": 1 },
                "ability_schema_version": 1,
                "actor": format!("{:?}", ctx.actor),
                "mode": ctx.mode().as_str(),
                "warnings": []
            }
        }))
    })
}

fn enrich_input_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["entity_type", "entity_id", "schema_version"],
        "properties": {
            "entity_type": { "type": "string" },
            "entity_id": { "type": "string" },
            "schema_version": { "type": "integer" }
        }
    })
}

fn closed_object_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "additionalProperties": false
    })
}
