use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::sync::{OnceLock, RwLock};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use thiserror::Error;

use crate::abilities::composition::{
    BindingRole, Block, BlockId, BlockType, ClaimRef, ClaimRefIndex, Composition, CompositionDocId,
    CompositionVersion, FieldBinding, ProvenanceRef,
};
use crate::abilities::provenance::{CompositionId, FieldPath};
use crate::abilities::registry::{Actor, ActorKind, SurfaceScope};
use crate::abilities::trust::TrustBand;
use crate::sensitivity::{
    render_policy_for_surface, ClaimVerificationState, RenderActor, RenderDecision, RenderSurface,
};
use crate::types::{
    ClaimSensitivity, ClaimState, IntelligenceClaim, SurfacingState, TemporalScope,
};

pub const DEFAULT_UNKNOWN_BLOCK_CAP: u32 = 5;
pub const FALLBACK_BANNER_COPY: &str =
    "Rendered as nearest known type — payload may be incomplete.";
const GENERIC_TEXT_TYPE_ID: &str = "dailyos/text";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectedComposition {
    pub composition_id: CompositionDocId,
    pub composition_version: Option<u64>,
    pub fallback_policy_version: u32,
    pub blocks: Vec<ProjectedBlock>,
    pub diagnostics: Vec<ProjectionDiagnostic>,
    pub unknown_block_count: u32,
    pub unknown_block_cap: u32,
    pub dropped_unknown_block_count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectedBlock {
    pub block_id: BlockId,
    pub block_index: u32,
    pub original_type_id: String,
    pub selected_known_type_id: String,
    pub payload: Value,
    pub banner: Option<String>,
    pub trust_band: TrustBand,
    pub claim_refs: Vec<ClaimRef>,
    pub provenance: Vec<ProvenanceRef>,
    pub edit_routes: Vec<EditRoute>,
    pub diagnostics: Vec<ProjectionDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct EditRoute {
    pub field_path: FieldPath,
    pub role: BindingRole,
    pub claim_refs: Vec<ClaimRef>,
    pub feedback_allowed: bool,
    pub refusal_reason: Option<EditRouteRefusalReason>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectionDiagnostic {
    pub diagnostic_kind: DiagnosticKind,
    pub composition_id: CompositionId,
    pub composition_version: u64,
    pub original_type_id: Option<String>,
    pub selected_known_type_id: Option<String>,
    pub block_id: Option<BlockId>,
    pub reason: DiagnosticReason,
    pub dropped_pointer_count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AuditIntent {
    pub event_kind: &'static str,
    pub category: AuditCategory,
    pub detail: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AuditCategory {
    Security,
    DataAccess,
    Ai,
    Anomaly,
    Config,
    System,
}

impl AuditCategory {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Security => "security",
            Self::DataAccess => "data_access",
            Self::Ai => "ai",
            Self::Anomaly => "anomaly",
            Self::Config => "config",
            Self::System => "system",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FallbackProjectionContext {
    pub actor: Actor,
    pub surface: SurfaceKind,
    pub fallback_policy_version: u32,
    pub unknown_block_cap: u32,
    pub include_non_sensitive_pointer_names: bool,
}

impl FallbackProjectionContext {
    pub fn new(actor: Actor, surface: SurfaceKind, fallback_policy_version: u32) -> Self {
        Self {
            actor,
            surface,
            fallback_policy_version,
            unknown_block_cap: DEFAULT_UNKNOWN_BLOCK_CAP,
            include_non_sensitive_pointer_names: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceKind {
    TauriApp,
    McpTool,
    McpToolDetail,
    SurfaceClient,
    Worker,
    Eval,
}

impl SurfaceKind {
    fn render_surface(self) -> RenderSurface {
        match self {
            Self::TauriApp => RenderSurface::TauriReport,
            Self::McpTool => RenderSurface::McpTool,
            Self::McpToolDetail => RenderSurface::McpToolDetail,
            Self::SurfaceClient => RenderSurface::McpTool,
            Self::Worker | Self::Eval => RenderSurface::LogStructured,
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ProjectionError {
    #[error("missing block projection rule for {block_type:?}")]
    MissingRule { block_type: BlockType },
    #[error("invalid producer output: {reason:?}")]
    InvalidProducerOutput { reason: ProducerOutputInvalidReason },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProducerOutputInvalidReason {
    SourceBindingMissingFieldPath,
    FeedbackTargetMissingFieldPath,
    BindingTargetsUnknownField,
    UnknownRole,
    AmbiguousReceiver,
    ConflictingDuplicateBinding,
    MissingDeclaredSchema,
    InvalidFieldPath,
    NonClaimFeedbackReceiverUnsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EditRouteRefusalReason {
    Computed,
    DisplayOnly,
    SourceWithoutTarget,
    MissingClaimRef,
    AmbiguousReceiver,
    SensitivityBlocked,
    UnknownRole,
    OutOfScope,
    FallbackDegradedWithoutReceiver,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticKind {
    BlockFallback,
    FieldDropped,
    EditRouteRefused,
    CapExceeded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticReason {
    UnknownBlockType,
    UnknownBlockCapExceeded,
    UnknownAdmittedField,
    SensitivityBlocked,
    OutOfScope,
    UnknownRole,
    MissingClaimRef,
    AmbiguousReceiver,
    FallbackDegradedWithoutReceiver,
}

#[derive(Debug, Clone)]
pub struct CustomBlockSchema {
    pub type_id: String,
    pub composition_kind: Option<String>,
    pub required_pointers: BTreeSet<String>,
    pub optional_pointers: BTreeSet<String>,
    pub render_annotations: BTreeSet<String>,
}

impl CustomBlockSchema {
    pub fn new(type_id: impl Into<String>) -> Self {
        Self {
            type_id: type_id.into(),
            composition_kind: None,
            required_pointers: BTreeSet::new(),
            optional_pointers: BTreeSet::new(),
            render_annotations: BTreeSet::new(),
        }
    }

    fn declared_pointers(&self) -> impl Iterator<Item = &String> {
        self.required_pointers
            .iter()
            .chain(self.optional_pointers.iter())
    }
}

#[derive(Debug, Clone)]
struct FieldPolicy {
    pointer: &'static str,
    sensitivity: ClaimSensitivity,
    allowed_surfaces: &'static [SurfaceKind],
    value_kind: ValueKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValueKind {
    Text,
    Number,
    Object,
    Bool,
    Array,
}

#[derive(Debug, Clone)]
struct BlockProjectionRule {
    block_type: BlockType,
    composition_kind: Option<&'static str>,
    type_namespace: Option<&'static str>,
    render_annotations: &'static [&'static str],
    fields: &'static [FieldPolicy],
    default_trust_band: TrustBand,
}

#[derive(Debug, Clone, Copy, Default)]
struct NearestTypeScore {
    kind_match: u32,
    required_overlap: u32,
    optional_overlap: u32,
    annotation_similarity: u32,
    namespace_similarity: u32,
}

impl NearestTypeScore {
    fn total(self) -> u32 {
        self.kind_match
            + self.required_overlap
            + self.optional_overlap
            + self.annotation_similarity
            + self.namespace_similarity
    }
}

static CUSTOM_BLOCK_SCHEMAS: OnceLock<RwLock<BTreeMap<String, CustomBlockSchema>>> =
    OnceLock::new();

pub fn register_custom_block_schema(schema: CustomBlockSchema) {
    let registry = CUSTOM_BLOCK_SCHEMAS.get_or_init(|| RwLock::new(BTreeMap::new()));
    registry
        .write()
        .expect("custom block schema registry poisoned")
        .insert(schema.type_id.clone(), schema);
}

fn custom_block_schema(type_id: &str) -> Option<CustomBlockSchema> {
    CUSTOM_BLOCK_SCHEMAS
        .get_or_init(|| RwLock::new(BTreeMap::new()))
        .read()
        .expect("custom block schema registry poisoned")
        .get(type_id)
        .cloned()
}

pub fn project_composition_for_surface(
    composition: &Composition,
    ctx: &FallbackProjectionContext,
) -> Result<(ProjectedComposition, Vec<AuditIntent>), ProjectionError> {
    binding_role_contract_assertion();

    let version = composition.metadata.composition_version;
    let unknown_block_count = composition
        .blocks()
        .filter(|block| block.block_type.is_custom())
        .count() as u32;
    let mut applied_unknown_blocks = 0_u32;
    let mut dropped_unknown_block_count = 0_u32;
    let mut dropped_block_ids = Vec::new();
    let mut blocks = Vec::new();
    let mut diagnostics = Vec::new();
    let mut audits = Vec::new();

    for (block_index, block) in composition.blocks().enumerate() {
        if let BlockType::Custom { type_id } = &block.block_type {
            if applied_unknown_blocks >= ctx.unknown_block_cap {
                dropped_unknown_block_count = dropped_unknown_block_count.saturating_add(1);
                dropped_block_ids.push(block.id.as_str().to_string());
                continue;
            }
            applied_unknown_blocks = applied_unknown_blocks.saturating_add(1);
            let (projected, mut block_audits) =
                project_custom_block(composition, block, block_index as u32, type_id, ctx)?;
            diagnostics.extend(projected.diagnostics.iter().cloned());
            audits.append(&mut block_audits);
            blocks.push(projected);
        } else {
            let (projected, mut block_audits) =
                project_known_block(composition, block, block_index as u32, ctx)?;
            diagnostics.extend(projected.diagnostics.iter().cloned());
            audits.append(&mut block_audits);
            blocks.push(projected);
        }
    }

    if dropped_unknown_block_count > 0 {
        diagnostics.push(projection_diagnostic(
            composition,
            DiagnosticKind::CapExceeded,
            None,
            None,
            None,
            DiagnosticReason::UnknownBlockCapExceeded,
            dropped_unknown_block_count,
        ));
        audits.push(AuditIntent {
            event_kind: "custom_block_fallback_cap_exceeded",
            category: AuditCategory::Anomaly,
            detail: json!({
                "schema_version": 1,
                "composition_id": composition.id.as_str(),
                "composition_version": composition_version_json(version),
                "unknown_block_count": unknown_block_count,
                "unknown_block_cap": ctx.unknown_block_cap,
                "dropped_unknown_block_count": dropped_unknown_block_count,
                "dropped_block_ids": dropped_block_ids,
                "fallback_policy_version": ctx.fallback_policy_version,
                "reason": "unknown_block_cap_exceeded",
            }),
        });
    }

    Ok((
        ProjectedComposition {
            composition_id: composition.id.clone(),
            composition_version: Some(version.0),
            fallback_policy_version: ctx.fallback_policy_version,
            blocks,
            diagnostics,
            unknown_block_count,
            unknown_block_cap: ctx.unknown_block_cap,
            dropped_unknown_block_count,
        },
        audits,
    ))
}

fn binding_role_contract_assertion() {
    let _ = BindingRole::FeedbackTarget;
    fn field_bindings_resolve(block: &Block) -> &Vec<FieldBinding> {
        &block.field_bindings
    }
    let _ = field_bindings_resolve;
}

fn project_known_block(
    composition: &Composition,
    block: &Block,
    block_index: u32,
    ctx: &FallbackProjectionContext,
) -> Result<(ProjectedBlock, Vec<AuditIntent>), ProjectionError> {
    let rule =
        rule_for_block_type(&block.block_type).ok_or_else(|| ProjectionError::MissingRule {
            block_type: block.block_type.clone(),
        })?;
    validate_field_bindings(block, &rule, false)?;
    let (payload, field_states, mut diagnostics) =
        project_payload_fields(composition, block, &rule, ctx, false);
    let mut audits = Vec::new();
    let dropped_unknown = count_unadmitted_payload_leaves(&block.attributes, &rule);
    if dropped_unknown > 0 {
        diagnostics.push(projection_diagnostic(
            composition,
            DiagnosticKind::FieldDropped,
            Some(block),
            Some(block.block_type.type_id().to_string()),
            Some(rule.block_type.type_id().to_string()),
            DiagnosticReason::UnknownAdmittedField,
            dropped_unknown,
        ));
        audits.push(AuditIntent {
            event_kind: "unknown_admitted_field",
            category: AuditCategory::Anomaly,
            detail: json!({
                "schema_version": 1,
                "composition_id": composition.id.as_str(),
                "composition_version": composition_version_json(composition.metadata.composition_version),
                "block_id": block.id.as_str(),
                "block_index": block_index,
                "original_type_id": block.block_type.type_id(),
                "dropped_pointer_count": dropped_unknown,
                "fallback_policy_version": ctx.fallback_policy_version,
                "reason": "unknown_admitted_field",
            }),
        });
    }
    let edit_routes = edit_routes_for_block(
        composition,
        block,
        &rule,
        &field_states,
        false,
        ctx,
        &mut diagnostics,
    );
    let trust_band =
        trust_band_from_attributes(block).unwrap_or(rule.default_trust_band);
    Ok((
        ProjectedBlock {
            block_id: block.id.clone(),
            block_index,
            original_type_id: block.block_type.type_id().to_string(),
            selected_known_type_id: rule.block_type.type_id().to_string(),
            payload,
            banner: None,
            trust_band,
            claim_refs: block.claim_refs.clone(),
            provenance: vec![block.provenance.clone()],
            edit_routes,
            diagnostics,
        },
        audits,
    ))
}

fn trust_band_from_attributes(block: &Block) -> Option<TrustBand> {
    block
        .attributes
        .get("trust_band")
        .and_then(|value| value.as_str())
        .and_then(|label| match label {
            "likely_current" => Some(TrustBand::LikelyCurrent),
            "use_with_caution" => Some(TrustBand::UseWithCaution),
            "needs_verification" => Some(TrustBand::NeedsVerification),
            _ => None,
        })
}

fn project_custom_block(
    composition: &Composition,
    block: &Block,
    block_index: u32,
    type_id: &str,
    ctx: &FallbackProjectionContext,
) -> Result<(ProjectedBlock, Vec<AuditIntent>), ProjectionError> {
    let schema = custom_block_schema(type_id);
    let (rule, projected_pointer_count, declared_pointer_count) = schema
        .as_ref()
        .and_then(|schema| {
            select_nearest_known_rule(schema).map(|rule| {
                let projected_count = compatible_schema_pointers(schema, &rule).count() as u32;
                (
                    rule,
                    projected_count,
                    schema.declared_pointers().count() as u32,
                )
            })
        })
        .filter(|(_, projected_count, _)| *projected_count > 0)
        .unwrap_or_else(|| {
            (
                generic_text_rule(),
                0,
                schema
                    .as_ref()
                    .map(|schema| schema.declared_pointers().count() as u32)
                    .unwrap_or_else(|| count_payload_leaves(&block.attributes)),
            )
        });

    validate_field_bindings(block, &rule, true)?;
    let (payload, field_states, mut diagnostics) =
        project_payload_fields(composition, block, &rule, ctx, true);
    let dropped_pointer_count = declared_pointer_count
        .saturating_sub(projected_pointer_count)
        .saturating_add(count_unadmitted_payload_leaves(&block.attributes, &rule));
    diagnostics.push(projection_diagnostic(
        composition,
        DiagnosticKind::BlockFallback,
        Some(block),
        Some(type_id.to_string()),
        Some(rule.block_type.type_id().to_string()),
        DiagnosticReason::UnknownBlockType,
        dropped_pointer_count,
    ));
    let edit_routes = edit_routes_for_block(
        composition,
        block,
        &rule,
        &field_states,
        true,
        ctx,
        &mut diagnostics,
    );
    Ok((
        ProjectedBlock {
            block_id: block.id.clone(),
            block_index,
            original_type_id: type_id.to_string(),
            selected_known_type_id: rule.block_type.type_id().to_string(),
            payload,
            banner: Some(FALLBACK_BANNER_COPY.to_string()),
            trust_band: TrustBand::NeedsVerification,
            claim_refs: block.claim_refs.clone(),
            provenance: vec![block.provenance.clone()],
            edit_routes,
            diagnostics,
        },
        vec![AuditIntent {
            event_kind: "custom_block_fallback_applied",
            category: AuditCategory::DataAccess,
            detail: json!({
                "schema_version": 1,
                "composition_id": composition.id.as_str(),
                "composition_version": composition_version_json(composition.metadata.composition_version),
                "block_id": block.id.as_str(),
                "block_index": block_index,
                "original_type_id": type_id,
                "selected_known_type_id": rule.block_type.type_id(),
                "projected_pointer_count": projected_pointer_count,
                "dropped_pointer_count": dropped_pointer_count,
                "pointer_names_included": ctx.include_non_sensitive_pointer_names,
                "composition_cap_state": "within_cap",
                "block_cap_action": "projected",
                "fallback_policy_version": ctx.fallback_policy_version,
            }),
        }],
    ))
}

fn validate_field_bindings(
    block: &Block,
    rule: &BlockProjectionRule,
    allow_degraded_custom_routes: bool,
) -> Result<(), ProjectionError> {
    for binding in &block.field_bindings {
        if !rule_has_field(rule, binding.field_path.as_str()) && !allow_degraded_custom_routes {
            return Err(ProjectionError::InvalidProducerOutput {
                reason: ProducerOutputInvalidReason::BindingTargetsUnknownField,
            });
        }
        match binding.role {
            BindingRole::Source => {
                for claim_ref in resolve_binding_claim_refs(block, binding) {
                    if claim_ref.field_path.is_none() {
                        return Err(ProjectionError::InvalidProducerOutput {
                            reason: ProducerOutputInvalidReason::SourceBindingMissingFieldPath,
                        });
                    }
                }
            }
            BindingRole::FeedbackTarget => {
                for claim_ref in resolve_binding_claim_refs(block, binding) {
                    if claim_ref.field_path.is_none() {
                        return Err(ProjectionError::InvalidProducerOutput {
                            reason: ProducerOutputInvalidReason::FeedbackTargetMissingFieldPath,
                        });
                    }
                }
            }
            BindingRole::ComputedFrom | BindingRole::DisplayOnly => {}
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FieldProjectionState {
    Rendered,
    SensitivityBlocked,
    OutOfScope,
    NotProjected,
}

fn project_payload_fields(
    composition: &Composition,
    block: &Block,
    rule: &BlockProjectionRule,
    ctx: &FallbackProjectionContext,
    fallback: bool,
) -> (
    Value,
    BTreeMap<String, FieldProjectionState>,
    Vec<ProjectionDiagnostic>,
) {
    let mut payload = Value::Object(Map::new());
    let mut field_states = BTreeMap::new();
    let mut diagnostics = Vec::new();
    for policy in rule.fields {
        let pointer = policy.pointer.to_string();
        if !policy.allowed_surfaces.contains(&ctx.surface) {
            field_states.insert(pointer, FieldProjectionState::SensitivityBlocked);
            diagnostics.push(field_drop_diagnostic(
                composition,
                block,
                rule,
                DiagnosticReason::SensitivityBlocked,
            ));
            continue;
        }
        if !scope_allows_field(block, policy.pointer, ctx) {
            field_states.insert(pointer, FieldProjectionState::OutOfScope);
            diagnostics.push(field_drop_diagnostic(
                composition,
                block,
                rule,
                DiagnosticReason::OutOfScope,
            ));
            continue;
        }
        let mut rendered_any = false;
        for (actual_pointer, raw_value) in values_for_pattern(&block.attributes, policy.pointer) {
            let Some(rendered_value) = render_value(&raw_value, policy, ctx) else {
                diagnostics.push(field_drop_diagnostic(
                    composition,
                    block,
                    rule,
                    DiagnosticReason::SensitivityBlocked,
                ));
                continue;
            };
            insert_json_pointer(&mut payload, &actual_pointer, rendered_value);
            rendered_any = true;
        }
        field_states.insert(
            pointer,
            if rendered_any {
                FieldProjectionState::Rendered
            } else if fallback {
                FieldProjectionState::NotProjected
            } else {
                FieldProjectionState::SensitivityBlocked
            },
        );
    }
    (payload, field_states, diagnostics)
}

fn field_drop_diagnostic(
    composition: &Composition,
    block: &Block,
    rule: &BlockProjectionRule,
    reason: DiagnosticReason,
) -> ProjectionDiagnostic {
    projection_diagnostic(
        composition,
        DiagnosticKind::FieldDropped,
        Some(block),
        Some(block.block_type.type_id().to_string()),
        Some(rule.block_type.type_id().to_string()),
        reason,
        1,
    )
}

fn edit_routes_for_block(
    composition: &Composition,
    block: &Block,
    rule: &BlockProjectionRule,
    field_states: &BTreeMap<String, FieldProjectionState>,
    fallback: bool,
    ctx: &FallbackProjectionContext,
    diagnostics: &mut Vec<ProjectionDiagnostic>,
) -> Vec<EditRoute> {
    let ambiguous_fields = ambiguous_feedback_fields(block);
    block
        .field_bindings
        .iter()
        .map(|binding| {
            let field_path = binding.field_path.as_str().to_string();
            let claim_refs = resolve_binding_claim_refs(block, binding);
            let field_state = field_states
                .get(&field_path)
                .copied()
                .unwrap_or(FieldProjectionState::NotProjected);
            let degraded_unknown_field = fallback && !rule_has_field(rule, &field_path);
            let refusal = route_refusal(
                binding,
                claim_refs.as_slice(),
                field_state,
                degraded_unknown_field,
                ambiguous_fields.contains(&field_path),
                ctx,
            );
            if let Some(reason) = refusal {
                if let Some(diag_reason) = diagnostic_reason_for_refusal(reason) {
                    diagnostics.push(projection_diagnostic(
                        composition,
                        DiagnosticKind::EditRouteRefused,
                        Some(block),
                        Some(block.block_type.type_id().to_string()),
                        Some(rule.block_type.type_id().to_string()),
                        diag_reason,
                        0,
                    ));
                }
            }
            EditRoute {
                field_path: binding.field_path.clone(),
                role: binding.role.clone(),
                claim_refs,
                feedback_allowed: refusal.is_none() && binding.role == BindingRole::FeedbackTarget,
                refusal_reason: refusal,
            }
        })
        .collect()
}

fn route_refusal(
    binding: &FieldBinding,
    claim_refs: &[ClaimRef],
    field_state: FieldProjectionState,
    degraded_unknown_field: bool,
    ambiguous_receiver: bool,
    ctx: &FallbackProjectionContext,
) -> Option<EditRouteRefusalReason> {
    if field_state == FieldProjectionState::OutOfScope {
        return Some(EditRouteRefusalReason::OutOfScope);
    }
    if field_state == FieldProjectionState::SensitivityBlocked {
        return Some(EditRouteRefusalReason::SensitivityBlocked);
    }
    if degraded_unknown_field {
        return Some(EditRouteRefusalReason::FallbackDegradedWithoutReceiver);
    }
    match binding.role {
        BindingRole::Source => Some(EditRouteRefusalReason::SourceWithoutTarget),
        BindingRole::ComputedFrom => Some(EditRouteRefusalReason::Computed),
        BindingRole::DisplayOnly => Some(EditRouteRefusalReason::DisplayOnly),
        BindingRole::FeedbackTarget => {
            if claim_refs.is_empty() {
                Some(EditRouteRefusalReason::MissingClaimRef)
            } else if ambiguous_receiver {
                Some(EditRouteRefusalReason::AmbiguousReceiver)
            } else if ctx.actor.kind() == ActorKind::SurfaceClient
                && !surface_client_can_submit(ctx)
            {
                Some(EditRouteRefusalReason::OutOfScope)
            } else {
                None
            }
        }
    }
}

fn diagnostic_reason_for_refusal(reason: EditRouteRefusalReason) -> Option<DiagnosticReason> {
    match reason {
        EditRouteRefusalReason::MissingClaimRef => Some(DiagnosticReason::MissingClaimRef),
        EditRouteRefusalReason::AmbiguousReceiver => Some(DiagnosticReason::AmbiguousReceiver),
        EditRouteRefusalReason::SensitivityBlocked => Some(DiagnosticReason::SensitivityBlocked),
        EditRouteRefusalReason::UnknownRole => Some(DiagnosticReason::UnknownRole),
        EditRouteRefusalReason::OutOfScope => Some(DiagnosticReason::OutOfScope),
        EditRouteRefusalReason::FallbackDegradedWithoutReceiver => {
            Some(DiagnosticReason::FallbackDegradedWithoutReceiver)
        }
        // Computed/DisplayOnly/SourceWithoutTarget refusals are routing-shape
        // decisions, not §9 diagnostic-reason cases. The closed DiagnosticReason
        // enum has no variant for them, and emitting `MissingClaimRef` here would
        // mislead W4-A/W5-A consumers about why the route was refused. The
        // refusal is surfaced via the EditRoute.refusal_reason field; no
        // ProjectionDiagnostic is published.
        EditRouteRefusalReason::Computed
        | EditRouteRefusalReason::DisplayOnly
        | EditRouteRefusalReason::SourceWithoutTarget => None,
    }
}

fn ambiguous_feedback_fields(block: &Block) -> HashSet<String> {
    let mut receivers: BTreeMap<String, BTreeSet<Vec<String>>> = BTreeMap::new();
    for binding in &block.field_bindings {
        if binding.role != BindingRole::FeedbackTarget {
            continue;
        }
        let normalized: Vec<String> = resolve_binding_claim_refs(block, binding)
            .into_iter()
            .map(|claim_ref| {
                format!(
                    "{}:{}:{}",
                    claim_ref.claim_id,
                    claim_ref.claim_version.unwrap_or(0),
                    claim_ref
                        .field_path
                        .as_ref()
                        .map(FieldPath::as_str)
                        .unwrap_or("")
                )
            })
            .collect();
        receivers
            .entry(binding.field_path.as_str().to_string())
            .or_default()
            .insert(normalized);
    }
    receivers
        .into_iter()
        .filter_map(|(field, sets)| (sets.len() > 1).then_some(field))
        .collect()
}

fn resolve_binding_claim_refs(block: &Block, binding: &FieldBinding) -> Vec<ClaimRef> {
    binding
        .claim_refs
        .iter()
        .filter_map(|ClaimRefIndex(index)| block.claim_refs.get(*index).cloned())
        .collect()
}

fn scope_allows_field(block: &Block, pointer: &str, ctx: &FallbackProjectionContext) -> bool {
    let claim_bound = block
        .field_bindings
        .iter()
        .any(|binding| binding.field_path.as_str() == pointer && !binding.claim_refs.is_empty());
    if !claim_bound || ctx.actor.kind() != ActorKind::SurfaceClient {
        return true;
    }
    surface_client_can_read(ctx)
}

fn surface_client_can_read(ctx: &FallbackProjectionContext) -> bool {
    let Actor::SurfaceClient { scopes, .. } = &ctx.actor else {
        return true;
    };
    scopes.iter().any(|scope| {
        let value = scope.as_str();
        value == "read.composition"
            || value.starts_with("read.")
            || value.starts_with("admin.")
            || value.starts_with("manage.")
    })
}

fn surface_client_can_submit(ctx: &FallbackProjectionContext) -> bool {
    let Actor::SurfaceClient { scopes, .. } = &ctx.actor else {
        return true;
    };
    scopes.contains(&SurfaceScope::new("submit.feedback"))
        || scopes.iter().any(|scope| {
            let value = scope.as_str();
            value.starts_with("admin.") || value.starts_with("manage.")
        })
}

fn render_value(
    value: &Value,
    policy: &FieldPolicy,
    ctx: &FallbackProjectionContext,
) -> Option<Value> {
    let value = value_for_kind(value, policy.value_kind)?;
    let text = value.as_str().unwrap_or_default();
    let claim = synthetic_claim(policy.sensitivity.clone(), text);
    match render_policy_for_surface(&claim, ctx.surface.render_surface(), &render_actor(ctx)) {
        RenderDecision::Render => Some(value),
        RenderDecision::RenderRedacted { affordance } => {
            Some(Value::String(affordance.label().to_string()))
        }
        RenderDecision::Drop => None,
    }
}

fn value_for_kind(value: &Value, kind: ValueKind) -> Option<Value> {
    match (kind, value) {
        (ValueKind::Text, Value::String(_)) => Some(value.clone()),
        (ValueKind::Number, Value::Number(_)) => Some(value.clone()),
        (ValueKind::Object, Value::Object(_)) => Some(value.clone()),
        (ValueKind::Bool, Value::Bool(_)) => Some(value.clone()),
        (ValueKind::Array, Value::Array(_)) => Some(value.clone()),
        _ => None,
    }
}

fn synthetic_claim(sensitivity: ClaimSensitivity, text: &str) -> IntelligenceClaim {
    IntelligenceClaim {
        id: "projection-field".to_string(),
        claim_version: 1,
        subject_ref: "projection".to_string(),
        claim_type: "projection.field".to_string(),
        field_path: None,
        topic_key: None,
        text: text.to_string(),
        dedup_key: "projection-field".to_string(),
        item_hash: None,
        actor: "user".to_string(),
        data_source: "projection".to_string(),
        source_ref: None,
        source_asof: None,
        observed_at: "2026-05-15T00:00:00Z".to_string(),
        created_at: "2026-05-15T00:00:00Z".to_string(),
        provenance_json: "{}".to_string(),
        metadata_json: None,
        claim_state: ClaimState::Active,
        surfacing_state: SurfacingState::Active,
        demotion_reason: None,
        reactivated_at: None,
        retraction_reason: None,
        expires_at: None,
        superseded_by: None,
        trust_score: None,
        trust_computed_at: None,
        trust_version: None,
        thread_id: None,
        temporal_scope: TemporalScope::State,
        sensitivity,
        verification_state: ClaimVerificationState::Active,
        verification_reason: None,
        needs_user_decision_at: None,
    }
}

fn render_actor(ctx: &FallbackProjectionContext) -> RenderActor {
    match &ctx.actor {
        Actor::User => RenderActor::user("user", Some("user")),
        Actor::SurfaceClient { instance, .. } => {
            RenderActor::agent(format!("surface_client:{}", instance.as_str()))
        }
        Actor::Agent => RenderActor::agent("agent"),
        Actor::Admin => RenderActor::agent("admin"),
        Actor::System => RenderActor::agent("system"),
    }
}

fn values_for_pattern(source: &Value, pattern: &str) -> Vec<(String, Value)> {
    let segments: Vec<&str> = pattern
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    let mut out = Vec::new();
    collect_pattern_values(source, &segments, String::new(), &mut out);
    out
}

fn collect_pattern_values(
    value: &Value,
    segments: &[&str],
    current_pointer: String,
    out: &mut Vec<(String, Value)>,
) {
    if segments.is_empty() {
        out.push((current_pointer, value.clone()));
        return;
    }
    let head = segments[0];
    if head == "*" {
        if let Value::Array(items) = value {
            for (index, item) in items.iter().enumerate() {
                collect_pattern_values(
                    item,
                    &segments[1..],
                    format!("{current_pointer}/{index}"),
                    out,
                );
            }
        }
    } else if let Value::Object(map) = value {
        if let Some(child) = map.get(head) {
            collect_pattern_values(
                child,
                &segments[1..],
                format!("{current_pointer}/{head}"),
                out,
            );
        }
    }
}

fn insert_json_pointer(target: &mut Value, pointer: &str, value: Value) {
    let segments: Vec<&str> = pointer
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    insert_segments(target, &segments, value);
}

fn insert_segments(target: &mut Value, segments: &[&str], value: Value) {
    if segments.is_empty() {
        *target = value;
        return;
    }
    let head = segments[0];
    if let Ok(index) = head.parse::<usize>() {
        if !target.is_array() {
            *target = Value::Array(Vec::new());
        }
        let array = target.as_array_mut().expect("array just initialized");
        while array.len() <= index {
            array.push(Value::Null);
        }
        insert_segments(&mut array[index], &segments[1..], value);
    } else {
        if !target.is_object() {
            *target = Value::Object(Map::new());
        }
        let object = target.as_object_mut().expect("object just initialized");
        let child = object.entry(head.to_string()).or_insert(Value::Null);
        insert_segments(child, &segments[1..], value);
    }
}

fn count_unadmitted_payload_leaves(source: &Value, rule: &BlockProjectionRule) -> u32 {
    let admitted: BTreeSet<String> = rule
        .fields
        .iter()
        .map(|field| field.pointer.to_string())
        .collect();
    count_unadmitted_payload_leaves_at(source, String::new(), &admitted)
}

fn count_unadmitted_payload_leaves_at(
    value: &Value,
    pointer: String,
    admitted: &BTreeSet<String>,
) -> u32 {
    match value {
        Value::Object(map) => map
            .iter()
            .map(|(key, child)| {
                count_unadmitted_payload_leaves_at(
                    child,
                    format!("{pointer}/{}", escape_pointer(key)),
                    admitted,
                )
            })
            .sum(),
        Value::Array(items) => items
            .iter()
            .enumerate()
            .map(|(index, child)| {
                count_unadmitted_payload_leaves_at(child, format!("{pointer}/{index}"), admitted)
            })
            .sum(),
        Value::Null => 0,
        _ => {
            if admitted
                .iter()
                .any(|pattern| pattern_matches_actual(pattern, &pointer))
            {
                0
            } else {
                1
            }
        }
    }
}

fn count_payload_leaves(source: &Value) -> u32 {
    match source {
        Value::Object(map) => map.values().map(count_payload_leaves).sum(),
        Value::Array(items) => items.iter().map(count_payload_leaves).sum(),
        Value::Null => 0,
        _ => 1,
    }
}

fn escape_pointer(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}

fn select_nearest_known_rule(schema: &CustomBlockSchema) -> Option<BlockProjectionRule> {
    let mut scored: Vec<(BlockProjectionRule, NearestTypeScore)> = known_projection_rules()
        .into_iter()
        .map(|rule| {
            let score = score_rule(schema, &rule);
            (rule, score)
        })
        .collect();
    scored.sort_by(|(a_rule, a_score), (b_rule, b_score)| {
        b_score
            .total()
            .cmp(&a_score.total())
            .then(b_score.kind_match.cmp(&a_score.kind_match))
            .then(b_score.required_overlap.cmp(&a_score.required_overlap))
            .then(b_score.optional_overlap.cmp(&a_score.optional_overlap))
            .then(
                b_score
                    .annotation_similarity
                    .cmp(&a_score.annotation_similarity),
            )
            .then(a_rule.block_type.type_id().cmp(b_rule.block_type.type_id()))
    });
    let (winner, score) = scored.into_iter().next()?;
    (score.total() > 0).then_some(winner)
}

fn score_rule(schema: &CustomBlockSchema, rule: &BlockProjectionRule) -> NearestTypeScore {
    let mut score = NearestTypeScore::default();
    if let (Some(schema_kind), Some(rule_kind)) = (&schema.composition_kind, rule.composition_kind)
    {
        if schema_kind == rule_kind {
            score.kind_match = 100;
        }
    }
    let rule_fields: BTreeSet<String> = rule
        .fields
        .iter()
        .map(|field| field.pointer.to_string())
        .collect();
    score.required_overlap = schema
        .required_pointers
        .iter()
        .filter(|pointer| rule_fields.contains(*pointer))
        .count() as u32
        * 10;
    score.optional_overlap = schema
        .optional_pointers
        .iter()
        .filter(|pointer| rule_fields.contains(*pointer))
        .count() as u32
        * 2;
    let annotations: BTreeSet<&str> = rule.render_annotations.iter().copied().collect();
    score.annotation_similarity = (schema
        .render_annotations
        .iter()
        .filter(|annotation| annotations.contains(annotation.as_str()))
        .count() as u32
        * 4)
    .min(20);
    if rule
        .type_namespace
        .is_some_and(|namespace| schema.type_id.starts_with(namespace))
    {
        score.namespace_similarity = 5;
    }
    score
}

fn compatible_schema_pointers<'a>(
    schema: &'a CustomBlockSchema,
    rule: &'a BlockProjectionRule,
) -> impl Iterator<Item = &'a String> {
    let rule_fields: BTreeSet<String> = rule
        .fields
        .iter()
        .map(|field| field.pointer.to_string())
        .collect();
    schema
        .declared_pointers()
        .filter(move |pointer| rule_fields.contains(*pointer))
}

fn pattern_matches_actual(pattern: &str, actual: &str) -> bool {
    let pattern_segments: Vec<&str> = pattern
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    let actual_segments: Vec<&str> = actual
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    pattern_segments.len() == actual_segments.len()
        && pattern_segments
            .iter()
            .zip(actual_segments.iter())
            .all(|(pattern, actual)| *pattern == "*" || pattern == actual)
}

fn rule_has_field(rule: &BlockProjectionRule, field_path: &str) -> bool {
    rule.fields
        .iter()
        .any(|field| pattern_matches_actual(field.pointer, field_path))
}

fn projection_diagnostic(
    composition: &Composition,
    diagnostic_kind: DiagnosticKind,
    block: Option<&Block>,
    original_type_id: Option<String>,
    selected_known_type_id: Option<String>,
    reason: DiagnosticReason,
    dropped_pointer_count: u32,
) -> ProjectionDiagnostic {
    ProjectionDiagnostic {
        diagnostic_kind,
        composition_id: CompositionId::new(composition.id.as_str()),
        composition_version: composition.metadata.composition_version.0,
        original_type_id,
        selected_known_type_id,
        block_id: block.map(|block| block.id.clone()),
        reason,
        dropped_pointer_count,
    }
}

fn composition_version_json(version: CompositionVersion) -> Value {
    json!(version.0)
}

fn generic_text_rule() -> BlockProjectionRule {
    BlockProjectionRule {
        block_type: BlockType::Custom {
            type_id: GENERIC_TEXT_TYPE_ID.to_string(),
        },
        composition_kind: None,
        type_namespace: Some("dailyos/"),
        render_annotations: &[],
        fields: &[],
        default_trust_band: TrustBand::NeedsVerification,
    }
}

fn rule_for_block_type(block_type: &BlockType) -> Option<BlockProjectionRule> {
    match block_type {
        BlockType::AccountOverview => Some(account_overview_rule()),
        BlockType::ClaimSummary => Some(claim_summary_rule()),
        BlockType::EvidenceList => Some(evidence_list_rule()),
        BlockType::HealthSnapshot => Some(health_snapshot_rule()),
        BlockType::RelationshipMap => Some(relationship_map_rule()),
        BlockType::RiskCallout => Some(risk_callout_rule()),
        BlockType::ActionList => Some(action_list_rule()),
        BlockType::MarkdownDocument => Some(markdown_document_rule()),
        // v1.4.3 W2 Wave 1 primitive blocks (DOS-682).
        BlockType::Pill => Some(pill_rule()),
        BlockType::StatusDot => Some(status_dot_rule()),
        BlockType::ProvenanceTag => Some(provenance_tag_rule()),
        BlockType::HealthBadge => Some(health_badge_rule()),
        BlockType::Avatar => Some(avatar_rule()),
        BlockType::FreshnessIndicator => Some(freshness_indicator_rule()),
        BlockType::TrustBandBadge => Some(trust_band_badge_rule()),
        BlockType::IntelligenceQualityBadge => Some(intelligence_quality_badge_rule()),
        BlockType::EntityChip => Some(entity_chip_rule()),
        BlockType::TypeBadge => Some(type_badge_rule()),
        BlockType::ScoreBand => Some(score_band_rule()),
        BlockType::Custom { .. } => None,
    }
}

fn known_projection_rules() -> Vec<BlockProjectionRule> {
    vec![
        account_overview_rule(),
        claim_summary_rule(),
        evidence_list_rule(),
        health_snapshot_rule(),
        relationship_map_rule(),
        risk_callout_rule(),
        action_list_rule(),
        markdown_document_rule(),
        // v1.4.3 W2 Wave 1 primitive blocks (DOS-682).
        pill_rule(),
        status_dot_rule(),
        provenance_tag_rule(),
        health_badge_rule(),
        avatar_rule(),
        freshness_indicator_rule(),
        trust_band_badge_rule(),
        intelligence_quality_badge_rule(),
        entity_chip_rule(),
        type_badge_rule(),
        score_band_rule(),
    ]
}

const ALL_SURFACES: &[SurfaceKind] = &[
    SurfaceKind::TauriApp,
    SurfaceKind::McpTool,
    SurfaceKind::McpToolDetail,
    SurfaceKind::SurfaceClient,
    SurfaceKind::Worker,
    SurfaceKind::Eval,
];
const FIRST_PARTY_SURFACES: &[SurfaceKind] = &[SurfaceKind::TauriApp];

const fn text_field(pointer: &'static str, sensitivity: ClaimSensitivity) -> FieldPolicy {
    FieldPolicy {
        pointer,
        sensitivity,
        allowed_surfaces: ALL_SURFACES,
        value_kind: ValueKind::Text,
    }
}

const fn first_party_text_field(
    pointer: &'static str,
    sensitivity: ClaimSensitivity,
) -> FieldPolicy {
    FieldPolicy {
        pointer,
        sensitivity,
        allowed_surfaces: FIRST_PARTY_SURFACES,
        value_kind: ValueKind::Text,
    }
}

const fn number_field(pointer: &'static str, sensitivity: ClaimSensitivity) -> FieldPolicy {
    FieldPolicy {
        pointer,
        sensitivity,
        allowed_surfaces: ALL_SURFACES,
        value_kind: ValueKind::Number,
    }
}

const fn object_field(pointer: &'static str, sensitivity: ClaimSensitivity) -> FieldPolicy {
    FieldPolicy {
        pointer,
        sensitivity,
        allowed_surfaces: ALL_SURFACES,
        value_kind: ValueKind::Object,
    }
}

const fn bool_field(pointer: &'static str, sensitivity: ClaimSensitivity) -> FieldPolicy {
    FieldPolicy {
        pointer,
        sensitivity,
        allowed_surfaces: ALL_SURFACES,
        value_kind: ValueKind::Bool,
    }
}

const fn array_field(pointer: &'static str, sensitivity: ClaimSensitivity) -> FieldPolicy {
    FieldPolicy {
        pointer,
        sensitivity,
        allowed_surfaces: ALL_SURFACES,
        value_kind: ValueKind::Array,
    }
}

const ACCOUNT_OVERVIEW_FIELDS: &[FieldPolicy] = &[
    text_field("/account/display_name", ClaimSensitivity::Internal),
    text_field("/summary", ClaimSensitivity::Internal),
    text_field("/health/band", ClaimSensitivity::Internal),
    number_field("/health/score", ClaimSensitivity::Internal),
    text_field("/risk/title", ClaimSensitivity::Internal),
    text_field("/risk/body", ClaimSensitivity::Internal),
    text_field("/actions/*/title", ClaimSensitivity::Internal),
    text_field("/relationships/*/label", ClaimSensitivity::Internal),
    text_field("/title", ClaimSensitivity::Internal),
    number_field("/claim_count", ClaimSensitivity::Internal),
    object_field("/counts_by_trust_band", ClaimSensitivity::Internal),
    array_field("/context", ClaimSensitivity::Internal),
    text_field("/account_id", ClaimSensitivity::Internal),
];
const CLAIM_SUMMARY_FIELDS: &[FieldPolicy] = &[
    text_field("/title", ClaimSensitivity::Internal),
    text_field("/body", ClaimSensitivity::Internal),
    text_field("/status", ClaimSensitivity::Internal),
    text_field("/source_asof", ClaimSensitivity::Internal),
    text_field("/trust/band", ClaimSensitivity::Internal),
    text_field("/trust/source_label", ClaimSensitivity::Internal),
    text_field("/text", ClaimSensitivity::Internal),
    text_field("/trust_band", ClaimSensitivity::Internal),
    text_field("/claim_id", ClaimSensitivity::Internal),
    text_field("/claim_type", ClaimSensitivity::Internal),
    text_field("/intent", ClaimSensitivity::Internal),
    bool_field("/empty_state", ClaimSensitivity::Internal),
];
const EVIDENCE_LIST_FIELDS: &[FieldPolicy] = &[
    text_field("/items/*/label", ClaimSensitivity::Internal),
    text_field("/items/*/source_label", ClaimSensitivity::Internal),
    text_field("/items/*/source_asof", ClaimSensitivity::Internal),
    first_party_text_field("/items/*/source_excerpt", ClaimSensitivity::Confidential),
];
const HEALTH_SNAPSHOT_FIELDS: &[FieldPolicy] = &[
    text_field("/band", ClaimSensitivity::Internal),
    number_field("/score", ClaimSensitivity::Internal),
    text_field("/rationale", ClaimSensitivity::Internal),
    text_field("/trend", ClaimSensitivity::Internal),
    text_field("/text", ClaimSensitivity::Internal),
    text_field("/trust_band", ClaimSensitivity::Internal),
    text_field("/claim_id", ClaimSensitivity::Internal),
    text_field("/claim_type", ClaimSensitivity::Internal),
    text_field("/source_asof", ClaimSensitivity::Internal),
];
const RELATIONSHIP_MAP_FIELDS: &[FieldPolicy] = &[
    text_field("/nodes/*/label", ClaimSensitivity::Internal),
    text_field("/nodes/*/role", ClaimSensitivity::Internal),
    text_field("/edges/*/label", ClaimSensitivity::Internal),
    text_field("/nodes/*/text", ClaimSensitivity::Internal),
    text_field("/nodes/*/trust_band", ClaimSensitivity::Internal),
    text_field("/nodes/*/claim_id", ClaimSensitivity::Internal),
    text_field("/nodes/*/source_asof", ClaimSensitivity::Internal),
    text_field("/claim_type", ClaimSensitivity::Internal),
];
const RISK_CALLOUT_FIELDS: &[FieldPolicy] = &[
    text_field("/title", ClaimSensitivity::Internal),
    text_field("/body", ClaimSensitivity::Internal),
    text_field("/severity", ClaimSensitivity::Internal),
    text_field("/recommended_action", ClaimSensitivity::Internal),
    text_field("/text", ClaimSensitivity::Internal),
    text_field("/trust_band", ClaimSensitivity::Internal),
    text_field("/claim_id", ClaimSensitivity::Internal),
    text_field("/claim_type", ClaimSensitivity::Internal),
    text_field("/source_asof", ClaimSensitivity::Internal),
];
const ACTION_LIST_FIELDS: &[FieldPolicy] = &[
    text_field("/items/*/title", ClaimSensitivity::Internal),
    text_field("/items/*/status", ClaimSensitivity::Internal),
    text_field("/items/*/due_at", ClaimSensitivity::Internal),
    text_field("/items/*/owner_label", ClaimSensitivity::Internal),
    text_field("/items/*/text", ClaimSensitivity::Internal),
    text_field("/items/*/trust_band", ClaimSensitivity::Internal),
    text_field("/items/*/claim_id", ClaimSensitivity::Internal),
    text_field("/items/*/source_asof", ClaimSensitivity::Internal),
    text_field("/claim_type", ClaimSensitivity::Internal),
];
const MARKDOWN_DOCUMENT_FIELDS: &[FieldPolicy] = &[
    text_field("/title", ClaimSensitivity::Internal),
    text_field("/body", ClaimSensitivity::Internal),
    text_field("/sections/*/heading", ClaimSensitivity::Internal),
    text_field("/sections/*/body", ClaimSensitivity::Internal),
];

// v1.4.3 W2 Wave 1 primitive block field policies (DOS-682). Placeholder
// `/payload/text` entries — per-primitive field structure lands in PR-D2/D3/D4
// when each block's payload contract is finalized.
const PILL_FIELDS: &[FieldPolicy] = &[text_field("/payload/text", ClaimSensitivity::Internal)];
const STATUS_DOT_FIELDS: &[FieldPolicy] = &[text_field("/payload/text", ClaimSensitivity::Internal)];
const PROVENANCE_TAG_FIELDS: &[FieldPolicy] = &[
    text_field("/payload/text", ClaimSensitivity::Internal),
    text_field("/payload/source", ClaimSensitivity::Internal),
    text_field("/payload/dataSource", ClaimSensitivity::Internal),
    text_field("/payload/data_source", ClaimSensitivity::Internal),
    text_field("/payload/itemSource", ClaimSensitivity::Internal),
    text_field("/payload/item_source", ClaimSensitivity::Internal),
    text_field("/payload/sourceLabel", ClaimSensitivity::Internal),
    text_field("/payload/source_label", ClaimSensitivity::Internal),
    text_field("/payload/label", ClaimSensitivity::Internal),
    text_field("/payload/age", ClaimSensitivity::Internal),
    text_field("/payload/asOf", ClaimSensitivity::Internal),
    text_field("/payload/as_of", ClaimSensitivity::Internal),
    text_field("/payload/sourceAsof", ClaimSensitivity::Internal),
    text_field("/payload/source_asof", ClaimSensitivity::Internal),
    text_field("/payload/sourcedAt", ClaimSensitivity::Internal),
    text_field("/payload/sourced_at", ClaimSensitivity::Internal),
    text_field("/payload/observedAt", ClaimSensitivity::Internal),
    text_field("/payload/observed_at", ClaimSensitivity::Internal),
    text_field("/payload/capturedAt", ClaimSensitivity::Internal),
    text_field("/payload/captured_at", ClaimSensitivity::Internal),
    text_field("/payload/variant", ClaimSensitivity::Internal),
    bool_field("/payload/discrepancy", ClaimSensitivity::Internal),
];
const HEALTH_BADGE_FIELDS: &[FieldPolicy] = &[
    number_field("/score", ClaimSensitivity::Internal),
    text_field("/band", ClaimSensitivity::Internal),
    text_field("/trend/direction", ClaimSensitivity::Internal),
    text_field("/trend/rationale", ClaimSensitivity::Internal),
    number_field("/confidence", ClaimSensitivity::Internal),
    bool_field("/sufficientData", ClaimSensitivity::Internal),
    bool_field("/showScore", ClaimSensitivity::Internal),
    text_field("/size", ClaimSensitivity::Internal),
    text_field("/source", ClaimSensitivity::Internal),
    text_field("/divergence/severity", ClaimSensitivity::Internal),
    bool_field("/divergence/leadingIndicator", ClaimSensitivity::Internal),
];
const AVATAR_FIELDS: &[FieldPolicy] = &[
    text_field("/name", ClaimSensitivity::Internal),
    text_field("/personId", ClaimSensitivity::Internal),
    text_field("/photoUrl", ClaimSensitivity::Internal),
    number_field("/size", ClaimSensitivity::Internal),
    text_field("/className", ClaimSensitivity::Internal),
];
const FRESHNESS_INDICATOR_FIELDS: &[FieldPolicy] = &[
    text_field("/at", ClaimSensitivity::Internal),
    text_field("/enrichedAt", ClaimSensitivity::Internal),
    text_field("/format", ClaimSensitivity::Internal),
    text_field("/dateFormat", ClaimSensitivity::Internal),
    number_field("/stalenessThreshold", ClaimSensitivity::Internal),
    text_field("/verb", ClaimSensitivity::Internal),
    array_field("/fragments", ClaimSensitivity::Internal),
    text_field("/variant", ClaimSensitivity::Internal),
    text_field("/className", ClaimSensitivity::Internal),
];
const TRUST_BAND_BADGE_FIELDS: &[FieldPolicy] = &[
    text_field("/band", ClaimSensitivity::Internal),
    bool_field("/compact", ClaimSensitivity::Internal),
    text_field("/label", ClaimSensitivity::Internal),
];
const INTELLIGENCE_QUALITY_BADGE_FIELDS: &[FieldPolicy] = &[
    number_field("/qualityScore", ClaimSensitivity::Internal),
    bool_field("/hasNewSignals", ClaimSensitivity::Internal),
    text_field("/lastEnriched", ClaimSensitivity::Internal),
    text_field("/enrichedAt", ClaimSensitivity::Internal),
    bool_field("/showLabel", ClaimSensitivity::Internal),
    bool_field("/showTooltip", ClaimSensitivity::Internal),
];
const ENTITY_CHIP_FIELDS: &[FieldPolicy] =
    &[text_field("/payload/text", ClaimSensitivity::Internal)];
const TYPE_BADGE_FIELDS: &[FieldPolicy] =
    &[text_field("/payload/text", ClaimSensitivity::Internal)];
const SCORE_BAND_FIELDS: &[FieldPolicy] =
    &[text_field("/payload/text", ClaimSensitivity::Internal)];

fn account_overview_rule() -> BlockProjectionRule {
    BlockProjectionRule {
        block_type: BlockType::AccountOverview,
        composition_kind: Some("entity_page"),
        type_namespace: Some("dailyos/account"),
        render_annotations: &["account", "overview", "summary"],
        fields: ACCOUNT_OVERVIEW_FIELDS,
        default_trust_band: TrustBand::UseWithCaution,
    }
}
fn claim_summary_rule() -> BlockProjectionRule {
    BlockProjectionRule {
        block_type: BlockType::ClaimSummary,
        composition_kind: Some("entity_page"),
        type_namespace: Some("dailyos/claim"),
        render_annotations: &["claim", "summary"],
        fields: CLAIM_SUMMARY_FIELDS,
        default_trust_band: TrustBand::UseWithCaution,
    }
}
fn evidence_list_rule() -> BlockProjectionRule {
    BlockProjectionRule {
        block_type: BlockType::EvidenceList,
        composition_kind: Some("entity_page"),
        type_namespace: Some("dailyos/evidence"),
        render_annotations: &["evidence", "sources"],
        fields: EVIDENCE_LIST_FIELDS,
        default_trust_band: TrustBand::UseWithCaution,
    }
}
fn health_snapshot_rule() -> BlockProjectionRule {
    BlockProjectionRule {
        block_type: BlockType::HealthSnapshot,
        composition_kind: Some("entity_page"),
        type_namespace: Some("dailyos/health"),
        render_annotations: &["health", "score"],
        fields: HEALTH_SNAPSHOT_FIELDS,
        default_trust_band: TrustBand::UseWithCaution,
    }
}
fn relationship_map_rule() -> BlockProjectionRule {
    BlockProjectionRule {
        block_type: BlockType::RelationshipMap,
        composition_kind: Some("entity_page"),
        type_namespace: Some("dailyos/relationship"),
        render_annotations: &["relationship", "map"],
        fields: RELATIONSHIP_MAP_FIELDS,
        default_trust_band: TrustBand::UseWithCaution,
    }
}
fn risk_callout_rule() -> BlockProjectionRule {
    BlockProjectionRule {
        block_type: BlockType::RiskCallout,
        composition_kind: Some("entity_page"),
        type_namespace: Some("dailyos/risk"),
        render_annotations: &["risk", "callout"],
        fields: RISK_CALLOUT_FIELDS,
        default_trust_band: TrustBand::UseWithCaution,
    }
}
fn action_list_rule() -> BlockProjectionRule {
    BlockProjectionRule {
        block_type: BlockType::ActionList,
        composition_kind: Some("entity_page"),
        type_namespace: Some("dailyos/action"),
        render_annotations: &["actions", "list"],
        fields: ACTION_LIST_FIELDS,
        default_trust_band: TrustBand::UseWithCaution,
    }
}
fn markdown_document_rule() -> BlockProjectionRule {
    BlockProjectionRule {
        block_type: BlockType::MarkdownDocument,
        composition_kind: Some("report"),
        type_namespace: Some("dailyos/markdown"),
        render_annotations: &["document", "markdown"],
        fields: MARKDOWN_DOCUMENT_FIELDS,
        default_trust_band: TrustBand::UseWithCaution,
    }
}

// v1.4.3 W2 Wave 1 primitive block projection rules (DOS-682). Each rule is
// the substrate-side stub that pairs the BlockType variant with its
// `dailyos/<kebab>` type_namespace + placeholder field policy. Per-primitive
// field policies + render annotations land in PR-D2/D3/D4 alongside each
// `wp/dailyos/blocks/<slug>/` directory.
fn pill_rule() -> BlockProjectionRule {
    BlockProjectionRule {
        block_type: BlockType::Pill,
        composition_kind: Some("entity_page"),
        type_namespace: Some("dailyos/pill"),
        render_annotations: &["pill"],
        fields: PILL_FIELDS,
        default_trust_band: TrustBand::UseWithCaution,
    }
}
fn status_dot_rule() -> BlockProjectionRule {
    BlockProjectionRule {
        block_type: BlockType::StatusDot,
        composition_kind: Some("entity_page"),
        type_namespace: Some("dailyos/status-dot"),
        render_annotations: &["status-dot"],
        fields: STATUS_DOT_FIELDS,
        default_trust_band: TrustBand::UseWithCaution,
    }
}
fn provenance_tag_rule() -> BlockProjectionRule {
    BlockProjectionRule {
        block_type: BlockType::ProvenanceTag,
        composition_kind: Some("entity_page"),
        type_namespace: Some("dailyos/provenance-tag"),
        render_annotations: &["provenance-tag"],
        fields: PROVENANCE_TAG_FIELDS,
        default_trust_band: TrustBand::UseWithCaution,
    }
}
fn health_badge_rule() -> BlockProjectionRule {
    BlockProjectionRule {
        block_type: BlockType::HealthBadge,
        composition_kind: Some("entity_page"),
        type_namespace: Some("dailyos/health-badge"),
        render_annotations: &["health-badge"],
        fields: HEALTH_BADGE_FIELDS,
        default_trust_band: TrustBand::UseWithCaution,
    }
}
fn avatar_rule() -> BlockProjectionRule {
    BlockProjectionRule {
        block_type: BlockType::Avatar,
        composition_kind: Some("entity_page"),
        type_namespace: Some("dailyos/avatar"),
        render_annotations: &["avatar"],
        fields: AVATAR_FIELDS,
        default_trust_band: TrustBand::UseWithCaution,
    }
}
fn freshness_indicator_rule() -> BlockProjectionRule {
    BlockProjectionRule {
        block_type: BlockType::FreshnessIndicator,
        composition_kind: Some("entity_page"),
        type_namespace: Some("dailyos/freshness-indicator"),
        render_annotations: &["freshness-indicator"],
        fields: FRESHNESS_INDICATOR_FIELDS,
        default_trust_band: TrustBand::UseWithCaution,
    }
}
fn trust_band_badge_rule() -> BlockProjectionRule {
    BlockProjectionRule {
        block_type: BlockType::TrustBandBadge,
        composition_kind: Some("entity_page"),
        type_namespace: Some("dailyos/trust-band-badge"),
        render_annotations: &["trust-band-badge"],
        fields: TRUST_BAND_BADGE_FIELDS,
        default_trust_band: TrustBand::UseWithCaution,
    }
}
fn intelligence_quality_badge_rule() -> BlockProjectionRule {
    BlockProjectionRule {
        block_type: BlockType::IntelligenceQualityBadge,
        composition_kind: Some("entity_page"),
        type_namespace: Some("dailyos/intelligence-quality-badge"),
        render_annotations: &["intelligence-quality-badge"],
        fields: INTELLIGENCE_QUALITY_BADGE_FIELDS,
        default_trust_band: TrustBand::UseWithCaution,
    }
}
fn entity_chip_rule() -> BlockProjectionRule {
    BlockProjectionRule {
        block_type: BlockType::EntityChip,
        composition_kind: Some("entity_page"),
        type_namespace: Some("dailyos/entity-chip"),
        render_annotations: &["entity-chip"],
        fields: ENTITY_CHIP_FIELDS,
        default_trust_band: TrustBand::UseWithCaution,
    }
}
fn type_badge_rule() -> BlockProjectionRule {
    BlockProjectionRule {
        block_type: BlockType::TypeBadge,
        composition_kind: Some("entity_page"),
        type_namespace: Some("dailyos/type-badge"),
        render_annotations: &["type-badge"],
        fields: TYPE_BADGE_FIELDS,
        default_trust_band: TrustBand::UseWithCaution,
    }
}
fn score_band_rule() -> BlockProjectionRule {
    BlockProjectionRule {
        block_type: BlockType::ScoreBand,
        composition_kind: Some("entity_page"),
        type_namespace: Some("dailyos/score-band"),
        render_annotations: &["score-band"],
        fields: SCORE_BAND_FIELDS,
        default_trust_band: TrustBand::UseWithCaution,
    }
}
