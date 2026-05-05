#[path = "harness/mod.rs"]
mod harness;

use std::{
    collections::BTreeSet,
    future::Future,
    fs,
    pin::Pin,
    path::{Path, PathBuf},
    sync::Arc,
};

use base64::Engine;
use dailyos_lib::abilities::registry::{AbilityPolicy, SignalPolicy};
use dailyos_lib::abilities::{
    AbilityCategory, AbilityContext, AbilityDescriptor, AbilityError, AbilityRegistry, Actor,
};
use dailyos_lib::services::context::{
    ExecutionMode, GleanAccountFacts, GleanClientHandle, SeedableRng, SeededRng,
};
use harness::{
    canonical_json_eq, diff_internal_provenance, diff_rendered_provenance, discover_fixtures,
    load_fixture, prepare_fixture_for_run, run_fixture, CategoryScorer, FixtureLoadError,
    FixtureRef, MaintenanceScorer, PublishScorer, ReadScorer, RunError, RunnerDeps,
    TransformScorer,
};
use serde_json::json;

#[test]
fn loader_loads_all_committed_bundles() {
    let root = fixture_root();
    let discovered: Vec<FixtureRef> =
        discover_fixtures(&[root.as_path()]).expect("fixture discovery succeeds");

    let expected = BTreeSet::from([2_u32, 3, 4, 6, 7, 8]);
    let mut seen = BTreeSet::new();

    for fixture_ref in discovered {
        let fixture = load_fixture(&fixture_ref.fixture_dir).expect("fixture loads");
        let bundle = fixture.metadata.bundle.expect("bundle metadata is set");

        assert!(
            !fixture.metadata.scenario_id.trim().is_empty(),
            "{} has a populated scenario_id",
            fixture_ref.fixture_dir.display()
        );

        seen.insert(bundle);
    }

    assert_eq!(seen, expected);
}

#[test]
fn loader_returns_typed_error_for_missing_required_file() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    write_minimal_fixture(temp_dir.path(), true);
    fs::remove_file(temp_dir.path().join("state.sql")).expect("remove state.sql");

    let error = load_fixture(temp_dir.path()).expect_err("missing state.sql fails");

    match error {
        FixtureLoadError::MissingRequiredFile { path } => {
            assert_eq!(path, temp_dir.path().join("state.sql"));
        }
        other => panic!("expected MissingRequiredFile, got {other:?}"),
    }
}

#[test]
fn loader_returns_typed_error_for_malformed_json() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    write_minimal_fixture(temp_dir.path(), true);
    fs::write(temp_dir.path().join("inputs.json"), "{").expect("write malformed JSON");

    let error = load_fixture(temp_dir.path()).expect_err("malformed inputs.json fails");

    match error {
        FixtureLoadError::ParseJson { path, .. } => {
            assert_eq!(path, temp_dir.path().join("inputs.json"));
        }
        other => panic!("expected ParseJson, got {other:?}"),
    }
}

#[test]
fn loader_handles_optional_expected_state_json_when_absent() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    write_minimal_fixture(temp_dir.path(), false);

    let fixture = load_fixture(temp_dir.path()).expect("fixture loads without expected_state.json");

    assert!(fixture.expected.state.is_none());
}

#[test]
fn runner_applies_state_sql_to_in_memory_db() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    write_minimal_fixture(temp_dir.path(), true);
    fs::write(
        temp_dir.path().join("state.sql"),
        "CREATE TABLE runner_smoke (value TEXT NOT NULL);\n\
         INSERT INTO runner_smoke (value) VALUES ('applied');\n",
    )
    .expect("write state.sql");
    let fixture = load_fixture(temp_dir.path()).expect("fixture loads");

    let prepared = prepare_fixture_for_run(&fixture).expect("fixture prepares");
    let count: i64 = prepared
        .conn
        .query_row(
            "SELECT COUNT(*) FROM runner_smoke WHERE value = 'applied'",
            [],
            |row| row.get(0),
        )
        .expect("query runner_smoke");

    assert_eq!(count, 1);
}

#[test]
fn runner_returns_state_sql_failed_on_malformed_sql() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    write_minimal_fixture(temp_dir.path(), true);
    fs::write(temp_dir.path().join("state.sql"), "CREATE TABLE broken (;")
        .expect("write malformed state.sql");
    let fixture = load_fixture(temp_dir.path()).expect("fixture loads");

    let Err(error) = prepare_fixture_for_run(&fixture) else {
        panic!("malformed state.sql should fail");
    };

    match error {
        RunError::StateSqlFailed(message) => {
            assert!(
                message.contains("near") || message.contains("syntax"),
                "unexpected SQLite error: {message}"
            );
        }
        other => panic!("expected StateSqlFailed, got {other:?}"),
    }
}

