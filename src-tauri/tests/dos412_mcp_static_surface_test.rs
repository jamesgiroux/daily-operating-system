use dailyos_lib::db::claims::ClaimSensitivity;
use dailyos_lib::db::ActionDb;
use dailyos_lib::services::sensitivity::{
    render_mcp_static_json_for_surface, render_mcp_static_text_for_surface,
    reveal_claim_text_for_tauri, McpStaticTextClass, RenderActor, RenderPolicyKind, RenderSurface,
    RenderableMcpClaimText, RenderableMcpStaticText, RenderableMcpText,
};
use rusqlite::{params, Connection};
use serde_json::json;

const CLAIMS_SCHEMA: &str = r#"
CREATE TABLE intelligence_claims (
    id TEXT PRIMARY KEY,
    subject_ref TEXT NOT NULL,
    claim_type TEXT NOT NULL,
    field_path TEXT,
    topic_key TEXT,
    text TEXT NOT NULL,
    dedup_key TEXT NOT NULL,
    item_hash TEXT,
    actor TEXT NOT NULL,
    data_source TEXT NOT NULL,
    source_ref TEXT,
    source_asof TEXT,
    observed_at TEXT NOT NULL,
    created_at TEXT NOT NULL,
    provenance_json TEXT NOT NULL,
    metadata_json TEXT,
    claim_state TEXT NOT NULL,
    surfacing_state TEXT NOT NULL,
    demotion_reason TEXT,
    reactivated_at TEXT,
    retraction_reason TEXT,
    expires_at TEXT,
    superseded_by TEXT,
    trust_score REAL,
    trust_computed_at TEXT,
    trust_version INTEGER,
    thread_id TEXT,
    temporal_scope TEXT NOT NULL,
    sensitivity TEXT NOT NULL,
    verification_state TEXT NOT NULL,
    verification_reason TEXT,
    needs_user_decision_at TEXT
);

CREATE TABLE sensitivity_reveal_audit (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    claim_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    revealed_at TEXT NOT NULL,
    audit_bucket TEXT NOT NULL DEFAULT ''
);

CREATE UNIQUE INDEX idx_sensitivity_reveal_audit_audit_bucket
    ON sensitivity_reveal_audit(claim_id, user_id, audit_bucket)
    WHERE audit_bucket != '';
"#;

#[test]
fn claim_carriers_apply_mcp_policy_to_exact_projection_text() {
    let conn = fixture_conn();
    let db = ActionDb::from_conn(&conn);
    let values = seed_policy_claims(
        &conn,
        "account",
        "acct-example",
        "entity_summary",
        "entity summary",
    );

    assert_eq!(
        render_claim_text(db, &values.public),
        Some("entity summary public example.com".to_string())
    );
    assert_eq!(
        render_claim_text(db, &values.internal),
        Some("entity summary internal example.com".to_string())
    );
    assert_eq!(render_claim_text(db, &values.confidential), None);
    assert_eq!(render_claim_text(db, &values.user_only), None);
}

#[test]
fn get_briefing_static_json_keeps_allowlisted_metadata_and_drops_unbacked_text() {
    let conn = fixture_conn();
    let db = ActionDb::from_conn(&conn);
    let values = seed_policy_claims(
        &conn,
        "account",
        "acct-example",
        "entity_summary",
        "briefing",
    );

    let rendered = render_mcp_static_json_for_surface(
        db,
        json!({
            "schedule": [{
                "title": "Roadmap review example.com",
                "start_time": "2026-05-06T10:00:00Z",
                "summary": values.confidential.text
            }],
            "actions": [{
                "title": values.internal.text,
                "priority": "P1"
            }],
            "emails": [{
                "subject": values.public.text,
                "snippet": values.confidential.text
            }],
            "briefing": {
                "narrative": values.public.text
            }
        }),
        &test_briefing_static_text_class,
    )
    .expect("briefing JSON renders with disallowed leaves dropped");

    let serialized = rendered.to_string();
    assert!(serialized.contains("Roadmap review example.com"));
    assert!(serialized.contains("2026-05-06T10:00:00Z"));
    assert!(serialized.contains("P1"));
    assert!(!serialized.contains("briefing public example.com"));
    assert!(!serialized.contains("briefing internal example.com"));
    assert!(!serialized.contains("briefing confidential example.com"));
}

