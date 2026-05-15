use crate::support::{bundle_output, json_contains_token};

#[test]
fn revoked_source_is_masked_in_customer_facing_output() {
    let output = bundle_output(17);
    let customer_facing = &output["customer_facing_output"];

    assert!(
        !json_contains_token(customer_facing, "object-b17-restricted-downstream"),
        "revoked or restricted object id leaked to customer-facing output"
    );
    assert_eq!(customer_facing["contains_revoked_source_detail"], false);
    assert_eq!(customer_facing["contains_internal_only_note"], false);
    assert!(
        customer_facing["blocked_source_claim_ids"]
            .as_array()
            .expect("blocked source ids")
            .iter()
            .any(|id| id.as_str() == Some("claim-b17-slack-revoked")),
        "customer-facing output should retain a safe blocked-source summary"
    );
}
