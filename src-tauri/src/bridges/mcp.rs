use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use crate::abilities::provenance::InvocationId;
use crate::abilities::{AbilityTracer, NoopAbilityTracer};
use crate::abilities::{AbilityDescriptor, AbilityRegistry, Actor};
use crate::bridges::tauri::TauriAbilityBridge;
use crate::bridges::types::{
    confirmation_args_hash, invoke_registry_json, surface_error, BridgeNoopIntelligenceProvider,
};
use crate::bridges::{
    AbilityResponseJson, BridgeActor, BridgeSurface, BridgeSurfaceError, ConfirmationToken,
    InvocationContext, McpSessionId, RenderedProvenance,
};
use crate::services::context::{
    ExecutionMode, ExternalClients, ServiceContext, SystemClock, SystemRng,
};
use crate::intelligence::provider::IntelligenceProvider;
use rmcp::model::{CallToolResult, Content};
use rmcp::Error as McpError;

// MCP invocation provenance cache caps. The cache keeps the newest 256 entries,
// limits total serialized detail provenance to 256 KiB, and refuses individual
// detail entries above ADR-0108's 10 KiB MCP tool-response budget.
const MCP_INVOCATION_CACHE_ENTRY_CAP: usize = 256;
const MCP_INVOCATION_CACHE_TOTAL_BYTE_CAP: usize = 256 * 1024;
const MCP_INVOCATION_CACHE_ENTRY_BYTE_CAP: usize = 10 * 1024;
const MCP_ACTOR_LABEL: &str = concat!("agent:dailyos-mcp:", env!("CARGO_PKG_VERSION"));
const MCP_CONFIRMATION_TOKEN_TTL_SECONDS: u32 = 5 * 60;

type InvocationCacheKey = (McpSessionId, InvocationId);
type ConfirmationTokenCacheKey = (McpSessionId, String, [u8; 32]);

#[derive(Debug, Clone)]
struct CachedInvocationProvenance {
    value: RenderedProvenance,
    serialized_len: usize,
}

#[derive(Debug, Default)]
struct McpInvocationCache {
    entries: HashMap<InvocationCacheKey, CachedInvocationProvenance>,
    order: VecDeque<InvocationCacheKey>,
    current_serialized_bytes: usize,
}

impl McpInvocationCache {
    fn insert(&mut self, key: InvocationCacheKey, value: RenderedProvenance) {
        let Ok(serialized_len) = value.serialized_len() else {
            return;
        };

        self.remove(&key);

        if serialized_len > MCP_INVOCATION_CACHE_ENTRY_BYTE_CAP {
            return;
        }

        self.current_serialized_bytes += serialized_len;
        self.order.push_back(key);
        self.entries.insert(
            key,
            CachedInvocationProvenance {
                value,
                serialized_len,
            },
        );
        self.enforce_bounds();
    }

    fn get(&self, key: &InvocationCacheKey) -> Option<RenderedProvenance> {
        self.entries.get(key).map(|entry| entry.value.clone())
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.entries.len()
    }

    #[cfg(test)]
    fn current_serialized_bytes(&self) -> usize {
        self.current_serialized_bytes
    }

    fn enforce_bounds(&mut self) {
        while self.entries.len() > MCP_INVOCATION_CACHE_ENTRY_CAP
            || self.current_serialized_bytes > MCP_INVOCATION_CACHE_TOTAL_BYTE_CAP
        {
            let Some(evicted) = self.order.pop_front() else {
                break;
            };
            self.remove_without_order_retain(&evicted);
        }
    }

    fn remove(&mut self, key: &InvocationCacheKey) {
        if self.remove_without_order_retain(key) {
            self.order.retain(|candidate| candidate != key);
        }
    }

    fn remove_without_order_retain(&mut self, key: &InvocationCacheKey) -> bool {
        let Some(entry) = self.entries.remove(key) else {
            return false;
        };
        self.current_serialized_bytes = self
            .current_serialized_bytes
            .saturating_sub(entry.serialized_len);
        true
    }
}

