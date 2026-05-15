use abilities_runtime::abilities::composition::{
    AbilityRef, BindingRole, Block, BlockId, BlockType, ClaimRef, ClaimRefIndex, Composition,
    CompositionDocId, CompositionKind, CompositionMetadata, CompositionVersion, FieldBinding,
    ProvenanceRef, RenderHints, Salience, Section, SectionId,
};
use abilities_runtime::abilities::provenance::{FieldPath, InvocationId, SchemaVersion};
use abilities_runtime::abilities::registry::{Actor, ScopeSet, SurfaceClientId, SurfaceScope};
use abilities_runtime::abilities::{
    project_composition_for_surface, register_custom_block_schema, CustomBlockSchema,
    EditRouteRefusalReason, FallbackProjectionContext, ProducerOutputInvalidReason,
    ProjectionError, SurfaceKind, FALLBACK_BANNER_COPY,
};
use chrono::{TimeZone, Utc};
use serde_json::{json, Value};

fn provenance_ref() -> ProvenanceRef {
    ProvenanceRef::from_pointer(
        InvocationId(uuid::Uuid::from_u128(
            0x1234_5678_90ab_cdef_1122_3344_5566_7788,
        )),
        "/sections/0/blocks/0",
    )
    .unwrap()
}

fn claim(id: &str, field_path: &str) -> ClaimRef {
    ClaimRef::with_field(id, 7, FieldPath::new(field_path).unwrap())
}

fn binding(field_path: &str, role: BindingRole, claim_refs: &[usize]) -> FieldBinding {
    FieldBinding {
        field_path: FieldPath::new(field_path).unwrap(),
        role,
        claim_refs: claim_refs.iter().copied().map(ClaimRefIndex).collect(),
    }
}

fn block(
    id: &str,
    block_type: BlockType,
    attributes: Value,
    claim_refs: Vec<ClaimRef>,
    field_bindings: Vec<FieldBinding>,
) -> Block {
    Block {
        id: BlockId::new(id),
        block_type,
        attributes,
        claim_refs,
        field_bindings,
        provenance: provenance_ref(),
        salience: Salience::default(),
        render_hints: RenderHints::default(),
    }
}

fn composition(blocks: Vec<Block>) -> Composition {
    let generated_at = Utc.with_ymd_and_hms(2026, 5, 15, 0, 0, 0).unwrap();
    let mut composition = Composition::empty(
        CompositionDocId::new("composition-fixture"),
        CompositionVersion::new(11),
        generated_at,
    );
    composition.kind = CompositionKind::EntityPage;
    composition.sections = vec![Section::new(SectionId::new("section-fixture"), blocks)];
    composition.generated_by = AbilityRef::new("fixture.ability");
    composition.metadata = CompositionMetadata {
        schema_version: SchemaVersion(1),
        generated_at,
        composition_version: CompositionVersion::new(11),
        generated_by: "fixture.ability".to_string(),
    };
    composition
}

fn ctx() -> FallbackProjectionContext {
    FallbackProjectionContext::new(Actor::User, SurfaceKind::TauriApp, 3)
}

fn surface_ctx(scopes: &[&str]) -> FallbackProjectionContext {
    let scopes = ScopeSet::new(scopes.iter().map(|scope| SurfaceScope::new(*scope))).unwrap();
    FallbackProjectionContext::new(
        Actor::SurfaceClient {
            instance: SurfaceClientId::new("surface-fixture"),
            scopes,
        },
        SurfaceKind::SurfaceClient,
        3,
    )
}

fn register_schema(type_id: &str, required: &[&str], optional: &[&str], annotations: &[&str]) {
    let mut schema = CustomBlockSchema::new(type_id);
    schema.composition_kind = Some("entity_page".to_string());
    schema.required_pointers = required.iter().map(|value| (*value).to_string()).collect();
    schema.optional_pointers = optional.iter().map(|value| (*value).to_string()).collect();
    schema.render_annotations = annotations
        .iter()
        .map(|value| (*value).to_string())
        .collect();
    register_custom_block_schema(schema);
}

fn project(
    comp: &Composition,
    context: &FallbackProjectionContext,
) -> (
    abilities_runtime::abilities::ProjectedComposition,
    Vec<abilities_runtime::abilities::AuditIntent>,
) {
    project_composition_for_surface(comp, context).expect("projection succeeds")
}

