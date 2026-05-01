use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::TimeZone;
use dailyos_abilities_macro::ability;
use dailyos_lib::abilities::provenance::{
    provenance_for_test, AbilityOutput, Diagnostics, SubjectAttribution, SubjectRef,
};
use dailyos_lib::abilities::{AbilityContext, AbilityResult, Actor};
use dailyos_lib::observability::{EvaluateModeSubscriber, Outcome};
use dailyos_lib::services::context::{
    Clock, ExternalClients, FixedClock, SeedableRng, ServiceContext,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing_test::traced_test;

mod abilities {
    pub use dailyos_lib::abilities::*;

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
struct SpanInput {
    value: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct SpanOutput {
    ok: bool,
}

#[ability(
    name = "dos210_span_fixture",
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
async fn dos210_span_fixture(
    ctx: &AbilityContext<'_>,
    input: SpanInput,
) -> AbilityResult<SpanOutput> {
    let subject = SubjectAttribution::direct_confident(SubjectRef::Account("acct-fixture".into()));
    let provenance = provenance_for_test(
        "dos210_span_fixture",
        ctx.services().clock.now(),
        subject,
        Vec::new(),
        Vec::new(),
        BTreeMap::new(),
        None,
        Vec::new(),
    );

    Ok(AbilityOutput {
        data: SpanOutput {
            ok: !input.value.is_empty(),
        },
        ability_version: provenance.ability_version.clone(),
        diagnostics: Diagnostics::default(),
        provenance,
    })
}

#[traced_test]
#[test]
fn span_carries_required_fields_and_redacts_payload() {
    let subscriber = Arc::new(EvaluateModeSubscriber::new());
    let _ = __ABILITY_EVALUATE_SUBSCRIBER_DOS210_SPAN_FIXTURE.set(subscriber.clone());
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let clock = FixedClock::new(
        chrono::Utc
            .with_ymd_and_hms(2026, 5, 1, 12, 0, 0)
            .unwrap(),
    );
    let rng = SeedableRng::new(42);
    let external = ExternalClients::default();
    let services = ServiceContext::new_evaluate(&clock, &rng, &external);
    let ctx = AbilityContext::new(&services, Actor::User, None);

    let output = runtime.block_on(dos210_span_fixture(
        &ctx,
        SpanInput {
            value: "sensitive-payload-marker".to_string(),
        },
    ))
    .unwrap();
    assert!(output.data.ok);

    let records = subscriber.snapshot();
    assert_eq!(records.len(), 1);
    let record = &records[0];

    assert_ne!(record.invocation_id, uuid::Uuid::nil());
    assert_eq!(record.ability_name, "dos210_span_fixture");
    assert_eq!(record.ability_category, "Read");
    assert_eq!(record.actor, "User");
    assert_eq!(record.mode, "evaluate");
    // Span instrumentation uses chrono::Utc::now() per ADR-0120 (registry-emitted
    // wall clock; not the ServiceContext clock seam, which is for ability-runtime
    // logic). Assert the timestamp is plausible: within 60s of test start.
    let test_start = chrono::Utc::now();
    let drift = (record.started_at - test_start).num_seconds().abs();
    assert!(drift < 60, "started_at drifted >60s from wall clock: {drift}s");
    assert!(record.ended_at >= record.started_at);
    assert!(matches!(record.outcome, Outcome::Ok));
    assert!(record.duration_ms <= 60_000);

    let rendered = serde_json::to_string(record).unwrap();
    assert!(!rendered.contains("sensitive-payload-marker"));
    assert!(!logs_contain("sensitive-payload-marker"));
}
