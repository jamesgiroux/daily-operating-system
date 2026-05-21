use abilities_runtime::services::workspace_intake::WorkspaceIntakeError;

#[test]
fn workspace_intake_error_has_typed_invalid_slug_variants() {
    assert!(matches!(
        WorkspaceIntakeError::InvalidSourceTypeSlug("bad".into()),
        WorkspaceIntakeError::InvalidSourceTypeSlug(_)
    ));
    assert!(matches!(
        WorkspaceIntakeError::InvalidModeSlug("bad".into()),
        WorkspaceIntakeError::InvalidModeSlug(_)
    ));
    assert!(matches!(
        WorkspaceIntakeError::InvalidCategorySlug("bad".into()),
        WorkspaceIntakeError::InvalidCategorySlug(_)
    ));
    assert!(matches!(
        WorkspaceIntakeError::InvalidEntityName("Bad Name".into()),
        WorkspaceIntakeError::InvalidEntityName(_)
    ));
}
