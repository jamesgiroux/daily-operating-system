//! Salience scoring engine.
//!
//! Computes a salience score from explicit inspectable factors
//! (trust, novelty, freshness, urgency, similarity, fit). No
//! provider / LLM call from this module — factor rationale is
//! constructed from typed `FactorRationale` values, never assembled
//! from raw strings. The CI invariant for this file is the absence
//! of any LLM client import.