#[test]
fn known_block_type_coverage_gate() {
    let fixtures = vec![
        (
            BlockType::AccountOverview,
            json!({"summary": "Summary"}),
            "/summary",
        ),
        (BlockType::ClaimSummary, json!({"title": "Claim"}), "/title"),
        (
            BlockType::EvidenceList,
            json!({"items": [{"label": "Evidence"}]}),
            "/items/0/label",
        ),
        (
            BlockType::HealthSnapshot,
            json!({"band": "steady"}),
            "/band",
        ),
        (
            BlockType::RelationshipMap,
            json!({"nodes": [{"label": "Team"}]}),
            "/nodes/0/label",
        ),
        (BlockType::RiskCallout, json!({"title": "Risk"}), "/title"),
        (
            BlockType::ActionList,
            json!({"items": [{"title": "Follow up"}]}),
            "/items/0/title",
        ),
        (
            BlockType::MarkdownDocument,
            json!({"title": "Doc"}),
            "/title",
        ),
    ];
    for (block_type, attrs, field_path) in fixtures {
        let block = block(
            "known",
            block_type,
            attrs,
            vec![claim("claim-known", field_path)],
            vec![],
        );
        let result = project_composition_for_surface(&composition(vec![block]), &ctx());
        assert!(result.is_ok());
    }
}

#[test]
fn no_catch_all_admitted_field_gate() {
    register_schema(
        "dailyos/no-catch-all",
        &["/title"],
        &["/private"],
        &["document"],
    );
    let custom = block(
        "block-no-catch-all",
        BlockType::Custom {
            type_id: "dailyos/no-catch-all".to_string(),
        },
        json!({"title": "Visible", "private": "sentinel-private"}),
        vec![],
        vec![],
    );
    let (projection, _) = project(&composition(vec![custom]), &ctx());
    let serialized = serde_json::to_string(&projection).unwrap();
    assert!(!serialized.contains("sentinel-private"));
}

#[test]
fn dos570_fixture_1_unknown_sensitive_payload_dropped() {
    register_schema("dailyos/custom-sensitive", &["/title"], &[], &["document"]);
    let sentinel = "sentinel-user@example.com";
    let custom = block(
        "block-sensitive",
        BlockType::Custom {
            type_id: "dailyos/custom-sensitive".to_string(),
        },
        json!({"title": "Visible", "secret_email": sentinel, "nested": {"token": sentinel}}),
        vec![claim("claim-1", "/title")],
        vec![binding("/title", BindingRole::Source, &[0])],
    );
    let (projection, audits) = project(&composition(vec![custom]), &ctx());
    let serialized = serde_json::to_string(&(projection, audits)).unwrap();
    assert!(!serialized.contains(sentinel));
}

#[test]
fn dos570_fixture_2_refs_preserved() {
    register_schema("dailyos/custom-refs", &["/title"], &[], &["document"]);
    let refs = vec![claim("claim-preserved", "/title")];
    let provenance = provenance_ref();
    let mut custom = block(
        "block-refs",
        BlockType::Custom {
            type_id: "dailyos/custom-refs".to_string(),
        },
        json!({"title": "Visible", "private_note": "hidden"}),
        refs.clone(),
        vec![binding("/title", BindingRole::FeedbackTarget, &[0])],
    );
    custom.provenance = provenance.clone();
    let (projection, _) = project(&composition(vec![custom]), &ctx());
    assert_eq!(projection.blocks[0].claim_refs, refs);
    assert_eq!(projection.blocks[0].provenance, vec![provenance]);
    assert!(projection.blocks[0]
        .payload
        .pointer("/private_note")
        .is_none());
}

#[test]
fn dos570_fixture_3_no_schema_generic() {
    let custom = block(
        "block-no-schema",
        BlockType::Custom {
            type_id: "dailyos/no-schema".to_string(),
        },
        json!({"title": "Hidden"}),
        vec![claim("claim-1", "/title")],
        vec![],
    );
    let (projection, _) = project(&composition(vec![custom]), &ctx());
    assert_eq!(projection.blocks[0].selected_known_type_id, "dailyos/text");
    assert_eq!(projection.blocks[0].payload, json!({}));
    assert_eq!(
        projection.blocks[0].banner.as_deref(),
        Some(FALLBACK_BANNER_COPY)
    );
}

#[test]
fn dos570_fixture_4_no_intersection_generic() {
    register_schema(
        "dailyos/no-intersection",
        &["/unmatched"],
        &[],
        &["unknown"],
    );
    let custom = block(
        "block-no-intersection",
        BlockType::Custom {
            type_id: "dailyos/no-intersection".to_string(),
        },
        json!({"unmatched": "Do not invent text"}),
        vec![],
        vec![],
    );
    let (projection, _) = project(&composition(vec![custom]), &ctx());
    assert_eq!(projection.blocks[0].selected_known_type_id, "dailyos/text");
    assert_eq!(projection.blocks[0].payload, json!({}));
}

