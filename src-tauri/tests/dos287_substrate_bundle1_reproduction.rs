use chrono::{TimeZone, Utc};
use dailyos_lib::abilities::provenance::SubjectRef;
use dailyos_lib::abilities::trust::{
    compile_trust, CrossEntityCoherenceInput, FreshnessContext, SourceLifecycleState,
    TrustComputation, TrustConfig, TrustContext, TrustFactorInputs, UserFeedbackSignal,
};
use dailyos_lib::db::claims::{
    ClaimSensitivity, ClaimState, ClaimVerificationState, IntelligenceClaim, SurfacingState,
    TemporalScope,
};
use dailyos_lib::db::ActionDb;
use dailyos_lib::intelligence::io::{
    IntelRisk, IntelWin, IntelligenceJson, ItemSource, StakeholderInsight,
};
use dailyos_lib::services::trust_extraction::{extract_target_footprint, ExtractionOutcome};
use rusqlite::{params, Connection};

const CLAIMS_SCHEMA_SQL: &str = include_str!("../src/migrations/129_dos_7_claims_schema.sql");
const FEEDBACK_SCHEMA_SQL: &str = include_str!("../src/migrations/084_feedback_events.sql");
const MALFORMED_LOG_SQL: &str = include_str!("../src/migrations/126_suppression_malformed_log.sql");
const PROJECTION_STATUS_SQL: &str =
    include_str!("../src/migrations/134_dos_301_claim_projection_status.sql");
const TYPED_FEEDBACK_SQL: &str =
    include_str!("../src/migrations/135_dos_294_typed_feedback_schema.sql");

const PARENT_ACCOUNT_ID: &str = "dos287-example-parent";
const TARGET_ACCOUNT_ID: &str = "dos287-target-example";
const FOREIGN_ACCOUNT_ID: &str = "dos287-adjacent-example";
const TARGET_DOMAIN: &str = "target.example.org";
const FOREIGN_DOMAIN: &str = "adjacent.example.com";
const FOREIGN_INFRA_HOST: &str = "cluster-1.example.com";
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
fn bundle1_cross_entity_bleed_scores_low_and_clean_scores_high() {
    let conn = fresh_db();
    let db = ActionDb::from_conn(&conn);
    seed_bundle1_fixture(db);

    let bleed_text = trust_claim_text(&contaminated_target_enrichment());
    let clean_text = trust_claim_text(&prior_target_intelligence());

    let bleed_factor = cross_entity_coherence_factor(db, &bleed_text);
    let clean_factor = cross_entity_coherence_factor(db, &clean_text);

    assert!(
        bleed_factor <= 0.3,
        "bundle-1 bleed should score below the W4-A coherence threshold, got {bleed_factor}"
    );
    assert!(
        clean_factor >= 0.95,
        "clean bundle-1 target context should remain coherent, got {clean_factor}"
    );
}

fn cross_entity_coherence_factor(db: &ActionDb, claim_text: &str) -> f64 {
    let subject = SubjectRef::Account(TARGET_ACCOUNT_ID.to_string());
    let extraction = extract_target_footprint(db, &subject, "account", TARGET_ACCOUNT_ID)
        .expect("extract target trust footprint");
    let (target_footprint, portfolio_footprints) = match extraction {
        ExtractionOutcome::Ok {
            footprint,
            portfolio_footprints,
        } => (footprint, portfolio_footprints),
        ExtractionOutcome::SkipExtractorMismatch { reason } => {
            panic!("bundle-1 trust footprint should extract cleanly, got {reason:?}")
        }
    };
    assert!(
        portfolio_footprints
            .iter()
            .any(|footprint| footprint.subject
                == SubjectRef::Account(FOREIGN_ACCOUNT_ID.to_string())),
        "bundle-1 fixture must include adjacent account in portfolio footprints"
    );

    let claim = trust_claim(claim_text);
    let computation = compile_trust(
        &claim,
        TrustContext {
            now: Utc.with_ymd_and_hms(2026, 5, 4, 13, 0, 0).unwrap(),
            config: TrustConfig::default(),
            renewal_context: None,
            factor_inputs: TrustFactorInputs {
                source_reliability: 1.0,
                source_reliability_corroborators: Vec::new(),
                freshness: FreshnessContext {
                    timestamp_known: true,
                    age_days: 0.0,
                },
                corroboration_strength: 1.0,
                contradiction_count: 0,
                user_feedback: UserFeedbackSignal::None,
                subject_fit_confidence: 1.0,
                internal_consistency: 1.0,
                source_lifecycle: SourceLifecycleState::Active,
                read_state_indeterminate: false,
            },
            cross_entity: CrossEntityCoherenceInput {
                claim_text: claim.text.clone(),
                target_footprint,
                portfolio_footprints,
                cross_entity_context_expected: false,
            },
            target_surface: None,
        },
    )
    .expect("compile trust");

    trust_factor_raw_value(&computation, "cross_entity_coherence")
}

fn trust_factor_raw_value(computation: &TrustComputation, name: &str) -> f64 {
    computation
        .evidence
        .factor_breakdown
        .iter()
        .find(|factor| factor.name == name)
        .map(|factor| factor.raw_value)
        .expect("trust factor should be present")
}

