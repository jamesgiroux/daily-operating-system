use dailyos_lib::db::ActionDb;
use dailyos_lib::intelligence::contamination::{
    collect_narrative_text, detect_cross_entity_contamination, ContaminationKind,
    ContaminationValidation,
};
use dailyos_lib::intelligence::io::{
    write_intelligence_json, IntelRisk, IntelWin, IntelligenceJson, ItemSource, StakeholderInsight,
};
use dailyos_lib::substrate_test_api::{
    compose_enrichment_intelligence_with_policy, EnrichmentComposition, EnrichmentInput,
};
use rusqlite::{params, Connection};

const CLAIMS_SCHEMA_SQL: &str = include_str!("../src/migrations/129_dos_7_claims_schema.sql");
const FEEDBACK_SCHEMA_SQL: &str = include_str!("../src/migrations/084_feedback_events.sql");
const MALFORMED_LOG_SQL: &str = include_str!("../src/migrations/126_suppression_malformed_log.sql");
const PROJECTION_STATUS_SQL: &str =
    include_str!("../src/migrations/134_dos_301_claim_projection_status.sql");
const TYPED_FEEDBACK_SQL: &str =
    include_str!("../src/migrations/135_dos_294_typed_feedback_schema.sql");

const TARGET_ACCOUNT_ID: &str = "dos287-acme-corp";
const FOREIGN_ACCOUNT_ID: &str = "dos287-acme-subsidiary";
const TARGET_DOMAIN: &str = "acme.com";
const FOREIGN_DOMAIN: &str = "acme-test.com";
const FOREIGN_VIP_HOST: &str = "vip2-acme-test.com";
const TARGET_STAKEHOLDER_ID: &str = "person-dos287-alice";
const FOREIGN_STAKEHOLDER_ID: &str = "person-dos287-blake";
const FOREIGN_STAKEHOLDER_NAME: &str = "Blake Branch";

