use crate::support::bundle_output;

#[test]
fn calendar_meeting_links_entities_before_briefing_topics_render() {
    let output = bundle_output(15);
    let meeting = &output["surfaces"]["prepare_meeting"];

    assert_eq!(meeting["meeting_id"], "meeting-b15-example");
    assert_eq!(meeting["attendee_context"][0]["entity_id"], "account-b15-example");
    assert_eq!(meeting["attendee_context"][1]["entity_id"], "person-b15-example");
    assert_eq!(meeting["topics"][0]["source_claim_ids"][0], "claim-b15-account-health-current");
    assert_eq!(meeting["topics"][0]["trust_band"], "likely_current");
}
