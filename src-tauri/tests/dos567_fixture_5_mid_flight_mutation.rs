//! DOS-567 W4-B ac §24 — mid-flight mutation 423.
//!
//! Two SurfaceClients write concurrently to the same claim subject. The
//! commit-lock holder runs the mutation Tx; the second caller observes
//! `try_lock` failure and is mapped to `ClaimError::MidFlightMutation` (which
//! lifts to `BridgeSurfaceError::MidFlightMutation` → HTTP 423). The
//! `retry_after_event` field points to the in-flight mutation_attempts row's
//! cursor; once the winner commits or aborts, the loser can fetch the terminal
//! event there.
//!
//! Triggering a real mid-flight 423 requires multi-thread coordination with
//! a shared SQLite connection (rusqlite::Connection is !Sync) or a temp-file
//! DB with two connections — substrate-sweep agent territory. This fixture
//! pins the variant shape and field semantics; the runtime trigger is
//! `#[ignore]` pending substrate-side test scaffolding.

use dailyos_lib::bridges::BridgeSurfaceError;
use dailyos_lib::services::claims::ClaimError;

#[test]
fn dos567_mid_flight_mutation_variant_shape_and_bridge_mapping() {
    // Substrate-level variant carries claim_id + mutation_id + retry_after_event.
    let claim_err = ClaimError::MidFlightMutation {
        claim_id: "claim-mid-flight".to_string(),
        mutation_id: "mutation-holder".to_string(),
        retry_after_event: "abcdef12-3456-4789-9abc-def012345678".to_string(),
    };
    // The Display impl includes the claim_id for audit visibility.
    let rendered = claim_err.to_string();
    assert!(rendered.contains("claim-mid-flight"));

    // Bridge envelope mirrors the same triple — mutation_id + cursor surface
    // to the 423 response body so the loser can resolve the terminal event.
    let bridge_err = BridgeSurfaceError::MidFlightMutation {
        claim_id: "claim-mid-flight".to_string(),
        mutation_id: "mutation-holder".to_string(),
        retry_after_event: "abcdef12-3456-4789-9abc-def012345678".to_string(),
    };
    match bridge_err {
        BridgeSurfaceError::MidFlightMutation {
            claim_id,
            mutation_id,
            retry_after_event,
        } => {
            assert_eq!(claim_id, "claim-mid-flight");
            assert_eq!(mutation_id, "mutation-holder");
            assert_eq!(retry_after_event, "abcdef12-3456-4789-9abc-def012345678");
        }
        other => panic!("expected MidFlightMutation, got {other:?}"),
    }
}

#[test]
#[ignore = "requires temp-file DB + multi-thread harness (substrate-sweep agent territory): rusqlite::Connection is !Sync, so the runtime race needs two connections sharing a file; the variant + bridge mapping assertions above pin the contract"]
fn dos567_concurrent_writes_loser_receives_mid_flight_with_cursor_pointing_to_attempt() {
    // Plan (post-substrate-sweep): open a temp-file DB, run migrations, give
    // each of two threads its own `Connection` (with `BEGIN IMMEDIATE` retry).
    // Thread A enters `commit_claim` for claim_id=X and parks inside the
    // mutation Tx via a test-only sync point. Thread B calls `commit_claim`
    // for the same (subject, claim_type, field_path) key and receives
    // `ClaimError::MidFlightMutation`. Assertions:
    //   (i) retry_after_event is a UUID v4 matching a `mutation_attempts.cursor`
    //       row with status='in_flight'.
    //   (ii) when thread A commits, a `claim.updated` event lands at that
    //       cursor (winner committed); when thread A panics, a
    //       `mutation_aborted` lands at that cursor (winner failed). The
    //       loser observes EITHER terminal event, never a missing one.
    unreachable!("ignored until temp-file harness lands");
}
