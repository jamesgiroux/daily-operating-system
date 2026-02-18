//! Proactive surfacing engine (I260).
//!
//! Periodically mines the database for patterns, correlations, and temporal
//! signals, then surfaces synthesized insights through the existing briefing
//! callout infrastructure.

pub mod detectors;
pub mod engine;
pub mod scanner;
