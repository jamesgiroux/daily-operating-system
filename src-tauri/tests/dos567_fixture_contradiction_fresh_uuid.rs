//! Contradiction/fork path must mint a fresh UUID for the new row when the
//! routing target is `Mutate`.
//!
//! Bug shape: when a caller submits `ClaimProposal { id: Some(existing_id),
//! expected_claim_version: Some(current) }` and the text differs enough to
//! route through the contradiction branch, `proposal.id` is the EXISTING
//! claim's id. Reusing it for the new contradicting row would PK-collide
//! with the existing claim row in `intelligence_claims`.
//!
//! Expected: the new contradicting row carries a freshly-minted UUID
//! distinct from the Mutate target id, and the commit succeeds without
//! PK collision.
//!
//! Fix lives in `commit_claim`'s contradiction branch: the new-row id is
//! generated unconditionally for `ClaimMutationTarget::Mutate` rather than
//! reusing `proposal.id`.

use chrono::{TimeZone, Utc};
use dailyos_lib::db::claims::{ClaimSensitivity, TemporalScope};
use dailyos_lib::db::ActionDb;
use dailyos_lib::migration_test_api::run_migrations;
use dailyos_lib::services::claims::{commit_claim, ClaimProposal, CommittedClaim};
use dailyos_lib::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};
use rusqlite::{params, Connection};

fn fresh_full_db() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory db");
    run_migrations(&conn).expect("apply production migrations");
    conn
}

fn ctx_parts() -> (FixedClock, SeedableRng, ExternalClients) {
    (
        FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 13, 12, 0, 0).unwrap()),
        SeedableRng::new(606),
        ExternalClients::default(),
    )
}

fn live_ctx<'a>(
    clock: &'a FixedClock,
    rng: &'a SeedableRng,
    external: &'a ExternalClients,
) -> ServiceContext<'a> {
    ServiceContext::new_live(clock, rng, external).with_actor("agent:test_contradiction_uuid")
}

fn subject_ref(account_id: &str) -> String {
    serde_json::json!({ "kind": "account", "id": account_id }).to_string()
}

fn risk_proposal(
    account_id: &str,
    id: Option<String>,
    expected: Option<u64>,
    text: &str,
) -> ClaimProposal {
    let observed_at = "2026-05-13T12:00:00+00:00".to_string();
    ClaimProposal {
        id,
        expected_claim_version: expected,
        subject_ref: subject_ref(account_id),
        claim_type: "risk".to_string(),
        field_path: Some("risks".to_string()),
        topic_key: None,
        text: text.to_string(),
        actor: "agent:test".to_string(),
        data_source: "test".to_string(),
        source_ref: None,
        source_asof: Some(observed_at.clone()),
        observed_at,
        provenance_json: "{}".to_string(),
        metadata_json: None,
        thread_id: None,
        temporal_scope: Some(TemporalScope::State),
        sensitivity: Some(ClaimSensitivity::Internal),
        supersedes: None,
        tombstone: None,
    }
}

#[test]
fn dos567_contradiction_fork_via_mutate_target_mints_fresh_uuid_no_pk_collision() {
    let conn = fresh_full_db();
    conn.execute(
        "INSERT INTO accounts (id, name, updated_at) VALUES (?1, ?2, ?3)",
        params![
            "acct-contradiction",
            "Contradiction Example",
            "2026-05-13T12:00:00Z"
        ],
    )
    .expect("seed account");

    let (clock, rng, external) = ctx_parts();
    let ctx = live_ctx(&clock, &rng, &external);
    let db = ActionDb::from_conn(&conn);

    // Bootstrap an existing claim ("renewal risk: X") at v=1.
    let bootstrap = commit_claim(
        &ctx,
        db,
        risk_proposal("acct-contradiction", None, None, "renewal risk: X"),
    )
    .expect("bootstrap insert");
    let existing_id = match bootstrap {
        CommittedClaim::Inserted { claim } => claim.id,
        other => panic!("expected Inserted, got {other:?}"),
    };
    let existing_version: i64 = conn
        .query_row(
            "SELECT claim_version FROM intelligence_claims WHERE id = ?1",
            params![existing_id],
            |row| row.get(0),
        )
        .expect("existing version");
    assert_eq!(existing_version, 1);

    // Mutate the same claim with DIFFERENT text — this drives the
    // contradiction branch in `commit_claim` because dedup_key (based on
    // item_hash) differs while (subject_ref, claim_type, field_path) match
    // the existing claim. Pre-fix, `proposal.id = Some(existing_id)` would
    // be reused for the NEW contradicting row, PK-colliding against the
    // existing claim row.
    let mutate = risk_proposal(
        "acct-contradiction",
        Some(existing_id.clone()),
        Some(1),
        "renewal risk: completely different framing",
    );
    let committed = commit_claim(&ctx, db, mutate).expect("contradiction fork must not PK-collide");

    let (primary_id, new_id) = match committed {
        CommittedClaim::Forked {
            primary_claim,
            new_claim_id,
            ..
        } => (primary_claim.id, new_claim_id),
        CommittedClaim::Inserted { claim } => {
            // Some semantic-canonicalization paths upstream of the
            // contradiction branch may absorb the proposal as a new row
            // without the Forked envelope. Either way, the new row must
            // not share the existing claim id.
            (existing_id.clone(), claim.id)
        }
        other => panic!("expected Forked or Inserted, got {other:?}"),
    };

    assert_eq!(primary_id, existing_id);
    assert_ne!(
        new_id, existing_id,
        "contradicting row must mint a fresh UUID, not reuse Mutate target id"
    );

    // Both rows live in intelligence_claims: the original (untouched id)
    // and the new contradicting row at a distinct id. No PK collision.
    let rows: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM intelligence_claims WHERE id IN (?1, ?2)",
            params![existing_id, new_id],
            |row| row.get(0),
        )
        .expect("count claim rows");
    assert_eq!(rows, 2, "both existing and new contradicting rows present");
}
