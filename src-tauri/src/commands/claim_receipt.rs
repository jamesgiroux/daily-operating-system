#![allow(
    clippy::let_underscore_must_use,
    reason = "tauri::command macro emits internal Result glue that discards generated metadata"
)]

//! Tauri command wrapper for `services::claim_receipt::render::render_receipt_for`.
//!
//! Threads `&AppState` into the shipped claim receipt substrate per AC-339.1 so the
//! `useClaimReceiptSubscription` TS hook can drive a re-render off the
//! `claim_verification_state_changed` signal (and adjacent claim-lifecycle
//! signals) without exposing the receipt rendering substrate to direct
//! invocation in user code.
//!
//! The signal substrate already exists:
//! * Registry: `signals::policy_registry::SignalType::ClaimVerificationStateChanged`
//!   (durable-claim policy; sync propagation via `ClaimSubject` resolver).
//! * Emission site: `services::claims::emit_claim_feedback_signals` at
//!   `claims.rs:8713` fires `claim_verification_state_changed` on every
//!   verification-state transition driven by `record_claim_feedback`.
//! * Coverage tests: `services::claims::tests` at `claims.rs:14472..=14486`.
//!
//! No new signal type is introduced here. The fan-out is wired by
//! re-using the existing `ClaimVerificationStateChanged` signal as the
//! subscription trigger plus a 250 ms trailing-edge debounce per
//! `(target.claim_id, surface)` pair in the TS hook (per AC-339.6).
//!
//! Proposal-receipt deferral (per §5.6 + cycle-1 codex-consult F3): only the
//! `Claim` arm of `ReceiptTarget` resolves; `Proposal` / `WorkItem` arms return
//! `RenderError::TargetNotFound`. W4 extension may add render policy for
//! Proposal targets; that is NOT in v1.4.4 W1 scope.

use std::sync::Arc;

use tauri::State;

use crate::services::claim_receipt::contracts::{ClaimReceipt, ReceiptTarget, SurfaceContext};
use crate::services::claim_receipt::render::render_receipt_for;
use crate::state::AppState;

#[tauri::command]
pub async fn render_claim_receipt(
    target: ReceiptTarget,
    surface: SurfaceContext,
    state: State<'_, Arc<AppState>>,
) -> Result<ClaimReceipt, String> {
    render_receipt_for(state.inner().as_ref(), target, surface)
        .await
        .map_err(|error| error.to_string())
}
