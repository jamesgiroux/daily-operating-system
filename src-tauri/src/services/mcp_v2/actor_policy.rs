//! MCP v2 actor and ability-policy projection.
//!
//! ADR-0102 §B makes MCP asymmetric with `SurfaceClient`: MCP grants are
//! resolved and enforced at the gateway, while the runtime actor variant
//! carries only identity and optional conversation continuity. This module is
//! the small adapter between the MCP manifest records and the abilities
//! runtime shapes.

use std::time::SystemTime;

use abilities_runtime::abilities::registry::{
    AbilityPolicy, Actor, ActorKind, McpClientId as RuntimeMcpClientId, McpExposure,
    OpaqueConversationHandle as RuntimeOpaqueConversationHandle,
};

use super::contracts::{McpClientId, OpaqueConversationHandle, Scope, ScopedName};

const MCP_CLIENT_ACTORS: &[ActorKind] = &[ActorKind::McpClient];

/// Opaque Keychain account reference for an MCP client's transport key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeychainRef(pub String);

/// Gateway-loaded pairing record. Raw transport key bytes never live here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientRecord {
    pub client_id: McpClientId,
    pub paired_at: SystemTime,
    pub revoked_at: Option<SystemTime>,
    pub transport_key_ref: KeychainRef,
}

/// Per-tool invocation rate limit from the MCP client manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolRateLimit {
    pub max_calls: u32,
    pub window_seconds: u32,
}

/// Per-call manifest grant resolved by indexed `(client_id, tool_name)` lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolGrant {
    pub tool_name: ScopedName,
    pub scopes_granted: Vec<Scope>,
    pub exposure: McpExposure,
    pub rate_limit: ToolRateLimit,
}

/// Project a manifest grant into the runtime policy shell used by the
/// abilities gate for MCP-originated calls.
///
/// MCP scope checks are gateway-mediated against [`ToolGrant::scopes_granted`].
/// The runtime actor carries no scopes, so `required_scopes` stays empty here.
pub fn project_policy(tool_grant: &ToolGrant) -> AbilityPolicy {
    AbilityPolicy {
        allowed_actors: MCP_CLIENT_ACTORS,
        mcp_exposure: tool_grant.exposure,
        ..AbilityPolicy::default()
    }
}

/// Construct the runtime actor variant for an MCP-originated invocation.
///
/// `tool_grant` is intentionally accepted here even though scopes are not put
/// on the actor. Passing it through this API makes the gateway call site read
/// as "manifest resolved before runtime actor construction", which is the
/// ADR-0102 asymmetry this module exists to preserve.
pub fn project_actor(
    client_id: &McpClientId,
    tool_grant: &ToolGrant,
    conversation_handle: Option<&OpaqueConversationHandle>,
) -> Actor {
    let _ = project_policy(tool_grant);
    Actor::McpClient {
        client_id: RuntimeMcpClientId::new(client_id.as_str()),
        conversation_handle: conversation_handle
            .map(|handle| RuntimeOpaqueConversationHandle::new(handle.as_str())),
    }
}
