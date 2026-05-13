use chrono::{TimeZone, Utc};
use dailyos_lib::db::claims::{ClaimSensitivity, TemporalScope};
use dailyos_lib::db::ActionDb;
use dailyos_lib::services::claims::{commit_claim, withdraw_claim, ClaimProposal, CommittedClaim};
use dailyos_lib::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};
use rusqlite::Connection;

const CLAIMS_SCHEMA_SQL: &str = include_str!("../src/migrations/129_dos_7_claims_schema.sql");
const PROJECTION_STATUS_SQL: &str =
    include_str!("../src/migrations/134_dos_301_claim_projection_status.sql");
const TYPED_FEEDBACK_SQL: &str =
    include_str!("../src/migrations/135_dos_294_typed_feedback_schema.sql");
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
"#;

fn setup_conn() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory db");
    conn.execute_batch(
        "CREATE TABLE accounts (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            claim_version INTEGER NOT NULL DEFAULT 0
        );",
    )
    .expect("create account table");
    conn.execute_batch(CLAIMS_SCHEMA_SQL)
        .expect("apply claims schema");
    conn.execute_batch(PROJECTION_STATUS_SQL)
        .expect("apply projection status schema");
    conn.execute_batch(TYPED_FEEDBACK_SQL)
        .expect("apply typed feedback schema");
    conn.execute_batch(STRUCTURED_CLAIM_CANONICALIZATION_COLUMNS_SQL)
        .expect("apply structured claim canonicalization columns");
    conn.execute(
        "INSERT INTO accounts (id, name, updated_at) VALUES ('acct-dos411', 'Test Account', '2026-05-06T12:00:00Z')",
        [],
    )
    .expect("seed account");
    conn
}

fn ctx<'a>(
    clock: &'a FixedClock,
    rng: &'a SeedableRng,
    external: &'a ExternalClients,
) -> ServiceContext<'a> {
    ServiceContext::new_live(clock, rng, external).with_actor("user")
}

fn proposal(text: &str, observed_at: &str, supersedes: Option<&str>) -> ClaimProposal {
    ClaimProposal {
        id: None,
        subject_ref: serde_json::json!({
            "kind": "account",
            "id": "acct-dos411",
        })
        .to_string(),
        claim_type: "user_note".to_string(),
        field_path: None,
        topic_key: None,
        text: text.to_string(),
        actor: "user".to_string(),
        data_source: "manual".to_string(),
        source_ref: None,
        source_asof: Some(observed_at.to_string()),
        observed_at: observed_at.to_string(),
        provenance_json: "{}".to_string(),
        metadata_json: Some(serde_json::json!({ "title": "Lifecycle note" }).to_string()),
        thread_id: None,
        temporal_scope: Some(TemporalScope::State),
        sensitivity: Some(ClaimSensitivity::Internal),
        supersedes: supersedes.map(str::to_string),
        tombstone: None,
    }
}

fn inserted_id(result: CommittedClaim) -> String {
    match result {
        CommittedClaim::Inserted { claim } => claim.id,
        other => panic!("expected inserted claim, got {other:?}"),
    }
}

#[test]
fn user_note_commit_supersede_withdraw_lifecycle() {
    let conn = setup_conn();
    let db = ActionDb::from_conn(&conn);
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 0).unwrap());
    let rng = SeedableRng::new(411);
    let external = ExternalClients::default();
    let ctx = ctx(&clock, &rng, &external);

    let first = inserted_id(
        commit_claim(
            &ctx,
            db,
            proposal("first note", "2026-05-06T12:00:00Z", None),
        )
        .expect("commit initial user_note"),
    );

    let second = inserted_id(
        commit_claim(
            &ctx,
            db,
            proposal("edited note", "2026-05-06T12:01:00Z", Some(&first)),
        )
        .expect("commit superseding user_note"),
    );

    let old_lifecycle: (String, String, Option<String>, Option<String>) = conn
        .query_row(
            "SELECT claim_state, surfacing_state, demotion_reason, superseded_by
             FROM intelligence_claims WHERE id = ?1",
            [&first],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("read superseded lifecycle");
    assert_eq!(
        old_lifecycle,
        (
            "dormant".to_string(),
            "dormant".to_string(),
            Some("superseded".to_string()),
            Some(second.clone()),
        )
    );

    let supersession_edges: i64 = conn
        .query_row(
            "SELECT count(*) FROM claim_contradictions
             WHERE primary_claim_id = ?1
               AND contradicting_claim_id = ?2
               AND branch_kind = 'supersession'",
            [&first, &second],
            |row| row.get(0),
        )
        .expect("read supersession edge");
    assert_eq!(supersession_edges, 1);

    let withdrawn =
        withdraw_claim(&ctx, db, &second, "user_deleted").expect("withdraw edited user_note");
    assert_eq!(
        withdrawn.claim_state,
        dailyos_lib::db::claims::ClaimState::Withdrawn
    );
    assert_eq!(withdrawn.retraction_reason.as_deref(), Some("user_deleted"));

    let active_user_notes: i64 = conn
        .query_row(
            "SELECT count(*) FROM intelligence_claims
             WHERE claim_type = 'user_note'
               AND claim_state = 'active'
               AND surfacing_state = 'active'",
            [],
            |row| row.get(0),
        )
        .expect("count active user notes");
    assert_eq!(active_user_notes, 0);
}
