mod loader;
mod types;

pub mod classifier;
pub mod report;
pub mod scoring;

pub use loader::{discover_fixtures, load_fixture, FixtureLoadError};
pub use types::*;
