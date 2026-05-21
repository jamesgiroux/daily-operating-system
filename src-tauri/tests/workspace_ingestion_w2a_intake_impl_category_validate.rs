use std::path::PathBuf;

#[test]
fn intake_impl_validates_category_before_pipeline_invocation() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = std::fs::read_to_string(
        root.join("src/services/workspace_ingestion/workspace_intake_impl.rs"),
    )
    .expect("read bridge");
    let validate = source
        .find("WorkspaceCategoryRegistry::validate")
        .expect("validate call");
    let run = source.find(".run(conn, request)").expect("pipeline run");
    assert!(
        validate < run,
        "category validation must precede pipeline.run"
    );
    assert!(source.contains("WorkspaceIntakeError::CategoryNotAllowed"));
}