fn trust_claim(text: &str) -> IntelligenceClaim {
    IntelligenceClaim {
        id: "dos287-bundle1-claim".to_string(),
        subject_ref: serde_json::json!({
            "kind": "account",
            "id": TARGET_ACCOUNT_ID,
        })
        .to_string(),
        claim_type: "risk".to_string(),
        field_path: Some("risks.summary".to_string()),
        topic_key: None,
        text: text.to_string(),
        dedup_key: "dos287-bundle1-dedup".to_string(),
        item_hash: Some("dos287-bundle1-hash".to_string()),
        actor: "agent:test".to_string(),
        data_source: "test-fixture".to_string(),
        source_ref: None,
        source_asof: Some("2026-05-04T12:00:00Z".to_string()),
        observed_at: "2026-05-04T12:00:00Z".to_string(),
        created_at: "2026-05-04T12:00:00Z".to_string(),
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
        sensitivity: ClaimSensitivity::Internal,
        verification_state: ClaimVerificationState::Active,
        verification_reason: None,
        needs_user_decision_at: None,
    }
}

fn trust_claim_text(intel: &IntelligenceJson) -> String {
    let mut parts = Vec::new();
    if let Some(text) = intel.executive_assessment.as_deref() {
        parts.push(text.to_string());
    }
    parts.extend(intel.risks.iter().map(|risk| risk.text.clone()));
    parts.extend(intel.recent_wins.iter().map(|win| win.text.clone()));
    for stakeholder in &intel.stakeholder_insights {
        parts.push(stakeholder.name.clone());
        if let Some(role) = stakeholder.role.as_deref() {
            parts.push(role.to_string());
        }
        if let Some(engagement) = stakeholder.engagement.as_deref() {
            parts.push(engagement.to_string());
        }
        if let Some(assessment) = stakeholder.assessment.as_deref() {
            parts.push(assessment.to_string());
        }
    }
    parts.join("\n")
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
            params![PARENT_ACCOUNT_ID, "Example Portfolio"],
        )
        .expect("seed parent account");
    db.conn_ref()
        .execute(
            "INSERT INTO accounts (id, name, parent_id, account_type, updated_at, archived)
             VALUES (?1, ?2, ?3, 'customer', '2026-05-04T12:00:00Z', 0)",
            params![TARGET_ACCOUNT_ID, "Target Example", PARENT_ACCOUNT_ID],
        )
        .expect("seed target account");
    db.conn_ref()
        .execute(
            "INSERT INTO accounts (id, name, parent_id, account_type, updated_at, archived)
             VALUES (?1, ?2, ?3, 'customer', '2026-05-04T12:00:00Z', 0)",
            params![FOREIGN_ACCOUNT_ID, "Adjacent Example", PARENT_ACCOUNT_ID],
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
            params![FOREIGN_ACCOUNT_ID, FOREIGN_INFRA_HOST],
        )
        .expect("seed foreign infrastructure host");

    seed_person(
        db,
        TARGET_STAKEHOLDER_ID,
        "Alice Adams",
        "alice@target.example.org",
    );
    seed_person(
        db,
        FOREIGN_STAKEHOLDER_ID,
        FOREIGN_STAKEHOLDER_NAME,
        "blake@adjacent.example.com",
    );
    seed_account_stakeholder(
        db,
        TARGET_ACCOUNT_ID,
        TARGET_STAKEHOLDER_ID,
        "trusted champion",
        "Alice owns the Target Example rollout.",
    );
    seed_account_stakeholder(
        db,
        FOREIGN_ACCOUNT_ID,
        FOREIGN_STAKEHOLDER_ID,
        "blocked",
        "Blake owns the Adjacent Example cluster migration.",
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
            "Target Example has steady adoption and an active renewal plan.".to_string(),
        ),
        risks: vec![IntelRisk {
            text: "Procurement timing is the only active renewal risk.".to_string(),
            item_source: Some(item_source("target-crm")),
            ..Default::default()
        }],
        recent_wins: vec![IntelWin {
            text: "Alice Adams expanded executive enablement for Target Example.".to_string(),
            item_source: Some(item_source("target-crm")),
            ..Default::default()
        }],
        stakeholder_insights: vec![StakeholderInsight {
            name: "Alice Adams".to_string(),
            person_id: Some(TARGET_STAKEHOLDER_ID.to_string()),
            role: Some("Champion".to_string()),
            engagement: Some("trusted champion".to_string()),
            assessment: Some("Alice owns the Target Example rollout.".to_string()),
            item_source: Some(item_source("target-crm")),
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
            "Target Example enrichment incorrectly says {FOREIGN_STAKEHOLDER_NAME} is \
             managing {FOREIGN_INFRA_HOST} launch risk for Adjacent Example at {FOREIGN_DOMAIN}."
        )),
        risks: vec![IntelRisk {
            text: format!("{FOREIGN_INFRA_HOST} cache instability is blocking the renewal."),
            item_source: Some(item_source("foreign-glean")),
            ..Default::default()
        }],
        recent_wins: vec![IntelWin {
            text: format!("{FOREIGN_STAKEHOLDER_NAME} completed Adjacent Example SSO validation."),
            item_source: Some(item_source("foreign-glean")),
            ..Default::default()
        }],
        stakeholder_insights: vec![StakeholderInsight {
            name: FOREIGN_STAKEHOLDER_NAME.to_string(),
            person_id: Some(FOREIGN_STAKEHOLDER_ID.to_string()),
            role: Some("Technical owner".to_string()),
            engagement: Some("blocked".to_string()),
            assessment: Some(format!(
                "{FOREIGN_STAKEHOLDER_NAME} is focused on {FOREIGN_INFRA_HOST} migration risk."
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
