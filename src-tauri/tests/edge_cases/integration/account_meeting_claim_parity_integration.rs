use crate::support::{bundle_output, matrix_row};

#[test]
fn account_context_and_meeting_briefing_share_claim_refs() {
    let output = bundle_output(15);
    let account_claim = output["surfaces"]["get_entity_context"]["entries"][0]["claim_id"]
        .as_str()
        .expect("account claim id");
    let meeting_claims = output["surfaces"]["prepare_meeting"]["topics"][0]["source_claim_ids"]
        .as_array()
        .expect("meeting source claims");

    assert!(meeting_claims.iter().any(|claim| claim.as_str() == Some(account_claim)));
    assert_eq!(
        matrix_row(&output, "primary_account_id")["equal"],
        true
    );
}
