use crate::support::bundle_state;

#[test]
fn invocation_claim_and_signal_events_populate_activity_log() {
    let state = bundle_state(18);
    assert_eq!(
        state["version_events"][0]["event_kind"],
        "claim.write_rejected"
    );
    assert_eq!(state["signal_coalescing"]["status"], "completed");
    assert_eq!(
        state["generated_output_rejections"][0]["rejection_reason"],
        "stale_generation_rejected"
    );
}
