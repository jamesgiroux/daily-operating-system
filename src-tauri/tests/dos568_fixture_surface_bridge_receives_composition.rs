mod dos568_support;

use dailyos_lib::abilities::registry::AbilityRegistry;
use dailyos_lib::bridges::surface_client::{SurfaceClientBridge, SurfaceClientBridgeConfig};
use dos568_support::{
    composition_version, fresh_full_db, invoke_account_overview_json, seed_base_account_state,
    shared, surface_session_with_account_scope, ACCOUNT_OVERVIEW_ABILITY,
};

#[tokio::test]
async fn dos568_surface_invoke_bridge_reaches_global_registry_and_receives_composition() {
    let conn = fresh_full_db();
    seed_base_account_state(&conn);
    let db = shared(conn);
    let registry = AbilityRegistry::global_checked().expect("global registry builds");
    let descriptor = registry
        .iter_all()
        .find(|descriptor| descriptor.name == ACCOUNT_OVERVIEW_ABILITY)
        .expect("account overview descriptor registered");
    assert!(!descriptor.policy.client_side_executable);

    let bridge = SurfaceClientBridge::new(SurfaceClientBridgeConfig::default());
    let session = surface_session_with_account_scope();
    let authorization = bridge
        .authorize(
            registry,
            &session,
            ACCOUNT_OVERVIEW_ABILITY,
            "req_dos568_surface_invoke",
        )
        .expect("signed /v1/surface/invoke server path authorizes client-side-disabled ability");
    assert_eq!(
        authorization.canonical_ability_name,
        ACCOUNT_OVERVIEW_ABILITY
    );

    let output = invoke_account_overview_json(db, session.actor.clone(), 0).await;

    assert_eq!(composition_version(&output), 1);
    assert_eq!(
        output
            .pointer("/data/generated_by")
            .and_then(|value| value.as_str()),
        Some(ACCOUNT_OVERVIEW_ABILITY)
    );
    assert_eq!(
        output
            .pointer("/provenance/actor/source")
            .and_then(|value| value.as_str()),
        Some("surface_client:sc_dos568_fixture")
    );
    assert!(output.pointer("/data/sections/0/blocks/0").is_some());
}
