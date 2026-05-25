use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use dailyos_lib::abilities::provenance::source::EntityId;
use dailyos_lib::db::{ActionDb, DbAccount};
use dailyos_lib::entity::EntityType;
use dailyos_lib::services::context::{ExternalClients, ServiceContext, SystemClock, SystemRng};
use dailyos_lib::services::workspace_ingestion::contracts::{RejectionReason, WorkspaceFileKind};
use dailyos_lib::services::workspace_ingestion::pipeline::{
    file_id_from_identity, EntityRef, IngestReceipt, IngestRequest,
};
use dailyos_lib::services::workspace_ingestion::registry::WorkspaceSourceRegistry;
use dailyos_lib::services::workspace_ingestion::runs::IngestionMode;
use dailyos_lib::services::workspace_ingestion::wiring;
use parking_lot::Mutex;
use rusqlite::{params, Connection};

#[test]
fn graph_audit_zero_gaps_on_hermetic_fixture_db() {
    let fixture = Fixture::new();
    fixture.seed_account("acct-v146-graph", "Graph Account");
    let note = fixture.write_account_file(
        "Graph Account",
        "graph-note.md",
        "Graph validation context.",
    );

    let receipt = fixture.ingest_account_note(
        &note,
        EntityRef {
            entity_type: EntityType::Account,
            entity_id: EntityId::new("acct-v146-graph".to_string()),
            entity_name: Some("Graph Account".to_string()),
        },
        None,
    );

    assert_eq!(receipt.claim_proposals.len(), 1);
    assert_eq!(
        count(
            &fixture.conn,
            "SELECT COUNT(*) FROM intelligence_claims",
            [],
        ),
        1
    );
    assert_eq!(
        count(
            &fixture.conn,
            "SELECT COUNT(*)
             FROM workspace_file_lifecycle
             WHERE file_id = ?1 AND lifecycle_state = 'ingested'",
            params![&receipt.file_id],
        ),
        1
    );
    assert_eq!(
        count(
            &fixture.conn,
            "SELECT COUNT(*)
             FROM document_entity_links
             WHERE file_id = ?1
               AND entity_type = 'account'
               AND entity_id = 'acct-v146-graph'
               AND rejected = 0",
            params![&receipt.file_id],
        ),
        1
    );
    assert_eq!(
        count(
            &fixture.conn,
            "SELECT COUNT(*)
             FROM document_ingestion_runs
             WHERE run_id = ?1
               AND file_id = ?2
               AND status = 'success'
               AND claim_count_produced = 1",
            params![&receipt.ingestion_run_id.0, &receipt.file_id],
        ),
        1
    );

    let source_ref = format!("workspace_file:{}", receipt.file_id);
    let claim = fixture
        .conn
        .query_row(
            "SELECT subject_ref, data_source, source_ref, source_asof,
                    provenance_json, metadata_json, sensitivity
             FROM intelligence_claims
             WHERE source_ref = ?1",
            [&source_ref],
            |row| {
                Ok(ClaimRow {
                    subject_ref: row.get(0)?,
                    data_source: row.get(1)?,
                    source_ref: row.get(2)?,
                    source_asof: row.get(3)?,
                    provenance_json: row.get(4)?,
                    metadata_json: row.get(5)?,
                    sensitivity: row.get(6)?,
                })
            },
        )
        .expect("workspace claim row");

    let subject_ref: serde_json::Value =
        serde_json::from_str(&claim.subject_ref).expect("subject_ref json");
    assert_eq!(subject_ref["kind"], "account");
    assert_eq!(subject_ref["id"], "acct-v146-graph");
    assert_eq!(claim.data_source, "workspace_file:entity_doc");
    assert_eq!(claim.source_ref, source_ref);
    assert!(!claim.source_asof.trim().is_empty());
    assert_eq!(claim.sensitivity, "user_only");

    let provenance: serde_json::Value =
        serde_json::from_str(&claim.provenance_json).expect("provenance json");
    let metadata: serde_json::Value =
        serde_json::from_str(&claim.metadata_json).expect("metadata json");
    assert_eq!(metadata["producer"], "workspace_ingestion");
    assert_eq!(metadata["workspace_file_id"], receipt.file_id);
    assert_eq!(metadata["ingestion_run_id"], receipt.ingestion_run_id.0);
    assert_eq!(metadata["workspace_file_kind"], "entity_doc");
    assert_eq!(metadata["resolved_category"], "notes");
    assert!(metadata["document_entity_link_id"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));

    let serialized_provenance = provenance.to_string();
    assert!(serialized_provenance.contains(&receipt.file_id));
    assert!(!serialized_provenance.contains("Graph Account"));
    assert!(!serialized_provenance.contains("graph-note.md"));
    assert!(!serialized_provenance.contains("Graph validation context"));
}

