#![cfg(feature = "release-gate")]

use std::ffi::OsString;
use std::fs;
use std::path::Path;

use dailyos_lib::harness::{
    compute_default_fixtures_hash, AbilityCategory, FixtureRunSummary, HarnessReport,
    RegressionClass, Severity,
};
use dailyos_lib::release_gate::{
    exit_code_for_evidence, parse_cli_from, parse_harness_report, run_gate_with_db_reader,
    validate_manual_evidence_json, GateConfig, GateEvidenceV1, GateStatus, ManualDbReader,
    EXIT_INFRA_FAILURE, EXIT_MANDATORY_FAILURE, EXIT_SUCCESS,
};
use serde_json::json;

const TEST_GIT_SHA: &str = "0123456789abcdef";

#[test]
fn release_gate_parses_harness_report_bundle_coverage() {
    let temp = tempfile::tempdir().expect("tempdir");
    let report_path = temp.path().join("harness-report.json");
    write_harness_report(
        &report_path,
        &[(1, true, 12), (5, true, 20), (13, true, 30)],
    );

    let parsed = parse_harness_report(&report_path).expect("report parses");

    assert_eq!(parsed.bundle_coverage.bundles_run, vec![1, 5, 13]);
    assert_eq!(parsed.bundle_coverage.bundles_passed, vec![1, 5, 13]);
    assert!(parsed.bundle_coverage.bundles_failed.is_empty());
}

#[test]
fn release_gate_requires_bleed_suite_green() {
    let temp = tempfile::tempdir().expect("tempdir");
    let report_path = temp.path().join("harness-report.json");
    let output_dir = temp.path().join("release-gate");
    write_harness_report(
        &report_path,
        &[(1, true, 12), (5, true, 20), (13, true, 30)],
    );
    write_dos288_evidence(&output_dir, "dos288_bleed_detection_test", "fail");
    write_dos288_evidence(&output_dir, "dos288_ownership_validator_test", "pass");
    let config = config_for_report(&report_path, &output_dir);

    let outcome = run_gate_with_db_reader(&config, &UnusedDbReader).expect("gate runs");
    let evidence = read_evidence(&outcome.evidence_json_path);

    assert_eq!(outcome.exit_code, EXIT_MANDATORY_FAILURE);
    assert!(evidence
        .invariants
        .iter()
        .any(|invariant| invariant.id == "dos288_bleed_detection_test"
            && invariant.status == GateStatus::Fail));
}

#[test]
fn release_gate_requires_get_entity_context_bundle1_parity() {
    let evidence = run_with_bundle_statuses(&[(1, false, 12), (5, true, 20), (13, true, 30)]);

    assert_eq!(exit_code_for_evidence(&evidence), EXIT_MANDATORY_FAILURE);
    assert!(evidence.invariants.iter().any(|invariant| {
        invariant.id == "bundle-1.get_entity_context_parity_subject_ownership_no_bleed"
            && invariant.status == GateStatus::Fail
    }));
}

#[test]
fn release_gate_requires_prepare_meeting_bundle5_no_resurrection() {
    let evidence = run_with_bundle_statuses(&[(1, true, 12), (5, false, 20), (13, true, 30)]);

    assert_eq!(exit_code_for_evidence(&evidence), EXIT_MANDATORY_FAILURE);
    assert!(evidence.invariants.iter().any(|invariant| {
        invariant.id == "bundle-5.prepare_meeting_correction_tombstone_no_resurrection"
            && invariant.status == GateStatus::Fail
    }));
}

#[test]
fn release_gate_requires_prepare_meeting_bundle13_subject_bleed_rejection() {
    let evidence = run_with_bundle_statuses(&[(1, true, 12), (5, true, 20), (13, false, 30)]);

    assert_eq!(exit_code_for_evidence(&evidence), EXIT_MANDATORY_FAILURE);
    assert!(evidence.invariants.iter().any(|invariant| {
        invariant.id == "bundle-13.prepare_meeting_subject_bleed_rejection"
            && invariant.status == GateStatus::Fail
    }));
}

#[test]
fn release_gate_mandatory_bundle_failsoft_failure_blocks() {
    let temp = tempfile::tempdir().expect("tempdir");
    let report_path = temp.path().join("harness-report.json");
    let output_dir = temp.path().join("release-gate");
    write_harness_report_with_regressions(
        &report_path,
        &[
            (1, true, 12, None),
            (5, true, 20, None),
            (
                13,
                false,
                30,
                Some((RegressionClass::LogicChange, Severity::FailSoft)),
            ),
        ],
    );
    write_dos288_evidence(&output_dir, "dos288_bleed_detection_test", "pass");
    write_dos288_evidence(&output_dir, "dos288_ownership_validator_test", "pass");
    let config = config_for_report(&report_path, &output_dir);

    let outcome = run_gate_with_db_reader(&config, &UnusedDbReader).expect("gate runs");
    let evidence = read_evidence(&outcome.evidence_json_path);

    assert_eq!(outcome.exit_code, EXIT_MANDATORY_FAILURE);
    assert!(evidence.suites.iter().any(|suite| {
        suite.name == "bundle-13" && suite.mandatory && suite.status == GateStatus::Fail
    }));
}

