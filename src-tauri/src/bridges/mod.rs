//! Runtime-surface bridges for ability invocation.

pub mod eval;
pub mod mcp;
pub mod tauri;
pub mod types;
pub mod worker;

pub use types::{
    AbilityInvokeError, AbilityResponseJson, BridgeActor, BridgeSurface, BridgeSurfaceError,
    confirmation_args_hash, ConfirmationToken, InvocationContext, InvocationProvenanceCache,
    McpSessionId, RenderedProvenance, UserAttestationRequest,
};
