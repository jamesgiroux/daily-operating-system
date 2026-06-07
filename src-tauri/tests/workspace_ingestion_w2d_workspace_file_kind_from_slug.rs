use abilities_runtime::abilities::provenance::source::WorkspaceFileKind;

#[test]
fn workspace_file_kind_from_slug_covers_canonical_variants() {
    assert_eq!(
        WorkspaceFileKind::from_slug("inbox"),
        Some(WorkspaceFileKind::Inbox)
    );
    assert_eq!(
        WorkspaceFileKind::from_slug("entity_doc"),
        Some(WorkspaceFileKind::EntityDoc)
    );
    assert_eq!(
        WorkspaceFileKind::from_slug("drive_sync"),
        Some(WorkspaceFileKind::DriveSync)
    );
    assert_eq!(
        WorkspaceFileKind::from_slug("user_attachment"),
        Some(WorkspaceFileKind::UserAttachment)
    );
    assert_eq!(
        WorkspaceFileKind::from_slug("generic_transcript"),
        Some(WorkspaceFileKind::GenericTranscript)
    );
    assert_eq!(
        WorkspaceFileKind::from_slug("granola_transcript"),
        Some(WorkspaceFileKind::GranolaTranscript)
    );
    assert_eq!(
        WorkspaceFileKind::from_slug("quill_transcript"),
        Some(WorkspaceFileKind::QuillTranscript)
    );
    assert_eq!(
        WorkspaceFileKind::from_slug("mcp_placement"),
        Some(WorkspaceFileKind::McpPlacement)
    );
    assert_eq!(WorkspaceFileKind::from_slug("unknown"), None);
}
