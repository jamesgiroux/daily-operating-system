#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegressionClass {
    ProviderDrift,
    PromptChange,
    InputChange,
    CanonicalizationBug,
    LogicChange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegressionEvidence {
    pub class: RegressionClass,
    pub reason: String,
}

pub trait RegressionClassifier {
    fn classify(&self, expected: &serde_json::Value, actual: &serde_json::Value) -> RegressionEvidence;
}
