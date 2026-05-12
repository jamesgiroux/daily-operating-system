use chrono::{TimeZone, Utc};
use dailyos_lib::db::ActionDb;
use dailyos_lib::services::claims::{commit_claim, ClaimProposal};
use dailyos_lib::services::context::{
    ClaimDismissalSurface, ExternalClients, FixedClock, SeedableRng, ServiceContext,
};
use rusqlite::{params, Connection};

const CLAIMS_SCHEMA_SQL: &str = include_str!("../src/migrations/129_dos_7_claims_schema.sql");
const PROJECTION_STATUS_SQL: &str =
    include_str!("../src/migrations/134_dos_301_claim_projection_status.sql");
const TYPED_FEEDBACK_SQL: &str =
    include_str!("../src/migrations/135_dos_294_typed_feedback_schema.sql");
const CLAIM_SURFACE_DISMISSALS_SQL: &str =
    include_str!("../src/migrations/154_claim_surface_dismissals.sql");
const SEMANTIC_EVIDENCE_SQL: &str = include_str!("../src/migrations/165_semantic_evidence.sql");

const STRUCTURED_CLAIM_CANONICALIZATION_COLUMNS_SQL: &str = r#"
ALTER TABLE intelligence_claims ADD COLUMN structured_claim_json TEXT;
ALTER TABLE intelligence_claims ADD COLUMN predicate_ref TEXT;
ALTER TABLE intelligence_claims ADD COLUMN polarity TEXT;
ALTER TABLE intelligence_claims ADD COLUMN object_value JSON;
ALTER TABLE intelligence_claims ADD COLUMN qualifiers JSON;
ALTER TABLE intelligence_claims ADD COLUMN structural_canonical_id TEXT;
ALTER TABLE intelligence_claims ADD COLUMN canonical_status TEXT NOT NULL DEFAULT 'pending_backfill'
    CHECK (canonical_status IN ('pending_backfill','legacy_unmigrated','live'));
ALTER TABLE intelligence_claims ADD COLUMN non_semantic_mergeable BOOLEAN NOT NULL DEFAULT TRUE;
ALTER TABLE intelligence_claims ADD COLUMN structural_field_content_hash TEXT;
ALTER TABLE intelligence_claims ADD COLUMN backfill_epoch INTEGER NOT NULL DEFAULT 0;
ALTER TABLE intelligence_claims ADD COLUMN backfill_attempts INTEGER NOT NULL DEFAULT 0;
"#;