#[test]
fn runner_constructs_evaluate_context_with_fixture_clock_and_seed() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    write_minimal_fixture(temp_dir.path(), true);
    fs::write(temp_dir.path().join("seed.txt"), "217\n").expect("write seed");
    let fixture = load_fixture(temp_dir.path()).expect("fixture loads");
    let prepared = prepare_fixture_for_run(&fixture).expect("fixture prepares");

    let ctx = prepared.service_context();
    let expected_rng = SeedableRng::new(fixture.seed);

    assert_eq!(ctx.mode, ExecutionMode::Evaluate);
    assert_eq!(ctx.clock.now(), fixture.clock);
    assert_eq!(ctx.rng.random_u64(), expected_rng.random_u64());
}

#[test]
fn runner_loads_external_replay_fixture_into_external_clients() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    write_minimal_fixture(temp_dir.path(), true);
    let account_id = "acct-runner-example";
    let key = GleanClientHandle::request_key_for_fetch_account_facts(
        account_id,
        "harness-default-tenant",
    );
    let body = br#"{"account_id":"acct-runner-example","facts":["runner replay fact"]}"#;
    let body_base64 = base64::engine::general_purpose::STANDARD.encode(body);
    write_json(
        &temp_dir.path().join("external_replay.json"),
        json!({
            "version": 1,
            "fixtures": [{
                "request_key_hex": key.to_hex(),
                "response": {
                    "status": 200,
                    "headers": [["Content-Type", "application/json"]],
                    "body_base64": body_base64
                }
            }]
        }),
    );
    let fixture = load_fixture(temp_dir.path()).expect("fixture loads");

    let prepared = prepare_fixture_for_run(&fixture).expect("fixture prepares");
    let response = prepared
        .external_clients
        .glean
        .fetch_account_facts(account_id)
        .expect("external replay hit");

    assert_eq!(
        response,
        GleanAccountFacts {
            account_id: account_id.to_string(),
            facts: vec!["runner replay fact".to_string()],
        }
    );
}

#[test]
fn runner_invokes_bundle_fixture_through_eval_bridge_and_captures_output() {
    let fixture = bundle_fixture(2);
    let deps = synthetic_runner_deps();

    let result = run_fixture(&deps, &fixture).expect("fixture invokes through eval bridge");

    assert!(result
        .actual_output
        .as_object()
        .is_some_and(|object| !object.is_empty()));
    assert_eq!(
        result.actual_output["stub_kind"],
        "enrich_account_intelligence_test_stub"
    );
    assert_eq!(result.actual_output["entity_id"], "acct-test-1");
    assert_eq!(result.actual_provenance["surface"], "eval");
}

#[test]
fn runner_captures_post_action_state_with_intelligence_claims_rows() {
    let fixture = bundle_fixture(2);
    let deps = synthetic_runner_deps();

    let result = run_fixture(&deps, &fixture).expect("fixture invokes");
    let state = result.actual_state.expect("post-action state captured");
    let claims = state["post_action_state"]["intelligence_claims"]
        .as_array()
        .expect("intelligence_claims array captured");
    let ground_truth = claims
        .iter()
        .find(|claim| claim["claim_id"] == "claim-test-ground-truth-eu-expansion")
        .expect("seeded claim captured");

    assert_eq!(ground_truth["subject_ref"]["kind"], "account");
    assert_eq!(ground_truth["trust_score"], json!(0.92));
    assert_eq!(ground_truth["trust_version"], json!(1));
    assert_eq!(ground_truth["trust_band"], "likely_current");
    assert!(state["post_action_state"]["preserved_claims"]
        .as_array()
        .expect("preserved_claims array captured")
        .iter()
        .any(|claim| claim
            .as_str()
            .is_some_and(|text| text.contains("claim-test-ground-truth-eu-expansion"))));
}

#[test]
fn runner_propagates_invocation_failed_on_unknown_ability_name() {
    let fixture = fixture_with_ability_name(bundle_fixture(2), "unknown_ability");
    let deps = synthetic_runner_deps();
    let error = match run_fixture(&deps, &fixture) {
        Ok(_) => panic!("unknown ability should fail"),
        Err(error) => error,
    };

    match error {
        RunError::InvocationFailed(message) => {
            assert_eq!(message, "AbilityUnavailable");
        }
        other => panic!("expected InvocationFailed, got {other:?}"),
    }
}

