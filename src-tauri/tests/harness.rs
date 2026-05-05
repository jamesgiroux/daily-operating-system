#[path = "harness/mod.rs"]
mod harness;

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use harness::{discover_fixtures, load_fixture, FixtureLoadError, FixtureRef};
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
    write_json(&fixture_dir.join("provider_replay.json"), json!({"fixtures": []}));
    write_json(&fixture_dir.join("external_replay.json"), json!({"fixtures": []}));
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
