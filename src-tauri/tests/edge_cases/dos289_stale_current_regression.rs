use crate::support::{bundle_output, json_contains_token};

#[test]
fn stale_escalation_is_not_rendered_as_current_meeting_topic() {
    let output = bundle_output(14);
    let prepare = &output["surfaces"]["prepare_meeting"];
    let rendered_topics = &prepare["topics"];

    assert!(
        !json_contains_token(rendered_topics, "claim-b14-superseded-open-risk"),
        "bundle-14 stale superseded escalation leaked into rendered current topics"
    );
    assert_eq!(
        output["current_rendering"]["stale_current_advice_absent"], true,
        "bundle-14 current rendering oracle must reject stale-current advice"
    );
    assert_eq!(
        prepare["rejected_provider_candidates"][0]["source_claim_id"],
        "claim-b14-superseded-open-risk",
        "provider's stale-current candidate must be rejected, not rephrased as open"
    );
}
