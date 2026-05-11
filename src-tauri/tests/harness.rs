#![cfg(feature = "release-gate")]

#[path = "harness/mod.rs"]
mod harness;

use std::{
    collections::BTreeSet,
    fs,
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
    process::{Command, Output},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use base64::Engine;
use dailyos_lib::abilities::prepare_meeting::{prepare_meeting, prompts, PrepareMeetingInput};
use dailyos_lib::abilities::registry::{AbilityPolicy, McpExposure, SignalPolicy};
use dailyos_lib::abilities::{
    AbilityCategory, AbilityContext, AbilityDescriptor, AbilityError, AbilityRegistry, Actor,
    NOOP_ABILITY_TRACER,
};
use dailyos_lib::intelligence::provider::{
    canonical_prompt_hash, CanonicalPromptRequest, Completion, FingerprintMetadata,
    IntelligenceProvider, ModelName, ModelTier, PromptInput, ProviderError, ProviderKind,
};
use dailyos_lib::services::context::{
    ClaimDismissalSurface, ExecutionMode, ExternalClientError, GleanAccountFacts,
    GleanClientHandle, SeedableRng, SeededRng,
};
use harness::{
    baseline_fingerprint_for_fixture, canonical_json_eq, diff_internal_provenance,
    diff_rendered_provenance, discover_fixtures, load_fixture, prepare_fixture_for_run,
    run_fixture, run_harness_suite, severity_of, CategoryScorer, ClassificationFingerprint,
    FixtureLoadError, FixtureRef, FixtureRunSummary, HarnessReport, MaintenanceScorer,
    PublishScorer, ReadScorer, RegressionClass, RegressionClassifier, RunError, RunnerDeps,
    Severity, TransformScorer,
};
use serde_json::json;

#[test]
fn loader_loads_all_committed_bundles() {
    let root = fixture_root();
    let discovered: Vec<FixtureRef> =
        discover_fixtures(&[root.as_path()]).expect("fixture discovery succeeds");

    let expected = BTreeSet::from([1_u32, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13]);
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
fn prepare_meeting_eval_fixtures_cover_required_scenarios() {
    let root = fixture_root();
    let discovered: Vec<FixtureRef> =
        discover_fixtures(&[root.as_path()]).expect("fixture discovery succeeds");
    let mut prepare_bundles = BTreeSet::new();
    let mut scenario_tags = BTreeSet::new();

    for fixture_ref in discovered {
        let fixture = load_fixture(&fixture_ref.fixture_dir).expect("fixture loads");
        if fixture
            .metadata
            .surfaces_exercised
            .iter()
            .any(|surface| surface == "prepare_meeting")
        {
            prepare_bundles.insert(fixture.metadata.bundle.expect("bundle metadata is set"));
            scenario_tags.extend(fixture.metadata.surfaces_exercised);
        }
    }

    assert!(
        prepare_bundles.len() >= 5,
        "at least five prepare_meeting fixtures are committed"
    );
    assert!(
        prepare_bundles.contains(&5),
        "bundle-5 parity fixture exists"
    );
    assert!(scenario_tags.contains("first-meeting-person"));
    assert!(scenario_tags.contains("recurring-one-on-one"));
    assert!(scenario_tags.contains("multi-attendee-known-account"));
    assert!(scenario_tags.contains("stale-glean"));
    assert!(scenario_tags.contains("revoked-source"));
    assert!(scenario_tags.contains("subject-bleed-gate"));
}

#[test]
fn prepare_meeting_bundle5_parity_fixture_is_byte_identical() {
    let bundle_dir = fixture_root().join("bundle-5");
    let legacy = fs::read_to_string(bundle_dir.join("legacy_output.json"))
        .expect("bundle-5 legacy output fixture reads");
    let expected = fs::read_to_string(bundle_dir.join("expected_output.json"))
        .expect("bundle-5 expected output fixture reads");

    assert_eq!(legacy, expected);
}

#[test]
fn prepare_meeting_public_fixtures_execute_without_private_context() {
    for bundle in [5, 9, 10, 11, 12, 13] {
        let fixture = bundle_fixture(bundle);
        assert!(
            fixture.inputs_json["input_json"].get("context").is_none(),
            "bundle-{bundle} must not pass private prepare_meeting context"
        );

        let prepared = prepare_fixture_for_run(&fixture)
            .unwrap_or_else(|error| panic!("bundle-{bundle} should prepare: {error}"));
        let services = prepared.service_context();
        let input: PrepareMeetingInput =
            serde_json::from_value(fixture.inputs_json["input_json"].clone())
                .unwrap_or_else(|error| panic!("bundle-{bundle} input parses: {error}"));
        let ctx = AbilityContext::new(
            &services,
            &prepared.provider,
            &NOOP_ABILITY_TRACER,
            Actor::User,
            None,
            ClaimDismissalSurface::Eval,
        );
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        let output = runtime
            .block_on(prepare_meeting(&ctx, input))
            .unwrap_or_else(|error| panic!("bundle-{bundle} should execute: {error:?}"));
        assert_eq!(output.data().schema_version.0, 1);
        let expected_prompt_hash = fixture.provider_replay["fixtures"][0]["canonical_prompt_hash"]
            .as_str()
            .unwrap_or_else(|| panic!("bundle-{bundle} provider replay has canonical hash"));
        let actual_prompt_hash = output
            .provenance()
            .prompt_fingerprint
            .as_ref()
            .unwrap_or_else(|| panic!("bundle-{bundle} emits prompt fingerprint"))
            .canonical_prompt_hash
            .0
            .as_str();
        assert_eq!(
            actual_prompt_hash, expected_prompt_hash,
            "bundle-{bundle} provenance canonical_prompt_hash must match replay key"
        );
        if bundle == 13 {
            let brief = output.data();
            assert!(
                brief
                    .topics
                    .iter()
                    .all(|topic| topic.subject.id != "dos287-adjacent-example"),
                "bundle-13 must reject direct adjacent-account subject/source bleed"
            );
            assert!(
                brief
                    .topics
                    .iter()
                    .any(|topic| topic.subject.id == "dos287-target-example"),
                "bundle-13 must retain the in-scope target account topic"
            );
        }
    }
}

#[test]
fn prepare_meeting_bundle13_filters_adjacent_source_ref_claim_from_prompt_input() {
    let fixture = bundle_fixture(13);
    let prepared = prepare_fixture_for_run(&fixture).expect("bundle-13 should prepare");
    let completion = fixture.provider_replay["fixtures"][0]["completion"]
        .as_str()
        .expect("bundle-13 completion text")
        .to_string();
    let provider = PromptCaptureProvider::new(completion);
    let services = prepared.service_context();
    let input: PrepareMeetingInput =
        serde_json::from_value(fixture.inputs_json["input_json"].clone())
            .expect("bundle-13 input parses");
    let ctx = AbilityContext::new(
        &services,
        &provider,
        &NOOP_ABILITY_TRACER,
        Actor::User,
        None,
        ClaimDismissalSurface::Eval,
    );
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");

    runtime
        .block_on(prepare_meeting(&ctx, input))
        .expect("bundle-13 should execute");
    let prompt = provider.captured_prompt();
    let canonical_inputs = prompt
        .canonical_json_inputs
        .expect("prepare_meeting prompt has canonical JSON inputs");
    let evidence = canonical_inputs
        .pointer("/context/evidence")
        .and_then(serde_json::Value::as_array)
        .expect("canonical prompt context evidence array");

    assert!(
        evidence
            .iter()
            .any(|source| source["id"].as_str() == Some("src-b13-target")),
        "target-account source must remain in prompt evidence"
    );
    assert!(
        evidence
            .iter()
            .all(|source| source["id"].as_str() != Some("src-b13-adjacent")),
        "adjacent source_ref-matched claim must not enter prompt evidence"
    );
    assert!(
        !serde_json::to_string(&canonical_inputs)
            .expect("canonical prompt inputs serialize")
            .contains("Adjacent Example has an unrelated infrastructure escalation"),
        "adjacent claim text must not cross the provider boundary"
    );
}

#[test]
fn harness_loads_fixture_manifest_and_metadata() {
    let root = fixture_root();
    let discovered: Vec<FixtureRef> =
        discover_fixtures(&[root.as_path()]).expect("fixture discovery succeeds");

    assert!(!discovered.is_empty(), "committed fixtures are discovered");

    for fixture_ref in discovered {
        let fixture = load_fixture(&fixture_ref.fixture_dir).expect("fixture loads");
        let bundle = fixture.metadata.bundle.expect("bundle metadata is set");

        assert!(fixture_ref.has_label(&format!("bundle-{bundle}")));
        assert!(!fixture.metadata.scenario_id.trim().is_empty());
        assert!(!fixture.metadata.invariant.trim().is_empty());
        assert!(!fixture.metadata.expected_render_policy.trim().is_empty());
    }
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
                "auth_scope_id": "harness-default-tenant",
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
fn harness_loads_provider_replay_by_canonical_prompt_hash() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    write_minimal_fixture(temp_dir.path(), true);
    let prompt = PromptInput::new("provider replay prompt");
    let fingerprint_metadata = FingerprintMetadata::default();
    let replay_hash = canonical_prompt_hash(CanonicalPromptRequest {
        prompt: &prompt,
        fingerprint_metadata: &fingerprint_metadata,
    });
    write_json(
        &temp_dir.path().join("provider_replay.json"),
        json!({
            "version": 1,
            "fixtures": [{
                "canonical_prompt_hash": replay_hash,
                "completion": { "text": "fixture completion" }
            }]
        }),
    );
    let fixture = load_fixture(temp_dir.path()).expect("fixture loads");
    let prepared = prepare_fixture_for_run(&fixture).expect("fixture prepares");

    let completion =
        complete_replay_provider(&prepared.provider, prompt).expect("provider replay fixture hit");

    assert_eq!(completion, "fixture completion");
}

#[test]
fn harness_replay_provenance_hash_uses_non_default_lookup_metadata() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    write_minimal_fixture(temp_dir.path(), true);
    let rendered = prompts::render_prompt(r#"{"meeting_id":"meeting-non-default"}"#, 7);
    let prompt = rendered.prompt_input();
    let fingerprint_metadata = FingerprintMetadata {
        provider: ProviderKind::OpenAI,
        model: ModelName::new("gpt-replay-non-default"),
        temperature: 0.42,
        top_p: Some(0.91),
        seed: Some(8_675_309),
        tokens_input: None,
        tokens_output: None,
        provider_completion_id: None,
    };
    let replay_hash = canonical_prompt_hash(CanonicalPromptRequest {
        prompt: &prompt,
        fingerprint_metadata: &fingerprint_metadata,
    });
    write_json(
        &temp_dir.path().join("provider_replay.json"),
        json!({
            "version": 1,
            "provider": "openai",
            "model": "gpt-replay-non-default",
            "temperature": 0.42,
            "top_p": 0.91,
            "seed": 8675309,
            "fixtures": [{
                "canonical_prompt_hash": replay_hash.clone(),
                "completion": { "text": "fixture completion" }
            }]
        }),
    );
    let fixture = load_fixture(temp_dir.path()).expect("fixture loads");
    let prepared = prepare_fixture_for_run(&fixture).expect("fixture prepares");
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");

    let completion = runtime
        .block_on(prepared.provider.complete(prompt, ModelTier::Synthesis))
        .expect("provider replay fixture hit");
    let fingerprint = prompts::fingerprint_from_completion(&completion, &rendered);

    assert_eq!(completion.text, "fixture completion");
    assert_eq!(fingerprint.provider, "openai");
    assert_eq!(fingerprint.model.0, "gpt-replay-non-default");
    assert_eq!(fingerprint.canonical_prompt_hash.0, replay_hash);
}

#[test]
fn harness_rejects_provider_replay_with_only_legacy_prompt_hash() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    write_minimal_fixture(temp_dir.path(), true);
    write_json(
        &temp_dir.path().join("provider_replay.json"),
        json!({
            "version": 1,
            "fixtures": [{
                "prompt_replay_hash": "legacy-only",
                "completion": { "text": "fixture completion" }
            }]
        }),
    );
    let fixture = load_fixture(temp_dir.path()).expect("fixture loads");
    let error = match prepare_fixture_for_run(&fixture) {
        Ok(_) => panic!("legacy-only provider replay hash should fail"),
        Err(error) => error,
    };

    match error {
        RunError::ProviderReplayInvalid(message) => {
            assert!(
                message.contains("legacy prompt_replay_hash"),
                "unexpected ProviderReplayInvalid message: {message}"
            );
        }
        other => panic!("expected ProviderReplayInvalid, got {other:?}"),
    }
}

#[test]
fn harness_replay_provider_missing_hash_is_hard_failure() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    write_minimal_fixture(temp_dir.path(), true);
    let fixture = load_fixture(temp_dir.path()).expect("fixture loads");
    let prepared = prepare_fixture_for_run(&fixture).expect("fixture prepares");
    let prompt = PromptInput::new("provider replay prompt not present");
    let fingerprint_metadata = FingerprintMetadata::default();
    let expected_hash = canonical_prompt_hash(CanonicalPromptRequest {
        prompt: &prompt,
        fingerprint_metadata: &fingerprint_metadata,
    });

    let Err(error) = complete_replay_provider(&prepared.provider, prompt) else {
        panic!("missing replay hash should fail closed");
    };

    match error {
        ProviderError::FixtureMissingCompletion { hash } => {
            assert_eq!(hash, expected_hash);
        }
        other => panic!("expected FixtureMissingCompletion, got {other:?}"),
    }
}