#[test]
fn query_entity_claim_summary_uses_claim_metadata_not_string_identity() {
    let conn = fixture_conn();
    let db = ActionDb::from_conn(&conn);
    let summaries = seed_policy_claims(
        &conn,
        "account",
        "acct-example",
        "entity_summary",
        "entity summary",
    );
    let actions = seed_policy_claims(&conn, "account", "acct-example", "open_loop", "action item");
    let open_actions = [
        render_claim_text(db, &actions.public),
        render_claim_text(db, &actions.internal),
        render_claim_text(db, &actions.confidential),
        render_claim_text(db, &actions.user_only),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();

    let response = json!({
        "intelligence_summary": render_claim_text(db, &summaries.public),
        "open_actions": open_actions
    });

    let serialized = response.to_string();
    assert!(serialized.contains("entity summary public example.com"));
    assert!(serialized.contains("action item public example.com"));
    assert!(serialized.contains("action item internal example.com"));
    assert!(!serialized.contains("action item confidential example.com"));
    assert!(!serialized.contains("action item user only example.com"));
}

#[test]
fn search_meetings_static_fields_allow_metadata_and_drop_generated_snippets() {
    let conn = fixture_conn();
    let db = ActionDb::from_conn(&conn);
    let transcript = seed_policy_claims(
        &conn,
        "meeting",
        "mtg-example",
        "meeting_event_note",
        "transcript",
    );
    let prep = seed_policy_claims(&conn, "account", "acct-example", "entity_summary", "prep");

    let rendered = json!({
        "title": render_static_text(db, "Pipeline review example.com", McpStaticTextClass::MeetingTitle),
        "account_name": render_static_text(db, "Example Account example.com", McpStaticTextClass::AccountName),
        "summary": render_static_text(db, transcript.public.text, McpStaticTextClass::MeetingSummary),
        "prep": render_static_text(db, prep.public.text, McpStaticTextClass::MeetingPrepSummary),
    });

    let serialized = rendered.to_string();
    assert!(serialized.contains("Pipeline review example.com"));
    assert!(serialized.contains("Example Account example.com"));
    assert!(!serialized.contains("transcript public example.com"));
    assert!(!serialized.contains("prep public example.com"));
}

#[test]
fn search_content_static_chunks_drop_without_claim_metadata() {
    let conn = fixture_conn();
    let db = ActionDb::from_conn(&conn);
    let chunks = seed_policy_claims(&conn, "account", "acct-example", "entity_risk", "chunk");

    assert_eq!(
        render_static_text(db, "renewal-notes.md", McpStaticTextClass::ContentFilename),
        Some("renewal-notes.md".to_string())
    );
    assert_eq!(
        render_static_text(db, chunks.public.text, McpStaticTextClass::ContentChunk),
        None
    );
    assert_eq!(
        render_static_text(db, chunks.internal.text, McpStaticTextClass::ContentChunk),
        None
    );
}

#[test]
fn paraphrased_confidential_claim_text_in_static_mcp_dto_is_dropped() {
    let conn = fixture_conn();
    let db = ActionDb::from_conn(&conn);
    let chunks = seed_policy_claims(
        &conn,
        "account",
        "acct-example",
        "entity_risk",
        "renewal blocker",
    );
    let paraphrased_private_snippet = "renewal blocker private issue summarized example.com";

    assert_ne!(paraphrased_private_snippet, chunks.confidential.text);
    assert_eq!(
        render_static_text(
            db,
            paraphrased_private_snippet,
            McpStaticTextClass::ContentChunk
        ),
        None
    );
}

#[test]
fn static_claim_carrier_drops_forged_dto_text() {
    let conn = fixture_conn();
    let db = ActionDb::from_conn(&conn);
    let values = seed_policy_claims(
        &conn,
        "account",
        "acct-example",
        "entity_summary",
        "static stored",
    );

    assert_eq!(
        render_claim_text_with_dto_text(
            db,
            &values.internal,
            "forged static renderer text example.com"
        ),
        None
    );
}

#[test]
fn static_claim_carrier_drops_withdrawn_claim() {
    let conn = fixture_conn();
    let db = ActionDb::from_conn(&conn);
    let values = seed_policy_claims(
        &conn,
        "account",
        "acct-example",
        "entity_summary",
        "withdrawn static",
    );
    update_claim_lifecycle(&conn, &values.internal.id, "withdrawn", "dormant");

    assert_eq!(render_claim_text(db, &values.internal), None);
}

#[test]
fn static_claim_carrier_drops_inactive_surfacing_state() {
    let conn = fixture_conn();
    let db = ActionDb::from_conn(&conn);
    let values = seed_policy_claims(
        &conn,
        "account",
        "acct-example",
        "entity_summary",
        "inactive surface",
    );
    update_claim_lifecycle(&conn, &values.internal.id, "active", "dormant");

    assert_eq!(render_claim_text(db, &values.internal), None);
}

#[test]
fn static_claim_carrier_renders_matching_stored_text() {
    let conn = fixture_conn();
    let db = ActionDb::from_conn(&conn);
    let values = seed_policy_claims(
        &conn,
        "account",
        "acct-example",
        "entity_summary",
        "matching static",
    );

    assert_eq!(
        render_claim_text(db, &values.internal),
        Some("matching static internal example.com".to_string())
    );
}

#[test]
fn withdrawn_claim_cannot_reveal_through_tauri() {
    let conn = fixture_conn();
    let db = ActionDb::from_conn(&conn);
    let values = seed_policy_claims(
        &conn,
        "account",
        "acct-example",
        "entity_summary",
        "withdrawn reveal",
    );
    update_claim_lifecycle(&conn, &values.confidential.id, "withdrawn", "dormant");

    let result = reveal_claim_text_for_tauri(
        db,
        &values.confidential.id,
        RenderSurface::TauriEntityDetail,
        &RenderActor::user("user", Some("user")),
    );

    assert!(result.is_err(), "withdrawn claims must fail closed");
    assert_eq!(reveal_audit_count(&conn), 0);
}

#[test]
fn dormant_claim_cannot_reveal_through_tauri() {
    let conn = fixture_conn();
    let db = ActionDb::from_conn(&conn);
    let values = seed_policy_claims(
        &conn,
        "account",
        "acct-example",
        "entity_summary",
        "dormant reveal",
    );
    update_claim_lifecycle(&conn, &values.confidential.id, "active", "dormant");

    let result = reveal_claim_text_for_tauri(
        db,
        &values.confidential.id,
        RenderSurface::TauriEntityDetail,
        &RenderActor::user("user", Some("user")),
    );

    assert!(result.is_err(), "dormant claims must fail closed");
    assert_eq!(reveal_audit_count(&conn), 0);
}

#[test]
fn active_surfaced_confidential_claim_reveals_through_tauri() {
    let conn = fixture_conn();
    let db = ActionDb::from_conn(&conn);
    let values = seed_policy_claims(
        &conn,
        "account",
        "acct-example",
        "entity_summary",
        "active reveal",
    );

    let rendered = reveal_claim_text_for_tauri(
        db,
        &values.confidential.id,
        RenderSurface::TauriEntityDetail,
        &RenderActor::user("user", Some("user")),
    )
    .expect("active surfaced confidential claim reveals after click");

    assert_eq!(rendered.text, "active reveal confidential example.com");
    assert_eq!(rendered.policy.kind, RenderPolicyKind::Render);
    assert_eq!(reveal_audit_count(&conn), 1);
}

struct PolicyValue {
    id: String,
    text: String,
    sensitivity: ClaimSensitivity,
}

struct PolicyValues {
    public: PolicyValue,
    internal: PolicyValue,
    confidential: PolicyValue,
    user_only: PolicyValue,
}

fn fixture_conn() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory db");
    conn.execute_batch(CLAIMS_SCHEMA)
        .expect("create claims table");
    conn
}

