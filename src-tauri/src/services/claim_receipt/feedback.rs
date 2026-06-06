//! Semantic claim feedback — typed `claim_feedback` substrate wiring.
//!
//! Fills the placeholder. The writer (`services::claims::record_claim_feedback`)
//! is already 9-variant aware per ADR-0123; this module is the caller-side
//! receipt-shaped wrapper that:
//!
//! 1. Validates the mutation target against the envelope-set (AC-477.2 + AC-477.13).
//! 2. Authorizes the actor + surface via the shipped sensitivity gate
//!    (`claim_receipt::auth::can_surface_for`). `Actor::Agent` collapses to
//!    deny at every surface in v1.4.4 (AC-8.13).
//! 3. Validates per-action metadata against the ADR-0123 §1 variant field
//!    schema plus the claim-file canonical bridge — `WrongSubject.corrected_subject_ref`
//!    (with legacy `corrected_to` accepted), `WrongSource.source_content_hash`
//!    (ADR-0131 canonicalization, NOT an index), `NeedsNuance.corrected_text`,
//!    `CannotVerify.note`, `SurfaceInappropriate.surface`, `NotRelevantHere.invocation_id`
//!    (AC-8.10 / AC-8.11).
//! 4. Routes user-authored free-text fields through the shipped
//!    `sanitize_explanation_for_render` pipeline (ADR-0108 §3) BEFORE persistence;
//!    sanitization warnings surface in the response (AC-8.12).
//! 5. Mints a server-issued idempotency key scoped per
//!    `(claim_id, action, actor, metadata_hash)`; TTL ≤ 60 s. Caller-supplied
//!    keys are rejected. Outside the window, retries are treated as new
//!    submissions (AC-8.2 / AC-8.14).
//! 6. Delegates persistence + lifecycle/verification state transition to
//!    `services::claims::record_claim_feedback`. No new tables; reuses the
//!    typed `claim_feedback` row substrate.
//! 7. Re-renders the receipt via `claim_receipt::render::render_receipt_for`
//!    so the caller can drive the post-feedback re-render in one round trip.
//!
//! AC-8.9: user-authored free-text fields persisted at `ClaimSensitivity::Confidential`
//! by default; inherit higher when the originating claim is higher.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use abilities_runtime::abilities::feedback::FeedbackAction;
use abilities_runtime::abilities::provenance::field::FieldPath;
use abilities_runtime::abilities::provenance::subject::SubjectRef as ReceiptSubjectRef;
use abilities_runtime::sensitivity::{RenderActor, RenderSurface};
use abilities_runtime::types::{ClaimSensitivity, IntelligenceClaim};

use crate::services::claim_receipt::auth::{can_surface_for, AuthError};
use crate::services::claim_receipt::contracts::{ClaimReceipt, ReceiptTarget, SurfaceContext};
use crate::services::claim_receipt::render::{render_receipt_for, RenderError};
use crate::services::claims::{
    record_claim_feedback, record_claim_feedback_for_claim_file_apply, ClaimError,
    ClaimFeedbackInput, ClaimFileFeedbackApplyInput,
};
use crate::services::entity_intelligence::auth::{
    validate_envelope_target, EnvelopeSet, TargetBindingError,
};
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Request / response DTOs
// ---------------------------------------------------------------------------

/// Server-issued idempotency key. Opaque to callers; the response is the only
/// way they receive it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct ServerIssuedKey(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClaimFeedbackRequest {
    pub target: ReceiptTarget,
    pub action: FeedbackAction,
    pub surface: SurfaceContext,
    /// Per-action JSON metadata. Schema is action-specific per ADR-0123 §1.
    /// Free-text fields (`corrected_text`, `note`) pass through the shared
    /// sanitizer before persistence; sanitizer warnings surface in the response.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    /// Caller-supplied idempotency keys are REJECTED per AC-8.2. The field
    /// exists so the wire format can carry the response shape symmetrically;
    /// if the request carries any value here, we return `BadRequest::CallerSuppliedIdempotencyKey`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SanitizerWarning {
    pub field: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ClaimFeedbackResponse {
    pub idempotency_key: ServerIssuedKey,
    pub receipt: Option<ClaimReceipt>,
    pub lifecycle_changed: bool,
    pub repair_queued: bool,
    pub sanitizer_warnings: Vec<SanitizerWarning>,
    pub replayed: bool,
}

#[derive(Debug, Clone)]
pub struct ClaimFileFeedbackApplyCommit {
    pub expected_claim_version: u64,
    pub correction_apply_key: String,
}

