//! Trust Compiler value objects and pure scoring entry points.
//!
//! This module deliberately contains no database access, wall-clock reads, or
//! signal emission types. Service code extracts deterministic inputs and passes
//! them into the pure compiler.

pub mod config;
pub mod types;

pub use config::{TrustConfig, TrustConfigError, TrustFactorWeights};
pub use types::{
    ConfidenceCaveat, ConfidenceEvidence, CrossEntityCoherenceInput, CrossEntityHit,
    CrossEntityHitKind, EntityFootprint, FactorEvidence, FreshnessContext, TargetFootprint,
    TrustBand, TrustComputation, TrustContext, TrustFactorInputs, TrustScore, UserFeedbackSignal,
};
