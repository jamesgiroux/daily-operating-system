#![allow(dead_code)]

use super::classifier::RegressionEvidence;
use super::scoring::ScoreResult;
use super::types::FixtureRef;

#[derive(Debug, Clone, PartialEq)]
pub struct HarnessReport {
    pub entries: Vec<HarnessReportEntry>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HarnessReportEntry {
    pub fixture: FixtureRef,
    pub score: Option<ScoreResult>,
    pub regression: Option<RegressionEvidence>,
}
