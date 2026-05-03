//! Claims ghost-resurrection 5-run simulation.
//!
//! Asserts the tombstone PRE-GATE in commit_claim prevents an AI-surfaced
//! risk that the user dismissed from re-appearing across multiple
//! enrichment cycles. Load-bearing proof of the claim substrate's core invariant.

use chrono::{Duration, TimeZone, Utc};
use dailyos_lib::db::claims::{ClaimSensitivity, TemporalScope};
use dailyos_lib::db::ActionDb;
use dailyos_lib::services::claims::{
    commit_claim, ClaimError, ClaimProposal, CommittedClaim, TombstoneSpec,
};
use dailyos_lib::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};
use rusqlite::{params, Connection};

const ORIGINAL_RISK: &str = "ARR at risk due to renewal slip";
const PARAPHRASED_RISK: &str = "Renewal slip puts ARR at risk";
const DISTINCT_PARAPHRASED_RISK: &str = "Renewal slippage increases ARR risk";

const D1_CLAIMS_SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS intelligence_claims (
    id              TEXT PRIMARY KEY,
    subject_ref     TEXT NOT NULL,
    claim_type      TEXT NOT NULL,
    field_path      TEXT,
    topic_key       TEXT,
    text            TEXT NOT NULL,
    dedup_key       TEXT NOT NULL,
    item_hash       TEXT,
    actor           TEXT NOT NULL,
    data_source     TEXT NOT NULL,
    source_ref      TEXT,
    source_asof     TEXT,
    observed_at     TEXT NOT NULL,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    provenance_json TEXT NOT NULL,
    metadata_json   TEXT,

    claim_state         TEXT NOT NULL DEFAULT 'active'
                                  CHECK (claim_state IN ('active', 'dormant', 'tombstoned', 'withdrawn')),
    surfacing_state     TEXT NOT NULL DEFAULT 'active'
                                  CHECK (surfacing_state IN ('active', 'dormant')),
    demotion_reason     TEXT,
    reactivated_at      TEXT,
    retraction_reason   TEXT,
    expires_at          TEXT,
    superseded_by       TEXT,

    trust_score         REAL,
    trust_computed_at   TEXT,
    trust_version       INTEGER,

    thread_id           TEXT,
    temporal_scope      TEXT NOT NULL DEFAULT 'state'
                                  CHECK (temporal_scope IN ('state', 'point_in_time', 'trend')),
    sensitivity         TEXT NOT NULL DEFAULT 'internal'
                                  CHECK (sensitivity IN ('public', 'internal', 'confidential', 'user_only'))
);

CREATE INDEX IF NOT EXISTS idx_claims_default_read
    ON intelligence_claims(subject_ref, claim_state, surfacing_state, claim_type);

CREATE INDEX IF NOT EXISTS idx_claims_suppression_lookup
    ON intelligence_claims(subject_ref, claim_type, field_path, claim_state, dedup_key);

CREATE INDEX IF NOT EXISTS idx_claims_dedup_key
    ON intelligence_claims(dedup_key)
    WHERE claim_state = 'active';

CREATE INDEX IF NOT EXISTS idx_claims_thread_id
    ON intelligence_claims(thread_id)
    WHERE thread_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_claims_superseded_by
    ON intelligence_claims(superseded_by)
    WHERE superseded_by IS NOT NULL;

