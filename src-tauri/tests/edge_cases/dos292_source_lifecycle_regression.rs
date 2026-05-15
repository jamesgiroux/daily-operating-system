use crate::support::{bundle_output, json_contains_token};

#[test]
fn revoked_glean_content_does_not_leak_through_any_channel() {
    let output = bundle_output(17);
    let matrix = output["render_policy_channel_matrix"]
        .as_array()
        .expect("bundle-17 channel matrix");

    assert_eq!(matrix.len(), 9, "ADR-0108 channel matrix must keep 9 channels");
    for rendered in matrix {
        let channel = rendered["channel"].as_str().expect("channel name");
        assert_eq!(rendered["revoked_source_rejected"], true, "{channel}");
        assert_eq!(rendered["restricted_source_rejected"], true, "{channel}");
        assert_eq!(rendered["internal_only_rejected"], true, "{channel}");
        assert!(
            !json_contains_token(rendered, "raw-attribution-b17-restricted"),
            "{channel} leaked raw revoked/restricted attribution"
        );
        assert!(
            !json_contains_token(rendered, "object-b17-restricted-downstream"),
            "{channel} leaked restricted downstream object id"
        );
        assert!(
            !json_contains_token(rendered, "prompt-hash-b17-secret"),
            "{channel} leaked restricted prompt hash"
        );
    }
}