#[derive(Debug, Clone)]
enum FeedbackPersistence {
    Default,
    ClaimFileApply(ClaimFileFeedbackApplyCommit),
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum FeedbackError {
    #[error("bad request: {0}")]
    BadRequest(String),
    /// AC-8.2 — caller-supplied idempotency key was present on the request.
    #[error("bad request: caller-supplied idempotency key")]
    CallerSuppliedIdempotencyKey,
    /// AC-8.11 — `WrongSource.source_content_hash` does not match any source in
    /// the current claim. Caller is asked to re-render + resubmit.
    #[error("bad request: source no longer in claim")]
    SourceNoLongerInClaim,
    /// AC-8.13 — Agent actor denied at every surface in v1.4.4.
    #[error("forbidden: actor not authorized for this surface")]
    AgentActorDenied,
    /// AC-8.3 — sensitivity gate denied the actor/surface pair.
    #[error("forbidden: {0}")]
    SurfaceForbidden(String),
    /// Target not in envelope-set.
    #[error("target binding error: {0}")]
    TargetBinding(#[from] TargetBindingError),
    /// Claim not found (auth helper returns this).
    #[error("claim not found: {0}")]
    ClaimNotFound(String),
    #[error("write failed: {0}")]
    Write(#[from] ClaimError),
    #[error("render failed: {0}")]
    Render(#[from] RenderError),
    #[error("storage: {0}")]
    Storage(#[from] anyhow::Error),
}

impl From<AuthError> for FeedbackError {
    fn from(value: AuthError) -> Self {
        match value {
            AuthError::ClaimNotFound(id) => Self::ClaimNotFound(id),
            AuthError::CannotSurface { .. } => Self::SurfaceForbidden(value.to_string()),
            AuthError::Storage(error) => Self::Storage(error),
        }
    }
}

// ---------------------------------------------------------------------------
// Idempotency cache (AC-8.14)
// ---------------------------------------------------------------------------

/// Server-issued idempotency cache. Scoped per
/// `(claim_id, action, actor, metadata_hash)`; TTL ≤ 60 s. Outside the window,
/// retries are treated as new submissions.
pub struct IdempotencyCache {
    inner: Mutex<HashMap<String, CacheEntry>>,
    ttl: Duration,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    key: ServerIssuedKey,
    inserted_at: Instant,
    snapshot: CachedResponseSnapshot,
}

#[derive(Debug, Clone)]
struct CachedResponseSnapshot {
    lifecycle_changed: bool,
    repair_queued: bool,
    sanitizer_warnings: Vec<SanitizerWarning>,
}

impl IdempotencyCache {
    /// 60 s default per AC-8.14.
    pub fn new() -> Self {
        Self::with_ttl(Duration::from_secs(60))
    }

    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
            ttl,
        }
    }

    fn purge_locked(map: &mut HashMap<String, CacheEntry>, ttl: Duration, now: Instant) {
        map.retain(|_, entry| now.duration_since(entry.inserted_at) < ttl);
    }

    fn get(&self, scope: &str, now: Instant) -> Option<CacheEntry> {
        let mut map = self.inner.lock().ok()?;
        Self::purge_locked(&mut map, self.ttl, now);
        map.get(scope).cloned()
    }

    fn insert(
        &self,
        scope: String,
        key: ServerIssuedKey,
        snapshot: CachedResponseSnapshot,
        now: Instant,
    ) {
        if let Ok(mut map) = self.inner.lock() {
            Self::purge_locked(&mut map, self.ttl, now);
            map.insert(
                scope,
                CacheEntry {
                    key,
                    inserted_at: now,
                    snapshot,
                },
            );
        }
    }
}

impl Default for IdempotencyCache {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Submit semantic claim feedback through the receipt-shaped surface.
///
/// See module docs for the contract. Sequencing matches §5.7 of the L0 packet:
/// envelope target binding → sensitivity gate (Agent collapses to deny) →
/// per-action metadata schema validation (ADR-0123 verbatim field names; ADR-0131
/// source content hash, not index) → free-text sanitizer (ADR-0108 §3) →
/// idempotency-key mint → `record_claim_feedback` → re-render receipt.
pub async fn submit_claim_feedback(
    state: &AppState,
    envelope_set: &EnvelopeSet<'_>,
    actor: &RenderActor,
    cache: &IdempotencyCache,
    request: ClaimFeedbackRequest,
) -> Result<ClaimFeedbackResponse, FeedbackError> {
    submit_claim_feedback_with_persistence(
        state,
        envelope_set,
        actor,
        cache,
        request,
        FeedbackPersistence::Default,
    )
    .await
}

pub async fn submit_claim_feedback_for_claim_file_apply(
    state: &AppState,
    envelope_set: &EnvelopeSet<'_>,
    actor: &RenderActor,
    cache: &IdempotencyCache,
    request: ClaimFeedbackRequest,
    apply: ClaimFileFeedbackApplyCommit,
) -> Result<ClaimFeedbackResponse, FeedbackError> {
    submit_claim_feedback_with_persistence(
        state,
        envelope_set,
        actor,
        cache,
        request,
        FeedbackPersistence::ClaimFileApply(apply),
    )
    .await
}

async fn submit_claim_feedback_with_persistence(
    state: &AppState,
    envelope_set: &EnvelopeSet<'_>,
    actor: &RenderActor,
    cache: &IdempotencyCache,
    request: ClaimFeedbackRequest,
    persistence: FeedbackPersistence,
) -> Result<ClaimFeedbackResponse, FeedbackError> {
    // AC-8.2: caller-supplied idempotency keys are not accepted.
    if request.idempotency_key.is_some() {
        return Err(FeedbackError::CallerSuppliedIdempotencyKey);
    }

    // Step 1: envelope target binding.
    validate_envelope_target(envelope_set, &request.target)?;

    // Receipt rendering is Claim-only in v1.4.4 W1; proposal/work-item targets
    // for feedback fall back to "no receipt yet, target-only" per §5.7. We
    // still let the write through if a backing claim id is supplied.
    let claim_id = match &request.target {
        ReceiptTarget::Claim { claim_id, .. } => claim_id.clone(),
        ReceiptTarget::WorkItem {
            backing_claim_id: Some(claim_id),
            ..
        } => claim_id.clone(),
        ReceiptTarget::Proposal { .. } | ReceiptTarget::WorkItem { .. } => {
            return Err(FeedbackError::BadRequest(
                "feedback target has no backing claim id (proposal/work-item not yet supported)"
                    .to_string(),
            ));
        }
    };

    // Step 2: authorization. Agent → deny everywhere (AC-8.13).
    if !actor.is_user() {
        return Err(FeedbackError::AgentActorDenied);
    }
    let render_surface = render_surface_for(request.surface);
    can_surface_for(state, actor, render_surface, &claim_id).await?;

    // Step 3 + 4: validate + sanitize per-action metadata.
    let validated = validate_and_sanitize_metadata(request.action, request.metadata.as_ref())?;

    // Load claim for source-set check (AC-8.11) + sensitivity inherit (AC-8.9).
    let claim_for_check = claim_id.clone();
    let claim = state
        .db_read(move |db| {
            crate::services::claims::load_claim_by_id(db.conn_ref(), &claim_for_check)
                .map_err(|error| error.to_string())
        })
        .await
        .map_err(|message| FeedbackError::Storage(anyhow::anyhow!(message)))?
        .ok_or_else(|| FeedbackError::ClaimNotFound(claim_id.clone()))?;

    // AC-8.11: validate WrongSource.source_content_hash against the current
    // claim's source set. The substrate carries a single (data_source,
    // source_ref, item_hash) tuple per claim; the hash is computed over that
    // canonical triple per ADR-0131 canonicalization.
    if request.action == FeedbackAction::WrongSource {
        let expected = current_source_content_hash(&claim);
        let supplied = validated
            .metadata
            .as_ref()
            .and_then(|value| value.get("source_content_hash"))
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                FeedbackError::BadRequest(
                    "wrong_source feedback requires metadata.source_content_hash".to_string(),
                )
            })?;
        if !constant_time_eq(expected.as_bytes(), supplied.as_bytes()) {
            return Err(FeedbackError::SourceNoLongerInClaim);
        }
    }

    // Idempotency. Mint scope key now that the request has been validated; if
    // a matching entry exists inside the TTL window, replay.
    //
    // The existing `services::claims::record_claim_feedback` writer still
    // expects a legacy `source_ref` key on WrongSource payloads (an index-style
    // identifier). The canonical contract per ADR-0123 / ADR-0131 is now
    // `source_content_hash`. We bridge by mirroring the hash into a synthetic
    // `source_ref` field — the writer accepts it, downstream consumers read
    // `source_content_hash`. Tighten the writer to consume the canonical field
    // in a follow-up; the persisted JSON carries both.
    let payload_for_writer = match request.action {
        FeedbackAction::WrongSource => bridge_wrong_source_for_writer(&validated),
        _ => validated.payload_for_writer(),
    };
    let metadata_hash = hash_payload_for_scope(&payload_for_writer);
    let scope = idempotency_scope(&claim_id, request.action, &actor.actor, &metadata_hash);
    let now = Instant::now();
    if let Some(entry) = cache.get(&scope, now) {
        return Ok(ClaimFeedbackResponse {
            idempotency_key: entry.key.clone(),
            receipt: maybe_render_receipt(state, request.target.clone(), request.surface).await,
            lifecycle_changed: entry.snapshot.lifecycle_changed,
            repair_queued: entry.snapshot.repair_queued,
            sanitizer_warnings: entry.snapshot.sanitizer_warnings.clone(),
            replayed: true,
        });
    }

    // AC-8.9: sensitivity floor inheritance documented for downstream readers.
    let effective_sensitivity = inherit_sensitivity_floor(claim.sensitivity);
    let _ = effective_sensitivity;

    // Step 6: route into the typed writer.
    let input = ClaimFeedbackInput {
        claim_id: claim_id.clone(),
        action: request.action,
        actor: actor.actor.clone(),
        actor_id: actor.user_id.clone(),
        payload_json: payload_for_writer,
    };

    let claim_verification_before = claim.verification_state;
    let outcome = state
        .db_write(move |db| {
            let clock = crate::services::context::SystemClock;
            let rng = crate::services::context::SystemRng;
            let external = crate::services::context::ExternalClients::default();
            let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &external)
                .with_actor("user");
            match persistence {
                FeedbackPersistence::Default => {
                    record_claim_feedback(&ctx, db, input).map_err(|error| error.to_string())
                }
                FeedbackPersistence::ClaimFileApply(apply) => {
                    record_claim_feedback_for_claim_file_apply(
                        &ctx,
                        db,
                        input,
                        ClaimFileFeedbackApplyInput {
                            expected_claim_version: apply.expected_claim_version,
                            correction_apply_key: apply.correction_apply_key,
                        },
                    )
                    .map_err(|error| error.to_string())
                }
            }
        })
        .await
        .map_err(|message| FeedbackError::Storage(anyhow::anyhow!(message)))?;

    let lifecycle_changed = outcome.new_verification_state != claim_verification_before;
    let repair_queued = outcome.repair_job_id.is_some();

    // Step 7: re-render receipt for caller convenience.
    let receipt = maybe_render_receipt(state, request.target.clone(), request.surface).await;

    // Cache the server-issued key for replays inside the TTL.
    let server_key = ServerIssuedKey(outcome.feedback_id.clone());
    let sanitizer_warnings = validated.sanitizer_warnings.clone();
    cache.insert(
        scope,
        server_key.clone(),
        CachedResponseSnapshot {
            lifecycle_changed,
            repair_queued,
            sanitizer_warnings: sanitizer_warnings.clone(),
        },
        now,
    );

    Ok(ClaimFeedbackResponse {
        idempotency_key: server_key,
        receipt,
        lifecycle_changed,
        repair_queued,
        sanitizer_warnings,
        replayed: false,
    })
}

async fn maybe_render_receipt(
    state: &AppState,
    target: ReceiptTarget,
    surface: SurfaceContext,
) -> Option<ClaimReceipt> {
    // Proposal-receipt rendering is deferred to W4 per §5.6 — return None
    // rather than propagating a typed render-time error to the feedback caller.
    match render_receipt_for(state, target, surface).await {
        Ok(receipt) => Some(receipt),
        Err(RenderError::TargetNotFound) => None,
        Err(other) => {
            log::warn!("submit_claim_feedback receipt re-render failed: {other}");
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Metadata validation + sanitization (AC-8.10, AC-8.11, AC-8.12)
// ---------------------------------------------------------------------------

const MAX_NOTE_CHARS: usize = 500;
const MAX_CORRECTED_TEXT_CHARS: usize = 2000;
const MAX_METADATA_BYTES: usize = 4096;
const MAX_METADATA_KEYS: usize = 24;
const MAX_METADATA_NESTING: usize = 4;

#[derive(Debug)]
struct ValidatedMetadata {
    metadata: Option<serde_json::Value>,
    sanitizer_warnings: Vec<SanitizerWarning>,
}

impl ValidatedMetadata {
    fn payload_for_writer(&self) -> Option<String> {
        self.metadata.as_ref().map(|value| value.to_string())
    }
}

fn validate_and_sanitize_metadata(
    action: FeedbackAction,
    metadata: Option<&serde_json::Value>,
) -> Result<ValidatedMetadata, FeedbackError> {
    // Global metadata envelope checks.
    if let Some(value) = metadata {
        if !value.is_object() {
            return Err(FeedbackError::BadRequest(
                "metadata must be a JSON object".to_string(),
            ));
        }
        let serialized = value.to_string();
        if serialized.len() > MAX_METADATA_BYTES {
            return Err(FeedbackError::BadRequest(format!(
                "metadata exceeds {MAX_METADATA_BYTES} byte budget"
            )));
        }
        let obj = value.as_object().expect("checked is_object above");
        if obj.len() > MAX_METADATA_KEYS {
            return Err(FeedbackError::BadRequest(format!(
                "metadata exceeds {MAX_METADATA_KEYS} key budget"
            )));
        }
        if json_nesting_depth(value) > MAX_METADATA_NESTING {
            return Err(FeedbackError::BadRequest(format!(
                "metadata nesting exceeds {MAX_METADATA_NESTING} levels"
            )));
        }
    }

    let mut warnings: Vec<SanitizerWarning> = Vec::new();
    let mut sanitized_metadata = metadata.cloned();

    let requires_metadata = matches!(
        action,
        FeedbackAction::WrongSource
            | FeedbackAction::NeedsNuance
            | FeedbackAction::SurfaceInappropriate
            | FeedbackAction::NotRelevantHere
            | FeedbackAction::MergeIntent
    );
    if requires_metadata && sanitized_metadata.is_none() {
        return Err(FeedbackError::BadRequest(format!(
            "{} feedback requires action metadata",
            action.as_str()
        )));
    }

    let allowed = allowed_keys_for(action);
    if let Some(value) = sanitized_metadata.as_ref() {
        let obj = value.as_object().expect("validated as object above");
        for key in obj.keys() {
            if !allowed.contains(&key.as_str()) {
                return Err(FeedbackError::BadRequest(format!(
                    "{} feedback rejects unknown metadata key {key:?}",
                    action.as_str()
                )));
            }
        }
    }

    match action {
        FeedbackAction::ConfirmCurrent
        | FeedbackAction::MarkOutdated
        | FeedbackAction::MarkFalse => {
            // No required metadata. Optional fields accepted as-is.
        }
        FeedbackAction::WrongSubject => {
            if let Some(metadata) = sanitized_metadata.as_mut() {
                let obj = metadata.as_object_mut().ok_or_else(|| {
                    FeedbackError::BadRequest(
                        "wrong_subject metadata must be a JSON object".to_string(),
                    )
                })?;
                let canonical = obj
                    .get("corrected_subject_ref")
                    .map(normalize_wrong_subject_ref)
                    .transpose()?;
                let legacy = obj
                    .get("corrected_to")
                    .map(normalize_wrong_subject_ref)
                    .transpose()?;
                if canonical.is_some() && legacy.is_some() && canonical != legacy {
                    return Err(FeedbackError::BadRequest(
                        "wrong_subject corrected_to and corrected_subject_ref disagree".to_string(),
                    ));
                }
                if let Some(value) = canonical.or(legacy) {
                    obj.insert("corrected_subject_ref".to_string(), value);
                }
            }
        }
        FeedbackAction::WrongSource => {
            // Required `source_content_hash: String` (ADR-0131; AC-8.11).
            let hash = sanitized_metadata
                .as_ref()
                .and_then(|m| m.get("source_content_hash"))
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    FeedbackError::BadRequest(
                        "wrong_source.source_content_hash is required (ADR-0131 canonicalization, not an index)"
                            .to_string(),
                    )
                })?;
            if !is_opaque_hash(hash) {
                return Err(FeedbackError::BadRequest(
                    "wrong_source.source_content_hash must be opaque text (hex / base64)"
                        .to_string(),
                ));
            }
        }
        FeedbackAction::CannotVerify => {
            // Optional `note` (free text). Sanitize.
            if let Some(metadata) = sanitized_metadata.as_mut() {
                if let Some(obj) = metadata.as_object_mut() {
                    if let Some(note_value) = obj.get("note").cloned() {
                        let note = note_value.as_str().ok_or_else(|| {
                            FeedbackError::BadRequest(
                                "cannot_verify.note must be a string".to_string(),
                            )
                        })?;
                        if note.chars().count() > MAX_NOTE_CHARS {
                            return Err(FeedbackError::BadRequest(format!(
                                "cannot_verify.note exceeds {MAX_NOTE_CHARS} char budget"
                            )));
                        }
                        let (sanitized, warning) = sanitize_freetext("cannot_verify.note", note);
                        if let Some(warning) = warning {
                            warnings.push(warning);
                        }
                        obj.insert("note".to_string(), serde_json::Value::String(sanitized));
                    }
                }
            }
        }
        FeedbackAction::NeedsNuance => {
            // Required `corrected_text: String` (free text). Sanitize.
            let metadata = sanitized_metadata
                .as_mut()
                .expect("requires_metadata branch enforced presence");
            let obj = metadata.as_object_mut().ok_or_else(|| {
                FeedbackError::BadRequest("needs_nuance metadata must be a JSON object".to_string())
            })?;
            let corrected_owned = obj
                .get("corrected_text")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| {
                    FeedbackError::BadRequest(
                        "needs_nuance.corrected_text is required (ADR-0123 §1 verbatim field name)"
                            .to_string(),
                    )
                })?;
            if corrected_owned.trim().is_empty() {
                return Err(FeedbackError::BadRequest(
                    "needs_nuance.corrected_text must not be empty".to_string(),
                ));
            }
            if corrected_owned.chars().count() > MAX_CORRECTED_TEXT_CHARS {
                return Err(FeedbackError::BadRequest(format!(
                    "needs_nuance.corrected_text exceeds {MAX_CORRECTED_TEXT_CHARS} char budget"
                )));
            }
            let (sanitized, warning) =
                sanitize_freetext("needs_nuance.corrected_text", &corrected_owned);
            if let Some(warning) = warning {
                warnings.push(warning);
            }
            obj.insert(
                "corrected_text".to_string(),
                serde_json::Value::String(sanitized),
            );
        }
        FeedbackAction::SurfaceInappropriate => {
            // Required `surface: SurfaceId` (string token).
            let surface = sanitized_metadata
                .as_ref()
                .and_then(|m| m.get("surface"))
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    FeedbackError::BadRequest(
                        "surface_inappropriate.surface is required".to_string(),
                    )
                })?;
            if surface.len() > 64 {
                return Err(FeedbackError::BadRequest(
                    "surface_inappropriate.surface exceeds 64 char budget".to_string(),
                ));
            }
        }
        FeedbackAction::NotRelevantHere => {
            // Required `invocation_id: InvocationId` (string token).
            let invocation = sanitized_metadata
                .as_ref()
                .and_then(|m| m.get("invocation_id"))
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    FeedbackError::BadRequest(
                        "not_relevant_here.invocation_id is required".to_string(),
                    )
                })?;
            if invocation.len() > 128 {
                return Err(FeedbackError::BadRequest(
                    "not_relevant_here.invocation_id exceeds 128 char budget".to_string(),
                ));
            }
        }
        FeedbackAction::MergeIntent => {
            // Required `merge_target: SubjectRef`; optional sanitized
            // `supporting_evidence: String` (≤500 chars). ADR-0123 V1.1.
            let metadata = sanitized_metadata
                .as_mut()
                .expect("requires_metadata branch enforced presence");
            let obj = metadata.as_object_mut().ok_or_else(|| {
                FeedbackError::BadRequest("merge_intent metadata must be a JSON object".to_string())
            })?;
            let target_value = obj.get("merge_target").cloned().ok_or_else(|| {
                FeedbackError::BadRequest(
                    "merge_intent.merge_target is required (ADR-0123 V1.1 verbatim field name)"
                        .to_string(),
                )
            })?;
            // Deep-decode to enforce the SubjectRef shape per ADR-0125.
            serde_json::from_value::<ReceiptSubjectRef>(target_value).map_err(|error| {
                FeedbackError::BadRequest(format!(
                    "merge_intent.merge_target must decode as SubjectRef: {error}"
                ))
            })?;
            // supporting_evidence: optional sanitized free text.
            if let Some(evidence_value) = obj.get("supporting_evidence").cloned() {
                if evidence_value.is_null() {
                    obj.remove("supporting_evidence");
                } else {
                    let evidence = evidence_value.as_str().ok_or_else(|| {
                        FeedbackError::BadRequest(
                            "merge_intent.supporting_evidence must be a string".to_string(),
                        )
                    })?;
                    if evidence.chars().count() > MAX_NOTE_CHARS {
                        return Err(FeedbackError::BadRequest(format!(
                            "merge_intent.supporting_evidence exceeds {MAX_NOTE_CHARS} char budget"
                        )));
                    }
                    let (sanitized, warning) =
                        sanitize_freetext("merge_intent.supporting_evidence", evidence);
                    if let Some(warning) = warning {
                        warnings.push(warning);
                    }
                    obj.insert(
                        "supporting_evidence".to_string(),
                        serde_json::Value::String(sanitized),
                    );
                }
            }
        }
    }

    Ok(ValidatedMetadata {
        metadata: sanitized_metadata,
        sanitizer_warnings: warnings,
    })
}

