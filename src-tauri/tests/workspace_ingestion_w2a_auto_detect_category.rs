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
