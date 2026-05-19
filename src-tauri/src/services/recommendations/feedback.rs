//! Feedback loop.
//!
//! Wires user feedback (accepted / dismissed / not-useful /
//! too-noisy / converted) on a `Recommendation` claim into the
//! salience-factor weight table, adjusting future ranking on the
//! same fixture entity. Extends the existing semantic feedback
//! substrate; does not introduce a parallel feedback path.
