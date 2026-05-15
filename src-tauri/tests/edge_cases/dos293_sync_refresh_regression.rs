use crate::support::{bundle_state, claim_text};

#[test]
fn user_correction_row_persists_after_concurrent_enrichment() {
    let state = bundle_state(18);
    let preserved = claim_text(&state, "claim-b18-user-correction-current");

    assert_eq!(
        preserved,
        "User corrected the implementation summary to say the account wants a written agenda before renewal planning."
    );
    let claim = state["intelligence_claims"]
        .as_array()
        .expect("intelligence_claims")
        .iter()
        .find(|claim| claim["claim_id"] == "claim-b18-user-correction-current")
        .expect("user correction claim");
    assert_eq!(claim["actor"], "user");
    assert_eq!(claim["generation"], 4);
    assert_eq!(claim["metadata"]["user_authored"], true);
}