const LEGACY_SCHEMA_SQL: &str = r#"
CREATE TABLE accounts (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    lifecycle TEXT,
    arr REAL,
    health TEXT,
    contract_start TEXT,
    contract_end TEXT,
    nps INTEGER,
    tracker_path TEXT,
    parent_id TEXT,
    is_internal INTEGER NOT NULL DEFAULT 0,
    account_type TEXT NOT NULL DEFAULT 'customer',
    updated_at TEXT NOT NULL,
    archived INTEGER NOT NULL DEFAULT 0,
    keywords TEXT,
    keywords_extracted_at TEXT,
    metadata TEXT,
    commercial_stage TEXT,
    arr_range_low REAL,
    arr_range_high REAL,
    renewal_likelihood REAL,
    renewal_likelihood_source TEXT,
    renewal_likelihood_updated_at TEXT,
    renewal_model TEXT,
    renewal_pricing_method TEXT,
    support_tier TEXT,
    support_tier_source TEXT,
    support_tier_updated_at TEXT,
    active_subscription_count INTEGER,
    growth_potential_score REAL,
    growth_potential_score_source TEXT,
    icp_fit_score REAL,
    icp_fit_score_source TEXT,
    primary_product TEXT,
    customer_status TEXT,
    customer_status_source TEXT,
    customer_status_updated_at TEXT,
    company_overview TEXT,
    strategic_programs TEXT,
    notes TEXT,
    user_health_sentiment TEXT,
    sentiment_set_at TEXT,
    claim_version INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE account_domains (
    account_id TEXT NOT NULL,
    domain TEXT NOT NULL,
    source TEXT,
    PRIMARY KEY (account_id, domain)
);

CREATE TABLE people (
    id TEXT PRIMARY KEY,
    email TEXT,
    name TEXT NOT NULL,
    organization TEXT,
    role TEXT,
    relationship TEXT NOT NULL DEFAULT 'customer',
    notes TEXT,
    tracker_path TEXT,
    last_seen TEXT,
    first_seen TEXT,
    meeting_count INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL,
    archived INTEGER NOT NULL DEFAULT 0,
    linkedin_url TEXT,
    twitter_handle TEXT,
    phone TEXT,
    photo_url TEXT,
    bio TEXT,
    title_history TEXT,
    company_industry TEXT,
    company_size TEXT,
    company_hq TEXT,
    last_enriched_at TEXT,
    enrichment_sources TEXT,
    claim_version INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE account_stakeholders (
    account_id TEXT NOT NULL,
    person_id TEXT NOT NULL,
    engagement TEXT,
    data_source_engagement TEXT,
    assessment TEXT,
    data_source_assessment TEXT,
    data_source TEXT DEFAULT 'user',
    status TEXT NOT NULL DEFAULT 'active',
    confidence REAL,
    last_seen_in_glean TEXT,
    created_at TEXT,
    PRIMARY KEY (account_id, person_id)
);

CREATE TABLE account_stakeholder_roles (
    account_id TEXT NOT NULL,
    person_id TEXT NOT NULL,
    role TEXT NOT NULL,
    data_source TEXT,
    dismissed_at TEXT
);

CREATE TABLE entity_members (
    entity_id TEXT NOT NULL,
    person_id TEXT NOT NULL
);

CREATE TABLE projects (
    id TEXT PRIMARY KEY,
    name TEXT,
    parent_id TEXT,
    archived INTEGER NOT NULL DEFAULT 0,
    claim_version INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE meetings (
    id TEXT PRIMARY KEY,
    start_time TEXT,
    claim_version INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE meeting_entities (
    meeting_id TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    entity_type TEXT
);

CREATE TABLE meeting_attendees (
    meeting_id TEXT NOT NULL,
    person_id TEXT NOT NULL
);

CREATE TABLE emails (
    email_id TEXT PRIMARY KEY,
    claim_version INTEGER NOT NULL DEFAULT 0
);

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
    success_plan_signals_json TEXT,
    pull_quote TEXT,
    health_outlook_signals_json TEXT
);

CREATE TABLE entity_quality (
    entity_id TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL,
    health_score REAL,
    health_trend TEXT
);

CREATE TABLE intelligence_feedback (
    id TEXT PRIMARY KEY,
    entity_id TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    field TEXT NOT NULL,
    feedback_type TEXT NOT NULL,
    previous_value TEXT,
    context TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(entity_id, entity_type, field)
);

CREATE TABLE signal_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    signal_type TEXT NOT NULL,
    source TEXT,
    payload_json TEXT,
    confidence REAL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    superseded_by TEXT
);
"#;

#[test]
fn bundle1_same_domain_bleed_is_rejected_before_substrate_writes() {
    let conn = fresh_db();
    let db = ActionDb::from_conn(&conn);
    seed_bundle1_fixture(db);

    let tempdir = tempfile::tempdir().expect("tempdir");
    let input = EnrichmentInput {
        workspace: tempdir.path().to_path_buf(),
        entity_dir: tempdir.path().to_path_buf(),
        entity_id: TARGET_ACCOUNT_ID.to_string(),
        entity_type: "account".to_string(),
        prompt: String::new(),
        file_manifest: Vec::new(),
        file_count: 0,
        computed_health: None,
        entity_name: "Acme Corp".to_string(),
        relationship: None,
        intelligence_context: None,
        active_preset: None,
    };

    let prior = prior_target_intelligence();
    db.upsert_entity_intelligence(&prior)
        .expect("seed target prior intelligence");
    seed_stakeholder_cache(&conn, TARGET_ACCOUNT_ID, "Alice Adams", "trusted champion");
    write_intelligence_json(tempdir.path(), &prior).expect("seed target intelligence.json");

    db.upsert_entity_intelligence(&prior_foreign_intelligence())
        .expect("seed adjacent account prior intelligence");

    let db_before = entity_assessment_snapshot(&conn, TARGET_ACCOUNT_ID);
    let disk_before =
        std::fs::read_to_string(tempdir.path().join("intelligence.json")).expect("read disk");
    let target_claims_before = target_account_claim_count(&conn);

    let contaminated = contaminated_target_enrichment();
    let narrative = collect_narrative_text(&contaminated);
    let hits = detect_cross_entity_contamination(
        &narrative,
        TARGET_ACCOUNT_ID,
        &[TARGET_DOMAIN.to_string()],
        &[],
        db,
    );
    assert!(
        hits.iter().any(|hit| {
            hit.source_account_id.as_deref() == Some(FOREIGN_ACCOUNT_ID)
                && (hit.foreign_token == FOREIGN_DOMAIN || hit.foreign_token == FOREIGN_VIP_HOST)
        }) || hits.iter().any(|hit| {
            hit.kind == ContaminationKind::InfrastructureId && hit.foreign_token == FOREIGN_VIP_HOST
        }),
        "strict detector must identify adjacent-account content, got {hits:?}"
    );

    let composition = compose_enrichment_intelligence_with_policy(
        db,
        &input,
        &contaminated,
        ContaminationValidation::RejectOnHit,
    )
    .expect("compose contaminated enrichment");

    match composition {
        EnrichmentComposition::SkipDueToContamination { reason, prior, .. } => {
            assert!(
                reason.contains("cross-entity contamination"),
                "skip reason should explain contamination, got {reason}"
            );
            assert_eq!(prior.entity_id, TARGET_ACCOUNT_ID);
            assert_eq!(
                prior.executive_assessment.as_deref(),
                Some("Acme Corp has steady adoption and an active renewal plan.")
            );
        }
        EnrichmentComposition::Persist(prepared) => {
            panic!(
                "bundle-1 bleed should not persist; prepared payload was {:?}",
                prepared.intelligence()
            );
        }
    }

    assert_eq!(
        entity_assessment_snapshot(&conn, TARGET_ACCOUNT_ID),
        db_before,
        "SkipDueToContamination must preserve target entity_assessment"
    );
    assert_eq!(
        std::fs::read_to_string(tempdir.path().join("intelligence.json")).expect("read disk"),
        disk_before,
        "SkipDueToContamination must not rewrite intelligence.json"
    );
    assert_eq!(
        target_account_claim_count(&conn),
        target_claims_before,
        "no new target account claims should land after contamination rejection"
    );

    assert_no_target_claim_mentions(&conn, FOREIGN_VIP_HOST);
    assert_no_target_claim_mentions(&conn, FOREIGN_STAKEHOLDER_NAME);
    assert_no_cross_account_stakeholder_rows(&conn);
    assert_stakeholder_cache_excludes_foreign_content(&conn);
    assert_target_claim_attribution_excludes_foreign_sources(&conn);
}

fn fresh_db() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory db");
    conn.execute_batch(LEGACY_SCHEMA_SQL)
        .expect("apply legacy schema");
    conn.execute_batch(FEEDBACK_SCHEMA_SQL)
        .expect("apply feedback schema");
    conn.execute_batch(MALFORMED_LOG_SQL)
        .expect("apply malformed suppression log schema");
    conn.execute_batch(CLAIMS_SCHEMA_SQL)
        .expect("apply claims schema");
    conn.execute_batch(PROJECTION_STATUS_SQL)
        .expect("apply projection status schema");
    conn.execute_batch(TYPED_FEEDBACK_SQL)
        .expect("apply typed feedback schema");
    conn
}

fn seed_bundle1_fixture(db: &ActionDb) {
    db.conn_ref()
        .execute(
            "INSERT INTO accounts (id, name, account_type, updated_at, archived)
             VALUES (?1, ?2, 'customer', '2026-05-04T12:00:00Z', 0)",
            params![TARGET_ACCOUNT_ID, "Acme Corp"],
        )
        .expect("seed target account");
    db.conn_ref()
        .execute(
            "INSERT INTO accounts (id, name, account_type, updated_at, archived)
             VALUES (?1, ?2, 'customer', '2026-05-04T12:00:00Z', 0)",
            params![FOREIGN_ACCOUNT_ID, "Acme Subsidiary"],
        )
        .expect("seed foreign account");
    db.conn_ref()
        .execute(
            "INSERT INTO account_domains (account_id, domain, source)
             VALUES (?1, ?2, 'test')",
            params![TARGET_ACCOUNT_ID, TARGET_DOMAIN],
        )
        .expect("seed target domain");
    db.conn_ref()
        .execute(
            "INSERT INTO account_domains (account_id, domain, source)
             VALUES (?1, ?2, 'test')",
            params![FOREIGN_ACCOUNT_ID, FOREIGN_DOMAIN],
        )
        .expect("seed foreign domain");
    db.conn_ref()
        .execute(
            "INSERT INTO account_domains (account_id, domain, source)
             VALUES (?1, ?2, 'test')",
            params![FOREIGN_ACCOUNT_ID, FOREIGN_VIP_HOST],
        )
        .expect("seed foreign VIP host");

    seed_person(db, TARGET_STAKEHOLDER_ID, "Alice Adams", "alice@acme.com");
    seed_person(
        db,
        FOREIGN_STAKEHOLDER_ID,
        FOREIGN_STAKEHOLDER_NAME,
        "blake@acme-test.com",
    );
    seed_account_stakeholder(
        db,
        TARGET_ACCOUNT_ID,
        TARGET_STAKEHOLDER_ID,
        "trusted champion",
        "Alice owns the Acme Corp rollout.",
    );
    seed_account_stakeholder(
        db,
        FOREIGN_ACCOUNT_ID,
        FOREIGN_STAKEHOLDER_ID,
        "blocked",
        "Blake owns the Acme Subsidiary VIP2 migration.",
    );
}

fn seed_person(db: &ActionDb, id: &str, name: &str, email: &str) {
    db.conn_ref()
        .execute(
            "INSERT INTO people (id, email, name, relationship, updated_at, archived)
             VALUES (?1, ?2, ?3, 'external', '2026-05-04T12:00:00Z', 0)",
            params![id, email, name],
        )
        .expect("seed person");
}

fn seed_account_stakeholder(
    db: &ActionDb,
    account_id: &str,
    person_id: &str,
    engagement: &str,
    assessment: &str,
) {
    db.conn_ref()
        .execute(
            "INSERT INTO account_stakeholders
             (account_id, person_id, engagement, data_source_engagement,
              assessment, data_source_assessment, data_source, status, created_at)
             VALUES (?1, ?2, ?3, 'user', ?4, 'user', 'user', 'active',
                     '2026-05-04T12:00:00Z')",
            params![account_id, person_id, engagement, assessment],
        )
        .expect("seed account stakeholder");
}

fn prior_target_intelligence() -> IntelligenceJson {
    IntelligenceJson {
        entity_id: TARGET_ACCOUNT_ID.to_string(),
        entity_type: "account".to_string(),
        enriched_at: "2026-05-04T12:00:00Z".to_string(),
        executive_assessment: Some(
            "Acme Corp has steady adoption and an active renewal plan.".to_string(),
        ),
        risks: vec![IntelRisk {
            text: "Procurement timing is the only active renewal risk.".to_string(),
            item_source: Some(item_source("target-crm")),
            ..Default::default()
        }],
        recent_wins: vec![IntelWin {
            text: "Alice Adams expanded executive enablement for Acme Corp.".to_string(),
            item_source: Some(item_source("target-crm")),
            ..Default::default()
        }],
        stakeholder_insights: vec![StakeholderInsight {
            name: "Alice Adams".to_string(),
            person_id: Some(TARGET_STAKEHOLDER_ID.to_string()),
            role: Some("Champion".to_string()),
            engagement: Some("trusted champion".to_string()),
            assessment: Some("Alice owns the Acme Corp rollout.".to_string()),
            item_source: Some(item_source("target-crm")),
            ..Default::default()
        }],
        ..Default::default()
    }
}

fn prior_foreign_intelligence() -> IntelligenceJson {
    IntelligenceJson {
        entity_id: FOREIGN_ACCOUNT_ID.to_string(),
        entity_type: "account".to_string(),
        enriched_at: "2026-05-04T12:00:00Z".to_string(),
        executive_assessment: Some(format!(
            "Acme Subsidiary is blocked on the {FOREIGN_VIP_HOST} migration."
        )),
        risks: vec![IntelRisk {
            text: format!("{FOREIGN_VIP_HOST} has launch-blocking cache instability."),
            item_source: Some(item_source("foreign-glean")),
            ..Default::default()
        }],
        recent_wins: vec![IntelWin {
            text: format!("{FOREIGN_STAKEHOLDER_NAME} completed the Acme Subsidiary SSO plan."),
            item_source: Some(item_source("foreign-glean")),
            ..Default::default()
        }],
        stakeholder_insights: vec![StakeholderInsight {
            name: FOREIGN_STAKEHOLDER_NAME.to_string(),
            person_id: Some(FOREIGN_STAKEHOLDER_ID.to_string()),
            role: Some("Technical owner".to_string()),
            engagement: Some("blocked".to_string()),
            assessment: Some(format!(
                "{FOREIGN_STAKEHOLDER_NAME} owns {FOREIGN_VIP_HOST} remediation."
            )),
            item_source: Some(item_source("foreign-glean")),
            ..Default::default()
        }],
        ..Default::default()
    }
}

fn contaminated_target_enrichment() -> IntelligenceJson {
    IntelligenceJson {
        entity_id: TARGET_ACCOUNT_ID.to_string(),
        entity_type: "account".to_string(),
        enriched_at: "2026-05-04T13:00:00Z".to_string(),
        executive_assessment: Some(format!(
            "Acme Corp enrichment incorrectly says {FOREIGN_STAKEHOLDER_NAME} is \
             managing {FOREIGN_VIP_HOST} launch risk for Acme Subsidiary."
        )),
        risks: vec![IntelRisk {
            text: format!("{FOREIGN_VIP_HOST} cache instability is blocking the renewal."),
            item_source: Some(item_source("foreign-glean")),
            ..Default::default()
        }],
        recent_wins: vec![IntelWin {
            text: format!("{FOREIGN_STAKEHOLDER_NAME} completed Acme Subsidiary SSO validation."),
            item_source: Some(item_source("foreign-glean")),
            ..Default::default()
        }],
        stakeholder_insights: vec![StakeholderInsight {
            name: FOREIGN_STAKEHOLDER_NAME.to_string(),
            person_id: Some(FOREIGN_STAKEHOLDER_ID.to_string()),
            role: Some("Technical owner".to_string()),
            engagement: Some("blocked".to_string()),
            assessment: Some(format!(
                "{FOREIGN_STAKEHOLDER_NAME} is focused on {FOREIGN_VIP_HOST} migration risk."
            )),
            item_source: Some(item_source("foreign-glean")),
            ..Default::default()
        }],
        ..Default::default()
    }
}

fn item_source(source: &str) -> ItemSource {
    ItemSource {
        source: source.to_string(),
        confidence: 0.9,
        sourced_at: "2026-05-04T12:00:00Z".to_string(),
        reference: None,
    }
}

fn seed_stakeholder_cache(conn: &Connection, account_id: &str, name: &str, engagement: &str) {
    let json = serde_json::json!([
        {
            "name": name,
            "personId": TARGET_STAKEHOLDER_ID,
            "engagement": engagement,
            "assessment": "Alice owns the Acme Corp rollout."
        }
    ])
    .to_string();
    conn.execute(
        "UPDATE entity_assessment SET stakeholder_insights_json = ?1 WHERE entity_id = ?2",
        params![json, account_id],
    )
    .expect("seed stakeholder cache");
}

fn entity_assessment_snapshot(conn: &Connection, entity_id: &str) -> String {
    conn.query_row(
        "SELECT json_object(
            'executiveAssessment', executive_assessment,
            'risks', risks_json,
            'recentWins', recent_wins_json,
            'stakeholders', stakeholder_insights_json,
            'enrichedAt', enriched_at
         )
         FROM entity_assessment WHERE entity_id = ?1",
        params![entity_id],
        |row| row.get(0),
    )
    .expect("entity assessment snapshot")
}

