//! Deviation detection.
//!
//! Compares current recommendation candidates against learned
//! baselines on the same subject and surfaces deviations as
//! ranking input. May require a `deviation_baselines` table —
//! migration choice belongs to the lane that owns this file.