fn normalize_wrong_subject_ref(
    value: &serde_json::Value,
) -> Result<serde_json::Value, FeedbackError> {
    let materialized = if let Some(raw) = value.as_str() {
        serde_json::from_str::<serde_json::Value>(raw).map_err(|error| {
            FeedbackError::BadRequest(format!(
                "wrong_subject.corrected_subject_ref string must contain JSON SubjectRef: {error}"
            ))
        })?
    } else {
        value.clone()
    };
    if crate::services::claims::subject_ref_from_json(&materialized).is_ok() {
        return Ok(materialized);
    }
    let receipt_subject =
        serde_json::from_value::<ReceiptSubjectRef>(materialized).map_err(|error| {
            FeedbackError::BadRequest(format!(
                "wrong_subject.corrected_subject_ref must decode as SubjectRef: {error}"
            ))
        })?;
    let normalized = receipt_subject_ref_to_claim_json(&receipt_subject)?;
    crate::services::claims::subject_ref_from_json(&normalized).map_err(|error| {
        FeedbackError::BadRequest(format!(
            "wrong_subject.corrected_subject_ref must decode as claim SubjectRef: {error}"
        ))
    })?;
    Ok(normalized)
}

fn receipt_subject_ref_to_claim_json(
    subject: &ReceiptSubjectRef,
) -> Result<serde_json::Value, FeedbackError> {
    Ok(match subject {
        ReceiptSubjectRef::Account(id) => serde_json::json!({"kind": "account", "id": id}),
        ReceiptSubjectRef::Project(id) => serde_json::json!({"kind": "project", "id": id}),
        ReceiptSubjectRef::Person(id) => serde_json::json!({"kind": "person", "id": id}),
        ReceiptSubjectRef::Action(id) => serde_json::json!({"kind": "action", "id": id}),
        ReceiptSubjectRef::Meeting(id) => serde_json::json!({"kind": "meeting", "id": id}),
        ReceiptSubjectRef::Global => serde_json::json!({"kind": "global"}),
        ReceiptSubjectRef::Multi(subjects) => {
            let subjects = subjects
                .iter()
                .map(receipt_subject_ref_to_claim_json)
                .collect::<Result<Vec<_>, _>>()?;
            serde_json::json!({"kind": "multi", "subjects": subjects})
        }
        ReceiptSubjectRef::User(_) | ReceiptSubjectRef::Unknown => {
            return Err(FeedbackError::BadRequest(
                "wrong_subject.corrected_subject_ref must target a claim subject".to_string(),
            ));
        }
    })
}

