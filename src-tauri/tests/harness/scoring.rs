#![allow(dead_code)]

use serde_json::{Map, Number, Value};

use super::runner::RunResult;
use super::types::{AbilityCategory, ExpectedArtifacts};

const FLOAT_TOLERANCE: f64 = f64::EPSILON * 1024.0;

pub trait CategoryScorer {
    fn score(&self, expected: &ExpectedArtifacts, actual: &RunResult) -> ScoreResult;
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScoreResult {
    pub category: AbilityCategory,
    pub passed: bool,
    pub diffs: Vec<Diff>,
    pub continuous_score: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Diff {
    pub kind: DiffKind,
    pub path: String,
    pub expected: Value,
    pub actual: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffKind {
    OutputMismatch,
    ProvenanceMismatch,
    StateMismatch,
    SourceWarning,
}

pub struct ReadScorer;

pub struct TransformScorer {
    pub threshold: f64,
}

pub struct MaintenanceScorer;

pub struct PublishScorer;

impl CategoryScorer for ReadScorer {
    fn score(&self, expected: &ExpectedArtifacts, actual: &RunResult) -> ScoreResult {
        let mut diffs = diff_output(&expected.output, &actual.actual_output);
        diffs.extend(diff_expected_provenance(expected, &actual.actual_provenance));
        diffs.extend(diff_expected_state(expected.state.as_ref(), actual.actual_state.as_ref()));

        ScoreResult {
            category: AbilityCategory::Read,
            passed: diffs.is_empty(),
            diffs,
            continuous_score: None,
        }
    }
}

impl CategoryScorer for TransformScorer {
    fn score(&self, expected: &ExpectedArtifacts, actual: &RunResult) -> ScoreResult {
        let mut diffs = diff_output(&expected.output, &actual.actual_output);
        diffs.extend(diff_expected_provenance(expected, &actual.actual_provenance));
        diffs.extend(diff_expected_state(expected.state.as_ref(), actual.actual_state.as_ref()));

        let continuous_score = if diffs.is_empty() { 1.0 } else { 0.0 };

        ScoreResult {
            category: AbilityCategory::Transform,
            passed: diffs.is_empty() && continuous_score >= self.threshold,
            diffs,
            continuous_score: Some(continuous_score),
        }
    }
}

impl CategoryScorer for MaintenanceScorer {
    fn score(&self, expected: &ExpectedArtifacts, actual: &RunResult) -> ScoreResult {
        let mut diffs = diff_output_field(
            &expected.output,
            &actual.actual_output,
            "planned_mutations",
        );
        diffs.extend(diff_expected_provenance(expected, &actual.actual_provenance));

        ScoreResult {
            category: AbilityCategory::Maintenance,
            passed: diffs.is_empty(),
            diffs,
            continuous_score: None,
        }
    }
}

impl CategoryScorer for PublishScorer {
    fn score(&self, expected: &ExpectedArtifacts, actual: &RunResult) -> ScoreResult {
        let mut diffs = diff_publish_output(&expected.output, &actual.actual_output);
        diffs.extend(diff_expected_provenance(expected, &actual.actual_provenance));

        ScoreResult {
            category: AbilityCategory::Publish,
            passed: diffs.is_empty(),
            diffs,
            continuous_score: None,
        }
    }
}

pub fn canonical_json_eq(left: &Value, right: &Value) -> bool {
    values_equal(left, right, "")
}

/// Diff full W3-B envelope (internal/eval surface).
pub fn diff_internal_provenance(expected: &Value, actual: &Value) -> Vec<Diff> {
    diff_values(expected, actual, "", provenance_diff_kind)
}

/// Diff rendered surface output only (MCP/Tauri bridge surface).
///
/// Known incomplete: this currently strips key patterns instead of applying
/// ADR-0108 actor-specific rendering.
// TODO: replace with ADR-0108 actor renderer when W5/W6 lands
pub fn diff_rendered_provenance(expected: &Value, actual: &Value) -> Vec<Diff> {
    let expected = strip_rendered_internal_provenance_fields(expected);
    let actual = strip_rendered_internal_provenance_fields(actual);

    diff_values(&expected, &actual, "", provenance_diff_kind)
}

fn diff_output(expected: &Value, actual: &Value) -> Vec<Diff> {
    diff_values(expected, actual, "", |_| DiffKind::OutputMismatch)
}

fn diff_output_field(expected: &Value, actual: &Value, field: &str) -> Vec<Diff> {
    let expected_field = expected.get(field).unwrap_or(&Value::Null);
    let actual_field = actual.get(field).unwrap_or(&Value::Null);

    diff_values(
        expected_field,
        actual_field,
        &pointer_path("", field),
        |_| DiffKind::OutputMismatch,
    )
}

fn diff_publish_output(expected: &Value, actual: &Value) -> Vec<Diff> {
    let field = ["outbox", "planned_publishes"]
        .iter()
        .copied()
        .find(|field| expected.get(*field).is_some())
        .or_else(|| {
            ["outbox", "planned_publishes"]
                .iter()
                .copied()
                .find(|field| actual.get(*field).is_some())
        })
        .unwrap_or("outbox");

    diff_output_field(expected, actual, field)
}

fn diff_expected_provenance(expected: &ExpectedArtifacts, actual: &Value) -> Vec<Diff> {
    if expected.expected_render_policy == "show" {
        diff_rendered_provenance(&expected.provenance, actual)
    } else {
        diff_internal_provenance(&expected.provenance, actual)
    }
}

fn diff_expected_state(expected: Option<&Value>, actual: Option<&Value>) -> Vec<Diff> {
    match (expected, actual) {
        (Some(expected), Some(actual)) => {
            diff_values(expected, actual, "", |_| DiffKind::StateMismatch)
        }
        (Some(expected), None) => vec![Diff {
            kind: DiffKind::StateMismatch,
            path: String::new(),
            expected: expected.clone(),
            actual: Value::Null,
        }],
        (None, _) => Vec::new(),
    }
}

fn diff_values(
    expected: &Value,
    actual: &Value,
    path: &str,
    diff_kind: fn(&str) -> DiffKind,
) -> Vec<Diff> {
    if values_equal(expected, actual, path) {
        return Vec::new();
    }

    match (expected, actual) {
        (Value::Object(expected_object), Value::Object(actual_object)) => {
            diff_objects(expected_object, actual_object, path, diff_kind)
        }
        (Value::Array(expected_array), Value::Array(actual_array)) => {
            diff_arrays(expected_array, actual_array, path, diff_kind)
        }
        _ => vec![Diff {
            kind: diff_kind(path),
            path: path.to_string(),
            expected: expected.clone(),
            actual: actual.clone(),
        }],
    }
}

fn diff_objects(
    expected: &Map<String, Value>,
    actual: &Map<String, Value>,
    path: &str,
    diff_kind: fn(&str) -> DiffKind,
) -> Vec<Diff> {
    let mut keys = expected.keys().chain(actual.keys()).collect::<Vec<_>>();
    keys.sort();
    keys.dedup();

    let mut diffs = Vec::new();
    for key in keys {
        let child_path = pointer_path(path, key);
        match (expected.get(key), actual.get(key)) {
            (Some(expected), Some(actual)) => {
                diffs.extend(diff_values(expected, actual, &child_path, diff_kind));
            }
            (Some(expected), None) => diffs.push(Diff {
                kind: diff_kind(&child_path),
                path: child_path,
                expected: expected.clone(),
                actual: Value::Null,
            }),
            (None, Some(actual)) => diffs.push(Diff {
                kind: diff_kind(&child_path),
                path: child_path,
                expected: Value::Null,
                actual: actual.clone(),
            }),
            (None, None) => {}
        }
    }

    diffs
}

fn diff_arrays(
    expected: &[Value],
    actual: &[Value],
    path: &str,
    diff_kind: fn(&str) -> DiffKind,
) -> Vec<Diff> {
    if is_non_significant_array(path) {
        return vec![Diff {
            kind: diff_kind(path),
            path: path.to_string(),
            expected: normalize_array_for_comparison(expected, path),
            actual: normalize_array_for_comparison(actual, path),
        }];
    }

    if expected.len() != actual.len() {
        return vec![Diff {
            kind: diff_kind(path),
            path: path.to_string(),
            expected: Value::Array(expected.to_vec()),
            actual: Value::Array(actual.to_vec()),
        }];
    }

    expected
        .iter()
        .zip(actual.iter())
        .enumerate()
        .flat_map(|(index, (expected, actual))| {
            let child_path = format!("{path}/{index}");
            diff_values(expected, actual, &child_path, diff_kind)
        })
        .collect()
}

fn values_equal(expected: &Value, actual: &Value, path: &str) -> bool {
    match (expected, actual) {
        (Value::Null, Value::Null) | (Value::Bool(_), Value::Bool(_)) => expected == actual,
        (Value::Number(expected), Value::Number(actual)) => numbers_equal(expected, actual),
        (Value::String(expected), Value::String(actual)) => expected == actual,
        (Value::Array(expected), Value::Array(actual)) => arrays_equal(expected, actual, path),
        (Value::Object(expected), Value::Object(actual)) => objects_equal(expected, actual, path),
        _ => false,
    }
}

fn objects_equal(expected: &Map<String, Value>, actual: &Map<String, Value>, path: &str) -> bool {
    if expected.len() != actual.len() {
        return false;
    }

    expected.iter().all(|(key, expected)| {
        actual
            .get(key)
            .is_some_and(|actual| values_equal(expected, actual, &pointer_path(path, key)))
    })
}

fn arrays_equal(expected: &[Value], actual: &[Value], path: &str) -> bool {
    if expected.len() != actual.len() {
        return false;
    }

    if is_non_significant_array(path) {
        return normalize_array_for_comparison(expected, path)
            == normalize_array_for_comparison(actual, path);
    }

    expected
        .iter()
        .zip(actual.iter())
        .enumerate()
        .all(|(index, (expected, actual))| {
            values_equal(expected, actual, &format!("{path}/{index}"))
        })
}

fn numbers_equal(expected: &Number, actual: &Number) -> bool {
    if expected == actual {
        return true;
    }

    if (expected.is_i64() || expected.is_u64()) && (actual.is_i64() || actual.is_u64()) {
        return false;
    }

    let Some(expected) = expected.as_f64() else {
        return false;
    };
    let Some(actual) = actual.as_f64() else {
        return false;
    };

    if !expected.is_finite() || !actual.is_finite() {
        return false;
    }

    let scale = expected.abs().max(actual.abs()).max(1.0);
    (expected - actual).abs() <= FLOAT_TOLERANCE * scale
}

fn normalize_array_for_comparison(values: &[Value], path: &str) -> Value {
    let mut normalized_values = values
        .iter()
        .enumerate()
        .map(|(index, value)| canonicalize_value(value, &format!("{path}/{index}")))
        .collect::<Vec<_>>();

    normalized_values.sort_by_key(canonical_sort_key);

    Value::Array(normalized_values)
}

fn canonicalize_value(value: &Value, path: &str) -> Value {
    match value {
        Value::Array(values) if is_non_significant_array(path) => {
            normalize_array_for_comparison(values, path)
        }
        Value::Array(values) => Value::Array(
            values
                .iter()
                .enumerate()
                .map(|(index, value)| canonicalize_value(value, &format!("{path}/{index}")))
                .collect(),
        ),
        Value::Object(object) => {
            let mut keys = object.keys().collect::<Vec<_>>();
            keys.sort();

            let mut canonical = Map::new();
            for key in keys {
                canonical.insert(
                    key.clone(),
                    canonicalize_value(&object[key], &pointer_path(path, key)),
                );
            }

            Value::Object(canonical)
        }
        other => other.clone(),
    }
}

fn canonical_sort_key(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| format!("{value:?}"))
}

fn strip_rendered_internal_provenance_fields(value: &Value) -> Value {
    match value {
        Value::Array(values) => Value::Array(
            values
                .iter()
                .map(strip_rendered_internal_provenance_fields)
                .collect(),
        ),
        Value::Object(object) => Value::Object(
            object
                .iter()
                .filter(|(key, _)| !is_rendered_internal_key(key))
                .map(|(key, value)| {
                    (
                        key.clone(),
                        strip_rendered_internal_provenance_fields(value),
                    )
                })
                .collect(),
        ),
        other => other.clone(),
    }
}

fn is_rendered_internal_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();

    key == "id"
        || key == "ids"
        || key == "children"
        || key.ends_with("_id")
        || key.ends_with("_ids")
        || key.contains("hash")
        || key.contains("seed")
}

fn provenance_diff_kind(path: &str) -> DiffKind {
    if path_has_segment(path, "warnings") || path_has_segment(path, "source_warnings") {
        DiffKind::SourceWarning
    } else {
        DiffKind::ProvenanceMismatch
    }
}

fn is_non_significant_array(path: &str) -> bool {
    path_has_segment(path, "warnings") || path_has_segment(path, "source_warnings")
}

fn path_has_segment(path: &str, segment: &str) -> bool {
    path.split('/').any(|part| part == segment)
}

fn pointer_path(parent: &str, key: &str) -> String {
    let escaped = escape_json_pointer_segment(key);
    if parent.is_empty() {
        format!("/{escaped}")
    } else {
        format!("{parent}/{escaped}")
    }
}

fn escape_json_pointer_segment(segment: &str) -> String {
    segment.replace('~', "~0").replace('/', "~1")
}
