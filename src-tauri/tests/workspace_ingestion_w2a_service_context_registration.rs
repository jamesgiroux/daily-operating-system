use std::path::PathBuf;

#[test]
fn service_context_exposes_optional_workspace_intake_and_live_registration() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let context = std::fs::read_to_string(root.join("abilities-runtime/src/services/context.rs"))
        .expect("read abilities-runtime context");
    assert!(context.contains("workspace_intake: Option<Arc<dyn WorkspaceIntakeService>>"));
    assert!(
        context.contains("pub fn workspace_intake(&self) -> Option<&dyn WorkspaceIntakeService>")
    );

    let app_context = std::fs::read_to_string(root.join("src/services/context.rs"))
        .expect("read app service context");
    assert!(app_context.contains("attach_live_workspace_readers_with_signal_engine"));
    assert!(app_context.contains(
        "IngestPipelineWorkspaceIntake::from_config_or_empty_with_signal_engine(signal_engine)"
    ));
    assert!(app_context.contains(".with_workspace_intake("));
}
