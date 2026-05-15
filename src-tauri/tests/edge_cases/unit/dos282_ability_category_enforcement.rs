use std::future::Future;
use std::pin::Pin;

use dailyos_lib::abilities::registry::{
    AbilityPolicy, McpExposure, RegistryViolation, SignalPolicy,
};
use dailyos_lib::abilities::{
    AbilityCategory, AbilityContext, AbilityDescriptor, AbilityError, AbilityRegistry, ActorKind,
};
use serde_json::json;

type ErasedFuture<'a> =
    Pin<Box<dyn Future<Output = Result<serde_json::Value, AbilityError>> + Send + 'a>>;

#[test]
fn read_and_transform_categories_cannot_mutate_directly_or_transitively() {
    let direct = AbilityRegistry::from_descriptors_checked(vec![descriptor(
        "read_mutator",
        AbilityCategory::Read,
        &[],
        &["intelligence_claims"],
    )])
    .unwrap_err();
    assert!(direct.iter().any(|violation| matches!(
        violation,
        RegistryViolation::CategoryViolation {
            ability,
            category: AbilityCategory::Read,
            ..
        } if ability == "read_mutator"
    )));

    let transitive = AbilityRegistry::from_descriptors_checked(vec![
        descriptor("transform_parent", AbilityCategory::Transform, &["publisher"], &[]),
        descriptor("publisher", AbilityCategory::Publish, &[], &["actions"]),
    ])
    .unwrap_err();
    assert!(transitive.iter().any(|violation| matches!(
        violation,
        RegistryViolation::CategoryViolation {
            ability,
            category: AbilityCategory::Transform,
            transitively_composes: AbilityCategory::Publish,
        } if ability == "transform_parent"
    )));
}

fn descriptor(
    name: &'static str,
    category: AbilityCategory,
    composes: &'static [&'static str],
    mutates: &'static [&'static str],
) -> AbilityDescriptor {
    AbilityDescriptor {
        name,
        version: "0.0.1-test",
        schema_version: 1,
        category,
        policy: AbilityPolicy {
            allowed_actors: &[ActorKind::System],
            allowed_modes: &[],
            requires_confirmation: false,
            required_scopes: &[],
            client_side_executable: false,
            rate_limit: None,
            may_publish: category == AbilityCategory::Publish,
            mcp_exposure: McpExposure::None,
        },
        composes: Box::leak(
            composes
                .iter()
                .map(|ability| dailyos_lib::abilities::registry::ComposesEntry {
                    id: dailyos_lib::abilities::provenance::CompositionId::new(*ability),
                    ability,
                    optional: false,
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ),
        mutates,
        experimental: false,
        registered_at: Some("2026-05-15"),
        signal_policy: SignalPolicy::default(),
        invoke_erased: |_ctx: &AbilityContext<'_>, _input| {
            Box::pin(async { Ok(json!({ "ok": true })) }) as ErasedFuture<'_>
        },
        input_schema: || json!({ "type": "object", "additionalProperties": false }),
        output_schema: || json!({ "type": "object", "additionalProperties": false }),
    }
}
