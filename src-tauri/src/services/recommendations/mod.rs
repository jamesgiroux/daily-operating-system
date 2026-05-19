//! Recommendations and salience subsystem.
//!
//! Recommendations ride the existing claim substrate as
//! `ClaimType::Recommendation`. This module owns the recommendation
//! contracts, salience scoring engine, surfacing + trigger policies,
//! feedback wiring, cross-surface render, and the evaluation harness.
//!
//! Skeleton-only on landing; per-file ownership follows the salience
//! wave plan. The shape of `RecommendationClaim`, salience factors,
//! user-feedback state, and conversion state is pinned at the ADR
//! level (see ADR-0125 §5).

pub mod contracts;
pub mod deviation;
pub mod engagement;
pub mod eval;
pub mod feedback;
pub mod recommendation;
pub mod render;
pub mod salience;
pub mod surfacing;
pub mod triggers;
pub mod why_this_now;
