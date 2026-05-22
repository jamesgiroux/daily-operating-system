#![allow(
    clippy::let_underscore_must_use,
    reason = "tauri::command macro emits internal Result glue that discards generated metadata"
)]

//! Tauri command wrapper for `services::claim_receipt::feedback::submit_claim_feedback`.
//!
//! Composes the substrate-side primitives — envelope target binding, sensitivity gate
//! (Agent → deny in v1.4.4), per-action metadata schema validation, ADR-0108 §3 sanitizer,
//! server-issued idempotency key, typed feedback write — and returns the post-feedback
//! receipt so the TS layer can drive a re-render in one round trip.
//!
//! ## Cycle-2 envelope wiring (code-reviewer F3 + L3 cycle-1 codex F4)
//!
//! The command accepts an `envelope_render_id` AND an `actor_principal_id` from the
//! caller. The TS hook threads `envelope_render_id` back from the previous
//! `render_claim_receipt` round-trip, and the surface layer supplies the authenticated
//! principal id. The command looks up the cached envelope via
//! [`crate::services::entity_intelligence::envelope_cache::lookup_envelope_for_render`]
//! keyed by `(envelope_render_id, actor_principal_id, surface)` — so
//! `validate_envelope_target` is a real binding check, not a tautology built
//! from the target's own id, AND cross-principal cache lookups are rejected.
//!
//! Cache miss (no entry / expired) → `BadRequest::EnvelopeRequired`. The cycle-1
//! tautological single-claim fallback has been removed (L3 cycle-2 F4). Cross-actor
//! lookup → `Forbidden::PrincipalMismatch`. W2 wiring lands the ability-side
//! `record_envelope_for_render` calls into every renderable-envelope producer
//! (`get_entity_intelligence`, `get_daily_briefing`, etc.); until then any caller
//! that does not first render an envelope will receive `EnvelopeRequired`. This
//! is intentional — the workaround MUST NOT survive as the production path.
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
use crate::services::entity_intelligence::envelope_cache::{
    lookup_envelope_for_render, EnvelopeCacheError,
};
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

/// Default surface tag for feedback originating from the Tauri entity detail
/// surface. Other surfaces (MCP feedback) MUST pass their own surface tag.
const DEFAULT_SURFACE_TAG: &str = "tauri_entity_detail";

#[tauri::command]
pub async fn submit_claim_feedback_command(
    request: ClaimFeedbackRequest,
    envelope_render_id: Option<String>,
    actor_principal_id: Option<String>,
    surface: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<ClaimFeedbackResponse, String> {
    // L3 cycle-2 F4: actor_principal_id is REQUIRED. The previous hardcoded
    // `RenderActor::user("user", None)` was a cross-principal hazard; we
    // refuse to synthesize a principal at the command layer.
    let principal = actor_principal_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            "bad request: actor_principal_id is required (no principal available at command layer)"
                .to_string()
        })?;

    // L3 cycle-2 F4: envelope_render_id is REQUIRED. Cycle-1's
    // tautological single-claim fallback has been removed. Callers MUST
    // first render an envelope via `render_claim_receipt` (or the
    // entity-intelligence ability when W2 wiring lands) and thread the
    // returned render id back here.
    let render_id = envelope_render_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            "bad request: envelope_required — envelope_render_id is mandatory; \
             render an envelope first and pass the returned render id"
                .to_string()
        })?;

    let surface_tag = surface
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(DEFAULT_SURFACE_TAG);

    let cached =
        lookup_envelope_for_render(render_id, principal, surface_tag).map_err(|err| match err {
            EnvelopeCacheError::EnvelopeRequired(_) => format!(
                "bad request: envelope_required — no envelope binding for render id `{render_id}` \
                 (cache miss or expired); re-render and resubmit"
            ),
            EnvelopeCacheError::PrincipalMismatch { .. } => format!(
                "forbidden: principal_mismatch — envelope `{render_id}` was minted for a \
                 different actor/surface"
            ),
        })?;

    let envelope = CachedEnvelopeAdapter {
        origin: EnvelopeOrigin::new(cached.ability),
        claim_ids: cached.claim_ids,
        proposal_ids: cached.proposal_ids,
    };

    let envelope_view: &dyn EnvelopeView = &envelope;
    let set = EnvelopeSet::new(envelope_view);

    // L3 cycle-2 F4: actor derived from request-supplied principal id, not
    // hardcoded. `RenderActor::user` is correct here — the command is the
    // Tauri user-surface entry point; agent-feedback submissions land via a
    // different command path (MCP tool) once W4 ships.
    let actor = RenderActor::user(principal.to_string(), Some(principal.to_string()));

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
/// without owning a borrow. Carries both claim_ids and proposal_ids —
/// proposal targets are deferred to W4 but the cache primitive supports them
/// upstream.
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
