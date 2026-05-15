use crate::support::bundle_output;

#[test]
fn primary_entity_id_is_identical_across_app_and_mcp_surfaces() {
    let output = bundle_output(15);
    let surfaces = &output["surfaces"];
    let expected = "account-b15-example";

    assert_eq!(
        surfaces["get_entity_context"]["subject"]["entity_id"],
        expected
    );
    assert_eq!(
        surfaces["prepare_meeting"]["attendee_context"][0]["entity_id"],
        expected
    );
    assert_eq!(
        surfaces["get_daily_readiness"]["surfaces"]["account"][0]["entity_id"],
        expected
    );
    assert_eq!(surfaces["mcp"]["data"]["primary_account_id"], expected);
    assert_eq!(
        output["primary_entity_oracle"]["disagreement_counterexample"]["status"],
        "release_gate_failure",
        "bundle-15 must keep primary entity disagreement release-gated"
    );
}
