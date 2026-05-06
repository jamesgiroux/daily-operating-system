#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use super::runner::RunResult;
use super::scoring::Diff;
use super::types::EvalFixture;

pub struct RegressionClassifier;

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum RegressionClass {
    InputChange,
    PromptChange,
    CanonicalizationBug,
    ProviderDrift,
    LogicChange,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum Severity {
    /// PromptChange + ProviderDrift + LogicChange: blocks pending explicit
    /// reviewer rebaseline, not because the change is inherently wrong.
    FailSoft,
    /// CanonicalizationBug + InputChange: hard until fixed or intentionally updated.
    Hard,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ClassificationFingerprint {
    pub inputs_hash: String,
    pub state_sql_hash: String,
    pub prompt_template_version: Option<String>,
    pub canonical_prompt_hash: Option<String>,
    pub completion_text_hash: Option<String>,
}

impl RegressionClassifier {
    /// Classify per the deterministic precedence in plan §2.
    pub fn classify(
        &self,
        baseline: &ClassificationFingerprint,
        current: &ClassificationFingerprint,
        score_diffs: &[Diff],
    ) -> Option<(RegressionClass, Severity)> {
        if current.inputs_hash != baseline.inputs_hash
            || current.state_sql_hash != baseline.state_sql_hash
        {
            return Some(classification(RegressionClass::InputChange));
        }

        if current.prompt_template_version != baseline.prompt_template_version {
            return Some(classification(RegressionClass::PromptChange));
        }

        if current.canonical_prompt_hash != baseline.canonical_prompt_hash {
            return Some(classification(RegressionClass::CanonicalizationBug));
        }

        if let Some(baseline_completion_text_hash) = &baseline.completion_text_hash {
            if current.completion_text_hash.as_ref() != Some(baseline_completion_text_hash) {
                return Some(classification(RegressionClass::ProviderDrift));
            }
        }

        if !score_diffs.is_empty() {
            return Some(classification(RegressionClass::LogicChange));
        }

        None
    }
}

/// Map RegressionClass → Severity per the FailSoft/Hard policy.
pub fn severity_of(class: &RegressionClass) -> Severity {
    match class {
        RegressionClass::InputChange | RegressionClass::CanonicalizationBug => Severity::Hard,
        RegressionClass::PromptChange
        | RegressionClass::ProviderDrift
        | RegressionClass::LogicChange => Severity::FailSoft,
    }
}

/// Read fingerprint from a fixture's metadata.json + inputs.json + state.sql.
pub fn baseline_fingerprint_for_fixture(fixture: &EvalFixture) -> ClassificationFingerprint {
    ClassificationFingerprint {
        inputs_hash: hash_canonical_json(&fixture.inputs_json),
        state_sql_hash: hash_text(&fixture.state_sql),
        prompt_template_version: fixture
            .metadata
            .prompt_template_version
            .clone()
            .or_else(|| prompt_template_version_from_value(&fixture.expected.provenance)),
        canonical_prompt_hash: Some(fixture.metadata.prompt_fingerprint_baseline.clone()),
        completion_text_hash: fixture.metadata.completion_text_hash.clone(),
    }
}

/// Read fingerprint from the actual run's RunResult (provider response, applied state).
pub fn current_fingerprint_for_run(
    fixture: &EvalFixture,
    result: &RunResult,
) -> ClassificationFingerprint {
    ClassificationFingerprint {
        inputs_hash: hash_canonical_json(&fixture.inputs_json),
        state_sql_hash: hash_text(&fixture.state_sql),
        prompt_template_version: prompt_template_version_from_value(&result.actual_provenance)
            .or_else(|| diagnostic_field(&result.diagnostics, "prompt_template_version"))
            .or_else(|| fixture.metadata.prompt_template_version.clone())
            .or_else(|| prompt_template_version_from_value(&fixture.expected.provenance)),
        canonical_prompt_hash: canonical_prompt_hash_from_value(&result.actual_provenance)
            .or_else(|| diagnostic_field(&result.diagnostics, "canonical_prompt_hash"))
            .or_else(|| Some(fixture.metadata.prompt_fingerprint_baseline.clone())),
        completion_text_hash: diagnostic_field(&result.diagnostics, "completion_text_hash")
            .or_else(|| {
                diagnostic_field(&result.diagnostics, "completion_text")
                    .map(|text| hash_text(&text))
            }),
    }
}

fn classification(class: RegressionClass) -> (RegressionClass, Severity) {
    let severity = severity_of(&class);
    (class, severity)
}

fn prompt_template_version_from_value(value: &Value) -> Option<String> {
    fingerprint_field_from_value(value, "prompt_template_version")
}

fn canonical_prompt_hash_from_value(value: &Value) -> Option<String> {
    fingerprint_field_from_value(value, "canonical_prompt_hash")
}

fn fingerprint_field_from_value(value: &Value, key: &str) -> Option<String> {
    nested_string(value, &["prompt_fingerprint", key])
        .or_else(|| nested_string(value, &["provenance", "prompt_fingerprint", key]))
        .or_else(|| nested_string(value, &["value", "prompt_fingerprint", key]))
        .or_else(|| nested_string(value, &["value", "provenance", "prompt_fingerprint", key]))
        .or_else(|| string_field(value, key))
}

fn diagnostic_field(diagnostics: &[String], key: &str) -> Option<String> {
    diagnostics
        .iter()
        .find_map(|diagnostic| diagnostic_field_from_string(diagnostic, key))
}

fn diagnostic_field_from_string(diagnostic: &str, key: &str) -> Option<String> {
    serde_json::from_str::<Value>(diagnostic)
        .ok()
        .and_then(|value| nested_diagnostic_field(&value, key))
        .or_else(|| key_value_diagnostic_field(diagnostic, key))
}

fn nested_diagnostic_field(value: &Value, key: &str) -> Option<String> {
    string_field(value, key)
        .or_else(|| nested_string(value, &["prompt_fingerprint", key]))
        .or_else(|| nested_string(value, &["fingerprint", key]))
}

fn key_value_diagnostic_field(diagnostic: &str, key: &str) -> Option<String> {
    let expected_prefix = format!("{key}=");
    diagnostic
        .split_whitespace()
        .find_map(|part| part.strip_prefix(&expected_prefix))
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn nested_string(value: &Value, path: &[&str]) -> Option<String> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_str().map(str::to_string)
}

fn string_field(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_string)
}

fn hash_canonical_json(value: &Value) -> String {
    let canonical = canonicalize_json(value);
    let bytes = serde_json::to_vec(&canonical).expect("canonical JSON serializes");
    hash_bytes(&bytes)
}

fn hash_text(value: &str) -> String {
    hash_bytes(value.as_bytes())
}

fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

fn canonicalize_json(value: &Value) -> Value {
    match value {
        Value::Array(values) => Value::Array(values.iter().map(canonicalize_json).collect()),
        Value::Object(object) => {
            let mut keys = object.keys().collect::<Vec<_>>();
            keys.sort();

            let mut canonical = Map::new();
            for key in keys {
                canonical.insert(key.clone(), canonicalize_json(&object[key]));
            }
            Value::Object(canonical)
        }
        value => value.clone(),
    }
}
