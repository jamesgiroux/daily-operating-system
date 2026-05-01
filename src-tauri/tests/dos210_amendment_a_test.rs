use std::collections::BTreeMap;

use chrono::TimeZone;
use dailyos_lib::abilities::provenance::{
    provenance_for_test, AbilityOutput, CompositionId, Diagnostics, ProvenanceWarning,
    SubjectAttribution, SubjectRef,
};
use dailyos_lib::abilities::registry::{AbilityPolicy, ComposesEntry, SignalPolicy};
use dailyos_lib::abilities::{
    AbilityCategory, AbilityContext, AbilityDescriptor, AbilityError, AbilityErrorKind,
    AbilityRegistry, AbilityResult, Actor,
};
use dailyos_lib::services::context::ExecutionMode;

#[derive(Debug, PartialEq, Eq)]
struct FixtureOutput {
    ok: bool,
}

fn passthrough_erased(
    _ctx: &AbilityContext<'_>,
    input: serde_json::Value,
) -> Result<serde_json::Value, AbilityError> {
    Ok(input)
}

fn empty_schema() -> serde_json::Value {
    serde_json::json!({ "type": "object" })
}

fn descriptor(name: &'static str, composes: Vec<ComposesEntry>) -> AbilityDescriptor {
    AbilityDescriptor {
        name,
        version: "0.1.0",
        schema_version: 1,
        category: AbilityCategory::Read,
        policy: AbilityPolicy {
            allowed_actors: vec![Actor::System],
            allowed_modes: vec![ExecutionMode::Evaluate],
            requires_confirmation: false,
            may_publish: false,
        },
        composes,
        mutates: Vec::new(),
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
        ability: "child_b".to_string(),
        optional,
    }
}

fn register_pair(optional: bool) -> AbilityRegistry {
    AbilityRegistry::from_descriptors_checked(vec![
        descriptor("child_b", Vec::new()),
        descriptor("parent_a", vec![composed_entry(optional)]),
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
    let provenance = provenance_for_test(
        "parent_a",
        produced_at,
        subject,
        Vec::new(),
        Vec::new(),
        BTreeMap::new(),
        None,
        warnings,
    );

    AbilityOutput {
        data: FixtureOutput { ok: true },
        ability_version: provenance.ability_version.clone(),
        diagnostics: Diagnostics::default(),
        provenance,
    }
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
    assert!(output.data.ok);
    assert_eq!(output.provenance.warnings.len(), 1);

    match &output.provenance.warnings[0] {
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
