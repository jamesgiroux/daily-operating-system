use abilities_runtime::abilities::provenance::source::WorkspaceFileKind;
use chrono::Utc;
use dailyos_lib::services::workspace_ingestion::contracts::FileIdentity;
use dailyos_lib::services::workspace_ingestion::lifecycle::{
    LifecycleError, LifecycleRepo, LifecycleState,
};
use rusqlite::Connection;

fn seeded_conn() -> Connection {
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
    .expect("seed");
    conn
}

#[test]
fn legal_and_illegal_lifecycle_transitions_are_enforced() {
    let conn = seeded_conn();
    LifecycleRepo::transition(
        &conn,
        "wf-1",
        LifecycleState::Pending,
        LifecycleState::Ingesting,
    )
    .expect("legal");
    let err = LifecycleRepo::transition(
        &conn,
        "wf-1",
        LifecycleState::Ingesting,
        LifecycleState::Pending,
    )
    .expect_err("illegal");
    assert!(matches!(err, LifecycleError::InvalidStateTransition { .. }));
}
