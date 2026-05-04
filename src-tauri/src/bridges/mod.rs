//! Runtime-surface bridges for ability invocation.

pub mod eval;
pub mod tauri;
pub mod types;
pub mod worker;

pub use types::{
    AbilityInvokeError, AbilityResponseJson, BridgeActor, BridgeSurface, BridgeSurfaceError,
    InvocationContext, InvocationProvenanceCache, McpSessionId, RenderedProvenance,
};