#[test]
fn runner_reports_malformed_provider_replay_as_provider_replay_invalid() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    write_minimal_fixture(temp_dir.path(), true);
    write_json(
        &temp_dir.path().join("provider_replay.json"),
        json!({"version": 1}),
    );
    let fixture = load_fixture(temp_dir.path()).expect("fixture loads");
    let error = match prepare_fixture_for_run(&fixture) {
        Ok(_) => panic!("malformed provider replay should fail"),
        Err(error) => error,
    };

    match error {
        RunError::ProviderReplayInvalid(message) => {
            assert!(
                message.contains("fixtures array"),
                "unexpected ProviderReplayInvalid message: {message}"
            );
        }
        other => panic!("expected ProviderReplayInvalid, got {other:?}"),
    }
}

#[test]
fn harness_external_replay_missing_flow_is_hard_failure() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    write_minimal_fixture(temp_dir.path(), true);
    let fixture = load_fixture(temp_dir.path()).expect("fixture loads");
    let prepared = prepare_fixture_for_run(&fixture).expect("fixture prepares");
    let missing_account_id = "acct-missing-example";
    let expected_key = GleanClientHandle::request_key_for_fetch_account_facts(
        missing_account_id,
        "harness-default-tenant",
    );

    let error = prepared
        .external_clients
        .glean
        .fetch_account_facts(missing_account_id)
        .expect_err("missing external replay flow should fail closed");

    match error {
        ExternalClientError::ReplayFixtureMissing(missing) => {
            assert_eq!(missing.request_key_hex, expected_key.to_hex());
            assert_eq!(missing.method, "GET");
        }
        other => panic!("expected ReplayFixtureMissing, got {other:?}"),
    }
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
        diff.kind == harness::DiffKind::ProvenanceMismatch && diff.path == "/sources/0/source_asof"
    }));
}