#[test]
fn dos570_fixture_5_array_projection() {
    register_schema(
        "dailyos/custom-actions",
        &["/items/*/title"],
        &["/items/*/private_note"],
        &["actions"],
    );
    let custom = block(
        "block-array",
        BlockType::Custom {
            type_id: "dailyos/custom-actions".to_string(),
        },
        json!({"items": [{"title": "Follow up", "private_note": "sentinel-private"}]}),
        vec![claim("claim-action", "/items/0/title")],
        vec![binding("/items/0/title", BindingRole::FeedbackTarget, &[0])],
    );
    let (projection, _) = project(&composition(vec![custom]), &ctx());
    assert_eq!(
        projection.blocks[0].payload.pointer("/items/0/title"),
        Some(&json!("Follow up"))
    );
    assert!(!serde_json::to_string(&projection)
        .unwrap()
        .contains("sentinel-private"));
}

#[test]
fn dos570_fixture_6_unsafe_widening_rejected() {
    register_schema(
        "dailyos/unsafe-widening",
        &["/title", "/body"],
        &[],
        &["document"],
    );
    let custom = block(
        "block-unsafe",
        BlockType::Custom {
            type_id: "dailyos/unsafe-widening".to_string(),
        },
        json!({"title": {"object": "sentinel-object"}, "body": ["sentinel-array"]}),
        vec![],
        vec![],
    );
    let (projection, _) = project(&composition(vec![custom]), &ctx());
    let serialized = serde_json::to_string(&projection).unwrap();
    assert!(!serialized.contains("sentinel-object"));
    assert!(!serialized.contains("sentinel-array"));
}

#[test]
fn dos570_fixture_7_cap_exceeded() {
    let blocks: Vec<Block> = (0..9)
        .map(|index| {
            block(
                &format!("block-cap-{index}"),
                BlockType::Custom {
                    type_id: format!("dailyos/no-schema-cap-{index}"),
                },
                json!({"title": "Hidden"}),
                vec![],
                vec![],
            )
        })
        .collect();
    let mut context = ctx();
    context.unknown_block_cap = 5;
    let (projection, audits) = project(&composition(blocks), &context);
    assert_eq!(projection.blocks.len(), 5);
    assert_eq!(projection.unknown_block_count, 9);
    assert_eq!(projection.dropped_unknown_block_count, 4);
    assert_eq!(
        audits
            .iter()
            .filter(|audit| audit.event_kind == "custom_block_fallback_cap_exceeded")
            .count(),
        1
    );
}

#[test]
fn dos570_fixture_8_diagnostics_redaction() {
    let known = block(
        "block-redaction",
        BlockType::RiskCallout,
        json!({"title": "Risk", "body": "Body"}),
        vec![claim("claim-redacted", "/title")],
        vec![binding("/title", BindingRole::Source, &[0])],
    );
    let (projection, _) = project(
        &composition(vec![known]),
        &surface_ctx(&["submit.feedback"]),
    );
    assert!(projection.blocks[0].payload.pointer("/title").is_none());
    let serialized = serde_json::to_string(&projection.diagnostics).unwrap();
    assert!(!serialized.contains("/title"));
    assert!(serialized.contains("out_of_scope"));
}

#[test]
fn dos570_fixture_9_binding_role_matrix() {
    let known = block(
        "block-roles",
        BlockType::RiskCallout,
        json!({"title": "Risk", "body": "Body", "severity": "high", "recommended_action": "Call"}),
        vec![
            claim("claim-source", "/title"),
            claim("claim-target", "/recommended_action"),
        ],
        vec![
            binding("/title", BindingRole::Source, &[0]),
            binding("/severity", BindingRole::ComputedFrom, &[0]),
            binding("/body", BindingRole::DisplayOnly, &[]),
            binding("/recommended_action", BindingRole::FeedbackTarget, &[1]),
        ],
    );
    let (projection, _) = project(
        &composition(vec![known]),
        &surface_ctx(&["read.composition", "submit.feedback"]),
    );
    let routes = &projection.blocks[0].edit_routes;
    assert!(
        !routes
            .iter()
            .find(|route| route.role == BindingRole::Source)
            .unwrap()
            .feedback_allowed
    );
    assert!(
        !routes
            .iter()
            .find(|route| route.role == BindingRole::ComputedFrom)
            .unwrap()
            .feedback_allowed
    );
    assert!(
        !routes
            .iter()
            .find(|route| route.role == BindingRole::DisplayOnly)
            .unwrap()
            .feedback_allowed
    );
    assert!(
        routes
            .iter()
            .find(|route| route.role == BindingRole::FeedbackTarget)
            .unwrap()
            .feedback_allowed
    );
    assert!(serde_json::from_value::<BindingRole>(json!("future_role")).is_err());
}