#[test]
fn release_gate_records_latency_p50_p99_from_suite_report() {
    let evidence = run_with_bundle_statuses(&[(1, true, 10), (5, true, 20), (13, true, 30)]);

    assert_eq!(evidence.latency.sample_count, 3);
    assert_eq!(evidence.latency.p50_ms, Some(20));
    assert_eq!(evidence.latency.p99_ms, Some(30));
}

#[test]
fn release_gate_writes_evidence_json_and_md() {
    let temp = tempfile::tempdir().expect("tempdir");
    let report_path = temp.path().join("harness-report.json");
    let output_dir = temp.path().join("release-gate");
    write_harness_report(
        &report_path,
        &[(1, true, 10), (5, true, 20), (13, true, 30)],
    );
    write_dos288_evidence(&output_dir, "dos288_bleed_detection_test", "pass");
    write_dos288_evidence(&output_dir, "dos288_ownership_validator_test", "pass");
    let config = config_for_report(&report_path, &output_dir);

    let outcome = run_gate_with_db_reader(&config, &UnusedDbReader).expect("gate runs");

    assert_eq!(outcome.exit_code, EXIT_SUCCESS);
    assert!(outcome.evidence_json_path.is_file());
    assert!(outcome.evidence_markdown_path.is_file());
    let markdown = fs::read_to_string(outcome.evidence_markdown_path).expect("read markdown");
    assert!(markdown.contains("# Release Gate PASS"));
    assert!(!markdown.contains("claim text"));
}

#[test]
fn release_gate_manual_evidence_twenty_meetings_required() {
    let value = json!({
        "meeting_count": 19,
        "date_range": { "start": "2026-05-01", "end": "2026-05-07" },
        "operator": "abcdef1234567890",
        "redaction_level": "hash",
        "seven_day_parallel_run_ref": "1234567890abcdef",
        "attached_artifacts": [{
            "artifact_id": "fedcba0987654321",
            "source_class": "manual_summary",
            "hash_prefix": "abcdef123456"
        }],
        "dos411_claim_backed_lifecycle_green": true,
        "dos412_sensitivity_rendering_green": true
    });

    let error = validate_manual_evidence_json(&value).expect_err("19 meetings rejected");

    assert!(error.contains("meeting_count"));
}

#[test]
fn release_gate_missing_harness_report_is_infra_failure() {
    let temp = tempfile::tempdir().expect("tempdir");
    let output_dir = temp.path().join("release-gate");
    write_dos288_evidence(&output_dir, "dos288_bleed_detection_test", "pass");
    write_dos288_evidence(&output_dir, "dos288_ownership_validator_test", "pass");
    let config = config_for_report(&temp.path().join("missing-report.json"), &output_dir);

    let outcome = run_gate_with_db_reader(&config, &UnusedDbReader).expect("gate writes evidence");

    assert_eq!(outcome.exit_code, EXIT_INFRA_FAILURE);
}

#[test]
fn release_gate_rejects_stale_harness_report() {
    let temp = tempfile::tempdir().expect("tempdir");
    let report_path = temp.path().join("harness-report.json");
    let output_dir = temp.path().join("release-gate");
    write_harness_report_with_binding(
        &report_path,
        &[(1, true, 12), (5, true, 20), (13, true, 30)],
        "fedcba9876543210",
        &live_fixtures_hash(),
    );
    write_dos288_evidence(&output_dir, "dos288_bleed_detection_test", "pass");
    write_dos288_evidence(&output_dir, "dos288_ownership_validator_test", "pass");
    let config = config_for_report(&report_path, &output_dir);

    let outcome = run_gate_with_db_reader(&config, &UnusedDbReader).expect("gate writes evidence");
    let evidence = read_evidence(&outcome.evidence_json_path);

    assert_eq!(outcome.exit_code, EXIT_INFRA_FAILURE);
    assert!(evidence.suites.iter().any(|suite| {
        suite.name == "harness"
            && suite.status == GateStatus::InfraFailure
            && suite
                .failures
                .iter()
                .any(|failure| failure.contains("harness-report-stale"))
    }));
}

#[test]
fn release_gate_rejects_mismatched_fixtures_hash() {
    let temp = tempfile::tempdir().expect("tempdir");
    let report_path = temp.path().join("harness-report.json");
    let output_dir = temp.path().join("release-gate");
    write_harness_report_with_binding(
        &report_path,
        &[(1, true, 12), (5, true, 20), (13, true, 30)],
        TEST_GIT_SHA,
        "0000000000000000",
    );
    write_dos288_evidence(&output_dir, "dos288_bleed_detection_test", "pass");
    write_dos288_evidence(&output_dir, "dos288_ownership_validator_test", "pass");
    let config = config_for_report(&report_path, &output_dir);

    let outcome = run_gate_with_db_reader(&config, &UnusedDbReader).expect("gate writes evidence");
    let evidence = read_evidence(&outcome.evidence_json_path);

    assert_eq!(outcome.exit_code, EXIT_INFRA_FAILURE);
    assert!(evidence.suites.iter().any(|suite| {
        suite.name == "harness"
            && suite.status == GateStatus::InfraFailure
            && suite
                .failures
                .iter()
                .any(|failure| failure.contains("harness-report-stale"))
    }));
}

