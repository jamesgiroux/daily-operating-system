//! Scope/permission matrix for Actor::McpClient.
//!
//! Consumed by gateway after auth resolves the per-client scope manifest.
//! Per ADR-0102 §B — 2026-05-19 amendment — scope authorization for
//! McpClient invocations is gateway-mediated; the runtime variant carries
//! no scopes. Implementation pending.
