//! Runtime-surface bridges for ability invocation.

pub mod correction_payload;
pub mod eval;
pub mod mcp;
pub mod surface_client;
pub mod tauri;
pub mod types;
pub mod worker;

pub use correction_payload::{
    project_claim_for_scope, project_composition_for_scope, CorrectionPayload,
};
pub use types::{
    confirmation_args_hash, AbilityInvokeError, AbilityResponseJson, AttestationDecision,
    AttestationRequestId, BridgeActor, BridgeSurface, BridgeSurfaceError, ConfirmationRecord,
    ConfirmationToken, ConfirmationTokenStore, InvocationContext, InvocationProvenanceCache,
    McpSessionId, RenderedProvenance, UserAttestationRequest,
};