#[test]
fn runner_propagates_byte_equal_unavailable_for_unauthorized_ability() {
    let unknown_fixture = fixture_with_ability_name(bundle_fixture(2), "unknown_ability");
    let authorized_deps = synthetic_runner_deps();
    let unauthorized_deps = runner_deps_with(vec![unauthorized_enrich_descriptor()]);

    let unknown_error = invocation_failed_message(run_fixture(&authorized_deps, &unknown_fixture));
    let unauthorized_error =
        invocation_failed_message(run_fixture(&unauthorized_deps, &bundle_fixture(2)));

    assert_eq!(unauthorized_error, unknown_error);
    assert_eq!(unauthorized_error, "AbilityUnavailable");
}

#[test]
fn read_scorer_passes_on_exact_canonical_output_match() {
    let expected = expected_artifacts(
        json!({"answer": {"count": 1, "label": "ready"}}),
        json!({"sources": [{"title": "source-a"}], "warnings": ["b", "a"]}),
        None,
        "show-public-only",
    );
    let actual = run_result(
        json!({"answer": {"label": "ready", "count": 1}}),
        json!({"warnings": ["a", "b"], "sources": [{"title": "source-a"}]}),
        None,
    );

    let score = ReadScorer.score(&expected, &actual);

    assert!(score.passed, "{:?}", score.diffs);
    assert!(score.diffs.is_empty());
    assert_eq!(score.category, harness::AbilityCategory::Read);
    assert_eq!(score.continuous_score, None);
}

#[test]
fn read_scorer_fails_on_output_mismatch_with_path_diff() {
    let expected = expected_artifacts(
        json!({"items": [{"name": "first"}, {"name": "expected"}]}),
        json!({"sources": []}),
        None,
        "show-public-only",
    );
    let actual = run_result(
        json!({"items": [{"name": "first"}, {"name": "actual"}]}),
        json!({"sources": []}),
        None,
    );

    let score = ReadScorer.score(&expected, &actual);

    assert!(!score.passed);
    assert!(score.diffs.iter().any(|diff| {
        diff.kind == harness::DiffKind::OutputMismatch
            && diff.path == "/items/1/name"
            && diff.expected == json!("expected")
            && diff.actual == json!("actual")
    }));
}

#[test]
fn read_scorer_fails_on_provenance_mismatch() {
    let expected = expected_artifacts(
        json!({"ok": true}),
        json!({"sources": [{"title": "source-a", "source_asof": "2026-01-01"}]}),
        None,
        "show-public-only",
    );
    let actual = run_result(
        json!({"ok": true}),
        json!({"sources": [{"title": "source-a", "source_asof": "2026-01-02"}]}),
        None,
    );

    let score = ReadScorer.score(&expected, &actual);

    assert!(!score.passed);
    assert!(score.diffs.iter().any(|diff| {
        diff.kind == harness::DiffKind::ProvenanceMismatch
            && diff.path == "/sources/0/source_asof"
    }));
}

#[test]
fn read_scorer_fails_on_state_mismatch_when_expected_state_present() {
    let expected = expected_artifacts(
        json!({"ok": true}),
        json!({"sources": []}),
        Some(json!({"post_action_state": {"trust": [{"score": 0.9}]}})),
        "show-public-only",
    );
    let actual = run_result(
        json!({"ok": true}),
        json!({"sources": []}),
        Some(json!({"post_action_state": {"trust": [{"score": 0.1}]}})),
    );

    let score = ReadScorer.score(&expected, &actual);

    assert!(!score.passed);
    assert!(score.diffs.iter().any(|diff| {
        diff.kind == harness::DiffKind::StateMismatch
            && diff.path == "/post_action_state/trust/0/score"
    }));
}

#[test]
fn transform_scorer_returns_continuous_score_one_on_match() {
    let expected = expected_artifacts(
        json!({"summary": "stable"}),
        json!({"sources": [{"title": "source-a"}]}),
        None,
        "show-public-only",
    );
    let actual = run_result(
        json!({"summary": "stable"}),
        json!({"sources": [{"title": "source-a"}]}),
        None,
    );

    let score = TransformScorer { threshold: 0.8 }.score(&expected, &actual);

    assert!(score.passed, "{:?}", score.diffs);
    assert_eq!(score.category, harness::AbilityCategory::Transform);
    assert_eq!(score.continuous_score, Some(1.0));
}

