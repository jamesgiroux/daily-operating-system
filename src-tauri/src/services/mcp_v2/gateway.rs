//! MCP v2 gateway: the single entry point that receives MCP tool calls.
//!
//! Validates Actor::McpClient policy against the server-side scope
//! manifest loaded by auth; dispatches through registered McpToolHandler
//! implementations; records audit attribution; emits McpToolInvoked
//! signals — signal type registration happens in signals/policy_registry.rs
//! when the first handler lands.
//!
//! Implementation pending — see ADR-0102 amendment §C for the contract
//! and ADR-0128 for the product-surface frame.
