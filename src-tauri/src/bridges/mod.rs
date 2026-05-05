//! Runtime-surface bridges for ability invocation.

pub mod eval;
pub mod mcp;
pub mod tauri;
pub mod types;
pub mod worker;

pub use types::{
    confirmation_args_hash, AbilityInvokeError, AbilityResponseJson, AttestationDecision,
    AttestationRequestId, BridgeActor, BridgeSurface, BridgeSurfaceError, ConfirmationToken,
    InvocationContext, InvocationProvenanceCache, McpSessionId, RenderedProvenance,
    UserAttestationRequest,
};