#[derive(Debug, Default)]
struct McpConfirmationTokenCache {
    entries: HashMap<ConfirmationTokenCacheKey, ConfirmationToken>,
}

impl McpConfirmationTokenCache {
    fn insert(
        &mut self,
        session: McpSessionId,
        ability: String,
        args_hash: [u8; 32],
        token: ConfirmationToken,
    ) {
        self.entries.insert((session, ability, args_hash), token);
    }

    fn take(
        &mut self,
        session: McpSessionId,
        ability: &str,
        args_hash: [u8; 32],
    ) -> Option<ConfirmationToken> {
        self.entries
            .remove(&(session, ability.to_string(), args_hash))
    }
}

pub struct McpAbilityBridge<'registry> {
    registry: &'registry AbilityRegistry,
    provider: Arc<dyn IntelligenceProvider + Send + Sync>,
    tracer: Arc<dyn AbilityTracer>,
    /// Filtered descriptor cache built once at startup from
    /// registry.iter_for(Actor::Agent). call_tool re-fetches policy by name
    /// before invocation; no cached-state escalation. The cache is just to
    /// avoid scanning the full registry on every list_tools call.
    actor_filtered_descriptors: Vec<&'registry AbilityDescriptor>,
    /// (McpSessionId, InvocationId) -> RenderedProvenance, set on success.
    /// Cleared on server restart. No process-global lookup.
    invocation_cache: Arc<Mutex<McpInvocationCache>>,
    /// (McpSessionId, ability, args_hash) -> ConfirmationToken, consumed by
    /// the next matching call_tool invocation. Token bytes never need to be
    /// supplied through MCP ability tool args.
    confirmation_tokens: Arc<Mutex<McpConfirmationTokenCache>>,
}

impl<'registry> McpAbilityBridge<'registry> {
    pub fn new(registry: &'registry AbilityRegistry) -> Self {
        Self::new_with_provider_and_tracer(
            registry,
            Arc::new(BridgeNoopIntelligenceProvider),
            Arc::new(NoopAbilityTracer),
        )
    }

