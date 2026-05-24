//! Workspace ingestion dependency wiring.

use std::path::PathBuf;

use super::contracts::NullSignalEmitter;
use super::extract::WorkspaceExtractor;
use super::pipeline::IngestPipeline;

pub fn build_pipeline(workspace_root: PathBuf) -> IngestPipeline {
    IngestPipeline::new(
        Box::new(WorkspaceExtractor),
        Box::new(NullSignalEmitter),
        super::pipeline::DEFAULT_MAX_FILE_BYTES,
        "workspace-extractor-v1",
        workspace_root,
    )
}