CREATE TABLE IF NOT EXISTS claim_corroborations (
    id                    TEXT PRIMARY KEY,
    claim_id              TEXT NOT NULL REFERENCES intelligence_claims(id),
    data_source           TEXT NOT NULL,
    source_asof           TEXT,
    source_mechanism      TEXT,
    strength              REAL NOT NULL DEFAULT 0.5
                                    CHECK (strength >= 0.0 AND strength <= 1.0),
    reinforcement_count   INTEGER NOT NULL DEFAULT 1,
    last_reinforced_at    TEXT NOT NULL DEFAULT (datetime('now')),
    created_at            TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_corroborations_claim
    ON claim_corroborations(claim_id);

CREATE INDEX IF NOT EXISTS idx_corroborations_source
    ON claim_corroborations(claim_id, data_source);

CREATE TABLE IF NOT EXISTS claim_contradictions (
    id                     TEXT PRIMARY KEY,
    primary_claim_id       TEXT NOT NULL REFERENCES intelligence_claims(id),
    contradicting_claim_id TEXT NOT NULL REFERENCES intelligence_claims(id),
    branch_kind            TEXT NOT NULL
                                  CHECK (branch_kind IN ('contradiction', 'clarification', 'supersession')),
    detected_at            TEXT NOT NULL DEFAULT (datetime('now')),
    reconciliation_kind    TEXT
                                  CHECK (reconciliation_kind IS NULL OR reconciliation_kind IN
                                         ('user_picked_winner', 'evidence_converged', 'merged_as_qualified', 'both_dormant')),
    reconciliation_note    TEXT,
    reconciled_at          TEXT,
    winner_claim_id        TEXT REFERENCES intelligence_claims(id),
    merged_claim_id        TEXT REFERENCES intelligence_claims(id)
);

CREATE INDEX IF NOT EXISTS idx_contradictions_primary
    ON claim_contradictions(primary_claim_id);

CREATE INDEX IF NOT EXISTS idx_contradictions_unreconciled
    ON claim_contradictions(reconciled_at)
    WHERE reconciled_at IS NULL;

CREATE TABLE IF NOT EXISTS agent_trust_ledger (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_kind         TEXT NOT NULL,
    agent_id           TEXT NOT NULL,
    claim_type         TEXT,
    correct_count      INTEGER NOT NULL DEFAULT 0,
    incorrect_count    INTEGER NOT NULL DEFAULT 0,
    total_count        INTEGER NOT NULL DEFAULT 0,
    last_updated_at    TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE (agent_kind, agent_id, claim_type)
);

CREATE INDEX IF NOT EXISTS idx_agent_trust_lookup
    ON agent_trust_ledger(agent_kind, agent_id);

CREATE TABLE IF NOT EXISTS claim_feedback (
    id              TEXT PRIMARY KEY,
    claim_id        TEXT NOT NULL REFERENCES intelligence_claims(id),
    feedback_type   TEXT NOT NULL
                              CHECK (feedback_type IN ('confirm', 'correct', 'reject', 'wrong_subject', 'cannot_verify')),
    actor           TEXT NOT NULL,
    actor_id        TEXT,
    payload_json    TEXT,
    submitted_at    TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_feedback_claim
    ON claim_feedback(claim_id);

CREATE INDEX IF NOT EXISTS idx_feedback_type
    ON claim_feedback(feedback_type, submitted_at);

CREATE TABLE IF NOT EXISTS claim_repair_job (
    id                   TEXT PRIMARY KEY,
    claim_id             TEXT NOT NULL REFERENCES intelligence_claims(id),
    feedback_id          TEXT REFERENCES claim_feedback(id),
    state                TEXT NOT NULL DEFAULT 'pending'
                                  CHECK (state IN ('pending', 'in_progress', 'completed', 'failed', 'budget_exhausted')),
    attempts             INTEGER NOT NULL DEFAULT 0,
    max_attempts         INTEGER NOT NULL DEFAULT 3,
    last_attempt_at      TEXT,
    completed_at         TEXT,
    error_message        TEXT,
    created_at           TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_repair_pending
    ON claim_repair_job(state, created_at)
    WHERE state IN ('pending', 'in_progress');
"#;

// The live invalidation migration does not create a `claim_invalidation` table; it adds
// per-entity `claim_version` columns plus shared `migration_state` rows.
// `commit_claim` reaches this surface through `ActionDb::bump_for_subject`.
const W1_CLAIM_INVALIDATION_SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS accounts (
    id TEXT PRIMARY KEY,
    claim_version INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY,
    claim_version INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS people (
    id TEXT PRIMARY KEY,
    claim_version INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS meetings (
    id TEXT PRIMARY KEY,
    claim_version INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS migration_state (
    key   TEXT PRIMARY KEY,
    value INTEGER NOT NULL
);

INSERT OR IGNORE INTO migration_state (key, value) VALUES ('global_claim_epoch', 0);
INSERT OR IGNORE INTO migration_state (key, value) VALUES ('schema_epoch', 1);
"#;

fn fresh_full_db() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory db");
    conn.execute_batch(W1_CLAIM_INVALIDATION_SCHEMA_SQL)
        .expect("apply W1 invalidation schema");
    conn.execute_batch(D1_CLAIMS_SCHEMA_SQL)
        .expect("apply D1 claims schema");
    conn
}

fn seed_account(conn: &Connection, account_id: &str) {
    conn.execute(
        "INSERT OR IGNORE INTO accounts (id) VALUES (?1)",
        params![account_id],
    )
    .expect("seed account");
}

fn ctx_parts() -> (FixedClock, SeedableRng, ExternalClients) {
    (
        FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 2, 12, 0, 0).unwrap()),
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

fn advance_pass(clock: &FixedClock) {
    clock.advance(Duration::hours(1));
}

fn subject_ref(account_id: &str) -> String {
    serde_json::to_string(&serde_json::json!({
        "kind": "account",
        "id": account_id,
    }))
    .expect("serialize subject_ref")
}

fn claim_proposal(
    ctx: &ServiceContext<'_>,
    account_id: &str,
    actor: &str,
    data_source: &str,
    text: &str,
    is_dismissal: bool,
) -> ClaimProposal {
    let observed_at = ctx.clock.now().to_rfc3339();
    ClaimProposal {
        subject_ref: subject_ref(account_id),
        claim_type: "risk".to_string(),
        field_path: Some("risks".to_string()),
        topic_key: None,
        text: text.to_string(),
        actor: actor.to_string(),
        data_source: data_source.to_string(),
        source_ref: None,
        source_asof: Some(observed_at.clone()),
        observed_at,
        provenance_json: "{}".to_string(),
        metadata_json: None,
        thread_id: None,
        temporal_scope: TemporalScope::State,
        sensitivity: ClaimSensitivity::Internal,
        tombstone: is_dismissal.then(|| TombstoneSpec {
            retraction_reason: "user_removal".to_string(),
            expires_at: None,
        }),
    }
}

fn commit_claim_attempt(
    ctx: &ServiceContext<'_>,
    conn: &Connection,
    account_id: &str,
    actor: &str,
    data_source: &str,
    text: &str,
    is_dismissal: bool,
) -> Result<CommittedClaim, ClaimError> {
    commit_claim(
        ctx,
        ActionDb::from_conn(conn),
        claim_proposal(ctx, account_id, actor, data_source, text, is_dismissal),
    )
}

fn assert_inserted(result: Result<CommittedClaim, ClaimError>) {
    match result {
        Ok(CommittedClaim::Inserted { .. }) => {}
        other => panic!("expected inserted claim, got {other:?}"),
    }
}

fn assert_tombstoned(result: Result<CommittedClaim, ClaimError>) {
    match result {
        Ok(CommittedClaim::Tombstoned { .. }) => {}
        other => panic!("expected tombstoned claim, got {other:?}"),
    }
}

fn assert_tombstone_pre_gate(result: Result<CommittedClaim, ClaimError>) {
    match result {
        Err(ClaimError::TombstonedPreGate) => {}
        other => panic!("expected TombstonedPreGate, got {other:?}"),
    }
}

fn raw_active_risk_text_count_for(conn: &Connection, account_id: &str, text: &str) -> i64 {
    conn.query_row(
        "SELECT count(*)
         FROM intelligence_claims
         WHERE subject_ref = ?1
           AND claim_type = 'risk'
           AND field_path = 'risks'
           AND claim_state = 'active'
           AND surfacing_state = 'active'
           AND text = ?2 COLLATE NOCASE",
        params![subject_ref(account_id), text],
        |row| row.get(0),
    )
    .expect("count raw active risk text")
}

fn tombstone_risk_text_count_for(conn: &Connection, account_id: &str, text: &str) -> i64 {
    conn.query_row(
        "SELECT count(*)
         FROM intelligence_claims
         WHERE subject_ref = ?1
           AND claim_type = 'risk'
           AND field_path = 'risks'
           AND claim_state = 'tombstoned'
           AND retraction_reason = 'user_removal'
           AND text = ?2 COLLATE NOCASE",
        params![subject_ref(account_id), text],
        |row| row.get(0),
    )
    .expect("count tombstone risk text")
}

fn visible_active_risk_count_for(conn: &Connection, account_id: &str) -> i64 {
    conn.query_row(
        "SELECT count(*)
         FROM intelligence_claims active
         WHERE active.subject_ref = ?1
           AND active.claim_type = 'risk'
           AND active.field_path = 'risks'
           AND active.claim_state = 'active'
           AND active.surfacing_state = 'active'
           AND NOT EXISTS (
               SELECT 1
               FROM intelligence_claims tombstone
               WHERE tombstone.dedup_key = active.dedup_key
                 AND tombstone.claim_state = 'tombstoned'
                 AND (tombstone.expires_at IS NULL OR tombstone.expires_at > ?2)
           )",
        params![subject_ref(account_id), Utc::now().to_rfc3339()],
        |row| row.get(0),
    )
    .expect("count visible active risks")
}

fn visible_active_risk_text_count_for(conn: &Connection, account_id: &str, text: &str) -> i64 {
    conn.query_row(
        "SELECT count(*)
         FROM intelligence_claims active
         WHERE active.subject_ref = ?1
           AND active.claim_type = 'risk'
           AND active.field_path = 'risks'
           AND active.claim_state = 'active'
           AND active.surfacing_state = 'active'
           AND active.text = ?2 COLLATE NOCASE
           AND NOT EXISTS (
               SELECT 1
               FROM intelligence_claims tombstone
               WHERE tombstone.dedup_key = active.dedup_key
                 AND tombstone.claim_state = 'tombstoned'
                 AND (tombstone.expires_at IS NULL OR tombstone.expires_at > ?3)
           )",
        params![subject_ref(account_id), text, Utc::now().to_rfc3339()],
        |row| row.get(0),
    )
    .expect("count visible active risk text")
}

#[test]
fn five_run_ghost_resurrection_simulation_keeps_tombstoned_risk_dead() {
    let conn = fresh_full_db();
    seed_account(&conn, "acct-1");
    let (clock, rng, external) = ctx_parts();
    let ctx = live_ctx(&clock, &rng, &external);

    assert_inserted(commit_claim_attempt(
        &ctx,
        &conn,
        "acct-1",
        "ai",
        "glean",
        ORIGINAL_RISK,
        false,
    ));
    assert_eq!(raw_active_risk_text_count_for(&conn, "acct-1", ORIGINAL_RISK), 1);
    assert_eq!(visible_active_risk_count_for(&conn, "acct-1"), 1);
    advance_pass(&clock);

    assert_tombstoned(commit_claim_attempt(
        &ctx,
        &conn,
        "acct-1",
        "user",
        "user_dismissal",
        ORIGINAL_RISK,
        true,
    ));
    assert_eq!(tombstone_risk_text_count_for(&conn, "acct-1", ORIGINAL_RISK), 1);
    assert_eq!(
        visible_active_risk_text_count_for(&conn, "acct-1", ORIGINAL_RISK),
        0
    );
    advance_pass(&clock);

    assert_tombstone_pre_gate(commit_claim_attempt(
        &ctx,
        &conn,
        "acct-1",
        "ai",
        "glean",
        ORIGINAL_RISK,
        false,
    ));
    assert_eq!(
        raw_active_risk_text_count_for(&conn, "acct-1", ORIGINAL_RISK),
        1,
        "PRE-GATE must prevent a second active original-risk row"
    );
    advance_pass(&clock);

    // Known canonicalization limitation: paraphrases have a different item_hash today,
    // so the tombstone PRE-GATE does not suppress them until canonicalization
    // learns same-meaning equivalence.
    assert_inserted(commit_claim_attempt(
        &ctx,
        &conn,
        "acct-1",
        "ai",
        "glean",
        PARAPHRASED_RISK,
        false,
    ));
    assert_eq!(
        visible_active_risk_text_count_for(&conn, "acct-1", PARAPHRASED_RISK),
        1
    );
    assert_eq!(visible_active_risk_count_for(&conn, "acct-1"), 1);
    advance_pass(&clock);

    assert_tombstone_pre_gate(commit_claim_attempt(
        &ctx,
        &conn,
        "acct-1",
        "ai",
        "glean",
        ORIGINAL_RISK,
        false,
    ));
    assert_eq!(visible_active_risk_count_for(&conn, "acct-1"), 1);
    assert_eq!(
        raw_active_risk_text_count_for(&conn, "acct-1", ORIGINAL_RISK),
        1,
        "exact-match retry must not insert another active original-risk row"
    );
}

#[test]
fn five_run_simulation_paraphrase_can_also_be_dismissed_independently() {
    let conn = fresh_full_db();
    seed_account(&conn, "acct-1");
    let (clock, rng, external) = ctx_parts();
    let ctx = live_ctx(&clock, &rng, &external);

    assert_inserted(commit_claim_attempt(
        &ctx,
        &conn,
        "acct-1",
        "ai",
        "glean",
        ORIGINAL_RISK,
        false,
    ));
    advance_pass(&clock);

    assert_tombstoned(commit_claim_attempt(
        &ctx,
        &conn,
        "acct-1",
        "user",
        "user_dismissal",
        ORIGINAL_RISK,
        true,
    ));
    advance_pass(&clock);

    assert_tombstone_pre_gate(commit_claim_attempt(
        &ctx,
        &conn,
        "acct-1",
        "ai",
        "glean",
        ORIGINAL_RISK,
        false,
    ));
    advance_pass(&clock);

    assert_inserted(commit_claim_attempt(
        &ctx,
        &conn,
        "acct-1",
        "ai",
        "glean",
        PARAPHRASED_RISK,
        false,
    ));
    assert_eq!(
        visible_active_risk_text_count_for(&conn, "acct-1", PARAPHRASED_RISK),
        1
    );
    advance_pass(&clock);

    assert_tombstoned(commit_claim_attempt(
        &ctx,
        &conn,
        "acct-1",
        "user",
        "user_dismissal",
        PARAPHRASED_RISK,
        true,
    ));
    assert_eq!(
        visible_active_risk_text_count_for(&conn, "acct-1", PARAPHRASED_RISK),
        0
    );
    advance_pass(&clock);

    assert_tombstone_pre_gate(commit_claim_attempt(
        &ctx,
        &conn,
        "acct-1",
        "ai",
        "glean",
        PARAPHRASED_RISK,
        false,
    ));
    assert_eq!(
        raw_active_risk_text_count_for(&conn, "acct-1", PARAPHRASED_RISK),
        1,
        "paraphrase tombstone must prevent a second active row for that exact paraphrase"
    );

    assert_inserted(commit_claim_attempt(
        &ctx,
        &conn,
        "acct-1",
        "ai",
        "glean",
        DISTINCT_PARAPHRASED_RISK,
        false,
    ));
    assert_eq!(
        visible_active_risk_text_count_for(&conn, "acct-1", DISTINCT_PARAPHRASED_RISK),
        1,
        "distinct paraphrases still bypass PRE-GATE until DOS-280 canonicalization lands"
    );
}

#[test]
fn five_run_simulation_pre_gate_does_not_block_different_subject() {
    let conn = fresh_full_db();
    seed_account(&conn, "acct-1");
    seed_account(&conn, "acct-2");
    let (clock, rng, external) = ctx_parts();
    let ctx = live_ctx(&clock, &rng, &external);

    assert_inserted(commit_claim_attempt(
        &ctx,
        &conn,
        "acct-1",
        "ai",
        "glean",
        ORIGINAL_RISK,
        false,
    ));
    advance_pass(&clock);

    assert_tombstoned(commit_claim_attempt(
        &ctx,
        &conn,
        "acct-1",
        "user",
        "user_dismissal",
        ORIGINAL_RISK,
        true,
    ));
    advance_pass(&clock);

    assert_tombstone_pre_gate(commit_claim_attempt(
        &ctx,
        &conn,
        "acct-1",
        "ai",
        "glean",
        ORIGINAL_RISK,
        false,
    ));
    advance_pass(&clock);

    assert_inserted(commit_claim_attempt(
        &ctx,
        &conn,
        "acct-2",
        "ai",
        "glean",
        ORIGINAL_RISK,
        false,
    ));
    assert_eq!(visible_active_risk_count_for(&conn, "acct-2"), 1);
    assert_eq!(tombstone_risk_text_count_for(&conn, "acct-2", ORIGINAL_RISK), 0);
    advance_pass(&clock);

    assert_tombstone_pre_gate(commit_claim_attempt(
        &ctx,
        &conn,
        "acct-1",
        "ai",
        "glean",
        ORIGINAL_RISK,
        false,
    ));
    assert_eq!(visible_active_risk_count_for(&conn, "acct-1"), 0);
    assert_eq!(
        raw_active_risk_text_count_for(&conn, "acct-1", ORIGINAL_RISK),
        1,
        "acct-1 exact-match retries must not insert extra active rows"
    );
    assert_eq!(
        raw_active_risk_text_count_for(&conn, "acct-2", ORIGINAL_RISK),
        1,
        "acct-1 tombstone must not block acct-2"
    );
}