#[test]
fn maintenance_scorer_compares_planned_mutations_field() {
    let expected = expected_artifacts(
        json!({
            "planned_mutations": [{"table": "records", "value": "expected"}],
            "ignored_rendered_output": "expected"
        }),
        json!({"sources": []}),
        None,
        "show-public-only",
    );
    let actual = run_result(
        json!({
            "planned_mutations": [{"table": "records", "value": "actual"}],
            "ignored_rendered_output": "actual"
        }),
        json!({"sources": []}),
        None,
    );

    let score = MaintenanceScorer.score(&expected, &actual);

    assert!(!score.passed);
    assert_eq!(score.category, harness::AbilityCategory::Maintenance);
    assert_eq!(score.diffs.len(), 1);
    assert_eq!(score.diffs[0].kind, harness::DiffKind::OutputMismatch);
    assert_eq!(score.diffs[0].path, "/planned_mutations/0/value");
}

#[test]
fn publish_scorer_compares_outbox_field_only() {
    let expected = expected_artifacts(
        json!({
            "outbox": [{"channel": "email", "to": "person@example.invalid"}],
            "external_side_effect": "not-sent"
        }),
        json!({"sources": []}),
        None,
        "show-public-only",
    );
    let actual_with_same_outbox = run_result(
        json!({
            "outbox": [{"channel": "email", "to": "person@example.invalid"}],
            "external_side_effect": "sent"
        }),
        json!({"sources": []}),
        None,
    );
    let actual_with_changed_outbox = run_result(
        json!({
            "outbox": [{"channel": "email", "to": "other@example.invalid"}],
            "external_side_effect": "not-sent"
        }),
        json!({"sources": []}),
        None,
    );

    let passing_score = PublishScorer.score(&expected, &actual_with_same_outbox);
    let failing_score = PublishScorer.score(&expected, &actual_with_changed_outbox);

    assert!(passing_score.passed, "{:?}", passing_score.diffs);
    assert!(!failing_score.passed);
    assert_eq!(failing_score.category, harness::AbilityCategory::Publish);
    assert_eq!(failing_score.diffs.len(), 1);
    assert_eq!(failing_score.diffs[0].path, "/outbox/0/to");
}

#[test]
fn diff_internal_provenance_returns_path_for_mismatched_field() {
    let diffs = diff_internal_provenance(
        &json!({"invocation_id": "expected", "sources": []}),
        &json!({"invocation_id": "actual", "sources": []}),
    );

    assert_eq!(diffs.len(), 1);
    assert_eq!(diffs[0].kind, harness::DiffKind::ProvenanceMismatch);
    assert_eq!(diffs[0].path, "/invocation_id");
}

#[test]
fn diff_rendered_provenance_strips_internal_ids_before_comparison() {
    let diffs = diff_rendered_provenance(
        &json!({
            "invocation_id": "expected",
            "prompt_hash": "expected-hash",
            "seed": 1,
            "summary": "visible",
            "children": [{"invocation_id": "child-expected", "summary": "deep"}],
            "sources": [{"source_id": "source-expected", "title": "source-a"}]
        }),
        &json!({
            "invocation_id": "actual",
            "prompt_hash": "actual-hash",
            "seed": 2,
            "summary": "visible",
            "children": [{"invocation_id": "child-actual", "summary": "changed"}],
            "sources": [{"source_id": "source-actual", "title": "source-a"}]
        }),
    );

    assert!(diffs.is_empty(), "{diffs:?}");
}

#[test]
fn canonical_json_eq_handles_object_key_order() {
    assert!(canonical_json_eq(
        &json!({"b": 2, "a": {"d": 4, "c": 3}}),
        &json!({"a": {"c": 3, "d": 4}, "b": 2})
    ));
}

#[test]
fn canonical_json_eq_handles_float_tolerance_for_close_values() {
    assert!(canonical_json_eq(
        &json!({"score": 1.0}),
        &json!({"score": 1.0 + (f64::EPSILON * 128.0)})
    ));
}

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn expected_artifacts(
    output: serde_json::Value,
    provenance: serde_json::Value,
    state: Option<serde_json::Value>,
    expected_render_policy: &str,
) -> harness::ExpectedArtifacts {
    harness::ExpectedArtifacts {
        output,
        provenance,
        state,
        expected_render_policy: expected_render_policy.to_string(),
    }
}

fn run_result(
    actual_output: serde_json::Value,
    actual_provenance: serde_json::Value,
    actual_state: Option<serde_json::Value>,
) -> harness::RunResult {
    harness::RunResult {
        actual_output,
        actual_provenance,
        actual_state,
        diagnostics: Vec::new(),
    }
}

