#![cfg(feature = "release-gate")]

use std::ffi::OsString;
use std::fs;
use std::path::Path;

use dailyos_lib::harness::{
    compute_default_fixtures_hash, AbilityCategory, FixtureRunSummary, HarnessReport,
};
use dailyos_lib::release_gate::{
    parse_cli_from, run_gate_with_db_reader, GateConfig, GateEvidenceV1, GateStatus,
    ManualDbReader, EXIT_MANDATORY_FAILURE, RELEASE_GATE_BUILD_GIT_SHA,
};

const FORCE_BLEED_REGRESSION_ENV: &str = "DAILYOS_DOS288_FORCE_BLEED_REGRESSION";

#[test]
fn release_gate_runs_dos288_subprocess_with_release_gate_feature() {
    let temp = tempfile::tempdir().expect("tempdir");
    let report_path = temp.path().join("harness-report.json");
    let output_dir = temp.path().join("release-gate");
    write_passing_harness_report(&report_path);
    let config = config_for_subprocess_run(&report_path, &output_dir);

    let _guard = EnvGuard::set(FORCE_BLEED_REGRESSION_ENV, "1");
    let outcome = run_gate_with_db_reader(&config, &UnusedDbReader).expect("gate writes evidence");
    let evidence = read_evidence(&outcome.evidence_json_path);

    assert_eq!(outcome.exit_code, EXIT_MANDATORY_FAILURE);
    assert!(evidence.suites.iter().any(|suite| {
        suite.name == "dos288_bleed_detection_test" && suite.status == GateStatus::Fail
    }));
    assert!(evidence.invariants.iter().any(|invariant| {
        invariant.id == "dos288_bleed_detection_test" && invariant.status == GateStatus::Fail
    }));
}

fn config_for_subprocess_run(report_path: &Path, output_dir: &Path) -> GateConfig {
    let args = vec![
        OsString::from("release-gate"),
        OsString::from("--harness-report"),
        report_path.as_os_str().to_os_string(),
        OsString::from("--output-dir"),
        output_dir.as_os_str().to_os_string(),
    ];
    parse_cli_from(args).expect("config parses")
}

fn write_passing_harness_report(path: &Path) {
    let mut report = HarnessReport::new();
    report.git_sha = RELEASE_GATE_BUILD_GIT_SHA.to_string();
    report.fixtures_hash = compute_default_fixtures_hash().expect("fixture hash computes");
    for bundle in [1, 5, 13] {
        report.add_fixture_summary(FixtureRunSummary {
            fixture_dir: format!("fixtures/bundle-{bundle}"),
            bundle: Some(bundle),
            scenario_id: format!("bundle-{bundle}-scenario"),
            category: if bundle == 1 {
                AbilityCategory::Read
            } else {
                AbilityCategory::Transform
            },
            passed: true,
            continuous_score: Some(1.0),
            regression: None,
            diff_count: 0,
            runtime_ms: 10,
        });
    }
    report.finalize();
    report.write_json(path).expect("write harness report");
}

fn read_evidence(path: &Path) -> GateEvidenceV1 {
    let contents = fs::read_to_string(path).expect("read evidence");
    serde_json::from_str(&contents).expect("parse evidence")
}

struct EnvGuard {
    name: &'static str,
    previous: Option<OsString>,
}

impl EnvGuard {
    fn set(name: &'static str, value: &str) -> Self {
        let previous = std::env::var_os(name);
        std::env::set_var(name, value);
        Self { name, previous }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        if let Some(value) = &self.previous {
            std::env::set_var(self.name, value);
        } else {
            std::env::remove_var(self.name);
        }
    }
}

struct UnusedDbReader;

impl ManualDbReader for UnusedDbReader {
    fn open_readonly_schema_version(&self, _path: &Path) -> Result<String, String> {
        panic!("hermetic mode must not open a manual DB");
    }
}
