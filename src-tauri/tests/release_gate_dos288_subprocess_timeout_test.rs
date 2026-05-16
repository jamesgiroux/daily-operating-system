#![cfg(feature = "release-gate")]

use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use dailyos_lib::harness::{
    compute_default_fixtures_hash, AbilityCategory, FixtureRunSummary, HarnessReport,
};
use dailyos_lib::release_gate::{
    parse_cli_from, run_gate_with_db_reader, GateConfig, GateEvidenceV1, GateStatus,
    ManualDbReader, DEFAULT_MANDATORY_BUNDLES, EXIT_INFRA_FAILURE, RELEASE_GATE_BUILD_GIT_SHA,
};

const CARGO_ENV: &str = "CARGO";
const TIMEOUT_SUMMARY: &str = "dos288-selector-timeout-exceeded";

#[test]
fn release_gate_dos288_subprocess_timeout_records_infra_failure() {
    let temp = tempfile::tempdir().expect("tempdir");
    let report_path = temp.path().join("harness-report.json");
    let output_dir = temp.path().join("release-gate");
    let fake_cargo = temp.path().join("fake-cargo");
    write_passing_harness_report(&report_path);
    write_sleeping_cargo(&fake_cargo);
    let config = config_for_timeout_run(&report_path, &output_dir);

    let _guard = EnvGuard::set(CARGO_ENV, fake_cargo.as_os_str());
    let outcome = run_gate_with_db_reader(&config, &UnusedDbReader).expect("gate writes evidence");
    let evidence = read_evidence(&outcome.evidence_json_path);

    assert_eq!(outcome.exit_code, EXIT_INFRA_FAILURE);
    assert!(evidence.suites.iter().any(|suite| {
        suite.name == "dos288_bleed_detection_test"
            && suite.status == GateStatus::InfraFailure
            && suite.failure_summary.as_deref() == Some(TIMEOUT_SUMMARY)
    }));
}

fn config_for_timeout_run(report_path: &Path, output_dir: &Path) -> GateConfig {
    let args = vec![
        OsString::from("release-gate"),
        OsString::from("--harness-report"),
        report_path.as_os_str().to_os_string(),
        OsString::from("--output-dir"),
        output_dir.as_os_str().to_os_string(),
        OsString::from("--dos288-timeout-secs"),
        OsString::from("1"),
    ];
    parse_cli_from(args).expect("config parses")
}

fn write_sleeping_cargo(path: &Path) {
    fs::write(path, "#!/bin/sh\nexec /bin/sleep 30\n").expect("write fake cargo");
    let mut permissions = fs::metadata(path)
        .expect("fake cargo metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("fake cargo executable");
}

fn write_passing_harness_report(path: &Path) {
    let mut report = HarnessReport::new();
    report.git_sha = RELEASE_GATE_BUILD_GIT_SHA.to_string();
    report.fixtures_hash = compute_default_fixtures_hash().expect("fixture hash computes");
    for bundle_name in DEFAULT_MANDATORY_BUNDLES {
        let bundle = bundle_name
            .strip_prefix("bundle-")
            .expect("bundle prefix")
            .parse::<u32>()
            .expect("bundle number");
        report.add_fixture_summary(FixtureRunSummary {
            fixture_dir: format!("fixtures/{bundle_name}"),
            bundle: Some(bundle),
            scenario_id: format!("{bundle_name}-scenario"),
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
    fn set(name: &'static str, value: &std::ffi::OsStr) -> Self {
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