#[test]
fn harness_provenance_warning_diff_fails() {
    let expected = expected_artifacts(
        json!({"ok": true}),
        json!({"warnings": ["expected warning"]}),
        None,
        "show-public-only",
    );
    let actual = run_result(
        json!({"ok": true}),
        json!({"warnings": ["actual warning"]}),
        None,
    );

    let score = ReadScorer.score(&expected, &actual);

    assert!(!score.passed);
    assert!(score
        .diffs
        .iter()
        .any(|diff| { diff.kind == harness::DiffKind::SourceWarning && diff.path == "/warnings" }));
}

#[test]
fn harness_expected_internal_provenance_diff_fails_full_envelope() {
    let expected = expected_artifacts(
        json!({"ok": true}),
        json!({
            "invocation_id": "expected",
            "prompt_hash": "expected-hash",
            "seed": 1,
            "summary": "visible"
        }),
        None,
        "show-public-only",
    );
    let actual = run_result(
        json!({"ok": true}),
        json!({
            "invocation_id": "actual",
            "prompt_hash": "actual-hash",
            "seed": 2,
            "summary": "visible"
        }),
        None,
    );

    let score = ReadScorer.score(&expected, &actual);

    assert!(!score.passed);
    assert!(score.diffs.iter().any(|diff| {
        diff.kind == harness::DiffKind::ProvenanceMismatch && diff.path == "/invocation_id"
    }));
    assert!(score.diffs.iter().any(|diff| {
        diff.kind == harness::DiffKind::ProvenanceMismatch && diff.path == "/prompt_hash"
    }));
    assert!(score.diffs.iter().any(|diff| {
        diff.kind == harness::DiffKind::ProvenanceMismatch && diff.path == "/seed"
    }));
}

