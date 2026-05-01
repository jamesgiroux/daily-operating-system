use chrono::TimeZone;
use dailyos_lib::abilities::provenance::{
    AbilityOutput, CompositionId, FieldAttribution, FieldPath, ProvenanceBuilder,
    ProvenanceBuilderConfig, ProvenanceWarning, SubjectAttribution, SubjectRef,
};
use dailyos_lib::abilities::registry::{AbilityPolicy, ComposesEntry, SignalPolicy};
use dailyos_lib::abilities::{
    AbilityCategory, AbilityContext, AbilityDescriptor, AbilityError, AbilityErrorKind,
    AbilityRegistry, AbilityResult, Actor,
};
use dailyos_lib::services::context::ExecutionMode;

#[derive(Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
struct FixtureOutput {
    ok: bool,
}

fn passthrough_erased<'a>(
    _ctx: &'a AbilityContext<'a>,
    input: serde_json::Value,
) -> std::pin::Pin<
    Box<dyn std::future::Future<Output = Result<serde_json::Value, AbilityError>> + Send + 'a>,
> {
    Box::pin(async move { Ok(input) })
}

fn empty_schema() -> serde_json::Value {
    serde_json::json!({ "type": "object" })
}

fn static_slice<T>(values: Vec<T>) -> &'static [T] {
    Box::leak(values.into_boxed_slice())
}

fn descriptor(name: &'static str, composes: &'static [ComposesEntry]) -> AbilityDescriptor {
    AbilityDescriptor {
        name,
        version: "0.1.0",
        schema_version: 1,
        category: AbilityCategory::Read,
        policy: AbilityPolicy {
            allowed_actors: &[Actor::System],
            allowed_modes: &[ExecutionMode::Evaluate],
            requires_confirmation: false,
            may_publish: false,
        },
        composes,
        mutates: &[],
        experimental: false,
        registered_at: None,
        signal_policy: SignalPolicy::default(),
        invoke_erased: passthrough_erased,
        input_schema: empty_schema,
        output_schema: empty_schema,
    }
}

fn composed_entry(optional: bool) -> ComposesEntry {
    ComposesEntry {
        id: CompositionId::new("b-read"),
        ability: "child_b",
        optional,
    }
}

fn register_pair(optional: bool) -> AbilityRegistry {
    AbilityRegistry::from_descriptors_checked(vec![
        descriptor("child_b", &[]),
        descriptor("parent_a", static_slice(vec![composed_entry(optional)])),
    ])
    .unwrap()
}

fn child_b_fails() -> AbilityResult<FixtureOutput> {
    Err(AbilityError {
        kind: AbilityErrorKind::HardError("simulated".to_string()),
        message: "simulated child read failed".to_string(),
    })
}

fn error_kind_label(kind: &AbilityErrorKind) -> String {
    match kind {
        AbilityErrorKind::Validation => "Validation".to_string(),
        AbilityErrorKind::Capability => "Capability".to_string(),
        AbilityErrorKind::OptionalComposedReadFailed { .. } => {
            "OptionalComposedReadFailed".to_string()
        }
        AbilityErrorKind::HardError(_) => "HardError".to_string(),
    }
}

fn parent_output(warnings: Vec<ProvenanceWarning>) -> AbilityOutput<FixtureOutput> {
    let produced_at = chrono::Utc
        .with_ymd_and_hms(2026, 5, 1, 12, 0, 0)
        .unwrap();
    let subject = SubjectAttribution::direct_confident(SubjectRef::Account("acct-fixture".into()));
    let mut builder = ProvenanceBuilder::new(ProvenanceBuilderConfig::new("parent_a", produced_at));
    builder.set_subject(subject.clone());
    for warning in warnings {
        builder.add_warning(warning);
    }
    builder
        .attribute(
            FieldPath::new("/ok").unwrap(),
            FieldAttribution::constant(subject),
        )
        .unwrap();
    builder.finalize(FixtureOutput { ok: true }).unwrap()
}

fn parent_a_optional() -> AbilityResult<FixtureOutput> {
    let composition_id = CompositionId::new("b-read");
    let warnings = match child_b_fails() {
        Ok(_) => Vec::new(),
        Err(error) => vec![ProvenanceWarning::OptionalComposedReadFailed {
            composition_id,
            reason: error_kind_label(&error.kind),
        }],
    };

    Ok(parent_output(warnings))
}

fn parent_a_nonoptional() -> AbilityResult<FixtureOutput> {
    child_b_fails()?;
    Ok(parent_output(Vec::new()))
}

#[test]
fn optional_composed_read_failure_emits_warning_not_error() {
    let registry = register_pair(true);
    assert_eq!(registry.iter_for(Actor::System).count(), 2);

    let output = parent_a_optional().unwrap();
    assert!(output.data().ok);
    assert_eq!(output.provenance().warnings.len(), 1);

    match &output.provenance().warnings[0] {
        ProvenanceWarning::OptionalComposedReadFailed {
            composition_id,
            reason,
        } => {
            assert_eq!(composition_id, &CompositionId::new("b-read"));
            assert_eq!(reason, "HardError");
        }
        other => panic!("expected optional composed warning, got {other:?}"),
    }
}

#[test]
fn nonoptional_composed_read_failure_propagates_as_hard_error() {
    let registry = register_pair(false);
    assert_eq!(registry.iter_for(Actor::System).count(), 2);

    let err = parent_a_nonoptional().unwrap_err();
    assert_eq!(err.kind, AbilityErrorKind::HardError("simulated".to_string()));
}
