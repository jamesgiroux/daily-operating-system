use dailyos_lib::abilities::feedback::ClaimVerificationState;
use dailyos_lib::db::claims::{
    ClaimSensitivity, ClaimState, IntelligenceClaim, SurfacingState, TemporalScope,
};
use dailyos_lib::migration_test_api::run_migrations;
use dailyos_lib::services::sensitivity::{
    render_policy_for_sensitivity_name, render_policy_for_surface, render_policy_for_surface_name,
    renderable_claim_text, RedactionAffordance, RenderActor, RenderDecision, RenderPolicyKind,
    RenderSurface,
};
use rusqlite::Connection;

#[test]
fn adr_0108_surface_policy_matrix_is_fail_closed() {
    let user = RenderActor::user("user", Some("user"));
    let other_user = RenderActor::user("user", Some("other-user"));
    let agent = RenderActor::agent("agent:mcp");

    assert_eq!(
        render_policy_for_surface(
            &claim(ClaimSensitivity::Public, "agent:test"),
            RenderSurface::TauriEntityDetail,
            &user,
        ),
        RenderDecision::Render
    );
    assert_eq!(
        render_policy_for_surface(
            &claim(ClaimSensitivity::Public, "agent:test"),
            RenderSurface::McpTool,
            &agent,
        ),
        RenderDecision::Render
    );
    assert_eq!(
        render_policy_for_surface(
            &claim(ClaimSensitivity::Public, "agent:test"),
            RenderSurface::LogStructured,
            &agent,
        ),
        RenderDecision::Drop
    );

    assert_eq!(
        render_policy_for_surface(
            &claim(ClaimSensitivity::Internal, "agent:test"),
            RenderSurface::TauriBriefingPrep,
            &user,
        ),
        RenderDecision::Render
    );
    assert_eq!(
        render_policy_for_surface(
            &claim(ClaimSensitivity::Internal, "agent:test"),
            RenderSurface::McpToolDetail,
            &agent,
        ),
        RenderDecision::Render
    );
    assert_eq!(
        render_policy_for_surface(
            &claim(ClaimSensitivity::Internal, "agent:test"),
            RenderSurface::P2Publication,
            &agent,
        ),
        RenderDecision::Drop
    );

    assert_confidential_click_to_reveal(render_policy_for_surface(
        &claim(ClaimSensitivity::Confidential, "agent:test"),
        RenderSurface::TauriEntityDetail,
        &user,
    ));
    assert!(matches!(
        render_policy_for_surface(
            &claim(ClaimSensitivity::Confidential, "agent:test"),
            RenderSurface::TauriBriefingPrep,
            &user,
        ),
        RenderDecision::RenderRedacted {
            affordance: RedactionAffordance::ConfidentialHidden { .. }
        }
    ));
    assert_eq!(
        render_policy_for_surface(
            &claim(ClaimSensitivity::Confidential, "agent:test"),
            RenderSurface::McpTool,
            &agent,
        ),
        RenderDecision::Drop
    );

    assert_eq!(
        render_policy_for_surface(
            &claim(ClaimSensitivity::UserOnly, "user"),
            RenderSurface::TauriEntityDetail,
            &user,
        ),
        RenderDecision::Render
    );
    assert!(matches!(
        render_policy_for_surface(
            &claim(ClaimSensitivity::UserOnly, "user"),
            RenderSurface::TauriEntityDetail,
            &other_user,
        ),
        RenderDecision::RenderRedacted {
            affordance: RedactionAffordance::UserOnlyHidden { .. }
        }
    ));
    assert_eq!(
        render_policy_for_surface(
            &claim(ClaimSensitivity::UserOnly, "user"),
            RenderSurface::McpTool,
            &agent,
        ),
        RenderDecision::Drop
    );

    assert_eq!(
        render_policy_for_surface_name(
            &claim(ClaimSensitivity::Public, "user"),
            "unknown_surface",
            &user,
        ),
        RenderDecision::Drop
    );
    assert_eq!(
        render_policy_for_sensitivity_name("unknown", "tauri_entity_detail", "user", &user),
        RenderDecision::Drop
    );
}

