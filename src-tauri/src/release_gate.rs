use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::abilities::registry::AbilityRegistry;
use crate::db::ActionDb;
use crate::harness::{
    compute_default_fixtures_hash, run_harness_suite, BundleLoader, HarnessReport, RunnerDeps,
    Severity,
};

pub const RELEASE_GATE_SCHEMA_VERSION: &str = "release_gate_evidence_v1";
pub const EXIT_SUCCESS: u8 = 0;
pub const EXIT_MANDATORY_FAILURE: u8 = 1;
pub const EXIT_INFRA_FAILURE: u8 = 2;

pub const DEFAULT_MANDATORY_BUNDLES: &[&str] = &[
    "bundle-1",
    "bundle-5",
    "bundle-13",
    "bundle-14",
    "bundle-15",
    "bundle-16",
    "bundle-17",
    "bundle-18",
];
pub const DEFAULT_TRACKED_BUNDLES: &[&str] = &[
    "bundle-2",
    "bundle-3",
    "bundle-4",
    "bundle-6",
    "bundle-7",
    "bundle-8",
    "bundle-9",
    "bundle-10",
    "bundle-11",
    "bundle-12",
];
pub const RELEASE_GATE_BUILD_GIT_SHA: &str = env!("BUILD_GIT_SHA");

/// Edge-case fast subset cargo test targets that gate per-PR CI.
///
/// CI orchestration is required to invoke each named target with
/// `cargo test --features release-gate --test <name>` before the release
/// gate is allowed to pass. The targets exercise the per-axis regressions
/// and the edge-case unit/integration substrate; their absence from CI is
/// a release-gate failure.
pub const DEFAULT_MANDATORY_TEST_TARGETS: &[&str] = &["edge_cases_fast", "edge_cases_full"];

