//! Workspace signal emission — W3-B fills this with the
//! `WorkspaceSignalEmitter` impl of `contracts::SignalEmitter`. Each of the
//! five trait methods maps 1:1 to a `SignalType::WorkspaceFile*` variant added
//! to `signals/policy_registry.rs` by the same lane.
//!
//! W1-A pre-creates this placeholder so no later lane needs to create a new
//! file or edit `mod.rs`.
