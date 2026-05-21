use std::path::PathBuf;

#[test]
fn workspace_intake_is_declared_inside_inline_services_module() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let lib_rs = std::fs::read_to_string(root.join("abilities-runtime/src/lib.rs"))
        .expect("read abilities-runtime lib.rs");
    assert!(lib_rs.contains("pub mod services {"));
    assert!(lib_rs.contains("pub mod workspace_intake;"));
    assert!(
        !root.join("abilities-runtime/src/services/mod.rs").exists(),
        "W2-A must not create a separate services/mod.rs"
    );
}
