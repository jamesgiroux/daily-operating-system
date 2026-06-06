use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use dailyos_lib::abilities::provenance::source::EntityId;
use dailyos_lib::abilities::provenance::trust::claim_trust_band_from_score;
use dailyos_lib::abilities::registry::McpExposure;
use dailyos_lib::abilities::source_management_ledger::contracts::{
    SourceManagementActionInput, SourceManagementActionKind, SourceManagementActionRequest,
    SourceManagementLedgerInput, SourceManagementLedgerPrivacyProfile,
    SourceManagementLedgerReadRequest,
};
use dailyos_lib::abilities::trust::TrustBand;
use dailyos_lib::abilities::workspace_graph::contracts::{
    WorkspaceGraphInput, WorkspaceGraphPrivacyProfile, WorkspaceGraphReadRequest,
    WorkspaceGraphResponse,
};
#[cfg(feature = "test-harness")]
use dailyos_lib::abilities::{AbilityRegistry, Actor};
#[cfg(feature = "test-harness")]
use dailyos_lib::bridges::mcp::McpAbilityBridge;
#[cfg(feature = "test-harness")]
use dailyos_lib::bridges::tauri::{TauriAbilityBridge, TauriTestInvokeContext};
#[cfg(feature = "test-harness")]
use dailyos_lib::bridges::{BridgeSurface, McpSessionId};
use dailyos_lib::db::claims::{ClaimSensitivity, TemporalScope};
use dailyos_lib::db::{ActionDb, DbAccount};
use dailyos_lib::entity::EntityType;
#[cfg(feature = "test-harness")]
use dailyos_lib::intelligence::provider::ReplayProvider;
use dailyos_lib::services::claims::{
    commit_claim, load_entity_context_claims_active_for_surface, ClaimProposal, CommittedClaim,
};
use dailyos_lib::services::context::{ClaimDismissalSurface, FixedClock, SeedableRng};
#[cfg(feature = "test-harness")]
use dailyos_lib::services::context::{EntityContextClaimReadFuture, EntityContextClaimReadHandle};
use dailyos_lib::services::context::{ExternalClients, ServiceContext, SystemClock, SystemRng};
use dailyos_lib::services::mcp_v2::actor_policy::{ToolGrant, ToolRateLimit};
use dailyos_lib::services::mcp_v2::contracts::{
    McpClientId, McpToolRequestEnvelope, McpToolResult, ScopedName, ToolError,
};
use dailyos_lib::services::mcp_v2::gateway::Gateway;
use dailyos_lib::services::mcp_v2::handlers::registration::register_v147_handlers;
use dailyos_lib::services::mcp_v2::taxonomy::{TaxonomyCatalog, YamlTaxonomyCatalog};
use dailyos_lib::services::workspace_ingestion::contracts::{
    NullExtractor, RejectionReason, WorkspaceFileKind,
};
use dailyos_lib::services::workspace_ingestion::graph::{
    diagnostic_key_for_tests, read_workspace_graph,
};
use dailyos_lib::services::workspace_ingestion::pipeline::{
    file_id_from_identity, EntityRef, IngestError, IngestPipeline, IngestReceipt, IngestRequest,
};
use dailyos_lib::services::workspace_ingestion::registry::WorkspaceSourceRegistry;
use dailyos_lib::services::workspace_ingestion::runs::IngestionMode;
use dailyos_lib::services::workspace_ingestion::signals::WorkspaceSignalEmitter;
use dailyos_lib::services::workspace_ingestion::wiring;
use dailyos_lib::services::{
    source_management_ledger::{apply_source_management_action, read_source_management_ledger},
    trust_recompute::recompute_claim_trust_for_subject,
};
use parking_lot::Mutex;
#[cfg(feature = "test-harness")]
use rusqlite::OpenFlags;
use rusqlite::{params, Connection};
use serde_json::json;

#[test]
fn mcp_placement_handler_registered_but_not_local_stdio_invocable() {
    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    let catalog: Arc<dyn TaxonomyCatalog> =
        Arc::new(YamlTaxonomyCatalog::load_embedded().expect("catalog"));
    let mut gateway = Gateway::new();
    gateway.set_taxonomy(Arc::clone(&catalog));
    register_v147_handlers(
        &mut gateway,
        &catalog,
        runtime.handle().clone(),
        Arc::new(dailyos_lib::signals::propagation::default_engine()),
    )
    .expect("register v2 handlers");
    gateway.seal().expect("registered handlers validate");

    let tool_name = ScopedName::new("dailyos.write.place_document");
    assert!(gateway.registered_tools().any(|name| name == &tool_name));

    let response = gateway.handle_local_stdio_tool_call(
        &McpClientId::new("validation-client"),
        McpToolRequestEnvelope {
            conversation_handle: None,
            tool_name: tool_name.clone(),
            params: serde_json::json!({}),
        },
        &[ToolGrant {
            tool_name: tool_name.clone(),
            scopes_granted: vec![],
            exposure: McpExposure::None,
            rate_limit: ToolRateLimit {
                max_calls: 0,
                window_seconds: 0,
            },
        }],
    );

    assert_eq!(
        response.result,
        McpToolResult::Error {
            error: ToolError::ExposureForbidden {
                tool_name: tool_name.clone()
            }
        },
        "registered placement handler should stay unavailable to local stdio until ADR-0128 expands write exposure"
    );
}

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

    assert_graph_zero_gaps_for_entity(
        &fixture,
        "account",
        "acct-v146-graph",
        1,
        1,
        &[
            &receipt.file_id,
            "Graph Account",
            "graph-note.md",
            "Graph validation context",
        ],
    );
}