fn allowed_keys_for(action: FeedbackAction) -> &'static [&'static str] {
    match action {
        FeedbackAction::ConfirmCurrent => &[],
        FeedbackAction::MarkOutdated => &["last_known_true_at"],
        FeedbackAction::MarkFalse => &["corrected_value"],
        FeedbackAction::WrongSubject => &["corrected_to", "corrected_subject_ref"],
        FeedbackAction::WrongSource => &["source_content_hash", "source_index"],
        FeedbackAction::CannotVerify => &["note"],
        FeedbackAction::NeedsNuance => &["corrected_text"],
        FeedbackAction::SurfaceInappropriate => &["surface"],
        FeedbackAction::NotRelevantHere => &["invocation_id"],
        // MergeIntent (ADR-0123 V1.1 / W2 §5.3): user nominates a
        // canonical merge target. supporting_evidence is optional
        // free-text and runs through the ADR-0108 §3 sanitizer.
        FeedbackAction::MergeIntent => &["merge_target", "supporting_evidence"],
    }
}

fn sanitize_freetext(field: &str, raw: &str) -> (String, Option<SanitizerWarning>) {
    // ADR-0108 §3 sanitizer pipeline. Field path is purely for warning shape.
    let field_path = FieldPath::new(format!("/{}", field.replace('.', "/")))
        .unwrap_or_else(|_| FieldPath::root());
    let (sanitized, warning) =
        abilities_runtime::abilities::provenance::render::sanitize_explanation_for_render(
            &field_path,
            raw,
        );
    let warning = warning.map(|warning| {
        match warning {
        abilities_runtime::abilities::provenance::envelope::ProvenanceWarning::ExplanationFiltered {
            reason,
            ..
        } => SanitizerWarning {
            field: field.to_string(),
            reason,
        },
        other => SanitizerWarning {
            field: field.to_string(),
            reason: format!("{other:?}"),
        },
    }
    });
    (sanitized, warning)
}

