//! Thompson Sampling for signal weight learning (I307 / ADR-0080 Phase 3).
//!
//! Uses Beta distribution sampling to balance exploration/exploitation when
//! learning signal source reliability from user corrections.

use rand_distr::{Beta, Distribution};

/// Sample a reliability value from Beta(alpha, beta).
///
/// Uses Thompson Sampling: each call returns a different sample, so the
/// system naturally explores sources with uncertain reliability while
/// exploiting sources with known high reliability.
pub fn sample_reliability(alpha: f64, beta: f64) -> f64 {
    // Clamp to valid Beta distribution parameters (> 0)
    let a = alpha.max(0.01);
    let b = beta.max(0.01);
    match Beta::new(a, b) {
        Ok(dist) => {
            let mut rng = rand::rng();
            dist.sample(&mut rng).clamp(0.01, 0.99)
        }
        Err(_) => mean_reliability(alpha, beta),
    }
}

/// Deterministic reliability estimate: Beta distribution mean.
///
/// Use this when you need a stable value (e.g. for display or logging).
pub fn mean_reliability(alpha: f64, beta: f64) -> f64 {
    let a = alpha.max(0.01);
    let b = beta.max(0.01);
    a / (a + b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_reliability_high_alpha() {
        // alpha=28, beta=2 → mean ≈ 0.93, samples should cluster near there
        let samples: Vec<f64> = (0..100).map(|_| sample_reliability(28.0, 2.0)).collect();
        let mean: f64 = samples.iter().sum::<f64>() / samples.len() as f64;
        assert!(mean > 0.85, "mean of samples with alpha=28, beta=2 should be > 0.85, got {}", mean);
        assert!(mean < 0.99, "mean should be < 0.99, got {}", mean);
    }

    #[test]
    fn test_sample_reliability_low_alpha() {
        // alpha=3, beta=7 → mean ≈ 0.30, samples should cluster near there
        let samples: Vec<f64> = (0..100).map(|_| sample_reliability(3.0, 7.0)).collect();
        let mean: f64 = samples.iter().sum::<f64>() / samples.len() as f64;
        assert!(mean > 0.15, "mean of samples with alpha=3, beta=7 should be > 0.15, got {}", mean);
        assert!(mean < 0.45, "mean should be < 0.45, got {}", mean);
    }

    #[test]
    fn test_mean_reliability() {
        assert!((mean_reliability(28.0, 2.0) - 0.9333).abs() < 0.01);
        assert!((mean_reliability(3.0, 7.0) - 0.30).abs() < 0.01);
        assert!((mean_reliability(1.0, 1.0) - 0.50).abs() < 0.01);
    }

    #[test]
    fn test_sample_reliability_edge_cases() {
        // Very small parameters should not panic
        let _ = sample_reliability(0.001, 0.001);
        let _ = sample_reliability(0.0, 0.0); // clamped to 0.01
        let _ = sample_reliability(100.0, 100.0);
    }
}
