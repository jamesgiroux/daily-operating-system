use crate::support::bundle_output;

#[test]
fn project_context_matches_project_claim_used_by_meeting_prep() {
    let output = bundle_output(15);
    let project_claim = output["surfaces"]["get_entity_context_project"]["entries"][0]["claim_id"]
        .as_str()
        .expect("project claim id");
    let topic_claims = output["surfaces"]["prepare_meeting"]["topics"][0]["source_claim_ids"]
        .as_array()
        .expect("topic source claims");

    assert!(topic_claims.iter().any(|claim| claim.as_str() == Some(project_claim)));
    assert_eq!(
        output["project_page_surface_pinned"]["account_page_project_section"]["claim_id"],
        project_claim
    );
}
