#[path = "harness/mod.rs"]
mod harness;

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use base64::Engine;
use dailyos_lib::abilities::registry::AbilityRegistry;
use dailyos_lib::services::context::{
    ExecutionMode, GleanAccountFacts, GleanClientHandle, SeedableRng, SeededRng,
};
use harness::{
    discover_fixtures, load_fixture, prepare_fixture_for_run, run_fixture, FixtureLoadError,
    FixtureRef, RunError, RunnerDeps,
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
fn runner_returns_not_yet_wired_for_ability_invocation_pending_chunk_3() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    write_minimal_fixture(temp_dir.path(), true);
    let fixture = load_fixture(temp_dir.path()).expect("fixture loads");
    let registry =
        AbilityRegistry::from_descriptors_checked(Vec::new()).expect("empty registry is valid");
    let deps = RunnerDeps {
        registry: Arc::new(registry),
    };

    let Err(error) = run_fixture(&deps, &fixture) else {
        panic!("chunk 2 should defer ability invocation");
    };

    match error {
        RunError::NotYetWired(message) => {
            assert!(message.contains("ability invocation pending W4-C bridge integration"));
        }
        other => panic!("expected NotYetWired, got {other:?}"),
    }
}

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
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
    write_json(&fixture_dir.join("inputs.json"), json!({"input": true}));
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