#[test]
fn harness_expected_rendered_provenance_diff_via_eval_bridge_smoke_when_dos217_landed() {
    let expected = expected_artifacts(
        json!({"ok": true}),
        json!({
            "invocation_id": "expected",
            "prompt_hash": "expected-hash",
            "seed": 1,
            "summary": "visible"
        }),
        None,
        "show",
    );
    let actual_internal_only_changed = run_result(
        json!({"ok": true}),
        json!({
            "invocation_id": "actual",
            "prompt_hash": "actual-hash",
            "seed": 2,
            "summary": "visible"
        }),
        None,
    );
    let actual_visible_changed = run_result(
        json!({"ok": true}),
        json!({
            "invocation_id": "actual",
            "prompt_hash": "actual-hash",
            "seed": 2,
            "summary": "changed"
        }),
        None,
    );

    let passing_score = ReadScorer.score(&expected, &actual_internal_only_changed);
    let failing_score = ReadScorer.score(&expected, &actual_visible_changed);

    assert!(passing_score.passed, "{:?}", passing_score.diffs);
    assert!(!failing_score.passed);
    assert!(failing_score.diffs.iter().any(|diff| {
        diff.kind == harness::DiffKind::ProvenanceMismatch && diff.path == "/summary"
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
fn transform_scorer_partial_match_below_threshold_fails() {
    let expected = expected_artifacts(
        json!({
            "field_a": "match-a",
            "field_b": "match-b",
            "field_c": "expected-c",
            "field_d": "expected-d",
            "field_e": "expected-e"
        }),
        json!({}),
        None,
        "show-public-only",
    );
    let actual = run_result(
        json!({
            "field_a": "match-a",
            "field_b": "match-b",
            "field_c": "actual-c",
            "field_d": "actual-d",
            "field_e": "actual-e"
        }),
        json!({}),
        None,
    );

    let score = TransformScorer { threshold: 0.8 }.score(&expected, &actual);

    assert!(!score.passed);
    assert_eq!(score.category, harness::AbilityCategory::Transform);
    assert_eq!(score.diffs.len(), 3);
    let continuous_score = score.continuous_score.expect("continuous score");
    assert!(
        (continuous_score - 0.4).abs() < f64::EPSILON,
        "expected score 0.4, got {continuous_score}"
    );
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
fn diff_rendered_documented_as_incomplete_with_todo_marker() {
    let scoring_source = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/harness/scoring.rs"),
    )
    .expect("read scoring source");

    assert!(scoring_source.contains("Known incomplete"));
    assert!(scoring_source.contains("TODO: replace with ADR-0108 actor renderer when W5/W6 lands"));
}

#[test]
#[ignore = "documents desired ADR-0108 allowlist behavior once W5/W6 actor rendering lands"]
fn diff_rendered_provenance_adr_0108_renderer_allows_only_public_fields() {
    let diffs = diff_rendered_provenance(
        &json!({
            "summary": "visible",
            "sources": [{"title": "source-a"}]
        }),
        &json!({
            "summary": "visible",
            "sources": [{"title": "source-a"}],
            "debug_context": {
                "raw_prompt": "fixture prompt",
                "workspace_path": "/tmp/dailyos-fixture"
            }
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

#[test]
fn harness_fixture_labels_core_regression_edge_subset() {
    let fixtures = [
        FixtureRef {
            fixture_dir: PathBuf::from("fixtures/bundle-1/core"),
            labels: vec![
                "@core".to_string(),
                "@regression".to_string(),
                "@golden-daily-loop".to_string(),
            ],
        },
        FixtureRef {
            fixture_dir: PathBuf::from("fixtures/bundle-5/edge"),
            labels: vec!["@edge".to_string(), "@golden-daily-loop".to_string()],
        },
        FixtureRef {
            fixture_dir: PathBuf::from("fixtures/bundle-6/regression"),
            labels: vec!["@regression".to_string()],
        },
    ];

    let selected_dirs = |label: &str| -> Vec<String> {
        fixtures
            .iter()
            .filter(|fixture| fixture.has_label(label))
            .map(|fixture| fixture.fixture_dir.display().to_string())
            .collect()
    };

    assert_eq!(selected_dirs("@core"), vec!["fixtures/bundle-1/core"]);
    assert_eq!(
        selected_dirs("@regression"),
        vec!["fixtures/bundle-1/core", "fixtures/bundle-6/regression"]
    );
    assert_eq!(selected_dirs("@edge"), vec!["fixtures/bundle-5/edge"]);
    assert_eq!(
        selected_dirs("@golden-daily-loop"),
        vec!["fixtures/bundle-1/core", "fixtures/bundle-5/edge"]
    );
}

#[test]
fn classifier_input_change_takes_priority_when_inputs_hash_diff() {
    let baseline = classification_fingerprint();
    let mut current = baseline.clone();
    current.inputs_hash = "inputs-current".to_string();
    current.prompt_template_version = Some("prompt-v2".to_string());

    assert_eq!(
        RegressionClassifier.classify(&baseline, &current, &[]),
        Some((RegressionClass::InputChange, Severity::Hard))
    );
}

#[test]
fn classifier_input_change_when_state_sql_hash_diff() {
    let baseline = classification_fingerprint();
    let mut current = baseline.clone();
    current.state_sql_hash = "state-current".to_string();

    assert_eq!(
        RegressionClassifier.classify(&baseline, &current, &[]),
        Some((RegressionClass::InputChange, Severity::Hard))
    );
}

#[test]
fn classifier_prompt_change_when_template_version_diff_and_inputs_match() {
    let baseline = classification_fingerprint();
    let mut current = baseline.clone();
    current.prompt_template_version = Some("prompt-v2".to_string());

    assert_eq!(
        RegressionClassifier.classify(&baseline, &current, &[]),
        Some((RegressionClass::PromptChange, Severity::FailSoft))
    );
}

#[test]
fn classifier_canonicalization_bug_when_canonical_hash_diff_and_template_inputs_match() {
    let baseline = classification_fingerprint();
    let mut current = baseline.clone();
    current.canonical_prompt_hash = Some("canonical-current".to_string());

    assert_eq!(
        RegressionClassifier.classify(&baseline, &current, &[]),
        Some((RegressionClass::CanonicalizationBug, Severity::Hard))
    );
}

#[test]
fn classifier_provider_drift_when_completion_text_diff_and_fingerprint_matches() {
    let baseline = classification_fingerprint();
    let mut current = baseline.clone();
    current.completion_text_hash = Some("completion-current".to_string());

    assert_eq!(
        RegressionClassifier.classify(&baseline, &current, &[]),
        Some((RegressionClass::ProviderDrift, Severity::FailSoft))
    );
}

#[test]
fn classifier_skips_provider_drift_when_baseline_completion_text_hash_absent() {
    let mut baseline = classification_fingerprint();
    baseline.completion_text_hash = None;
    let mut current = baseline.clone();
    current.completion_text_hash = Some("completion-current".to_string());

    assert_eq!(
        RegressionClassifier.classify(&baseline, &current, &[]),
        None
    );
}

#[test]
fn classifier_logic_change_when_no_fingerprint_diff_but_score_diffs_present() {
    let baseline = classification_fingerprint();
    let current = baseline.clone();
    let diffs = vec![score_diff()];

    assert_eq!(
        RegressionClassifier.classify(&baseline, &current, &diffs),
        Some((RegressionClass::LogicChange, Severity::FailSoft))
    );
}

#[test]
fn harness_classifies_provider_drift() {
    let baseline = classification_fingerprint();
    let mut current = baseline.clone();
    current.completion_text_hash = Some("completion-current".to_string());

    assert_eq!(
        RegressionClassifier.classify(&baseline, &current, &[]),
        Some((RegressionClass::ProviderDrift, Severity::FailSoft))
    );
}

#[test]
fn harness_classifies_prompt_change_fail_soft() {
    let baseline = classification_fingerprint();
    let mut current = baseline.clone();
    current.prompt_template_version = Some("prompt-v2".to_string());

    assert_eq!(
        RegressionClassifier.classify(&baseline, &current, &[]),
        Some((RegressionClass::PromptChange, Severity::FailSoft))
    );
}

#[test]
fn harness_classifies_input_change() {
    let baseline = classification_fingerprint();
    let mut current = baseline.clone();
    current.inputs_hash = "inputs-current".to_string();

    assert_eq!(
        RegressionClassifier.classify(&baseline, &current, &[]),
        Some((RegressionClass::InputChange, Severity::Hard))
    );
}

#[test]
fn harness_classifies_canonicalization_bug_hard_fail() {
    let baseline = classification_fingerprint();
    let mut current = baseline.clone();
    current.canonical_prompt_hash = Some("canonical-current".to_string());

    assert_eq!(
        RegressionClassifier.classify(&baseline, &current, &[]),
        Some((RegressionClass::CanonicalizationBug, Severity::Hard))
    );
}

#[test]
fn harness_classifies_logic_change_fail_soft() {
    let baseline = classification_fingerprint();
    let current = baseline.clone();
    let diffs = vec![score_diff()];

    assert_eq!(
        RegressionClassifier.classify(&baseline, &current, &diffs),
        Some((RegressionClass::LogicChange, Severity::FailSoft))
    );
}

#[test]
fn classifier_returns_none_when_all_match() {
    let baseline = classification_fingerprint();
    let current = baseline.clone();

    assert_eq!(
        RegressionClassifier.classify(&baseline, &current, &[]),
        None
    );
}

#[test]
fn classifier_severity_input_change_is_hard() {
    assert_eq!(severity_of(&RegressionClass::InputChange), Severity::Hard);
}

#[test]
fn classifier_severity_prompt_change_is_fail_soft() {
    assert_eq!(
        severity_of(&RegressionClass::PromptChange),
        Severity::FailSoft
    );
}

#[test]
fn classifier_severity_canonicalization_bug_is_hard() {
    assert_eq!(
        severity_of(&RegressionClass::CanonicalizationBug),
        Severity::Hard
    );
}

#[test]
fn classifier_precedence_input_beats_prompt_when_both_differ() {
    let baseline = classification_fingerprint();
    let mut current = baseline.clone();
    current.inputs_hash = "inputs-current".to_string();
    current.prompt_template_version = Some("prompt-v2".to_string());

    assert_eq!(
        RegressionClassifier.classify(&baseline, &current, &[]),
        Some((RegressionClass::InputChange, Severity::Hard))
    );
}

#[test]
fn baseline_fingerprint_reads_prompt_fingerprint_baseline_from_metadata() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    write_minimal_fixture(temp_dir.path(), true);
    let fixture = load_fixture(temp_dir.path()).expect("fixture loads");

    let fingerprint = baseline_fingerprint_for_fixture(&fixture);

    assert_eq!(
        fingerprint.canonical_prompt_hash,
        Some("fingerprint".to_string())
    );
}

#[test]
fn baseline_fingerprint_reads_completion_text_hash_from_metadata() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    write_minimal_fixture(temp_dir.path(), true);
    let metadata_path = temp_dir.path().join("metadata.json");
    let mut metadata_json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&metadata_path).expect("read metadata"))
            .expect("parse metadata");
    metadata_json["completion_text_hash"] = json!("completion-baseline");
    write_json(&metadata_path, metadata_json);
    let fixture = load_fixture(temp_dir.path()).expect("fixture loads");

    let fingerprint = baseline_fingerprint_for_fixture(&fixture);

    assert_eq!(
        fingerprint.completion_text_hash,
        Some("completion-baseline".to_string())
    );
}

#[test]
fn harness_w2_prompt_replay_hash_rejected_as_canonical_regression_hash() {
    let mut fixture = bundle_fixture(2);
    fixture.metadata.prompt_fingerprint_baseline = "canonical-baseline".to_string();
    let result = run_result(
        json!({"ok": true}),
        json!({
            "prompt_fingerprint": {
                "prompt_replay_hash": "w2-replay-only"
            }
        }),
        None,
    );

    let fingerprint = harness::current_fingerprint_for_run(&fixture, &result);

    assert_eq!(
        fingerprint.canonical_prompt_hash,
        Some("canonical-baseline".to_string())
    );
    assert_ne!(
        fingerprint.canonical_prompt_hash,
        Some("w2-replay-only".to_string())
    );
}

#[test]
fn harness_metadata_json_required_fields_validated() {
    let root = fixture_root();
    let discovered = discover_fixtures(&[root.as_path()]).expect("fixture discovery succeeds");

    assert!(!discovered.is_empty(), "committed fixtures are discovered");

    for fixture_ref in discovered {
        let metadata_path = fixture_ref.fixture_dir.join("metadata.json");
        let metadata_json: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&metadata_path).expect("read metadata.json"))
                .expect("parse metadata.json");
        let fixture = load_fixture(&fixture_ref.fixture_dir).expect("metadata deserializes");

        assert!(
            fixture.metadata.bundle.is_some_and(|bundle| bundle > 0),
            "{} has populated bundle",
            metadata_path.display()
        );
        for field in [
            "scenario_id",
            "invariant",
            "expected_render_policy",
            "anonymization_cert",
            "retention_policy",
            "prompt_fingerprint_baseline",
            "pass_fail_definition",
        ] {
            assert!(
                metadata_json
                    .get(field)
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|value| !value.trim().is_empty()),
                "{} has populated {field}",
                metadata_path.display()
            );
        }
        for field in [
            "surfaces_exercised",
            "source_lifecycle_refs",
            "trust_factors_dominant",
        ] {
            assert!(
                metadata_json
                    .get(field)
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|value| !value.is_empty()),
                "{} has non-empty {field}",
                metadata_path.display()
            );
        }
    }
}

