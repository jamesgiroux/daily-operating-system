//! Pipeline wiring — DOS-466 (W2-A) creates the `build_pipeline()` constructor
//! that takes `contracts::NullExtractor` + `contracts::NullSignalEmitter` defaults.
//! DOS-470 (W3-A) and DOS-471 (W3-B) each patch a single argument position to
//! swap in `WorkspaceExtractor` and `WorkspaceSignalEmitter` respectively.
//!
//! The dependency-injection seam lets W3-A and W3-B land without modifying
//! `pipeline.rs` (W2-A's exclusive) or any earlier-wave file.
//!
//! W1-A pre-creates this placeholder so no later lane needs to create a new
//! file or edit `mod.rs`.
