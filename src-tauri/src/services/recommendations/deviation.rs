//! Deviation detection.
//!
//! Compares current recommendation candidates against learned baselines on the
//! same subject and returns typed ranking input. This module is deliberately
//! deterministic: no provider calls and no source-text interpolation.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviationBand {
    Stable,
    Notable,
    Material,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviationBaseline {
    pub mean: f64,
    pub standard_deviation: f64,
    pub sample_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviationAssessment {
    pub current: f64,
    pub baseline: DeviationBaseline,
    pub delta: f64,
    pub z_score: Option<f64>,
    pub band: DeviationBand,
}

pub fn assess_deviation(current: f64, baseline: DeviationBaseline) -> DeviationAssessment {
    let current = clamp_score(current);
    let mean = clamp_score(baseline.mean);
    let standard_deviation = if baseline.standard_deviation.is_finite() {
        baseline.standard_deviation.max(0.0)
    } else {
        0.0
    };
    let baseline = DeviationBaseline {
        mean,
        standard_deviation,
        sample_count: baseline.sample_count,
    };
    let delta = current - mean;
    let z_score = if standard_deviation > f64::EPSILON && baseline.sample_count >= 3 {
        Some(delta / standard_deviation)
    } else {
        None
    };
    let band = match z_score.map(f64::abs) {
        Some(value) if value >= 2.0 => DeviationBand::Material,
        Some(value) if value >= 1.0 => DeviationBand::Notable,
        _ if delta.abs() >= 0.35 => DeviationBand::Material,
        _ if delta.abs() >= 0.15 => DeviationBand::Notable,
        _ => DeviationBand::Stable,
    };

    DeviationAssessment {
        current,
        baseline,
        delta,
        z_score,
        band,
    }
}

fn clamp_score(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn z_score_drives_material_deviation_when_baseline_is_mature() {
        let assessment = assess_deviation(
            0.9,
            DeviationBaseline {
                mean: 0.5,
                standard_deviation: 0.1,
                sample_count: 8,
            },
        );

        assert_eq!(assessment.band, DeviationBand::Material);
        assert_eq!(assessment.z_score, Some(4.0));
    }

    #[test]
    fn absolute_delta_handles_small_or_flat_baselines() {
        let assessment = assess_deviation(
            0.7,
            DeviationBaseline {
                mean: 0.5,
                standard_deviation: 0.0,
                sample_count: 1,
            },
        );

        assert_eq!(assessment.band, DeviationBand::Notable);
        assert_eq!(assessment.z_score, None);
    }

    #[test]
    fn invalid_scores_are_clamped_before_assessment() {
        let assessment = assess_deviation(
            f64::NAN,
            DeviationBaseline {
                mean: 2.0,
                standard_deviation: f64::NAN,
                sample_count: 5,
            },
        );

        assert_eq!(assessment.current, 0.0);
        assert_eq!(assessment.baseline.mean, 1.0);
        assert_eq!(assessment.baseline.standard_deviation, 0.0);
        assert_eq!(assessment.band, DeviationBand::Material);
    }
}
