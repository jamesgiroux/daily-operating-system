mod loader;
mod runner;
mod types;

pub mod classifier;
pub mod report;
pub mod scoring;

#[allow(unused_imports)]
pub use classifier::{
    baseline_fingerprint_for_fixture, current_fingerprint_for_run, severity_of,
    ClassificationFingerprint, RegressionClass, RegressionClassifier, Severity,
};
pub use loader::{discover_fixtures, load_fixture, FixtureLoadError};
#[allow(unused_imports)]
pub use report::{BundleCoverage, CategorySummary, FixtureRunSummary, HarnessReport};
pub(crate) use runner::prepare_fixture_for_run;
#[allow(unused_imports)]
pub use runner::RunResult;
pub use runner::{run_fixture, run_harness_suite, RunError, RunnerDeps};
#[allow(unused_imports)]
pub use scoring::{
    canonical_json_eq, diff_internal_provenance, diff_rendered_provenance, CategoryScorer, Diff,
    DiffKind, MaintenanceScorer, PublishScorer, ReadScorer, ScoreResult, TransformScorer,
};
pub use types::*;
