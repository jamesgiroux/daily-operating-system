//! MCP v2 gateway: headless MCP head over the DailyOS abilities runtime.
//!
//! Per ADR-0128 and the 2026-05-19 ADR-0102 amendment, this module is the
//! single ingress for MCP-originated invocations. Tool handlers never read the
//! DB directly; every call flows through the gateway into services::* or the
//! abilities runtime.
//!
//! The local MCP substrate enforces authorization and operational controls:
//! server-side scope manifest, exposure tier, rate limits, conversation handle
//! lifecycle, audit, and signals. Local transport ceremony is intentionally not
//! part of this layer.

pub mod actor_policy;
pub mod audit;
pub mod auth;
pub mod contracts;
pub mod gateway;
pub mod handler_context;
pub mod handlers;
pub mod local_runtime;
pub mod runtime_projection;
pub mod target_handles;
pub mod taxonomy;
pub mod transport;
