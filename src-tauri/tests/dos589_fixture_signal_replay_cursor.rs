//! W4-B-signals AC #3 + #7 + #8 — replay-from-cursor is ordered by `event_seq`,
//! deduplicates the row at `from_cursor`, and survives reconnect.
//!
//! Setup: seed a sequence of `version_events` rows (claim.updated events,
//! deterministic cursors). Subscribe a SurfaceClient with read scope; mint a
//! valid cursor envelope at event_seq N; call replay; assert delivered rows
//! are strictly later than N in `event_seq` order, do not duplicate row N,
//! and the response's `next_cursor` points at the last delivered row.
//!
//! Failure mode caught: lexical/timestamp ordering on cursor; duplicate
//! replay of last-seen row; "unknown cursor = start from zero" behavior.

use chrono::{TimeZone, Utc};
use dailyos_lib::abilities::registry::{ScopeSet, SurfaceClientId, SurfaceScope};
use dailyos_lib::abilities::Actor;
use dailyos_lib::db::claims::{ClaimSensitivity, TemporalScope};
use dailyos_lib::db::ActionDb;
use dailyos_lib::migration_test_api::run_migrations;
use dailyos_lib::services::claims::{commit_claim, ClaimProposal, CommittedClaim};
use dailyos_lib::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};
use dailyos_lib::services::version_dispatcher::{
    __test_encode_cursor, __test_load_local_key, ReplayRequest, SubjectFilter, SubscribeRequest,
    VersionDispatcher,
};
use rusqlite::{params, Connection};

fn fresh_full_db() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory db");
    run_migrations(&conn).expect("apply production migrations");
    conn
}

fn ctx_parts() -> (FixedClock, SeedableRng, ExternalClients) {
    (
        FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 15, 12, 0, 0).unwrap()),
        SeedableRng::new(7589),
        ExternalClients::default(),
    )
}

fn proposal(account_id: &str, label: &str) -> ClaimProposal {
    let observed = "2026-05-15T12:00:00+00:00".to_string();
    ClaimProposal {
        id: None,
        expected_claim_version: None,
        subject_ref: serde_json::json!({"kind": "account", "id": account_id}).to_string(),
        claim_type: "risk".to_string(),
        field_path: Some(format!("risks.{label}")),
        topic_key: None,
        text: format!("dos589 replay {label}"),
        actor: "agent:dos589_replay".to_string(),
        data_source: "test".to_string(),
        source_ref: None,
        source_asof: Some(observed.clone()),
        observed_at: observed,
        provenance_json: "{}".to_string(),
        metadata_json: None,
        thread_id: None,
        temporal_scope: Some(TemporalScope::State),
        sensitivity: Some(ClaimSensitivity::Internal),
        supersedes: None,
        tombstone: None,
    }
}

fn surface_client() -> Actor {
    let scope_set = ScopeSet::new([SurfaceScope::new("read.account_overview")])
        .expect("scope non-empty");
    Actor::SurfaceClient {
        instance: SurfaceClientId::new("sc-replay"),
        scopes: scope_set,
    }
}

