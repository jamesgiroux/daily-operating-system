use chrono::{TimeZone, Utc};
use dailyos_lib::abilities::composition::{
    AbilityRef, Composition, CompositionDocId, CompositionKind, CompositionMetadata,
    CompositionVersion,
};
use dailyos_lib::abilities::provenance::SchemaVersion;
use dailyos_lib::db::claims::{ClaimSensitivity, TemporalScope};
use dailyos_lib::db::ActionDb;
use dailyos_lib::doctor::inspect_watermarks;
use dailyos_lib::migration_test_api::run_migrations;
use dailyos_lib::services::claims::{commit_claim, ClaimProposal, CommittedClaim};
use dailyos_lib::services::compositions::{
    commit_composition, CompositionError, CompositionProposal,
};
use dailyos_lib::services::context::{
    Clock, ExternalClients, FixedClock, SeedableRng, ServiceContext,
};
use rusqlite::{params, Connection};

fn fresh_full_db() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory db");
    run_migrations(&conn).expect("apply production migrations");
    conn
}

fn ctx_parts() -> (FixedClock, SeedableRng, ExternalClients) {
    (
        FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 13, 12, 0, 0).unwrap()),
        SeedableRng::new(17),
        ExternalClients::default(),
    )
}

fn live_ctx<'a>(
    clock: &'a FixedClock,
    rng: &'a SeedableRng,
    external: &'a ExternalClients,
) -> ServiceContext<'a> {
    ServiceContext::new_live(clock, rng, external).with_actor("agent:test_composer")
}

fn subject_ref(account_id: &str) -> String {
    serde_json::json!({
        "kind": "account",
        "id": account_id,
    })
    .to_string()
}

fn risk_claim(ctx: &ServiceContext<'_>, account_id: &str) -> ClaimProposal {
    let observed_at = ctx.clock.now().to_rfc3339();
    ClaimProposal {
        id: None,
        expected_claim_version: None,
        subject_ref: subject_ref(account_id),
        claim_type: "risk".to_string(),
        field_path: Some("risks".to_string()),
        topic_key: None,
        text: "renewal timing requires attention".to_string(),
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

// Composition::empty is invoked directly at each call site below — a local
// wrapper helper would re-introduce a return-type signature that the
// substrate-authorship lint at scripts/check_composition_authorship.sh
// flags (ADR-0130 §1). Substrate authorship stays inside abilities-runtime.

#[test]
fn dos567_fresh_claim_insert_gets_version_one_and_outbox_event() {
    let conn = fresh_full_db();
    conn.execute(
        "INSERT INTO accounts (id, name, updated_at) VALUES (?1, ?2, ?3)",
        params!["acct-dos567", "Example Account", "2026-05-13T12:00:00Z"],
    )
    .expect("seed account");
    let (clock, rng, external) = ctx_parts();
    let ctx = live_ctx(&clock, &rng, &external);

    let committed = commit_claim(
        &ctx,
        ActionDb::from_conn(&conn),
        risk_claim(&ctx, "acct-dos567"),
    )
    .expect("commit claim");
    let claim = match committed {
        CommittedClaim::Inserted { claim } => claim,
        other => panic!("expected inserted claim, got {other:?}"),
    };
    assert_eq!(claim.claim_version, 1);

    let (stored_version, event_count, attempt_count): (i64, i64, i64) = conn
        .query_row(
            "SELECT c.claim_version,
                    (SELECT COUNT(*) FROM version_events ve
                     WHERE ve.claim_id = c.id AND ve.current_version = c.claim_version),
                    (SELECT COUNT(*) FROM mutation_attempts ma
                     JOIN version_events ve ON ve.mutation_id = ma.mutation_id
                     WHERE ve.claim_id = c.id AND ma.status = 'committed')
             FROM intelligence_claims c
             WHERE c.id = ?1",
            params![claim.id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("version row");
    assert_eq!(stored_version, 1);
    assert_eq!(event_count, 1);
    assert_eq!(attempt_count, 1);
}

#[test]
fn dos567_composition_commit_assigns_versions_and_rejects_stale() {
    let conn = fresh_full_db();
    let db = ActionDb::from_conn(&conn);
    let (clock, rng, external) = ctx_parts();
    let ctx = live_ctx(&clock, &rng, &external);

    let bootstrap = CompositionProposal {
        composition_id: CompositionDocId::new("comp-dos567"),
        expected_composition_version: 0,
        composition: Composition::empty(CompositionDocId::new("comp-dos567"), CompositionVersion::new(41), clock.now()),
    };
    let committed = commit_composition(&ctx, db, bootstrap).expect("bootstrap composition");
    assert_eq!(committed.composition_version, 1);
    assert_eq!(committed.composition.metadata.composition_version.0, 1);

    let stale = CompositionProposal {
        composition_id: CompositionDocId::new("comp-dos567"),
        expected_composition_version: 0,
        composition: Composition::empty(CompositionDocId::new("comp-dos567"), CompositionVersion::new(1), clock.now()),
    };
    let error = commit_composition(&ctx, db, stale).expect_err("stale composition rejected");
    assert!(matches!(
        error,
        CompositionError::StaleVersion {
            expected: 0,
            current: 1,
            ..
        }
    ));

    let next = CompositionProposal {
        composition_id: CompositionDocId::new("comp-dos567"),
        expected_composition_version: 1,
        composition: Composition::empty(CompositionDocId::new("comp-dos567"), CompositionVersion::new(1), clock.now()),
    };
    let committed = commit_composition(&ctx, db, next).expect("second composition commit");
    assert_eq!(committed.composition_version, 2);

    let event_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM version_events
             WHERE composition_id = 'comp-dos567' AND event_kind = 'composition.updated'",
            [],
            |row| row.get(0),
        )
        .expect("composition events");
    assert_eq!(event_count, 2);
}

#[test]
fn dos567_doctor_accepts_clean_claim_and_composition_watermarks() {
    let conn = fresh_full_db();
    conn.execute(
        "INSERT INTO accounts (id, name, updated_at) VALUES (?1, ?2, ?3)",
        params!["acct-doctor", "Doctor Example", "2026-05-13T12:00:00Z"],
    )
    .expect("seed account");
    let db = ActionDb::from_conn(&conn);
    let (clock, rng, external) = ctx_parts();
    let ctx = live_ctx(&clock, &rng, &external);

    commit_claim(&ctx, db, risk_claim(&ctx, "acct-doctor")).expect("commit claim");
    commit_composition(
        &ctx,
        db,
        CompositionProposal {
            composition_id: CompositionDocId::new("comp-doctor"),
            expected_composition_version: 0,
            composition: Composition::empty(CompositionDocId::new("comp-doctor"), CompositionVersion::new(0), clock.now()),
        },
    )
    .expect("commit composition");

    let report = inspect_watermarks(db).expect("doctor report");
    assert_eq!(report.claims_below_floor, 0);
    assert_eq!(report.compositions_below_floor, 0);
    assert_eq!(report.zombie_attempts, 0);
    assert_eq!(report.claims_missing_outbox, 0);
    assert_eq!(report.compositions_missing_outbox, 0);
}
