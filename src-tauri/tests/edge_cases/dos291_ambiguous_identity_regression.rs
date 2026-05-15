use crate::support::bundle_output;

#[test]
fn same_domain_twins_remain_ambiguous_not_primary() {
    let output = bundle_output(16);
    let same_domain = &output["subject_selection"]["same-domain-twins"];

    assert_eq!(same_domain["state"], "ambiguous");
    assert!(same_domain["primary_subject_ref"].is_null());
    assert_eq!(same_domain["confident_primary_rendered"], false);
    assert_eq!(
        same_domain["render_policy"], "confirmation_request",
        "same-domain twins must ask for confirmation instead of selecting a primary"
    );
}