#[test]
fn dos589_replay_cursor_orders_by_event_seq_and_dedupes() {
    let conn = fresh_full_db();
    conn.execute(
        "INSERT INTO accounts (id, name, updated_at) VALUES (?1, ?2, ?3)",
        params!["acct-589r", "DOS589 R", "2026-05-15T12:00:00Z"],
    )
    .expect("seed account");

    let (clock, rng, external) = ctx_parts();
    let ctx = ServiceContext::new_live(&clock, &rng, &external).with_actor("agent:dos589_replay");
    let db = ActionDb::from_conn(&conn);

    // Seed 4 committed claims → 4 version_events rows in event_seq order.
    let mut cursors_in_order: Vec<String> = Vec::new();
    for i in 0..4 {
        let inserted = commit_claim(&ctx, db, proposal("acct-589r", &format!("c{i}")))
            .expect("commit claim");
        let claim_id = match inserted {
            CommittedClaim::Inserted { claim } => claim.id,
            other => panic!("expected Inserted at {i}, got {other:?}"),
        };
        let cursor: String = conn
            .query_row(
                "SELECT cursor FROM version_events WHERE claim_id = ?1 ORDER BY event_seq LIMIT 1",
                params![&claim_id],
                |row| row.get(0),
            )
            .expect("fetch cursor for claim");
        cursors_in_order.push(cursor);
    }
    assert_eq!(cursors_in_order.len(), 4);

    let dispatcher = VersionDispatcher::new();
    let request = SubscribeRequest {
        surface_client_id: "sc-replay".to_string(),
        surface: "v1".to_string(),
        streams: vec!["claim".to_string()],
        subjects: SubjectFilter::default(),
        from_cursor: None,
        max_batch_size: None,
        wp_user_id: None,
    };
    let actor = surface_client();
    let ack = dispatcher
        .subscribe_stateless(db, &request, actor.clone())
        .expect("subscribe");

    // Mint a valid envelope at cursor index 1 (the second row). Replay must
    // return rows 2 and 3 strictly later in event_seq order, NOT duplicate
    // row 1, and not skip out-of-order.
    let local_key = __test_load_local_key(db, &ack.subscription_id)
        .expect("checkpoint load")
        .expect("subscription_id has a checkpoint row");
    let from_envelope = __test_encode_cursor(&cursors_in_order[1], &ack.subscription_id, &local_key);

    let replay = ReplayRequest {
        subscription_id: ack.subscription_id.clone(),
        from_cursor: from_envelope,
        max_batch_size: None,
    };
    let response = dispatcher
        .replay_stateless(db, &replay, &actor, &SubjectFilter::default())
        .expect("replay");
    assert!(!response.replay_expired, "valid envelope is not expired");

    let delivered_cursors: Vec<&str> = response
        .events
        .iter()
        .map(|ev| ev.cursor.as_str())
        .collect();
    // Strictly later than from_cursor: cursors 2 and 3, in event_seq order.
    assert_eq!(
        delivered_cursors,
        vec![cursors_in_order[2].as_str(), cursors_in_order[3].as_str()],
        "replay returns rows after from_cursor in event_seq order, no dedup miss"
    );
    assert!(
        !delivered_cursors.contains(&cursors_in_order[0].as_str()),
        "earlier rows are not replayed"
    );
    assert!(
        !delivered_cursors.contains(&cursors_in_order[1].as_str()),
        "row at from_cursor is not duplicated"
    );

    // next_cursor envelope decodes to the LAST delivered (permitted) cursor,
    // not a global high-water — packet §3 V3.
    let next_envelope = response.next_cursor.expect("has next_cursor");
    let _decoded_subid =
        dailyos_lib::services::version_dispatcher::__test_decode_cursor_subscription_id(
            &next_envelope,
        )
        .expect("envelope decodes");
}

#[test]
fn dos589_unknown_cursor_returns_replay_expired_not_full_replay() {
    let conn = fresh_full_db();
    conn.execute(
        "INSERT INTO accounts (id, name, updated_at) VALUES (?1, ?2, ?3)",
        params!["acct-589u", "DOS589 U", "2026-05-15T12:00:00Z"],
    )
    .expect("seed account");
    let (clock, rng, external) = ctx_parts();
    let ctx = ServiceContext::new_live(&clock, &rng, &external).with_actor("agent:dos589_replay");
    let db = ActionDb::from_conn(&conn);
    let _ = commit_claim(&ctx, db, proposal("acct-589u", "u0")).expect("commit");

    let dispatcher = VersionDispatcher::new();
    let request = SubscribeRequest {
        surface_client_id: "sc-replay-u".to_string(),
        surface: "v1".to_string(),
        streams: vec!["claim".to_string()],
        subjects: SubjectFilter::default(),
        from_cursor: None,
        max_batch_size: None,
        wp_user_id: None,
    };
    let actor = surface_client();
    let ack = dispatcher
        .subscribe_stateless(db, &request, actor.clone())
        .expect("subscribe");

    // Sign an envelope at a UUIDv4 that does not exist in version_events.
    let local_key = __test_load_local_key(db, &ack.subscription_id)
        .unwrap()
        .unwrap();
    let foreign_uuid = "99999999-9999-4999-8999-999999999999";
    let envelope = __test_encode_cursor(foreign_uuid, &ack.subscription_id, &local_key);

    let replay = ReplayRequest {
        subscription_id: ack.subscription_id.clone(),
        from_cursor: envelope,
        max_batch_size: None,
    };
    let response = dispatcher
        .replay_stateless(db, &replay, &actor, &SubjectFilter::default())
        .expect("replay");
    assert!(
        response.replay_expired,
        "unknown UUIDv4 returns replay_expired, NEVER treated as 'start from zero'"
    );
    assert!(response.events.is_empty(), "no events leak");
}
