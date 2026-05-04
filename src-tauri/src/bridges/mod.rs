//! Runtime-surface bridges for ability invocation.

pub mod types;

pub use types::{
    AbilityInvokeError, AbilityResponseJson, BridgeActor, BridgeSurface, BridgeSurfaceError,
    InvocationContext, InvocationProvenanceCache, McpSessionId, RenderedProvenance,
};