#[test]
fn harness_report_aggregates_per_fixture_summaries() {
    let mut report = HarnessReport::new();
    report.add_fixture_summary(fixture_summary(
        "fixtures/bundle-2",
        Some(2),
        "read-pass",
        harness::AbilityCategory::Read,
        true,
        None,
        0,
    ));
    report.add_fixture_summary(fixture_summary(
        "fixtures/bundle-3",
        Some(3),
        "transform-fail",
        harness::AbilityCategory::Transform,
        false,
        Some((RegressionClass::LogicChange, Severity::FailSoft)),
        2,
    ));

    report.finalize();

    assert_eq!(
        report.category_counts["Read"],
        harness::CategorySummary {
            total: 1,
            passed: 1,
            failed: 0,
        }
    );
    assert_eq!(
        report.category_counts["Transform"],
        harness::CategorySummary {
            total: 1,
            passed: 0,
            failed: 1,
        }
    );
    assert_eq!(report.regression_class_counts["LogicChange"], 1);
    assert_eq!(report.regression_class_counts["InputChange"], 0);
}

#[test]
fn harness_report_computes_bundle_coverage_from_fixtures() {
    let mut report = HarnessReport::new();
    report.add_fixture_summary(fixture_summary(
        "fixtures/bundle-2/a",
        Some(2),
        "bundle-2-pass",
        harness::AbilityCategory::Read,
        true,
        None,
        0,
    ));
    report.add_fixture_summary(fixture_summary(
        "fixtures/bundle-3/a",
        Some(3),
        "bundle-3-pass",
        harness::AbilityCategory::Read,
        true,
        None,
        0,
    ));
    report.add_fixture_summary(fixture_summary(
        "fixtures/bundle-3/b",
        Some(3),
        "bundle-3-fail",
        harness::AbilityCategory::Read,
        false,
        Some((RegressionClass::ProviderDrift, Severity::FailSoft)),
        1,
    ));

    report.finalize();

    assert_eq!(report.bundle_coverage.bundles_run, vec![2, 3]);
    assert_eq!(report.bundle_coverage.bundles_passed, vec![2]);
    assert_eq!(report.bundle_coverage.bundles_failed, vec![3]);
    assert_eq!(
        report.bundle_coverage.bundles_unblocked,
        vec![1, 2, 3, 4, 6, 7, 8]
    );
}