#[test]
fn explicit_ingestion_to_claim_provenance_covers_entity_seeded_and_inbox_assignment() {
    let fixture = Fixture::new();
    fixture.seed_account("acct-v146-paths", "Path Matrix Account");

    let entity_seeded_note = fixture.write_account_file(
        "Path Matrix Account",
        "entity-seeded-note.md",
        "Entity seeded graph validation note.",
    );
    let entity_seeded = fixture.ingest_account_note_with_mode(
        &entity_seeded_note,
        EntityRef {
            entity_type: EntityType::Account,
            entity_id: EntityId::new("acct-v146-paths".to_string()),
            entity_name: Some("Path Matrix Account".to_string()),
        },
        IngestionMode::EntitySeeded,
        None,
    );
    assert_eq!(entity_seeded.claim_proposals.len(), 1);
    assert_workspace_claim(
        &fixture.conn,
        &entity_seeded.file_id,
        "acct-v146-paths",
        "workspace_file:entity_doc",
        "entity_doc",
    );
    assert_eq!(
        count(
            &fixture.conn,
            "SELECT COUNT(*)
             FROM document_ingestion_runs
             WHERE run_id = ?1
               AND mode = 'entity_seeded'
               AND status = 'success'
               AND claim_count_produced = 1",
            params![&entity_seeded.ingestion_run_id.0],
        ),
        1
    );
    assert_eq!(
        count(
            &fixture.conn,
            "SELECT COUNT(*)
             FROM document_entity_links
             WHERE file_id = ?1
               AND attribution_source = 'entity_intake'
               AND rejected = 0",
            params![&entity_seeded.file_id],
        ),
        1
    );

    let inbox_note = fixture.write_inbox_file(
        "assigned-note.md",
        "Inbox assignment graph validation note.",
    );
    let process_result = dailyos_lib::command_test_api::process_inbox_file_for_tests(
        ActionDb::from_conn(&fixture.conn),
        &fixture.workspace_root,
        "assigned-note.md",
    )
    .expect("process inbox");
    assert_eq!(process_result["status"], "needs_entity");

    let inbox_file_id = file_id_for_path(&fixture.workspace_root, &inbox_note);
    let inbox_receipt = dailyos_lib::command_test_api::assign_inbox_entity_for_tests(
        ActionDb::from_conn(&fixture.conn),
        &fixture.workspace_root,
        inbox_file_id.clone(),
        "account".to_string(),
        "acct-v146-paths".to_string(),
        "path-matrix-account".to_string(),
        "inbox".to_string(),
    )
    .expect("assign inbox entity");
    assert_eq!(inbox_receipt.file_id, inbox_file_id);
    assert_eq!(inbox_receipt.lifecycle_state_after, "ingested");
    assert_workspace_claim(
        &fixture.conn,
        &inbox_receipt.file_id,
        "acct-v146-paths",
        "workspace_file:inbox",
        "inbox",
    );
    assert_eq!(
        count(
            &fixture.conn,
            "SELECT COUNT(*)
             FROM document_entity_links
             WHERE file_id = ?1
               AND attribution_source = 'user_relink'
               AND rejected = 0",
            params![&inbox_receipt.file_id],
        ),
        1
    );
    assert_eq!(
        count(
            &fixture.conn,
            "SELECT COUNT(*)
             FROM intelligence_claims
             WHERE subject_ref LIKE '%acct-v146-paths%'",
            [],
        ),
        2
    );

    assert_graph_zero_gaps_for_entity(
        &fixture,
        "account",
        "acct-v146-paths",
        2,
        2,
        &[
            &entity_seeded.file_id,
            &inbox_receipt.file_id,
            "Path Matrix Account",
            "entity-seeded-note.md",
            "assigned-note.md",
            "Entity seeded graph validation note",
            "Inbox assignment graph validation note",
        ],
    );
}