const DOS288_SELECTORS: &[&str] = &[
    "dos288_bleed_detection_test",
    "dos288_ownership_validator_test",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum GateMode {
    Hermetic,
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateConfig {
    pub mode: GateMode,
    pub bundle_filters: Vec<String>,
    pub mandatory_bundles: Vec<String>,
    pub tracked_bundles: Vec<String>,
    pub output_dir: PathBuf,
    pub harness_report: Option<PathBuf>,
    pub db_path: Option<PathBuf>,
    pub manual_evidence: Option<PathBuf>,
    pub run_tests: bool,
    pub git_sha: String,
}

impl GateConfig {
    pub fn default_output_dir() -> PathBuf {
        manifest_dir().join("target/release-gate")
    }

    fn default_harness_report_path() -> PathBuf {
        manifest_dir().join("target/eval/harness-report.json")
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GateEvidenceV1 {
    pub schema_version: String,
    pub run_id: String,
    pub mode: GateMode,
    pub generated_at: DateTime<Utc>,
    pub git_sha: String,
    pub db_schema_version: String,
    pub suites: Vec<SuiteResult>,
    pub invariants: Vec<InvariantResult>,
    pub mandatory_bundles: Vec<String>,
    pub tracked_bundles: Vec<String>,
    pub manual: Option<ManualDogfoodEvidence>,
    pub latency: LatencySummary,
    pub summary_markdown: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuiteResult {
    pub name: String,
    pub source: String,
    pub command_or_report: String,
    pub status: GateStatus,
    pub mandatory: bool,
    pub duration_ms: Option<u64>,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvariantResult {
    pub id: String,
    pub bundle: Option<String>,
    pub surface: String,
    pub status: GateStatus,
    pub mandatory: bool,
    pub evidence_ref: String,
    pub failure_summary: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateStatus {
    Pass,
    Fail,
    InfraFailure,
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManualDogfoodEvidence {
    pub meeting_count: u32,
    pub date_range: ManualDateRange,
    pub operator: HashOnly,
    pub redaction_level: ManualRedactionLevel,
    pub seven_day_parallel_run_ref: HashOnly,
    pub attached_artifacts: Vec<ManualArtifactRef>,
    pub dos411_claim_backed_lifecycle_green: bool,
    pub dos412_sensitivity_rendering_green: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HashOnly(String);

impl HashOnly {
    pub fn new(value: impl Into<String>) -> Result<Self, String> {
        let value = value.into();
        let valid_len = (8..=64).contains(&value.len());
        let valid_chars = value.chars().all(|ch| ch.is_ascii_hexdigit());
        if valid_len && valid_chars {
            Ok(Self(value))
        } else {
            Err("value must match ^[a-fA-F0-9]{8,64}$".to_string())
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Serialize for HashOnly {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for HashOnly {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManualRedactionLevel {
    Hash,
    Synthetic,
    Identifier,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManualDateRange {
    pub start: String,
    pub end: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManualArtifactRef {
    pub artifact_id: HashOnly,
    pub source_class: ManualSourceClass,
    pub hash_prefix: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManualSourceClass {
    ProspectMeeting,
    RenewalMeeting,
    AccountReview,
    ExecutiveBusinessReview,
    SupportCase,
    SuccessPlanReview,
    ManualSummary,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LatencySummary {
    pub source: String,
    pub sample_count: u32,
    pub p50_ms: Option<u64>,
    pub p99_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateOutcome {
    pub exit_code: u8,
    pub summary_markdown: String,
    pub evidence_json_path: PathBuf,
    pub evidence_markdown_path: PathBuf,
}

#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct GateError {
    message: String,
}

impl GateError {
    pub fn config(message: impl Into<String>) -> Self {
        Self {
            message: format!("release-gate config error: {}", message.into()),
        }
    }

    pub fn infra(message: impl Into<String>) -> Self {
        Self {
            message: format!("release-gate infra error: {}", message.into()),
        }
    }

    pub fn exit_code(&self) -> u8 {
        EXIT_INFRA_FAILURE
    }
}

pub trait ManualDbReader {
    fn open_readonly_schema_version(&self, path: &Path) -> Result<String, String>;
}

pub struct ActionDbManualReader;

impl ManualDbReader for ActionDbManualReader {
    fn open_readonly_schema_version(&self, path: &Path) -> Result<String, String> {
        let db =
            ActionDb::open_readonly_at(path, std::sync::Arc::new(crate::db::LocalKeychain::new()))
                .map_err(|error| error.to_string())?;
        schema_version_from_db(&db)
    }
}

#[derive(Debug, Parser)]
#[command(
    name = "release-gate",
    about = "Run the v1.4.0 Golden Daily Loop release gate",
    disable_help_subcommand = true
)]
struct CliArgs {
    #[arg(long, value_enum, default_value = "hermetic")]
    mode: GateMode,
    #[arg(long = "bundle", value_parser = normalize_bundle_name)]
    bundle_filters: Vec<String>,
    #[arg(long = "output-dir")]
    output_dir: Option<PathBuf>,
    #[arg(long = "harness-report")]
    harness_report: Option<String>,
    #[arg(long = "db")]
    db_path: Option<PathBuf>,
    #[arg(long = "manual-evidence")]
    manual_evidence: Option<PathBuf>,
    #[arg(long = "no-run-tests")]
    no_run_tests: bool,
    #[arg(
        long = "git-sha",
        help = "Optional assertion. The canonical SHA is embedded at build time; if provided, it must match the embedded SHA."
    )]
    git_sha: Option<String>,
}

pub fn run_from_args<I>(args: I) -> Result<GateOutcome, GateError>
where
    I: IntoIterator<Item = OsString>,
{
    let config = parse_cli_from(args)?;
    run_gate_with_db_reader(&config, &ActionDbManualReader)
}

pub fn parse_cli_from<I>(args: I) -> Result<GateConfig, GateError>
where
    I: IntoIterator<Item = OsString>,
{
    let cli = CliArgs::try_parse_from(strip_arg_separators(args))
        .map_err(|error| GateError::config(error.to_string()))?;

    let bundle_filters = if cli.bundle_filters.is_empty() {
        DEFAULT_MANDATORY_BUNDLES
            .iter()
            .map(|bundle| (*bundle).to_string())
            .collect()
    } else {
        cli.bundle_filters
    };
    let harness_report = cli
        .harness_report
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from);

    if cli.mode == GateMode::Manual && cli.manual_evidence.is_none() {
        return Err(GateError::config(
            "--mode manual requires --manual-evidence <path>",
        ));
    }

    Ok(GateConfig {
        mode: cli.mode,
        bundle_filters,
        mandatory_bundles: DEFAULT_MANDATORY_BUNDLES
            .iter()
            .map(|bundle| (*bundle).to_string())
            .collect(),
        tracked_bundles: DEFAULT_TRACKED_BUNDLES
            .iter()
            .map(|bundle| (*bundle).to_string())
            .collect(),
        output_dir: cli
            .output_dir
            .unwrap_or_else(GateConfig::default_output_dir),
        harness_report,
        db_path: cli.db_path,
        manual_evidence: cli.manual_evidence,
        run_tests: !cli.no_run_tests,
        git_sha: resolve_git_sha(cli.git_sha)?,
    })
}

pub fn run_gate_with_db_reader(
    config: &GateConfig,
    db_reader: &dyn ManualDbReader,
) -> Result<GateOutcome, GateError> {
    let mut evidence = match config.mode {
        GateMode::Hermetic => build_hermetic_evidence(config)?,
        GateMode::Manual => build_manual_evidence(config, db_reader)?,
    };
    let summary = render_markdown_summary(&evidence);
    evidence.summary_markdown = summary.clone();

    let evidence_json_path = config.output_dir.join("evidence.json");
    let evidence_markdown_path = config.output_dir.join("evidence.md");
    write_evidence_artifacts(&evidence, &evidence_json_path, &evidence_markdown_path)?;

    Ok(GateOutcome {
        exit_code: exit_code_for_evidence(&evidence),
        summary_markdown: summary,
        evidence_json_path,
        evidence_markdown_path,
    })
}

pub fn exit_code_for_evidence(evidence: &GateEvidenceV1) -> u8 {
    let mandatory_infra = evidence
        .suites
        .iter()
        .filter(|suite| suite.mandatory)
        .any(|suite| suite.status == GateStatus::InfraFailure)
        || evidence
            .invariants
            .iter()
            .filter(|invariant| invariant.mandatory)
            .any(|invariant| invariant.status == GateStatus::InfraFailure);
    if mandatory_infra {
        return EXIT_INFRA_FAILURE;
    }

    let mandatory_failed = evidence
        .suites
        .iter()
        .filter(|suite| suite.mandatory)
        .any(|suite| suite.status == GateStatus::Fail)
        || evidence
            .invariants
            .iter()
            .filter(|invariant| invariant.mandatory)
            .any(|invariant| invariant.status == GateStatus::Fail);
    if mandatory_failed {
        EXIT_MANDATORY_FAILURE
    } else {
        EXIT_SUCCESS
    }
}

pub fn render_markdown_summary(evidence: &GateEvidenceV1) -> String {
    let exit_code = exit_code_for_evidence(evidence);
    let status = match exit_code {
        EXIT_SUCCESS => "PASS",
        EXIT_MANDATORY_FAILURE => "FAIL",
        _ => "INFRA FAILURE",
    };
    let mandatory_passed = evidence
        .invariants
        .iter()
        .filter(|invariant| invariant.mandatory && invariant.status == GateStatus::Pass)
        .count();
    let mandatory_total = evidence
        .invariants
        .iter()
        .filter(|invariant| invariant.mandatory)
        .count();
    let mut lines = vec![
        format!("# Release Gate {status}"),
        String::new(),
        format!("- run_id: `{}`", evidence.run_id),
        format!("- mode: `{:?}`", evidence.mode).to_ascii_lowercase(),
        format!("- git_sha: `{}`", hash_prefix_or_unknown(&evidence.git_sha)),
        format!("- mandatory_invariants: `{mandatory_passed}/{mandatory_total}`"),
        format!(
            "- latency: p50=`{}` p99=`{}` samples=`{}`",
            display_latency(evidence.latency.p50_ms),
            display_latency(evidence.latency.p99_ms),
            evidence.latency.sample_count
        ),
        String::new(),
        "## Mandatory Bundles".to_string(),
    ];

    for bundle in &evidence.mandatory_bundles {
        let status = bundle_status(evidence, bundle);
        lines.push(format!("- `{bundle}`: `{status}`"));
    }

    lines.push(String::new());
    lines.push("## Mandatory Invariants".to_string());
    for invariant in evidence
        .invariants
        .iter()
        .filter(|invariant| invariant.mandatory)
    {
        let failure = invariant
            .failure_summary
            .as_ref()
            .map(|summary| format!(" redacted_summary_hash=`{}`", hash_prefix(summary)))
            .unwrap_or_default();
        lines.push(format!(
            "- `{}`: `{:?}`{}",
            invariant.id, invariant.status, failure
        ));
    }

    lines.push(String::new());
    lines.push("Security: evidence is redacted to IDs, counts, statuses, source classes, and hash prefixes.".to_string());
    lines.push(String::new());
    lines.join("\n")
}

pub fn parse_harness_report(path: &Path) -> Result<HarnessReport, GateError> {
    let contents = fs::read_to_string(path).map_err(|error| {
        GateError::infra(format!(
            "failed to read harness report {}: {error}",
            path.display()
        ))
    })?;
    serde_json::from_str(&contents).map_err(|error| {
        GateError::infra(format!(
            "failed to parse harness report {}: {error}",
            path.display()
        ))
    })
}

pub fn validate_manual_evidence_json(value: &Value) -> Result<ManualDogfoodEvidence, String> {
    let evidence: ManualDogfoodEvidence =
        serde_json::from_value(value.clone()).map_err(|error| error.to_string())?;
    validate_manual_evidence(&evidence)?;
    Ok(evidence)
}

fn build_hermetic_evidence(config: &GateConfig) -> Result<GateEvidenceV1, GateError> {
    let mut suites = Vec::new();
    let loader = BundleLoader::from_default_fixture_root();
    let binding = EvidenceBinding::for_config(config)?;
    let report_result = harness_report_for_config(config, &loader, &binding);
    let report = match report_result {
        Ok((report, source)) => {
            suites.push(SuiteResult {
                name: "harness".to_string(),
                source,
                command_or_report: default_or_configured_report_path(config)
                    .display()
                    .to_string(),
                status: GateStatus::Pass,
                mandatory: true,
                duration_ms: None,
                failures: Vec::new(),
            });
            suites.extend(bundle_suites_from_report(&report, config));
            Some(report)
        }
        Err(error) => {
            let failure = if error.to_string().contains("harness-report-stale") {
                error.to_string()
            } else {
                redacted_summary("harness_infra", &error.to_string())
            };
            suites.push(SuiteResult {
                name: "harness".to_string(),
                source: "harness".to_string(),
                command_or_report: default_or_configured_report_path(config)
                    .display()
                    .to_string(),
                status: GateStatus::InfraFailure,
                mandatory: true,
                duration_ms: None,
                failures: vec![failure],
            });
            None
        }
    };

    suites.extend(dos288_suite_results(config, &binding));

    let mut invariants = Vec::new();
    invariants.extend(bundle_invariants(report.as_ref(), config));
    invariants.push(provenance_source_coverage_invariant(
        report.as_ref(),
        config,
        &loader,
    ));
    invariants.extend(dos288_invariants(&suites));

    let latency = report
        .as_ref()
        .map(latency_from_report)
        .unwrap_or_else(|| LatencySummary {
            source: "missing_harness_report".to_string(),
            ..LatencySummary::default()
        });

    Ok(base_evidence(
        config,
        "fixture".to_string(),
        suites,
        invariants,
        None,
        latency,
    ))
}

fn build_manual_evidence(
    config: &GateConfig,
    db_reader: &dyn ManualDbReader,
) -> Result<GateEvidenceV1, GateError> {
    let db_path = config
        .db_path
        .as_ref()
        .ok_or_else(|| GateError::config("--mode manual requires --db <path>"))?;
    let db_schema_version = db_reader
        .open_readonly_schema_version(db_path)
        .map_err(|error| GateError::infra(format!("manual DB read-only open failed: {error}")))?;
    let manual_path = config
        .manual_evidence
        .as_ref()
        .ok_or_else(|| GateError::config("--mode manual requires --manual-evidence <path>"))?;
    let manual_json = fs::read_to_string(manual_path).map_err(|error| {
        GateError::infra(format!(
            "failed to read manual evidence {}: {error}",
            manual_path.display()
        ))
    })?;
    let manual_value: Value = serde_json::from_str(&manual_json).map_err(|error| {
        GateError::infra(format!(
            "failed to parse manual evidence {}: {error}",
            manual_path.display()
        ))
    })?;
    let manual = validate_manual_evidence_json(&manual_value)
        .map_err(|error| GateError::infra(format!("manual evidence invalid: {error}")))?;

    let suites = vec![SuiteResult {
        name: "manual_dogfood".to_string(),
        source: "manual_evidence".to_string(),
        command_or_report: manual_path.display().to_string(),
        status: GateStatus::Pass,
        mandatory: true,
        duration_ms: None,
        failures: Vec::new(),
    }];
    let invariants = vec![
        manual_invariant(
            "manual.twenty_meetings",
            manual.meeting_count >= 20,
            "manual",
            "meeting_count",
        ),
        manual_invariant(
            "manual.seven_day_parallel_run",
            !manual.seven_day_parallel_run_ref.as_str().trim().is_empty(),
            "manual",
            "seven_day_parallel_run_ref",
        ),
        manual_invariant(
            "manual.dos411_claim_backed_lifecycle",
            manual.dos411_claim_backed_lifecycle_green,
            "tauri",
            "dos411_claim_backed_lifecycle_green",
        ),
        manual_invariant(
            "manual.dos412_sensitivity_rendering",
            manual.dos412_sensitivity_rendering_green,
            "tauri_mcp",
            "dos412_sensitivity_rendering_green",
        ),
    ];

    Ok(base_evidence(
        config,
        db_schema_version,
        suites,
        invariants,
        Some(manual),
        LatencySummary {
            source: "manual_mode_not_measured".to_string(),
            sample_count: 0,
            p50_ms: None,
            p99_ms: None,
        },
    ))
}

fn base_evidence(
    config: &GateConfig,
    db_schema_version: String,
    suites: Vec<SuiteResult>,
    invariants: Vec<InvariantResult>,
    manual: Option<ManualDogfoodEvidence>,
    latency: LatencySummary,
) -> GateEvidenceV1 {
    GateEvidenceV1 {
        schema_version: RELEASE_GATE_SCHEMA_VERSION.to_string(),
        run_id: format!("release-gate-{}", Uuid::new_v4()),
        mode: config.mode,
        generated_at: Utc::now(),
        git_sha: config.git_sha.clone(),
        db_schema_version,
        suites,
        invariants,
        mandatory_bundles: config.mandatory_bundles.clone(),
        tracked_bundles: config.tracked_bundles.clone(),
        manual,
        latency,
        summary_markdown: String::new(),
    }
}

fn harness_report_for_config(
    config: &GateConfig,
    loader: &BundleLoader,
    binding: &EvidenceBinding,
) -> Result<(HarnessReport, String), GateError> {
    if let Some(path) = &config.harness_report {
        let report = parse_harness_report(path)?;
        validate_harness_report_binding(&report, binding)?;
        return Ok((report, "harness_report".to_string()));
    }

    if !config.run_tests {
        let path = GateConfig::default_harness_report_path();
        let report = parse_harness_report(&path)?;
        validate_harness_report_binding(&report, binding)?;
        return Ok((report, "harness_report".to_string()));
    }

    let fixture_refs = loader
        .fixtures_for_bundle_names(&config.bundle_filters)
        .map_err(|error| GateError::infra(format!("fixture discovery failed: {error}")))?;
    if fixture_refs.is_empty() {
        return Err(GateError::infra(format!(
            "no fixtures matched bundles {}",
            config.bundle_filters.join(",")
        )));
    }

    let registry = AbilityRegistry::from_inventory_checked().map_err(|violations| {
        GateError::infra(format!("ability registry invalid: {violations:?}"))
    })?;
    let deps = RunnerDeps {
        registry: Arc::new(registry),
    };
    let report_path = GateConfig::default_harness_report_path();
    let mut report = run_harness_suite(&deps, &fixture_refs, &report_path)
        .map_err(|error| GateError::infra(format!("harness run failed: {error}")))?;
    report.git_sha = binding.git_sha.clone();
    report.fixtures_hash = binding.fixtures_hash.clone();
    report.write_json(&report_path).map_err(|error| {
        GateError::infra(format!(
            "failed to bind harness report {}: {error}",
            report_path.display()
        ))
    })?;
    Ok((report, "in_process_harness".to_string()))
}

fn bundle_suites_from_report(report: &HarnessReport, config: &GateConfig) -> Vec<SuiteResult> {
    config
        .mandatory_bundles
        .iter()
        .chain(config.tracked_bundles.iter())
        .filter_map(|bundle| {
            let bundle_number = bundle_number(bundle)?;
            let run = report.bundle_coverage.bundles_run.contains(&bundle_number);
            if !run && !config.mandatory_bundles.contains(bundle) {
                return None;
            }
            let mandatory = config.mandatory_bundles.contains(bundle);
            let status = bundle_gate_status(report, bundle, mandatory);
            let runtime_ms = report
                .fixtures
                .iter()
                .filter(|fixture| fixture.bundle == Some(bundle_number))
                .map(|fixture| fixture.runtime_ms)
                .sum::<u64>();
            Some(SuiteResult {
                name: bundle.clone(),
                source: "harness".to_string(),
                command_or_report: "target/eval/harness-report.json".to_string(),
                status,
                mandatory,
                duration_ms: (runtime_ms > 0).then_some(runtime_ms),
                failures: if status == GateStatus::Pass {
                    Vec::new()
                } else {
                    vec![redacted_summary("bundle_status", bundle)]
                },
            })
        })
        .collect()
}

fn bundle_invariants(report: Option<&HarnessReport>, config: &GateConfig) -> Vec<InvariantResult> {
    [
        (
            "bundle-1.get_entity_context_parity_subject_ownership_no_bleed",
            "bundle-1",
            "get_entity_context",
        ),
        (
            "bundle-5.prepare_meeting_correction_tombstone_no_resurrection",
            "bundle-5",
            "prepare_meeting",
        ),
        (
            "bundle-13.prepare_meeting_subject_bleed_rejection",
            "bundle-13",
            "prepare_meeting",
        ),
    ]
    .into_iter()
    .map(|(id, bundle, surface)| {
        let mandatory = config
            .mandatory_bundles
            .iter()
            .any(|candidate| candidate == bundle);
        let (status, failure_summary) =
            match report.map(|report| bundle_report_status(report, bundle, mandatory)) {
                Some(GateStatus::Pass) => (GateStatus::Pass, None),
                Some(GateStatus::Fail) => (
                    GateStatus::Fail,
                    Some(redacted_summary("mandatory_bundle_failed", bundle)),
                ),
                Some(GateStatus::InfraFailure) | None => (
                    GateStatus::InfraFailure,
                    Some(redacted_summary("mandatory_bundle_missing", bundle)),
                ),
                Some(GateStatus::Skipped) => (
                    GateStatus::InfraFailure,
                    Some(redacted_summary("mandatory_bundle_skipped", bundle)),
                ),
            };
        InvariantResult {
            id: id.to_string(),
            bundle: Some(bundle.to_string()),
            surface: surface.to_string(),
            status,
            mandatory,
            evidence_ref: "target/eval/harness-report.json".to_string(),
            failure_summary,
        }
    })
    .collect()
}

fn provenance_source_coverage_invariant(
    report: Option<&HarnessReport>,
    config: &GateConfig,
    loader: &BundleLoader,
) -> InvariantResult {
    let evidence_ref = "fixture.expected_provenance".to_string();
    if report.is_none() {
        return InvariantResult {
            id: "provenance.source_coverage".to_string(),
            bundle: None,
            surface: "eval".to_string(),
            status: GateStatus::InfraFailure,
            mandatory: true,
            evidence_ref,
            failure_summary: Some(redacted_summary("provenance_missing_report", "harness")),
        };
    }

    let fixtures = match loader.fixtures_for_bundle_names(&config.bundle_filters) {
        Ok(fixtures) => fixtures,
        Err(error) => {
            return InvariantResult {
                id: "provenance.source_coverage".to_string(),
                bundle: None,
                surface: "eval".to_string(),
                status: GateStatus::InfraFailure,
                mandatory: true,
                evidence_ref,
                failure_summary: Some(redacted_summary(
                    "provenance_fixture_load",
                    &error.to_string(),
                )),
            };
        }
    };

    for fixture_ref in fixtures {
        let fixture = match crate::harness::load_fixture(&fixture_ref.fixture_dir) {
            Ok(fixture) => fixture,
            Err(error) => {
                return InvariantResult {
                    id: "provenance.source_coverage".to_string(),
                    bundle: None,
                    surface: "eval".to_string(),
                    status: GateStatus::InfraFailure,
                    mandatory: true,
                    evidence_ref,
                    failure_summary: Some(redacted_summary(
                        "provenance_fixture_load",
                        &error.to_string(),
                    )),
                };
            }
        };
        if fixture.metadata.source_lifecycle_refs.is_empty()
            || !json_contains_key(&fixture.expected.provenance, "source_asof")
                && !json_contains_key(&fixture.expected.provenance, "source_asof_reachable")
        {
            return InvariantResult {
                id: "provenance.source_coverage".to_string(),
                bundle: fixture
                    .metadata
                    .bundle
                    .map(|bundle| format!("bundle-{bundle}")),
                surface: "eval".to_string(),
                status: GateStatus::Fail,
                mandatory: true,
                evidence_ref,
                failure_summary: Some(redacted_summary(
                    "provenance_source_attribution_missing",
                    &fixture.metadata.scenario_id,
                )),
            };
        }
    }

    InvariantResult {
        id: "provenance.source_coverage".to_string(),
        bundle: None,
        surface: "eval".to_string(),
        status: GateStatus::Pass,
        mandatory: true,
        evidence_ref,
        failure_summary: None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EvidenceBinding {
    git_sha: String,
    fixtures_hash: String,
}

impl EvidenceBinding {
    fn for_config(config: &GateConfig) -> Result<Self, GateError> {
        let fixtures_hash = compute_default_fixtures_hash().map_err(|error| {
            GateError::infra(format!("failed to compute fixture tree hash: {error}"))
        })?;
        Ok(Self {
            git_sha: config.git_sha.clone(),
            fixtures_hash,
        })
    }
}

fn validate_harness_report_binding(
    report: &HarnessReport,
    binding: &EvidenceBinding,
) -> Result<(), GateError> {
    bind_evidence_to_commit(
        Some(report.git_sha.as_str()),
        Some(report.fixtures_hash.as_str()),
        binding,
    )
}

fn bind_evidence_to_commit(
    report_git_sha: Option<&str>,
    report_fixtures_hash: Option<&str>,
    binding: &EvidenceBinding,
) -> Result<(), GateError> {
    let report_git_sha = report_git_sha
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("<missing>");
    let report_fixtures_hash = report_fixtures_hash
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("<missing>");

    if report_git_sha == binding.git_sha && report_fixtures_hash == binding.fixtures_hash {
        Ok(())
    } else {
        Err(GateError::infra(format!(
            "harness-report-stale: report bound to {report_git_sha} fixtures {report_fixtures_hash} but current state is {} {}",
            binding.git_sha, binding.fixtures_hash
        )))
    }
}

fn dos288_suite_results(config: &GateConfig, binding: &EvidenceBinding) -> Vec<SuiteResult> {
    DOS288_SELECTORS
        .iter()
        .map(|selector| {
            if config.run_tests {
                run_dos288_selector(selector)
            } else {
                read_dos288_evidence(selector, &config.output_dir, binding)
            }
        })
        .collect()
}

fn run_dos288_selector(selector: &str) -> SuiteResult {
    let started = std::time::Instant::now();
    // remains an integration-test binary rather than a library module.
    // The release gate deliberately keeps only this selector as a subprocess;
    // the Golden Daily Loop harness itself runs in-process for structured data.
    let args = dos288_selector_args(selector);
    let command_or_report = dos288_selector_command(selector);
    let output = Command::new("cargo")
        .current_dir(repo_root())
        .args(&args)
        .env("CARGO_TERM_COLOR", "never")
        .output();
    match output {
        Ok(output) if output.status.success() => SuiteResult {
            name: selector.to_string(),
            source: "cargo_test_selector".to_string(),
            command_or_report,
            status: GateStatus::Pass,
            mandatory: true,
            duration_ms: Some(started.elapsed().as_millis() as u64),
            failures: Vec::new(),
        },
        Ok(output) => SuiteResult {
            name: selector.to_string(),
            source: "cargo_test_selector".to_string(),
            command_or_report,
            status: GateStatus::Fail,
            mandatory: true,
            duration_ms: Some(started.elapsed().as_millis() as u64),
            failures: vec![redacted_summary(
                "dos288_selector_failed",
                &format!("{selector}:{:?}", output.status.code()),
            )],
        },
        Err(error) => SuiteResult {
            name: selector.to_string(),
            source: "cargo_test_selector".to_string(),
            command_or_report,
            status: GateStatus::InfraFailure,
            mandatory: true,
            duration_ms: Some(started.elapsed().as_millis() as u64),
            failures: vec![redacted_summary(
                "dos288_selector_infra",
                &error.to_string(),
            )],
        },
    }
}

fn dos288_selector_args(selector: &str) -> Vec<String> {
    [
        "test",
        "--manifest-path",
        "src-tauri/Cargo.toml",
        "--no-default-features",
        "--features",
        "release-gate",
        "--test",
        selector,
        "--",
        "--nocapture",
        "--test-threads=1",
    ]
    .iter()
    .map(|arg| (*arg).to_string())
    .collect()
}

fn dos288_selector_command(selector: &str) -> String {
    format!("cargo {}", dos288_selector_args(selector).join(" "))
}

fn read_dos288_evidence(
    selector: &str,
    output_dir: &Path,
    binding: &EvidenceBinding,
) -> SuiteResult {
    let path = output_dir.join(format!("{selector}.json"));
    let parsed = fs::read_to_string(&path)
        .ok()
        .and_then(|contents| serde_json::from_str::<Value>(&contents).ok());
    let binding_error = parsed.as_ref().and_then(|value| {
        bind_evidence_to_commit(
            value.get("git_sha").and_then(Value::as_str),
            value.get("fixtures_hash").and_then(Value::as_str),
            binding,
        )
        .err()
    });
    let status = if binding_error.is_some() {
        GateStatus::InfraFailure
    } else {
        parsed
            .as_ref()
            .and_then(|value| {
                value
                    .get("status")
                    .and_then(Value::as_str)
                    .or_else(|| value.get("result").and_then(Value::as_str))
                    .map(|status| match status {
                        "pass" | "passed" | "green" => GateStatus::Pass,
                        "fail" | "failed" | "red" => GateStatus::Fail,
                        _ => GateStatus::InfraFailure,
                    })
            })
            .unwrap_or(GateStatus::InfraFailure)
    };
    SuiteResult {
        name: selector.to_string(),
        source: "dos288_evidence_file".to_string(),
        command_or_report: path.display().to_string(),
        status,
        mandatory: true,
        duration_ms: None,
        failures: match (status, binding_error) {
            (GateStatus::Pass, _) => Vec::new(),
            (_, Some(error)) => vec![error.to_string()],
            _ => vec![redacted_summary(
                "dos288_evidence_missing_or_failed",
                selector,
            )],
        },
    }
}

fn dos288_invariants(suites: &[SuiteResult]) -> Vec<InvariantResult> {
    DOS288_SELECTORS
        .iter()
        .map(|selector| {
            let status = suites
                .iter()
                .find(|suite| suite.name == *selector)
                .map(|suite| suite.status)
                .unwrap_or(GateStatus::InfraFailure);
            InvariantResult {
                id: selector.to_string(),
                bundle: None,
                surface: "provenance_ownership".to_string(),
                status,
                mandatory: true,
                evidence_ref: selector.to_string(),
                failure_summary: (status != GateStatus::Pass)
                    .then(|| redacted_summary("dos288_required_green", selector)),
            }
        })
        .collect()
}

fn manual_invariant(id: &str, passed: bool, surface: &str, evidence_ref: &str) -> InvariantResult {
    InvariantResult {
        id: id.to_string(),
        bundle: None,
        surface: surface.to_string(),
        status: if passed {
            GateStatus::Pass
        } else {
            GateStatus::Fail
        },
        mandatory: true,
        evidence_ref: evidence_ref.to_string(),
        failure_summary: (!passed).then(|| redacted_summary("manual_invariant_failed", id)),
    }
}

fn latency_from_report(report: &HarnessReport) -> LatencySummary {
    let mut runtimes = report
        .fixtures
        .iter()
        .map(|fixture| fixture.runtime_ms)
        .collect::<Vec<_>>();
    runtimes.sort_unstable();
    LatencySummary {
        source: "harness_report_fixture_runtime_ms".to_string(),
        sample_count: runtimes.len() as u32,
        p50_ms: percentile_nearest_rank(&runtimes, 0.50),
        p99_ms: percentile_nearest_rank(&runtimes, 0.99),
    }
}

fn percentile_nearest_rank(sorted_values: &[u64], percentile: f64) -> Option<u64> {
    if sorted_values.is_empty() {
        return None;
    }
    let rank = (percentile * sorted_values.len() as f64).ceil() as usize;
    let index = rank.saturating_sub(1).min(sorted_values.len() - 1);
    Some(sorted_values[index])
}

fn validate_manual_evidence(evidence: &ManualDogfoodEvidence) -> Result<(), String> {
    if evidence.meeting_count < 20 {
        return Err("meeting_count must be at least 20".to_string());
    }
    if evidence.date_range.start.trim().is_empty() || evidence.date_range.end.trim().is_empty() {
        return Err("date_range.start and date_range.end are required".to_string());
    }
    if evidence.operator.as_str().trim().is_empty() {
        return Err("operator must be a hash-shaped operator identifier".to_string());
    }
    if evidence.redaction_level != ManualRedactionLevel::Hash {
        return Err("redaction_level must be hash".to_string());
    }
    if evidence
        .seven_day_parallel_run_ref
        .as_str()
        .trim()
        .is_empty()
    {
        return Err("seven_day_parallel_run_ref must be hash-shaped".to_string());
    }
    if evidence.attached_artifacts.is_empty() {
        return Err("at least one attached artifact ref is required".to_string());
    }
    if !evidence.dos411_claim_backed_lifecycle_green {
        return Err("DOS-411 claim-backed lifecycle evidence must be green".to_string());
    }
    if !evidence.dos412_sensitivity_rendering_green {
        return Err("DOS-412 sensitivity rendering evidence must be green".to_string());
    }
    for artifact in &evidence.attached_artifacts {
        if artifact.hash_prefix.len() < 8
            || artifact.hash_prefix.len() > 16
            || !artifact
                .hash_prefix
                .chars()
                .all(|ch| ch.is_ascii_hexdigit())
        {
            return Err(
                "manual artifacts must use controlled source classes and 8-16 char hash prefixes"
                    .to_string(),
            );
        }
    }
    Ok(())
}

fn schema_version_from_db(db: &ActionDb) -> Result<String, String> {
    let user_version = db
        .conn_ref()
        .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
        .map_err(|error| error.to_string())?;
    Ok(format!("sqlite_user_version:{user_version}"))
}

fn write_evidence_artifacts(
    evidence: &GateEvidenceV1,
    json_path: &Path,
    markdown_path: &Path,
) -> Result<(), GateError> {
    if let Some(parent) = json_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            GateError::infra(format!(
                "failed to create evidence directory {}: {error}",
                parent.display()
            ))
        })?;
    }
    let json = serde_json::to_string_pretty(evidence)
        .map_err(|error| GateError::infra(format!("failed to serialize evidence: {error}")))?;
    fs::write(json_path, json).map_err(|error| {
        GateError::infra(format!(
            "failed to write evidence JSON {}: {error}",
            json_path.display()
        ))
    })?;
    fs::write(markdown_path, &evidence.summary_markdown).map_err(|error| {
        GateError::infra(format!(
            "failed to write evidence markdown {}: {error}",
            markdown_path.display()
        ))
    })?;
    Ok(())
}

fn strip_arg_separators<I>(args: I) -> Vec<OsString>
where
    I: IntoIterator<Item = OsString>,
{
    args.into_iter()
        .enumerate()
        .filter_map(|(index, arg)| {
            if index > 0 && arg == "--" {
                None
            } else {
                Some(arg)
            }
        })
        .collect()
}

fn normalize_bundle_name(value: &str) -> Result<String, GateError> {
    if value.contains('_') {
        return Err(GateError::config(
            "bundle names must use hyphens, for example bundle-1",
        ));
    }
    let Some(number) = value.strip_prefix("bundle-") else {
        return Err(GateError::config(format!(
            "bundle name must look like bundle-N; got `{value}`"
        )));
    };
    let parsed = number
        .parse::<u32>()
        .map_err(|_| GateError::config(format!("bundle number must be numeric; got `{value}`")))?;
    if !(1..=13).contains(&parsed) {
        return Err(GateError::config(format!(
            "bundle number must be in 1..=13; got `{value}`"
        )));
    }
    Ok(format!("bundle-{parsed}"))
}

fn bundle_number(value: &str) -> Option<u32> {
    value.strip_prefix("bundle-")?.parse().ok()
}

fn bundle_report_status(report: &HarnessReport, bundle: &str, mandatory: bool) -> GateStatus {
    bundle_gate_status(report, bundle, mandatory)
}

fn bundle_gate_status(report: &HarnessReport, bundle: &str, mandatory: bool) -> GateStatus {
    let Some(bundle_number) = bundle_number(bundle) else {
        return GateStatus::InfraFailure;
    };
    let summaries = report
        .fixtures
        .iter()
        .filter(|fixture| fixture.bundle == Some(bundle_number))
        .collect::<Vec<_>>();
    if summaries.is_empty() {
        return GateStatus::InfraFailure;
    }
    if summaries.iter().all(|summary| summary.passed) {
        return GateStatus::Pass;
    }

    let failed_summaries = summaries
        .iter()
        .filter(|summary| !summary.passed)
        .collect::<Vec<_>>();
    if mandatory && !failed_summaries.is_empty() {
        return GateStatus::Fail;
    }
    if failed_summaries.iter().all(|summary| {
        matches!(
            summary.regression.as_ref().map(|(_, severity)| severity),
            Some(Severity::FailSoft)
        )
    }) {
        return GateStatus::Pass;
    }
    GateStatus::Fail
}

fn default_or_configured_report_path(config: &GateConfig) -> PathBuf {
    config
        .harness_report
        .clone()
        .unwrap_or_else(GateConfig::default_harness_report_path)
}

fn resolve_git_sha(cli_value: Option<String>) -> Result<String, GateError> {
    resolve_git_sha_from_build(cli_value.as_deref(), Some(RELEASE_GATE_BUILD_GIT_SHA))
}

fn resolve_git_sha_from_build(
    cli_value: Option<&str>,
    build_sha: Option<&str>,
) -> Result<String, GateError> {
    let embedded = build_sha
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown");
    if embedded == "unknown" {
        return Err(GateError::infra(
            "gate-built-without-sha: rebuild release-gate with DAILYOS_BUILD_SHA env or in a git checkout",
        ));
    }

    if let Some(cli_value) = cli_value.map(str::trim).filter(|value| !value.is_empty()) {
        if cli_value != embedded {
            return Err(GateError::infra(format!(
                "gate-binary-rebuilt-required: binary embeds SHA {embedded} but CLI requested {cli_value}; rebuild release-gate from current HEAD.",
            )));
        }
    }

    Ok(embedded.to_string())
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn repo_root() -> PathBuf {
    manifest_dir()
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(manifest_dir)
}

fn bundle_status(evidence: &GateEvidenceV1, bundle: &str) -> &'static str {
    evidence
        .invariants
        .iter()
        .find(|invariant| invariant.bundle.as_deref() == Some(bundle))
        .map(|invariant| match invariant.status {
            GateStatus::Pass => "pass",
            GateStatus::Fail => "fail",
            GateStatus::InfraFailure => "infra_failure",
            GateStatus::Skipped => "skipped",
        })
        .unwrap_or("missing")
}

fn display_latency(value: Option<u64>) -> String {
    value
        .map(|value| format!("{value}ms"))
        .unwrap_or_else(|| "n/a".to_string())
}

fn redacted_summary(label: &str, detail: &str) -> String {
    format!("{label}:{}", hash_prefix(detail))
}

fn hash_prefix(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    hex::encode(hasher.finalize())
        .chars()
        .take(12)
        .collect::<String>()
}

fn hash_prefix_or_unknown(value: &str) -> String {
    if value == "unknown" {
        value.to_string()
    } else {
        value.chars().take(12).collect()
    }
}

fn json_contains_key(value: &Value, needle: &str) -> bool {
    match value {
        Value::Object(object) => object
            .iter()
            .any(|(key, value)| key == needle || json_contains_key(value, needle)),
        Value::Array(values) => values.iter().any(|value| json_contains_key(value, needle)),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn release_gate_cli_defaults_to_mandatory_bundle_set() {
        let config = parse_cli_from(["release-gate"].map(OsString::from)).unwrap();

        assert_eq!(
            config.bundle_filters,
            DEFAULT_MANDATORY_BUNDLES
                .iter()
                .map(|bundle| (*bundle).to_string())
                .collect::<Vec<_>>()
        );
        assert_eq!(config.mode, GateMode::Hermetic);
    }

    #[test]
    fn release_gate_cli_rejects_manual_without_evidence() {
        let error =
            parse_cli_from(["release-gate", "--mode", "manual"].map(OsString::from)).unwrap_err();

        assert_eq!(error.exit_code(), EXIT_INFRA_FAILURE);
        assert!(error.to_string().contains("--manual-evidence"));
    }

    #[test]
    fn release_gate_cli_accepts_pnpm_separator() {
        let config =
            parse_cli_from(["release-gate", "--", "--mode", "hermetic"].map(OsString::from))
                .unwrap();

        assert_eq!(config.mode, GateMode::Hermetic);
    }

    #[test]
    fn release_gate_rejects_mismatched_cli_sha() {
        let error =
            parse_cli_from(["release-gate", "--git-sha", "zzz"].map(OsString::from)).unwrap_err();

        assert_eq!(error.exit_code(), EXIT_INFRA_FAILURE);
        assert!(error.to_string().contains("gate-binary-rebuilt-required"));
        assert!(error
            .to_string()
            .contains("rebuild release-gate from current HEAD"));
    }

    #[test]
    fn release_gate_rejects_unknown_build_sha() {
        let error = resolve_git_sha_from_build(None, Some("unknown")).unwrap_err();

        assert_eq!(error.exit_code(), EXIT_INFRA_FAILURE);
        assert!(error.to_string().contains("gate-built-without-sha"));
    }

    #[test]
    fn release_gate_accepts_matching_cli_sha_assertion() {
        let resolved = resolve_git_sha_from_build(Some("abc123"), Some("abc123")).unwrap();

        assert_eq!(resolved, "abc123");
    }

    #[test]
    fn release_gate_dos288_selector_command_uses_release_gate_feature() {
        assert_eq!(
            dos288_selector_command("dos288_bleed_detection_test"),
            "cargo test --manifest-path src-tauri/Cargo.toml --no-default-features --features release-gate --test dos288_bleed_detection_test -- --nocapture --test-threads=1"
        );
    }

    #[test]
    fn release_gate_evidence_schema_v1_roundtrips() {
        let evidence = sample_evidence(vec![], vec![]);

        let encoded = serde_json::to_string(&evidence).unwrap();
        let decoded: GateEvidenceV1 = serde_json::from_str(&encoded).unwrap();

        assert_eq!(decoded.schema_version, RELEASE_GATE_SCHEMA_VERSION);
        assert_eq!(decoded, evidence);
    }

    #[test]
    fn release_gate_markdown_summary_redacts_raw_claim_text() {
        let mut evidence = sample_evidence(
            vec![],
            vec![InvariantResult {
                id: "bundle-1.raw-text-check".to_string(),
                bundle: Some("bundle-1".to_string()),
                surface: "get_entity_context".to_string(),
                status: GateStatus::Fail,
                mandatory: true,
                evidence_ref: "fixture".to_string(),
                failure_summary: Some(
                    "Target Example owns the renewal plan raw claim text".to_string(),
                ),
            }],
        );

        let summary = render_markdown_summary(&evidence);
        evidence.summary_markdown = summary.clone();

        assert!(!summary.contains("Target Example owns the renewal plan"));
        assert!(summary.contains("redacted_summary_hash"));
    }

    #[test]
    fn release_gate_tracked_bundle_failure_non_blocking() {
        let evidence = sample_evidence(
            vec![SuiteResult {
                name: "bundle-2".to_string(),
                source: "harness".to_string(),
                command_or_report: "target/eval/harness-report.json".to_string(),
                status: GateStatus::Fail,
                mandatory: false,
                duration_ms: Some(5),
                failures: vec![redacted_summary("tracked_failed", "bundle-2")],
            }],
            vec![],
        );

        assert_eq!(exit_code_for_evidence(&evidence), EXIT_SUCCESS);
    }

    #[test]
    fn release_gate_mandatory_failure_exit_one() {
        let evidence = sample_evidence(
            vec![],
            vec![InvariantResult {
                id: "bundle-5.prepare_meeting_correction_tombstone_no_resurrection".to_string(),
                bundle: Some("bundle-5".to_string()),
                surface: "prepare_meeting".to_string(),
                status: GateStatus::Fail,
                mandatory: true,
                evidence_ref: "target/eval/harness-report.json".to_string(),
                failure_summary: Some(redacted_summary("failed", "bundle-5")),
            }],
        );

        assert_eq!(exit_code_for_evidence(&evidence), EXIT_MANDATORY_FAILURE);
    }

    #[test]
    fn release_gate_infra_failure_exit_two() {
        let evidence = sample_evidence(
            vec![SuiteResult {
                name: "harness".to_string(),
                source: "harness".to_string(),
                command_or_report: "target/eval/harness-report.json".to_string(),
                status: GateStatus::InfraFailure,
                mandatory: true,
                duration_ms: None,
                failures: vec![redacted_summary("missing", "harness")],
            }],
            vec![],
        );

        assert_eq!(exit_code_for_evidence(&evidence), EXIT_INFRA_FAILURE);
    }

    #[test]
    fn release_gate_manual_db_uses_readonly_open() {
        struct MockReader(std::sync::Mutex<Vec<PathBuf>>);
        impl ManualDbReader for MockReader {
            fn open_readonly_schema_version(&self, path: &Path) -> Result<String, String> {
                self.0.lock().unwrap().push(path.to_path_buf());
                Ok("mock_schema:1".to_string())
            }
        }

        let temp = tempdir().unwrap();
        let manual_path = temp.path().join("manual.json");
        fs::write(
            &manual_path,
            serde_json::to_string(&valid_manual_evidence()).unwrap(),
        )
        .unwrap();
        let config = GateConfig {
            mode: GateMode::Manual,
            bundle_filters: Vec::new(),
            mandatory_bundles: DEFAULT_MANDATORY_BUNDLES
                .iter()
                .map(|bundle| (*bundle).to_string())
                .collect(),
            tracked_bundles: DEFAULT_TRACKED_BUNDLES
                .iter()
                .map(|bundle| (*bundle).to_string())
                .collect(),
            output_dir: temp.path().join("out"),
            harness_report: None,
            db_path: Some(temp.path().join("dailyos-dev.db")),
            manual_evidence: Some(manual_path),
            run_tests: false,
            git_sha: "abc123".to_string(),
        };
        let reader = MockReader(std::sync::Mutex::new(Vec::new()));

        let outcome = run_gate_with_db_reader(&config, &reader).unwrap();

        assert_eq!(outcome.exit_code, EXIT_SUCCESS);
        assert_eq!(
            reader.0.lock().unwrap().as_slice(),
            &[temp.path().join("dailyos-dev.db")]
        );
    }

    fn sample_evidence(
        suites: Vec<SuiteResult>,
        mut invariants: Vec<InvariantResult>,
    ) -> GateEvidenceV1 {
        if invariants.is_empty() {
            invariants.push(InvariantResult {
                id: "bundle-1.get_entity_context_parity_subject_ownership_no_bleed".to_string(),
                bundle: Some("bundle-1".to_string()),
                surface: "get_entity_context".to_string(),
                status: GateStatus::Pass,
                mandatory: true,
                evidence_ref: "target/eval/harness-report.json".to_string(),
                failure_summary: None,
            });
        }
        GateEvidenceV1 {
            schema_version: RELEASE_GATE_SCHEMA_VERSION.to_string(),
            run_id: "release-gate-test".to_string(),
            mode: GateMode::Hermetic,
            generated_at: DateTime::parse_from_rfc3339("2026-05-07T12:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            git_sha: "unknown".to_string(),
            db_schema_version: "fixture".to_string(),
            suites,
            invariants,
            mandatory_bundles: DEFAULT_MANDATORY_BUNDLES
                .iter()
                .map(|bundle| (*bundle).to_string())
                .collect(),
            tracked_bundles: DEFAULT_TRACKED_BUNDLES
                .iter()
                .map(|bundle| (*bundle).to_string())
                .collect(),
            manual: None,
            latency: LatencySummary {
                source: "unit".to_string(),
                sample_count: 3,
                p50_ms: Some(2),
                p99_ms: Some(3),
            },
            summary_markdown: String::new(),
        }
    }

    fn valid_manual_evidence() -> ManualDogfoodEvidence {
        ManualDogfoodEvidence {
            meeting_count: 20,
            date_range: ManualDateRange {
                start: "2026-05-01".to_string(),
                end: "2026-05-07".to_string(),
            },
            operator: HashOnly::new("abcdef1234567890").unwrap(),
            redaction_level: ManualRedactionLevel::Hash,
            seven_day_parallel_run_ref: HashOnly::new("1234567890abcdef").unwrap(),
            attached_artifacts: vec![ManualArtifactRef {
                artifact_id: HashOnly::new("fedcba0987654321").unwrap(),
                source_class: ManualSourceClass::ManualSummary,
                hash_prefix: "abcdef123456".to_string(),
            }],
            dos411_claim_backed_lifecycle_green: true,
            dos412_sensitivity_rendering_green: true,
        }
    }

    #[test]
    fn manual_validation_rejects_less_than_twenty_meetings() {
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

        let error = validate_manual_evidence_json(&value).unwrap_err();

        assert!(error.contains("meeting_count"));
    }

    #[test]
    fn manual_evidence_rejects_raw_operator_at_deserialization() {
        let value = json!({
            "meeting_count": 20,
            "date_range": { "start": "2026-05-01", "end": "2026-05-07" },
            "operator": "Ada Lovelace",
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

        let error = validate_manual_evidence_json(&value).unwrap_err();

        assert!(error.contains("^[a-fA-F0-9]{8,64}$"));
    }

    #[test]
    fn manual_evidence_rejects_raw_artifact_id_at_deserialization() {
        let value = json!({
            "meeting_count": 20,
            "date_range": { "start": "2026-05-01", "end": "2026-05-07" },
            "operator": "abcdef1234567890",
            "redaction_level": "hash",
            "seven_day_parallel_run_ref": "1234567890abcdef",
            "attached_artifacts": [{
                "artifact_id": "customer-call-recording",
                "source_class": "manual_summary",
                "hash_prefix": "abcdef123456"
            }],
            "dos411_claim_backed_lifecycle_green": true,
            "dos412_sensitivity_rendering_green": true
        });

        let error = validate_manual_evidence_json(&value).unwrap_err();

        assert!(error.contains("^[a-fA-F0-9]{8,64}$"));
    }
}
