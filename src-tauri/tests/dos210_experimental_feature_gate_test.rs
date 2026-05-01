#![allow(dead_code, unused_imports)]

use chrono::TimeZone;
use dailyos_abilities_macro::ability;
use dailyos_lib::abilities::provenance::{
    FieldAttribution, FieldPath, ProvenanceBuilder, ProvenanceBuilderConfig, SubjectAttribution,
    SubjectRef,
};
use dailyos_lib::abilities::{AbilityContext, AbilityRegistry, AbilityResult};
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
    pub mod context {
        pub use dailyos_lib::services::context::*;
    }
}

mod observability {
    pub use dailyos_lib::observability::*;
}

#[derive(Deserialize, JsonSchema)]
struct ExperimentalInput;

#[derive(Debug, Serialize, JsonSchema)]
struct ExperimentalOutput {
    ok: bool,
}

#[ability(
    name = "dos210_experimental_feature_gate_fixture",
    category = Read,
    version = "0.1.0",
    schema_version = 1,
    allowed_actors = [System],
    allowed_modes = [Evaluate],
    requires_confirmation = false,
    may_publish = false,
    composes = [],
    experimental = true,
    registered_at = "2999-01-01T00:00:00Z",
    signal_policy = { emits_on_output_change = [], coalesce = false }
)]
async fn dos210_experimental_feature_gate_fixture(
    _ctx: &AbilityContext<'_>,
    _input: ExperimentalInput,
) -> AbilityResult<ExperimentalOutput> {
    let produced_at = chrono::Utc
        .with_ymd_and_hms(2026, 5, 1, 12, 0, 0)
        .unwrap();
    let subject = SubjectAttribution::direct_confident(SubjectRef::Account("acct-fixture".into()));
    let mut builder = ProvenanceBuilder::new(ProvenanceBuilderConfig::new(
        "dos210_experimental_feature_gate_fixture",
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
        .finalize(ExperimentalOutput { ok: true })
        .map_err(|error| dailyos_lib::abilities::AbilityError {
            kind: dailyos_lib::abilities::AbilityErrorKind::Validation,
            message: error.to_string(),
        })
}

#[cfg(feature = "experimental")]
#[test]
fn experimental_ability_is_in_inventory_when_feature_on() {
    let registry = AbilityRegistry::from_inventory_checked().unwrap();

    assert!(registry
        .iter_for(dailyos_lib::abilities::Actor::System)
        .any(|descriptor| descriptor.name == "dos210_experimental_feature_gate_fixture"));
}

#[cfg(not(feature = "experimental"))]
#[test]
fn experimental_ability_not_in_inventory_when_feature_off() {
    let registry = AbilityRegistry::from_inventory_checked().unwrap();

    assert!(!registry
        .iter_for(dailyos_lib::abilities::Actor::System)
        .any(|descriptor| descriptor.name == "dos210_experimental_feature_gate_fixture"));
}
