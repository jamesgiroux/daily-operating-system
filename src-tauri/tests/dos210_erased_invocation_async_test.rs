use chrono::TimeZone;
use dailyos_abilities_macro::ability;
use dailyos_lib::abilities::provenance::{
    FieldAttribution, FieldPath, ProvenanceBuilder, ProvenanceBuilderConfig, SubjectAttribution,
    SubjectRef,
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
struct AsyncInput {
    value: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct AsyncOutput {
    echoed: String,
}

#[ability(
    name = "dos210_async_erased_fixture",
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
async fn dos210_async_erased_fixture(
    ctx: &AbilityContext<'_>,
    input: AsyncInput,
) -> AbilityResult<AsyncOutput> {
    tokio::task::yield_now().await;

    let subject = SubjectAttribution::direct_confident(SubjectRef::Account("acct-fixture".into()));
    let mut builder = ProvenanceBuilder::new(ProvenanceBuilderConfig::new(
        "dos210_async_erased_fixture",
        ctx.services().clock.now(),
    ));
    builder.set_subject(subject.clone());
    builder
        .attribute(
            FieldPath::new("/echoed").unwrap(),
            FieldAttribution::constant(subject),
        )
        .unwrap();
    builder
        .finalize(AsyncOutput {
            echoed: input.value,
        })
        .map_err(|error| dailyos_lib::abilities::AbilityError {
            kind: dailyos_lib::abilities::AbilityErrorKind::Validation,
            message: error.to_string(),
        })
}

#[tokio::test]
async fn invoke_by_name_json_works_inside_async_runtime() {
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
            "dos210_async_erased_fixture",
            serde_json::json!({ "value": "inside-runtime" }),
        )
        .await
        .unwrap();

    assert_eq!(value["data"]["echoed"], "inside-runtime");
}
