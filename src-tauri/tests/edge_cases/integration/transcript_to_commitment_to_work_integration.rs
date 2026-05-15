use crate::support::bundle_output;

#[test]
fn transcript_commitment_flows_into_open_loop_work_surface() {
    let output = bundle_output(15);
    let person_action = output["surfaces"]["get_entity_context_person"]["actions"][0]["claim_id"]
        .as_str()
        .expect("person action claim id");
    let open_loop = &output["surfaces"]["prepare_meeting"]["open_loops"][0];

    assert_eq!(open_loop["meeting_id"], "meeting-b15-example");
    assert_eq!(open_loop["person_id"], "person-b15-example");
    assert!(open_loop["source_claim_ids"]
        .as_array()
        .expect("source claims")
        .iter()
        .any(|claim| claim.as_str() == Some(person_action)));
}
