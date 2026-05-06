use dailyos_lib::db::claims::ClaimSensitivity;
use dailyos_lib::db::ActionDb;
use dailyos_lib::services::sensitivity::{
    render_mcp_static_json_for_surface, render_mcp_static_text_for_surface, McpStaticTextContext,
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
"#;

#[test]
fn get_briefing_static_json_applies_mcp_policy_to_briefing_actions_emails_schedule() {
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
            "schedule": [values.public, values.confidential],
            "actions": [{"title": values.internal}, {"title": values.user_only}],
            "emails": [{"snippet": values.confidential}],
            "briefing": {"narrative": values.public}
        }),
        &[],
    )
    .expect("briefing JSON renders with private leaves dropped");

    let serialized = rendered.to_string();
    assert!(serialized.contains("briefing public example.com"));
    assert!(serialized.contains("briefing internal example.com"));
    assert!(!serialized.contains("briefing confidential example.com"));
    assert!(!serialized.contains("briefing user only example.com"));
}

#[test]
fn query_entity_static_fields_apply_mcp_policy_to_summary_and_action_titles() {
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
    let summary_context = [McpStaticTextContext::new(
        "account",
        "acct-example",
        &["entity_summary"],
    )];
    let action_context = [McpStaticTextContext::new(
        "account",
        "acct-example",
        &["open_loop"],
    )];

    let open_actions = [
        render_mcp_static_text_for_surface(db, &actions.public, &action_context),
        render_mcp_static_text_for_surface(db, &actions.internal, &action_context),
        render_mcp_static_text_for_surface(db, &actions.confidential, &action_context),
        render_mcp_static_text_for_surface(db, &actions.user_only, &action_context),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    let response = json!({
        "intelligence_summary": render_mcp_static_text_for_surface(db, &summaries.public, &summary_context),
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
fn search_meetings_static_fields_apply_mcp_policy_to_transcript_and_prep_snippets() {
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
    let contexts = [
        McpStaticTextContext::new("meeting", "mtg-example", &["meeting_event_note"]),
        McpStaticTextContext::new("account", "acct-example", &["entity_summary"]),
    ];

    let results = [
        render_mcp_static_text_for_surface(db, &transcript.public, &contexts),
        render_mcp_static_text_for_surface(db, &transcript.internal, &contexts),
        render_mcp_static_text_for_surface(db, &transcript.confidential, &contexts),
        render_mcp_static_text_for_surface(db, &transcript.user_only, &contexts),
        render_mcp_static_text_for_surface(db, &prep.public, &contexts),
        render_mcp_static_text_for_surface(db, &prep.confidential, &contexts),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    let response = json!({ "results": results });

    let serialized = response.to_string();
    assert!(serialized.contains("transcript public example.com"));
    assert!(serialized.contains("transcript internal example.com"));
    assert!(serialized.contains("prep public example.com"));
    assert!(!serialized.contains("transcript confidential example.com"));
    assert!(!serialized.contains("transcript user only example.com"));
    assert!(!serialized.contains("prep confidential example.com"));
}

#[test]
fn search_content_static_chunks_apply_mcp_policy_to_semantic_excerpts() {
    let conn = fixture_conn();
    let db = ActionDb::from_conn(&conn);
    let chunks = seed_policy_claims(&conn, "account", "acct-example", "entity_risk", "chunk");

    assert_eq!(
        render_mcp_static_text_for_surface(db, &chunks.public, &[]),
        Some(chunks.public.clone())
    );
    assert_eq!(
        render_mcp_static_text_for_surface(db, &chunks.internal, &[]),
        Some(chunks.internal.clone())
    );
    assert_eq!(
        render_mcp_static_text_for_surface(
            db,
            &format!("excerpt start {} excerpt end", chunks.confidential),
            &[]
        ),
        None
    );
    assert_eq!(
        render_mcp_static_text_for_surface(
            db,
            &format!("excerpt start {} excerpt end", chunks.user_only),
            &[]
        ),
        None
    );
}

struct PolicyValues {
    public: String,
    internal: String,
    confidential: String,
    user_only: String,
}

fn fixture_conn() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory db");
    conn.execute_batch(CLAIMS_SCHEMA)
        .expect("create claims table");
    conn
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

    seed_claim(
        conn,
        subject_kind,
        subject_id,
        claim_type,
        ClaimSensitivity::Public,
        &public,
    );
    seed_claim(
        conn,
        subject_kind,
        subject_id,
        claim_type,
        ClaimSensitivity::Internal,
        &internal,
    );
    seed_claim(
        conn,
        subject_kind,
        subject_id,
        claim_type,
        ClaimSensitivity::Confidential,
        &confidential,
    );
    seed_claim(
        conn,
        subject_kind,
        subject_id,
        claim_type,
        ClaimSensitivity::UserOnly,
        &user_only,
    );

    PolicyValues {
        public,
        internal,
        confidential,
        user_only,
    }
}

fn seed_claim(
    conn: &Connection,
    subject_kind: &str,
    subject_id: &str,
    claim_type: &str,
    sensitivity: ClaimSensitivity,
    text: &str,
) {
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
}
