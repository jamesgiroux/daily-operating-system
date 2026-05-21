use abilities_runtime::abilities::provenance::source::WorkspaceFileKind;
use chrono::Utc;
use dailyos_lib::services::workspace_ingestion::contracts::{FileIdentity, WorkspaceCategory};
use dailyos_lib::services::workspace_ingestion::lifecycle::{LifecycleRepo, LifecycleState};
use rusqlite::Connection;

fn conn() -> Connection {
    let conn = Connection::open_in_memory().expect("in-memory sqlite");
    conn.execute_batch(include_str!(
        "../src/migrations/250_workspace_file_lifecycle.sql"
    ))
    .expect("v250");
    conn.execute_batch(include_str!(
        "../src/migrations/251_workspace_file_lifecycle_category.sql"
    ))
    .expect("v251");
    conn
}

fn identity() -> FileIdentity {
    FileIdentity {
        canonical_path: "/tmp/workspace/file.md".into(),
        device: 1,
        inode: 2,
    }
}

#[test]
fn lifecycle_repo_writes_pending_transition_override_and_category() {
    let conn = conn();
    LifecycleRepo::insert_pending(
        &conn,
        "wf-1",
        &identity(),
        &WorkspaceFileKind::Inbox,
        Utc::now(),
        None,
    )
    .expect("insert pending");
    LifecycleRepo::transition(
        &conn,
        "wf-1",
        LifecycleState::Pending,
        LifecycleState::Ingesting,
    )
    .expect("transition");
    LifecycleRepo::record_user_override(&conn, "wf-1", "user-1").expect("override");
    LifecycleRepo::update_category(&conn, "wf-1", Some(&WorkspaceCategory::Notes))
        .expect("category");

    let row = LifecycleRepo::get(&conn, "wf-1")
        .expect("get")
        .expect("row");
    assert_eq!(row.lifecycle_state, LifecycleState::Ingesting);
    assert_eq!(row.category, Some(WorkspaceCategory::Notes));
    assert_eq!(row.user_override.expect("override").actor_id, "user-1");
}
