#[allow(dead_code, unused_imports)]
#[path = "harness/mod.rs"]
mod harness;

use chrono::{DateTime, Utc};
use serde_json::json;
use std::path::PathBuf;

#[cfg(feature = "harness-hermetic")]
const HARNESS_DB_PATH_ENV: &str = "DAILYOS_HARNESS_DB_PATH";

#[test]
#[cfg(feature = "harness-hermetic")]
fn harness_hermetic_feature_active_when_enabled() {
    assert!(cfg!(feature = "harness-hermetic"));
}

#[test]
fn harness_hermetic_runner_rejects_non_fixture_db_path_under_feature() {
    let fixture = minimal_fixture();

    #[cfg(feature = "harness-hermetic")]
    {
        std::env::set_var(HARNESS_DB_PATH_ENV, "/tmp/dailyos-prod.sqlite");
        let result = harness::prepare_fixture_for_run(&fixture);
        std::env::remove_var(HARNESS_DB_PATH_ENV);

        let Err(error) = result else {
            panic!("non-fixture DB path should fail under harness-hermetic");
        };
        assert!(
            error.to_string().contains("harness hermetic invariant failed"),
            "unexpected error: {error}"
        );
    }

    #[cfg(not(feature = "harness-hermetic"))]
    {
        let _prepared = harness::prepare_fixture_for_run(&fixture)
            .expect("non-hermetic runner still uses in-memory DB");
    }
}

fn minimal_fixture() -> harness::EvalFixture {
    harness::EvalFixture {
        fixture_dir: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/bundle-2"),
        metadata: harness::FixtureMetadata {
            bundle: Some(216),
            scenario_id: "dos216-hermetic-feature".to_string(),
            invariant: "hermetic runner rejects production DB paths".to_string(),
            expected_render_policy: "show".to_string(),
            surfaces_exercised: vec!["harness".to_string()],
            source_lifecycle_refs: vec!["dos216".to_string()],
            anonymization_cert: "synthetic".to_string(),
            retention_policy: "test-only".to_string(),
            prompt_fingerprint_baseline: "fingerprint".to_string(),
            prompt_template_version: None,
            completion_text_hash: None,
            trust_factors_dominant: vec!["source_reliability".to_string()],
            pass_fail_definition: "runner enforces hermetic DB path".to_string(),
            fixture_design_notes: None,
            post_action_state: None,
        },
        state_sql: "-- dos216 hermetic feature smoke\n".to_string(),
        inputs_json: json!({}),
        provider_replay: json!({"version": 1, "fixtures": []}),
        external_replay: json!({"version": 1, "fixtures": []}),
        clock: DateTime::parse_from_rfc3339("2026-05-01T12:00:00Z")
            .expect("valid fixture clock")
            .with_timezone(&Utc),
        seed: 42,
        expected: harness::ExpectedArtifacts {
            output: json!({}),
            provenance: json!({}),
            state: None,
            expected_render_policy: "show".to_string(),
        },
    }
}
