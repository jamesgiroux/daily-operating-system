//! W4-B-signals AC #6 — `subscription.backpressure` event when the outbound queue
//! exceeds its threshold; W4-B mutation commit path is NOT blocked.
//!
//! Setup: dispatcher constructed with a tiny capacity (hard_cap = 2) so the
//! test does not need to seed 1024+ events. Subscribe a SurfaceClient with
//! a push handle; do NOT drain the receiver. Seed three committed claims
//! (W4-B emits one `version_events` row per claim). Run `dispatch_pending`
//! and assert:
//!  - the subscriber's checkpoint is marked replay_required,
//!  - a `subscription.backpressure` event is delivered to the affected
//!    subscriber's backpressure channel (not a broadcast),
//!  - the mutation commit path completes for all three claims regardless.

use chrono::{TimeZone, Utc};
use dailyos_lib::abilities::registry::{ScopeSet, SurfaceClientId, SurfaceScope};
use dailyos_lib::abilities::Actor;
use dailyos_lib::db::claims::{ClaimSensitivity, TemporalScope};
use dailyos_lib::db::ActionDb;
use dailyos_lib::migration_test_api::run_migrations;
use dailyos_lib::services::claims::{commit_claim, ClaimProposal};
use dailyos_lib::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};
use dailyos_lib::services::version_dispatcher::{
    BackpressureEvent, SubjectFilter, SubscribeRequest, VersionDispatcher,
};
use rusqlite::{params, Connection};

fn fresh_full_db() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory db");
    run_migrations(&conn).expect("apply production migrations");
    conn
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
        text: format!("dos589 bp {label}"),
        actor: "agent:dos589_bp".to_string(),
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
    let scope_set =
        ScopeSet::new([SurfaceScope::new("read.account_overview")]).expect("scope non-empty");
    Actor::SurfaceClient {
        instance: SurfaceClientId::new("sc-bp"),
        scopes: scope_set,
    }
}

#[test]
fn dos589_backpressure_marks_replay_required_without_blocking_mutation() {
    let conn = fresh_full_db();
    conn.execute(
        "INSERT INTO accounts (id, name, updated_at) VALUES (?1, ?2, ?3)",
        params!["acct-589-bp", "DOS589 BP", "2026-05-15T12:00:00Z"],
    )
    .expect("seed account");
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 15, 12, 0, 0).unwrap());
    let rng = SeedableRng::new(31_589);
    let external = ExternalClients::default();
    let ctx = ServiceContext::new_live(&clock, &rng, &external).with_actor("agent:dos589_bp");
    let db = ActionDb::from_conn(&conn);

    // Tiny dispatcher: soft=1, hard=2. The 3rd claim's row should overflow
    // and trip backpressure.
    let dispatcher = VersionDispatcher::with_capacity(1, 2);
    let actor = surface_client();
    let request = SubscribeRequest {
        surface_client_id: "sc-bp".to_string(),
        surface: "v1".to_string(),
        streams: vec!["claim".to_string()],
        subjects: SubjectFilter::default(),
        from_cursor: None,
        max_batch_size: None,
        wp_user_id: None,
    };
    let (ack, _rx, mut bp_rx) = dispatcher
        .subscribe(db, &request, actor.clone())
        .expect("subscribe with push handle");
    // NB: _rx is held but never drained — that's the slow-subscriber model.

    // Seed three claims. Each commit_claim writes a version_events row,
    // and crucially each must succeed irrespective of dispatcher state.
    for i in 0..3 {
        commit_claim(&ctx, db, proposal("acct-589-bp", &format!("c{i}")))
            .unwrap_or_else(|_| panic!("commit claim {i} (mutation path independent of dispatcher)"));
    }

    // Confirm W4-B did its work despite the slow subscriber.
    let event_rows: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM version_events WHERE claim_id IN ( \
                SELECT id FROM intelligence_claims WHERE subject_ref LIKE '%acct-589-bp%' \
             )",
            [],
            |row| row.get(0),
        )
        .expect("count events");
    assert!(event_rows >= 3, "W4-B commit path is not blocked by backpressure (got {event_rows})");

    // Drive a dispatch cycle. The 3rd push should overflow → mark replay-required.
    dispatcher
        .dispatch_pending(db)
        .expect("dispatch loop runs to completion");

    // Subscriber's backpressure channel surfaces the event.
    let mut saw_replay_required_event = false;
    while let Ok(event) = bp_rx.try_recv() {
        if event.event_kind == BackpressureEvent::KIND
            && event.subscription_id == ack.subscription_id
            && event.replay_required
        {
            saw_replay_required_event = true;
        }
    }
    assert!(
        saw_replay_required_event,
        "subscription.backpressure with replay_required=true is delivered to the affected subscriber"
    );

    // Durable checkpoint reflects the replay-required state.
    let replay_required_flag: i64 = conn
        .query_row(
            "SELECT replay_required FROM subscription_checkpoints WHERE subscription_id = ?1",
            params![&ack.subscription_id],
            |row| row.get(0),
        )
        .expect("read replay_required");
    assert_eq!(
        replay_required_flag, 1,
        "checkpoint persists replay_required across reconnect"
    );
}
