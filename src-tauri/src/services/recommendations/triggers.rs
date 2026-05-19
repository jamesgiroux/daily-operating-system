//! Trigger policy.
//!
//! Consumes upstream signals (claim changes, signal-bus events) and
//! emits `SalienceCandidateRefreshTriggered` when a candidate's
//! salience must be re-scored. Deterministic — no provider / LLM
//! call from this module.
