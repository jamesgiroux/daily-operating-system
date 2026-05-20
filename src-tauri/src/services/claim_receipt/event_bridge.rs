//! DOS-339 cycle-2 fix: signal → Tauri-event bridge for claim receipts.
//!
//! The Rust signal substrate (`signals::policy_registry::SignalType::ClaimVerificationStateChanged`)
//! fires whenever `services::claims::record_claim_feedback` mutates verification
//! state. The TS hook `useClaimReceiptSubscription` listens for the Tauri event
//! `claim_receipt:invalidated` and re-fetches receipts. Without this bridge, the
//! signal never crosses the Rust→TS boundary and the hook is dead-wired
//! (codex review cycle-1 P1).
//!
//! ## Bridge shape
//!
//! 1. `lib.rs` startup calls [`set_app_handle`] once during Tauri setup, stashing
//!    the [`tauri::AppHandle`] in a process-wide [`OnceLock`].
//! 2. `services::claims::emit_claim_feedback_signals` calls
//!    [`emit_claim_receipt_invalidated`] after the signal row commits — the
//!    bridge synthesizes a `ClaimReceiptInvalidationPayload` matching the TS
//!    hook's `ClaimReceiptInvalidationPayload` interface and ships it on the
//!    `claim_receipt:invalidated` Tauri event.
//!
//! The bridge is best-effort: failure to emit the event logs a warning but
//! never blocks the underlying mutation. Same shape as
//! `google::app_handle.emit("calendar-updated", ())` — the substrate writes
//! win, the UI nudge is a fan-out.
//!
//! ## Why a global OnceLock?
//!
//! `services::claims::emit_claim_feedback_signals` is called from inside a
//! sync DB transaction (`with_claim_transaction`) and does not have AppState
//! threaded through it. Threading AppState through every signal-emitting
//! service path is a substrate-wide refactor that is out of scope for v1.4.4
//! W1. The OnceLock matches the existing pattern in `claim_feedback.rs::IDEMPOTENCY_CACHE`.

use std::sync::OnceLock;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

/// Tauri event name. MUST match
/// `CLAIM_RECEIPT_INVALIDATION_EVENT` in
/// `src/services/claim-receipt/useClaimReceiptSubscription.ts`.
pub const CLAIM_RECEIPT_INVALIDATION_EVENT: &str = "claim_receipt:invalidated";

/// Payload shape matches the TS `ClaimReceiptInvalidationPayload` interface
/// (camelCase serialization).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaimReceiptInvalidationPayload {
    pub signal_type: String,
    pub claim_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
}

static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

/// Install the app handle. Called once from `lib.rs` Tauri setup. Subsequent
/// calls are no-ops (OnceLock semantics).
pub fn set_app_handle(handle: AppHandle) {
    // OnceLock::set returns Err if already initialized — that's the
    // intended subsequent-call behavior, not a real error path.
    if APP_HANDLE.set(handle).is_err() {
        log::debug!("claim_receipt event_bridge: app handle already installed");
    }
}

/// Emit a `claim_receipt:invalidated` Tauri event for the given claim. Best
/// effort — returns `false` if the bridge has not been installed (e.g. in
/// non-Tauri test contexts) or the emit fails. Never panics.
pub fn emit_claim_receipt_invalidated(
    signal_type: &str,
    claim_id: &str,
    from: Option<&str>,
    to: Option<&str>,
) -> bool {
    let Some(handle) = APP_HANDLE.get() else {
        // Bridge not installed — e.g. tests, headless contexts, MCP-only
        // process. Caller logs at substrate layer; this path is silent.
        return false;
    };
    let payload = ClaimReceiptInvalidationPayload {
        signal_type: signal_type.to_string(),
        claim_id: claim_id.to_string(),
        from: from.map(str::to_string),
        to: to.map(str::to_string),
    };
    match handle.emit(CLAIM_RECEIPT_INVALIDATION_EVENT, payload) {
        Ok(_) => true,
        Err(e) => {
            log::warn!(
                "claim_receipt:invalidated event emission failed; \
                 repair_target=signals_engine \
                 signal_type={signal_type} \
                 claim_id={claim_id}: {e}"
            );
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emit_is_no_op_when_handle_not_installed() {
        // Outside of Tauri context (unit tests), the bridge silently returns
        // false. This guarantees substrate writes are not blocked when the
        // bridge isn't wired.
        let emitted =
            emit_claim_receipt_invalidated("claim_verification_state_changed", "claim-1", None, None);
        assert!(
            !emitted,
            "without app_handle installed, emit must return false (best-effort)"
        );
    }

    #[test]
    fn payload_serializes_to_camel_case() {
        // The TS hook's ClaimReceiptInvalidationPayload uses camelCase keys
        // (signalType, claimId). Lock the serialization here so a future
        // serde-rename regression fails this test before reaching the hook.
        let payload = ClaimReceiptInvalidationPayload {
            signal_type: "claim_verification_state_changed".to_string(),
            claim_id: "claim-1".to_string(),
            from: Some("active".to_string()),
            to: Some("contested".to_string()),
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("\"signalType\""), "payload should emit signalType (camelCase): {json}");
        assert!(json.contains("\"claimId\""), "payload should emit claimId (camelCase): {json}");
        assert!(!json.contains("snake_case"), "no snake_case keys: {json}");
    }

    #[test]
    fn payload_omits_optional_fields_when_none() {
        let payload = ClaimReceiptInvalidationPayload {
            signal_type: "claim_verification_state_changed".to_string(),
            claim_id: "claim-1".to_string(),
            from: None,
            to: None,
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(!json.contains("\"from\""));
        assert!(!json.contains("\"to\""));
    }
}
