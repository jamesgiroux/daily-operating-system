use abilities_runtime::abilities::provenance::source::WorkspaceFileKind;
use chrono::{DateTime, Utc};
use dailyos_lib::db::ActionDb;
use dailyos_lib::services::workspace_ingestion::lifecycle::{LifecycleRepo, LifecycleState};
use dailyos_lib::services::workspace_ingestion::pipeline::file_id_from_identity;
use dailyos_lib::services::workspace_ingestion::registry::WorkspaceSourceRegistry;
use rusqlite::Connection;
use std::path::{Path, PathBuf};

#[test]
fn process_inbox_file_without_entity_id_creates_lifecycle_row_and_run() {
    let conn = migrated_conn();
    let db = ActionDb::from_conn(&conn);
    let workspace = tempfile::tempdir().expect("workspace");
    let workspace_root = workspace.path().canonicalize().expect("workspace root");
    let file_path = write_inbox_file(&workspace_root, "notes.txt", b"Follow up with Acme.");

    let result = dailyos_lib::command_test_api::process_inbox_file_for_tests(
        db,
        &workspace_root,
        "notes.txt",
    )
    .expect("process");

    assert_eq!(result["status"], "needs_entity");
    let file_id = file_id_for(&workspace_root, &file_path);
    let lifecycle = LifecycleRepo::get(&conn, &file_id)
        .expect("get lifecycle")
        .expect("lifecycle row");
    assert_eq!(
        lifecycle.lifecycle_state,
        LifecycleState::PendingEntityAssignment
    );

    let run_count: i64 = conn
        .query_row(
            "SELECT count(*) FROM document_ingestion_runs WHERE file_id = ?1 AND status = 'success'",
            [&file_id],
            |row| row.get(0),
        )
        .expect("run count");
    assert_eq!(run_count, 1);
}

#[test]
fn invalid_inbox_filename_emits_workspace_rejection_signal() {
    let conn = migrated_conn();
    let db = ActionDb::from_conn(&conn);
    let workspace = tempfile::tempdir().expect("workspace");
    let workspace_root = workspace.path().canonicalize().expect("workspace root");

    let error = dailyos_lib::command_test_api::process_inbox_file_for_tests(
        db,
        &workspace_root,
        "../outside.txt",
    )
    .expect_err("invalid filename rejected");

    assert!(error.contains("path traversal"));
    let value: String = conn
        .query_row(
            "SELECT value FROM signal_events
             WHERE entity_type = 'workspace_ingestion'
               AND entity_id = 'workspace_ingestion'
               AND signal_type = 'workspace_file_rejected'",
            [],
            |row| row.get(0),
        )
        .expect("workspace rejection signal");
    assert!(value.contains("\"reason_code\":\"path_traversal_attempt\""));
    assert!(!value.contains("\"file_id\""));
}

#[test]
fn get_inbox_files_returns_lifecycle_rows_not_filesystem_listing() {
    let conn = migrated_conn();
    let db = ActionDb::from_conn(&conn);
    let workspace = tempfile::tempdir().expect("workspace");
    let workspace_root = workspace.path().canonicalize().expect("workspace root");
    let tracked = write_inbox_file(&workspace_root, "tracked.txt", b"Tracked content");
    write_inbox_file(&workspace_root, "loose.txt", b"Filesystem-only content");
    let file_id = seed_pending_assignment(&conn, &workspace_root, &tracked);

    let files = dailyos_lib::command_test_api::get_inbox_files_for_tests(db, &workspace_root)
        .expect("files");

    assert_eq!(files.len(), 1);
    assert_eq!(files[0].file_id.as_deref(), Some(file_id.as_str()));
    assert_eq!(files[0].filename, "tracked.txt");
    assert_eq!(files[0].processing_status.as_deref(), Some("needs_entity"));
}

#[test]
fn copy_to_inbox_creates_lifecycle_row_after_copy() {
    let conn = migrated_conn();
    let db = ActionDb::from_conn(&conn);
    let workspace = tempfile::tempdir().expect("workspace");
    let workspace_root = workspace.path().canonicalize().expect("workspace root");
    let source_dir = tempfile::tempdir().expect("source");
    let source_root = source_dir.path().canonicalize().expect("source root");
    let source = source_root.join("drop.txt");
    std::fs::write(&source, b"Dropped inbox content").expect("write source");

    let report = dailyos_lib::command_test_api::copy_to_inbox_for_tests(
        db,
        &workspace_root,
        vec![source.to_string_lossy().to_string()],
        std::slice::from_ref(&source_root),
    )
    .expect("copy");

    assert_eq!(report.copied_count, 1);
    assert_eq!(report.copied_filenames, vec!["drop.txt"]);

    let inbox_path = workspace_root.join("_inbox/drop.txt");
    assert!(inbox_path.is_file());
    let file_id = file_id_for(&workspace_root, &inbox_path);
    let lifecycle = LifecycleRepo::get(&conn, &file_id)
        .expect("get lifecycle")
        .expect("lifecycle row");
    assert_eq!(
        lifecycle.lifecycle_state,
        LifecycleState::PendingEntityAssignment
    );
}

fn migrated_conn() -> Connection {
    let conn = Connection::open_in_memory().expect("sqlite");
    dailyos_lib::migration_test_api::run_migrations(&conn).expect("migrations");
    conn
}

fn write_inbox_file(workspace_root: &Path, filename: &str, bytes: &[u8]) -> PathBuf {
    let inbox = workspace_root.join("_inbox");
    std::fs::create_dir_all(&inbox).expect("inbox dir");
    let path = inbox.join(filename);
    std::fs::write(&path, bytes).expect("write inbox file");
    path
}

fn file_id_for(workspace_root: &Path, file_path: &Path) -> String {
    let (_file, identity) =
        WorkspaceSourceRegistry::open_validated(workspace_root, file_path).expect("open");
    file_id_from_identity(&identity, workspace_root).expect("file id")
}

fn seed_pending_assignment(conn: &Connection, workspace_root: &Path, file_path: &Path) -> String {
    let (_file, identity) =
        WorkspaceSourceRegistry::open_validated(workspace_root, file_path).expect("open");
    let source_asof: DateTime<Utc> = identity
        .canonical_path
        .metadata()
        .and_then(|metadata| metadata.modified())
        .expect("mtime")
        .into();
    let file_id = file_id_from_identity(&identity, workspace_root).expect("file id");
    LifecycleRepo::insert_pending(
        conn,
        &file_id,
        &identity,
        &WorkspaceFileKind::Inbox,
        source_asof,
        None,
    )
    .expect("insert lifecycle");
    LifecycleRepo::transition(
        conn,
        &file_id,
        LifecycleState::Pending,
        LifecycleState::Ingesting,
    )
    .expect("ingesting");
    LifecycleRepo::transition(
        conn,
        &file_id,
        LifecycleState::Ingesting,
        LifecycleState::PendingEntityAssignment,
    )
    .expect("pending entity assignment");
    file_id
}
