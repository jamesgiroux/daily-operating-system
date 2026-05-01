use dailyos_lib::abilities::provenance::CompositionId;
use dailyos_lib::abilities::{
    AbilityCategory, AbilityContext, AbilityDescriptor, AbilityError, AbilityRegistry, Actor,
};
use dailyos_lib::abilities::registry::{AbilityPolicy, SignalPolicy};
use dailyos_lib::services::context::ExecutionMode;

fn passthrough_erased(
    _ctx: &AbilityContext<'_>,
    input: serde_json::Value,
) -> Result<serde_json::Value, AbilityError> {
    Ok(input)
}

fn empty_schema() -> serde_json::Value {
    serde_json::json!({ "type": "object" })
}

fn clean_read_descriptor() -> AbilityDescriptor {
    AbilityDescriptor {
        name: "read_helper_transitive_mutation",
        version: "0.1.0",
        schema_version: 1,
        category: AbilityCategory::Read,
        policy: AbilityPolicy {
            allowed_actors: vec![Actor::System],
            allowed_modes: vec![ExecutionMode::Evaluate],
            requires_confirmation: false,
            may_publish: false,
        },
        composes: Vec::new(),
        mutates: Vec::new(),
        experimental: false,
        registered_at: None,
        signal_policy: SignalPolicy::default(),
        invoke_erased: passthrough_erased,
        input_schema: empty_schema,
        output_schema: empty_schema,
    }
}

fn helper_transitively_mutates() -> &'static str {
    "services::accounts::update_account_field"
}

#[test]
#[ignore = "fixture-trace mechanism deferred to follow-up; macro-only AST detection is the W3-A baseline"]
fn read_ability_calling_helper_that_mutates_fails_drift_check() {
    eprintln!(
        "DOS-210 metadata drift needs a fixture-trace pass that invokes \
         `read_helper_transitive_mutation`, observes `{}`, and compares it \
         with the descriptor's empty declared mutates set.",
        helper_transitively_mutates()
    );

    let registry = AbilityRegistry::from_descriptors_checked(vec![clean_read_descriptor()])
        .expect("current registry has no fixture-trace drift binding gate");
    assert_eq!(registry.iter_for(Actor::System).count(), 1);

    let _composition_id_shape = CompositionId::new("helper_transitive_mutation");
}
