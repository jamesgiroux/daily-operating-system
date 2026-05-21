use abilities_runtime::abilities::provenance::source::WorkspaceFileKind;
use chrono::Utc;
use dailyos_lib::services::workspace_ingestion::contracts::FileIdentity;
use dailyos_lib::services::workspace_ingestion::lifecycle::{LifecycleRepo, LifecycleState};
use dailyos_lib::services::workspace_ingestion::pipeline::{quarantine_source, QuarantineActor};
use rusqlite::Connection;

#[test]
fn quarantine_source_records_typed_actor_and_is_idempotent() {
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
    let actor = QuarantineActor::User {
        user_id: "user-1".into(),
    };
    quarantine_source(&conn, "wf-1", "bad file", actor.clone()).expect("quarantine");
    quarantine_source(&conn, "wf-1", "bad file", actor).expect("idempotent");
    let row = LifecycleRepo::get(&conn, "wf-1")
        .expect("get")
        .expect("row");
    assert_eq!(row.lifecycle_state, LifecycleState::Quarantined);
    assert_eq!(row.user_override.expect("override").actor_id, "user-1");
}
