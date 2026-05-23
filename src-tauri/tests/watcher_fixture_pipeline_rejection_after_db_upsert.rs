use std::path::Path;

use abilities_runtime::abilities::provenance::source::{EntityId, WorkspaceFileKind};
use chrono::{DateTime, Utc};
use dailyos_lib::db::{ActionDb, DbAccount};
use dailyos_lib::entity::EntityType;
use dailyos_lib::services::context::{ExternalClients, ServiceContext, SystemClock, SystemRng};
use dailyos_lib::services::workspace_ingestion::contracts::{
    NullExtractor, NullSignalEmitter, RejectionReason,
};
use dailyos_lib::services::workspace_ingestion::pipeline::{
    file_id_from_identity, EntityRef, IngestError, IngestPipeline, IngestRequest,
};
use dailyos_lib::services::workspace_ingestion::registry::WorkspaceSourceRegistry;
use dailyos_lib::services::workspace_ingestion::runs::IngestionMode;
use rusqlite::Connection;

#[test]
fn pipeline_rejection_after_db_upsert_preserves_entity_row_and_markdown() {
    let conn = Connection::open_in_memory().expect("sqlite");
    dailyos_lib::migration_test_api::run_migrations(&conn).expect("migrations");
    let db = ActionDb::from_conn(&conn);
    let workspace = tempfile::tempdir().expect("workspace");
    let workspace_root = workspace
        .path()
        .canonicalize()
        .expect("canonical workspace");
    let account_dir = workspace_root.join("Accounts").join("Acme");
    std::fs::create_dir_all(&account_dir).expect("account dir");
    let dashboard = account_dir.join("dashboard.json");
    std::fs::write(&dashboard, r#"{"version":1,"companyOverview":"too large"}"#)
        .expect("dashboard");

    let account = DbAccount {
        id: "acme".to_string(),
        name: "Acme".to_string(),
        tracker_path: Some("Accounts/Acme".to_string()),
        updated_at: Utc::now().to_rfc3339(),
        ..Default::default()
    };
    db.upsert_account(&account).expect("account upsert");
    dailyos_lib::accounts::write_account_markdown(&workspace_root, &account, None, db)
        .expect("markdown");

    let pipeline = IngestPipeline::new(
        Box::new(NullExtractor),
        Box::new(NullSignalEmitter),
        1,
        "w2b-reject-test",
        workspace_root.clone(),
    );
    let err = run_pipeline(
        &pipeline,
        &db,
        &workspace_root,
        &dashboard,
        WorkspaceFileKind::EntityDoc,
        Some(entity_ref(
            EntityType::Account,
            &account.id,
            Some(&account.name),
        )),
    )
    .expect_err("oversized file rejected");

    assert!(matches!(
        err,
        IngestError::Rejected(RejectionReason::FileTooLarge)
    ));
    assert!(db.get_account("acme").expect("get account").is_some());
    assert!(account_dir.join("dashboard.md").exists());
}

fn run_pipeline(
    pipeline: &IngestPipeline,
    db: &ActionDb,
    workspace_root: &Path,
    path: &Path,
    source_type: WorkspaceFileKind,
    entity: Option<EntityRef>,
) -> Result<(), IngestError> {
    let (file, identity) = WorkspaceSourceRegistry::open_validated(workspace_root, path)
        .map_err(IngestError::Rejected)?;
    let source_asof: DateTime<Utc> = identity
        .canonical_path
        .metadata()
        .and_then(|metadata| metadata.modified())
        .map_err(IngestError::Io)?
        .into();
    let file_id = file_id_from_identity(&identity, workspace_root)
        .map_err(|_| IngestError::Rejected(RejectionReason::OutsideWorkspace))?;

    let request = IngestRequest {
        file,
        identity,
        file_id,
        source_asof,
        source_type,
        entity,
        mode: IngestionMode::Realtime,
        category_hint: None,
        invocation_actor: "system:test".to_string(),
        validated_content: None,
    };
    let clock = SystemClock;
    let rng = SystemRng;
    let external = ExternalClients::default();
    let ctx = ServiceContext::new_live(&clock, &rng, &external).with_actor("system:test");
    pipeline.run(&ctx, db, request).map(|_| ())
}

fn entity_ref(entity_type: EntityType, id: &str, name: Option<&str>) -> EntityRef {
    EntityRef {
        entity_type,
        entity_id: EntityId::new(id.to_string()),
        entity_name: name.map(str::to_string),
    }
}
