#![allow(dead_code)]

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixtureManifest {
    pub ability_name: String,
    pub category: AbilityCategory,
    pub fixtures: Vec<FixtureRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AbilityCategory {
    #[serde(rename = "read", alias = "Read")]
    Read,
    #[serde(rename = "transform", alias = "Transform")]
    Transform,
    #[serde(rename = "maintenance", alias = "Maintenance")]
    Maintenance,
    #[serde(rename = "publish", alias = "Publish")]
    Publish,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixtureRef {
    pub fixture_dir: PathBuf,
    pub labels: Vec<String>,
}

impl FixtureRef {
    pub fn has_label(&self, label: &str) -> bool {
        self.labels.iter().any(|candidate| candidate == label)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FixtureMetadata {
    pub bundle: Option<u32>,
    pub scenario_id: String,
    pub invariant: String,
    pub expected_render_policy: String,
    pub surfaces_exercised: Vec<String>,
    pub source_lifecycle_refs: Vec<String>,
    pub anonymization_cert: String,
    pub retention_policy: String,
    pub prompt_fingerprint_baseline: String,
    #[serde(default)]
    pub prompt_template_version: Option<String>,
    pub trust_factors_dominant: Vec<String>,
    pub pass_fail_definition: String,
    pub fixture_design_notes: Option<serde_json::Value>,
    pub post_action_state: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvalFixture {
    pub fixture_dir: PathBuf,
    pub metadata: FixtureMetadata,
    pub state_sql: String,
    pub inputs_json: serde_json::Value,
    pub provider_replay: serde_json::Value,
    pub external_replay: serde_json::Value,
    pub clock: DateTime<Utc>,
    pub seed: u64,
    pub expected: ExpectedArtifacts,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExpectedArtifacts {
    pub output: serde_json::Value,
    pub provenance: serde_json::Value,
    pub state: Option<serde_json::Value>,
    pub expected_render_policy: String,
}
