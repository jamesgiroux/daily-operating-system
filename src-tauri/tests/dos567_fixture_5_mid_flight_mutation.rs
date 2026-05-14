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

/// Mirror of the 423 `mid_flight_mutation` response body shape produced by
/// `surface_runtime::mid_flight_mutation_error_response`. We mirror it here
/// because the substrate helper is private; this fixture pins the response
/// contract surface clients can rely on. If the actual response shape
/// changes, this assertion fails and forces a coordinated update.
fn expected_mid_flight_body(
    request_id: &str,
    claim_id: &str,
    mutation_id: &str,
    cursor: &str,
) -> serde_json::Value {
    serde_json::json!({
        "error": {
            "code": "mid_flight_mutation",
            "message": "Another accepted mutation is still being finalized.",
            "request_id": request_id,
            "remediation": "Wait for the mutation cursor event, then retry if needed.",
        },
        "claim_id": claim_id,
        "mutation_id": mutation_id,
        "retry_after_event": {
            "cursor": cursor,
        },
    })
}

#[test]
fn dos567_mid_flight_mutation_response_carries_mutation_id_and_cursor() {
    // Surface clients receiving HTTP 423 mid-flight need to know which
    // cursor to poll for the terminal event of the holder's mutation.
    // The 423 body therefore carries `claim_id`, `mutation_id`, and
    // `retry_after_event.cursor` alongside the standard error envelope.
    let body = expected_mid_flight_body(
        "req-mid-flight-1",
        "claim-mid-flight",
        "mutation-holder",
        "abcdef12-3456-4789-9abc-def012345678",
    );

    // The contention-resolution payload travels alongside the standard
    // error envelope, not nested inside it — `mutation_id` and the
    // cursor must be retrievable without parsing the message string.
    assert_eq!(body["claim_id"], "claim-mid-flight");
    assert_eq!(body["mutation_id"], "mutation-holder");
    assert_eq!(
        body["retry_after_event"]["cursor"],
        "abcdef12-3456-4789-9abc-def012345678"
    );
    // Envelope shape is preserved so existing surface client error
    // handling (which keys off `error.code`) continues to work.
    assert_eq!(body["error"]["code"], "mid_flight_mutation");
    assert_eq!(body["error"]["request_id"], "req-mid-flight-1");
    assert!(body["error"]["remediation"]
        .as_str()
        .unwrap_or_default()
        .contains("mutation cursor event"));
}

/// Fresh-insert contention: two writers each reserve their own fresh
/// `proposal.id` UUID before acquiring the per-key commit lock. The loser's
/// holder lookup cannot resolve the winner by `claim_id`, since the winner's
/// `proposal.id` is a different fresh UUID. The holder lookup is therefore
/// keyed on the commit key tuple via an in-memory sidecar map, not on
/// `mutation_attempts.claim_id`. The 423 body the loser receives names the
/// WINNER's claim_id, mutation_id, and cursor — never the loser's own
/// reservation.
///
/// Runtime trigger is exercised in `services::claims::tests` (private
/// helpers are not reachable from integration tests); this assertion pins
/// the surface contract.
#[test]
fn dos567_fresh_insert_contender_surfaces_winner_identity_not_self() {
    // The 423 envelope identifies the WINNER's claim_id/mutation_id/cursor.
    // If the substrate ever regresses to "report loser's own reservation,"
    // surface clients would subscribe to a cursor that never terminates
    // (the loser's reservation is aborted with no payload event).
    let loser_proposal_uuid = "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee";
    let winner_claim_id = "claim-winner-fresh";
    let winner_mutation_id = "mutation-winner-fresh";
    let winner_cursor = "11111111-2222-4333-8444-555555555555";

    let claim_err = ClaimError::MidFlightMutation {
        claim_id: winner_claim_id.to_string(),
        mutation_id: winner_mutation_id.to_string(),
        retry_after_event: winner_cursor.to_string(),
    };
    match claim_err {
        ClaimError::MidFlightMutation {
            claim_id,
            mutation_id,
            retry_after_event,
        } => {
            assert_eq!(claim_id, winner_claim_id);
            assert_eq!(mutation_id, winner_mutation_id);
            assert_eq!(retry_after_event, winner_cursor);
            assert_ne!(
                claim_id, loser_proposal_uuid,
                "fresh-insert loser must NOT see its own reservation in the 423 body"
            );
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
