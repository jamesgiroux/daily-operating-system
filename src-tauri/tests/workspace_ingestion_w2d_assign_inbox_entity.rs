use abilities_runtime::abilities::provenance::source::WorkspaceFileKind;
use chrono::{DateTime, Utc};
use dailyos_lib::db::{ActionDb, DbAccount};
use dailyos_lib::services::workspace_ingestion::lifecycle::{LifecycleRepo, LifecycleState};
use dailyos_lib::services::workspace_ingestion::pipeline::file_id_from_identity;
use dailyos_lib::services::workspace_ingestion::registry::WorkspaceSourceRegistry;
use rusqlite::Connection;
use std::path::{Path, PathBuf};

#[test]
fn assign_inbox_entity_adds_link_reruns_pipeline_and_ingests() {
    let conn = migrated_conn();
    let db = ActionDb::from_conn(&conn);
    let workspace = tempfile::tempdir().expect("workspace");
    let workspace_root = workspace
        .path()
        .canonicalize()
        .expect("canonical workspace");
    let file_path = write_inbox_file(
        &workspace_root,
        "notes.txt",
        b"Follow up with Acme next week.",
    );
    seed_account(db);
    let file_id = seed_pending_assignment(&conn, &workspace_root, &file_path);

    let receipt = dailyos_lib::command_test_api::assign_inbox_entity_for_tests(
        db,
        &workspace_root,
        file_id.clone(),
        "account".to_string(),
        "acme".to_string(),
        "acme".to_string(),
        "inbox".to_string(),
    )
    .expect("assign succeeds");

    assert_eq!(receipt.file_id, file_id);
    assert_eq!(receipt.lifecycle_state_after, "ingested");
    assert_eq!(receipt.resolved_path, Some("_inbox/notes.txt".to_string()));

    let lifecycle = LifecycleRepo::get(&conn, &file_id)
        .expect("get lifecycle")
        .expect("lifecycle row");
    assert_eq!(lifecycle.lifecycle_state, LifecycleState::Ingested);
    assert_eq!(lifecycle.entity_type.as_deref(), Some("account"));
    assert_eq!(lifecycle.entity_id.as_deref(), Some("acme"));

    let link_count: i64 = conn
        .query_row(
            "SELECT count(*) FROM document_entity_links \
             WHERE file_id = ?1 AND entity_type = 'account' AND entity_id = 'acme' \
               AND attribution_source = 'user_relink' AND rejected = 0",
            [&file_id],
            |row| row.get(0),
        )
        .expect("link count");
    assert_eq!(link_count, 1);

    let run_count: i64 = conn
        .query_row(
            "SELECT count(*) FROM document_ingestion_runs WHERE file_id = ?1 AND status = 'success'",
            [&file_id],
            |row| row.get(0),
        )
        .expect("run count");
    assert_eq!(run_count, 1);
}

fn seed_account(db: &ActionDb) {
    let account = DbAccount {
        id: "acme".to_string(),
        name: "Acme".to_string(),
        tracker_path: Some("Accounts/Acme".to_string()),
        updated_at: Utc::now().to_rfc3339(),
        ..Default::default()
    };
    db.upsert_account(&account).expect("account upsert");
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