#[test]
fn signal_propagation_invalidates_prep_partial_evidence() {
    let fixture = Fixture::new();
    fixture.seed_account("acct-v146-signal", "Signal Account");
    fixture.seed_upcoming_meeting("meeting-v146-signal", "acct-v146-signal");
    let note = fixture.write_account_file(
        "Signal Account",
        "signal-note.md",
        "Signal validation context that must stay out of signal payloads.",
    );
    let prep_queue = Arc::new(Mutex::new(Vec::<String>::new()));

    let receipt = fixture.ingest_account_note(
        &note,
        EntityRef {
            entity_type: EntityType::Account,
            entity_id: EntityId::new("acct-v146-signal".to_string()),
            entity_name: Some("Signal Account".to_string()),
        },
        Some(Arc::clone(&prep_queue)),
    );

    assert!(prep_queue
        .lock()
        .contains(&"meeting-v146-signal".to_string()));

    let (entity_type, entity_id, value): (String, String, Option<String>) = fixture
        .conn
        .query_row(
            "SELECT entity_type, entity_id, value
             FROM signal_events
             WHERE signal_type = 'workspace_file_ingested'
             ORDER BY created_at DESC
             LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("workspace_file_ingested signal");
    assert_eq!(entity_type, "account");
    assert_eq!(entity_id, "acct-v146-signal");

    let payload = value.expect("signal payload");
    assert!(payload.contains(&receipt.file_id));
    assert!(payload.contains(&receipt.ingestion_run_id.0));
    assert!(!payload.contains("Signal Account"));
    assert!(!payload.contains("signal-note.md"));
    assert!(!payload.contains("Signal validation context"));
    assert!(!payload.contains(fixture.workspace_root.to_string_lossy().as_ref()));
}

#[cfg(unix)]
#[test]
fn filesystem_validation_negative_fixtures() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::symlink;

    let fixture = Fixture::new();
    let outside = tempfile::tempdir().expect("outside tempdir");
    let outside_file = outside.path().join("outside.md");
    std::fs::write(&outside_file, "outside").expect("outside write");

    assert_rejected(
        &fixture.workspace_root,
        Path::new("notes/../escape.md"),
        RejectionReason::PathTraversalAttempt,
    );
    assert_rejected(
        &fixture.workspace_root,
        &outside_file,
        RejectionReason::OutsideWorkspace,
    );
    assert_rejected(
        &fixture.workspace_root,
        Path::new("."),
        RejectionReason::OutsideWorkspace,
    );

    let encoded_dir = fixture.workspace_root.join("%2e%2e");
    std::fs::create_dir(&encoded_dir).expect("encoded dir");
    std::fs::write(encoded_dir.join("escape.md"), "encoded").expect("encoded write");
    assert_rejected(
        &fixture.workspace_root,
        Path::new("%2e%2e/escape.md"),
        RejectionReason::PathTraversalAttempt,
    );

    let symlink_path = fixture.workspace_root.join("outside-link.md");
    symlink(&outside_file, &symlink_path).expect("symlink");
    assert_rejected(
        &fixture.workspace_root,
        Path::new("outside-link.md"),
        RejectionReason::OutsideWorkspace,
    );

    let nul_path = Path::new(OsStr::from_bytes(b"nul\0file.md"));
    assert_rejected(
        &fixture.workspace_root,
        nul_path,
        RejectionReason::PathTraversalAttempt,
    );

    let hardlink_source = fixture.workspace_root.join("hardlink-source.md");
    let hardlink_alias = fixture.workspace_root.join("hardlink-alias.md");
    std::fs::write(&hardlink_source, "hardlink").expect("hardlink source");
    if std::fs::hard_link(&hardlink_source, &hardlink_alias).is_ok() {
        assert_rejected(
            &fixture.workspace_root,
            Path::new("hardlink-source.md"),
            RejectionReason::SymlinkRefused,
        );
    }

    for table in [
        "workspace_file_lifecycle",
        "document_ingestion_runs",
        "document_entity_links",
        "intelligence_claims",
    ] {
        let sql = format!("SELECT COUNT(*) FROM {table}");
        assert_eq!(
            count(&fixture.conn, &sql, []),
            0,
            "{table} should stay empty"
        );
    }
}

#[cfg(not(unix))]
#[test]
fn filesystem_validation_negative_fixtures() {
    let fixture = Fixture::new();
    assert_rejected(
        &fixture.workspace_root,
        Path::new("notes/../escape.md"),
        RejectionReason::OutsideWorkspace,
    );
}

