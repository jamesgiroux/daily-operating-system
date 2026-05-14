//! Claims ghost-resurrection 5-run simulation.
//!
//! Asserts the tombstone PRE-GATE in commit_claim prevents an AI-surfaced
//! risk that the user dismissed from re-appearing across multiple
//! enrichment cycles. Load-bearing proof of the claim substrate's core invariant.

use chrono::{Duration, TimeZone, Utc};
use dailyos_lib::db::claims::{ClaimSensitivity, TemporalScope};
use dailyos_lib::db::ActionDb;
use dailyos_lib::migration_test_api::run_migrations;
use dailyos_lib::services::claims::{
    commit_claim, ClaimError, ClaimProposal, CommittedClaim, TombstoneSpec,
};
use dailyos_lib::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};
use rusqlite::{params, Connection};

const ORIGINAL_RISK: &str = "ARR at risk due to renewal slip";
const PARAPHRASED_RISK: &str = "Renewal slip puts ARR at risk";
const DISTINCT_PARAPHRASED_RISK: &str = "Renewal slippage increases ARR risk";

fn fresh_full_db() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory db");
    // Running the full migration set ensures the schema stays in lockstep with
    // production, including post-v167 tables that commit_claim writes to via the
    // v170 canonicalization cutover (canonicalization_decisions, ambiguous_claim_pairs).
    run_migrations(&conn).expect("apply production migrations");
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
        id: None,
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
        temporal_scope: Some(TemporalScope::State),
        sensitivity: Some(ClaimSensitivity::Internal),
        supersedes: None,
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
    assert_eq!(
        raw_active_risk_text_count_for(&conn, "acct-1", ORIGINAL_RISK),
        1
    );
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
    assert_eq!(
        tombstone_risk_text_count_for(&conn, "acct-1", ORIGINAL_RISK),
        1
    );
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
        "distinct paraphrases still bypass PRE-GATE until semantic canonicalization lands"
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
    assert_eq!(
        tombstone_risk_text_count_for(&conn, "acct-2", ORIGINAL_RISK),
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
