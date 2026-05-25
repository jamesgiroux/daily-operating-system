//! Workspace ingestion dependency wiring.

use std::path::PathBuf;

use super::extract::WorkspaceExtractor;
use super::pipeline::IngestPipeline;
use super::signals::WorkspaceSignalEmitter;

pub fn build_pipeline(workspace_root: PathBuf) -> IngestPipeline {
    IngestPipeline::new(
        Box::new(WorkspaceExtractor),
        Box::new(WorkspaceSignalEmitter),
        super::pipeline::DEFAULT_MAX_FILE_BYTES,
        "workspace-extractor-v1",
        workspace_root,
    )
}