#[test]
fn renderable_claim_text_never_embeds_confidential_source_text_in_redaction() {
    let user = RenderActor::user("user", Some("user"));
    let source = "Confidential renewal blocker";
    let rendered = renderable_claim_text(
        &claim_with_text(ClaimSensitivity::Confidential, "agent:test", source),
        RenderSurface::TauriEntityDetail,
        &user,
    )
    .expect("confidential first-party UI receives redaction affordance");

    assert_eq!(rendered.text, "Confidential claim hidden");
    assert!(!rendered.text.contains(source));
    assert_eq!(rendered.policy.kind, RenderPolicyKind::Redacted);
}

#[test]
fn sensitivity_reveal_audit_migration_repairs_current_audit_bucket_v143() {
    let conn = Connection::open_in_memory().expect("open in-memory database");
    conn.execute_batch(include_str!(
        "../src/migrations/142_sensitivity_reveal_audit.sql"
    ))
    .expect("migration applies");
    conn.execute_batch(include_str!(
        "../src/migrations/143_sensitivity_reveal_audit_idempotency.sql"
    ))
    .expect("idempotency migration applies");
    setup_migration_runner_state(&conn);
    run_pending_from_v143(&conn);

    let columns = reveal_audit_columns(&conn);

    assert!(columns.contains(&"claim_id".to_string()));
    assert!(columns.contains(&"user_id".to_string()));
    assert!(columns.contains(&"revealed_at".to_string()));
    assert!(columns.contains(&"reveal_action_id".to_string()));
    assert!(!columns.contains(&"audit_bucket".to_string()));
    assert!(!columns.contains(&"reveal_session_id".to_string()));
    assert_reveal_action_id_unique_index(&conn);
    assert_index_missing(&conn, "idx_sensitivity_reveal_audit_audit_bucket");
}

#[test]
fn sensitivity_reveal_audit_migration_repairs_legacy_reveal_session_v143() {
    let conn = Connection::open_in_memory().expect("open in-memory database");
    conn.execute_batch(include_str!(
        "../src/migrations/142_sensitivity_reveal_audit.sql"
    ))
    .expect("base audit migration applies");
    conn.execute_batch(
        "ALTER TABLE sensitivity_reveal_audit
            ADD COLUMN reveal_session_id TEXT NOT NULL DEFAULT '';
         CREATE UNIQUE INDEX IF NOT EXISTS idx_sensitivity_reveal_audit_reveal_session
            ON sensitivity_reveal_audit(claim_id, user_id, reveal_session_id)
            WHERE reveal_session_id != '';",
    )
    .expect("legacy reveal session idempotency migration applies");
    setup_migration_runner_state(&conn);
    run_pending_from_v143(&conn);

    let columns = reveal_audit_columns(&conn);

    assert!(columns.contains(&"claim_id".to_string()));
    assert!(columns.contains(&"user_id".to_string()));
    assert!(columns.contains(&"revealed_at".to_string()));
    assert!(columns.contains(&"reveal_action_id".to_string()));
    assert!(!columns.contains(&"reveal_session_id".to_string()));
    assert!(!columns.contains(&"audit_bucket".to_string()));
    assert_reveal_action_id_unique_index(&conn);
    assert_index_missing(&conn, "idx_sensitivity_reveal_audit_reveal_session");
}

fn setup_migration_runner_state(conn: &Connection) {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        INSERT INTO schema_version (version) VALUES (143);

        CREATE TABLE IF NOT EXISTS meetings (id TEXT PRIMARY KEY);
        CREATE TABLE IF NOT EXISTS meeting_prep (id TEXT PRIMARY KEY);
        CREATE TABLE IF NOT EXISTS meeting_transcripts (id TEXT PRIMARY KEY);
        CREATE TABLE IF NOT EXISTS account_stakeholders (
            id TEXT PRIMARY KEY,
            data_source TEXT
        );
        CREATE TABLE IF NOT EXISTS entity_assessment (
            id TEXT PRIMARY KEY,
            health_json TEXT,
            org_health_json TEXT,
            dimensions_json TEXT,
            success_plan_signals_json TEXT
        );
        CREATE TABLE IF NOT EXISTS entity_quality (
            id TEXT PRIMARY KEY,
            health_score REAL,
            health_trend TEXT,
            coherence_score REAL,
            coherence_flagged INTEGER
        );
        CREATE TABLE IF NOT EXISTS person_relationships (
            id TEXT PRIMARY KEY,
            rationale TEXT
        );
        CREATE TABLE IF NOT EXISTS email_signals (
            id TEXT PRIMARY KEY,
            source TEXT
        );
        CREATE TABLE IF NOT EXISTS entities (
            id TEXT PRIMARY KEY,
            entity_type TEXT NOT NULL DEFAULT 'project'
        );
        CREATE TABLE IF NOT EXISTS entity_members (
            entity_id TEXT NOT NULL,
            person_id TEXT NOT NULL,
            relationship_type TEXT DEFAULT 'associated',
            PRIMARY KEY (entity_id, person_id)
        );",
    )
    .expect("create migration runner fixture state");
}