#[test]
fn harness_reports_bundle_coverage_1_and_5() {
    let mut report = HarnessReport::new();
    report.add_fixture_summary(fixture_summary(
        "fixtures/bundle-1/core",
        Some(1),
        "bundle-1-core",
        harness::AbilityCategory::Read,
        true,
        None,
        0,
    ));
    report.add_fixture_summary(fixture_summary(
        "fixtures/bundle-5/correction-resurrection",
        Some(5),
        "bundle-5-correction-resurrection",
        harness::AbilityCategory::Maintenance,
        false,
        Some((RegressionClass::LogicChange, Severity::FailSoft)),
        1,
    ));

    report.finalize();

    assert_eq!(report.bundle_coverage.bundles_run, vec![1, 5]);
    assert_eq!(report.bundle_coverage.bundles_passed, vec![1]);
    assert_eq!(report.bundle_coverage.bundles_failed, vec![5]);
    assert_eq!(report.regression_class_counts["LogicChange"], 1);
}

#[test]
fn harness_report_serializes_to_json_with_stable_field_order() {
    let mut report = HarnessReport::new();
    report.run_id = "harness-stable-run".to_string();
    report.started_at = fixed_report_time();
    report.finished_at = fixed_report_time();
    report.add_fixture_summary(fixture_summary(
        "fixtures/bundle-2",
        Some(2),
        "stable-order",
        harness::AbilityCategory::Read,
        true,
        None,
        0,
    ));
    report.finalize();
    report.started_at = fixed_report_time();
    report.finished_at = fixed_report_time();

    let first = serde_json::to_string_pretty(&report).expect("serialize first report");
    let second = serde_json::to_string_pretty(&report).expect("serialize second report");

    assert_eq!(first, second);
    assert_substrings_in_order(
        &first,
        &[
            "\"run_id\"",
            "\"git_sha\"",
            "\"fixtures_hash\"",
            "\"started_at\"",
            "\"finished_at\"",
            "\"fixtures\"",
            "\"bundle_coverage\"",
            "\"regression_class_counts\"",
            "\"category_counts\"",
        ],
    );
    let regression_counts =
        section_between(&first, "\"regression_class_counts\"", "\"category_counts\"");
    assert_substrings_in_order(
        regression_counts,
        &[
            "\"CanonicalizationBug\"",
            "\"InputChange\"",
            "\"LogicChange\"",
            "\"PromptChange\"",
            "\"ProviderDrift\"",
        ],
    );
    assert_substrings_in_order(
        section_after(&first, "\"category_counts\""),
        &[
            "\"Maintenance\"",
            "\"Publish\"",
            "\"Read\"",
            "\"Transform\"",
        ],
    );
}