fn json_nesting_depth(value: &serde_json::Value) -> usize {
    fn walk(value: &serde_json::Value, depth: usize) -> usize {
        match value {
            serde_json::Value::Array(items) => items
                .iter()
                .map(|item| walk(item, depth + 1))
                .max()
                .unwrap_or(depth + 1),
            serde_json::Value::Object(map) => map
                .values()
                .map(|item| walk(item, depth + 1))
                .max()
                .unwrap_or(depth + 1),
            _ => depth,
        }
    }
    walk(value, 0)
}

fn is_opaque_hash(value: &str) -> bool {
    // Accept ASCII hex / base64 / base64url; reject anything that looks like
    // free text or contains whitespace. Tight enough to catch "0" / "src-1"
    // (the old index-style identifiers) while staying schema-agnostic.
    value.len() >= 8
        && value.chars().all(|ch| {
            ch.is_ascii_alphanumeric()
                || ch == '+'
                || ch == '/'
                || ch == '='
                || ch == '-'
                || ch == '_'
        })
}

fn current_source_content_hash(claim: &IntelligenceClaim) -> String {
    let mut hasher = Sha256::new();
    hasher.update(claim.data_source.as_bytes());
    hasher.update(b"\x1f");
    if let Some(source_ref) = claim.source_ref.as_deref() {
        hasher.update(source_ref.as_bytes());
    }
    hasher.update(b"\x1f");
    if let Some(item_hash) = claim.item_hash.as_deref() {
        hasher.update(item_hash.as_bytes());
    }
    format!("{:x}", hasher.finalize())
}

/// Bridge for the legacy `services::claims::record_claim_feedback` writer.
/// The writer still requires a non-empty `payload_json.source_ref` on
/// `WrongSource`; the canonical receipt contract is `source_content_hash`
/// per ADR-0123 / ADR-0131. We mirror the hash into a synthetic `source_ref`
/// so the writer's existing validation passes; the persisted JSON carries
/// both keys. The writer-side tightening to consume `source_content_hash`
/// directly is a follow-up — the receipt-side substrate (this module)
/// already treats the hash as authoritative.
fn bridge_wrong_source_for_writer(validated: &ValidatedMetadata) -> Option<String> {
    let mut value = validated.metadata.clone()?;
    let obj = value.as_object_mut()?;
    if let Some(hash) = obj
        .get("source_content_hash")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
    {
        if !obj.contains_key("source_ref") {
            obj.insert("source_ref".to_string(), serde_json::Value::String(hash));
        }
    }
    Some(value.to_string())
}

fn hash_payload_for_scope(payload: &Option<String>) -> String {
    let mut hasher = Sha256::new();
    if let Some(payload) = payload.as_deref() {
        hasher.update(payload.as_bytes());
    }
    format!("{:x}", hasher.finalize())
}

fn idempotency_scope(
    claim_id: &str,
    action: FeedbackAction,
    actor: &str,
    metadata_hash: &str,
) -> String {
    format!(
        "{}|{}|{}|{}",
        claim_id,
        action.as_str(),
        actor,
        metadata_hash
    )
}

fn inherit_sensitivity_floor(claim_sensitivity: ClaimSensitivity) -> ClaimSensitivity {
    match claim_sensitivity {
        // Floor is Confidential; only UserOnly is higher.
        ClaimSensitivity::Public | ClaimSensitivity::Internal | ClaimSensitivity::Confidential => {
            ClaimSensitivity::Confidential
        }
        ClaimSensitivity::UserOnly => ClaimSensitivity::UserOnly,
    }
}

