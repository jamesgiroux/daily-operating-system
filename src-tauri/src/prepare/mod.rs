//! Phase 1 preparation operations (ADR-0049: Eliminate Python runtime).
//!
//! Ports all Python ops/ modules and orchestrators to native Rust.
//! Calendar fetch and Gmail fetch live in google_api/ — this module covers:
//! - constants: email priority keywords, domains, work-day boundaries
//! - email_classify: 3-tier email priority classification
//! - actions: workspace markdown action parsing + SQLite dedup
//! - gaps: calendar gap analysis and focus block suggestions
//! - meeting_context: rich meeting context for prep generation
//! - orchestrate: thin orchestrators that compose the above

pub mod actions;
pub mod constants;
pub mod email_classify;
pub mod email_enrich;
pub mod entity_resolver;
pub mod gaps;
pub mod meeting_context;
pub mod orchestrate;
