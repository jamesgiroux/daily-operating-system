use chrono::{TimeZone, Utc};
use dailyos_lib::db::ActionDb;
use dailyos_lib::services::claims::{commit_claim, ClaimProposal};
use dailyos_lib::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};
use rusqlite::{params, Connection};

const CLAIMS_SCHEMA_SQL: &str = include_str!("../src/migrations/129_dos_7_claims_schema.sql");
const PROJECTION_STATUS_SQL: &str =
    include_str!("../src/migrations/134_dos_301_claim_projection_status.sql");
const TYPED_FEEDBACK_SQL: &str =
    include_str!("../src/migrations/135_dos_294_typed_feedback_schema.sql");

const LEGACY_READER_SCHEMA_SQL: &str = r#"
CREATE TABLE accounts (
    id TEXT PRIMARY KEY,
    claim_version INTEGER NOT NULL DEFAULT 0,
    company_overview TEXT,
    updated_at TEXT
);

CREATE TABLE projects (id TEXT PRIMARY KEY, claim_version INTEGER NOT NULL DEFAULT 0);
CREATE TABLE people (id TEXT PRIMARY KEY, claim_version INTEGER NOT NULL DEFAULT 0);
CREATE TABLE meetings (id TEXT PRIMARY KEY, claim_version INTEGER NOT NULL DEFAULT 0);
CREATE TABLE emails (email_id TEXT PRIMARY KEY, claim_version INTEGER NOT NULL DEFAULT 0);

CREATE TABLE migration_state (
    key TEXT PRIMARY KEY,
    value INTEGER NOT NULL
);
INSERT OR IGNORE INTO migration_state (key, value) VALUES ('global_claim_epoch', 0);
INSERT OR IGNORE INTO migration_state (key, value) VALUES ('schema_epoch', 1);

CREATE TABLE entity_assessment (
    entity_id TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL DEFAULT 'account',
    enriched_at TEXT,
    source_file_count INTEGER DEFAULT 0,
    executive_assessment TEXT,
    risks_json TEXT,
    recent_wins_json TEXT,
    current_state_json TEXT,
    stakeholder_insights_json TEXT,
    next_meeting_readiness_json TEXT,
    company_context_json TEXT,
    health_json TEXT,
    org_health_json TEXT,
    value_delivered TEXT,
    success_metrics TEXT,
    open_commitments TEXT,
    relationship_depth TEXT,
    consistency_status TEXT,
    consistency_findings_json TEXT,
    consistency_checked_at TEXT,
    portfolio_json TEXT,
    network_json TEXT,
    user_edits_json TEXT,
    source_manifest_json TEXT,
    dimensions_json TEXT,
    pull_quote TEXT
);

CREATE TABLE entity_quality (
    entity_id TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL,
    health_score REAL,
    health_trend TEXT
);
"#;

fn ctx_parts() -> (FixedClock, SeedableRng, ExternalClients) {
    (
        FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 3, 12, 0, 0).unwrap()),
        SeedableRng::new(7),
        ExternalClients::default(),
    )
}

fn live_ctx<'a>(
    clock: &'a FixedClock,
    rng: &'a SeedableRng,
    external: &'a ExternalClients,
) -> ServiceContext<'a> {
    ServiceContext::new_live(clock, rng, external)
}

fn fresh_db() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory db");
    conn.execute_batch(LEGACY_READER_SCHEMA_SQL)
        .expect("apply legacy reader schema");
    conn.execute_batch(CLAIMS_SCHEMA_SQL)
        .expect("apply claims schema");
    conn.execute_batch(PROJECTION_STATUS_SQL)
        .expect("apply projection status schema");
    conn.execute_batch(TYPED_FEEDBACK_SQL)
        .expect("apply typed feedback schema");
    conn
}

#[test]
fn commit_claim_projects_entity_summary_to_legacy_reader() {
    let conn = fresh_db();
    conn.execute(
        "INSERT INTO accounts (id, updated_at) VALUES (?1, ?2)",
        params!["acct-dos301-roundtrip", "2026-05-03T12:00:00Z"],
    )
    .expect("seed account");

    let (clock, rng, external) = ctx_parts();
    let ctx = live_ctx(&clock, &rng, &external);
    let subject_ref = serde_json::json!({
        "kind": "account",
        "id": "acct-dos301-roundtrip",
    })
    .to_string();

    commit_claim(
        &ctx,
        ActionDb::from_conn(&conn),
        ClaimProposal {
            subject_ref,
            claim_type: "entity_summary".to_string(),
            field_path: Some("executiveAssessment".to_string()),
            topic_key: None,
            text: "roundtrip summary visible to legacy reader".to_string(),
            actor: "agent:test".to_string(),
            data_source: "test".to_string(),
            source_ref: None,
            source_asof: Some("2026-05-03T12:00:00Z".to_string()),
            observed_at: "2026-05-03T12:00:00Z".to_string(),
            provenance_json: "{}".to_string(),
            metadata_json: None,
            thread_id: None,
            temporal_scope: None,
            sensitivity: None,
            tombstone: None,
        },
    )
    .expect("commit claim");

    let intel = ActionDb::from_conn(&conn)
        .get_entity_intelligence("acct-dos301-roundtrip")
        .expect("legacy reader")
        .expect("projected row");

    assert_eq!(
        intel.executive_assessment.as_deref(),
        Some("roundtrip summary visible to legacy reader")
    );
}
