//! Weighted Bayesian signal fusion (ADR-0080).
//!
//! Computes per-signal weights using source tier + temporal decay + learned
//! reliability, then fuses using weighted log-odds combination.

use crate::db::ActionDb;

use super::bus::{self, SignalEvent};
use super::decay;

/// Compute the effective weight for a signal event.
///
/// `weight = decayed_weight(tier_weight, age, half_life) * learned_reliability`
pub fn compute_signal_weight(db: &ActionDb, event: &SignalEvent) -> f64 {
    let tier_weight = bus::source_base_weight(&event.source);
    let age_days = decay::age_days_from_now(&event.created_at);
    let decayed = decay::decayed_weight(tier_weight, age_days, event.decay_half_life_days as f64);
    let reliability = bus::get_learned_reliability(db, &event.source, &event.entity_type, &event.signal_type);
    decayed * reliability
}

/// Fuse multiple (confidence, weight) pairs using weighted log-odds combination.
///
/// For each signal:
///   log_odds_contribution = weight * ln(confidence / (1 - confidence))
/// Sum all contributions, then convert back:
///   combined = 1 / (1 + exp(-sum))
///
/// This replaces the unweighted version in entity_resolver for weighted fusion.
pub fn fuse_confidence(signals: &[(f64, f64)]) -> f64 {
    if signals.is_empty() {
        return 0.5;
    }

    if signals.len() == 1 {
        return signals[0].0;
    }

    let mut weighted_log_odds_sum: f64 = 0.0;

    for &(confidence, weight) in signals {
        let p = confidence.clamp(0.01, 0.99);
        let log_odds = (p / (1.0 - p)).ln();
        weighted_log_odds_sum += weight * log_odds;
    }

    let combined = 1.0 / (1.0 + (-weighted_log_odds_sum).exp());
    combined.clamp(0.0, 0.999)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuse_high_confidence_weighted() {
        // Two strong signals (0.8 conf, 1.0 weight) and (0.9 conf, 0.9 weight)
        let result = fuse_confidence(&[(0.8, 1.0), (0.9, 0.9)]);
        assert!(result > 0.95, "weighted compounding should exceed 0.95, got {}", result);
    }

    #[test]
    fn test_strong_dominates_weak_contradiction() {
        // Strong signal at 0.9 (weight 1.0) vs weak contradiction at 0.1 (weight 0.4)
        let result = fuse_confidence(&[(0.9, 1.0), (0.1, 0.4)]);
        assert!(
            result > 0.70 && result < 0.95,
            "strong should dominate weak contradiction, got {}",
            result
        );
    }

    #[test]
    fn test_single_signal_passthrough() {
        let result = fuse_confidence(&[(0.75, 1.0)]);
        assert!((result - 0.75).abs() < 0.001);
    }

    #[test]
    fn test_empty_returns_prior() {
        assert!((fuse_confidence(&[]) - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_equal_weights_compound() {
        // Three 0.7-confidence signals with equal weight 1.0
        let result = fuse_confidence(&[(0.7, 1.0), (0.7, 1.0), (0.7, 1.0)]);
        // log_odds(0.7) ≈ 0.847, sum = 2.541, combined ≈ 0.927
        assert!(result > 0.90, "three 0.7s should compound above 0.90, got {}", result);
    }

    #[test]
    fn test_low_weight_reduces_influence() {
        // High confidence but very low weight
        let full_weight = fuse_confidence(&[(0.5, 1.0), (0.9, 1.0)]);
        let low_weight = fuse_confidence(&[(0.5, 1.0), (0.9, 0.1)]);
        assert!(
            full_weight > low_weight,
            "low weight should reduce influence: full={}, low={}",
            full_weight,
            low_weight
        );
    }
}