fn render_claim_text(db: &ActionDb, value: &PolicyValue) -> Option<String> {
    render_claim_text_with_dto_text(db, value, &value.text)
}

fn render_claim_text_with_dto_text(
    db: &ActionDb,
    value: &PolicyValue,
    dto_text: impl Into<String>,
) -> Option<String> {
    render_mcp_static_text_for_surface(
        db,
        RenderableMcpText::Claim(RenderableMcpClaimText {
            text: dto_text.into(),
            claim_id: value.id.clone(),
            sensitivity: value.sensitivity.clone(),
        }),
    )
}

fn render_static_text(
    db: &ActionDb,
    text: impl Into<String>,
    surface_class: McpStaticTextClass,
) -> Option<String> {
    render_mcp_static_text_for_surface(
        db,
        RenderableMcpText::Static(RenderableMcpStaticText::new(text, surface_class)),
    )
}

fn test_briefing_static_text_class(path: &[String], _text: &str) -> Option<McpStaticTextClass> {
    let root = path.first().map(String::as_str)?;
    let leaf = path.last().map(String::as_str)?;
    match (root, leaf) {
        ("schedule", "title") => Some(McpStaticTextClass::MeetingTitle),
        ("schedule", "start_time") => Some(McpStaticTextClass::DateTime),
        ("actions", "priority") => Some(McpStaticTextClass::ActionPriority),
        ("actions", "title") => Some(McpStaticTextClass::ActionTitle),
        ("emails", "subject") => Some(McpStaticTextClass::EmailSubject),
        ("emails", "snippet") => Some(McpStaticTextClass::EmailSnippet),
        ("briefing", "narrative") => Some(McpStaticTextClass::BriefingNarrative),
        _ => None,
    }
}