fn run_with_bundle_statuses(bundle_statuses: &[(u32, bool, u64)]) -> GateEvidenceV1 {
    let temp = tempfile::tempdir().expect("tempdir");
    let report_path = temp.path().join("harness-report.json");
    let output_dir = temp.path().join("release-gate");
    write_harness_report(&report_path, bundle_statuses);
    write_dos288_evidence(&output_dir, "dos288_bleed_detection_test", "pass");
    write_dos288_evidence(&output_dir, "dos288_ownership_validator_test", "pass");
    let config = config_for_report(&report_path, &output_dir);

    let outcome = run_gate_with_db_reader(&config, &UnusedDbReader).expect("gate runs");

    read_evidence(&outcome.evidence_json_path)
}

fn config_for_report(report_path: &Path, output_dir: &Path) -> GateConfig {
    let args = vec![
        OsString::from("release-gate"),
        OsString::from("--no-run-tests"),
        OsString::from("--harness-report"),
        report_path.as_os_str().to_os_string(),
        OsString::from("--output-dir"),
        output_dir.as_os_str().to_os_string(),
        OsString::from("--git-sha"),
        OsString::from(TEST_GIT_SHA),
    ];
    parse_cli_from(args).expect("config parses")
}

fn write_harness_report(path: &Path, bundle_statuses: &[(u32, bool, u64)]) {
    write_harness_report_with_binding(path, bundle_statuses, TEST_GIT_SHA, &live_fixtures_hash());
}

fn write_harness_report_with_binding(
    path: &Path,
    bundle_statuses: &[(u32, bool, u64)],
    git_sha: &str,
    fixtures_hash: &str,
) {
    let statuses = bundle_statuses
        .iter()
        .map(|(bundle, passed, runtime_ms)| (*bundle, *passed, *runtime_ms, None))
        .collect::<Vec<_>>();
    write_harness_report_with_regressions_and_binding(path, &statuses, git_sha, fixtures_hash);
}

fn write_harness_report_with_regressions(
    path: &Path,
    bundle_statuses: &[(u32, bool, u64, Option<(RegressionClass, Severity)>)],
) {
    write_harness_report_with_regressions_and_binding(
        path,
        bundle_statuses,
        TEST_GIT_SHA,
        &live_fixtures_hash(),
    );
}

fn write_harness_report_with_regressions_and_binding(
    path: &Path,
    bundle_statuses: &[(u32, bool, u64, Option<(RegressionClass, Severity)>)],
    git_sha: &str,
    fixtures_hash: &str,
) {
    let mut report = HarnessReport::new();
    report.git_sha = git_sha.to_string();
    report.fixtures_hash = fixtures_hash.to_string();
    for (bundle, passed, runtime_ms, regression) in bundle_statuses {
        report.add_fixture_summary(FixtureRunSummary {
            fixture_dir: format!("fixtures/bundle-{bundle}"),
            bundle: Some(*bundle),
            scenario_id: format!("bundle-{bundle}-scenario"),
            category: if *bundle == 1 {
                AbilityCategory::Read
            } else {
                AbilityCategory::Transform
            },
            passed: *passed,
            continuous_score: Some(if *passed { 1.0 } else { 0.0 }),
            regression: regression.clone(),
            diff_count: if *passed { 0 } else { 1 },
            runtime_ms: *runtime_ms,
        });
    }
    report.finalize();
    report.write_json(path).expect("write harness report");
}

fn write_dos288_evidence(output_dir: &Path, selector: &str, status: &str) {
    fs::create_dir_all(output_dir).expect("create output dir");
    fs::write(
        output_dir.join(format!("{selector}.json")),
        serde_json::to_string_pretty(&json!({
            "status": status,
            "git_sha": TEST_GIT_SHA,
            "fixtures_hash": live_fixtures_hash()
        }))
        .expect("serialize"),
    )
    .expect("write dos288 evidence");
}

fn live_fixtures_hash() -> String {
    compute_default_fixtures_hash().expect("fixture hash computes")
}

fn read_evidence(path: &Path) -> GateEvidenceV1 {
    let contents = fs::read_to_string(path).expect("read evidence");
    serde_json::from_str(&contents).expect("parse evidence")
}

struct UnusedDbReader;

impl ManualDbReader for UnusedDbReader {
    fn open_readonly_schema_version(&self, _path: &Path) -> Result<String, String> {
        panic!("hermetic mode must not open a manual DB");
    }
}