#[test]
fn signal_propagation_invalidates_prep() {
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

    let (entity_type, entity_id, source, value): (String, String, String, Option<String>) = fixture
        .conn
        .query_row(
            "SELECT entity_type, entity_id, data_source, value
             FROM signal_events
             WHERE signal_type = 'workspace_file_ingested'
             ORDER BY created_at DESC
             LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("workspace_file_ingested signal");
    assert_eq!(entity_type, "account");
    assert_eq!(entity_id, "acct-v146-signal");
    assert_eq!(source, "workspace_ingestion");

    let payload = value.expect("signal payload");
    assert!(payload.contains(&receipt.file_id));
    assert!(payload.contains(&receipt.ingestion_run_id.0));
    assert!(!payload.contains("Signal Account"));
    assert!(!payload.contains("signal-note.md"));
    assert!(!payload.contains("Signal validation context"));
    assert!(!payload.contains(fixture.workspace_root.to_string_lossy().as_ref()));

    let (entity_type, entity_id, source, value): (String, String, String, Option<String>) = fixture
        .conn
        .query_row(
            "SELECT entity_type, entity_id, data_source, value
             FROM signal_events
             WHERE signal_type = 'entity_intelligence_updated'
             ORDER BY created_at DESC
             LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("entity_intelligence_updated signal");
    assert_eq!(entity_type, "account");
    assert_eq!(entity_id, "acct-v146-signal");
    assert_eq!(source, "workspace_ingestion");

    let payload = value.expect("entity intelligence payload");
    assert!(payload.contains(&receipt.file_id));
    assert!(payload.contains(&receipt.ingestion_run_id.0));
    assert!(payload.contains("workspace_file_ingested"));
    assert!(!payload.contains("Signal Account"));
    assert!(!payload.contains("signal-note.md"));
    assert!(!payload.contains("Signal validation context"));
    assert!(!payload.contains(fixture.workspace_root.to_string_lossy().as_ref()));
}

#[cfg(feature = "test-harness")]
#[tokio::test]
async fn context_inclusion_privacy_parity() {
    let db_dir = tempfile::tempdir().expect("db tempdir");
    let db_path = db_dir.path().join("v146-context-parity.db");
    let fixture = Fixture::new_file_backed(&db_path);
    fixture.seed_account("acct-v146-context", "Context Account");
    let note = fixture.write_account_file(
        "Context Account",
        "context-parity-note.md",
        "Context parity note that must not leak to MCP prompt contexts.",
    );
    let receipt = fixture.ingest_account_note(
        &note,
        EntityRef {
            entity_type: EntityType::Account,
            entity_id: EntityId::new("acct-v146-context".to_string()),
            entity_name: Some("Context Account".to_string()),
        },
        None,
    );
    let user_only_claim_id =
        fixture.claim_id_for_source_and_sensitivity(&receipt.file_id, "user_only");
    let internal_claim_id = fixture.commit_context_claim(
        &receipt.file_id,
        "claim-v146-context-internal",
        "Internal context parity claim visible to MCP.",
        ClaimSensitivity::Internal,
    );

    let registry = AbilityRegistry::from_inventory_checked().expect("ability registry builds");
    let input = json!({
        "schema_version": 2,
        "entity_type": "account",
        "entity_id": "acct-v146-context",
        "depth": "standard",
    });
    let clock = FixedClock::new(
        DateTime::parse_from_rfc3339("2026-05-26T00:00:00Z")
            .expect("fixed time")
            .with_timezone(&Utc),
    );
    let rng = SeedableRng::new(146);
    let external = ExternalClients::default();
    let provider = ReplayProvider::new(std::collections::HashMap::new());

    let tauri_reader_db = Arc::new(Mutex::new(open_readonly_fixture_conn(&db_path)));
    let services = ServiceContext::new_live(&clock, &rng, &external)
        .with_actor("user")
        .with_entity_context_claim_reader(Arc::new(ActionDbClaimReader {
            conn: Arc::clone(&tauri_reader_db),
        }));
    let tauri_bridge = TauriAbilityBridge::new(&registry);
    let tauri_user = tauri_bridge
        .invoke_with_service_context_for_tests(
            &services,
            &provider,
            "get_entity_context",
            input.clone(),
        )
        .await
        .expect("Tauri user context succeeds");
    assert_eq!(
        sorted_entry_ids(&tauri_user.data),
        sorted(vec![internal_claim_id.clone(), user_only_claim_id.clone()]),
        "Tauri user context can see the user-only source claim"
    );

    let tauri_mcp_surface = tauri_bridge
        .invoke_with_service_context_for_tests_as(
            &services,
            &provider,
            TauriTestInvokeContext::new(
                Actor::Agent,
                BridgeSurface::McpTool,
                ClaimDismissalSurface::McpTool,
            ),
            "get_entity_context",
            input.clone(),
        )
        .await
        .expect("Tauri MCP-surface context succeeds");
    assert_eq!(
        sorted_entry_ids(&tauri_mcp_surface.data),
        vec![internal_claim_id.clone()],
        "MCP-surface context must filter user-only claims"
    );

    let mcp_reader_db = Arc::new(Mutex::new(ActionDb::from_connection_for_tests(
        open_readonly_fixture_conn(&db_path),
    )));
    let mcp_bridge = McpAbilityBridge::new_with_action_db_readers(&registry, mcp_reader_db);
    let mcp_response = mcp_bridge
        .invoke_ability(
            McpSessionId::from_uuid(uuid::Uuid::from_u128(146)),
            "get_entity_context",
            input,
            false,
            None,
        )
        .await
        .expect("MCP get_entity_context succeeds");
    assert_eq!(
        sorted_entry_ids(&mcp_response.data),
        vec![internal_claim_id.clone()],
        "actual MCP bridge must match the prompt-safe Tauri MCP-surface view"
    );
    assert_eq!(
        mcp_response.rendered_provenance.surface,
        BridgeSurface::McpTool
    );

    let serialized = serde_json::to_string(&mcp_response.data).expect("mcp response json");
    let workspace_root = fixture.workspace_root.to_string_lossy().to_string();
    for forbidden in [
        "Context Account",
        "context-parity-note.md",
        "Context parity note that must not leak to MCP prompt contexts.",
        workspace_root.as_str(),
    ] {
        assert!(
            !serialized.contains(forbidden),
            "MCP context response leaked raw fixture fragment `{forbidden}`"
        );
    }
}

#[test]
fn trust_band_discipline() {
    let fixture = Fixture::new();
    fixture.seed_account("acct-v146-trust", "Trust Account");
    let now = DateTime::parse_from_rfc3339("2026-05-26T00:00:00Z")
        .expect("fixed trust time")
        .with_timezone(&Utc);

    let fresh_claim_id = fixture.commit_validation_claim(
        "acct-v146-trust",
        "Fresh source-backed note for Trust Account.",
        Some(now - Duration::days(1)),
        now - Duration::days(1),
        Some("workspace_file:v146-trust-fresh"),
    );
    let stale_claim_id = fixture.commit_validation_claim(
        "acct-v146-trust",
        "Stale source-backed note for Trust Account.",
        Some(now - Duration::days(120)),
        now - Duration::days(120),
        Some("workspace_file:v146-trust-stale"),
    );
    let missing_source_asof_claim_id = fixture.commit_validation_claim(
        "acct-v146-trust",
        "Timestamp-unknown note for Trust Account.",
        None,
        now - Duration::days(1),
        Some("workspace_file:v146-trust-missing-source-asof"),
    );

    let clock = FixedClock::new(now);
    let rng = SeedableRng::new(146);
    let external = ExternalClients::default();
    let ctx =
        ServiceContext::new_live(&clock, &rng, &external).with_actor("system:v146_validation");
    let report = recompute_claim_trust_for_subject(
        &ctx,
        &ActionDb::from_conn(&fixture.conn),
        "account",
        "acct-v146-trust",
    )
    .expect("trust recompute");
    assert_eq!(report.claims_seen, 3);
    assert_eq!(report.claims_updated, 3);
    assert_eq!(report.claims_skipped, 0);

    assert_eq!(
        fixture.claim_trust_band(&fresh_claim_id),
        TrustBand::LikelyCurrent,
        "fresh source_asof should survive recompute as likely current"
    );
    assert!(
        matches!(
            fixture.claim_trust_band(&stale_claim_id),
            TrustBand::UseWithCaution | TrustBand::NeedsVerification
        ),
        "stale source_asof must not render as likely current"
    );
    assert!(
        matches!(
            fixture.claim_trust_band(&missing_source_asof_claim_id),
            TrustBand::UseWithCaution | TrustBand::NeedsVerification
        ),
        "missing source_asof must not render as likely current"
    );
}

#[test]
fn lifecycle_actions_and_user_correction_round_trip() {
    let fixture = Fixture::new();
    fixture.seed_account("acct-v146-life", "Lifecycle Account");
    let relink_note = fixture.write_account_file(
        "Lifecycle Account",
        "lifecycle-relink.md",
        "Lifecycle relink validation note.",
    );
    let relink_receipt = fixture.ingest_account_note(
        &relink_note,
        EntityRef {
            entity_type: EntityType::Account,
            entity_id: EntityId::new("acct-v146-life".to_string()),
            entity_name: Some("Lifecycle Account".to_string()),
        },
        None,
    );
    assert_eq!(
        context_claim_count(&fixture, "acct-v146-life"),
        1,
        "ingested workspace claims should be visible before policy actions"
    );

    let signal_engine = Arc::new(dailyos_lib::signals::propagation::default_engine());
    let diagnostic_key = diagnostic_key_for_tests("v146-lifecycle");
    let relink_key = fixture.source_key_for_lifecycle_state(&diagnostic_key, "ingested");

    let relink = fixture.apply_source_action(
        Arc::clone(&signal_engine),
        &diagnostic_key,
        &relink_key,
        SourceManagementActionKind::Relink,
    );
    assert_eq!(relink.status, "linked");
    assert_eq!(
        fixture.source_lifecycle_state(&relink_receipt.file_id),
        "ingested"
    );
    assert_eq!(
        count(
            &fixture.conn,
            "SELECT COUNT(*)
             FROM document_entity_links
             WHERE file_id = ?1
               AND user_override_actor = 'user:v146_validation'",
            params![&relink_receipt.file_id],
        ),
        1,
        "relink action should record a user override through the link service"
    );

    let quarantine = fixture.apply_source_action(
        Arc::clone(&signal_engine),
        &diagnostic_key,
        &relink_key,
        SourceManagementActionKind::Quarantine,
    );
    assert_eq!(quarantine.lifecycle_state, "quarantined");
    assert_eq!(
        context_claim_count(&fixture, "acct-v146-life"),
        0,
        "quarantined workspace source should stop feeding entity context"
    );

    let policy_note = fixture.write_account_file(
        "Lifecycle Account",
        "lifecycle-policy.md",
        "Lifecycle policy validation note.",
    );
    let policy_receipt = fixture.ingest_account_note(
        &policy_note,
        EntityRef {
            entity_type: EntityType::Account,
            entity_id: EntityId::new("acct-v146-life".to_string()),
            entity_name: Some("Lifecycle Account".to_string()),
        },
        None,
    );
    assert_eq!(context_claim_count(&fixture, "acct-v146-life"), 1);
    let policy_key = fixture.source_key_for_lifecycle_state(&diagnostic_key, "ingested");

    for (action, expected_state) in [
        (SourceManagementActionKind::Ignore, "ignored"),
        (SourceManagementActionKind::Scratchpad, "scratchpad"),
        (SourceManagementActionKind::Archive, "archived"),
        (SourceManagementActionKind::Delete, "deleted"),
    ] {
        let receipt = fixture.apply_source_action(
            Arc::clone(&signal_engine),
            &diagnostic_key,
            &policy_key,
            action,
        );
        assert_eq!(receipt.lifecycle_state, expected_state);
        assert_eq!(
            fixture.source_lifecycle_state(&policy_receipt.file_id),
            expected_state
        );
        assert_eq!(
            context_claim_count(&fixture, "acct-v146-life"),
            0,
            "{expected_state} workspace source should stay out of entity context"
        );
    }

    let ledger = read_source_management_ledger(
        &fixture.conn,
        SourceManagementLedgerReadRequest {
            input: SourceManagementLedgerInput {
                schema_version: 1,
                entity_type: "account".to_string(),
                entity_id: "acct-v146-life".to_string(),
                cursor: None,
                page_size: 25,
            },
            privacy_profile: SourceManagementLedgerPrivacyProfile::SurfaceClient,
        },
        &diagnostic_key,
    )
    .expect("source ledger after lifecycle actions");
    let states = ledger
        .sources
        .iter()
        .map(|source| source.lifecycle_state.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert!(states.contains("quarantined"));
    assert!(states.contains("deleted"));

    let policy_signal_count = count(
        &fixture.conn,
        "SELECT COUNT(*)
         FROM signal_events
         WHERE signal_type = 'workspace_source_policy_changed'
           AND entity_type = 'account'
           AND entity_id = 'acct-v146-life'",
        [],
    );
    assert!(
        (1..=4).contains(&policy_signal_count),
        "policy actions should emit at least one invalidating signal; rapid actions may coalesce"
    );
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

    let non_utf8_path = Path::new(OsStr::from_bytes(b"non-utf8-\xff.md"));
    assert_rejected(
        &fixture.workspace_root,
        non_utf8_path,
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
            "{table} should stay empty for registry-level rejections"
        );
    }

    let explicit = Fixture::new();
    // dos7-allowed: v146-validation-fixture
    std::fs::write(explicit.workspace_root.join("oversized.md"), "012345678")
        .expect("oversized write");
    std::fs::write(
        explicit.workspace_root.join("invalid-utf8.md"),
        [0xff, 0xfe],
    )
    .expect("invalid utf8 write");
    std::fs::write(
        explicit.workspace_root.join("nul-content.md"),
        b"safe\0unsafe",
    )
    .expect("nul content write");

    assert_pipeline_rejected(
        &explicit,
        Path::new("oversized.md"),
        RejectionReason::FileTooLarge,
        8,
    );
    assert_pipeline_rejected(
        &explicit,
        Path::new("invalid-utf8.md"),
        RejectionReason::UnsupportedFormat,
        1024,
    );
    assert_pipeline_rejected(
        &explicit,
        Path::new("nul-content.md"),
        RejectionReason::UnsupportedFormat,
        1024,
    );

    assert_eq!(
        count(
            &explicit.conn,
            "SELECT COUNT(*)
             FROM workspace_file_lifecycle
             WHERE lifecycle_state = 'rejected'",
            [],
        ),
        3,
        "pipeline-level rejections should leave only rejected lifecycle audit rows",
    );
    assert_rejection_signal_reasons(
        &explicit.conn,
        &["file_too_large", "unsupported_format", "unsupported_format"],
    );
    for table in [
        "document_ingestion_runs",
        "document_entity_links",
        "content_index",
        "content_embeddings",
        "intelligence_claims",
    ] {
        let sql = format!("SELECT COUNT(*) FROM {table}");
        assert_eq!(
            count(&explicit.conn, &sql, []),
            0,
            "{table} should stay empty for pipeline-level rejections"
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
        Self::with_connection(conn)
    }

    #[cfg(feature = "test-harness")]
    fn new_file_backed(db_path: &Path) -> Self {
        let conn = Connection::open(db_path).expect("file-backed sqlite");
        Self::with_connection(conn)
    }

    fn with_connection(conn: Connection) -> Self {
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

    fn write_inbox_file(&self, filename: &str, content: &str) -> PathBuf {
        let dir = self.workspace_root.join("_inbox");
        std::fs::create_dir_all(&dir).expect("inbox dir");
        let path = dir.join(filename);
        std::fs::write(&path, content).expect("write inbox note");
        path
    }

    fn ingest_account_note(
        &self,
        path: &Path,
        entity: EntityRef,
        prep_queue: Option<Arc<Mutex<Vec<String>>>>,
    ) -> IngestReceipt {
        self.ingest_account_note_with_mode(path, entity, IngestionMode::Realtime, prep_queue)
    }

    fn ingest_account_note_with_mode(
        &self,
        path: &Path,
        entity: EntityRef,
        mode: IngestionMode,
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
            mode,
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

    #[cfg(feature = "test-harness")]
    fn claim_id_for_source_and_sensitivity(&self, file_id: &str, sensitivity: &str) -> String {
        self.conn
            .query_row(
                "SELECT id
                 FROM intelligence_claims
                 WHERE source_ref = ?1
                   AND sensitivity = ?2
                 ORDER BY created_at DESC, id DESC
                 LIMIT 1",
                params![format!("workspace_file:{file_id}"), sensitivity],
                |row| row.get(0),
            )
            .expect("claim id for source and sensitivity")
    }

    #[cfg(feature = "test-harness")]
    fn commit_context_claim(
        &self,
        file_id: &str,
        claim_id: &str,
        text: &str,
        sensitivity: ClaimSensitivity,
    ) -> String {
        let source_asof: String = self
            .conn
            .query_row(
                "SELECT source_asof
                 FROM workspace_file_lifecycle
                 WHERE file_id = ?1",
                [file_id],
                |row| row.get(0),
            )
            .expect("workspace source_asof");
        let clock = FixedClock::new(
            DateTime::parse_from_rfc3339("2026-05-26T00:00:00Z")
                .expect("fixed time")
                .with_timezone(&Utc),
        );
        let rng = SeedableRng::new(476);
        let external = ExternalClients::default();
        let ctx =
            ServiceContext::new_live(&clock, &rng, &external).with_actor("system:v146_validation");
        let committed = commit_claim(
            &ctx,
            ActionDb::from_conn(&self.conn),
            ClaimProposal {
                id: None,
                expected_claim_version: None,
                subject_ref: json!({
                    "kind": "account",
                    "id": "acct-v146-context",
                })
                .to_string(),
                claim_type: "user_note".to_string(),
                field_path: Some("workspace.context_parity".to_string()),
                topic_key: None,
                text: text.to_string(),
                actor: "system:v146_validation".to_string(),
                data_source: "workspace_file:entity_doc".to_string(),
                source_ref: Some(format!("workspace_file:{file_id}")),
                source_asof: Some(source_asof.clone()),
                observed_at: source_asof,
                provenance_json: "{}".to_string(),
                metadata_json: Some(
                    json!({
                        "producer": "v146_validation",
                        "validation_claim_key": claim_id,
                        "workspace_file_id": file_id,
                        "workspace_file_kind": "entity_doc",
                    })
                    .to_string(),
                ),
                thread_id: None,
                temporal_scope: Some(TemporalScope::State),
                sensitivity: Some(sensitivity),
                supersedes: None,
                tombstone: None,
            },
        )
        .expect("commit context parity claim");

        match committed {
            CommittedClaim::Inserted { claim } => claim.id,
            other => panic!("expected inserted context parity claim, got {other:?}"),
        }
    }

    fn commit_validation_claim(
        &self,
        account_id: &str,
        text: &str,
        source_asof: Option<DateTime<Utc>>,
        observed_at: DateTime<Utc>,
        source_ref: Option<&str>,
    ) -> String {
        let clock = FixedClock::new(
            DateTime::parse_from_rfc3339("2026-05-26T00:00:00Z")
                .expect("fixed time")
                .with_timezone(&Utc),
        );
        let rng = SeedableRng::new(476);
        let external = ExternalClients::default();
        let ctx =
            ServiceContext::new_live(&clock, &rng, &external).with_actor("system:v146_validation");
        let committed = commit_claim(
            &ctx,
            ActionDb::from_conn(&self.conn),
            ClaimProposal {
                id: None,
                expected_claim_version: None,
                subject_ref: json!({
                    "kind": "account",
                    "id": account_id,
                })
                .to_string(),
                claim_type: "user_note".to_string(),
                field_path: None,
                topic_key: None,
                text: text.to_string(),
                actor: "system:v146_validation".to_string(),
                data_source: "workspace_file:entity_doc".to_string(),
                source_ref: source_ref.map(str::to_string),
                source_asof: source_asof.map(|value| value.to_rfc3339()),
                observed_at: observed_at.to_rfc3339(),
                provenance_json: "{}".to_string(),
                metadata_json: Some(
                    json!({
                        "producer": "v146_validation",
                        "internal_consistency": 1.0,
                    })
                    .to_string(),
                ),
                thread_id: None,
                temporal_scope: Some(TemporalScope::State),
                sensitivity: Some(ClaimSensitivity::Internal),
                supersedes: None,
                tombstone: None,
            },
        )
        .expect("commit validation claim");

        match committed {
            CommittedClaim::Inserted { claim } => claim.id,
            other => panic!("expected inserted validation claim, got {other:?}"),
        }
    }

    fn claim_trust_band(&self, claim_id: &str) -> TrustBand {
        let trust_score: Option<f64> = self
            .conn
            .query_row(
                "SELECT trust_score FROM intelligence_claims WHERE id = ?1",
                [claim_id],
                |row| row.get(0),
            )
            .expect("claim trust score");
        claim_trust_band_from_score(trust_score)
    }

    fn source_lifecycle_state(&self, file_id: &str) -> String {
        self.conn
            .query_row(
                "SELECT lifecycle_state
                 FROM workspace_file_lifecycle
                 WHERE file_id = ?1",
                [file_id],
                |row| row.get(0),
            )
            .expect("source lifecycle state")
    }

    fn source_key_for_lifecycle_state(
        &self,
        diagnostic_key: &dailyos_lib::services::workspace_ingestion::graph::WorkspaceGraphDiagnosticKey,
        lifecycle_state: &str,
    ) -> String {
        let ledger = read_source_management_ledger(
            &self.conn,
            SourceManagementLedgerReadRequest {
                input: SourceManagementLedgerInput {
                    schema_version: 1,
                    entity_type: "account".to_string(),
                    entity_id: "acct-v146-life".to_string(),
                    cursor: None,
                    page_size: 25,
                },
                privacy_profile: SourceManagementLedgerPrivacyProfile::SurfaceClient,
            },
            diagnostic_key,
        )
        .expect("source management ledger");
        let matches = ledger
            .sources
            .iter()
            .filter(|source| source.lifecycle_state == lifecycle_state)
            .map(|source| source.source_key.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            matches.len(),
            1,
            "expected exactly one source in lifecycle state {lifecycle_state}, got {:?}",
            ledger
                .sources
                .iter()
                .map(|source| source.lifecycle_state.as_str())
                .collect::<Vec<_>>()
        );
        matches[0].clone()
    }

    fn apply_source_action(
        &self,
        signal_engine: Arc<dailyos_lib::signals::propagation::PropagationEngine>,
        diagnostic_key: &dailyos_lib::services::workspace_ingestion::graph::WorkspaceGraphDiagnosticKey,
        source_key: &str,
        action: SourceManagementActionKind,
    ) -> dailyos_lib::abilities::source_management_ledger::contracts::SourceManagementActionReceipt
    {
        let clock = SystemClock;
        let rng = SystemRng;
        let external = ExternalClients::default();
        let ctx =
            ServiceContext::new_live(&clock, &rng, &external).with_actor("user:v146_validation");
        apply_source_management_action(
            &ctx,
            &ActionDb::from_conn(&self.conn),
            self.workspace_root.clone(),
            Some(signal_engine),
            SourceManagementActionRequest {
                input: SourceManagementActionInput {
                    schema_version: 1,
                    entity_type: "account".to_string(),
                    entity_id: "acct-v146-life".to_string(),
                    source_key: source_key.to_string(),
                    action,
                    reason: Some("user_requested".to_string()),
                },
                actor_id: "user:v146_validation".to_string(),
            },
            diagnostic_key,
        )
        .expect("source management action")
    }
}

fn context_claim_count(fixture: &Fixture, account_id: &str) -> usize {
    load_entity_context_claims_active_for_surface(
        ActionDb::from_conn(&fixture.conn),
        "account",
        account_id,
        1,
        ClaimDismissalSurface::TauriEntityDetail.as_str(),
    )
    .expect("entity context claims")
    .len()
}

#[cfg(feature = "test-harness")]
struct ActionDbClaimReader {
    conn: Arc<Mutex<Connection>>,
}

#[cfg(feature = "test-harness")]
impl EntityContextClaimReadHandle for ActionDbClaimReader {
    fn read_entity_context_claims<'a>(
        &'a self,
        entity_type: String,
        entity_id: String,
        surface: ClaimDismissalSurface,
        depth: usize,
    ) -> EntityContextClaimReadFuture<'a> {
        let result = {
            let conn = self.conn.lock();
            load_entity_context_claims_active_for_surface(
                ActionDb::from_conn(&conn),
                &entity_type,
                &entity_id,
                depth,
                surface.as_str(),
            )
            .map_err(|error| format!("entity context claim read failed: {error}"))
        };
        Box::pin(std::future::ready(result))
    }
}

#[cfg(feature = "test-harness")]
fn open_readonly_fixture_conn(db_path: &Path) -> Connection {
    let conn = Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .expect("open readonly fixture DB");
    conn.execute_batch(
        "PRAGMA busy_timeout = 5000; PRAGMA foreign_keys = ON; PRAGMA query_only = ON;",
    )
    .expect("configure readonly fixture DB");
    conn
}

#[cfg(feature = "test-harness")]
fn sorted_entry_ids(value: &serde_json::Value) -> Vec<String> {
    sorted(
        value["entries"]
            .as_array()
            .expect("entries array")
            .iter()
            .map(|entry| entry["id"].as_str().expect("entry id").to_string())
            .collect(),
    )
}

#[cfg(feature = "test-harness")]
fn sorted(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values
}

fn assert_workspace_claim(
    conn: &Connection,
    file_id: &str,
    entity_id: &str,
    expected_data_source: &str,
    expected_kind: &str,
) {
    let source_ref = format!("workspace_file:{file_id}");
    let claim = conn
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
    assert_eq!(subject_ref["id"], entity_id);
    assert_eq!(claim.data_source, expected_data_source);
    assert_eq!(claim.source_ref, source_ref);
    assert!(!claim.source_asof.trim().is_empty());
    assert_eq!(claim.sensitivity, "user_only");

    let metadata: serde_json::Value =
        serde_json::from_str(&claim.metadata_json).expect("metadata json");
    assert_eq!(metadata["producer"], "workspace_ingestion");
    assert_eq!(metadata["workspace_file_id"], file_id);
    assert_eq!(metadata["workspace_file_kind"], expected_kind);
    assert!(metadata["ingestion_run_id"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
    assert!(metadata["document_entity_link_id"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
}

fn assert_graph_zero_gaps_for_entity(
    fixture: &Fixture,
    entity_type: &str,
    entity_id: &str,
    expected_file_links: usize,
    expected_claim_total: u32,
    forbidden_fragments: &[&str],
) {
    let response = read_workspace_graph(
        &fixture.conn,
        WorkspaceGraphReadRequest {
            input: WorkspaceGraphInput {
                schema_version: 1,
                entity_filter: None,
                category_filter: None,
                cursor: None,
                if_none_match: None,
                include_entity_names: false,
                page_size: 50,
            },
            privacy_profile: WorkspaceGraphPrivacyProfile::FirstParty,
        },
        &diagnostic_key_for_tests("v146-graph-audit"),
    )
    .expect("workspace graph read");
    let WorkspaceGraphResponse::Projection(projection) = response else {
        panic!("expected workspace graph projection");
    };
    assert!(
        projection.audit.gaps.is_empty(),
        "workspace graph audit gaps: {:?}",
        projection.audit.gaps
    );
    let entity = projection
        .projection
        .entities
        .iter()
        .find(|entity| entity.entity_type == entity_type && entity.entity_id == entity_id)
        .expect("graph entity");
    assert_eq!(entity.file_links.len(), expected_file_links);
    assert_eq!(entity.claim_summary.total, expected_claim_total);

    let serialized = serde_json::to_string(&projection).expect("projection json");
    for fragment in forbidden_fragments {
        assert!(
            !serialized.contains(fragment),
            "graph projection should not leak raw fixture fragment `{fragment}`"
        );
    }
}

fn file_id_for_path(workspace_root: &Path, path: &Path) -> String {
    let (_file, identity) =
        WorkspaceSourceRegistry::open_validated(workspace_root, path).expect("open validated");
    file_id_from_identity(&identity, workspace_root).expect("file id")
}

fn assert_rejected(workspace_root: &Path, path: &Path, expected: RejectionReason) {
    let err = WorkspaceSourceRegistry::open_validated(workspace_root, path)
        .expect_err("path should be rejected");
    assert_eq!(err, expected);
}

fn assert_pipeline_rejected(
    fixture: &Fixture,
    path: &Path,
    expected: RejectionReason,
    max_file_bytes: u64,
) {
    let (file, identity) = WorkspaceSourceRegistry::open_validated(&fixture.workspace_root, path)
        .expect("open validated before pipeline rejection");
    let source_asof: DateTime<Utc> = identity
        .canonical_path
        .metadata()
        .and_then(|metadata| metadata.modified())
        .expect("source asof")
        .into();
    let file_id = file_id_from_identity(&identity, &fixture.workspace_root).expect("file id");
    let request = IngestRequest {
        file,
        identity,
        file_id,
        source_asof,
        source_type: WorkspaceFileKind::EntityDoc,
        entity: None,
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
    let db = ActionDb::from_conn(&fixture.conn);
    let pipeline = IngestPipeline::new(
        Box::new(NullExtractor),
        Box::new(WorkspaceSignalEmitter),
        max_file_bytes,
        "workspace-validation-null-extractor-v1",
        fixture.workspace_root.clone(),
    );
    let err = pipeline
        .run(&ctx, db, request)
        .expect_err("ingest should reject");
    match err {
        IngestError::Rejected(reason) => assert_eq!(reason, expected),
        other => panic!("unexpected ingest error: {other:?}"),
    }
}

fn assert_rejection_signal_reasons(conn: &Connection, expected: &[&str]) {
    let mut stmt = conn
        .prepare(
            "SELECT value
             FROM signal_events
             WHERE signal_type = 'workspace_file_rejected'",
        )
        .expect("signal query");
    let mut reasons = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .expect("signal rows")
        .map(|row| {
            let payload = row.expect("signal payload");
            assert!(!payload.contains("oversized.md"));
            assert!(!payload.contains("invalid-utf8.md"));
            assert!(!payload.contains("nul-content.md"));
            let value: serde_json::Value =
                serde_json::from_str(&payload).expect("signal json payload");
            value["reason_code"]
                .as_str()
                .expect("reason_code")
                .to_string()
        })
        .collect::<Vec<_>>();
    reasons.sort();
    let mut expected = expected
        .iter()
        .map(|reason| reason.to_string())
        .collect::<Vec<_>>();
    expected.sort();
    assert_eq!(reasons, expected);
}

fn count<P>(conn: &Connection, sql: &str, params: P) -> i64
where
    P: rusqlite::Params,
{
    conn.query_row(sql, params, |row| row.get(0))
        .expect("count query")
}