struct ClaimRow {
    subject_ref: String,
    data_source: String,
    source_ref: String,
    source_asof: String,
    provenance_json: String,
    metadata_json: String,
    sensitivity: String,
}

struct Fixture {
    conn: Connection,
    _workspace: tempfile::TempDir,
    workspace_root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let conn = Connection::open_in_memory().expect("sqlite");
        dailyos_lib::migration_test_api::run_migrations(&conn).expect("migrations");
        let workspace = tempfile::tempdir().expect("workspace");
        let workspace_root = workspace
            .path()
            .canonicalize()
            .expect("canonical workspace");
        Self {
            conn,
            _workspace: workspace,
            workspace_root,
        }
    }

    fn seed_account(&self, id: &str, name: &str) {
        let account = DbAccount {
            id: id.to_string(),
            name: name.to_string(),
            tracker_path: Some(format!("Accounts/{name}")),
            updated_at: Utc::now().to_rfc3339(),
            ..Default::default()
        };
        ActionDb::from_conn(&self.conn)
            .upsert_account(&account)
            .expect("account upsert");
    }

    fn seed_upcoming_meeting(&self, meeting_id: &str, account_id: &str) {
        let now = Utc::now();
        let start_time = (now + Duration::hours(2)).to_rfc3339();
        let created_at = now.to_rfc3339();
        self.conn
            .execute(
                "INSERT INTO meetings (id, title, meeting_type, start_time, created_at)
                 VALUES (?1, 'Validation Meeting', 'customer', ?2, ?3)",
                params![meeting_id, start_time, created_at],
            )
            .expect("meeting");
        self.conn
            .execute(
                "INSERT OR IGNORE INTO meeting_prep (meeting_id) VALUES (?1)",
                [meeting_id],
            )
            .expect("meeting_prep");
        self.conn
            .execute(
                "INSERT OR IGNORE INTO meeting_transcripts (meeting_id) VALUES (?1)",
                [meeting_id],
            )
            .expect("meeting_transcripts");
        self.conn
            .execute(
                "INSERT INTO meeting_entities (meeting_id, entity_id, entity_type)
                 VALUES (?1, ?2, 'account')",
                params![meeting_id, account_id],
            )
            .expect("meeting entity");
    }

    fn write_account_file(&self, account_dir: &str, filename: &str, content: &str) -> PathBuf {
        let dir = self.workspace_root.join("Accounts").join(account_dir);
        std::fs::create_dir_all(&dir).expect("account dir");
        let path = dir.join(filename);
        std::fs::write(&path, content).expect("write note");
        path
    }

    fn ingest_account_note(
        &self,
        path: &Path,
        entity: EntityRef,
        prep_queue: Option<Arc<Mutex<Vec<String>>>>,
    ) -> IngestReceipt {
        let (file, identity) = WorkspaceSourceRegistry::open_validated(&self.workspace_root, path)
            .expect("open validated");
        let source_asof: DateTime<Utc> = identity
            .canonical_path
            .metadata()
            .and_then(|metadata| metadata.modified())
            .expect("source asof")
            .into();
        let file_id = file_id_from_identity(&identity, &self.workspace_root).expect("file id");
        let request = IngestRequest {
            file,
            identity,
            file_id,
            source_asof,
            source_type: WorkspaceFileKind::EntityDoc,
            entity: Some(entity),
            mode: IngestionMode::Realtime,
            category_hint: None,
            invocation_actor: "user".to_string(),
            validated_content: None,
        };

        let clock = SystemClock;
        let rng = SystemRng;
        let external = ExternalClients::default();
        let ctx =
            ServiceContext::new_live(&clock, &rng, &external).with_actor("system:v146_validation");
        let db = ActionDb::from_conn(&self.conn);
        let pipeline = wiring::build_pipeline(self.workspace_root.clone());
        let mut signal_engine = dailyos_lib::signals::propagation::default_engine();
        if let Some(queue) = prep_queue {
            signal_engine.set_prep_queue(queue);
        }
        pipeline
            .run_with_signal_engine(&ctx, db, &signal_engine, request)
            .expect("ingest succeeds")
    }
}

fn assert_rejected(workspace_root: &Path, path: &Path, expected: RejectionReason) {
    let err = WorkspaceSourceRegistry::open_validated(workspace_root, path)
        .expect_err("path should be rejected");
    assert_eq!(err, expected);
}

fn count<P>(conn: &Connection, sql: &str, params: P) -> i64
where
    P: rusqlite::Params,
{
    conn.query_row(sql, params, |row| row.get(0))
        .expect("count query")
}
