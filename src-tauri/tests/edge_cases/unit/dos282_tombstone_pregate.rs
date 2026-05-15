use crate::support::{bundle_state, json_contains_token};

#[test]
fn user_tombstone_blocks_ai_reassertion_before_render() {
    let state = bundle_state(14);
    let claims = state["intelligence_claims"]
        .as_array()
        .expect("intelligence_claims");
    let superseded = claims
        .iter()
        .find(|claim| claim["claim_id"] == "claim-b14-superseded-open-risk")
        .expect("superseded stale risk claim");

    assert_eq!(superseded["claim_state"], "dormant");
    assert_eq!(superseded["surfacing_state"], "dormant");
    assert_eq!(superseded["demotion_reason"], "superseded");
    assert!(
        !json_contains_token(&state["rendered_current_topics"], "claim-b14-superseded-open-risk"),
        "tombstoned/superseded stale claim must not be reasserted into current topics"
    );
}