fn target_account_claim_count(conn: &Connection) -> i64 {
    conn.query_row(
        "SELECT COUNT(*) FROM intelligence_claims
         WHERE json_extract(subject_ref, '$.kind') = 'account'
           AND json_extract(subject_ref, '$.id') = ?1",
        params![TARGET_ACCOUNT_ID],
        |row| row.get(0),
    )
    .expect("target claim count")
}

fn assert_no_target_claim_mentions(conn: &Connection, forbidden: &str) {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM intelligence_claims
             WHERE json_extract(subject_ref, '$.kind') = 'account'
               AND json_extract(subject_ref, '$.id') = ?1
               AND lower(text) LIKE '%' || lower(?2) || '%'",
            params![TARGET_ACCOUNT_ID, forbidden],
            |row| row.get(0),
        )
        .expect("target forbidden claim count");
    assert_eq!(
        count, 0,
        "target account claims must not mention foreign token {forbidden}"
    );
}

fn assert_no_cross_account_stakeholder_rows(conn: &Connection) {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM account_stakeholders
             WHERE account_id = ?1 AND person_id = ?2",
            params![TARGET_ACCOUNT_ID, FOREIGN_STAKEHOLDER_ID],
            |row| row.get(0),
        )
        .expect("cross-account stakeholder row count");
    assert_eq!(
        count, 0,
        "foreign stakeholder must not be linked to target account"
    );
}

