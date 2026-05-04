use std::collections::{HashMap, VecDeque};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::abilities::provenance::InvocationId;
use crate::abilities::{AbilityError, Actor, ConfirmationToken};
use crate::services::context::ExecutionMode;

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
