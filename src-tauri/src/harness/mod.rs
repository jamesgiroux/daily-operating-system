mod loader;
mod runner;
pub mod types;

pub mod bundle_helpers;
pub mod classifier;
pub mod report;
pub mod scoring;

pub use crate::bridges::eval::{EvalAbilityBridge, EvalAbilityDeps, EvalFixtureServices};
#[allow(unused_imports)]
pub use classifier::{
    baseline_fingerprint_for_fixture, current_fingerprint_for_run, severity_of,
    ClassificationFingerprint, RegressionClass, RegressionClassifier, Severity,
};
#[allow(unused_imports)]
pub use loader::{discover_fixtures, load_fixture, BundleLoader, FixtureLoadError};
#[allow(unused_imports)]
pub use report::{
    compute_default_fixtures_hash, compute_fixtures_hash, BundleCoverage, CategorySummary,
    FixtureRunSummary, HarnessReport,
};
#[allow(unused_imports)]
pub use runner::prepare_fixture_for_run;
#[allow(unused_imports)]
pub use runner::RunResult;
#[allow(unused_imports)]
pub use runner::{run_fixture, run_harness_suite, RunError, RunnerDeps};
#[allow(unused_imports)]
pub use scoring::{
    canonical_json_eq, diff_internal_provenance, diff_rendered_provenance, CategoryScorer, Diff,
    DiffKind, MaintenanceScorer, PublishScorer, ReadScorer, ScoreResult, TransformDimensionScores,
    TransformScorer,
};
pub use types::*;
