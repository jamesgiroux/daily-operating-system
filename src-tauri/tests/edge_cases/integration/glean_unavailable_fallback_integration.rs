use crate::support::bundle_state;

#[test]
fn glean_unavailable_fallback_degrades_trust_without_leaking_raw_content() {
    let state = bundle_state(17);
    let unavailable = state["intelligence_claims"]
        .as_array()
        .expect("intelligence_claims")
        .iter()
        .find(|claim| claim["claim_id"] == "claim-b17-google-disconnected")
        .expect("unavailable source claim");

    assert_eq!(unavailable["metadata"]["source_lifecycle_state"], "unavailable");
    assert_eq!(unavailable["metadata"]["render_policy"], "degraded_safe_summary");
    assert_ne!(unavailable["trust_band"], "likely_current");
}
