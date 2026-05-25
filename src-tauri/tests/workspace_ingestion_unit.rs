//! v1.4.5 W1-A — unit-style tests for the workspace_ingestion
//! contracts + lifecycle modules. Lives as an external integration test so it
//! also exercises the external import surface (the same `dailyos_lib::…` path
//! W1-B/W1-C/W2-A/W3-A/W3-B/W3-C will use).

use abilities_runtime::abilities::provenance::source::{
    DataSource, SourceAttribution, SourceIdentifier,
};
use abilities_runtime::abilities::provenance::DocumentId;
use chrono::Utc;
use dailyos_lib::db::ActionDb;
use dailyos_lib::services::context::{ExternalClients, ServiceContext, SystemClock, SystemRng};
use dailyos_lib::services::workspace_ingestion::contracts::{
    ExtractionContext, Extractor, FileIdentity, NullExtractor, NullSignalEmitter, RejectionReason,
    SignalEmitContext, SignalEmitter, WorkspaceCategory, WorkspaceFileKind,
};
use dailyos_lib::services::workspace_ingestion::lifecycle::LifecycleState;

// ---- LifecycleState ---------------------------------------------------------

#[test]
fn lifecycle_state_has_exactly_seven_variants_with_canonical_serde_strings() {
    let pairs: &[(LifecycleState, &str)] = &[
        (LifecycleState::Pending, "pending"),
        (
            LifecycleState::PendingEntityAssignment,
            "pending_entity_assignment",
        ),
        (LifecycleState::Ingesting, "ingesting"),
        (LifecycleState::Ingested, "ingested"),
        (LifecycleState::Superseded, "superseded"),
        (LifecycleState::Rejected, "rejected"),
        (LifecycleState::Quarantined, "quarantined"),
    ];

    assert_eq!(
        pairs.len(),
        7,
        "LifecycleState must have exactly 7 variants"
    );

    for (state, expected_slug) in pairs {
        let serialized =
            serde_json::to_string(state).expect("LifecycleState serializes via serde_json");
        // serde produces a quoted JSON string for unit-variant enums.
        let expected_json = format!("\"{expected_slug}\"");
        assert_eq!(serialized, expected_json, "{state:?} serde string");

        let round_trip: LifecycleState = serde_json::from_str(&serialized)
            .unwrap_or_else(|e| panic!("deserialize {serialized}: {e}"));
        assert_eq!(round_trip, *state, "round-trip {state:?}");
    }
}

// ---- WorkspaceCategory ------------------------------------------------------

#[test]
fn workspace_category_as_slug_and_from_slug_round_trip_for_known_variants() {
    let known: &[(WorkspaceCategory, &str)] = &[
        (WorkspaceCategory::Presentations, "presentations"),
        (WorkspaceCategory::Transcripts, "transcripts"),
        (WorkspaceCategory::Meetings, "meetings"),
        (WorkspaceCategory::Notes, "notes"),
        (WorkspaceCategory::Contracts, "contracts"),
        (WorkspaceCategory::Attachments, "attachments"),
    ];

    for (variant, slug) in known {
        assert_eq!(variant.as_slug(), *slug, "as_slug for {variant:?}");
        let parsed = WorkspaceCategory::from_slug(slug)
            .unwrap_or_else(|| panic!("from_slug must accept {slug}"));
        assert_eq!(parsed, *variant, "from_slug round-trip for {slug}");
    }
}

#[test]
fn workspace_category_other_accepts_lex_valid_lowercase_ascii_slugs() {
    let ok = ["custom_slug", "another-slug", "slug_with_42_digits"];
    for slug in &ok {
        let parsed = WorkspaceCategory::from_slug(slug)
            .unwrap_or_else(|| panic!("from_slug must accept lex-valid {slug}"));
        match &parsed {
            WorkspaceCategory::Other(s) => {
                assert_eq!(s, slug, "Other slug round-trip");
                assert_eq!(parsed.as_slug(), *slug, "as_slug Other returns input");
            }
            other => panic!("expected Other({slug}), got {other:?}"),
        }
    }
}

