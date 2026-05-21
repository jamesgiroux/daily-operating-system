use abilities_runtime::abilities::provenance::source::WorkspaceFileKind;
use chrono::Utc;
use dailyos_lib::services::workspace_ingestion::contracts::FileIdentity;
use dailyos_lib::services::workspace_ingestion::lifecycle::LifecycleRepo;
use rusqlite::Connection;

#[test]
fn lifecycle_get_returns_none_or_populated_row() {
    let conn = Connection::open_in_memory().expect("in-memory sqlite");
    conn.execute_batch(include_str!(
        "../src/migrations/250_workspace_file_lifecycle.sql"
    ))
    .expect("v250");
    conn.execute_batch(include_str!(
        "../src/migrations/251_workspace_file_lifecycle_category.sql"
    ))
    .expect("v251");
    assert!(LifecycleRepo::get(&conn, "missing").expect("get").is_none());
    let identity = FileIdentity {
        canonical_path: "/tmp/workspace/file.md".into(),
        device: 1,
        inode: 2,
    };
    LifecycleRepo::insert_pending(
        &conn,
        "wf-1",
        &identity,
        &WorkspaceFileKind::Inbox,
        Utc::now(),
        None,
    )
    .expect("insert");
    assert_eq!(
        LifecycleRepo::get(&conn, "wf-1")
            .expect("get")
            .expect("row")
            .file_id,
        "wf-1"
    );
}
