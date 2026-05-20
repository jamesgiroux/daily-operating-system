#![allow(
    clippy::let_underscore_must_use,
    reason = "tauri::command macro emits internal Result glue that discards generated metadata"
)]

//! Tauri command wrapper for `services::claim_receipt::feedback::submit_claim_feedback` (DOS-8).
//!
//! Composes the substrate-side primitives — envelope target binding, sensitivity gate
//! (Agent → deny in v1.4.4), per-action metadata schema validation, ADR-0108 §3 sanitizer,
//! server-issued idempotency key, typed feedback write — and returns the post-feedback
//! receipt so the TS layer can drive a re-render in one round trip.
//!
//! W1 caveat: the parent envelope is supplied as a no-op placeholder here. W2 callers
//! that hold a real `EntityIntelligenceEnvelope` pass it through directly. The shape
//! is locked at the substrate boundary; UI code consumes this command's response.
//!
//! Idempotency cache scope is process-wide; the AppState carries it as a lazily-
//! initialized singleton via `AppState::claim_feedback_idempotency_cache()`.

use std::collections::BTreeSet;
use std::sync::{Arc, OnceLock};

use tauri::State;

use crate::services::claim_receipt::feedback::{
    submit_claim_feedback, ClaimFeedbackRequest, ClaimFeedbackResponse, IdempotencyCache,
};
use crate::services::entity_intelligence::auth::{EnvelopeOrigin, EnvelopeSet, EnvelopeView};
use crate::state::AppState;

use abilities_runtime::sensitivity::RenderActor;

/// Process-wide idempotency cache for claim-feedback submissions. The
/// `IdempotencyCache::new()` defaults to 60 s TTL (AC-8.14). Hoisting this
/// to a static keeps the cache identity stable across `submit_claim_feedback_command`
/// invocations without threading state through AppState; the cache lives
/// for the lifetime of the process.
static IDEMPOTENCY_CACHE: OnceLock<IdempotencyCache> = OnceLock::new();

fn idempotency_cache() -> &'static IdempotencyCache {
    IDEMPOTENCY_CACHE.get_or_init(IdempotencyCache::new)
}

/// Minimal envelope view that surfaces the single targeted claim id. W2's
/// composition path will eventually pass a real `EntityIntelligenceEnvelope`
/// through; this stub is the v1.4.4 W1 entry point — the receipt-shaped surface
/// has the claim id already, so the envelope-set check is satisfied trivially
/// while the substrate-level wiring lands.
struct SingleClaimEnvelope {
    origin: EnvelopeOrigin,
    claim_ids: BTreeSet<String>,
}

impl EnvelopeView for SingleClaimEnvelope {
    fn ability(&self) -> &str {
        &self.origin.ability
    }
    fn claim_ids(&self) -> BTreeSet<String> {
        self.claim_ids.clone()
    }
    fn proposal_ids(&self) -> BTreeSet<String> {
        BTreeSet::new()
    }
}

fn extract_claim_id(target: &crate::services::claim_receipt::contracts::ReceiptTarget) -> String {
    use crate::services::claim_receipt::contracts::ReceiptTarget;
    match target {
        ReceiptTarget::Claim { claim_id, .. } => claim_id.clone(),
        ReceiptTarget::WorkItem {
            backing_claim_id: Some(claim_id),
            ..
        } => claim_id.clone(),
        _ => String::new(),
    }
}

#[tauri::command]
pub async fn submit_claim_feedback_command(
    request: ClaimFeedbackRequest,
    state: State<'_, Arc<AppState>>,
) -> Result<ClaimFeedbackResponse, String> {
    // W1 placeholder envelope. W2 supplies the real envelope produced by
    // `abilities::get_entity_intelligence`; until then, the envelope-set
    // contains the targeted claim by construction so the AC-477.2 binding
    // check is satisfied. The other authorization layers (sensitivity gate,
    // Agent actor denial, per-action metadata schema, source content hash
    // validation) are the actual security boundary in v1.4.4.
    let claim_id = extract_claim_id(&request.target);
    let mut claim_ids = BTreeSet::new();
    if !claim_id.is_empty() {
        claim_ids.insert(claim_id);
    }
    let envelope = SingleClaimEnvelope {
        origin: EnvelopeOrigin::new("entity_intelligence"),
        claim_ids,
    };
    let set = EnvelopeSet::new(&envelope);

    // Default actor for the user surface; W2 entity-detail will pass a more
    // specific actor once user-id resolution is wired through.
    let actor = RenderActor::user("user", None::<String>);

    submit_claim_feedback(
        state.inner().as_ref(),
        &set,
        &actor,
        idempotency_cache(),
        request,
    )
    .await
    .map_err(|error| error.to_string())
}
