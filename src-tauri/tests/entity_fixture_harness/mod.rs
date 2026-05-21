//! Harness modules. See `tests/entity_fixture_harness.rs` for the
//! integration entry point.

pub mod accounts;
pub mod assertions;
pub mod dom;
pub mod fixture_builder;
pub mod matrix;
pub mod persons;
pub mod projects;
pub mod red_first;

use std::path::PathBuf;

use abilities_runtime::abilities::get_entity_intelligence::EntityIntelligenceEnvelope;

/// Absolute path to `tests/entity_fixture_harness/fixtures/`.
pub fn fixtures_dir() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.push("tests");
    dir.push("entity_fixture_harness");
    dir.push("fixtures");
    dir
}

/// Load a fixture JSON file under `fixtures/` and parse it as an envelope.
/// Returns Err(string) describing parse failure (with file path) so
/// fixture authoring errors are loud and traceable.
pub fn load_envelope(file_name: &str) -> Result<EntityIntelligenceEnvelope, String> {
    let path = fixtures_dir().join(file_name);
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| format!("failed to read fixture {}: {e}", path.display()))?;
    serde_json::from_str::<EntityIntelligenceEnvelope>(&raw)
        .map_err(|e| format!("failed to parse fixture {}: {e}", path.display()))
}

/// Convenience: list fixture filenames present on disk under `fixtures/`.
/// Used by the matrix-completeness assertion (AC-461.5b).
pub fn list_fixture_files() -> Vec<String> {
    let dir = fixtures_dir();
    let Ok(read) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = read
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".json") {
                Some(name)
            } else {
                None
            }
        })
        .collect();
    names.sort();
    names
}
