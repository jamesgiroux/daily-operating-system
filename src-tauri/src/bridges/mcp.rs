use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use crate::abilities::provenance::InvocationId;
use crate::abilities::{AbilityDescriptor, AbilityRegistry, Actor, ConfirmationToken};
use crate::bridges::types::{invoke_registry_json, surface_error};
use crate::bridges::{
    AbilityResponseJson, BridgeActor, BridgeSurface, BridgeSurfaceError, InvocationContext,
    McpSessionId, RenderedProvenance,
};
use crate::services::context::{
    ExecutionMode, ExternalClients, ServiceContext, SystemClock, SystemRng,
};

const MCP_INVOCATION_CACHE_ENTRY_CAP: usize = 256;
const MCP_ACTOR_LABEL: &str = concat!("agent:dailyos-mcp:", env!("CARGO_PKG_VERSION"));

type InvocationCacheKey = (McpSessionId, InvocationId);

pub struct McpAbilityBridge<'registry> {
    registry: &'registry AbilityRegistry,
    /// Filtered descriptor cache built once at startup from
    /// registry.iter_for(Actor::Agent). call_tool re-fetches policy by name
    /// before invocation; no cached-state escalation. The cache is just to
    /// avoid scanning the full registry on every list_tools call.
    actor_filtered_descriptors: Vec<&'registry AbilityDescriptor>,
    /// (McpSessionId, InvocationId) -> RenderedProvenance, set on success.
    /// Cleared on server restart. No process-global lookup.
    invocation_cache: Arc<Mutex<HashMap<InvocationCacheKey, RenderedProvenance>>>,
    invocation_order: Arc<Mutex<VecDeque<InvocationCacheKey>>>,
}

