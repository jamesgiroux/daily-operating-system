use std::path::PathBuf;

#[test]
fn workspace_intake_impl_bridges_raw_slugs_through_spawn_blocking() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = std::fs::read_to_string(
        root.join("src/services/workspace_ingestion/workspace_intake_impl.rs"),
    )
    .expect("read bridge");
    assert!(source.contains("tokio::task::spawn_blocking"));
    assert!(source.contains("WorkspaceSourceRegistry::open_validated"));
    assert!(source.contains("file_id_from_identity"));
    assert!(source.contains("build_pipeline(workspace_root)"));
    assert!(source.contains(".run(conn, request)"));
    assert!(source.contains("resolved_path: receipt.resolved_path"));
}