fn seed_policy_claims(
    conn: &Connection,
    subject_kind: &str,
    subject_id: &str,
    claim_type: &str,
    label: &str,
) -> PolicyValues {
    let public = format!("{label} public example.com");
    let internal = format!("{label} internal example.com");
    let confidential = format!("{label} confidential example.com");
    let user_only = format!("{label} user only example.com");

    PolicyValues {
        public: seed_claim(
            conn,
            subject_kind,
            subject_id,
            claim_type,
            ClaimSensitivity::Public,
            &public,
        ),
        internal: seed_claim(
            conn,
            subject_kind,
            subject_id,
            claim_type,
            ClaimSensitivity::Internal,
            &internal,
        ),
        confidential: seed_claim(
            conn,
            subject_kind,
            subject_id,
            claim_type,
            ClaimSensitivity::Confidential,
            &confidential,
        ),
        user_only: seed_claim(
            conn,
            subject_kind,
            subject_id,
            claim_type,
            ClaimSensitivity::UserOnly,
            &user_only,
        ),
    }
}

fn seed_claim(
    conn: &Connection,
    subject_kind: &str,
    subject_id: &str,
    claim_type: &str,
    sensitivity: ClaimSensitivity,
    text: &str,
) -> PolicyValue {
    let sensitivity_name = match sensitivity {
        ClaimSensitivity::Public => "public",
        ClaimSensitivity::Internal => "internal",
        ClaimSensitivity::Confidential => "confidential",
        ClaimSensitivity::UserOnly => "user_only",
    };
    let id =
        format!("{subject_kind}-{subject_id}-{claim_type}-{sensitivity_name}").replace('_', "-");
    let dedup_key = format!("{id}-dedup");
    conn.execute(
        "INSERT INTO intelligence_claims /* dos7-allowed: DOS-412 MCP static-surface policy fixture seeds sensitivity rows */ (
            id, subject_ref, claim_type, text, dedup_key, actor, data_source,
            observed_at, created_at, provenance_json, claim_state, surfacing_state,
            temporal_scope, sensitivity, verification_state
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'test', '2026-05-06T00:00:00Z',
                  '2026-05-06T00:00:00Z', '{}', 'active', 'active', 'state', ?7, 'active')",
        params![
            id,
            json!({ "kind": subject_kind, "id": subject_id }).to_string(),
            claim_type,
            text,
            dedup_key,
            if matches!(sensitivity, ClaimSensitivity::UserOnly) {
                "user:owner"
            } else {
                "agent:test"
            },
            sensitivity_name,
        ],
    )
    .expect("insert claim fixture");

    PolicyValue {
        id,
        text: text.to_string(),
        sensitivity,
    }
}

fn update_claim_lifecycle(
    conn: &Connection,
    claim_id: &str,
    claim_state: &str,
    surfacing_state: &str,
) {
    conn.execute(
        "UPDATE intelligence_claims /* dos7-allowed: DOS-412 MCP static-surface policy fixture mutates lifecycle state */ \
         SET claim_state = ?2, surfacing_state = ?3 WHERE id = ?1",
        params![claim_id, claim_state, surfacing_state],
    )
    .expect("update claim lifecycle fixture");
}

fn reveal_audit_count(conn: &Connection) -> i64 {
    conn.query_row("SELECT COUNT(*) FROM sensitivity_reveal_audit", [], |row| {
        row.get(0)
    })
    .expect("count reveal audit rows")
}
