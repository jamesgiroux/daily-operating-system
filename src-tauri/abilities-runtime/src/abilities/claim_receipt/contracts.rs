//! `claim_receipt` ability input contract.
//!
//! Output contract (`ClaimReceiptSnapshot`) lives in `services::context`
//! beside the narrow read handle so the live adapter and the ability share
//! the same DTO without a circular module dependency.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub use crate::services::context::{
    ClaimReceiptAction, ClaimReceiptFreshness, ClaimReceiptLifecycle, ClaimReceiptProvenance,
    ClaimReceiptProvenanceSource, ClaimReceiptRedactionLevel, ClaimReceiptSnapshot,
    ClaimReceiptSurfaceContext, ClaimReceiptTarget, ClaimReceiptTrust,
};

/// `claim_receipt` ability input.
///
/// Mirrors the existing Tauri command `render_claim_receipt(target, surface)`
/// signature so the wire shape is identical for callers that switch from the
/// command to the ability invocation path (WP block runtime client).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClaimReceiptInput {
    /// Substrate schema version. Bumped when the input/output contract
    /// changes incompatibly; for v1.4.4 W2 the only supported version is `1`.
    pub schema_version: u32,
    /// Target the receipt projects over.
    pub target: ClaimReceiptTarget,
    /// Surface routing for the audience filter
    /// (`audience_for_surface(surface)` → `Audience::UserTauri` vs
    /// `Audience::AgentMcp` per the receipt privacy boundary).
    pub surface: ClaimReceiptSurfaceContext,
}
