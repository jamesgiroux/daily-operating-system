//! Workspace ingestion service — single mutation boundary for all file-derived
//! facts entering the DailyOS claim graph (v1.4.5 W1-A).
//!
//! All `pub mod` declarations are listed one-per-line in alphabetical order. The
//! shape is enforced by `tests/mod_rs_shape.rs`; subsequent v1.4.5 lanes fill
//! their owned placeholder file but never edit this `mod.rs` and never create
//! a new submodule file.

pub mod contracts;
pub mod extract;
pub mod graph;
pub mod lifecycle;
pub mod link;
pub mod pipeline;
pub mod registry;
pub mod runs;
pub mod signals;
pub mod wiring;
