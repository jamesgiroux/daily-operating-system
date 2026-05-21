//! Staged ingestion pipeline — DOS-466 (W2-A) fills this with the
//! `IngestPipeline::run(IngestRequest)` + `quarantine_source` +
//! `auto_detect_category` implementations.
//!
//! W1-A pre-creates this placeholder so no later lane needs to create a new
//! file or edit `mod.rs`.