    pub fn new_with_provider_and_tracer(
        registry: &'registry AbilityRegistry,
        provider: Arc<dyn IntelligenceProvider + Send + Sync>,
        tracer: Arc<dyn AbilityTracer>,
    ) -> Self {
        let actor_filtered_descriptors = registry
            .iter_for(Actor::Agent)
            .filter(|descriptor| {
                descriptor
                    .policy
                    .allowed_modes
                    .contains(&ExecutionMode::Live)
                    && descriptor.mutates.is_empty()
            })
            .collect();

        Self {
            registry,
            provider,
            tracer,
            actor_filtered_descriptors,
            invocation_cache: Arc::new(Mutex::new(McpInvocationCache::default())),
            confirmation_tokens: Arc::new(Mutex::new(McpConfirmationTokenCache::default())),
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
            self.provider.as_ref(),
            self.tracer.as_ref(),
            invocation,
            ability_name,
            input_json,
        )
        .await
        .map_err(surface_error)?;

        self.insert_provenance(
            session,
            response.invocation_id,
            RenderedProvenance::new(
                BridgeSurface::McpToolDetail,
                response.rendered_provenance.value.clone(),
            ),
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
    }

    pub fn get_provenance_tool_response(
        &self,
        session: McpSessionId,
        invocation_id: InvocationId,
    ) -> CallToolResult {
        match self.get_provenance(session, invocation_id) {
            Some(provenance) => {
                let detail =
                    RenderedProvenance::new(BridgeSurface::McpToolDetail, provenance.value);
                CallToolResult::success(vec![json_content(detail)])
            }
            None => CallToolResult::error(vec![json_content(
                BridgeSurfaceError::AbilityUnavailable,
            )]),
        }
    }

    pub async fn request_confirmation_tool(
        &self,
        session: McpSessionId,
        ability: &str,
        input_json: &serde_json::Value,
        tauri_bridge: &TauriAbilityBridge<'_>,
    ) -> Result<CallToolResult, McpError> {
        let args_hash = confirmation_args_hash(input_json);
        let user_attestation = tauri_bridge.user_attestation_request(
            BridgeActor::Agent,
            ability.to_string(),
            args_hash,
            MCP_CONFIRMATION_TOKEN_TTL_SECONDS,
        );
        let token = tauri_bridge
            .issue_confirmation_token(
                BridgeActor::Agent,
                ability.to_string(),
                args_hash,
                user_attestation,
            )
            .await
            .map_err(mcp_error_from_bridge_surface_error)?;

        self.insert_confirmation_token(session, ability.to_string(), args_hash, token.clone());

        Ok(CallToolResult::success(vec![json_content(token)]))
    }

    pub fn take_confirmation_token(
        &self,
        session: McpSessionId,
        ability: &str,
        input_json: &serde_json::Value,
    ) -> Option<ConfirmationToken> {
        self.confirmation_tokens
            .lock()
            .expect("mcp confirmation token cache poisoned")
            .take(session, ability, confirmation_args_hash(input_json))
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
        cache.insert(key, provenance);
    }

    fn insert_confirmation_token(
        &self,
        session: McpSessionId,
        ability: String,
        args_hash: [u8; 32],
        token: ConfirmationToken,
    ) {
        self.confirmation_tokens
            .lock()
            .expect("mcp confirmation token cache poisoned")
            .insert(session, ability, args_hash, token);
    }
}

fn json_content<T: serde::Serialize>(value: T) -> Content {
    match Content::json(value) {
        Ok(content) => content,
        Err(_) => Content::text("\"ability_unavailable\""),
    }
}

fn mcp_error_from_bridge_surface_error(error: BridgeSurfaceError) -> McpError {
    let data = serde_json::to_value(error)
        .unwrap_or_else(|_| serde_json::Value::String("ability_unavailable".to_string()));
    McpError::invalid_params(error.to_string(), Some(data))
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::Arc;

    use chrono::Utc;
    use serde_json::json;

    use super::*;
    use crate::abilities::registry::{AbilityPolicy, SignalPolicy};
    use crate::abilities::{AbilityCategory, AbilityContext, AbilityError};
    use crate::bridges::tauri::UserAttestationHost;
    use crate::bridges::UserAttestationRequest;

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

    fn internal_provenance_erased<'a>(
        ctx: &'a AbilityContext<'a>,
        input: serde_json::Value,
    ) -> ErasedFuture<'a> {
        Box::pin(async move {
            let mut envelope = envelope_json(
                ctx,
                json!({
                    "input": input,
                    "actor": format!("{:?}", ctx.actor),
                    "mode": ctx.mode().as_str(),
                }),
            );
            envelope["provenance"]["internal_id"] = json!("internal-account-217");
            envelope["provenance"]["prompt_hash"] = json!("prompt-hash-217");
            envelope["provenance"]["seed"] = json!(217);
            envelope["provenance"]["children"] = json!([{ "internal_id": "child-217" }]);
            Ok(envelope)
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

    fn confirmation_descriptor(mut descriptor: AbilityDescriptor) -> AbilityDescriptor {
        descriptor.policy.requires_confirmation = true;
        descriptor
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
            "additionalProperties": false,
            "properties": {
                "subject": { "type": "string" },
                "value": {}
            }
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

    fn rendered_with_payload(index: u128, payload_len: usize) -> RenderedProvenance {
        RenderedProvenance::new(
            BridgeSurface::McpToolDetail,
            json!({
                "invocation_id": uuid::Uuid::from_u128(index).to_string(),
                "payload": "x".repeat(payload_len)
            }),
        )
    }

    fn token_for(
        actor: BridgeActor,
        ability: &str,
        input: &serde_json::Value,
        issued_at: chrono::DateTime<Utc>,
        ttl_seconds: u32,
    ) -> ConfirmationToken {
        ConfirmationToken {
            actor,
            ability: ability.to_string(),
            args_hash: confirmation_args_hash(input),
            issued_at,
            ttl_seconds,
            token: "fixture-token".to_string(),
        }
    }

    fn with_invoke_erased(
        mut descriptor: AbilityDescriptor,
        invoke_erased: for<'a> fn(&'a AbilityContext<'a>, serde_json::Value) -> ErasedFuture<'a>,
    ) -> AbilityDescriptor {
        descriptor.invoke_erased = invoke_erased;
        descriptor
    }

    fn with_mutates(
        mut descriptor: AbilityDescriptor,
        mutates: &'static [&'static str],
    ) -> AbilityDescriptor {
        descriptor.mutates = mutates;
        descriptor
    }

    #[derive(Default)]
    struct ApprovingAttestationHost;

    impl UserAttestationHost for ApprovingAttestationHost {
        fn request_user_attestation<'a>(
            &'a self,
            _request: UserAttestationRequest,
        ) -> Pin<Box<dyn Future<Output = Result<(), BridgeSurfaceError>> + Send + 'a>> {
            Box::pin(async { Ok(()) })
        }
    }

