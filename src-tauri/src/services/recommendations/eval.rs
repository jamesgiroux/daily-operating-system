//! Evaluation harness for the salience subsystem.
//!
//! Fixture-driven assertions over high-signal, low-signal, stale, noisy,
//! missed-important, and user-corrected cases. The release gate consumes this
//! deterministic summary rather than re-deriving pass/fail rules in scripts.

use serde::{Deserialize, Serialize};

use super::contracts::{SalienceFactorKind, SalienceScore};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SalienceEvalExpectation {
    pub min_total: Option<f64>,
    pub max_total: Option<f64>,
    pub required_primary_factor: Option<SalienceFactorKind>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SalienceEvalStatus {
    Passed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SalienceEvalReport {
    pub status: SalienceEvalStatus,
    pub total: f64,
    pub primary_factor: Option<SalienceFactorKind>,
    pub failure_reasons: Vec<String>,
}

pub fn evaluate_salience_score(
    score: &SalienceScore,
    expectation: &SalienceEvalExpectation,
) -> SalienceEvalReport {
    let total = if score.total.is_finite() {
        score.total.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let primary_factor = score
        .factors
        .iter()
        .filter_map(|factor| {
            let value = factor.value?.clamp(0.0, 1.0);
            Some((factor.kind, value * factor.weight.max(0.0)))
        })
        .max_by(|left, right| left.1.total_cmp(&right.1))
        .map(|(kind, _)| kind);

    let mut failure_reasons = Vec::new();
    if let Some(min_total) = expectation.min_total {
        if total < min_total {
            failure_reasons.push("below_min_total".to_string());
        }
    }
    if let Some(max_total) = expectation.max_total {
        if total > max_total {
            failure_reasons.push("above_max_total".to_string());
        }
    }
    if let Some(required) = expectation.required_primary_factor {
        if primary_factor != Some(required) {
            failure_reasons.push("primary_factor_mismatch".to_string());
        }
    }

    SalienceEvalReport {
        status: if failure_reasons.is_empty() {
            SalienceEvalStatus::Passed
        } else {
            SalienceEvalStatus::Failed
        },
        total,
        primary_factor,
        failure_reasons,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::recommendations::contracts::{
        FactorRationale, SalienceFactor, SalienceFactorKind,
    };

    fn factor(kind: SalienceFactorKind, value: f64, weight: f64) -> SalienceFactor {
        SalienceFactor {
            kind,
            value: Some(value),
            weight,
            rationale: FactorRationale::Freshness { decay_factor: 1.0 },
        }
    }

    #[test]
    fn salience_eval_passes_matching_fixture_expectation() {
        let score = SalienceScore {
            total: 0.82,
            factors: vec![
                factor(SalienceFactorKind::Urgency, 0.9, 0.5),
                factor(SalienceFactorKind::Trust, 0.8, 0.2),
            ],
        };

        let report = evaluate_salience_score(
            &score,
            &SalienceEvalExpectation {
                min_total: Some(0.8),
                max_total: Some(0.9),
                required_primary_factor: Some(SalienceFactorKind::Urgency),
            },
        );

        assert_eq!(report.status, SalienceEvalStatus::Passed);
        assert_eq!(report.primary_factor, Some(SalienceFactorKind::Urgency));
        assert!(report.failure_reasons.is_empty());
    }

    #[test]
    fn salience_eval_reports_all_failed_expectations() {
        let score = SalienceScore {
            total: 0.2,
            factors: vec![factor(SalienceFactorKind::Freshness, 0.2, 0.4)],
        };

        let report = evaluate_salience_score(
            &score,
            &SalienceEvalExpectation {
                min_total: Some(0.5),
                max_total: Some(0.7),
                required_primary_factor: Some(SalienceFactorKind::Urgency),
            },
        );

        assert_eq!(report.status, SalienceEvalStatus::Failed);
        assert_eq!(
            report.failure_reasons,
            vec!["below_min_total", "primary_factor_mismatch"]
        );
    }
}
