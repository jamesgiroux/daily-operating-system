use crate::support::bundle_output;

#[test]
fn seeded_corpus_lint_failure_blocks_confident_render() {
    let output = bundle_output(15);
    let blocked = &output["surfaces"]["get_entity_context"]["blocked_claims"][0];

    assert_eq!(blocked["claim_id"], "claim-b15-lint-blocked-bleed");
    assert_eq!(blocked["render_policy"], "blocked");
    assert_eq!(blocked["rendered_confidently"], false);
    assert_eq!(output["validation_lint"]["release_gate_failure"], true);
}