fn bundle_fixture(bundle: u32) -> harness::EvalFixture {
    load_fixture(&fixture_root().join(format!("bundle-{bundle}"))).expect("bundle fixture loads")
}

fn fixture_with_ability_name(
    mut fixture: harness::EvalFixture,
    ability_name: &str,
) -> harness::EvalFixture {
    fixture.inputs_json["ability_name"] = json!(ability_name);
    fixture
}

fn synthetic_runner_deps() -> RunnerDeps {
    runner_deps_with(vec![enrich_account_intelligence_descriptor()])
}

fn runner_deps_with(descriptors: Vec<AbilityDescriptor>) -> RunnerDeps {
    RunnerDeps {
        registry: Arc::new(AbilityRegistry::from_descriptors_checked(descriptors).unwrap()),
    }
}

fn invocation_failed_message(result: Result<harness::RunResult, RunError>) -> String {
    match result {
        Ok(_) => panic!("fixture should fail"),
        Err(RunError::InvocationFailed(message)) => message,
        Err(other) => panic!("expected InvocationFailed, got {other:?}"),
    }
}

type ErasedFuture<'a> =
    Pin<Box<dyn Future<Output = Result<serde_json::Value, AbilityError>> + Send + 'a>>;

const SYSTEM_ACTORS: &[Actor] = &[Actor::System];
const USER_ACTORS: &[Actor] = &[Actor::User];
const EVALUATE_MODES: &[ExecutionMode] = &[ExecutionMode::Evaluate];

fn enrich_account_intelligence_descriptor() -> AbilityDescriptor {
    enrich_descriptor_with_policy(SYSTEM_ACTORS)
}

fn unauthorized_enrich_descriptor() -> AbilityDescriptor {
    enrich_descriptor_with_policy(USER_ACTORS)
}

fn enrich_descriptor_with_policy(allowed_actors: &'static [Actor]) -> AbilityDescriptor {
    AbilityDescriptor {
        name: "enrich_account_intelligence",
        version: "0.0.1-test",
        schema_version: 1,
        category: AbilityCategory::Read,
        policy: AbilityPolicy {
            allowed_actors,
            allowed_modes: EVALUATE_MODES,
            requires_confirmation: false,
            may_publish: false,
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

fn write_minimal_fixture(fixture_dir: &Path, include_expected_state: bool) {
    fs::write(fixture_dir.join("clock.txt"), "2026-05-01T12:00:00Z\n").expect("write clock");
    fs::write(fixture_dir.join("seed.txt"), "42\n").expect("write seed");
    fs::write(fixture_dir.join("state.sql"), "-- fixture SQL\n").expect("write state");
    write_json(
        &fixture_dir.join("metadata.json"),
        json!({
            "bundle": 99,
            "scenario_id": "loader-smoke",
            "invariant": "loader invariant",
            "expected_render_policy": "show",
            "surfaces_exercised": ["loader"],
            "source_lifecycle_refs": ["source-1"],
            "anonymization_cert": "synthetic",
            "retention_policy": "test-only",
            "prompt_fingerprint_baseline": "fingerprint",
            "trust_factors_dominant": ["source_reliability"],
            "pass_fail_definition": "loader parses the fixture",
            "fixture_design_notes": null,
            "post_action_state": null
        }),
    );
    write_json(
        &fixture_dir.join("inputs.json"),
        json!({
            "ability_name": "enrich_account_intelligence",
            "input_json": {
                "entity_type": "account",
                "entity_id": "acct-test-1",
                "schema_version": 1
            },
            "actor": "user",
            "mode": "evaluate",
            "dry_run": false
        }),
    );
    write_json(
        &fixture_dir.join("provider_replay.json"),
        json!({"version": 1, "fixtures": []}),
    );
    write_json(
        &fixture_dir.join("external_replay.json"),
        json!({"version": 1, "fixtures": []}),
    );
    write_json(&fixture_dir.join("expected_output.json"), json!({"ok": true}));
    write_json(
        &fixture_dir.join("expected_provenance.json"),
        json!({"sources": []}),
    );

    if include_expected_state {
        write_json(&fixture_dir.join("expected_state.json"), json!({"state": true}));
    }
}

fn write_json(path: &Path, value: serde_json::Value) {
    let contents = serde_json::to_string_pretty(&value).expect("serialize JSON");
    fs::write(path, contents).expect("write JSON fixture file");
}
