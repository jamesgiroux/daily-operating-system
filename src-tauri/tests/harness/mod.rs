mod loader;
mod runner;
mod types;

pub mod classifier;
pub mod report;
pub mod scoring;

pub use loader::{discover_fixtures, load_fixture, FixtureLoadError};
pub(crate) use runner::prepare_fixture_for_run;
#[allow(unused_imports)]
pub use runner::RunResult;
pub use runner::{run_fixture, RunError, RunnerDeps};
#[allow(unused_imports)]
pub use scoring::{
    canonical_json_eq, diff_internal_provenance, diff_rendered_provenance, CategoryScorer, Diff,
    DiffKind, MaintenanceScorer, PublishScorer, ReadScorer, ScoreResult, TransformScorer,
};
pub use types::*;
