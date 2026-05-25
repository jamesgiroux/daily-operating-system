use abilities_runtime::abilities::provenance::source::WorkspaceFileKind;
use chrono::Utc;
use dailyos_lib::db::ActionDb;
use dailyos_lib::entity::EntityType;
use dailyos_lib::services::context::{ExternalClients, ServiceContext, SystemClock, SystemRng};
use dailyos_lib::services::workspace_ingestion::contracts::{
    FileIdentity, NullSignalEmitter, RejectionReason, SignalEmitContext, SignalEmitError,
    SignalEmitter,
};
use dailyos_lib::services::workspace_ingestion::lifecycle::{LifecycleRepo, LifecycleState};
use dailyos_lib::services::workspace_ingestion::link::{LinkAttributionSource, LinkRepo};
use dailyos_lib::services::workspace_ingestion::pipeline::{quarantine_source, QuarantineActor};
use rusqlite::Connection;
use std::sync::Mutex;

#[derive(Default)]
struct RecordingSignalEmitter {
    quarantined_target: Mutex<Option<(Option<String>, Option<String>)>>,
}

impl SignalEmitter for RecordingSignalEmitter {
    fn emit_file_ingested(
        &self,
        _ctx: &SignalEmitContext<'_, '_>,
        _file_id: &str,
        _ingestion_run_id: &str,
        _entity_type: &str,
        _entity_id: &str,
    ) -> Result<(), SignalEmitError> {
        Ok(())
    }

    fn emit_file_rejected(
        &self,
        _ctx: &SignalEmitContext<'_, '_>,
        _file_id: Option<&str>,
        _reason: RejectionReason,
    ) -> Result<(), SignalEmitError> {
        Ok(())
    }

    fn emit_file_pending_entity_assignment(
        &self,
        _ctx: &SignalEmitContext<'_, '_>,
        _file_id: &str,
        _ingestion_run_id: &str,
    ) -> Result<(), SignalEmitError> {
        Ok(())
    }

    fn emit_file_quarantined(
        &self,
        _ctx: &SignalEmitContext<'_, '_>,
        _file_id: &str,
        _reason: &str,
        _actor: &str,
        entity_type: Option<&str>,
        entity_id: Option<&str>,
    ) -> Result<(), SignalEmitError> {
        *self.quarantined_target.lock().expect("record target") = Some((
            entity_type.map(str::to_string),
            entity_id.map(str::to_string),
        ));
        Ok(())
    }

    fn emit_link_changed(
        &self,
        _ctx: &SignalEmitContext<'_, '_>,
        _file_id: &str,
        _entity_type: &str,
        _entity_id: &str,
        _actor: &str,
    ) -> Result<(), SignalEmitError> {
        Ok(())
    }
}

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
    conn.execute_batch(include_str!(
        "../src/migrations/254_document_entity_links.sql"
    ))
    .expect("v254");
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
    let db = ActionDb::from_conn(&conn);
    let clock = SystemClock;
    let rng = SystemRng;
    let external = ExternalClients::default();
    let services = ServiceContext::new_live(&clock, &rng, &external);
    let signal_ctx = SignalEmitContext::new(&services, db, None);
    let emitter = NullSignalEmitter;
    quarantine_source(&signal_ctx, &emitter, "wf-1", "bad file", actor.clone())
        .expect("quarantine");
    quarantine_source(&signal_ctx, &emitter, "wf-1", "bad file", actor).expect("idempotent");
    let row = LifecycleRepo::get(&conn, "wf-1")
        .expect("get")
        .expect("row");
    assert_eq!(row.lifecycle_state, LifecycleState::Quarantined);
    assert_eq!(row.user_override.expect("override").actor_id, "user-1");
}

#[test]
fn quarantine_source_targets_single_active_link_when_lifecycle_entity_is_empty() {
    let conn = Connection::open_in_memory().expect("in-memory sqlite");
    conn.execute_batch(include_str!(
        "../src/migrations/250_workspace_file_lifecycle.sql"
    ))
    .expect("v250");
    conn.execute_batch(include_str!(
        "../src/migrations/251_workspace_file_lifecycle_category.sql"
    ))
    .expect("v251");
    conn.execute_batch(include_str!(
        "../src/migrations/254_document_entity_links.sql"
    ))
    .expect("v254");
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
    LinkRepo::add_link(
        &conn,
        "wf-1",
        EntityType::Account,
        "acc-1",
        LinkAttributionSource::Classifier,
        0.9,
        Some("fixture"),
        "system:test",
    )
    .expect("active link");

    let db = ActionDb::from_conn(&conn);
    let clock = SystemClock;
    let rng = SystemRng;
    let external = ExternalClients::default();
    let services = ServiceContext::new_live(&clock, &rng, &external);
    let signal_ctx = SignalEmitContext::new(&services, db, None);
    let emitter = RecordingSignalEmitter::default();
    quarantine_source(
        &signal_ctx,
        &emitter,
        "wf-1",
        "bad file",
        QuarantineActor::User {
            user_id: "user-1".into(),
        },
    )
    .expect("quarantine");

    let target = emitter
        .quarantined_target
        .lock()
        .expect("record target")
        .clone()
        .expect("quarantine signal target");
    assert_eq!(
        target,
        (Some("account".to_string()), Some("acc-1".to_string()))
    );
}