#[test]
fn harness_report_writes_to_target_eval_harness_report_json() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let output_path = temp_dir.path().join("target/eval/harness-report.json");
    let mut report = HarnessReport::new();
    report.add_fixture_summary(fixture_summary(
        "fixtures/bundle-2",
        Some(2),
        "write-report",
        harness::AbilityCategory::Read,
        true,
        None,
        0,
    ));
    report.finalize();

    report
        .write_json(&output_path)
        .expect("write harness-report.json");

    let written = fs::read_to_string(&output_path).expect("read harness-report.json");
    let parsed: serde_json::Value =
        serde_json::from_str(&written).expect("parse harness-report.json");

    assert!(output_path.is_file());
    assert_eq!(
        parsed["fixtures"].as_array().expect("fixtures array").len(),
        1
    );
    assert_eq!(parsed["bundle_coverage"]["bundles_run"], json!([2]));
}

#[test]
fn run_harness_suite_iterates_all_provided_fixtures_and_writes_report() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let output_path = temp_dir.path().join("target/eval/harness-report.json");
    let fixture_refs = vec![
        FixtureRef {
            fixture_dir: fixture_root().join("bundle-2"),
            labels: vec!["bundle-2".to_string()],
        },
        FixtureRef {
            fixture_dir: fixture_root().join("bundle-3"),
            labels: vec!["bundle-3".to_string()],
        },
    ];
    let deps = synthetic_runner_deps();

    let report = run_harness_suite(&deps, &fixture_refs, &output_path)
        .expect("suite runs and writes report");

    assert_eq!(report.fixtures.len(), 2);
    assert_eq!(report.bundle_coverage.bundles_run, vec![2, 3]);
    assert!(report
        .fixtures
        .iter()
        .all(|summary| summary.category == harness::AbilityCategory::Read));
    assert!(report
        .fixtures
        .iter()
        .all(|summary| !summary.fixture_dir.is_empty()));
    assert_eq!(report.regression_class_counts["LogicChange"], 2);
    assert!(output_path.is_file());

    let written = fs::read_to_string(output_path).expect("read report");
    let parsed: serde_json::Value = serde_json::from_str(&written).expect("parse report");
    assert_eq!(
        parsed["fixtures"].as_array().expect("fixtures array").len(),
        2
    );
}

#[test]
fn harness_report_records_regression_class_when_classifier_fires() {
    let baseline = classification_fingerprint();
    let mut current = baseline.clone();
    current.inputs_hash = "inputs-current".to_string();
    let regression = RegressionClassifier
        .classify(&baseline, &current, &[])
        .expect("classifier fires");
    let mut report = HarnessReport::new();

    report.add_fixture_summary(fixture_summary(
        "fixtures/bundle-2",
        Some(2),
        "input-change",
        harness::AbilityCategory::Read,
        false,
        Some(regression.clone()),
        1,
    ));
    report.finalize();

    assert_eq!(regression, (RegressionClass::InputChange, Severity::Hard));
    assert_eq!(report.regression_class_counts["InputChange"], 1);
    assert_eq!(report.fixtures[0].regression.as_ref(), Some(&regression));
}

#[test]
fn eval_fixture_governance_rejects_phone_like_tokens() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let fixture_root = temp_dir.path().join("fixtures");
    fs::create_dir_all(&fixture_root).expect("create fixture root");
    fs::write(
        fixture_root.join("inputs.json"),
        r#"{"support_phone":"555-010-0000"}"#,
    )
    .expect("write fixture with phone-like token");

    let output = run_fixture_anonymization_lint_for_root(&fixture_root);
    let text = fixture_lint_output_text(&output);

    assert!(
        !output.status.success(),
        "lint must reject phone tokens\n{text}"
    );
    assert!(
        text.contains("phone-like-number"),
        "lint output must identify phone-like token\n{text}"
    );
}