// Mirror of v167_structured_claim_canonicalization tables that
// claim_surface_shadow_columns joins against. Without these tables present,
// the LEFT JOIN-style EXISTS subqueries in load_claims_where fail at prepare.
const CANONICALIZATION_DECISIONS_SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS canonicalization_decisions (
    decision_id TEXT PRIMARY KEY,
    claim_id_a TEXT NOT NULL,
    claim_id_b TEXT NOT NULL,
    decision TEXT NOT NULL
        CHECK (decision IN ('merge','fork','fork_ambiguous','fork_contradiction','fork_filtered')),
    mode TEXT NOT NULL CHECK (mode IN ('shadow','live')),
    is_authoritative BOOLEAN NOT NULL GENERATED ALWAYS AS (mode = 'live') STORED,
    field_scores JSONB NOT NULL,
    reason TEXT NOT NULL,
    reason_secondary JSONB,
    threshold_band TEXT CHECK (
        threshold_band IS NULL OR threshold_band IN ('high','ambiguous','low')
    ),
    embedding_model_version TEXT,
    comparator_threshold_version TEXT,
    field_provenance JSONB NOT NULL,
    canonicalization_mode TEXT NOT NULL CHECK (
        canonicalization_mode IN ('full','hash_fallback','deterministic')
    ),
    supersedes_decision_id TEXT REFERENCES canonicalization_decisions(decision_id),
    idempotency_key TEXT NOT NULL UNIQUE,
    claim_a_revision_hash TEXT NOT NULL,
    claim_b_revision_hash TEXT NOT NULL,
    evaluated_at TIMESTAMP NOT NULL,
    FOREIGN KEY (claim_id_a) REFERENCES intelligence_claims(id),
    FOREIGN KEY (claim_id_b) REFERENCES intelligence_claims(id)
);
CREATE TABLE IF NOT EXISTS ambiguous_claim_pairs (
    pair_id TEXT PRIMARY KEY,
    claim_id_a TEXT NOT NULL,
    claim_id_b TEXT NOT NULL,
    field_scores JSONB NOT NULL,
    decision_id TEXT NOT NULL REFERENCES canonicalization_decisions(decision_id),
    user_resolution TEXT CHECK (
        user_resolution IS NULL
        OR user_resolution IN ('merged','forked','contradicted','needs_user_decision')
    ),
    user_resolved_at TIMESTAMP,
    reconcile_attempts INT NOT NULL DEFAULT 0,
    next_reconcile_at TIMESTAMP,
    last_schema_version TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL,
    FOREIGN KEY (claim_id_a) REFERENCES intelligence_claims(id),
    FOREIGN KEY (claim_id_b) REFERENCES intelligence_claims(id)
);
"#;

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
    conn.execute_batch(CLAIM_SURFACE_DISMISSALS_SQL)
        .expect("apply claim surface dismissals schema");
    conn.execute_batch(SEMANTIC_EVIDENCE_SQL)
        .expect("apply semantic evidence schema");
    conn.execute_batch(STRUCTURED_CLAIM_CANONICALIZATION_COLUMNS_SQL)
        .expect("apply structured claim canonicalization columns");
    conn.execute_batch(CANONICALIZATION_DECISIONS_SCHEMA_SQL)
        .expect("apply canonicalization decisions schema");
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
            id: None,
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
            supersedes: None,
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

#[test]
fn tauri_report_read_uses_unfiltered_projection_after_entity_detail_dismissal() {
    let conn = fresh_db();
    conn.execute(
        "INSERT INTO accounts (id, updated_at) VALUES (?1, ?2)",
        params!["acct-dos301-surface-dismissal", "2026-05-03T12:00:00Z"],
    )
    .expect("seed account");

    let (clock, rng, external) = ctx_parts();
    let ctx = live_ctx(&clock, &rng, &external);
    let db = ActionDb::from_conn(&conn);
    let subject_ref = serde_json::json!({
        "kind": "account",
        "id": "acct-dos301-surface-dismissal",
    })
    .to_string();

    commit_claim(
        &ctx,
        db,
        ClaimProposal {
            id: Some("claim-dos301-entity-detail-dismissed-summary".to_string()),
            subject_ref: subject_ref.clone(),
            claim_type: "entity_summary".to_string(),
            field_path: Some("executiveAssessment".to_string()),
            topic_key: None,
            text: "summary must remain in shared report projection".to_string(),
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
            supersedes: None,
            tombstone: None,
        },
    )
    .expect("commit summary claim");

    conn.execute(
        "INSERT INTO claim_surface_dismissals (
            claim_id, surface, actor, dismissed_at
         ) VALUES (?1, ?2, ?3, ?4)",
        params![
            "claim-dos301-entity-detail-dismissed-summary",
            ClaimDismissalSurface::TauriEntityDetail.as_str(),
            "user",
            "2026-05-03T12:01:00Z",
        ],
    )
    .expect("dismiss summary only on entity detail");

    commit_claim(
        &ctx,
        db,
        ClaimProposal {
            id: Some("claim-dos301-report-visible-risk".to_string()),
            subject_ref,
            claim_type: "entity_risk".to_string(),
            field_path: Some("risks".to_string()),
            topic_key: None,
            text: "risk triggers projection rebuild".to_string(),
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
            supersedes: None,
            tombstone: None,
        },
    )
    .expect("commit risk claim");

    let intel = db
        .get_entity_intelligence("acct-dos301-surface-dismissal")
        .expect("legacy reader")
        .expect("projected row");

    assert_eq!(
        intel.executive_assessment.as_deref(),
        Some("summary must remain in shared report projection")
    );
    assert_eq!(intel.risks.len(), 1);
    assert_eq!(intel.risks[0].text, "risk triggers projection rebuild");
}