fn run_pending_from_v143(conn: &Connection) {
    let applied = run_migrations(conn).expect("action token migration applies");
    assert_eq!(applied, 2);
}

fn reveal_audit_columns(conn: &Connection) -> Vec<String> {
    let mut stmt = conn
        .prepare("PRAGMA table_info(sensitivity_reveal_audit)")
        .expect("query audit schema");
    stmt.query_map([], |row| row.get::<_, String>(1))
        .expect("read columns")
        .map(|row| row.expect("column row"))
        .collect()
}

fn assert_reveal_action_id_unique_index(conn: &Connection) {
    let unique_index_count: i64 = conn
        .query_row(
            "SELECT COUNT(*)
             FROM pragma_index_list('sensitivity_reveal_audit')
             WHERE name = 'idx_sensitivity_reveal_audit_action_token'
               AND [unique] = 1",
            [],
            |row| row.get(0),
        )
        .expect("read reveal audit indexes");
    assert_eq!(unique_index_count, 1);

    let index_sql: String = conn
        .query_row(
            "SELECT sql
             FROM sqlite_master
             WHERE type = 'index'
               AND name = 'idx_sensitivity_reveal_audit_action_token'",
            [],
            |row| row.get(0),
        )
        .expect("read reveal audit index SQL");
    assert!(index_sql.contains("(claim_id, user_id, reveal_action_id)"));
    assert!(index_sql.contains("WHERE reveal_action_id != ''"));
}

fn assert_index_missing(conn: &Connection, index_name: &str) {
    let index_count: i64 = conn
        .query_row(
            "SELECT COUNT(*)
             FROM pragma_index_list('sensitivity_reveal_audit')
             WHERE name = ?1",
            [index_name],
            |row| row.get(0),
        )
        .expect("read reveal audit indexes");
    assert_eq!(index_count, 0);
}

fn assert_confidential_click_to_reveal(decision: RenderDecision) {
    assert!(matches!(
        decision,
        RenderDecision::RenderRedacted {
            affordance: RedactionAffordance::ConfidentialClickToReveal {
                audit_required: true,
                ..
            }
        }
    ));
}

fn claim(sensitivity: ClaimSensitivity, actor: &str) -> IntelligenceClaim {
    claim_with_text(sensitivity, actor, "Source text")
}

fn claim_with_text(sensitivity: ClaimSensitivity, actor: &str, text: &str) -> IntelligenceClaim {
    IntelligenceClaim {
        id: "claim-dos412".to_string(),
        subject_ref: r#"{"kind":"account","id":"acct-dos412-example"}"#.to_string(),
        claim_type: "entity_summary".to_string(),
        field_path: None,
        topic_key: None,
        text: text.to_string(),
        dedup_key: "claim-dos412".to_string(),
        item_hash: None,
        actor: actor.to_string(),
        data_source: "test".to_string(),
        source_ref: None,
        source_asof: None,
        observed_at: "2026-05-06T00:00:00Z".to_string(),
        created_at: "2026-05-06T00:00:00Z".to_string(),
        provenance_json: "{}".to_string(),
        metadata_json: None,
        claim_state: ClaimState::Active,
        surfacing_state: SurfacingState::Active,
        demotion_reason: None,
        reactivated_at: None,
        retraction_reason: None,
        expires_at: None,
        superseded_by: None,
        trust_score: None,
        trust_computed_at: None,
        trust_version: None,
        thread_id: None,
        temporal_scope: TemporalScope::State,
        sensitivity,
        verification_state: ClaimVerificationState::Active,
        verification_reason: None,
        needs_user_decision_at: None,
    }
}