impl<'registry> McpAbilityBridge<'registry> {
    pub fn new(registry: &'registry AbilityRegistry) -> Self {
        let actor_filtered_descriptors = registry
            .iter_for(Actor::Agent)
            .filter(|descriptor| {
                descriptor
                    .policy
                    .allowed_modes
                    .contains(&ExecutionMode::Live)
            })
            .collect();

        Self {
            registry,
            actor_filtered_descriptors,
            invocation_cache: Arc::new(Mutex::new(HashMap::new())),
            invocation_order: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    /// Returns the cached actor-filtered descriptor list. The bridge does NOT
    /// re-scan the registry on every call; it scans once at construction.
    pub fn list_descriptors(&self) -> &[&'registry AbilityDescriptor] {
        &self.actor_filtered_descriptors
    }

    /// Invoke an ability by name. Re-fetches policy by name at call time
    /// (does not trust cached descriptors for authorization). Maps every
    /// reject reason to BridgeSurfaceError::AbilityUnavailable byte-equal.
    pub async fn invoke_ability(
        &self,
        session: McpSessionId,
        ability_name: &str,
        input_json: serde_json::Value,
        dry_run: bool,
        confirmation: Option<ConfirmationToken>,
    ) -> Result<AbilityResponseJson, BridgeSurfaceError> {
        let clock = SystemClock;
        let rng = SystemRng;
        let external = ExternalClients::default();
        let services =
            ServiceContext::new_live(&clock, &rng, &external).with_actor(MCP_ACTOR_LABEL);
        let invocation = InvocationContext {
            actor: BridgeActor::Agent,
            mode: ExecutionMode::Live,
            surface: BridgeSurface::McpTool,
            dry_run,
            confirmation: confirmation.as_ref(),
        };

        let response = invoke_registry_json(
            self.registry,
            &services,
            invocation,
            ability_name,
            input_json,
        )
        .await
        .map_err(surface_error)?;

        self.insert_provenance(
            session,
            response.invocation_id,
            response.rendered_provenance.clone(),
        );

        Ok(response)
    }

    /// Bounded session-scoped lookup. Returns RenderedProvenance only if the
    /// (session, invocation_id) pair was previously returned by this same
    /// session. No maintenance-audit fallback; no cross-session lookup.
    pub fn get_provenance(
        &self,
        session: McpSessionId,
        invocation_id: InvocationId,
    ) -> Option<RenderedProvenance> {
        self.invocation_cache
            .lock()
            .expect("mcp invocation cache poisoned")
            .get(&(session, invocation_id))
            .cloned()
    }

    fn insert_provenance(
        &self,
        session: McpSessionId,
        invocation_id: InvocationId,
        provenance: RenderedProvenance,
    ) {
        let key = (session, invocation_id);
        let mut cache = self
            .invocation_cache
            .lock()
            .expect("mcp invocation cache poisoned");
        let mut order = self
            .invocation_order
            .lock()
            .expect("mcp invocation cache order poisoned");

        if cache.insert(key, provenance).is_some() {
            order.retain(|candidate| candidate != &key);
        }
        order.push_back(key);

        while cache.len() > MCP_INVOCATION_CACHE_ENTRY_CAP {
            let Some(evicted) = order.pop_front() else {
                break;
            };
            cache.remove(&evicted);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::pin::Pin;

    use serde_json::json;

    use super::*;
    use crate::abilities::registry::{AbilityPolicy, SignalPolicy};
    use crate::abilities::{AbilityCategory, AbilityContext, AbilityError};

    const AGENT_ACTORS: &[Actor] = &[Actor::Agent];
    const USER_ACTORS: &[Actor] = &[Actor::User];
    const ADMIN_ACTORS: &[Actor] = &[Actor::Admin];
    const AGENT_SYSTEM_ACTORS: &[Actor] = &[Actor::Agent, Actor::System];
    const LIVE_MODES: &[ExecutionMode] = &[ExecutionMode::Live];
    const EVALUATE_MODES: &[ExecutionMode] = &[ExecutionMode::Evaluate];

    type ErasedFuture<'a> =
        Pin<Box<dyn Future<Output = Result<serde_json::Value, AbilityError>> + Send + 'a>>;

    fn success_erased<'a>(
        ctx: &'a AbilityContext<'a>,
        input: serde_json::Value,
    ) -> ErasedFuture<'a> {
        Box::pin(async move {
            Ok(envelope_json(
                ctx,
                json!({
                    "input": input,
                    "actor": format!("{:?}", ctx.actor),
                    "mode": ctx.mode().as_str(),
                }),
            ))
        })
    }

    fn envelope_json(ctx: &AbilityContext<'_>, data: serde_json::Value) -> serde_json::Value {
        let invocation_id = data
            .get("invocation_id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa");

        json!({
            "data": data,
            "ability_version": { "major": 1, "minor": 0 },
            "diagnostics": { "warnings": [] },
            "provenance": {
                "invocation_id": invocation_id,
                "ability_name": "fixture",
                "ability_version": { "major": 1, "minor": 0 },
                "ability_schema_version": 1,
                "actor": format!("{:?}", ctx.actor),
                "mode": ctx.mode().as_str(),
                "warnings": []
            }
        })
    }

    fn descriptor(
        name: &'static str,
        category: AbilityCategory,
        actors: &'static [Actor],
        modes: &'static [ExecutionMode],
    ) -> AbilityDescriptor {
        AbilityDescriptor {
            name,
            version: "1.0.0",
            schema_version: 1,
            category,
            policy: AbilityPolicy {
                allowed_actors: actors,
                allowed_modes: modes,
                requires_confirmation: false,
                may_publish: false,
            },
            composes: &[],
            mutates: &[],
            experimental: false,
            registered_at: None,
            signal_policy: SignalPolicy {
                emits_on_output_change: &[],
                coalesce: false,
            },
            invoke_erased: success_erased,
            input_schema: closed_object_schema,
            output_schema: closed_object_schema,
        }
    }

    #[cfg(feature = "experimental")]
    fn experimental_descriptor(
        mut descriptor: AbilityDescriptor,
        registered_at: &'static str,
    ) -> AbilityDescriptor {
        descriptor.experimental = true;
        descriptor.registered_at = Some(registered_at);
        descriptor
    }

    fn registry_with_abilities(descriptors: Vec<AbilityDescriptor>) -> AbilityRegistry {
        AbilityRegistry::from_descriptors_checked(descriptors).unwrap()
    }

    fn closed_object_schema() -> serde_json::Value {
        json!({
            "type": "object",
            "additionalProperties": false
        })
    }

    fn descriptor_names(bridge: &McpAbilityBridge<'_>) -> Vec<&'static str> {
        bridge
            .list_descriptors()
            .iter()
            .map(|descriptor| descriptor.name)
            .collect()
    }

    fn session(index: u128) -> McpSessionId {
        McpSessionId::from_uuid(uuid::Uuid::from_u128(index))
    }

    fn invocation(index: u128) -> InvocationId {
        InvocationId::new(uuid::Uuid::from_u128(index))
    }

    fn rendered(index: u128) -> RenderedProvenance {
        RenderedProvenance::new(
            BridgeSurface::McpTool,
            json!({
                "invocation_id": uuid::Uuid::from_u128(index).to_string(),
                "index": index.to_string()
            }),
        )
    }

    fn stale_cache_bridge<'registry>(
        registry: &'registry AbilityRegistry,
        cached_descriptors: Vec<&'registry AbilityDescriptor>,
    ) -> McpAbilityBridge<'registry> {
        McpAbilityBridge {
            registry,
            actor_filtered_descriptors: cached_descriptors,
            invocation_cache: Arc::new(Mutex::new(HashMap::new())),
            invocation_order: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    async fn error_bytes_for(registry: AbilityRegistry, ability_name: &'static str) -> Vec<u8> {
        let bridge = McpAbilityBridge::new(&registry);
        let err = bridge
            .invoke_ability(session(1), ability_name, json!({}), false, None)
            .await
            .unwrap_err();
        serde_json::to_vec(&err).unwrap()
    }

    #[test]
    fn mcp_bridge_list_descriptors_filters_to_agent_actor() {
        let registry = registry_with_abilities(vec![
            descriptor(
                "agent_read",
                AbilityCategory::Read,
                AGENT_ACTORS,
                LIVE_MODES,
            ),
            descriptor("user_read", AbilityCategory::Read, USER_ACTORS, LIVE_MODES),
        ]);
        let bridge = McpAbilityBridge::new(&registry);

        let names = descriptor_names(&bridge);

        assert_eq!(names, vec!["agent_read"]);
    }

    #[test]
    fn mcp_bridge_list_descriptors_does_not_include_maintenance_admin_or_experimental_or_mode_hidden(
    ) {
        let descriptors = vec![
            descriptor(
                "agent_read",
                AbilityCategory::Read,
                AGENT_ACTORS,
                LIVE_MODES,
            ),
            descriptor(
                "agent_maintenance",
                AbilityCategory::Maintenance,
                AGENT_SYSTEM_ACTORS,
                LIVE_MODES,
            ),
            descriptor(
                "admin_read",
                AbilityCategory::Read,
                ADMIN_ACTORS,
                LIVE_MODES,
            ),
            descriptor(
                "evaluate_only",
                AbilityCategory::Read,
                AGENT_ACTORS,
                EVALUATE_MODES,
            ),
        ];
        #[cfg(feature = "experimental")]
        let descriptors = {
            let mut descriptors = descriptors;
            descriptors.push(experimental_descriptor(
                descriptor(
                    "experimental_read",
                    AbilityCategory::Read,
                    AGENT_ACTORS,
                    LIVE_MODES,
                ),
                "2999-01-01T00:00:00Z",
            ));
            descriptors
        };

        let registry = registry_with_abilities(descriptors);
        let bridge = McpAbilityBridge::new(&registry);
        let names = descriptor_names(&bridge);

        assert_eq!(names, vec!["agent_read"]);
        assert!(!names.contains(&"agent_maintenance"));
        assert!(!names.contains(&"admin_read"));
        assert!(!names.contains(&"experimental_read"));
        assert!(!names.contains(&"evaluate_only"));
    }

    #[tokio::test]
    async fn mcp_bridge_invoke_unknown_ability_returns_byte_equal_unavailable() {
        let bytes = error_bytes_for(registry_with_abilities(vec![]), "unknown").await;

        assert_eq!(bytes, br#""ability_unavailable""#);
    }

    #[tokio::test]
    async fn mcp_bridge_invoke_unauthorized_actor_returns_byte_equal_unavailable_matching_unknown()
    {
        let unknown = error_bytes_for(registry_with_abilities(vec![]), "unknown").await;
        let unauthorized = error_bytes_for(
            registry_with_abilities(vec![descriptor(
                "user_only",
                AbilityCategory::Read,
                USER_ACTORS,
                LIVE_MODES,
            )]),
            "user_only",
        )
        .await;

        assert_eq!(unauthorized, unknown);
    }

    #[tokio::test]
    async fn mcp_bridge_invoke_call_tool_rechecks_actor_policy_for_guessed_name() {
        let registry = registry_with_abilities(vec![descriptor(
            "stale_agent_cached",
            AbilityCategory::Read,
            USER_ACTORS,
            LIVE_MODES,
        )]);
        let cached_descriptor = Box::leak(Box::new(descriptor(
            "stale_agent_cached",
            AbilityCategory::Read,
            AGENT_ACTORS,
            LIVE_MODES,
        )));
        let bridge = stale_cache_bridge(&registry, vec![cached_descriptor]);

        assert_eq!(descriptor_names(&bridge), vec!["stale_agent_cached"]);
        let err = bridge
            .invoke_ability(session(1), "stale_agent_cached", json!({}), false, None)
            .await
            .unwrap_err();

        assert_eq!(err, BridgeSurfaceError::AbilityUnavailable);
    }

    #[test]
    fn mcp_bridge_get_provenance_returns_none_for_unknown_session_or_invocation_id() {
        let registry = registry_with_abilities(vec![]);
        let bridge = McpAbilityBridge::new(&registry);

        bridge.insert_provenance(session(1), invocation(1), rendered(1));

        assert_eq!(bridge.get_provenance(session(2), invocation(1)), None);
        assert_eq!(bridge.get_provenance(session(1), invocation(2)), None);
    }

    #[test]
    fn mcp_bridge_get_provenance_rejects_cross_session_invocation_id() {
        let registry = registry_with_abilities(vec![]);
        let bridge = McpAbilityBridge::new(&registry);

        bridge.insert_provenance(session(1), invocation(1), rendered(1));

        assert_eq!(bridge.get_provenance(session(2), invocation(1)), None);
        assert_eq!(
            bridge.get_provenance(session(1), invocation(1)),
            Some(rendered(1))
        );
    }

    #[test]
    fn mcp_bridge_invocation_cache_evicts_at_count_cap_newest_first() {
        let registry = registry_with_abilities(vec![]);
        let bridge = McpAbilityBridge::new(&registry);
        let session = session(1);

        for index in 0..(MCP_INVOCATION_CACHE_ENTRY_CAP + 1) {
            let stable_index = index as u128 + 1;
            bridge.insert_provenance(session, invocation(stable_index), rendered(stable_index));
        }

        assert_eq!(
            bridge
                .invocation_cache
                .lock()
                .expect("mcp invocation cache poisoned")
                .len(),
            MCP_INVOCATION_CACHE_ENTRY_CAP
        );
        assert_eq!(bridge.get_provenance(session, invocation(1)), None);
        assert_eq!(
            bridge.get_provenance(
                session,
                invocation((MCP_INVOCATION_CACHE_ENTRY_CAP + 1) as u128)
            ),
            Some(rendered((MCP_INVOCATION_CACHE_ENTRY_CAP + 1) as u128))
        );
    }
}
