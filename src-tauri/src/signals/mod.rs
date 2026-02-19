//! Universal signal bus for intelligence fusion (I306 / ADR-0080 Phase 2).
//!
//! Every data source emits typed, weighted, time-decaying signals into a
//! SQLite event log. Signals are fused using weighted log-odds Bayesian
//! combination. The signal_weights table stores learned reliability via
//! Beta distributions (populated by I307 Thompson Sampling).
//!
//! I308 adds cross-entity propagation: when a signal is emitted, propagation
//! rules derive new signals on related entities (e.g., person title_change →
//! account stakeholder_change).

pub mod bus;
pub mod cadence;
pub mod callouts;
pub mod decay;
pub mod email_bridge;
pub mod email_context;
pub mod event_trigger;
pub mod feedback;
pub mod fusion;
pub mod invalidation;
pub mod patterns;
pub mod post_meeting;
pub mod propagation;
pub mod relevance;
pub mod rules;
pub mod sampling;