#[test]
fn dos570_fixture_10_source_without_feedback_target_refused() {
    let known = block(
        "block-source",
        BlockType::ClaimSummary,
        json!({"title": "Claim"}),
        vec![claim("claim-source", "/title")],
        vec![binding("/title", BindingRole::Source, &[0])],
    );
    let (projection, _) = project(&composition(vec![known]), &ctx());
    let route = &projection.blocks[0].edit_routes[0];
    assert!(!route.feedback_allowed);
    assert_eq!(
        route.refusal_reason,
        Some(EditRouteRefusalReason::SourceWithoutTarget)
    );
}

#[test]
fn dos570_fixture_11_computed_from_refused() {
    let known = block(
        "block-computed",
        BlockType::HealthSnapshot,
        json!({"band": "steady"}),
        vec![claim("claim-computed", "/band")],
        vec![binding("/band", BindingRole::ComputedFrom, &[0])],
    );
    let (projection, _) = project(
        &composition(vec![known.clone()]),
        &surface_ctx(&["read.composition"]),
    );
    assert!(projection.blocks[0].payload.pointer("/band").is_some());
    assert_eq!(
        projection.blocks[0].edit_routes[0].refusal_reason,
        Some(EditRouteRefusalReason::Computed)
    );
    let (projection, _) = project(
        &composition(vec![known]),
        &surface_ctx(&["submit.feedback"]),
    );
    assert!(projection.blocks[0].payload.pointer("/band").is_none());
}

#[test]
fn dos570_fixture_12_display_only_no_affordance() {
    let known = block(
        "block-display",
        BlockType::MarkdownDocument,
        json!({"title": "Read only"}),
        vec![claim("claim-display", "/title")],
        vec![binding("/title", BindingRole::DisplayOnly, &[0])],
    );
    let (projection, _) = project(
        &composition(vec![known]),
        &surface_ctx(&["read.composition", "submit.feedback"]),
    );
    assert_eq!(
        projection.blocks[0].edit_routes[0].refusal_reason,
        Some(EditRouteRefusalReason::DisplayOnly)
    );
    assert!(!projection.blocks[0].edit_routes[0].feedback_allowed);
}

#[test]
fn dos570_fixture_13_feedback_target_routes() {
    let known = block(
        "block-feedback",
        BlockType::RiskCallout,
        json!({"recommended_action": "Call", "title": "Risk"}),
        vec![claim("claim-target", "/recommended_action")],
        vec![
            binding("/recommended_action", BindingRole::FeedbackTarget, &[0]),
            binding("/title", BindingRole::FeedbackTarget, &[]),
        ],
    );
    let (projection, _) = project(
        &composition(vec![known]),
        &surface_ctx(&["read.composition", "submit.feedback"]),
    );
    let allowed = projection.blocks[0]
        .edit_routes
        .iter()
        .find(|route| route.field_path.as_str() == "/recommended_action")
        .unwrap();
    assert!(allowed.feedback_allowed);
    assert_eq!(allowed.claim_refs[0].claim_id, "claim-target");
    assert_eq!(allowed.claim_refs[0].claim_version, Some(7));
    assert_eq!(
        allowed.claim_refs[0].field_path.as_ref().unwrap().as_str(),
        "/recommended_action"
    );
    let refused = projection.blocks[0]
        .edit_routes
        .iter()
        .find(|route| route.field_path.as_str() == "/title")
        .unwrap();
    assert!(!refused.feedback_allowed);
    assert_eq!(
        refused.refusal_reason,
        Some(EditRouteRefusalReason::MissingClaimRef)
    );
}

#[test]
fn dos570_fixture_14_policy_version_cache_key() {
    register_schema("dailyos/policy-version", &["/title"], &[], &["document"]);
    let custom = block(
        "block-policy",
        BlockType::Custom {
            type_id: "dailyos/policy-version".to_string(),
        },
        json!({"title": "Stable"}),
        vec![],
        vec![],
    );
    let comp = composition(vec![custom]);
    let mut first = ctx();
    first.fallback_policy_version = 3;
    let mut second = ctx();
    second.fallback_policy_version = 4;
    let (a, _) = project(&comp, &first);
    let (b, _) = project(&comp, &second);
    assert_ne!(a.fallback_policy_version, b.fallback_policy_version);
    assert_eq!(a.blocks[0].payload, b.blocks[0].payload);
}

#[test]
fn invalid_producer_output_gate_is_closed_enum() {
    let known = block(
        "block-invalid",
        BlockType::ClaimSummary,
        json!({"title": "Claim"}),
        vec![ClaimRef::with_version("claim-source", 7)],
        vec![binding("/title", BindingRole::Source, &[0])],
    );
    let error = project_composition_for_surface(&composition(vec![known]), &ctx()).unwrap_err();
    assert_eq!(
        error,
        ProjectionError::InvalidProducerOutput {
            reason: ProducerOutputInvalidReason::SourceBindingMissingFieldPath
        }
    );
}
