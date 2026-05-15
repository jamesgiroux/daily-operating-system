use crate::support::bundle_state;

#[test]
fn stale_claim_uses_source_age_not_fresh_index_age() {
    let state = bundle_state(17);
    let stale = state["intelligence_claims"]
        .as_array()
        .expect("intelligence_claims")
        .iter()
        .find(|claim| claim["claim_id"] == "claim-b17-stale-downstream")
        .expect("stale downstream claim");

    assert_eq!(stale["source_asof"], "2025-10-01T09:00:00Z");
    assert_eq!(stale["observed_at"], "2026-05-15T11:45:00Z");
    assert_ne!(stale["trust_band"], "likely_current");
    assert_eq!(stale["metadata"]["render_policy"], "stale_qualified");
}
