//! Surfacing policy.
//!
//! Decides which scored candidates surface to the user (tiered),
//! which defer, which suppress, and emits the
//! `SurfacingDecisionMade` audit signal for each decision. Enforces
//! the daily surfacing budget. No provider / LLM call from this
//! module.
