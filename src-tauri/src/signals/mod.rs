//! Universal signal bus for intelligence fusion (I306 / ADR-0080 Phase 2).
//!
//! Every data source emits typed, weighted, time-decaying signals into a
//! SQLite event log. Signals are fused using weighted log-odds Bayesian
//! combination. The signal_weights table stores learned reliability via
//! Beta distributions (populated by I307 Thompson Sampling).

pub mod bus;
pub mod decay;
pub mod email_bridge;
pub mod email_context;
pub mod fusion;