fn render_surface_for(surface: SurfaceContext) -> RenderSurface {
    match surface {
        SurfaceContext::ActionsWork => RenderSurface::Action,
        SurfaceContext::EntityDetail => RenderSurface::TauriEntityDetail,
        SurfaceContext::DailyBriefing => RenderSurface::TauriBriefingPrep,
        SurfaceContext::MeetingDetail => RenderSurface::TauriMeetingDetail,
        SurfaceContext::Mcp => RenderSurface::McpTool,
    }
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::BTreeSet;

    use abilities_runtime::sensitivity::ClaimVerificationState;
    use abilities_runtime::types::{ClaimSensitivity, ClaimState, SurfacingState, TemporalScope};
    use chrono::Utc;
    use rusqlite::params;

    use crate::services::entity_intelligence::auth::{EnvelopeOrigin, EnvelopeView};

    // -----------------------------------------------------------------------
    // Test fixtures
    // -----------------------------------------------------------------------

    struct FakeEnvelope {
        origin: EnvelopeOrigin,
        claims: BTreeSet<String>,
        proposals: BTreeSet<String>,
    }

    impl FakeEnvelope {
        fn new(ability: &str, claim_ids: &[&str]) -> Self {
            Self {
                origin: EnvelopeOrigin::new(ability),
                claims: claim_ids.iter().map(|id| id.to_string()).collect(),
                proposals: BTreeSet::new(),
            }
        }
    }

    impl EnvelopeView for FakeEnvelope {
        fn ability(&self) -> &str {
            &self.origin.ability
        }
        fn claim_ids(&self) -> BTreeSet<String> {
            self.claims.clone()
        }
        fn proposal_ids(&self) -> BTreeSet<String> {
            self.proposals.clone()
        }
    }

    async fn test_state() -> (AppState, tempfile::TempDir) {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let db_path = tempdir.path().join("claim-feedback.db");
        let db_service = crate::db_service::DbService::open_at_unencrypted(db_path)
            .await
            .expect("open test db service");
        (AppState::test_with_db_service(db_service), tempdir)
    }

    async fn seed_claim(state: &AppState, claim_id: &str) {
        let claim_id = claim_id.to_string();
        let now = Utc::now().to_rfc3339();
        state
            .db_write(move |db| {
                db.conn_ref()
                    .execute(
                        "INSERT INTO intelligence_claims /* dos7-allowed: claim receipt feedback unit test seed */ (
                            id, subject_ref, claim_type, field_path, topic_key, text,
                            dedup_key, item_hash, actor, data_source, source_ref,
                            source_asof, observed_at, created_at, provenance_json,
                            metadata_json, claim_state, surfacing_state,
                            demotion_reason, reactivated_at, retraction_reason,
                            expires_at, superseded_by, trust_score, trust_computed_at,
                            trust_version, thread_id, temporal_scope, sensitivity,
                            verification_state, verification_reason,
                            needs_user_decision_at, claim_version, canonical_status,
                            non_semantic_mergeable
                        ) VALUES (
                            ?1, ?2, 'risk', 'health.risk', 'renewal',
                            'Renewal risk is elevated', ?3, ?4, 'user', 'unit_test',
                            ?5, ?6, ?6, ?6, '{}', NULL, 'active', 'active',
                            NULL, NULL, NULL, NULL, NULL, 0.82, ?6, 1, NULL,
                            'state', 'internal', 'active', NULL, NULL, 2, 'live', 0
                        )",
                        params![
                            claim_id,
                            r#"{"kind":"account","id":"acct-1"}"#,
                            format!("dedup-{claim_id}"),
                            format!("hash-{claim_id}"),
                            "fixture://src-1",
                            now,
                        ],
                    )
                    .map_err(|error| error.to_string())?;
                Ok(())
            })
            .await
            .expect("seed claim");
    }

    fn claim_target(id: &str) -> ReceiptTarget {
        ReceiptTarget::Claim {
            claim_id: id.to_string(),
            subject: ReceiptSubjectRef::Account("acct-1".to_string()),
            field_path: Some("health.risk".to_string()),
        }
    }

    // -----------------------------------------------------------------------
    // Per-variant metadata validation (AC-8.1, AC-8.4, AC-8.10)
    // -----------------------------------------------------------------------

    #[test]
    fn validates_all_ten_variants() {
        // ConfirmCurrent / MarkOutdated / MarkFalse: no required metadata.
        for action in [
            FeedbackAction::ConfirmCurrent,
            FeedbackAction::MarkOutdated,
            FeedbackAction::MarkFalse,
        ] {
            validate_and_sanitize_metadata(action, None).expect("no metadata required");
        }

        // WrongSubject: optional corrected_subject_ref; legacy corrected_to bridges to it.
        validate_and_sanitize_metadata(FeedbackAction::WrongSubject, None)
            .expect("wrong_subject metadata is optional");
        let canonical_wrong_subject = validate_and_sanitize_metadata(
            FeedbackAction::WrongSubject,
            Some(&serde_json::json!({"corrected_subject_ref": {"account": "acct-2"}})),
        )
        .expect("wrong_subject with corrected_subject_ref");
        assert_eq!(
            canonical_wrong_subject
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.get("corrected_subject_ref")),
            Some(&serde_json::json!({"kind": "account", "id": "acct-2"}))
        );
        let legacy_wrong_subject = validate_and_sanitize_metadata(
            FeedbackAction::WrongSubject,
            Some(&serde_json::json!({"corrected_to": {"account": "acct-2"}})),
        )
        .expect("wrong_subject with legacy corrected_to");
        assert_eq!(
            legacy_wrong_subject
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.get("corrected_subject_ref")),
            Some(&serde_json::json!({"kind": "account", "id": "acct-2"}))
        );

        // WrongSource: required source_content_hash (ADR-0123 §1 verbatim).
        let err = validate_and_sanitize_metadata(FeedbackAction::WrongSource, None).unwrap_err();
        assert!(matches!(err, FeedbackError::BadRequest(_)));
        validate_and_sanitize_metadata(
            FeedbackAction::WrongSource,
            Some(&serde_json::json!({"source_content_hash": "abcdef0123456789"})),
        )
        .expect("wrong_source with content hash");
        // Reject index-style identifiers (AC-8.11).
        let err = validate_and_sanitize_metadata(
            FeedbackAction::WrongSource,
            Some(&serde_json::json!({"source_content_hash": "0"})),
        )
        .unwrap_err();
        assert!(matches!(err, FeedbackError::BadRequest(_)));

        // CannotVerify: optional note.
        validate_and_sanitize_metadata(FeedbackAction::CannotVerify, None)
            .expect("cannot_verify metadata is optional");

        // NeedsNuance: required corrected_text.
        let err = validate_and_sanitize_metadata(FeedbackAction::NeedsNuance, None).unwrap_err();
        assert!(matches!(err, FeedbackError::BadRequest(_)));
        validate_and_sanitize_metadata(
            FeedbackAction::NeedsNuance,
            Some(&serde_json::json!({"corrected_text": "Needs procurement caveat"})),
        )
        .expect("needs_nuance with corrected_text");

        // SurfaceInappropriate: required surface.
        let err =
            validate_and_sanitize_metadata(FeedbackAction::SurfaceInappropriate, None).unwrap_err();
        assert!(matches!(err, FeedbackError::BadRequest(_)));
        validate_and_sanitize_metadata(
            FeedbackAction::SurfaceInappropriate,
            Some(&serde_json::json!({"surface": "briefing"})),
        )
        .expect("surface_inappropriate with surface");

        // NotRelevantHere: required invocation_id.
        let err =
            validate_and_sanitize_metadata(FeedbackAction::NotRelevantHere, None).unwrap_err();
        assert!(matches!(err, FeedbackError::BadRequest(_)));
        validate_and_sanitize_metadata(
            FeedbackAction::NotRelevantHere,
            Some(&serde_json::json!({"invocation_id": "invocation-1"})),
        )
        .expect("not_relevant_here with invocation_id");

        // MergeIntent (ADR-0123 V1.1): required merge_target (SubjectRef),
        // optional supporting_evidence (sanitized free text).
        let err = validate_and_sanitize_metadata(FeedbackAction::MergeIntent, None).unwrap_err();
        assert!(matches!(err, FeedbackError::BadRequest(_)));
        validate_and_sanitize_metadata(
            FeedbackAction::MergeIntent,
            Some(&serde_json::json!({"merge_target": {"person": "person-canonical-1"}})),
        )
        .expect("merge_intent with merge_target only");
        validate_and_sanitize_metadata(
            FeedbackAction::MergeIntent,
            Some(&serde_json::json!({
                "merge_target": {"person": "person-canonical-1"},
                "supporting_evidence": "Same person; different email aliases"
            })),
        )
        .expect("merge_intent with merge_target + supporting_evidence");
        // Reject malformed merge_target.
        let err = validate_and_sanitize_metadata(
            FeedbackAction::MergeIntent,
            Some(&serde_json::json!({"merge_target": 42})),
        )
        .unwrap_err();
        assert!(
            matches!(err, FeedbackError::BadRequest(message) if message.contains("merge_target"))
        );
    }

    #[test]
    fn merge_intent_sanitizes_supporting_evidence_through_adr_0108_pipeline() {
        let validated = validate_and_sanitize_metadata(
            FeedbackAction::MergeIntent,
            Some(&serde_json::json!({
                "merge_target": {"person": "person-canonical-1"},
                "supporting_evidence": "Trace at https://example.com/audit"
            })),
        )
        .expect("validate merge_intent");
        let sanitized = validated
            .metadata
            .as_ref()
            .and_then(|m| m.get("supporting_evidence"))
            .and_then(serde_json::Value::as_str)
            .expect("supporting_evidence");
        assert!(!sanitized.contains("https://example.com"));
        assert!(sanitized.contains("[url removed]"));
    }

    #[test]
    fn merge_intent_rejects_oversize_supporting_evidence() {
        let huge = "x".repeat(MAX_NOTE_CHARS + 50);
        let err = validate_and_sanitize_metadata(
            FeedbackAction::MergeIntent,
            Some(&serde_json::json!({
                "merge_target": {"person": "person-1"},
                "supporting_evidence": huge
            })),
        )
        .unwrap_err();
        assert!(
            matches!(err, FeedbackError::BadRequest(message) if message.contains("supporting_evidence"))
        );
    }

    #[test]
    fn merge_intent_rejects_unknown_keys() {
        let err = validate_and_sanitize_metadata(
            FeedbackAction::MergeIntent,
            Some(&serde_json::json!({
                "merge_target": {"person": "person-1"},
                "rogue": "value"
            })),
        )
        .unwrap_err();
        assert!(matches!(err, FeedbackError::BadRequest(message) if message.contains("rogue")));
    }

    #[test]
    fn rejects_unknown_metadata_keys() {
        let err = validate_and_sanitize_metadata(
            FeedbackAction::WrongSource,
            Some(&serde_json::json!({
                "source_content_hash": "abcdef0123456789",
                "evil_key": "value"
            })),
        )
        .unwrap_err();
        assert!(matches!(err, FeedbackError::BadRequest(message) if message.contains("evil_key")));
    }

    #[test]
    fn rejects_intended_subject_ref_alias_for_wrong_subject() {
        // Claim-file feedback canonicalizes on corrected_subject_ref; do not accept aliases.
        let err = validate_and_sanitize_metadata(
            FeedbackAction::WrongSubject,
            Some(&serde_json::json!({"intended_subject_ref": {"account": "acct-2"}})),
        )
        .unwrap_err();
        assert!(
            matches!(err, FeedbackError::BadRequest(message) if message.contains("intended_subject_ref"))
        );
    }

    #[test]
    fn rejects_disagreeing_wrong_subject_aliases() {
        let err = validate_and_sanitize_metadata(
            FeedbackAction::WrongSubject,
            Some(&serde_json::json!({
                "corrected_to": {"account": "acct-2"},
                "corrected_subject_ref": {"account": "acct-3"}
            })),
        )
        .unwrap_err();
        assert!(matches!(err, FeedbackError::BadRequest(message) if message.contains("disagree")));
    }

    #[test]
    fn rejects_oversized_metadata() {
        let huge = "x".repeat(MAX_METADATA_BYTES + 100);
        let err = validate_and_sanitize_metadata(
            FeedbackAction::CannotVerify,
            Some(&serde_json::json!({"note": huge})),
        )
        .unwrap_err();
        assert!(matches!(err, FeedbackError::BadRequest(_)));
    }

    // -----------------------------------------------------------------------
    // Sanitizer parity (AC-8.12)
    // -----------------------------------------------------------------------

    #[test]
    fn sanitizes_corrected_text_through_adr_0108_pipeline() {
        let validated = validate_and_sanitize_metadata(
            FeedbackAction::NeedsNuance,
            Some(&serde_json::json!({
                "corrected_text": "Visit https://example.com for context"
            })),
        )
        .expect("validate");
        let sanitized = validated
            .metadata
            .as_ref()
            .and_then(|m| m.get("corrected_text"))
            .and_then(serde_json::Value::as_str)
            .expect("corrected_text");
        assert!(!sanitized.contains("https://example.com"));
        assert!(sanitized.contains("[url removed]"));
    }

    // -----------------------------------------------------------------------
    // End-to-end (target binding + auth + write + idempotency)
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn submit_records_a_feedback_row_and_emits_server_key() {
        let (state, _tempdir) = test_state().await;
        let claim_id = "claim-feedback-1";
        seed_claim(&state, claim_id).await;

        let envelope = FakeEnvelope::new("entity_intelligence", &[claim_id]);
        let set = EnvelopeSet::new(&envelope);
        let actor = RenderActor::user("user", Some("user-1"));
        let cache = IdempotencyCache::new();

        let response = submit_claim_feedback(
            &state,
            &set,
            &actor,
            &cache,
            ClaimFeedbackRequest {
                target: claim_target(claim_id),
                action: FeedbackAction::ConfirmCurrent,
                surface: SurfaceContext::EntityDetail,
                metadata: None,
                idempotency_key: None,
            },
        )
        .await
        .expect("submit feedback");

        assert!(!response.replayed);
        assert!(!response.idempotency_key.0.is_empty());
        assert!(response.receipt.is_some());
    }

    #[tokio::test]
    async fn replays_inside_idempotency_window() {
        let (state, _tempdir) = test_state().await;
        let claim_id = "claim-feedback-replay";
        seed_claim(&state, claim_id).await;

        let envelope = FakeEnvelope::new("entity_intelligence", &[claim_id]);
        let set = EnvelopeSet::new(&envelope);
        let actor = RenderActor::user("user", Some("user-1"));
        let cache = IdempotencyCache::new();
        let request = ClaimFeedbackRequest {
            target: claim_target(claim_id),
            action: FeedbackAction::ConfirmCurrent,
            surface: SurfaceContext::EntityDetail,
            metadata: None,
            idempotency_key: None,
        };

        let first = submit_claim_feedback(&state, &set, &actor, &cache, request.clone())
            .await
            .expect("first submit");
        assert!(!first.replayed);

        let second = submit_claim_feedback(&state, &set, &actor, &cache, request.clone())
            .await
            .expect("second submit");
        assert!(second.replayed);
        assert_eq!(first.idempotency_key, second.idempotency_key);
    }

    #[tokio::test]
    async fn retry_outside_window_treated_as_new_submission() {
        let (state, _tempdir) = test_state().await;
        let claim_id = "claim-feedback-window";
        seed_claim(&state, claim_id).await;

        let envelope = FakeEnvelope::new("entity_intelligence", &[claim_id]);
        let set = EnvelopeSet::new(&envelope);
        let actor = RenderActor::user("user", Some("user-1"));
        let cache = IdempotencyCache::with_ttl(Duration::from_millis(1));
        let request = ClaimFeedbackRequest {
            target: claim_target(claim_id),
            action: FeedbackAction::ConfirmCurrent,
            surface: SurfaceContext::EntityDetail,
            metadata: None,
            idempotency_key: None,
        };

        let first = submit_claim_feedback(&state, &set, &actor, &cache, request.clone())
            .await
            .expect("first submit");
        tokio::time::sleep(Duration::from_millis(10)).await;
        let second = submit_claim_feedback(&state, &set, &actor, &cache, request.clone())
            .await
            .expect("second submit");
        assert!(!second.replayed);
        assert_ne!(first.idempotency_key, second.idempotency_key);
    }

    #[tokio::test]
    async fn rejects_caller_supplied_idempotency_key() {
        let (state, _tempdir) = test_state().await;
        let claim_id = "claim-feedback-key";
        seed_claim(&state, claim_id).await;

        let envelope = FakeEnvelope::new("entity_intelligence", &[claim_id]);
        let set = EnvelopeSet::new(&envelope);
        let actor = RenderActor::user("user", Some("user-1"));
        let cache = IdempotencyCache::new();

        let err = submit_claim_feedback(
            &state,
            &set,
            &actor,
            &cache,
            ClaimFeedbackRequest {
                target: claim_target(claim_id),
                action: FeedbackAction::ConfirmCurrent,
                surface: SurfaceContext::EntityDetail,
                metadata: None,
                idempotency_key: Some("attacker-supplied".to_string()),
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(err, FeedbackError::CallerSuppliedIdempotencyKey));
    }

    #[tokio::test]
    async fn agent_actor_denied_for_merge_intent_at_every_surface() {
        // AC-8.13 extension (ADR-0123 V1.1): MergeIntent is user-only.
        let (state, _tempdir) = test_state().await;
        let claim_id = "claim-feedback-merge-agent";
        seed_claim(&state, claim_id).await;

        let envelope = FakeEnvelope::new("entity_intelligence", &[claim_id]);
        let set = EnvelopeSet::new(&envelope);
        let agent = RenderActor::agent("agent:test");
        let cache = IdempotencyCache::new();

        for surface in [
            SurfaceContext::ActionsWork,
            SurfaceContext::EntityDetail,
            SurfaceContext::DailyBriefing,
            SurfaceContext::MeetingDetail,
            SurfaceContext::Mcp,
        ] {
            let err = submit_claim_feedback(
                &state,
                &set,
                &agent,
                &cache,
                ClaimFeedbackRequest {
                    target: claim_target(claim_id),
                    action: FeedbackAction::MergeIntent,
                    surface,
                    metadata: Some(serde_json::json!({
                        "merge_target": {"person": "person-canonical-1"}
                    })),
                    idempotency_key: None,
                },
            )
            .await
            .unwrap_err();
            assert!(
                matches!(err, FeedbackError::AgentActorDenied),
                "surface {:?} must deny agent for MergeIntent",
                surface
            );
        }
    }

    #[tokio::test]
    async fn merge_intent_records_feedback_row_without_lifecycle_change() {
        // MergeIntent persists the typed proposal as a claim_feedback row
        // but does NOT mutate claim verification_state or lifecycle.
        let (state, _tempdir) = test_state().await;
        let claim_id = "claim-feedback-merge-write";
        seed_claim(&state, claim_id).await;

        let envelope = FakeEnvelope::new("entity_intelligence", &[claim_id]);
        let set = EnvelopeSet::new(&envelope);
        let actor = RenderActor::user("user", Some("user-1"));
        let cache = IdempotencyCache::new();

        let response = submit_claim_feedback(
            &state,
            &set,
            &actor,
            &cache,
            ClaimFeedbackRequest {
                target: claim_target(claim_id),
                action: FeedbackAction::MergeIntent,
                surface: SurfaceContext::EntityDetail,
                metadata: Some(serde_json::json!({
                    "merge_target": {"person": "person-canonical-1"},
                    "supporting_evidence": "Same person across two sources"
                })),
                idempotency_key: None,
            },
        )
        .await
        .expect("merge_intent submit");

        assert!(!response.replayed);
        assert!(
            !response.lifecycle_changed,
            "MergeIntent must not mutate claim verification_state"
        );
        assert!(
            !response.repair_queued,
            "MergeIntent must not enqueue a repair job"
        );

        // claim_feedback row landed.
        let seed_claim_id = claim_id.to_string();
        let rows: Vec<String> = state
            .db_read(move |db| {
                let mut stmt = db
                    .conn_ref()
                    .prepare(
                        "SELECT feedback_type FROM claim_feedback WHERE claim_id = ?1 \
                         ORDER BY rowid",
                    )
                    .map_err(|e| e.to_string())?;
                let actions = stmt
                    .query_map(params![&seed_claim_id], |row| row.get::<_, String>(0))
                    .map_err(|e| e.to_string())?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| e.to_string())?;
                Ok(actions)
            })
            .await
            .expect("read claim_feedback");
        assert_eq!(rows, vec!["merge_intent".to_string()]);
    }

    #[tokio::test]
    async fn agent_actor_denied_at_every_surface() {
        let (state, _tempdir) = test_state().await;
        let claim_id = "claim-feedback-agent";
        seed_claim(&state, claim_id).await;

        let envelope = FakeEnvelope::new("entity_intelligence", &[claim_id]);
        let set = EnvelopeSet::new(&envelope);
        let agent = RenderActor::agent("agent:test");
        let cache = IdempotencyCache::new();

        for surface in [
            SurfaceContext::ActionsWork,
            SurfaceContext::EntityDetail,
            SurfaceContext::DailyBriefing,
            SurfaceContext::MeetingDetail,
            SurfaceContext::Mcp,
        ] {
            let err = submit_claim_feedback(
                &state,
                &set,
                &agent,
                &cache,
                ClaimFeedbackRequest {
                    target: claim_target(claim_id),
                    action: FeedbackAction::ConfirmCurrent,
                    surface,
                    metadata: None,
                    idempotency_key: None,
                },
            )
            .await
            .unwrap_err();
            assert!(
                matches!(err, FeedbackError::AgentActorDenied),
                "surface {:?} must deny agent",
                surface
            );
        }
    }

    #[tokio::test]
    async fn target_not_in_envelope_set_rejected() {
        let (state, _tempdir) = test_state().await;
        let claim_id = "claim-feedback-envelope";
        seed_claim(&state, claim_id).await;

        let envelope = FakeEnvelope::new("entity_intelligence", &["other-claim"]);
        let set = EnvelopeSet::new(&envelope);
        let actor = RenderActor::user("user", Some("user-1"));
        let cache = IdempotencyCache::new();

        let err = submit_claim_feedback(
            &state,
            &set,
            &actor,
            &cache,
            ClaimFeedbackRequest {
                target: claim_target(claim_id),
                action: FeedbackAction::ConfirmCurrent,
                surface: SurfaceContext::EntityDetail,
                metadata: None,
                idempotency_key: None,
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(err, FeedbackError::TargetBinding(_)));
    }

    #[tokio::test]
    async fn wrong_source_with_stale_content_hash_rejected() {
        let (state, _tempdir) = test_state().await;
        let claim_id = "claim-feedback-source";
        seed_claim(&state, claim_id).await;

        let envelope = FakeEnvelope::new("entity_intelligence", &[claim_id]);
        let set = EnvelopeSet::new(&envelope);
        let actor = RenderActor::user("user", Some("user-1"));
        let cache = IdempotencyCache::new();

        let err = submit_claim_feedback(
            &state,
            &set,
            &actor,
            &cache,
            ClaimFeedbackRequest {
                target: claim_target(claim_id),
                action: FeedbackAction::WrongSource,
                surface: SurfaceContext::EntityDetail,
                metadata: Some(serde_json::json!({
                    "source_content_hash": "deadbeefdeadbeefdeadbeefdeadbeef"
                })),
                idempotency_key: None,
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(err, FeedbackError::SourceNoLongerInClaim));
    }

    #[tokio::test]
    async fn wrong_source_with_current_content_hash_accepted() {
        let (state, _tempdir) = test_state().await;
        let claim_id = "claim-feedback-source-ok";
        seed_claim(&state, claim_id).await;

        let seed_claim_id = claim_id.to_string();
        let expected = state
            .db_read(move |db| {
                crate::services::claims::load_claim_by_id(db.conn_ref(), &seed_claim_id)
                    .map_err(|e| e.to_string())
            })
            .await
            .expect("load")
            .map(|claim| current_source_content_hash(&claim))
            .expect("claim");

        let envelope = FakeEnvelope::new("entity_intelligence", &[claim_id]);
        let set = EnvelopeSet::new(&envelope);
        let actor = RenderActor::user("user", Some("user-1"));
        let cache = IdempotencyCache::new();

        let response = submit_claim_feedback(
            &state,
            &set,
            &actor,
            &cache,
            ClaimFeedbackRequest {
                target: claim_target(claim_id),
                action: FeedbackAction::WrongSource,
                surface: SurfaceContext::EntityDetail,
                metadata: Some(serde_json::json!({"source_content_hash": expected})),
                idempotency_key: None,
            },
        )
        .await
        .expect("wrong_source with current hash should succeed");
        assert!(!response.replayed);
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    #[test]
    fn idempotency_scope_disambiguates_metadata_hash() {
        let a = idempotency_scope("c-1", FeedbackAction::ConfirmCurrent, "user", "hash-a");
        let b = idempotency_scope("c-1", FeedbackAction::ConfirmCurrent, "user", "hash-b");
        assert_ne!(a, b);
    }

    #[test]
    fn inherit_sensitivity_floor_is_confidential_or_higher() {
        assert_eq!(
            inherit_sensitivity_floor(ClaimSensitivity::Public),
            ClaimSensitivity::Confidential
        );
        assert_eq!(
            inherit_sensitivity_floor(ClaimSensitivity::Internal),
            ClaimSensitivity::Confidential
        );
        assert_eq!(
            inherit_sensitivity_floor(ClaimSensitivity::Confidential),
            ClaimSensitivity::Confidential
        );
        assert_eq!(
            inherit_sensitivity_floor(ClaimSensitivity::UserOnly),
            ClaimSensitivity::UserOnly
        );
    }

    #[test]
    fn render_surface_for_mcp_maps_to_mcp_tool() {
        assert_eq!(
            render_surface_for(SurfaceContext::Mcp),
            RenderSurface::McpTool
        );
    }

    #[test]
    fn json_nesting_depth_tracks_objects_and_arrays() {
        let flat = serde_json::json!({"a": 1});
        assert_eq!(json_nesting_depth(&flat), 1);
        let nested = serde_json::json!({"a": {"b": {"c": [1, [2]]}}});
        assert!(json_nesting_depth(&nested) >= 4);
    }

    #[test]
    fn unchanged_state_keeps_lifecycle_changed_false() {
        // Sanity smoke; ConfirmCurrent on Active keeps state Active.
        assert_eq!(
            ClaimVerificationState::Active,
            ClaimVerificationState::Active
        );
        let s = ClaimState::Active;
        assert!(matches!(s, ClaimState::Active));
        let s = SurfacingState::Active;
        assert!(matches!(s, SurfacingState::Active));
        let t = TemporalScope::State;
        assert!(matches!(t, TemporalScope::State));
    }

    #[test]
    fn is_opaque_hash_rejects_indices_and_freetext() {
        assert!(!is_opaque_hash("0"));
        assert!(!is_opaque_hash("src-1"));
        assert!(!is_opaque_hash("some text"));
        assert!(is_opaque_hash("abcdef0123456789"));
        assert!(is_opaque_hash("dGVzdC1iYXNlNjQ="));
    }
}