fn assert_stakeholder_cache_excludes_foreign_content(conn: &Connection) {
    let cache: Option<String> = conn
        .query_row(
            "SELECT stakeholder_insights_json FROM entity_assessment WHERE entity_id = ?1",
            params![TARGET_ACCOUNT_ID],
            |row| row.get(0),
        )
        .expect("stakeholder cache");
    let cache = cache.unwrap_or_default().to_lowercase();
    assert!(
        !cache.contains(&FOREIGN_STAKEHOLDER_NAME.to_lowercase()),
        "target stakeholder cache must not contain foreign stakeholder"
    );
    assert!(
        !cache.contains(FOREIGN_VIP_HOST),
        "target stakeholder cache must not contain foreign VIP host"
    );
}

fn assert_target_claim_attribution_excludes_foreign_sources(conn: &Connection) {
    let mut stmt = conn
        .prepare(
            "SELECT subject_ref, COALESCE(source_ref, ''), actor, data_source, text
             FROM intelligence_claims
             WHERE json_extract(subject_ref, '$.kind') = 'account'
               AND json_extract(subject_ref, '$.id') = ?1",
        )
        .expect("prepare attribution query");
    let rows = stmt
        .query_map(params![TARGET_ACCOUNT_ID], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })
        .expect("map attribution rows");

    let mut seen = 0;
    for row in rows {
        let (subject_ref, source_ref, actor, data_source, text) = row.expect("attribution row");
        seen += 1;
        let joined =
            format!("{subject_ref}\n{source_ref}\n{actor}\n{data_source}\n{text}").to_lowercase();
        assert!(
            !joined.contains(FOREIGN_ACCOUNT_ID),
            "target claim must not attribute to foreign account: {joined}"
        );
        assert!(
            !joined.contains(FOREIGN_STAKEHOLDER_ID),
            "target claim must not attribute to foreign stakeholder: {joined}"
        );
        assert!(
            !joined.contains("foreign-glean"),
            "target claim must not retain foreign source label: {joined}"
        );
    }
    assert!(seen > 0, "fixture should have legitimate target claims");
}