#[test]
fn eval_fixture_governance_rejects_identity_map_in_tree() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let fixture_root = temp_dir.path().join("fixtures");
    fs::create_dir_all(&fixture_root).expect("create fixture root");
    fs::write(fixture_root.join("fixture_identity_map.json"), "{}")
        .expect("write identity map fixture");

    let output = run_fixture_anonymization_lint_for_root(&fixture_root);
    let text = fixture_lint_output_text(&output);

    assert!(
        !output.status.success(),
        "lint must reject in-tree identity maps\n{text}"
    );
    assert!(
        text.contains("identity-map-file"),
        "lint output must identify identity map\n{text}"
    );
}

#[test]
#[ignore = "TODO(): wire the harness startup guard that aborts non-harness-hermetic runs; current verification intentionally runs the non-feature harness binary"]
fn harness_requires_hermetic_feature() {
    panic!("TODO(DOS-216): harness runner must fail closed unless built with harness-hermetic");
}

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn classification_fingerprint() -> ClassificationFingerprint {
    ClassificationFingerprint {
        inputs_hash: "inputs-baseline".to_string(),
        state_sql_hash: "state-baseline".to_string(),
        prompt_template_version: Some("prompt-v1".to_string()),
        canonical_prompt_hash: Some("canonical-baseline".to_string()),
        completion_text_hash: Some("completion-baseline".to_string()),
    }
}

fn score_diff() -> harness::Diff {
    harness::Diff {
        kind: harness::DiffKind::OutputMismatch,
        path: "/value".to_string(),
        expected: json!("expected"),
        actual: json!("actual"),
    }
}

fn complete_replay_provider(
    provider: &dyn IntelligenceProvider,
    prompt: PromptInput,
) -> Result<String, ProviderError> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    runtime
        .block_on(provider.complete(prompt, ModelTier::Synthesis))
        .map(|completion| completion.text)
}

struct PromptCaptureProvider {
    completion: String,
    prompt: Mutex<Option<PromptInput>>,
}

impl PromptCaptureProvider {
    fn new(completion: String) -> Self {
        Self {
            completion,
            prompt: Mutex::new(None),
        }
    }

    fn captured_prompt(&self) -> PromptInput {
        self.prompt
            .lock()
            .expect("prompt capture mutex")
            .clone()
            .expect("provider captured a prompt")
    }
}

#[async_trait]
impl IntelligenceProvider for PromptCaptureProvider {
    async fn complete(
        &self,
        prompt: PromptInput,
        _tier: ModelTier,
    ) -> Result<Completion, ProviderError> {
        *self.prompt.lock().expect("prompt capture mutex") = Some(prompt);
        Ok(Completion {
            text: self.completion.clone(),
            fingerprint_metadata: FingerprintMetadata::default(),
        })
    }

    fn provider_kind(&self) -> ProviderKind {
        ProviderKind::Other("prompt_capture")
    }

    fn current_model(&self, _tier: ModelTier) -> ModelName {
        ModelName::new("prompt-capture")
    }
}

fn run_fixture_anonymization_lint_for_root(fixture_root: &Path) -> Output {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repo root")
        .to_path_buf();
    let script = repo_root.join("src-tauri/scripts/check_fixture_anonymization.sh");

    Command::new("bash")
        .arg(script)
        .current_dir(&repo_root)
        .env("DOS216_FIXTURE_LINT_ROOT_OVERRIDE", &repo_root)
        .env("DOS216_FIXTURE_LINT_FIXTURE_ROOT_OVERRIDE", fixture_root)
        .output()
        .expect("run DOS-216 fixture anonymization lint")
}

fn fixture_lint_output_text(output: &Output) -> String {
    format!(
        "--- stdout ---\n{}\n--- stderr ---\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
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

fn fixture_summary(
    fixture_dir: &str,
    bundle: Option<u32>,
    scenario_id: &str,
    category: harness::AbilityCategory,
    passed: bool,
    regression: Option<(RegressionClass, Severity)>,
    diff_count: usize,
) -> FixtureRunSummary {
    FixtureRunSummary {
        fixture_dir: fixture_dir.to_string(),
        bundle,
        scenario_id: scenario_id.to_string(),
        category,
        passed,
        continuous_score: None,
        regression,
        diff_count,
        runtime_ms: 1,
    }
}

fn fixed_report_time() -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339("2026-05-01T12:00:00Z")
        .expect("valid report time")
        .with_timezone(&chrono::Utc)
}

fn assert_substrings_in_order(haystack: &str, needles: &[&str]) {
    let mut offset = 0;
    for needle in needles {
        let position = haystack[offset..]
            .find(needle)
            .unwrap_or_else(|| panic!("missing substring `{needle}` in `{haystack}`"));
        offset += position + needle.len();
    }
}

fn section_between<'a>(haystack: &'a str, start: &str, end: &str) -> &'a str {
    let after_start = section_after(haystack, start);
    let end_position = after_start
        .find(end)
        .unwrap_or_else(|| panic!("missing section end `{end}` in `{after_start}`"));
    &after_start[..end_position]
}

fn section_after<'a>(haystack: &'a str, start: &str) -> &'a str {
    let start_position = haystack
        .find(start)
        .unwrap_or_else(|| panic!("missing section start `{start}` in `{haystack}`"));
    &haystack[start_position + start.len()..]
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
    write_json(
        &fixture_dir.join("expected_output.json"),
        json!({"ok": true}),
    );
    write_json(
        &fixture_dir.join("expected_provenance.json"),
        json!({"sources": []}),
    );

    if include_expected_state {
        write_json(
            &fixture_dir.join("expected_state.json"),
            json!({"state": true}),
        );
    }
}

fn write_json(path: &Path, value: serde_json::Value) {
    let contents = serde_json::to_string_pretty(&value).expect("serialize JSON");
    fs::write(path, contents).expect("write JSON fixture file");
}
