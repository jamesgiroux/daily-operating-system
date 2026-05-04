use std::collections::{HashMap, VecDeque};

use serde::de::Error as _;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::abilities::provenance::InvocationId;
use crate::abilities::{
    AbilityCategory, AbilityContext, AbilityDescriptor, AbilityError, AbilityRegistry, Actor,
    ConfirmationToken,
};
use crate::services::context::{ExecutionMode, ServiceContext};

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

#[derive(Debug, Clone)]
pub struct InvocationContext<'a> {
    pub actor: BridgeActor,
    pub mode: ExecutionMode,
    pub surface: BridgeSurface,
    pub dry_run: bool,
    pub confirmation: Option<&'a ConfirmationToken>,
}

impl<'a> InvocationContext<'a> {
    pub fn registry_actor(&self) -> Actor {
        self.actor.registry_actor()
    }
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AbilityResponseJson {
    pub invocation_id: InvocationId,
    pub ability_name: String,
    pub ability_version: String,
    pub schema_version: u32,
    pub data: serde_json::Value,
    pub rendered_provenance: RenderedProvenance,
    pub diagnostics: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BridgeSurfaceError {
    #[error("ability unavailable")]
    AbilityUnavailable,
}

impl Serialize for ConfirmationToken {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        struct Wire<'a> {
            source: &'a str,
        }

        Wire {
            source: &self.source,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ConfirmationToken {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            source: String,
        }

        let wire = Wire::deserialize(deserializer)?;
        if wire.source.trim().is_empty() {
            return Err(D::Error::custom("confirmation token source is required"));
        }
        Ok(Self {
            source: wire.source,
        })
    }
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

    let ability_context = AbilityContext::new(
        services,
        invocation.registry_actor(),
        invocation.confirmation,
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

    Ok(descriptor)
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
        data,
        rendered_provenance: render_provenance(surface, provenance),
        diagnostics,
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
    RenderedProvenance::new(surface, provenance)
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
