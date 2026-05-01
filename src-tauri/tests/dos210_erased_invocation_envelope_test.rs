use chrono::TimeZone;
use dailyos_abilities_macro::ability;
use dailyos_lib::abilities::provenance::{
    FieldAttribution, FieldPath, ProvenanceBuilder, ProvenanceBuilderConfig, SubjectAttribution,
    SubjectRef, ThreadId,
};
use dailyos_lib::abilities::{AbilityContext, AbilityRegistry, AbilityResult, Actor};
use dailyos_lib::services::context::{
    ExternalClients, FixedClock, SeedableRng, ServiceContext,
};
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
struct EnvelopeInput {
    value: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct EnvelopeOutput {
    ok: bool,
}

#[ability(
    name = "dos210_erased_envelope_fixture",
    category = Read,
    version = "0.1.0",
    schema_version = 1,
    allowed_actors = [User],
    allowed_modes = [Evaluate],
    requires_confirmation = false,
    may_publish = false,
    composes = [],
    experimental = false,
    signal_policy = { emits_on_output_change = [], coalesce = false }
)]
async fn dos210_erased_envelope_fixture(
    ctx: &AbilityContext<'_>,
    input: EnvelopeInput,
) -> AbilityResult<EnvelopeOutput> {
    let subject = SubjectAttribution::direct_confident(SubjectRef::Account("acct-fixture".into()));
    let mut builder = ProvenanceBuilder::new(ProvenanceBuilderConfig::new(
        "dos210_erased_envelope_fixture",
        ctx.services().clock.now(),
    ));
    builder.set_subject(subject.clone());
    builder.add_thread_id(ThreadId::new("thread-fixture"));
    builder
        .attribute(
            FieldPath::new("/ok").unwrap(),
            FieldAttribution::constant(subject),
        )
        .unwrap();
    builder
        .finalize(EnvelopeOutput {
            ok: !input.value.is_empty(),
        })
        .map_err(|error| dailyos_lib::abilities::AbilityError {
            kind: dailyos_lib::abilities::AbilityErrorKind::Validation,
            message: error.to_string(),
        })
}

#[test]
fn invoke_by_name_json_returns_full_ability_output_envelope() {
    let registry = AbilityRegistry::from_inventory_checked().unwrap();
    let clock = FixedClock::new(
        chrono::Utc
            .with_ymd_and_hms(2026, 5, 1, 12, 0, 0)
            .unwrap(),
    );
    let rng = SeedableRng::new(42);
    let external = ExternalClients::default();
    let services = ServiceContext::new_evaluate(&clock, &rng, &external);
    let ctx = AbilityContext::new(&services, Actor::User, None);

    let value = registry
        .invoke_by_name_json(
            &ctx,
            "dos210_erased_envelope_fixture",
            serde_json::json!({ "value": "payload" }),
        )
        .unwrap();

    assert_eq!(value["data"]["ok"], true);
    assert!(value.get("provenance").is_some());
    assert!(value.get("ability_version").is_some());
    assert!(value.get("diagnostics").is_some());
    assert!(value["diagnostics"]["warnings"].is_array());
    assert_eq!(
        value["provenance"]["ability_name"],
        "dos210_erased_envelope_fixture"
    );
    assert!(value["provenance"]["warnings"].is_array());
    assert_eq!(value["provenance"]["thread_ids"][0], "thread-fixture");
}
