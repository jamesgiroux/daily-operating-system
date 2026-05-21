//! v1.4.5 W1-A — `mod.rs` shape gate.
//!
//! Asserts that `src-tauri/src/services/workspace_ingestion/mod.rs` contains
//! exactly one `pub mod` declaration per submodule, alphabetical, with no
//! `pub use` re-exports and no inline items. This is the structural prevention
//! for the cycle 4–5 cross-lane mod.rs ownership conflict the wave plan
//! suffered (downstream lanes were editing mod.rs to add their submodule;
//! cycle 4 fixed it by having W1-A pre-create every placeholder).
//!
//! Subsequent v1.4.5 lanes (W1-B/W1-C/W2-A/W3-A/W3-B/W3-C) fill placeholder
//! content but never edit `mod.rs` and never create a new submodule file.

use std::fs;
use std::path::PathBuf;

const EXPECTED_PUB_MODS: &[&str] = &[
    "contracts", "extract", "graph", "lifecycle", "link", "pipeline", "registry", "runs",
    "signals", "wiring",
];

#[test]
fn mod_rs_has_exactly_expected_alphabetical_pub_mods() {
    let path: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("services")
        .join("workspace_ingestion")
        .join("mod.rs");
    let source = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));

    let mut found: Vec<String> = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("pub mod ") {
            // Strip trailing semicolon + any trailing whitespace.
            let name = rest.trim_end_matches(';').trim().to_string();
            found.push(name);
        }
    }

    assert_eq!(
        found,
        EXPECTED_PUB_MODS
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>(),
        "mod.rs `pub mod` declarations must match the expected alphabetical list exactly. \
         Subsequent v1.4.5 lanes fill placeholder content but never edit mod.rs."
    );

    // No `pub use` re-exports allowed in mod.rs (keeps the file purely a
    // submodule manifest so future agents have no surface to add to).
    for line in source.lines() {
        let trimmed = line.trim();
        assert!(
            !trimmed.starts_with("pub use "),
            "mod.rs must not contain `pub use` re-exports (found: {trimmed:?})"
        );
    }
}
