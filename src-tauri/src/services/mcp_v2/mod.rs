//! MCP v2 gateway: headless MCP head over the DailyOS abilities runtime.
//!
//! Per ADR-0128 — MCP as co-equal product surface — and the 2026-05-19
//! amendment to ADR-0102 — Actor::McpClient actor variant + MCP client
//! authentication + session contract — this module is the single ingress
//! for MCP-originated invocations. Tool handlers never read the DB directly;
//! every call flows through the gateway into services::* or the abilities
//! runtime.
//!
//! Submodules:
//! - contracts — shared wire-shape types — ToolDescription, ScopedName,
//!   Scope, McpActor, ToolError, Page, OpaqueConversationHandle envelope,
//!   McpClientId envelope.
//! - gateway — single entry-point dispatcher; loads scope manifest;
//!   validates Actor::McpClient policy; emits audit + signals.
//! - handlers — per-tool handler implementations — placeholder.
//! - taxonomy — tool catalog and host-selection contract — placeholder.
//! - actor_policy — scope/permission matrix for Actor::McpClient.
//! - auth — pairing handshake; server-side scope manifest loading;
//!   transport HMAC verification; conversation handle minting.
//! - audit — audit log writer with keyed HMAC parameter/response hashes.

pub mod actor_policy;
pub mod audit;
pub mod auth;
pub mod contracts;
pub mod gateway;
pub mod handlers;
pub mod taxonomy;
pub mod transport;
