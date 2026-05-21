//! Workspace ingestion dependency wiring.

use std::path::PathBuf;

use super::contracts::{NullExtractor, NullSignalEmitter};
use super::pipeline::IngestPipeline;

pub fn build_pipeline(workspace_root: PathBuf) -> IngestPipeline {
    IngestPipeline::default_with(
        Box::new(NullExtractor),
        Box::new(NullSignalEmitter),
        workspace_root,
    )
}
