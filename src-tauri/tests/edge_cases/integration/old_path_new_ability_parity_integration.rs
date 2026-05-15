use crate::support::{bundle_output, matrix_row};

#[test]
fn old_surface_paths_and_new_ability_outputs_remain_in_field_parity() {
    let output = bundle_output(15);
    for field in [
        "primary_account_id",
        "eligible_meeting_count",
        "account_health_risk",
        "project_status",
        "current_state_claim_id",
        "source_asof",
        "trust_band",
    ] {
        assert_eq!(
            matrix_row(&output, field)["equal"], true,
            "{field} must remain in old-path/new-ability parity until cutover"
        );
    }
}
