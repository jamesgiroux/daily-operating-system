//! Workspace source registry — DOS-464 (W1-B) fills this with the path-validating
//! `WorkspaceSourceRegistry::open_validated(path) -> Result<(File, contracts::FileIdentity),
//! contracts::RejectionReason>` API and the `WorkspaceCategoryRegistry::validate(category,
//! entity_type)` boundary that complements `contracts::WorkspaceCategory::from_slug`.
//!
//! W1-A pre-creates this placeholder so no later lane needs to create a new
//! file or edit `mod.rs`.
