//! Frozen cross-lane contracts for the recommendations subsystem.
//!
//! Owns the `RecommendationClaim` DTO, salience-factor types,
//! `SurfacingDecision` shape, `EngagementSignal`, `FeedbackAction`,
//! and every other type consumed by more than one lane. Wire format
//! is camelCase JSON via serde; Rust/TS golden parity fixtures
//! belong here.
