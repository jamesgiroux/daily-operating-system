use std::fs;
use std::path::Path;

#[test]
fn ability_runtime_boundary_compile_failures() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/trybuild/boundary/*.rs");
}

#[test]
fn ability_runtime_manifest_excludes_raw_app_dependencies() {
    let manifest = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
        .expect("read abilities-runtime Cargo.toml");

    for denied in ["rusqlite", "tauri", "tokio-rusqlite"] {
        assert!(
            !manifest.contains(denied),
            "abilities-runtime must not depend on raw app boundary crate `{denied}`"
        );
    }
}
