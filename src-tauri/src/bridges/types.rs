use std::collections::{HashMap, VecDeque};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::abilities::provenance::InvocationId;
use crate::abilities::tracer::{AbilityTracer, SpanHandle};
use crate::abilities::{
    validate_schema_closure_for_ability, AbilityCategory, AbilityContext, AbilityDescriptor,
    AbilityError, AbilityRegistry, Actor, ConfirmationProof,
};
use crate::db::ActionDb;
use crate::intelligence::provider::{
    Completion, IntelligenceProvider, ModelName, ModelTier, PromptInput, ProviderError,
    ProviderKind,
};
use crate::services::context::{ExecutionMode, ServiceContext};
use crate::services::sensitivity::{
    render_mcp_ability_data_for_surface_with_provenance,
    render_mcp_ability_data_without_claim_lookup,
};
use crate::state::ContextSnapshot;

pub const BRIDGE_PROVENANCE_DETAIL_BYTE_CAP: usize = 10 * 1024;
pub const BRIDGE_PROVENANCE_CACHE_ENTRY_CAP: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BridgeActor {
    User,
    Agent,
    Admin,
    System,
}

impl BridgeActor {
    pub fn registry_actor(self) -> Actor {
        match self {
            Self::User => Actor::User,
            Self::Agent => Actor::Agent,
            Self::Admin => Actor::Admin,
            Self::System => Actor::System,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BridgeSurface {
    TauriApp,
    McpTool,
    McpToolDetail,
    Worker,
    Eval,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfirmationToken {
    pub actor: BridgeActor,
    pub ability: String,
    pub args_hash: [u8; 32],
    pub issued_at: DateTime<Utc>,
    pub ttl_seconds: u32,
    pub token: String,
}

impl ConfirmationToken {
    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        now.signed_duration_since(self.issued_at).num_seconds() >= self.ttl_seconds as i64
    }

    pub fn matches(&self, actor: &BridgeActor, ability: &str, args_hash: &[u8; 32]) -> bool {
        &self.actor == actor && self.ability == ability && &self.args_hash == args_hash
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AttestationRequestId(uuid::Uuid);

impl AttestationRequestId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl Default for AttestationRequestId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserAttestationRequest {
    pub request_id: AttestationRequestId,
    pub actor: BridgeActor,
    pub ability: String,
    pub args_hash: [u8; 32],
    pub requested_at: DateTime<Utc>,
    pub ttl_seconds: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttestationDecision {
    Approve,
    Reject,
}

#[derive(Debug, Clone)]
pub struct InvocationContext<'a> {
    pub actor: BridgeActor,
    pub mode: ExecutionMode,
    pub surface: BridgeSurface,
    pub dry_run: bool,
    pub confirmation: Option<&'a ConfirmationToken>,
    /// Server-side store of issued confirmation tokens. When present, the
    /// confirmation verifier consumes the token from the store and rejects any
    /// claimed token whose opaque id was never issued (or was already used) —
    /// closes the renderer-can-forge-token gap on Tauri. Other surfaces that
    /// don't issue Tauri-style tokens supply None and rely on their own
    /// attestation flow.
    pub confirmation_store: Option<&'a dyn ConfirmationTokenStore>,
}

impl<'a> InvocationContext<'a> {
    pub fn registry_actor(&self) -> Actor {
        self.actor.registry_actor()
    }
}

/// Server-side state for an issued Tauri confirmation token. The verifier
/// matches the renderer-supplied ConfirmationToken against this record to
/// confirm the token came from a real attestation event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfirmationRecord {
    pub actor: BridgeActor,
    pub ability: String,
    pub args_hash: [u8; 32],
    pub issued_at: DateTime<Utc>,
    pub ttl_seconds: u32,
}

/// Backing store for issued confirmation tokens. `consume` must atomically
/// remove the entry so reuse of a single token across two invocations is not
/// possible.
pub trait ConfirmationTokenStore: Send + Sync + std::fmt::Debug {
    fn consume(&self, opaque_token: &str) -> Option<ConfirmationRecord>;
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenderedProvenance {
    pub surface: BridgeSurface,
    pub value: serde_json::Value,
}

impl RenderedProvenance {
    pub fn new(surface: BridgeSurface, value: serde_json::Value) -> Self {
        Self { surface, value }
    }

    pub fn serialized_len(&self) -> Result<usize, serde_json::Error> {
        serde_json::to_vec(self).map(|bytes| bytes.len())
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct AbilityResponseJson {
    pub invocation_id: InvocationId,
    pub ability_name: String,
    pub ability_version: String,
    pub schema_version: u32,
    pub data: serde_json::Value,
    pub rendered_provenance: RenderedProvenance,
    pub diagnostics: serde_json::Value,
}

impl Serialize for AbilityResponseJson {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let include_diagnostics = !matches!(
            self.rendered_provenance.surface,
            BridgeSurface::McpTool | BridgeSurface::McpToolDetail
        );
        let mut state = serializer.serialize_struct(
            "AbilityResponseJson",
            if include_diagnostics { 7 } else { 6 },
        )?;
        state.serialize_field("invocation_id", &self.invocation_id)?;
        state.serialize_field("ability_name", &self.ability_name)?;
        state.serialize_field("ability_version", &self.ability_version)?;
        state.serialize_field("schema_version", &self.schema_version)?;
        state.serialize_field("data", &self.data)?;
        state.serialize_field("rendered_provenance", &self.rendered_provenance)?;
        if include_diagnostics {
            state.serialize_field("diagnostics", &self.diagnostics)?;
        }
        state.end()
    }
}

#[derive(Debug, Clone, PartialEq, Error, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BridgeSurfaceError {
    #[error("ability unavailable")]
    AbilityUnavailable,
    #[error("ownership validation failed: {0}")]
    Ownership(#[from] crate::abilities::provenance::OwnershipError),
}

#[derive(Debug, Error)]
pub enum AbilityInvokeError {
    #[error(transparent)]
    Surface(#[from] BridgeSurfaceError),
    #[error("ability invocation failed")]
    Ability(AbilityError),
    #[error("ability response was not a valid bridge envelope")]
    InvalidEnvelope,
    #[error("rendered provenance exceeded bridge cache byte cap")]
    ProvenanceTooLarge,
    #[error("rendered provenance serialization failed: {0}")]
    ProvenanceSerialize(#[from] serde_json::Error),
}

impl From<AbilityError> for AbilityInvokeError {
    fn from(error: AbilityError) -> Self {
        Self::Ability(error)
    }
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BridgeRejectReason {
    UnknownAbility,
    ActorPolicy,
    ModePolicy,
    MaintenanceGate,
    ExperimentalGate,
}

#[cfg(test)]
pub(crate) const PRE_DISPATCH_RESOLUTION_ORDER: [BridgeRejectReason; 5] = [
    BridgeRejectReason::UnknownAbility,
    BridgeRejectReason::ActorPolicy,
    BridgeRejectReason::ModePolicy,
    BridgeRejectReason::MaintenanceGate,
    BridgeRejectReason::ExperimentalGate,
];

pub(crate) async fn invoke_registry_json<'a>(
    registry: &AbilityRegistry,
    services: &'a ServiceContext<'a>,
    provider: &'a dyn IntelligenceProvider,
    tracer: &'a dyn AbilityTracer,
    invocation: InvocationContext<'a>,
    ability_name: &str,
    input_json: serde_json::Value,
) -> Result<AbilityResponseJson, AbilityInvokeError> {
    let descriptor = resolve_pre_dispatch(
        registry,
        ability_name,
        invocation.registry_actor(),
        invocation.mode,
        invocation.surface,
    )?;
    let ability_version = descriptor.version.to_string();
    let schema_version = descriptor.schema_version;
    let canonical_ability_name = descriptor.name.to_string();
    let input_schema = (descriptor.input_schema)();

    reject_reserved_input_fields(&input_json)?;
    validate_input_json_against_schema(&input_schema, &input_json)
        .map_err(|_| BridgeSurfaceError::AbilityUnavailable)?;

    let args_hash = confirmation_args_hash(&input_json);

    verify_confirmation_token(
        descriptor,
        &invocation,
        &canonical_ability_name,
        &args_hash,
        tracer,
        services.clock.now(),
    )?;

    let confirmation = invocation
        .confirmation
        .map(|token| token as &dyn ConfirmationProof);
    let ability_context = AbilityContext::new(
        services,
        provider,
        tracer,
        invocation.actor.registry_actor(),
        confirmation,
    );
    let output_json = registry
        .invoke_by_name_json(&ability_context, ability_name, input_json)
        .await?;

    ability_response_from_output_json(
        canonical_ability_name,
        ability_version,
        schema_version,
        invocation.surface,
        output_json,
    )
}

#[derive(Debug, Default)]
pub(crate) struct BridgeNoopIntelligenceProvider;

#[async_trait]
impl IntelligenceProvider for BridgeNoopIntelligenceProvider {
    async fn complete(
        &self,
        _prompt: PromptInput,
        _tier: ModelTier,
    ) -> Result<Completion, ProviderError> {
        Err(ProviderError::Unavailable(
            "no intelligence provider is configured for ability bridge invocation".to_string(),
        ))
    }

    fn provider_kind(&self) -> ProviderKind {
        ProviderKind::Other("bridge_noop")
    }

    fn current_model(&self, _tier: ModelTier) -> ModelName {
        ModelName::new("bridge-noop")
    }
}

pub(crate) static BRIDGE_NOOP_INTELLIGENCE_PROVIDER: BridgeNoopIntelligenceProvider =
    BridgeNoopIntelligenceProvider;

pub(crate) fn provider_from_context_snapshot(
    snapshot: &ContextSnapshot,
) -> &dyn IntelligenceProvider {
    snapshot
        .intelligence_provider
        .as_deref()
        .map(|provider| provider as &dyn IntelligenceProvider)
        .unwrap_or(&BRIDGE_NOOP_INTELLIGENCE_PROVIDER)
}

pub(crate) fn resolve_pre_dispatch<'a>(
    registry: &'a AbilityRegistry,
    ability_name: &str,
    actor: Actor,
    mode: ExecutionMode,
    surface: BridgeSurface,
) -> Result<&'a AbilityDescriptor, BridgeSurfaceError> {
    let descriptor = lookup_descriptor_by_name(registry, ability_name)
        .ok_or(BridgeSurfaceError::AbilityUnavailable)?;

    if !descriptor.policy.allowed_actors.contains(&actor) {
        return Err(BridgeSurfaceError::AbilityUnavailable);
    }

    if !descriptor.policy.allowed_modes.contains(&mode) {
        return Err(BridgeSurfaceError::AbilityUnavailable);
    }

    if maintenance_blocked_for_surface(descriptor, surface) {
        return Err(BridgeSurfaceError::AbilityUnavailable);
    }

    if descriptor.experimental && actor != Actor::System {
        return Err(BridgeSurfaceError::AbilityUnavailable);
    }

    if actor == Actor::Agent && !descriptor.mutates.is_empty() {
        return Err(BridgeSurfaceError::AbilityUnavailable);
    }

    validate_schema_closure_for_ability(descriptor.name, &(descriptor.input_schema)())
        .map_err(|_| BridgeSurfaceError::AbilityUnavailable)?;

    Ok(descriptor)
}

pub fn confirmation_args_hash(value: &serde_json::Value) -> [u8; 32] {
    let mut bytes = Vec::new();
    write_canonical_json(value, &mut bytes);
    Sha256::digest(bytes).into()
}

fn write_canonical_json(value: &serde_json::Value, out: &mut Vec<u8>) {
    match value {
        serde_json::Value::Null
        | serde_json::Value::Bool(_)
        | serde_json::Value::Number(_)
        | serde_json::Value::String(_) => {
            out.extend(
                serde_json::to_vec(value).expect("serializing a serde_json scalar should not fail"),
            );
        }
        serde_json::Value::Array(values) => {
            out.push(b'[');
            for (index, item) in values.iter().enumerate() {
                if index > 0 {
                    out.push(b',');
                }
                write_canonical_json(item, out);
            }
            out.push(b']');
        }
        serde_json::Value::Object(object) => {
            out.push(b'{');
            let mut entries = object.iter().collect::<Vec<_>>();
            entries.sort_by_key(|(key, _)| *key);
            for (index, (key, item)) in entries.into_iter().enumerate() {
                if index > 0 {
                    out.push(b',');
                }
                out.extend(
                    serde_json::to_vec(key)
                        .expect("serializing a serde_json object key should not fail"),
                );
                out.push(b':');
                write_canonical_json(item, out);
            }
            out.push(b'}');
        }
    }
}

fn verify_confirmation_token(
    descriptor: &AbilityDescriptor,
    invocation: &InvocationContext<'_>,
    ability_name: &str,
    args_hash: &[u8; 32],
    tracer: &dyn AbilityTracer,
    now: DateTime<Utc>,
) -> Result<(), BridgeSurfaceError> {
    if !requires_confirmation(descriptor) {
        return Ok(());
    }

    let Some(token) = invocation.confirmation else {
        record_confirmation_token_rejection(tracer, invocation.actor, ability_name, "missing");
        return Err(BridgeSurfaceError::AbilityUnavailable);
    };

    if token.is_expired(now) {
        record_confirmation_token_rejection(tracer, invocation.actor, ability_name, "expired");
        return Err(BridgeSurfaceError::AbilityUnavailable);
    }

    if token.ability.as_str() != ability_name {
        record_confirmation_token_rejection(
            tracer,
            invocation.actor,
            ability_name,
            "unknown_ability",
        );
        return Err(BridgeSurfaceError::AbilityUnavailable);
    }

    if token.actor != invocation.actor || token.args_hash != *args_hash {
        record_confirmation_token_rejection(
            tracer,
            invocation.actor,
            ability_name,
            "args_mismatch",
        );
        return Err(BridgeSurfaceError::AbilityUnavailable);
    }

    // The opaque token id MUST come from a server-side issuance event. Without
    // this lookup the renderer can mint forged tokens by recomputing the
    // deterministic args_hash and supplying any actor/ability/issued_at/ttl.
    if let Some(store) = invocation.confirmation_store {
        let record = match store.consume(&token.token) {
            Some(record) => record,
            None => {
                record_confirmation_token_rejection(
                    tracer,
                    invocation.actor,
                    ability_name,
                    "unknown_or_consumed_token",
                );
                return Err(BridgeSurfaceError::AbilityUnavailable);
            }
        };
        if record.actor != invocation.actor
            || record.ability != ability_name
            || record.args_hash != *args_hash
            || record.issued_at != token.issued_at
            || record.ttl_seconds != token.ttl_seconds
        {
            record_confirmation_token_rejection(
                tracer,
                invocation.actor,
                ability_name,
                "stored_record_mismatch",
            );
            return Err(BridgeSurfaceError::AbilityUnavailable);
        }
    }

    Ok(())
}

fn record_confirmation_token_rejection(
    tracer: &dyn AbilityTracer,
    actor: BridgeActor,
    ability_name: &str,
    reason: &'static str,
) {
    let actor = bridge_actor_label(actor);
    log::debug!(
        target: "dailyos_lib::bridges::confirmation",
        "confirmation token rejected actor={} ability_name={} reason={}",
        actor,
        ability_name,
        reason
    );
    tracer.record_event(
        &SpanHandle::noop(),
        "confirmation_token_rejected",
        serde_json::json!({
            "actor": actor,
            "ability_name": ability_name,
            "reason": reason,
        }),
    );
}

fn bridge_actor_label(actor: BridgeActor) -> &'static str {
    match actor {
        BridgeActor::User => "user",
        BridgeActor::Agent => "agent",
        BridgeActor::Admin => "admin",
        BridgeActor::System => "system",
    }
}

fn requires_confirmation(descriptor: &AbilityDescriptor) -> bool {
    descriptor.policy.requires_confirmation || descriptor.category == AbilityCategory::Publish
}

fn lookup_descriptor_by_name<'a>(
    registry: &'a AbilityRegistry,
    ability_name: &str,
) -> Option<&'a AbilityDescriptor> {
    [Actor::User, Actor::Agent, Actor::Admin, Actor::System]
        .into_iter()
        .find_map(|actor| {
            registry
                .iter_for(actor)
                .find(|descriptor| descriptor.name == ability_name)
        })
}

fn maintenance_blocked_for_surface(descriptor: &AbilityDescriptor, surface: BridgeSurface) -> bool {
    descriptor.category == AbilityCategory::Maintenance
        && matches!(
            surface,
            BridgeSurface::TauriApp | BridgeSurface::McpTool | BridgeSurface::McpToolDetail
        )
}

fn ability_response_from_output_json(
    ability_name: String,
    ability_version: String,
    schema_version: u32,
    surface: BridgeSurface,
    output_json: serde_json::Value,
) -> Result<AbilityResponseJson, AbilityInvokeError> {
    let output = output_json
        .as_object()
        .ok_or(AbilityInvokeError::InvalidEnvelope)?;
    let data = output
        .get("data")
        .cloned()
        .ok_or(AbilityInvokeError::InvalidEnvelope)?;
    let provenance = output
        .get("provenance")
        .cloned()
        .ok_or(AbilityInvokeError::InvalidEnvelope)?;
    let diagnostics = output
        .get("diagnostics")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({ "warnings": [] }));
    let invocation_id = parse_invocation_id(&provenance)?;

    Ok(AbilityResponseJson {
        invocation_id,
        ability_name,
        ability_version,
        schema_version,
        data: render_ability_data(surface, data, &provenance),
        rendered_provenance: render_provenance(surface, provenance),
        diagnostics: render_diagnostics(surface, diagnostics),
    })
}

fn parse_invocation_id(provenance: &serde_json::Value) -> Result<InvocationId, AbilityInvokeError> {
    let invocation_id = provenance
        .get("invocation_id")
        .and_then(serde_json::Value::as_str)
        .ok_or(AbilityInvokeError::InvalidEnvelope)?;
    InvocationId::parse(invocation_id).map_err(|_| AbilityInvokeError::InvalidEnvelope)
}

fn render_provenance(surface: BridgeSurface, provenance: serde_json::Value) -> RenderedProvenance {
    let value = match surface {
        BridgeSurface::McpTool | BridgeSurface::McpToolDetail => redact_mcp_provenance(provenance),
        BridgeSurface::TauriApp | BridgeSurface::Worker | BridgeSurface::Eval => provenance,
    };
    RenderedProvenance::new(surface, value)
}

fn render_ability_data(
    surface: BridgeSurface,
    data: serde_json::Value,
    provenance: &serde_json::Value,
) -> serde_json::Value {
    match surface {
        BridgeSurface::McpTool | BridgeSurface::McpToolDetail => {
            render_mcp_ability_data_with_authoritative_claims(data, provenance)
        }
        BridgeSurface::TauriApp | BridgeSurface::Worker | BridgeSurface::Eval => data,
    }
}

fn render_mcp_ability_data_with_authoritative_claims(
    data: serde_json::Value,
    provenance: &serde_json::Value,
) -> serde_json::Value {
    match ActionDb::open_readonly() {
        Ok(db) => render_mcp_ability_data_for_surface_with_provenance(&db, data, provenance),
        Err(error) => {
            log::warn!(
                target: "dailyos_lib::bridges::mcp_ability_data",
                "MCP ability data claim lookup unavailable; tagged claim text will be dropped: {error}"
            );
            render_mcp_ability_data_without_claim_lookup(data)
        }
    }
}

fn render_diagnostics(surface: BridgeSurface, diagnostics: serde_json::Value) -> serde_json::Value {
    match surface {
        BridgeSurface::McpTool | BridgeSurface::McpToolDetail => {
            serde_json::json!({ "warnings": [] })
        }
        BridgeSurface::TauriApp | BridgeSurface::Worker | BridgeSurface::Eval => diagnostics,
    }
}

pub(crate) fn surface_error(error: AbilityInvokeError) -> BridgeSurfaceError {
    match error {
        AbilityInvokeError::Surface(error) => error,
        AbilityInvokeError::Ability(_)
        | AbilityInvokeError::InvalidEnvelope
        | AbilityInvokeError::ProvenanceTooLarge
        | AbilityInvokeError::ProvenanceSerialize(_) => BridgeSurfaceError::AbilityUnavailable,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct InputSchemaValidationError;

fn reject_reserved_input_fields(input: &serde_json::Value) -> Result<(), BridgeSurfaceError> {
    let Some(object) = input.as_object() else {
        return Ok(());
    };

    for reserved in ["actor", "bridge_actor", "confirmation"] {
        if object.contains_key(reserved) {
            return Err(BridgeSurfaceError::AbilityUnavailable);
        }
    }

    Ok(())
}

fn validate_input_json_against_schema(
    schema: &serde_json::Value,
    input: &serde_json::Value,
) -> Result<(), InputSchemaValidationError> {
    validate_schema_keywords(schema, input)
}

fn validate_schema_keywords(
    schema: &serde_json::Value,
    input: &serde_json::Value,
) -> Result<(), InputSchemaValidationError> {
    let Some(schema_object) = schema.as_object() else {
        return Ok(());
    };

    if let Some(enum_values) = schema_object
        .get("enum")
        .and_then(serde_json::Value::as_array)
    {
        if !enum_values.iter().any(|candidate| candidate == input) {
            return Err(InputSchemaValidationError);
        }
    }

    if let Some(const_value) = schema_object.get("const") {
        if const_value != input {
            return Err(InputSchemaValidationError);
        }
    }

    if let Some(all_of) = schema_object
        .get("allOf")
        .and_then(serde_json::Value::as_array)
    {
        for child in all_of {
            validate_schema_keywords(child, input)?;
        }
    }

    if let Some(any_of) = schema_object
        .get("anyOf")
        .and_then(serde_json::Value::as_array)
    {
        if !any_of
            .iter()
            .any(|child| validate_schema_keywords(child, input).is_ok())
        {
            return Err(InputSchemaValidationError);
        }
    }

    if let Some(one_of) = schema_object
        .get("oneOf")
        .and_then(serde_json::Value::as_array)
    {
        let matches = one_of
            .iter()
            .filter(|child| validate_schema_keywords(child, input).is_ok())
            .count();
        if matches != 1 {
            return Err(InputSchemaValidationError);
        }
    }

    if let Some(schema_types) = schema_object.get("type") {
        if !schema_type_matches(schema_types, input) {
            return Err(InputSchemaValidationError);
        }
    }

    if schema_is_object_like(schema_object) {
        validate_object_schema(schema_object, input)?;
    }

    if schema_type_contains(schema_object.get("type"), "array") {
        validate_array_schema(schema_object, input)?;
    }

    Ok(())
}

fn validate_object_schema(
    schema_object: &serde_json::Map<String, serde_json::Value>,
    input: &serde_json::Value,
) -> Result<(), InputSchemaValidationError> {
    let input_object = input.as_object().ok_or(InputSchemaValidationError)?;
    let properties = schema_object
        .get("properties")
        .and_then(serde_json::Value::as_object);

    if let Some(required) = schema_object
        .get("required")
        .and_then(serde_json::Value::as_array)
    {
        for required_field in required {
            let Some(required_name) = required_field.as_str() else {
                continue;
            };
            if !input_object.contains_key(required_name) {
                return Err(InputSchemaValidationError);
            }
        }
    }

    if schema_object.get("additionalProperties") == Some(&serde_json::Value::Bool(false)) {
        for key in input_object.keys() {
            if properties.is_none_or(|properties| !properties.contains_key(key)) {
                return Err(InputSchemaValidationError);
            }
        }
    }

    if let Some(properties) = properties {
        for (key, property_schema) in properties {
            if let Some(value) = input_object.get(key) {
                validate_schema_keywords(property_schema, value)?;
            }
        }
    }

    Ok(())
}

fn validate_array_schema(
    schema_object: &serde_json::Map<String, serde_json::Value>,
    input: &serde_json::Value,
) -> Result<(), InputSchemaValidationError> {
    let input_array = input.as_array().ok_or(InputSchemaValidationError)?;
    let Some(items) = schema_object.get("items") else {
        return Ok(());
    };

    match items {
        serde_json::Value::Array(item_schemas) => {
            if input_array.len() > item_schemas.len() {
                return Err(InputSchemaValidationError);
            }
            for (value, item_schema) in input_array.iter().zip(item_schemas) {
                validate_schema_keywords(item_schema, value)?;
            }
        }
        item_schema => {
            for value in input_array {
                validate_schema_keywords(item_schema, value)?;
            }
        }
    }

    Ok(())
}

fn schema_is_object_like(schema_object: &serde_json::Map<String, serde_json::Value>) -> bool {
    schema_type_contains(schema_object.get("type"), "object")
        || schema_object.contains_key("properties")
        || schema_object.contains_key("additionalProperties")
}

fn schema_type_contains(schema_type: Option<&serde_json::Value>, expected: &str) -> bool {
    match schema_type {
        Some(serde_json::Value::String(value)) => value == expected,
        Some(serde_json::Value::Array(values)) => {
            values.iter().any(|value| value.as_str() == Some(expected))
        }
        _ => false,
    }
}

fn schema_type_matches(schema_type: &serde_json::Value, input: &serde_json::Value) -> bool {
    match schema_type {
        serde_json::Value::String(value) => single_schema_type_matches(value, input),
        serde_json::Value::Array(values) => values.iter().any(|value| {
            value
                .as_str()
                .is_some_and(|schema_type| single_schema_type_matches(schema_type, input))
        }),
        _ => true,
    }
}

fn single_schema_type_matches(schema_type: &str, input: &serde_json::Value) -> bool {
    match schema_type {
        "null" => input.is_null(),
        "boolean" => input.is_boolean(),
        "object" => input.is_object(),
        "array" => input.is_array(),
        "number" => input.is_number(),
        "integer" => input.as_i64().is_some() || input.as_u64().is_some(),
        "string" => input.is_string(),
        _ => true,
    }
}

fn redact_mcp_provenance(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(object) => serde_json::Value::Object(
            object
                .into_iter()
                .filter_map(|(key, value)| {
                    if should_redact_mcp_provenance_key(&key) {
                        None
                    } else {
                        Some((key, redact_mcp_provenance(value)))
                    }
                })
                .collect(),
        ),
        serde_json::Value::Array(values) => {
            serde_json::Value::Array(values.into_iter().map(redact_mcp_provenance).collect())
        }
        scalar => scalar,
    }
}

fn should_redact_mcp_provenance_key(key: &str) -> bool {
    matches!(
        key,
        "children"
            | "child_spans"
            | "completion"
            | "completions"
            | "internal_id"
            | "internal_ids"
            | "prompt"
            | "prompts"
            | "prompt_hash"
            | "raw_completion"
            | "raw_prompt"
            | "seed"
    ) || key.ends_with("_internal_id")
        || key.ends_with("_internal_ids")
        || key.ends_with("_prompt_hash")
        || key.ends_with("_seed")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct McpSessionId(uuid::Uuid);

impl McpSessionId {
    pub fn new_process_scoped() -> Self {
        Self(uuid::Uuid::new_v4())
    }

    pub fn from_uuid(value: uuid::Uuid) -> Self {
        Self(value)
    }

    pub fn as_uuid(self) -> uuid::Uuid {
        self.0
    }
}

type ProvenanceCacheKey = (McpSessionId, InvocationId);

#[derive(Debug, Clone)]
struct CachedProvenance {
    value: RenderedProvenance,
    serialized_len: usize,
}

#[derive(Debug, Clone)]
pub struct InvocationProvenanceCache {
    entries: HashMap<ProvenanceCacheKey, CachedProvenance>,
    order: VecDeque<ProvenanceCacheKey>,
    max_entries: usize,
    max_serialized_bytes: usize,
    current_serialized_bytes: usize,
}

impl Default for InvocationProvenanceCache {
    fn default() -> Self {
        Self::bounded(
            BRIDGE_PROVENANCE_CACHE_ENTRY_CAP,
            BRIDGE_PROVENANCE_DETAIL_BYTE_CAP,
        )
    }
}

impl InvocationProvenanceCache {
    pub fn bounded(max_entries: usize, max_serialized_bytes: usize) -> Self {
        Self {
            entries: HashMap::new(),
            order: VecDeque::new(),
            max_entries,
            max_serialized_bytes,
            current_serialized_bytes: 0,
        }
    }

    pub fn insert(
        &mut self,
        session_id: McpSessionId,
        invocation_id: InvocationId,
        value: RenderedProvenance,
    ) -> Result<(), AbilityInvokeError> {
        let serialized_len = value.serialized_len()?;
        if serialized_len > self.max_serialized_bytes {
            return Err(AbilityInvokeError::ProvenanceTooLarge);
        }

        let key = (session_id, invocation_id);
        self.remove(&key);

        self.current_serialized_bytes += serialized_len;
        self.order.push_back(key);
        self.entries.insert(
            key,
            CachedProvenance {
                value,
                serialized_len,
            },
        );
        self.enforce_bounds();
        Ok(())
    }

    pub fn get(
        &self,
        session_id: McpSessionId,
        invocation_id: InvocationId,
    ) -> Option<&RenderedProvenance> {
        self.entries
            .get(&(session_id, invocation_id))
            .map(|entry| &entry.value)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
        self.current_serialized_bytes = 0;
    }

    fn enforce_bounds(&mut self) {
        while self.entries.len() > self.max_entries
            || self.current_serialized_bytes > self.max_serialized_bytes
        {
            let Some(key) = self.order.pop_front() else {
                break;
            };
            if let Some(entry) = self.entries.remove(&key) {
                self.current_serialized_bytes = self
                    .current_serialized_bytes
                    .saturating_sub(entry.serialized_len);
            }
        }
    }

    fn remove(&mut self, key: &ProvenanceCacheKey) {
        if let Some(entry) = self.entries.remove(key) {
            self.current_serialized_bytes = self
                .current_serialized_bytes
                .saturating_sub(entry.serialized_len);
            self.order.retain(|candidate| candidate != key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::registry::{AbilityPolicy, SignalPolicy};
    use std::pin::Pin;
    use std::sync::Mutex;

    fn ok_erased<'a>(
        _ctx: &'a AbilityContext<'a>,
        input: serde_json::Value,
    ) -> Pin<
        Box<dyn std::future::Future<Output = Result<serde_json::Value, AbilityError>> + Send + 'a>,
    > {
        Box::pin(async move { Ok(input) })
    }

    fn closed_object_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "additionalProperties": false
        })
    }

    fn confirmation_descriptor() -> AbilityDescriptor {
        AbilityDescriptor {
            name: "confirmation_fixture",
            version: "0.1.0",
            schema_version: 1,
            category: AbilityCategory::Transform,
            policy: AbilityPolicy {
                allowed_actors: &[Actor::User],
                allowed_modes: &[ExecutionMode::Live],
                requires_confirmation: true,
                may_publish: false,
            },
            composes: &[],
            mutates: &[],
            experimental: false,
            registered_at: None,
            signal_policy: SignalPolicy::default(),
            invoke_erased: ok_erased,
            input_schema: closed_object_schema,
            output_schema: closed_object_schema,
        }
    }

    #[derive(Default)]
    struct RecordingTracer {
        events: Mutex<Vec<(String, serde_json::Value)>>,
    }

    impl RecordingTracer {
        fn rejection_reasons(&self) -> Vec<String> {
            self.events
                .lock()
                .unwrap()
                .iter()
                .filter(|(name, _)| name == "confirmation_token_rejected")
                .map(|(_, fields)| {
                    fields
                        .get("reason")
                        .and_then(serde_json::Value::as_str)
                        .expect("reason field")
                        .to_string()
                })
                .collect()
        }
    }

    impl AbilityTracer for RecordingTracer {
        fn start_span(&self, _name: &str) -> SpanHandle {
            SpanHandle { id: 1 }
        }

        fn record_event(&self, _span: &SpanHandle, name: &str, fields: serde_json::Value) {
            self.events.lock().unwrap().push((name.to_string(), fields));
        }
    }

    fn invocation<'a>(confirmation: Option<&'a ConfirmationToken>) -> InvocationContext<'a> {
        InvocationContext {
            actor: BridgeActor::User,
            mode: ExecutionMode::Live,
            surface: BridgeSurface::TauriApp,
            dry_run: false,
            confirmation,
            confirmation_store: None,
        }
    }

    fn confirmation_token(
        issued_at: DateTime<Utc>,
        ability: &str,
        args_hash: [u8; 32],
    ) -> ConfirmationToken {
        ConfirmationToken {
            actor: BridgeActor::User,
            ability: ability.to_string(),
            args_hash,
            issued_at,
            ttl_seconds: 60,
            token: "opaque-test-token".to_string(),
        }
    }

    #[test]
    fn confirmation_token_rejection_logs_branch_internally_without_byte_equal_loss() {
        let descriptor = confirmation_descriptor();
        let now = DateTime::parse_from_rfc3339("2026-05-05T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let args_hash = confirmation_args_hash(&serde_json::json!({"ok": true}));
        let different_args_hash = confirmation_args_hash(&serde_json::json!({"ok": false}));
        let expired_token = confirmation_token(
            now - chrono::Duration::seconds(60),
            "confirmation_fixture",
            args_hash,
        );
        let wrong_ability_token = confirmation_token(now, "other_ability", args_hash);
        let wrong_args_token = confirmation_token(now, "confirmation_fixture", different_args_hash);
        let tracer = RecordingTracer::default();
        let expected_error_bytes =
            serde_json::to_vec(&BridgeSurfaceError::AbilityUnavailable).unwrap();

        for (context, expected_reason) in [
            (invocation(None), "missing"),
            (invocation(Some(&expired_token)), "expired"),
            (invocation(Some(&wrong_ability_token)), "unknown_ability"),
            (invocation(Some(&wrong_args_token)), "args_mismatch"),
        ] {
            let error = verify_confirmation_token(
                &descriptor,
                &context,
                "confirmation_fixture",
                &args_hash,
                &tracer,
                now,
            )
            .expect_err("rejection should preserve external surface error");

            assert_eq!(serde_json::to_vec(&error).unwrap(), expected_error_bytes);
            assert_eq!(error, BridgeSurfaceError::AbilityUnavailable);
            assert!(tracer
                .rejection_reasons()
                .iter()
                .any(|reason| reason == expected_reason));
        }

        let events = tracer.events.lock().unwrap();
        assert_eq!(events.len(), 4);
        for (_, fields) in events.iter() {
            assert_eq!(
                fields.get("actor").and_then(serde_json::Value::as_str),
                Some("user")
            );
            assert_eq!(
                fields
                    .get("ability_name")
                    .and_then(serde_json::Value::as_str),
                Some("confirmation_fixture")
            );
            let rendered = fields.to_string();
            assert!(!rendered.contains("args_hash"));
            assert!(!rendered.contains("opaque-test-token"));
        }
    }
}
