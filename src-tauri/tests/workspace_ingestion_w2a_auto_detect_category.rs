use dailyos_lib::entity::EntityType;
use dailyos_lib::services::workspace_ingestion::contracts::WorkspaceCategory;
use dailyos_lib::services::workspace_ingestion::pipeline::IngestPipeline;
use dailyos_lib::services::workspace_ingestion::registry::WorkspaceCategoryRegistry;
use rusqlite::Connection;

fn registry_conn() -> Connection {
    let conn = Connection::open_in_memory().expect("in-memory sqlite");
    conn.execute_batch(include_str!(
        "../src/migrations/252_workspace_source_registry.sql"
    ))
    .expect("v252");
    conn
}

#[test]
fn auto_detect_category_uses_frozen_priority_table() {
    assert_eq!(
        IngestPipeline::auto_detect_category_pure(
            "q1-transcript-notes.md",
            "---\ndoc_type: deck\n---\nbody"
        ),
        Some(WorkspaceCategory::Presentations)
    );
    assert_eq!(
        IngestPipeline::auto_detect_category_pure("ACME-DECK-FINAL.txt", ""),
        Some(WorkspaceCategory::Presentations)
    );
    assert_eq!(
        IngestPipeline::auto_detect_category_pure("meeting-contract.pdf", ""),
        Some(WorkspaceCategory::Meetings),
        "filename rules are first-match within priority 2"
    );
    assert_eq!(
        IngestPipeline::auto_detect_category_pure("plain.pdf", ""),
        Some(WorkspaceCategory::Attachments)
    );
    assert_eq!(
        IngestPipeline::auto_detect_category_pure("plain.md", ""),
        Some(WorkspaceCategory::Notes)
    );
    assert_eq!(
        IngestPipeline::auto_detect_category_pure("plain.bin", "---\ndoc_type: Bad\n---\n"),
        None
    );
}

/// Regression for L2 cycle-1 codex BLOCK: a SHAPE-INVALID frontmatter
/// `doc_type` MUST resolve to terminal None at priority 1, not fall through
/// to filename-glob at priority 2. §7 security gate requires user-supplied
/// invalid intent to suppress filename override. Codex's exact example:
/// `doc_type: Bad` (uppercase B fails the shape check) plus a filename
/// that would otherwise match the `meeting` glob.
///
/// Note: shape-valid but unregistered `Other(s)` slugs are NOT this case —
/// they propagate as `Some(Other(s))` from `auto_detect_category_pure` and
/// get rejected later by `validate_detected_category` at registry-validation
/// time. That separation is tested in `detected_other_slug_survives_only_after_registry_validation`.
#[test]
fn invalid_frontmatter_doctype_is_terminal_none_even_with_filename_match() {
    // Invalid shape + filename glob that would otherwise match `meeting`.
    assert_eq!(
        IngestPipeline::auto_detect_category_pure(
            "meeting-notes.md",
            "---\ndoc_type: Bad\n---\nbody"
        ),
        None,
        "invalid-shape doc_type must suppress filename-glob fallback"
    );
    // Invalid shape with leading digit + filename glob that would match
    // `transcript`.
    assert_eq!(
        IngestPipeline::auto_detect_category_pure(
            "q3-transcript-notes.md",
            "---\ndoc_type: 1on1\n---\nbody"
        ),
        None,
        "leading-digit doc_type is shape-invalid; filename-glob fallback suppressed"
    );
    // Sanity: same filenames WITHOUT frontmatter still hit the filename glob.
    assert_eq!(
        IngestPipeline::auto_detect_category_pure("meeting-notes.md", ""),
        Some(WorkspaceCategory::Meetings)
    );
    assert_eq!(
        IngestPipeline::auto_detect_category_pure("q3-transcript-notes.md", ""),
        Some(WorkspaceCategory::Transcripts)
    );
    // Sanity: frontmatter without `doc_type:` key still falls through.
    assert_eq!(
        IngestPipeline::auto_detect_category_pure(
            "meeting-notes.md",
            "---\nauthor: jamesgiroux\n---\nbody"
        ),
        Some(WorkspaceCategory::Meetings),
        "frontmatter without doc_type key MUST fall through to priority 2"
    );
}

#[test]
fn detected_other_slug_survives_only_after_registry_validation() {
    let conn = registry_conn();
    WorkspaceCategoryRegistry::register_other(&conn, EntityType::Account, "briefs")
        .expect("register custom category");
    let detected =
        IngestPipeline::auto_detect_category_pure("file.txt", "---\ndoc_type: briefs\n---\n");
    assert_eq!(detected, Some(WorkspaceCategory::Other("briefs".into())));
    assert_eq!(
        IngestPipeline::validate_detected_category(&conn, detected, EntityType::Account)
            .expect("validate"),
        Some(WorkspaceCategory::Other("briefs".into()))
    );
    assert_eq!(
        IngestPipeline::validate_detected_category(
            &conn,
            Some(WorkspaceCategory::Other("missing".into())),
            EntityType::Account,
        )
        .expect("validate"),
        None
    );
}
