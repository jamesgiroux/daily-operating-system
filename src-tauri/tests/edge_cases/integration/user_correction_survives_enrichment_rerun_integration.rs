use crate::support::bundle_output;

#[test]
fn user_correction_survives_enrichment_rerun() {
    let output = bundle_output(18);
    let scenario = &output["scenarios"]["user-correction-vs-concurrent-enrichment"];

    assert_eq!(scenario["winner"], "user_authored");
    assert_eq!(scenario["current_generation"], 4);
    assert_eq!(scenario["attempted_generation"], 3);
    assert_eq!(scenario["enrichment_status"], "rejected");
}