    fn tool_result_json(result: &CallToolResult) -> serde_json::Value {
        let text = result.content[0].as_text().unwrap().text.as_str();
        serde_json::from_str(text).unwrap()
    }

    fn stale_cache_bridge<'registry>(
        registry: &'registry AbilityRegistry,
        cached_descriptors: Vec<&'registry AbilityDescriptor>,
    ) -> McpAbilityBridge<'registry> {
        McpAbilityBridge {
            registry,
            provider: Arc::new(BridgeNoopIntelligenceProvider),
            tracer: Arc::new(NoopAbilityTracer),
            actor_filtered_descriptors: cached_descriptors,
            invocation_cache: Arc::new(Mutex::new(McpInvocationCache::default())),
            confirmation_tokens: Arc::new(Mutex::new(McpConfirmationTokenCache::default())),
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

    async fn invoke_error_bytes(
        bridge: &McpAbilityBridge<'_>,
        ability_name: &str,
        input_json: serde_json::Value,
        confirmation: Option<ConfirmationToken>,
    ) -> Vec<u8> {
        let err = bridge
            .invoke_ability(session(1), ability_name, input_json, false, confirmation)
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
    fn mcp_list_tools_derives_from_registry_iter_for_agent() {
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

        assert_eq!(descriptor_names(&bridge), vec!["agent_read"]);
    }

    #[test]
    fn mcp_list_tools_filters_agent_actor() {
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
        assert!(names.contains(&"agent_read"));
        assert!(!names.contains(&"user_read"));
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
    async fn mcp_hidden_ability_error_bytes_match_unknown_ability() {
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
        let maintenance = error_bytes_for(
            registry_with_abilities(vec![descriptor(
                "agent_maintenance",
                AbilityCategory::Maintenance,
                AGENT_SYSTEM_ACTORS,
                LIVE_MODES,
            )]),
            "agent_maintenance",
        )
        .await;
        let mode_hidden = error_bytes_for(
            registry_with_abilities(vec![descriptor(
                "evaluate_only",
                AbilityCategory::Read,
                AGENT_ACTORS,
                EVALUATE_MODES,
            )]),
            "evaluate_only",
        )
        .await;

        assert_eq!(unauthorized, unknown);
        assert_eq!(maintenance, unknown);
        assert_eq!(mode_hidden, unknown);
        assert_eq!(unknown, br#""ability_unavailable""#);
    }

    #[tokio::test]
    async fn mcp_maintenance_synthetic_actor_rejected_requires_user_actor() {
        let registry = registry_with_abilities(vec![descriptor(
            "synthetic_maintenance",
            AbilityCategory::Maintenance,
            AGENT_SYSTEM_ACTORS,
            LIVE_MODES,
        )]);
        let bridge = McpAbilityBridge::new(&registry);

        assert!(!descriptor_names(&bridge).contains(&"synthetic_maintenance"));
        let err = bridge
            .invoke_ability(session(1), "synthetic_maintenance", json!({}), false, None)
            .await
            .unwrap_err();

        assert_eq!(err, BridgeSurfaceError::AbilityUnavailable);
    }

    #[tokio::test]
    async fn agent_entity_mutation_blocked_until_dos_379() {
        let registry = registry_with_abilities(vec![with_mutates(
            descriptor(
                "agent_entity_mutation",
                AbilityCategory::Publish,
                AGENT_ACTORS,
                LIVE_MODES,
            ),
            &["entity_members"],
        )]);
        let bridge = McpAbilityBridge::new(&registry);

        assert!(!descriptor_names(&bridge).contains(&"agent_entity_mutation"));
        let err = bridge
            .invoke_ability(session(1), "agent_entity_mutation", json!({}), false, None)
            .await
            .unwrap_err();

        assert_eq!(err, BridgeSurfaceError::AbilityUnavailable);
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

    #[tokio::test]
    async fn mcp_request_confirmation_tool_returns_token_via_tauri_bridge() {
        let registry = registry_with_abilities(vec![descriptor(
            "agent_write",
            AbilityCategory::Read,
            AGENT_ACTORS,
            LIVE_MODES,
        )]);
        let mcp_bridge = McpAbilityBridge::new(&registry);
        let tauri_bridge = TauriAbilityBridge::new_with_attestation_host(
            &registry,
            Arc::new(ApprovingAttestationHost),
        );
        let input = json!({ "subject": "dailyos" });

        let result = mcp_bridge
            .request_confirmation_tool(session(1), "agent_write", &input, &tauri_bridge)
            .await
            .unwrap();

        assert_eq!(result.is_error, Some(false));
        let token: ConfirmationToken = serde_json::from_value(tool_result_json(&result)).unwrap();
        assert_eq!(token.actor, BridgeActor::Agent);
        assert_eq!(token.ability, "agent_write");
        assert_eq!(token.args_hash, confirmation_args_hash(&input));
        assert!(!token.token.is_empty());
    }

    #[tokio::test]
    async fn mcp_request_confirmation_tool_args_hash_mismatch_yields_byte_equal_unavailable_on_later_call_tool(
    ) {
        let registry = registry_with_abilities(vec![confirmation_descriptor(descriptor(
            "agent_confirmed",
            AbilityCategory::Read,
            AGENT_ACTORS,
            LIVE_MODES,
        ))]);
        let mcp_bridge = McpAbilityBridge::new(&registry);
        let tauri_bridge = TauriAbilityBridge::new_with_attestation_host(
            &registry,
            Arc::new(ApprovingAttestationHost),
        );
        let issued_for = json!({ "subject": "x" });
        let later_call = json!({ "subject": "y" });

        let request_result = mcp_bridge
            .request_confirmation_tool(session(1), "agent_confirmed", &issued_for, &tauri_bridge)
            .await
            .unwrap();
        let token: ConfirmationToken =
            serde_json::from_value(tool_result_json(&request_result)).unwrap();

        let unknown = invoke_error_bytes(&mcp_bridge, "unknown", json!({}), None).await;
        let mismatch = invoke_error_bytes(
            &mcp_bridge,
            "agent_confirmed",
            later_call,
            Some(token),
        )
        .await;
        let missing =
            invoke_error_bytes(&mcp_bridge, "agent_confirmed", issued_for.clone(), None).await;
        let expired_token = token_for(
            BridgeActor::Agent,
            "agent_confirmed",
            &issued_for,
            Utc::now() - chrono::Duration::seconds(301),
            300,
        );
        let expired = invoke_error_bytes(
            &mcp_bridge,
            "agent_confirmed",
            issued_for,
            Some(expired_token),
        )
        .await;

        assert_eq!(mismatch, unknown);
        assert_eq!(missing, unknown);
        assert_eq!(expired, unknown);
        assert_eq!(unknown, br#""ability_unavailable""#);
    }

    #[tokio::test]
    async fn bridge_invoke_with_missing_confirmation_token_returns_byte_equal_unavailable() {
        let registry = registry_with_abilities(vec![confirmation_descriptor(descriptor(
            "agent_confirmed",
            AbilityCategory::Read,
            AGENT_ACTORS,
            LIVE_MODES,
        ))]);
        let bridge = McpAbilityBridge::new(&registry);

        let unknown = invoke_error_bytes(&bridge, "unknown", json!({}), None).await;
        let missing =
            invoke_error_bytes(&bridge, "agent_confirmed", json!({}), None).await;

        assert_eq!(missing, unknown);
    }

    #[tokio::test]
    async fn bridge_invoke_with_expired_confirmation_token_returns_byte_equal_unavailable() {
        let registry = registry_with_abilities(vec![confirmation_descriptor(descriptor(
            "agent_confirmed",
            AbilityCategory::Read,
            AGENT_ACTORS,
            LIVE_MODES,
        ))]);
        let bridge = McpAbilityBridge::new(&registry);
        let input = json!({});
        let expired_token = token_for(
            BridgeActor::Agent,
            "agent_confirmed",
            &input,
            Utc::now() - chrono::Duration::seconds(301),
            300,
        );

        let unknown = invoke_error_bytes(&bridge, "unknown", json!({}), None).await;
        let expired =
            invoke_error_bytes(&bridge, "agent_confirmed", input, Some(expired_token)).await;

        assert_eq!(expired, unknown);
    }

    #[tokio::test]
    async fn mcp_bridge_invoke_ability_populates_invocation_provenance_cache_on_success() {
        let registry = registry_with_abilities(vec![descriptor(
            "agent_read",
            AbilityCategory::Read,
            AGENT_ACTORS,
            LIVE_MODES,
        )]);
        let bridge = McpAbilityBridge::new(&registry);
        let session = session(1);

        let response = bridge
            .invoke_ability(session, "agent_read", json!({}), false, None)
            .await
            .unwrap();

        assert_eq!(response.rendered_provenance.surface, BridgeSurface::McpTool);
        let cached = bridge
            .get_provenance(session, response.invocation_id)
            .expect("successful MCP invoke should cache provenance");
        assert_eq!(cached.surface, BridgeSurface::McpToolDetail);
        assert_eq!(cached.value, response.rendered_provenance.value);
    }

    #[tokio::test]
    async fn mcp_response_includes_actor_filtered_rendered_provenance() {
        let registry = registry_with_abilities(vec![descriptor(
            "agent_read",
            AbilityCategory::Read,
            AGENT_ACTORS,
            LIVE_MODES,
        )]);
        let bridge = McpAbilityBridge::new(&registry);

        let response = bridge
            .invoke_ability(session(1), "agent_read", json!({}), false, None)
            .await
            .unwrap();

        assert_eq!(response.rendered_provenance.surface, BridgeSurface::McpTool);
        assert_eq!(response.rendered_provenance.value["actor"], "Agent");
        assert_eq!(response.rendered_provenance.value["mode"], "live");
    }

    #[test]
    fn mcp_session_id_is_process_scoped_and_cleared_on_restart() {
        let first_process_session = McpSessionId::new_process_scoped();
        let second_process_session = McpSessionId::new_process_scoped();
        assert_ne!(first_process_session, second_process_session);

        let registry = registry_with_abilities(vec![]);
        let first_process_bridge = McpAbilityBridge::new(&registry);
        first_process_bridge.insert_provenance(
            first_process_session,
            invocation(1),
            rendered(1),
        );
        let restarted_bridge = McpAbilityBridge::new(&registry);

        assert_eq!(
            restarted_bridge.get_provenance(first_process_session, invocation(1)),
            None
        );
    }

    #[tokio::test]
    async fn mcp_get_provenance_redacts_internal_ids_for_agent() {
        let registry = registry_with_abilities(vec![with_invoke_erased(
            descriptor(
                "agent_internal_provenance",
                AbilityCategory::Read,
                AGENT_ACTORS,
                LIVE_MODES,
            ),
            internal_provenance_erased,
        )]);
        let bridge = McpAbilityBridge::new(&registry);
        let session = session(1);

        let response = bridge
            .invoke_ability(
                session,
                "agent_internal_provenance",
                json!({}),
                false,
                None,
            )
            .await
            .unwrap();
        let detail = bridge
            .get_provenance(session, response.invocation_id)
            .expect("successful invocation caches detail provenance");

        assert_eq!(response.rendered_provenance.surface, BridgeSurface::McpTool);
        assert_eq!(detail.surface, BridgeSurface::McpToolDetail);
        for rendered in [&response.rendered_provenance.value, &detail.value] {
            assert!(rendered.get("internal_id").is_none());
            assert!(rendered.get("prompt_hash").is_none());
            assert!(rendered.get("seed").is_none());
            assert!(rendered.get("children").is_none());
            assert_eq!(rendered["invocation_id"], "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa");
        }
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
    fn mcp_bridge_get_provenance_tool_response_returns_rendered_mcp_tool_detail_for_known_pair() {
        let registry = registry_with_abilities(vec![]);
        let bridge = McpAbilityBridge::new(&registry);

        bridge.insert_provenance(session(1), invocation(1), rendered(1));

        let result = bridge.get_provenance_tool_response(session(1), invocation(1));
        assert_eq!(result.is_error, Some(false));
        let value = tool_result_json(&result);
        assert_eq!(value["surface"], "mcp_tool_detail");
        assert_eq!(value["value"]["index"], "1");
    }

    #[test]
    fn mcp_bridge_get_provenance_tool_response_returns_typed_error_for_missing_pair() {
        let registry = registry_with_abilities(vec![]);
        let bridge = McpAbilityBridge::new(&registry);

        let result = bridge.get_provenance_tool_response(session(1), invocation(1));

        assert_eq!(result.is_error, Some(true));
        assert_eq!(tool_result_json(&result), json!("ability_unavailable"));
    }

    #[test]
    fn mcp_bridge_get_provenance_tool_response_rejects_cross_session_invocation_id() {
        let registry = registry_with_abilities(vec![]);
        let bridge = McpAbilityBridge::new(&registry);

        bridge.insert_provenance(session(1), invocation(1), rendered(1));

        let result = bridge.get_provenance_tool_response(session(2), invocation(1));

        assert_eq!(result.is_error, Some(true));
        assert_eq!(tool_result_json(&result), json!("ability_unavailable"));
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

    #[test]
    fn mcp_bridge_invocation_cache_evicts_at_byte_cap_when_inserting_oversized_entry() {
        let registry = registry_with_abilities(vec![]);
        let bridge = McpAbilityBridge::new(&registry);
        let session = session(1);

        for index in 1..=32_u128 {
            bridge.insert_provenance(
                session,
                invocation(index),
                rendered_with_payload(index, 9 * 1024),
            );
        }

        let cache = bridge
            .invocation_cache
            .lock()
            .expect("mcp invocation cache poisoned");
        assert!(cache.current_serialized_bytes() <= MCP_INVOCATION_CACHE_TOTAL_BYTE_CAP);
        assert!(cache.len() < 32);
        drop(cache);

        assert_eq!(bridge.get_provenance(session, invocation(1)), None);
        assert!(bridge.get_provenance(session, invocation(32)).is_some());
    }

    #[test]
    fn mcp_bridge_get_provenance_tool_response_serialized_size_under_10kb_per_adr_0108() {
        let registry = registry_with_abilities(vec![]);
        let bridge = McpAbilityBridge::new(&registry);

        bridge.insert_provenance(session(1), invocation(1), rendered(1));

        let result = bridge.get_provenance_tool_response(session(1), invocation(1));
        let bytes = serde_json::to_vec(&result).unwrap();
        assert!(bytes.len() < MCP_INVOCATION_CACHE_ENTRY_BYTE_CAP);
    }
}
