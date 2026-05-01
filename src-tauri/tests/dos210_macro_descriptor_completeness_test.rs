use chrono::TimeZone;
use dailyos_abilities_macro::ability;
use dailyos_lib::abilities::provenance::{
    FieldAttribution, FieldPath, ProvenanceBuilder, ProvenanceBuilderConfig, SubjectAttribution,
    SubjectRef,
};
use dailyos_lib::abilities::{
    AbilityCategory, AbilityContext, AbilityRegistry, AbilityResult, Actor,
};
use dailyos_lib::services::context::ExecutionMode;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

mod abilities {
    pub mod provenance {
        pub use dailyos_lib::abilities::provenance::*;
    }

    pub mod registry {
        pub use dailyos_lib::abilities::registry::*;
    }
}

mod services {
    pub mod accounts {
        pub fn update_account_field() {}
    }

    pub mod context {
        pub use dailyos_lib::services::context::*;
    }
}

mod observability {
    pub use dailyos_lib::observability::*;
}

#[derive(Deserialize, JsonSchema)]
struct DescriptorInput;

#[derive(Debug, Serialize, JsonSchema)]
struct DescriptorOutput {
    ok: bool,
}

fn descriptor_output(ability_name: &'static str) -> AbilityResult<DescriptorOutput> {
    let produced_at = chrono::Utc
        .with_ymd_and_hms(2026, 5, 1, 12, 0, 0)
        .unwrap();
    let subject = SubjectAttribution::direct_confident(SubjectRef::Account("acct-fixture".into()));
    let mut builder = ProvenanceBuilder::new(ProvenanceBuilderConfig::new(
        ability_name,
        produced_at,
    ));
    builder.set_subject(subject.clone());
    builder
        .attribute(
            FieldPath::new("/ok").unwrap(),
            FieldAttribution::constant(subject),
        )
        .unwrap();
    builder
        .finalize(DescriptorOutput { ok: true })
        .map_err(|error| dailyos_lib::abilities::AbilityError {
            kind: dailyos_lib::abilities::AbilityErrorKind::Validation,
            message: error.to_string(),
        })
}

#[ability(
    name = "dos210_descriptor_child",
    category = Read,
    version = "0.1.0",
    schema_version = 1,
    allowed_actors = [System],
    allowed_modes = [Evaluate],
    requires_confirmation = false,
    may_publish = false,
    composes = [],
    experimental = false,
    signal_policy = { emits_on_output_change = [], coalesce = false }
)]
async fn dos210_descriptor_child(
    _ctx: &AbilityContext<'_>,
    _input: DescriptorInput,
) -> AbilityResult<DescriptorOutput> {
    descriptor_output("dos210_descriptor_child")
}

#[ability(
    name = "dos210_descriptor_parent",
    category = Publish,
    version = "0.1.0",
    schema_version = 1,
    allowed_actors = [User, System],
    allowed_modes = [Live, Evaluate],
    requires_confirmation = true,
    may_publish = true,
    composes = [{ id = "child-read", ability = "dos210_descriptor_child", optional = false }],
    experimental = false,
    signal_policy = { emits_on_output_change = ["account_changed"], coalesce = true }
)]
async fn dos210_descriptor_parent(
    _ctx: &AbilityContext<'_>,
    _input: DescriptorInput,
) -> AbilityResult<DescriptorOutput> {
    services::accounts::update_account_field();
    descriptor_output("dos210_descriptor_parent")
}

#[test]
fn macro_emitted_descriptor_carries_full_policy_into_inventory() {
    let registry = AbilityRegistry::from_inventory_checked().unwrap();
    let descriptor = registry
        .iter_for(Actor::System)
        .find(|descriptor| descriptor.name == "dos210_descriptor_parent")
        .unwrap();

    assert_eq!(descriptor.category, AbilityCategory::Publish);
    assert_eq!(descriptor.policy.allowed_actors, &[Actor::User, Actor::System]);
    assert_eq!(
        descriptor.policy.allowed_modes,
        &[ExecutionMode::Live, ExecutionMode::Evaluate]
    );
    assert!(descriptor.policy.requires_confirmation);
    assert!(descriptor.policy.may_publish);
    assert_eq!(descriptor.composes.len(), 1);
    assert_eq!(descriptor.composes[0].id.as_str(), "child-read");
    assert_eq!(descriptor.composes[0].ability, "dos210_descriptor_child");
    assert_eq!(descriptor.mutates, &["services::accounts::update_account_field"]);
    assert_eq!(
        descriptor.signal_policy.emits_on_output_change,
        &["account_changed"]
    );
    assert!(descriptor.signal_policy.coalesce);
}
