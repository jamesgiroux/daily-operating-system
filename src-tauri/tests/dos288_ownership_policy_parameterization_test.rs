use std::future::Future;
use std::pin::Pin;

use chrono::{TimeZone, Utc};
use dailyos_lib::abilities::provenance::{
    build_ownership_policy_for_invocation, validate_ability_output_value_ownership, DataSource,
    EntityId, FieldAttribution, FieldPath, OwnershipError, OwnershipPolicy, ProvenanceBuilder,
    ProvenanceBuilderConfig, SourceAttribution, SourceIdentifier, SubjectAttribution, SubjectRef,
};
use dailyos_lib::abilities::registry::{AbilityPolicy, McpExposure, SignalPolicy};
use dailyos_lib::abilities::{
    AbilityCategory, AbilityContext, AbilityDescriptor, AbilityError, ActorKind,
};
use dailyos_lib::services::context::ExecutionMode;
use serde_json::{json, Value};

const USER_ACTORS: &[ActorKind] = &[ActorKind::User];
const LIVE_MODES: &[ExecutionMode] = &[ExecutionMode::Live];

type ErasedFuture<'a> =
    Pin<Box<dyn Future<Output = Result<serde_json::Value, AbilityError>> + Send + 'a>>;

fn unused_invoke<'a>(_ctx: &'a AbilityContext<'a>, _input: serde_json::Value) -> ErasedFuture<'a> {
    Box::pin(async { unreachable!("descriptor is metadata-only for this regression") })
}

fn closed_object_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "entity_type": { "type": "string" },
            "entity_id": { "type": "string" }
        }
    })
}

fn ability_descriptor() -> AbilityDescriptor {
    AbilityDescriptor {
        name: "dos288_parameterization_fixture",
        version: "1.0.0",
        schema_version: 1,
        category: AbilityCategory::Read,
        policy: AbilityPolicy {
            allowed_actors: USER_ACTORS,
            allowed_modes: LIVE_MODES,
            requires_confirmation: false,
            may_publish: false,
            required_scopes: &[],
            mcp_exposure: McpExposure::None,
            client_side_executable: false,
            rate_limit: None,
        },
        composes: &[],
        mutates: &[],
        experimental: false,
        registered_at: None,
        signal_policy: SignalPolicy::default(),
        invoke_erased: unused_invoke,
        input_schema: closed_object_schema,
        output_schema: closed_object_schema,
    }
}

fn produced_at() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 0).unwrap()
}

fn bridge_output_with_source_entity(source_entity_id: &str) -> Value {
    let subject = SubjectAttribution::direct_confident(SubjectRef::Account("acct-target".into()));
    let mut builder = ProvenanceBuilder::new(ProvenanceBuilderConfig::new(
        "dos288_parameterization_fixture",
        produced_at(),
    ));
    builder.set_subject(subject.clone());
    let source = SourceAttribution::new(
        DataSource::User,
        vec![SourceIdentifier::Entity {
            entity_id: EntityId::new(source_entity_id),
            field: Some("claim".into()),
        }],
        produced_at(),
        Some(produced_at()),
        1.0,
        None,
    )
    .unwrap();
    let source_index = builder.add_source(source);
    builder
        .attribute(
            FieldPath::new("/claim").unwrap(),
            FieldAttribution::direct(subject, source_index),
        )
        .unwrap();
    let output = builder
        .finalize(json!({ "claim": "Target account renewal is on track." }))
        .unwrap();
    serde_json::to_value(output).unwrap()
}

#[test]
fn parameterized_policy_rejects_out_of_scope_source_that_empty_policy_allows() {
    let output = bridge_output_with_source_entity("acct-other");

    let empty_policy_report =
        validate_ability_output_value_ownership(output.clone(), &[], OwnershipPolicy::confident())
            .expect("empty policy preserves the historical false pass");
    assert_eq!(empty_policy_report.source_refs_resolved.len(), 1);
    assert!(!empty_policy_report.source_refs_resolved[0].entity_link_evidence);

    let input_json = json!({
        "entity_type": "account",
        "entity_id": "acct-target"
    });
    let policy = build_ownership_policy_for_invocation(
        &ability_descriptor(),
        &input_json,
        &output["provenance"],
    )
    .expect("policy builds from bridge metadata, input, and provenance");

    assert!(policy.require_entity_link_evidence);
    assert_eq!(
        policy
            .target_footprint
            .as_ref()
            .map(|footprint| &footprint.subject),
        Some(&SubjectRef::Account("acct-target".into()))
    );

    let err = validate_ability_output_value_ownership(output, &[], policy).unwrap_err();
    assert!(matches!(
        err,
        OwnershipError::SourceRefWithoutEntityLinkEvidence { .. }
    ));
}
