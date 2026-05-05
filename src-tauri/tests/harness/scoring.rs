#![allow(dead_code)]

use super::types::{AbilityCategory, EvalFixture};

#[derive(Debug, Clone, PartialEq)]
pub struct CategoryScore {
    pub category: AbilityCategory,
    pub passed: bool,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScoringError {
    pub message: String,
}

pub trait CategoryScorer {
    fn score(
        &self,
        fixture: &EvalFixture,
        actual_output: &serde_json::Value,
        actual_provenance: &serde_json::Value,
    ) -> Result<CategoryScore, ScoringError>;
}
