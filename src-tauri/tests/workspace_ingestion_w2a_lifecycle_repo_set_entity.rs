use abilities_runtime::abilities::provenance::source::WorkspaceFileKind;
use chrono::Utc;
use dailyos_lib::entity::EntityType;
use dailyos_lib::services::workspace_ingestion::contracts::FileIdentity;
use dailyos_lib::services::workspace_ingestion::lifecycle::{LifecycleError, LifecycleRepo};
use rusqlite::Connection;

#[test]
fn lifecycle_set_entity_updates_pending_row_and_rejects_missing_file() {
    let conn = Connection::open_in_memory().expect("in-memory sqlite");
    conn.execute_batch(include_str!(
        "../src/migrations/250_workspace_file_lifecycle.sql"
    ))
    .expect("v250");
    conn.execute_batch(include_str!(
        "../src/migrations/251_workspace_file_lifecycle_category.sql"
    ))
    .expect("v251");
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
    LifecycleRepo::set_entity(&conn, "wf-1", EntityType::Account, "acct-1", Some("acme"))
        .expect("set entity");
    let row = LifecycleRepo::get(&conn, "wf-1")
        .expect("get")
        .expect("row");
    assert_eq!(row.entity_type.as_deref(), Some("account"));
    assert_eq!(row.entity_id.as_deref(), Some("acct-1"));

    let err = LifecycleRepo::set_entity(&conn, "missing", EntityType::Account, "acct-1", None)
        .expect_err("missing");
    assert!(matches!(err, LifecycleError::FileNotFound));
}