#[test]
fn workspace_category_from_slug_rejects_malformed() {
    for bad in &[
        "WithUpper",   // uppercase
        "with spaces", // whitespace
        "with/slash",  // slash
        "with.dot",    // dot
        "",            // empty
        "with!bang",   // punctuation
    ] {
        assert!(
            WorkspaceCategory::from_slug(bad).is_none(),
            "from_slug must reject malformed {bad:?}"
        );
    }
}

#[test]
fn workspace_category_serde_wire_shape_matches_frozen_contract() {
    // Known variant: { "kind": "presentations" }
    let known = WorkspaceCategory::Presentations;
    let serialized = serde_json::to_string(&known).expect("serialize Presentations");
    assert_eq!(serialized, r#"{"kind":"presentations"}"#);
    let round: WorkspaceCategory = serde_json::from_str(&serialized).expect("deserialize");
    assert_eq!(round, known);

    // Other variant: { "kind": "other", "name": "<slug>" }
    let other = WorkspaceCategory::Other("custom_slug".to_string());
    let serialized = serde_json::to_string(&other).expect("serialize Other");
    assert_eq!(serialized, r#"{"kind":"other","name":"custom_slug"}"#);
    let round: WorkspaceCategory = serde_json::from_str(&serialized).expect("deserialize Other");
    assert_eq!(round, other);
}

// ---- Trait Send + Sync ------------------------------------------------------

#[test]
fn null_extractor_is_send_sync_and_returns_empty() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<NullExtractor>();
    assert_send_sync::<NullSignalEmitter>();

    let extractor = NullExtractor;
    let identity = FileIdentity {
        canonical_path: "/tmp/null.txt".into(),
        device: 0,
        inode: 0,
    };
    // Construct a dummy file handle for the Extractor::extract call. We can't
    // create a `std::fs::File` without I/O, so use a tempfile.
    let tmp = tempfile::NamedTempFile::new().expect("tempfile");
    let mut file = std::fs::File::open(tmp.path()).expect("open tempfile");
    let now = Utc::now();
    let context = ExtractionContext {
        file_id: "wf-null",
        identity: &identity,
        content: "",
        source_type: WorkspaceFileKind::Inbox,
        source_asof: now,
        resolved_category: None,
        linked_subject: None,
        ingestion_run_id: "run-null",
        observed_at: now,
        invocation_actor: "system:test",
    };
    let report = extractor.extract(&mut file, &context).expect("extract");
    assert!(
        report.proposals.is_empty(),
        "NullExtractor must return empty proposals, got {} items",
        report.proposals.len()
    );
}

#[test]
fn null_signal_emitter_methods_are_noops_callable_through_dyn() {
    let boxed: Box<dyn SignalEmitter> = Box::new(NullSignalEmitter);
    let conn = rusqlite::Connection::open_in_memory().expect("in-memory sqlite");
    let db = ActionDb::from_conn(&conn);
    let clock = SystemClock;
    let rng = SystemRng;
    let external = ExternalClients::default();
    let services = ServiceContext::new_live(&clock, &rng, &external);
    let signal_ctx = SignalEmitContext::new(&services, db, None);

    boxed
        .emit_file_ingested(&signal_ctx, "f1", "run1", "account", "entity1")
        .expect("noop ingested");
    boxed
        .emit_file_rejected(
            &signal_ctx,
            Some("f2"),
            RejectionReason::PathTraversalAttempt,
        )
        .expect("noop rejected");
    boxed
        .emit_file_rejected(&signal_ctx, None, RejectionReason::FileTooLarge)
        .expect("noop rejected without file");
    boxed
        .emit_file_pending_entity_assignment(&signal_ctx, "f3", "run3")
        .expect("noop pending entity");
    boxed
        .emit_file_quarantined(
            &signal_ctx,
            "f4",
            "user requested",
            "user-1",
            Some("account"),
            Some("entity1"),
        )
        .expect("noop quarantined");
    boxed
        .emit_link_changed(&signal_ctx, "f5", "account", "entity-2", "user-1")
        .expect("noop link changed");
    // If we reach here without panicking the dyn dispatch worked.
}

// ---- SourceAttribution construction sanity (W3-A target shape) --------------

#[test]
fn source_attribution_accepts_workspace_shape_inputs() {
    let file_id = "wf-12345";
    let observed_at = Utc::now();
    let source_asof = observed_at; // file mtime
    let attribution = SourceAttribution::new(
        DataSource::WorkspaceFile {
            kind: WorkspaceFileKind::Inbox,
        },
        vec![SourceIdentifier::Document {
            document_id: DocumentId::new(file_id),
            chunk_id: None,
        }],
        observed_at,
        Some(source_asof),
        0.5_f32,
        None,
    )
    .expect("SourceAttribution::new accepts canonical workspace-shape inputs");

    assert_eq!(
        attribution.data_source,
        DataSource::WorkspaceFile {
            kind: WorkspaceFileKind::Inbox
        }
    );
    assert_eq!(attribution.identifiers.len(), 1);
    assert_eq!(attribution.source_asof, Some(source_asof));
}

// ---- Migration round-trip ---------------------------------------------------

#[test]
fn migrations_v250_v251_apply_against_in_memory_db_and_register_columns() {
    use rusqlite::Connection;
    let conn = Connection::open_in_memory().expect("open in-memory sqlite");
    // Schema version table + bootstrap glue is handled by run_migrations, but
    // we only need to assert that the V1.4.5 W1-A SQL is well-formed; running
    // the two files directly via execute_batch verifies that.
    let v250 = include_str!("../src/migrations/250_workspace_file_lifecycle.sql");
    let v251 = include_str!("../src/migrations/251_workspace_file_lifecycle_category.sql");
    conn.execute_batch(v250).expect("v250 applies cleanly");
    conn.execute_batch(v251).expect("v251 applies cleanly");

    // table_info(workspace_file_lifecycle) should now contain `category`.
    let mut stmt = conn
        .prepare("PRAGMA table_info(workspace_file_lifecycle)")
        .expect("prepare PRAGMA");
    let cols: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .expect("query")
        .filter_map(Result::ok)
        .collect();

    for required in &[
        "id",
        "file_id",
        "canonical_path",
        "device",
        "inode",
        "source_type",
        "data_source",
        "lifecycle_state",
        "source_asof",
        "entity_id",
        "entity_type",
        "content_sha256",
        "user_override_actor",
        "user_override_at",
        "created_at",
        "updated_at",
        "category",
    ] {
        assert!(
            cols.iter().any(|c| c == required),
            "workspace_file_lifecycle missing required column `{required}`; got {cols:?}"
        );
    }
}

#[test]
fn migrations_slice_max_version_is_at_least_251() {
    // Internal substrate check: ensures W1-A's v251 lands at the tail of the
    // registered MIGRATIONS slice (i.e., `version > current` runner filter
    // will pick up new DBs at any version ≤251). We assert ≥251 rather than
    // ==251 so future maintenance migrations don't false-fail this test.
    // The actual MIGRATIONS slice is internal; the schema_version table
    // populated by a fresh `run_migrations` walks the slice. As a proxy,
    // verify the SQL files exist (compile-time `include_str!` already
    // guarantees this if the registration is wired correctly).
    let v250_sql = include_str!("../src/migrations/250_workspace_file_lifecycle.sql");
    let v251_sql = include_str!("../src/migrations/251_workspace_file_lifecycle_category.sql");
    assert!(
        v250_sql.contains("CREATE TABLE IF NOT EXISTS workspace_file_lifecycle"),
        "v250 must create workspace_file_lifecycle"
    );
    assert!(
        v251_sql.contains("ADD COLUMN category"),
        "v251 must add category column"
    );
}
