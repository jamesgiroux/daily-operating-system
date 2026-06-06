use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum McpRuntimeSourceKind {
    AppBundle,
    RepoBinaries,
}

impl McpRuntimeSourceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AppBundle => "app_bundle",
            Self::RepoBinaries => "repo_binaries",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpSidecarProvenance {
    pub name: String,
    pub filename: String,
    pub build_sha: String,
    pub sha256: String,
    pub stub: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpBundleProvenance {
    pub schema_version: u32,
    pub guard_epoch: String,
    pub target_triple: String,
    pub app_build_sha: String,
    pub generated_at: String,
    pub stub: bool,
    pub sidecars: Vec<McpSidecarProvenance>,
}

impl McpBundleProvenance {
    pub fn sidecar(&self, name: &str) -> Option<&McpSidecarProvenance> {
        self.sidecars.iter().find(|sidecar| sidecar.name == name)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpLauncherManifest {
    pub schema_version: u32,
    pub guard_epoch: String,
    pub app_build_sha: String,
    pub launcher_build_sha: String,
    pub sidecar_build_sha: String,
    pub source_kind: McpRuntimeSourceKind,
    pub bundle_provenance_path: PathBuf,
    pub launcher_path: PathBuf,
    pub sidecar_path: PathBuf,
    pub expected_launcher_sha256: String,
    pub expected_sidecar_sha256: String,
    pub final_server_db_mode: String,
    pub generated_at: String,
}
