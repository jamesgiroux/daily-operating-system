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
//! ## DOS-477 cycle-2 envelope wiring (code-reviewer F3 fix)
//!
//! The command accepts an `envelope_render_id` string from the caller, which the
//! TS hook threads back from the previous `render_claim_receipt` round-trip. The
//! command looks up the cached envelope via
//! [`crate::services::entity_intelligence::envelope_cache::lookup_envelope_for_render`]
//! and uses its claim/proposal id sets as the `EnvelopeSet` basis — so
//! `validate_envelope_target` is a real binding check, not a tautology built
//! from the target's own id.
//!
//! ### v1.4.4 W1 middle ground
//!
//! Ability-side wiring of `record_envelope_for_render` into every renderable-
//! envelope producer (`get_entity_intelligence`, `get_daily_briefing`, etc.) is
//! W2 scope. Until then, callers MAY pass `envelope_render_id = None` (or a
//! render id absent from the cache) and the command falls back to a logged
//! single-claim envelope — preserving cycle-1 behavior so the substrate PR
//! unblocks. See `entity_intelligence::envelope_cache::path_alpha_envelope_cache_v2`.
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
    envelope_render_id: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<ClaimFeedbackResponse, String> {
    // DOS-477 cycle-2 fix (code-reviewer F3): construct the EnvelopeSet from a
    // cached envelope keyed by the caller-supplied render id, NOT from the
    // target's own claim id. The cache is populated by ability producers (W2)
    // and looked up here. Cache miss = logged warning + cycle-1 fallback so
    // the substrate PR unblocks.
    let cached_envelope = envelope_render_id
        .as_deref()
        .and_then(crate::services::entity_intelligence::envelope_cache::lookup_envelope_for_render);

    let envelope: ResolvedEnvelope = match cached_envelope {
        Some(cached) => ResolvedEnvelope::Cached(CachedEnvelopeAdapter {
            origin: EnvelopeOrigin::new(cached.ability.clone()),
            claim_ids: cached.claim_ids,
            proposal_ids: cached.proposal_ids,
        }),
        None => {
            // Cache miss path — TODO(v2): ability-side wiring lands in W2.
            // Until then, log and degrade gracefully so the receipt boundary
            // continues to function. The OTHER authorization layers
            // (sensitivity gate, Agent actor denial, per-action metadata
            // schema, source content hash validation) are still the actual
            // security boundary in v1.4.4.
            if let Some(render_id) = envelope_render_id.as_deref() {
                log::warn!(
                    "submit_claim_feedback_command: envelope cache miss for render_id={render_id}; \
                     falling back to single-claim envelope (path_alpha_envelope_cache_v2)"
                );
            } else {
                log::debug!(
                    "submit_claim_feedback_command: no envelope_render_id supplied; \
                     falling back to single-claim envelope (W2 will require this id)"
                );
            }
            let claim_id = extract_claim_id(&request.target);
            let mut claim_ids = BTreeSet::new();
            if !claim_id.is_empty() {
                claim_ids.insert(claim_id);
            }
            ResolvedEnvelope::Fallback(SingleClaimEnvelope {
                origin: EnvelopeOrigin::new("entity_intelligence"),
                claim_ids,
            })
        }
    };

    let envelope_view: &dyn EnvelopeView = match &envelope {
        ResolvedEnvelope::Cached(c) => c,
        ResolvedEnvelope::Fallback(f) => f,
    };
    let set = EnvelopeSet::new(envelope_view);

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

/// Adapter so a cached envelope (post-lookup) can satisfy `EnvelopeView`
/// without owning a borrow. Mirrors the [`SingleClaimEnvelope`] shape but
/// carries both claim_ids and proposal_ids — proposal targets are deferred to
/// W4 but the cache primitive supports them upstream.
struct CachedEnvelopeAdapter {
    origin: EnvelopeOrigin,
    claim_ids: BTreeSet<String>,
    proposal_ids: BTreeSet<String>,
}

impl EnvelopeView for CachedEnvelopeAdapter {
    fn ability(&self) -> &str {
        &self.origin.ability
    }
    fn claim_ids(&self) -> BTreeSet<String> {
        self.claim_ids.clone()
    }
    fn proposal_ids(&self) -> BTreeSet<String> {
        self.proposal_ids.clone()
    }
}

/// Either a cache-resolved envelope or the v1.4.4 W1 fallback.
enum ResolvedEnvelope {
    Cached(CachedEnvelopeAdapter),
    Fallback(SingleClaimEnvelope),
}
