mod dos568_support;

use dos568_support::{
    canonical_json_bytes, clone_connection, fresh_full_db, invoke_account_overview_json,
    seed_base_account_state, shared, surface_actor_with_account_scope,
};

#[tokio::test]
async fn dos568_committed_output_is_deterministic_across_cloned_db_state() {
    let base = fresh_full_db();
    seed_base_account_state(&base);

    let first_db = shared(clone_connection(&base));
    let second_db = shared(clone_connection(&base));

    let first = invoke_account_overview_json(first_db, surface_actor_with_account_scope(), 0).await;
    let second =
        invoke_account_overview_json(second_db, surface_actor_with_account_scope(), 0).await;

    assert_eq!(canonical_json_bytes(&first), canonical_json_bytes(&second));
}
