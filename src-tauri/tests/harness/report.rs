#![allow(dead_code)]

use super::classifier::RegressionEvidence;
use super::scoring::CategoryScore;
use super::types::FixtureRef;

#[derive(Debug, Clone, PartialEq)]
pub struct HarnessReport {
    pub entries: Vec<HarnessReportEntry>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HarnessReportEntry {
    pub fixture: FixtureRef,
    pub score: Option<CategoryScore>,
    pub regression: Option<RegressionEvidence>,
}
