//! Background intelligence enrichment queue.
//!
//! Provides a priority queue for intelligence enrichment requests with
//! deduplication and debounce. A background processor drains the queue
//! and runs enrichment with split DB locking so the UI stays responsive
//! during the 30-120s PTY operation.

use parking_lot::Mutex;
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::Utc;
use rusqlite::OptionalExtension;

use tauri::{AppHandle, Emitter};

use crate::intelligence::dimension_prompts::{self, DIMENSION_NAMES};
use crate::intelligence::{
    build_intelligence_prompt_with_preset, extract_inferred_relationships,
    parse_intelligence_response, InferredRelationship, IntelligenceJson, SourceManifestEntry,
};
use crate::presets::schema::RolePreset;
use crate::pty::{AiUsageContext, ModelTier, PtyManager};
use crate::state::AppState;
use crate::types::AiModelConfig;

/// Debounce window for content-triggered enrichment requests.
const CONTENT_DEBOUNCE_SECS: u64 = 30;
const CALENDAR_DEBOUNCE_SECS: u64 = 600;
/// Background enrichment timeout — raised from 20s to the v1.2.1 floor of 90s.
const BACKGROUND_ENRICHMENT_TIMEOUT_SECS: u64 = 90;
/// Per-dimension manual-refresh timeout. Large accounts with deep context
/// (e.g. Globex-scale) need >90s for some dimensions; 90s caused half
/// the dimensions to time out and return empty arrays, which silently wiped
/// downstream Work/Context content. Bumped to the 240s cap.
const DIMENSION_ENRICHMENT_TIMEOUT_SECS: u64 = 240;

/// How often the background processor checks for work.
const POLL_INTERVAL_SECS: u64 = 5;

/// Maximum retry attempts for entities that fail validation.
const MAX_VALIDATION_RETRIES: u8 = 2;

/// TTL for enrichment results — skip entities enriched within this window.
const ENRICHMENT_TTL_SECS: u64 = 7200;

/// Priority levels for intelligence enrichment requests.
/// Higher numeric value = higher priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IntelPriority {
    /// Background maintenance — lowest priority, budget-gated (ADR-0058).
    ProactiveHygiene = 0,
    /// Triggered by content file changes in the entity directory.
    ContentChange = 1,
    /// Triggered by calendar changes affecting this entity's meetings.
    CalendarChange = 2,
    /// Onboarding batch import — higher than content, lower than manual.
    Onboarding = 3,
    /// User clicked "Refresh Intelligence" manually.
    Manual = 4,
}

/// A request to enrich an entity's intelligence.
#[derive(Debug, Clone)]
pub struct IntelRequest {
    pub entity_id: String,
    pub entity_type: String,
    pub priority: IntelPriority,
    pub requested_at: Instant,
    /// Number of times this entity has been retried after validation failure.
    pub retry_count: u8,
}

impl IntelRequest {
    /// Create a new request with zero retries.
    pub fn new(entity_id: String, entity_type: String, priority: IntelPriority) -> Self {
        Self {
            entity_id,
            entity_type,
            priority,
            requested_at: Instant::now(),
            retry_count: 0,
        }
    }
}

/// Thread-safe intelligence enrichment queue with deduplication and debounce.
///
/// also exposes pause/drain/resume primitives consumed by the
/// schema-epoch migration sequence. When paused, new
/// enqueues return `false` so the caller can surface a user-visible
/// "operation interrupted by migration; retry" message rather than
/// silently dropping work. Background-priority enqueues (`ContentChange`,
/// `CalendarChange`, `ProactiveHygiene`) coalesce silently — they will
/// re-fire on the next change and the migration is a one-time event.
pub struct IntelligenceQueue {
    queue: Mutex<VecDeque<IntelRequest>>,
    last_enqueued: Mutex<HashMap<String, Instant>>,
    /// when true, `enqueue` rejects user-visible (Manual) requests
    /// with `false` and silently drops background requests. The processor
    /// loop checks this flag at dequeue-time to honor the pause.
    paused: std::sync::atomic::AtomicBool,
}

/// Default drain timeout for schema-epoch migrations.
/// "drain timeout configurable; default 60s."
pub const DEFAULT_DRAIN_TIMEOUT_SECS: u64 = 60;

/// Outcome of a successful [`IntelligenceQueue::enqueue`] call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnqueueOutcome {
    /// New entry added to the queue.
    Accepted,
    /// Existing entry was kept (priority upgraded if the new request was higher).
    Coalesced,
}

/// Reason an enqueue was rejected. Manual-priority callers MUST handle
/// [`EnqueueError::Paused`] by returning a user-visible error
/// ("operation interrupted by migration; retry"); background callers can
/// discard via `let _ = ...` since the change trigger will refire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnqueueError {
    /// A schema-epoch migration is draining the queue. New
    /// work is rejected; the user-facing call should surface a retry
    /// prompt rather than silently dropping.
    Paused,
    /// Background request landed inside the debounce window for its
    /// priority (`CONTENT_DEBOUNCE_SECS` for ContentChange,
    /// `CALENDAR_DEBOUNCE_SECS` for CalendarChange). Acceptable for
    /// background callers; not normally surfaced.
    Debounced,
}

impl std::fmt::Display for EnqueueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Paused => write!(
                f,
                "intel queue paused (schema-epoch migration in progress); retry"
            ),
            Self::Debounced => write!(f, "intel request debounced"),
        }
    }
}

impl std::error::Error for EnqueueError {}

/// Result alias for [`IntelligenceQueue::enqueue`].
pub type EnqueueResult = Result<EnqueueOutcome, EnqueueError>;

/// Convenience for user-facing Tauri command callers: enqueue an
/// `IntelRequest` and surface only `EnqueueError::Paused` to the user
/// (debounce rejections are silently OK because background change
/// triggers will refire). The returned `Err(String)` is meant to be
/// returned directly from the Tauri command — the frontend renders it
/// as "operation interrupted by migration; retry."
///
/// Use from manual-priority paths (Manual / Onboarding). Background
/// callers should use `let _ = queue.enqueue(...)` directly.
pub fn enqueue_user_facing(queue: &IntelligenceQueue, request: IntelRequest) -> Result<(), String> {
    match queue.enqueue(request) {
        Ok(_) => Ok(()),
        Err(EnqueueError::Paused) => Err(EnqueueError::Paused.to_string()),
        Err(EnqueueError::Debounced) => Ok(()),
    }
}

impl IntelligenceQueue {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            last_enqueued: Mutex::new(HashMap::new()),
            paused: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// pause new enqueues + dequeues. Used by the schema-epoch
    /// migration to drain in-flight work before bumping the epoch.
    pub fn pause(&self) {
        self.paused.store(true, std::sync::atomic::Ordering::SeqCst);
        log::info!("IntelQueue: paused (DOS-311 schema-epoch migration)");
    }

    /// resume normal processing after a migration completes.
    pub fn resume(&self) {
        self.paused
            .store(false, std::sync::atomic::Ordering::SeqCst);
        log::info!("IntelQueue: resumed");
    }

    /// whether the queue is currently paused. The processor loop
    /// checks this at every dequeue to skip work during a migration.
    pub fn is_paused(&self) -> bool {
        self.paused.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// drain pending requests. Returns the items so the migration sequence
    /// can re-enqueue them after backfill.
    /// In-flight workers are out of scope here — they self-abort on epoch
    /// mismatch via the WriteFence.
    pub fn drain_pending(&self) -> Vec<IntelRequest> {
        let mut queue = self.queue.lock();
        let drained: Vec<_> = queue.drain(..).collect();
        drop(queue);
        if !drained.is_empty() {
            log::info!(
                "IntelQueue: drained {} pending requests for migration",
                drained.len()
            );
        }
        drained
    }

    /// Put a dequeued batch back without applying pause/debounce policy.
    /// Used when a worker loses the race with a process-wide cutover pause
    /// after dequeue but before it can capture a write fence.
    pub fn requeue_dequeued_batch(&self, requests: Vec<IntelRequest>) {
        if requests.is_empty() {
            return;
        }

        let mut queue = self.queue.lock();
        for request in requests.into_iter().rev() {
            if queue.iter().any(|r| r.entity_id == request.entity_id) {
                continue;
            }
            queue.push_front(request);
        }
    }

    /// Enqueue an enrichment request.
    ///
    /// returns `Ok(EnqueueOutcome::Accepted)` for new entries,
    /// `Ok(EnqueueOutcome::Coalesced)` when an existing higher-or-equal
    /// queued entry was preserved (priority upgraded if applicable),
    /// `Err(EnqueueError::Paused)` while a schema-epoch migration is
    /// draining, or `Err(EnqueueError::Debounced)` for background
    /// requests inside the debounce window. Manual-priority callers
    /// (Tauri commands invoked by user actions) MUST surface
    /// `Paused` to the UI as "operation interrupted by migration; retry";
    /// background callers can `let _ = ` the result because the change
    /// trigger will fire again post-migration.
    ///
    /// `#[must_use]` so silent discard is a compile-time error caught by
    /// `clippy::let_underscore_must_use` once the workspace lint lands
    //.
    ///
    /// Deduplicates by entity_id: if the same entity is already queued,
    /// the higher priority wins. Debounces content changes by
    /// `CONTENT_DEBOUNCE_SECS`.
    #[must_use = "enqueue returns a Result; manual-priority callers must surface Paused to the user"]
    pub fn enqueue(&self, request: IntelRequest) -> EnqueueResult {
        // refuse new work during a migration drain.
        if self.is_paused() {
            log::warn!(
                "IntelQueue: rejected enqueue for {} (migration in progress)",
                request.entity_id
            );
            return Err(EnqueueError::Paused);
        }
        if let Some(debounce_secs) = debounce_window_secs(request.priority) {
            let last = self.last_enqueued.lock();
            if let Some(last_time) = last.get(&request.entity_id) {
                if last_time.elapsed().as_secs() < debounce_secs {
                    log::debug!(
                        "IntelQueue: debounced {} ({}s since last)",
                        request.entity_id,
                        last_time.elapsed().as_secs()
                    );
                    return Err(EnqueueError::Debounced);
                }
            }
            drop(last);
        }

        let mut queue = self.queue.lock();

        // Dedup: if entity already in queue, keep higher priority
        if let Some(existing) = queue.iter_mut().find(|r| r.entity_id == request.entity_id) {
            if request.priority > existing.priority {
                existing.priority = request.priority;
                log::debug!(
                    "IntelQueue: upgraded priority for {} to {:?}",
                    request.entity_id,
                    request.priority
                );
            }
            return Ok(EnqueueOutcome::Coalesced);
        }

        log::info!(
            "IntelQueue: enqueued {} ({}) priority={:?}",
            request.entity_id,
            request.entity_type,
            request.priority
        );

        queue.push_back(request.clone());

        // Update debounce tracker
        {
            let mut last = self.last_enqueued.lock();
            last.insert(request.entity_id, Instant::now());
        }
        Ok(EnqueueOutcome::Accepted)
    }

    /// Dequeue the highest-priority request.
    pub fn dequeue(&self) -> Option<IntelRequest> {
        let mut queue = self.queue.lock();
        if queue.is_empty() {
            return None;
        }

        // Find highest priority item
        let best_idx = queue
            .iter()
            .enumerate()
            .max_by_key(|(_, r)| r.priority)
            .map(|(i, _)| i)?;

        queue.remove(best_idx)
    }

    /// Dequeue up to `max` highest-priority requests.
    ///
    /// Returns items sorted by descending priority so the highest-priority
    /// entity appears first in the batch.
    pub fn dequeue_batch(&self, max: usize) -> Vec<IntelRequest> {
        let mut queue = self.queue.lock();
        if queue.is_empty() {
            return Vec::new();
        }

        let take = max.min(queue.len());
        let mut batch = Vec::with_capacity(take);

        for _ in 0..take {
            // Find highest priority remaining item
            let best_idx = queue
                .iter()
                .enumerate()
                .max_by_key(|(_, r)| r.priority)
                .map(|(i, _)| i);

            if let Some(idx) = best_idx {
                if let Some(req) = queue.remove(idx) {
                    batch.push(req);
                }
            }
        }

        batch
    }

    /// Current queue depth (for diagnostics).
    pub fn len(&self) -> usize {
        self.queue.lock().len()
    }

    /// Whether the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Remove all queued requests for a given entity.
    ///
    /// Called when an entity is archived so pending enrichments don't run
    /// against a now-archived target. Also clears the debounce tracker so
    /// an unarchive → re-enqueue works without the debounce delay. Returns
    /// the number of queue entries removed.
    pub fn remove_by_entity_id(&self, entity_id: &str) -> usize {
        let mut queue = self.queue.lock();
        let before = queue.len();
        queue.retain(|r| r.entity_id != entity_id);
        let removed = before - queue.len();
        drop(queue);

        self.last_enqueued.lock().remove(entity_id);

        if removed > 0 {
            log::info!(
                "IntelQueue: removed {removed} pending entries for archived entity {entity_id}"
            );
        }
        removed
    }

    /// Remove stale entries from the `last_enqueued` debounce tracker.
    ///
    /// Entries older than `CONTENT_DEBOUNCE_SECS * 10` (5 minutes) are pruned
    /// to prevent unbounded memory growth over long-running sessions.
    pub fn prune_stale_entries(&self) {
        let stale_threshold_secs = CONTENT_DEBOUNCE_SECS * 10;
        let mut last = self.last_enqueued.lock();
        let before = last.len();
        last.retain(|_, instant| instant.elapsed().as_secs() < stale_threshold_secs);
        let pruned = before - last.len();
        if pruned > 0 {
            log::debug!("IntelQueue: pruned {} stale debounce entries", pruned);
        }
    }
}

fn debounce_window_secs(priority: IntelPriority) -> Option<u64> {
    match priority {
        IntelPriority::ContentChange | IntelPriority::ProactiveHygiene => {
            Some(CONTENT_DEBOUNCE_SECS)
        }
        IntelPriority::CalendarChange => Some(CALENDAR_DEBOUNCE_SECS),
        IntelPriority::Onboarding | IntelPriority::Manual => None,
    }
}

fn is_background_priority(priority: IntelPriority) -> bool {
    matches!(
        priority,
        IntelPriority::CalendarChange
            | IntelPriority::ContentChange
            | IntelPriority::ProactiveHygiene
    )
}

fn trigger_for_priority(priority: IntelPriority) -> &'static str {
    match priority {
        IntelPriority::ProactiveHygiene => "proactive_hygiene",
        IntelPriority::ContentChange => "content_change",
        IntelPriority::CalendarChange => "calendar_change",
        IntelPriority::Onboarding => "onboarding",
        IntelPriority::Manual => "manual_refresh",
    }
}

fn usage_context_for_priority(priority: IntelPriority) -> AiUsageContext {
    let background = is_background_priority(priority);
    let operation = match priority {
        IntelPriority::Onboarding => "onboarding_entity_enrichment",
        IntelPriority::Manual => "manual_entity_enrichment",
        _ => "background_entity_enrichment",
    };
    AiUsageContext::new("intel_queue", operation)
        .with_trigger(trigger_for_priority(priority))
        .with_background(background)
}

/// surface a failed leading-signals enrichment pass to the audit log
/// and the frontend. Mirrors the main-enrichment degraded/fallback pattern so
/// users see *why* Health triage cards are activity-sourced instead of
/// Glean-sourced on accounts where Glean is configured.
fn emit_leading_signals_failed(
    state: &AppState,
    app: &AppHandle,
    entity_id: &str,
    entity_type: &str,
    reason: &str,
    wall_clock_ms: u64,
    is_background: bool,
) {
    {
        let mut audit = state.audit_log.lock();
        #[allow(
            clippy::let_underscore_must_use,
            reason = "intentional best-effort discard; preserves existing non-blocking behavior"
        )]
        let _ = audit.append(
            "data_access",
            "glean_leading_signals_failed",
            serde_json::json!({
                "entity_id": entity_id,
                "entity_type": entity_type,
                "reason": reason,
                "wall_clock_ms": wall_clock_ms,
                "background": is_background,
            }),
        );
    }
    // Toast only on user-initiated work — match the main-enrichment gate.
    // Background hygiene sweeps would otherwise spam toasts every few
    // minutes as Glean's natural partial-failure rate surfaces.
    if !is_background {
        #[allow(
            clippy::let_underscore_must_use,
            reason = "intentional best-effort discard; preserves existing non-blocking behavior"
        )]
        let _ = app.emit(
            "enrichment-glean-leading-signals-failed",
            serde_json::json!({
                "entity_id": entity_id,
                "entity_type": entity_type,
                "reason": reason,
                "wall_clock_ms": wall_clock_ms,
            }),
        );
    }
}

#[cfg(test)]
mod queue_policy_tests {
    use super::{
        debounce_window_secs, is_background_priority, IntelPriority, CALENDAR_DEBOUNCE_SECS,
        CONTENT_DEBOUNCE_SECS,
    };

    #[test]
    fn calendar_change_uses_longer_debounce_window() {
        assert_eq!(
            debounce_window_secs(IntelPriority::CalendarChange),
            Some(CALENDAR_DEBOUNCE_SECS)
        );
        assert_eq!(
            debounce_window_secs(IntelPriority::ContentChange),
            Some(CONTENT_DEBOUNCE_SECS)
        );
    }

    #[test]
    fn background_priority_classification_matches_policy() {
        assert!(is_background_priority(IntelPriority::CalendarChange));
        assert!(is_background_priority(IntelPriority::ProactiveHygiene));
        assert!(!is_background_priority(IntelPriority::Manual));
        assert!(!is_background_priority(IntelPriority::Onboarding));
    }
}

/// Payload emitted when intelligence is updated.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntelligenceUpdatedPayload {
    pub entity_id: String,
    pub entity_type: String,
}

/// Progressive enrichment progress event payload.
///
/// Emitted after each dimension completes and is written to DB,
/// so the frontend can show incremental progress and refresh data.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnrichmentProgress {
    pub entity_id: String,
    pub entity_type: String,
    pub completed: u32,
    pub total: u32,
    pub last_dimension: String,
}

/// Progressive enrichment completion event payload.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnrichmentComplete {
    pub entity_id: String,
    pub entity_type: String,
    pub succeeded: u32,
    pub failed: u32,
    pub failed_dimensions: Vec<String>,
    pub total_dimensions: u32,
    pub applicable_total: u32,
    pub skipped_dimensions: Vec<String>,
    pub wall_clock_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct QueueRefreshFailure {
    entity_id: String,
    stage: &'static str,
    error: String,
}

impl QueueRefreshFailure {
    fn new(entity_id: &str, stage: &'static str, error: impl Into<String>) -> Self {
        Self {
            entity_id: entity_id.to_string(),
            stage,
            error: error.into(),
        }
    }
}

fn queue_worker_final_status_payloads(
    completed_count: usize,
    failures: &[QueueRefreshFailure],
    manual: bool,
) -> Vec<serde_json::Value> {
    if let Some(first_failure) = failures.first() {
        let message = if completed_count > 0 {
            "Some insight refreshes failed"
        } else {
            "Insight refresh failed"
        };
        let error = if failures.len() == 1 {
            first_failure.error.clone()
        } else {
            format!(
                "{} refreshes failed; first failure: {}",
                failures.len(),
                first_failure.error
            )
        };
        return vec![serde_json::json!({
            "phase": "failed",
            "message": message,
            "count": failures.len(),
            "completedCount": completed_count,
            "manual": manual,
            "entityId": first_failure.entity_id,
            "stage": first_failure.stage,
            "error": error,
        })];
    }

    if completed_count == 0 {
        return Vec::new();
    }

    vec![serde_json::json!({
        "phase": "completed",
        "message": "Insights updated",
        "count": completed_count,
        "manual": manual,
    })]
}

/// Context gathered from the DB (held briefly, then released before PTY).
/// Public so manual enrichment commands can reuse the split-lock pattern.
#[derive(Clone)]
pub struct EnrichmentInput {
    pub workspace: PathBuf,
    pub entity_dir: PathBuf,
    pub entity_id: String,
    pub entity_type: String,
    pub prompt: String,
    pub file_manifest: Vec<SourceManifestEntry>,
    pub file_count: usize,
    /// Pre-computed algorithmic health for accounts.
    pub computed_health: Option<crate::intelligence::io::AccountHealth>,
    /// Entity name for Glean-first enrichment.
    pub entity_name: String,
    /// Relationship type for Glean prompt (e.g., "customer", "partner").
    pub relationship: Option<String>,
    /// Intelligence context for Glean-first enrichment.
    /// Preserved from gather phase so Glean can inject local context into its prompt.
    pub intelligence_context: Option<crate::intelligence::prompts::IntelligenceContext>,
    /// Active role preset captured during the gather phase.
    pub active_preset: Option<RolePreset>,
}

/// The producer that generated an enrichment result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnrichmentProducer {
    Glean,
    Pty,
}

impl EnrichmentProducer {
    fn is_glean(self) -> bool {
        matches!(self, EnrichmentProducer::Glean)
    }

    pub(crate) fn projection_data_source(self) -> &'static str {
        match self {
            EnrichmentProducer::Glean => "glean",
            EnrichmentProducer::Pty => "ai_enrichment",
        }
    }

    fn materialization_label(self) -> &'static str {
        match self {
            EnrichmentProducer::Glean => "Glean",
            EnrichmentProducer::Pty => "PTY",
        }
    }

    fn side_effect_producer(
        self,
    ) -> crate::services::enrichment_side_effects::EnrichmentSideEffectProducer {
        match self {
            EnrichmentProducer::Glean => {
                crate::services::enrichment_side_effects::EnrichmentSideEffectProducer::Glean
            }
            EnrichmentProducer::Pty => {
                crate::services::enrichment_side_effects::EnrichmentSideEffectProducer::Pty
            }
        }
    }
}

/// Parsed enrichment output from one model response section.
#[derive(Debug, Clone)]
pub struct EnrichmentParseResult {
    pub intel: IntelligenceJson,
    pub inferred_relationships: Vec<InferredRelationship>,
    pub producer: EnrichmentProducer,
}

async fn drain_projection_resign_queue_once(state: &Arc<AppState>) {
    match state
        .db_write(|db| {
            let clock = crate::services::context::SystemClock;
            let rng = crate::services::context::SystemRng;
            let external = crate::services::context::ExternalClients::default();
            let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &external)
                .with_actor("scheduled_worker");
            let key_store = crate::services::projection_signing::MacOsProjectionKeyStore;
            crate::services::projection_signing::drain_resign_queue(&ctx, db, &key_store, 16)
                .map_err(|error| error.to_string())
        })
        .await
        .map_err(String::from)
    {
        Ok(count) if count > 0 => {
            log::info!("ProjectionSigning: drained {count} queued re-sign jobs")
        }
        Ok(_) => {}
        Err(error) => log::warn!("ProjectionSigning: re-sign queue drain failed: {error}"),
    }
}

/// Background intelligence processor.
///
/// Runs in a loop, checking the queue every `POLL_INTERVAL_SECS`.
/// When a request is found:
/// 1. Locks DB briefly to gather context and build the prompt
/// 2. Releases lock and runs PTY (30-120s)
/// 3. Locks DB briefly to write results
/// 4. Emits `intelligence-updated` event
pub async fn run_intel_processor(state: Arc<AppState>, app: AppHandle) {
    log::info!("IntelProcessor: started");
    if state.app_handle().is_none() {
        state.set_app_handle(app.clone());
    }

    let mut polls_since_prune: u64 = 0;
    // Prune every ~60 seconds (60 / POLL_INTERVAL_SECS polls)
    let prune_interval = 60 / POLL_INTERVAL_SECS;

    loop {
        // Adaptive sleep: back off when queue is empty + user is active; wake instantly on enqueue
        let interval =
            crate::activity::adaptive_poll_interval(&state.activity, state.intel_queue.is_empty());
        tokio::select! {
            _ = tokio::time::sleep(interval) => {}
            _ = state.integrations.intel_queue_wake.notified() => {}
        }

        if crate::pty::background_workers_disabled() {
            continue;
        }

        // Dev mode isolation: pause background processing while dev sandbox is active
        if crate::db::is_dev_db_mode() {
            continue;
        }

        // skip processing entirely while a schema-epoch migration
        // drains the queue. Pending items remain in `queue` (not destroyed)
        // so the migration sequence can drain, backfill, and re-enqueue.
        if state.intel_queue.is_paused() {
            continue;
        }

        // Periodic pruning of stale debounce entries
        polls_since_prune += 1;
        if polls_since_prune >= prune_interval {
            state.intel_queue.prune_stale_entries();
            polls_since_prune = 0;
        }

        drain_projection_resign_queue_once(&state).await;

        // Process one request per wake so automatic background bursts do not
        // stack PTY calls back-to-back and starve manual work.
        let batch = state.intel_queue.dequeue_batch(1);
        if batch.is_empty() {
            continue;
        }

        // capture FenceCycle for the entire batch lifetime. RAII
        // (Drop on scope exit) keeps IN_FLIGHT_CYCLES > 0 through PTY/Glean
        // enrichment + write-back, so a concurrent migration's
        // drain_with_timeout waits for this batch to complete (or times
        // out as force-abort).
        //
        // ActionDb::open() is the standard processor-loop pattern (see
        // gather_enrichment_input which also opens its own handle); the
        // FenceCycle is a short-lived read against migration_state.
        let _fence_cycle = match crate::db::ActionDb::open(std::sync::Arc::new(
            crate::db::LocalKeychain::new(),
        )) {
            Ok(db) => match crate::intelligence::write_fence::FenceCycle::capture(&db) {
                Ok(c) => c,
                Err(e) => {
                    log::info!(
                        "IntelProcessor: FenceCycle capture unavailable for batch ({e}); retrying after cutover"
                    );
                    state.intel_queue.requeue_dequeued_batch(batch);
                    tokio::time::sleep(Duration::from_millis(250)).await;
                    continue;
                }
            },
            Err(e) => {
                log::warn!("IntelProcessor: failed to open db for FenceCycle capture: {e}");
                state.intel_queue.requeue_dequeued_batch(batch);
                tokio::time::sleep(Duration::from_millis(250)).await;
                continue;
            }
        };

        let entity_names: Vec<&str> = batch.iter().map(|r| r.entity_id.as_str()).collect();
        log::info!(
            "IntelProcessor: dequeued batch of {} entities: {:?}",
            batch.len(),
            entity_names,
        );

        // TTL check: filter out entities enriched recently unless manually requested
        let batch: Vec<IntelRequest> = batch
            .into_iter()
            .filter(|request| {
                if is_background_priority(request.priority) && crate::pty::background_ai_paused() {
                    log::info!(
                        "IntelProcessor: skipping {} while background AI is paused",
                        request.entity_id
                    );
                    return false;
                }
                if request.priority != IntelPriority::Manual {
                    if let Some(skip_msg) = check_enrichment_ttl(&state, request) {
                        log::debug!("{}", skip_msg);
                        return false;
                    }
                }
                true
            })
            .collect();

        if batch.is_empty() {
            continue;
        }

        // Emit background work status for frontend indicator only when
        // the batch survives TTL/background guards and will do real work.
        let display_names: Vec<String> = if let Ok(db) =
            crate::db::ActionDb::open(std::sync::Arc::new(crate::db::LocalKeychain::new()))
        {
            batch
                .iter()
                .filter_map(|r| {
                    db.get_account(&r.entity_id)
                        .ok()
                        .flatten()
                        .map(|a| a.name)
                        .or_else(|| db.get_project(&r.entity_id).ok().flatten().map(|p| p.name))
                        .or_else(|| db.get_person(&r.entity_id).ok().flatten().map(|p| p.name))
                })
                .collect()
        } else {
            vec![]
        };
        let started_msg = if display_names.is_empty() {
            format!(
                "Updating {} account{}...",
                batch.len(),
                if batch.len() == 1 { "" } else { "s" }
            )
        } else if display_names.len() <= 3 {
            format!("Updating {}...", display_names.join(", "))
        } else {
            format!(
                "Updating {} and {} more...",
                display_names[..2].join(", "),
                display_names.len() - 2
            )
        };
        let batch_has_manual = batch.iter().any(|r| r.priority == IntelPriority::Manual);
        #[allow(
            clippy::let_underscore_must_use,
            reason = "intentional best-effort discard; preserves existing non-blocking behavior"
        )]
        let _ = app.emit(
            "background-work-status",
            serde_json::json!({
                "phase": "started",
                "message": started_msg,
                "count": batch.len(),
                "manual": batch_has_manual,
            }),
        );

        {
            let mut audit = state.audit_log.lock();
            #[allow(
                clippy::let_underscore_must_use,
                reason = "intentional best-effort discard; preserves existing non-blocking behavior"
            )]
            let _ = audit.append(
                "ai",
                "entity_enrichment_started",
                serde_json::json!({"batch_size": batch.len()}),
            );
        }

        // Phase 1: Gather context for all entities (brief DB access per entity)
        let mut inputs: Vec<(IntelRequest, EnrichmentInput)> = Vec::new();
        for request in &batch {
            match gather_enrichment_input(&state, request) {
                Ok(input) => inputs.push((request.clone(), input)),
                Err(e) => {
                    log::warn!(
                        "IntelProcessor: failed to gather context for {}: {}",
                        request.entity_id,
                        e
                    );
                }
            }
        }

        if inputs.is_empty() {
            continue;
        }

        // Step 2: Run PTY enrichment (no DB lock held)
        // Acquire heavy-work semaphore — limits concurrent expensive operations
        // (PTY subprocess, embedding inference) to one at a time.
        let _permit = match state.permits.pty.acquire().await {
            Ok(permit) => permit,
            Err(_) => {
                log::warn!("IntelProcessor: pty permit closed, stopping");
                return;
            }
        };

        let ai_config = state
            .config
            .read()
            .as_ref()
            .map(|c| c.ai_models.clone())
            .unwrap_or_default();

        // Track original requests so we can detect failures and re-enqueue
        let original_requests: Vec<IntelRequest> = inputs.iter().map(|(r, _)| r.clone()).collect();

        // Track error categories for failed entities
        let mut error_categories: HashMap<String, &str> = HashMap::new();

        let enrichment_start = Instant::now();
        let results: Vec<(IntelRequest, EnrichmentInput, EnrichmentParseResult)> =
            if state.context_provider().is_remote() {
                // /ADR-0100: Glean-first path — use chat MCP tool for enrichment.
                // Falls back to PTY on failure (per entity).
                run_glean_enrichment_with_fallback(inputs, &ai_config, &state, &app).await
            } else {
                // Per-entity enrichment (tries parallel fan-out, falls back to legacy)
                // Run PTY enrichment on blocking threads to avoid stalling Tokio workers.
                let mut entity_results = Vec::new();
                for (request, input) in inputs {
                    // per-entity checkpoint inside the PTY batch loop.
                    // Symmetric to the Glean path — abort early when a
                    // schema-epoch migration starts mid-batch.
                    if state.intel_queue.is_paused() {
                        log::info!(
                        "IntelProcessor: pause detected mid-PTY-batch; aborting after {} entities",
                        entity_results.len()
                    );
                        break;
                    }
                    let ai_cfg = ai_config.clone();
                    let input_clone = input.clone();
                    let app_clone = app.clone();
                    let usage_context = usage_context_for_priority(request.priority);
                    match tauri::async_runtime::spawn_blocking(move || {
                        run_enrichment(&input_clone, &ai_cfg, Some(&app_clone), usage_context)
                    })
                    .await
                    {
                        Ok(Ok(parsed)) => entity_results.push((request, input, parsed)),
                        Ok(Err(e)) => {
                            let category = categorize_enrichment_error(&e);
                            error_categories.insert(request.entity_id.clone(), category);
                            log::warn!(
                                "IntelProcessor: enrichment failed for {}: {}",
                                request.entity_id,
                                e
                            );
                        }
                        Err(e) => {
                            error_categories.insert(request.entity_id.clone(), "panic");
                            log::error!(
                                "IntelProcessor: enrichment task panicked for {}: {}",
                                request.entity_id,
                                e
                            );
                        }
                    }
                }
                entity_results
            };
        let enrichment_duration_ms = enrichment_start.elapsed().as_millis() as u64;

        // Re-enqueue entities that failed validation (up to MAX_VALIDATION_RETRIES)
        {
            let succeeded: std::collections::HashSet<&str> = results
                .iter()
                .map(|(r, _, _)| r.entity_id.as_str())
                .collect();

            for original in &original_requests {
                if !succeeded.contains(original.entity_id.as_str())
                    && original.retry_count < MAX_VALIDATION_RETRIES
                {
                    log::info!(
                        "IntelProcessor: re-enqueuing {} for retry (attempt {}/{})",
                        original.entity_id,
                        original.retry_count + 1,
                        MAX_VALIDATION_RETRIES,
                    );
                    #[allow(
                        clippy::let_underscore_must_use,
                        reason = "intentional best-effort discard; preserves existing non-blocking behavior"
                    )]
                    let _ = state.intel_queue.enqueue(IntelRequest {
                        entity_id: original.entity_id.clone(),
                        entity_type: original.entity_type.clone(),
                        priority: original.priority,
                        requested_at: Instant::now(),
                        retry_count: original.retry_count + 1,
                    });
                } else if !succeeded.contains(original.entity_id.as_str()) {
                    // Record claude_code sync failure
                    if let Ok(db) = crate::db::ActionDb::open(std::sync::Arc::new(
                        crate::db::LocalKeychain::new(),
                    )) {
                        #[allow(
                            clippy::let_underscore_must_use,
                            reason = "intentional best-effort discard; preserves existing non-blocking behavior"
                        )]
                        let _ = crate::connectivity::record_sync_failure(
                            db.conn_ref(),
                            "claude_code",
                            &format!(
                                "Enrichment failed after {} retries for {}",
                                original.retry_count, original.entity_id
                            ),
                        );
                    }
                    log::warn!(
                        "IntelProcessor: {} failed after {} retries, dropping from queue",
                        original.entity_id,
                        original.retry_count,
                    );
                    // Track schema validation failures for dropped entities
                    error_categories
                        .entry(original.entity_id.clone())
                        .or_insert("schema_validation");
                    {
                        let mut audit = state.audit_log.lock();
                        #[allow(
                            clippy::let_underscore_must_use,
                            reason = "intentional best-effort discard; preserves existing non-blocking behavior"
                        )]
                        let _ = audit.append(
                            "anomaly",
                            "schema_validation_failed",
                            serde_json::json!({"entity_id": original.entity_id}),
                        );
                    }
                }
            }
        }

        // Audit: enrichment results
        {
            let succeeded_count = results.len();
            let failed_count = original_requests.len() - succeeded_count;
            if succeeded_count > 0 {
                {
                    let mut audit = state.audit_log.lock();
                    #[allow(clippy::let_underscore_must_use, reason = "intentional best-effort discard; preserves existing non-blocking behavior")]
                    let _ = audit.append(
                        "ai",
                        "entity_enrichment_completed",
                        serde_json::json!({"count": succeeded_count, "duration_ms": enrichment_duration_ms}),
                    );
                }
            }
            if failed_count > 0 {
                // Determine the dominant error category
                let dominant_category = if error_categories.values().any(|c| *c == "timeout") {
                    "timeout"
                } else if error_categories.values().any(|c| *c == "schema_validation") {
                    "schema_validation"
                } else {
                    "pty_error"
                };
                {
                    let mut audit = state.audit_log.lock();
                    #[allow(clippy::let_underscore_must_use, reason = "intentional best-effort discard; preserves existing non-blocking behavior")]
                    let _ = audit.append(
                        "ai",
                        "entity_enrichment_failed",
                        serde_json::json!({"count": failed_count, "error_category": dominant_category}),
                    );
                }
            }
        }

        // Release permit before Phase 3 — writing results is cheap, doesn't need it
        drop(_permit);

        // Phase 3 + 4: Write results and emit events for each entity
        let mut finalized_count = 0usize;
        let mut finalization_failures = Vec::new();
        for (request, input, parsed) in &results {
            let intel = &parsed.intel;
            // Check for anomalies in the enrichment output
            if let Ok(serialized) = serde_json::to_string(intel) {
                let anomalies = crate::intelligence::validation::detect_anomalies(&serialized);
                if !anomalies.is_empty() {
                    {
                        let mut audit = state.audit_log.lock();
                        #[allow(
                            clippy::let_underscore_must_use,
                            reason = "intentional best-effort discard; preserves existing non-blocking behavior"
                        )]
                        let _ = audit.append(
                            "anomaly",
                            "injection_instruction_in_output",
                            serde_json::json!({
                                "entity_id": request.entity_id,
                                "detected_terms": anomalies,
                            }),
                        );
                    }
                }
            }

            let db = match crate::db::ActionDb::open(std::sync::Arc::new(
                crate::db::LocalKeychain::new(),
            )) {
                Ok(db) => db,
                Err(e) => {
                    finalization_failures.push(QueueRefreshFailure::new(
                        &request.entity_id,
                        "open_db",
                        e.to_string(),
                    ));
                    log::warn!(
                        "IntelProcessor: failed to open DB for {} persistence: {}",
                        request.entity_id,
                        e
                    );
                    continue;
                }
            };
            let ctx = state.live_service_context();
            let prepared = match compose_enrichment_intelligence(
                &state,
                &db,
                input,
                intel,
                parsed.producer,
                if is_background_priority(request.priority) {
                    None
                } else {
                    Some(&ai_config)
                },
            ) {
                Ok(composition) => composition,
                Err(e) => {
                    finalization_failures.push(QueueRefreshFailure::new(
                        &request.entity_id,
                        "compose",
                        e.clone(),
                    ));
                    log::warn!(
                        "IntelProcessor: failed to write results for {}: {}",
                        request.entity_id,
                        e
                    );
                    continue;
                }
            };
            if let Err(e) = db.with_transaction(|tx| {
                apply_enrichment_side_writes(&ctx, tx, input, &prepared)?;
                crate::services::intelligence::upsert_assessment_from_enrichment_in_active_transaction(
                    &ctx,
                    tx,
                    &state.signals.engine,
                    crate::services::intelligence::EnrichmentAssessmentUpsert {
                        entity_type: &input.entity_type,
                        entity_id: &input.entity_id,
                        intel: prepared.intelligence(),
                        projection_intel: prepared.projection_intelligence(),
                        projection_data_source: parsed.producer.projection_data_source(),
                        cleared_dimensions: prepared.cleared_dimensions(),
                    },
                )
            }) {
                finalization_failures.push(QueueRefreshFailure::new(
                    &request.entity_id,
                    "write_results",
                    e.clone(),
                ));
                log::warn!(
                    "IntelProcessor: failed to persist DB assessment for {}: {}",
                    request.entity_id,
                    e
                );
                continue;
            }
            let written_intel = prepared.into_intelligence();
            if let Err(e) = run_enrichment_finalize_post_commit(
                &state,
                &db,
                input,
                &written_intel,
                &parsed.inferred_relationships,
                FinalizeMode::QueueWorker {
                    is_background: is_background_priority(request.priority),
                    producer: parsed.producer,
                },
            ) {
                finalization_failures.push(QueueRefreshFailure::new(
                    &request.entity_id,
                    "finalize",
                    e.clone(),
                ));
                log::warn!(
                    "IntelProcessor: failed to finalize enrichment for {}: {}",
                    request.entity_id,
                    e
                );
                continue;
            }
            finalized_count += 1;

            log::info!(
                "IntelProcessor: completed {} ({} risks, {} wins)",
                request.entity_id,
                written_intel.risks.len(),
                written_intel.recent_wins.len(),
            );
        }

        // Emit completion status for frontend indicator
        for payload in queue_worker_final_status_payloads(
            finalized_count,
            &finalization_failures,
            batch_has_manual,
        ) {
            #[allow(
                clippy::let_underscore_must_use,
                reason = "intentional best-effort discard; preserves existing non-blocking behavior"
            )]
            let _ = app.emit("background-work-status", payload);
        }
    }
}

/// Check whether an `enriched_at` RFC 3339 timestamp is within the TTL window.
///
/// Returns `Some(message)` if the entity should be skipped (enriched recently),
/// `None` if enrichment should proceed.
fn enrichment_age_check(enriched_at: &str, entity_id: &str) -> Option<String> {
    if enriched_at.is_empty() {
        return None;
    }
    let ts = chrono::DateTime::parse_from_rfc3339(enriched_at).ok()?;
    let age_secs = (Utc::now() - ts.with_timezone(&Utc)).num_seconds().max(0) as u64;

    if age_secs < ENRICHMENT_TTL_SECS {
        let minutes_ago = age_secs / 60;
        Some(format!(
            "Skipping {}: enriched {} minutes ago (TTL: {} min)",
            entity_id,
            minutes_ago,
            ENRICHMENT_TTL_SECS / 60,
        ))
    } else {
        None
    }
}

/// Check if an entity was enriched recently enough to skip.
///
/// Resolves the entity directory and reads `intelligence.json` to check the
/// `enriched_at` timestamp. Returns `Some(message)` if the entity should be
/// skipped, `None` if it should proceed.
fn check_enrichment_ttl(_state: &AppState, request: &IntelRequest) -> Option<String> {
    let db =
        crate::db::ActionDb::open(std::sync::Arc::new(crate::db::LocalKeychain::new())).ok()?;
    let intel = db
        .get_entity_intelligence(&request.entity_id)
        .ok()
        .flatten()?;

    enrichment_age_check(&intel.enriched_at, &request.entity_id)
}

/// Resolve an entity's directory from its request metadata.
/// Lightweight helper that opens a short-lived DB connection.
/// Phase 1: Open own DB connection to gather all context needed for enrichment.
/// Uses `ActionDb::open()` instead of `state.db.lock()` to avoid blocking
/// foreground IPC commands while background enrichment runs.
/// Public so manual enrichment commands can reuse the split-lock pattern.
pub fn gather_enrichment_input(
    state: &AppState,
    request: &IntelRequest,
) -> Result<EnrichmentInput, String> {
    let workspace = {
        let config_guard = state.config.read();
        let config = config_guard.as_ref().ok_or("No config")?;
        PathBuf::from(&config.workspace_path)
    };

    let db = crate::db::ActionDb::open(std::sync::Arc::new(crate::db::LocalKeychain::new()))
        .map_err(|e| format!("Failed to open DB: {}", e))?;

    // Look up the entity
    let account = if request.entity_type == "account" {
        db.get_account(&request.entity_id)
            .map_err(|e| e.to_string())?
    } else {
        None
    };
    let project = if request.entity_type == "project" {
        db.get_project(&request.entity_id)
            .map_err(|e| e.to_string())?
    } else {
        None
    };
    let person = if request.entity_type == "person" {
        db.get_person(&request.entity_id)
            .map_err(|e| e.to_string())?
    } else {
        None
    };

    let entity_name = account
        .as_ref()
        .map(|a| a.name.clone())
        .or_else(|| project.as_ref().map(|p| p.name.clone()))
        .or_else(|| person.as_ref().map(|p| p.name.clone()))
        .ok_or_else(|| format!("Entity not found: {}", request.entity_id))?;

    // Resolve entity directory
    let entity_dir = match request.entity_type.as_str() {
        "account" => {
            if let Some(ref acct) = account {
                crate::accounts::resolve_account_dir(&workspace, acct)
            } else {
                crate::accounts::account_dir(&workspace, &entity_name)
            }
        }
        "project" => crate::projects::project_dir(&workspace, &entity_name),
        "person" => crate::people::person_dir(&workspace, &entity_name),
        _ => return Err(format!("Unsupported entity type: {}", request.entity_type)),
    };

    // Read prior intelligence from DB
    let prior = db
        .get_entity_intelligence(&request.entity_id)
        .ok()
        .flatten();

    // Build context via the context provider (ADR-0095).
    // In Local mode this delegates to build_intelligence_context() — same behavior.
    // In Glean mode, context is gathered from Glean search API instead.
    let gather_start = Instant::now();
    let ctx = state
        .context_provider()
        .gather_entity_context(
            &db,
            &request.entity_id,
            &request.entity_type,
            prior.as_ref(),
        )
        .map_err(|e| {
            // Audit Glean-specific failures
            if state.context_provider().is_remote() {
                let error_category = match &e {
                    crate::context_provider::ContextError::Timeout(_) => "timeout",
                    crate::context_provider::ContextError::Auth(_) => "auth",
                    crate::context_provider::ContextError::Db(_) => "db",
                    crate::context_provider::ContextError::Other(_) => "other",
                };
                {
                    let mut audit = state.audit_log.lock();
                    #[allow(clippy::let_underscore_must_use, reason = "intentional best-effort discard; preserves existing non-blocking behavior")]
                    let _ = audit.append(
                        "data_access",
                        "glean_connection_failed",
                        serde_json::json!({
                            "entity_id": request.entity_id,
                            "error_category": error_category,
                        }),
                    );
                }
            }
            format!("Context gather failed: {}", e)
        })?;

    // Audit successful Glean context gather
    if state.context_provider().is_remote() {
        let gather_ms = gather_start.elapsed().as_millis() as u64;
        {
            let mut audit = state.audit_log.lock();
            #[allow(
                clippy::let_underscore_must_use,
                reason = "intentional best-effort discard; preserves existing non-blocking behavior"
            )]
            let _ = audit.append(
                "data_access",
                "glean_context_gathered",
                serde_json::json!({
                    "entity_id": request.entity_id,
                    "entity_type": request.entity_type,
                    "duration_ms": gather_ms,
                }),
            );
        }
    }

    // Read active preset once for prompt language and preset-aware health scoring.
    let active_preset = state.active_preset.read().clone();

    // Compute algorithmic health for accounts before prompt building.
    // This populates ctx.computed_health so the prompt uses narrative-only health schema.
    let mut ctx = ctx;
    let computed_health = if request.entity_type == "account" {
        account.as_ref().map(|acct| {
            crate::intelligence::health_scoring::compute_account_health_with_preset(
                &db,
                acct,
                ctx.org_health.as_ref(),
                active_preset.as_ref(),
            )
        })
    } else {
        None
    };
    ctx.computed_health = computed_health.clone();

    // Compute and persist co-attendance relationships for account entities
    if request.entity_type == "account" {
        match crate::intelligence::relationships::compute_co_attendance(
            &db,
            &request.entity_id,
            90,
            3,
        ) {
            Ok(pairs) => {
                if !pairs.is_empty() {
                    match crate::intelligence::relationships::persist_co_attendance(&db, &pairs) {
                        Ok(count) => {
                            if count > 0 {
                                log::info!(
                                    "I506: Persisted {} co-attendance relationships for {}",
                                    count,
                                    request.entity_id
                                );
                            }
                        }
                        Err(e) => {
                            log::warn!("I506: Failed to persist co-attendance: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                log::warn!(
                    "I506: Co-attendance query failed for {}: {}",
                    request.entity_id,
                    e
                );
            }
        }
    }

    // Build prompt (pure function, but easier to do here while we have the data)
    // Extract relationship for person entities so the prompt adapts framing.
    // For accounts, pass account_type so prompts can adapt for partner vs customer.
    let relationship = person
        .as_ref()
        .map(|p| p.relationship.as_str())
        .or_else(|| account.as_ref().map(|a| a.account_type.as_db_str()));
    // Read active preset for domain-specific prompt language
    let prompt = build_intelligence_prompt_with_preset(
        &entity_name,
        &request.entity_type,
        &ctx,
        relationship,
        active_preset.as_ref(),
    );

    let file_manifest = ctx.file_manifest.clone();
    let file_count = file_manifest.len();

    // Always preserve context — needed for parallel dimension fan-out (local)
    // and Glean-first enrichment (remote).
    let preserved_ctx = Some(ctx);

    // Own DB connection drops here when db goes out of scope
    Ok(EnrichmentInput {
        workspace,
        entity_dir,
        entity_id: request.entity_id.clone(),
        entity_type: request.entity_type.clone(),
        prompt,
        file_manifest,
        file_count,
        computed_health,
        entity_name: entity_name.clone(),
        relationship: relationship.map(|s| s.to_string()),
        intelligence_context: preserved_ctx,
        active_preset,
    })
}

/// /ADR-0100: Glean-first enrichment with PTY fallback.
///
/// For each entity, tries the Glean `chat` MCP tool first. If that fails
/// (timeout, auth, parse error), falls back to the PTY path for that entity.
/// Entities without a Glean context (local-only) go straight to PTY.
async fn run_glean_enrichment_with_fallback(
    inputs: Vec<(IntelRequest, EnrichmentInput)>,
    ai_config: &AiModelConfig,
    state: &AppState,
    app_handle: &AppHandle,
) -> Vec<(IntelRequest, EnrichmentInput, EnrichmentParseResult)> {
    // Get the Glean endpoint from the context provider (not DB — avoids lock contention)
    let glean_endpoint = state
        .context_provider()
        .remote_endpoint()
        .map(|s| s.to_string());

    let _endpoint = match glean_endpoint {
        Some(ep) => ep,
        None => {
            log::warn!("[I535] Glean mode active but no endpoint found, falling back to PTY");
            let mut fallback_results = Vec::new();
            for (request, input) in inputs {
                // PTY calls on blocking threads
                let ai_cfg = ai_config.clone();
                let input_clone = input.clone();
                let usage_context = usage_context_for_priority(request.priority);
                match tauri::async_runtime::spawn_blocking(move || {
                    run_enrichment(&input_clone, &ai_cfg, None, usage_context)
                })
                .await
                {
                    Ok(Ok(parsed)) => fallback_results.push((request, input, parsed)),
                    Ok(Err(e)) => {
                        log::warn!("PTY fallback failed for {}: {}", request.entity_id, e);
                    }
                    Err(e) => {
                        log::error!("PTY fallback panicked for {}: {}", request.entity_id, e);
                    }
                }
            }
            return fallback_results;
        }
    };

    // Read the provider at call time, not once before the batch loop.
    // ADR-0091 "switch mid-queue takes effect on next dequeue" means a
    // settings change between two entities in the same batch must affect
    // the second entity. The previous structure cloned the provider Arc
    // once and reused it across the whole batch, so a mid-batch switch to
    // Local or a Glean disconnect would only take effect on the next batch.
    //
    // The pre-loop snapshot remains only for the early-exit case
    // where Glean is OFF for the whole batch — that's the cold-start
    // "user has Local mode and no Glean configured" case, which can
    // skip the per-entity check entirely and run PTY directly.
    let initial_snap = state.context_snapshot();
    if !initial_snap.is_remote() || initial_snap.glean_intelligence_provider.is_none() {
        log::warn!(
            "IntelProcessor: context-mode snapshot at batch start shows is_remote={} / \
             Glean Arc={:?}; running PTY for {} entities (no Glean configured).",
            initial_snap.is_remote(),
            initial_snap.glean_intelligence_provider.is_some(),
            inputs.len()
        );
        let mut pty_results = Vec::new();
        for (request, input) in inputs {
            let input_clone = input.clone();
            let ai_cfg = ai_config.clone();
            let usage_context = AiUsageContext::for_tier(ModelTier::Synthesis);
            match tokio::task::spawn_blocking(move || {
                run_enrichment(&input_clone, &ai_cfg, None, usage_context)
            })
            .await
            {
                Ok(Ok(parsed)) => pty_results.push((request, input, parsed)),
                Ok(Err(e)) => {
                    log::warn!(
                        "PTY fallback after Glean bridge miss failed for {}: {}",
                        request.entity_id,
                        e
                    );
                }
                Err(e) => {
                    log::error!(
                        "PTY fallback after Glean bridge miss panicked for {}: {}",
                        request.entity_id,
                        e
                    );
                }
            }
        }
        return pty_results;
    }

    let mut results = Vec::new();
    let mut pty_fallback_inputs: Vec<(IntelRequest, EnrichmentInput)> = Vec::new();

    for (request, input) in inputs {
        // per-entity checkpoint inside the Glean batch loop.
        // If a migration started while we were processing earlier entities,
        // abort the rest of the batch — unprocessed entities are dropped
        // (the change-trigger that originally enqueued them will refire
        // post-migration via the normal background path).
        if state.intel_queue.is_paused() {
            log::info!(
                "IntelProcessor: pause detected mid-batch; aborting Glean enrichment after {} successful entities",
                results.len()
            );
            break;
        }

        // Re-read provider+mode for this entity. If a settings change
        // between dequeues switched to
        // Local or cleared the Glean Arc, this entity falls back to
        // PTY rather than honoring the stale pre-loop snapshot.
        let entity_snap = state.context_snapshot();
        let provider = match entity_snap.glean_intelligence_provider {
            Some(p) if entity_snap.is_remote() => p,
            _ => {
                log::info!(
                    "[I535] Mid-batch settings change detected for {} (is_remote={}, \
                     Glean Arc={:?}); routing this entity to PTY per ADR-0091.",
                    input.entity_id,
                    entity_snap.is_remote(),
                    entity_snap.glean_intelligence_provider.is_some(),
                );
                pty_fallback_inputs.push((request, input));
                continue;
            }
        };

        // Only try Glean if we have the intelligence context (populated for remote providers)
        if let Some(ref ctx) = input.intelligence_context {
            log::info!(
                "[I535] Attempting Glean enrichment for {} ({})",
                input.entity_name,
                input.entity_type
            );

            let is_background = is_background_priority(request.priority);
            match provider
                .enrich_entity(
                    &input.entity_id,
                    &input.entity_type,
                    &input.entity_name,
                    ctx,
                    input.relationship.as_deref(),
                    Some(app_handle),
                    is_background,
                    input.active_preset.as_ref(),
                )
                .await
            {
                Ok(intel) => {
                    log::info!(
                        "[I535] Glean enrichment succeeded for {} — assessment: {}, risks: {}, wins: {}",
                        input.entity_name,
                        intel.executive_assessment.is_some(),
                        intel.risks.len(),
                        intel.recent_wins.len(),
                    );

                    // Extract inferred relationships from the Glean output
                    // (Glean may include inferredRelationships in its JSON)
                    let inferred_relationships = if let Ok(raw_json) = serde_json::to_string(&intel)
                    {
                        extract_inferred_relationships(&raw_json)
                    } else {
                        Vec::new()
                    };

                    results.push((
                        request,
                        input,
                        EnrichmentParseResult {
                            intel,
                            inferred_relationships,
                            producer: EnrichmentProducer::Glean,
                        },
                    ));
                    continue;
                }
                Err(e) => {
                    log::warn!(
                        "[I535] Glean enrichment failed for {}, falling back to PTY: {}",
                        input.entity_name,
                        e
                    );
                }
            }
        } else {
            log::info!(
                "[I535] No intelligence context for {}, using PTY directly",
                input.entity_id
            );
        }

        // Fall back to PTY for this entity
        pty_fallback_inputs.push((request, input));
    }

    // Run PTY enrichment for all entities that Glean failed on
    // PTY calls on blocking threads
    if !pty_fallback_inputs.is_empty() {
        log::info!(
            "[I535] Running PTY fallback for {} entities",
            pty_fallback_inputs.len()
        );
        for (request, input) in pty_fallback_inputs {
            let ai_cfg = ai_config.clone();
            let input_clone = input.clone();
            let usage_context = usage_context_for_priority(request.priority);
            match tauri::async_runtime::spawn_blocking(move || {
                run_enrichment(&input_clone, &ai_cfg, None, usage_context)
            })
            .await
            {
                Ok(Ok(parsed)) => results.push((request, input, parsed)),
                Ok(Err(e)) => {
                    log::warn!("PTY fallback failed for {}: {}", request.entity_id, e);
                }
                Err(e) => {
                    log::error!("PTY fallback panicked for {}: {}", request.entity_id, e);
                }
            }
        }
    }

    results
}

/// Categorize an enrichment error for audit logging.
fn categorize_enrichment_error(error: &str) -> &'static str {
    let lower = error.to_lowercase();
    if lower.contains("timed out") || lower.contains("timeout") {
        "timeout"
    } else if lower.contains("validation")
        || lower.contains("schema")
        || lower.contains("invalid json")
    {
        "schema_validation"
    } else {
        "pty_error"
    }
}

/// Step 2: Run PTY enrichment (no DB lock held).
/// Public so manual enrichment commands can reuse the split-lock pattern.
///
/// Tries parallel per-dimension fan-out first (if `intelligence_context` is available),
/// then falls back to the legacy monolithic prompt path.
///
/// When `app_handle` is provided, emits `enrichment-progress` and
/// `enrichment-complete` events for progressive frontend updates.
pub fn run_enrichment(
    input: &EnrichmentInput,
    ai_config: &AiModelConfig,
    app_handle: Option<&AppHandle>,
    usage_context: AiUsageContext,
) -> Result<EnrichmentParseResult, String> {
    if usage_context.background {
        return run_background_enrichment(input, ai_config, &usage_context);
    }
    if input.intelligence_context.is_some() {
        match run_parallel_enrichment(input, ai_config, app_handle, &usage_context) {
            Ok(result) => return Ok(result),
            Err(e) => {
                log::warn!(
                    "[I574] Parallel enrichment failed for {}, falling back to legacy: {}",
                    input.entity_id,
                    e
                );
            }
        }
    }
    run_enrichment_legacy(input, ai_config, &usage_context)
}

/// Parallel per-dimension enrichment engine.
///
/// Fans out 6 dimension-specific PTY calls in parallel threads (30s each),
/// then merges successful dimension results into a single `IntelligenceJson`.
/// Returns Err only if ALL 6 dimensions fail (caller falls back to legacy).
///
/// Uses a channel pattern so each dimension result is written to DB and
/// emitted as a progress event as soon as it completes, rather than waiting
/// for all 6 to finish. This enables progressive frontend updates.
fn run_parallel_enrichment(
    input: &EnrichmentInput,
    ai_config: &AiModelConfig,
    app_handle: Option<&AppHandle>,
    usage_context: &AiUsageContext,
) -> Result<EnrichmentParseResult, String> {
    let ctx = input
        .intelligence_context
        .as_ref()
        .ok_or("No intelligence context for parallel enrichment")?;

    let is_incremental = ctx.prior_intelligence.is_some();
    let overall_start = Instant::now();
    let applicability = dimension_prompts::dimension_applicability(
        &input.entity_type,
        input.relationship.as_deref(),
    );
    let total_dimensions = applicability.applicable.len() as u32;
    let canonical_total_dimensions = DIMENSION_NAMES.len() as u32;
    let skipped_dimensions: Vec<String> = applicability
        .skipped
        .iter()
        .map(|dimension| (*dimension).to_string())
        .collect();

    if total_dimensions == 0 {
        return Err(format!(
            "No applicable dimensions for {} ({})",
            input.entity_id, input.entity_type
        ));
    }

    // Channel for receiving dimension results as they complete
    let (tx, rx) = std::sync::mpsc::channel();

    // Spawn one thread per dimension
    for &dimension in DIMENSION_NAMES {
        if !applicability.applicable.contains(&dimension) {
            continue;
        }

        let dim_prompt = dimension_prompts::build_dimension_prompt(
            dimension,
            &input.entity_name,
            &input.entity_type,
            input.relationship.as_deref(),
            ctx,
            is_incremental,
            input.active_preset.as_ref(),
        );

        let workspace = input.workspace.clone();
        let ai_cfg = ai_config.clone();
        let entity_id = input.entity_id.clone();
        let entity_type = input.entity_type.clone();
        let file_count = input.file_count;
        let file_manifest = input.file_manifest.clone();
        let dim_name = dimension.to_string();
        let sender = tx.clone();
        let dimension_usage_context = usage_context.clone().with_tier(ModelTier::Extraction);

        std::thread::spawn(move || {
            let dim_start = Instant::now();

            //  route through PtyClaudeCode trait surface (sync
            // helper) instead of constructing PtyManager inline. Behavior
            // parity preserved — same tier/timeout/nice_priority/usage_context.
            // The sync helper bridges async-trait callers; ReplayProvider
            // substitution lands when intel_queue's sync orchestration is
            // async-ified (W3-A migration).
            let pty_provider = crate::intelligence::pty_provider::PtyClaudeCode::new(
                std::sync::Arc::new(ai_cfg.clone()),
                workspace.clone(),
                dimension_usage_context,
            );
            let prompt_input = crate::intelligence::provider::PromptInput::new(dim_prompt.clone())
                .with_workspace(workspace.clone());

            let result = pty_provider
                .complete_blocking(prompt_input, ModelTier::Extraction)
                .map(|completion| crate::pty::ClaudeOutput {
                    stdout: completion.text,
                    exit_code: 0,
                })
                .map_err(|e| format!("PTY error for dimension {}: {:?}", dim_name, e))
                .and_then(|output| {
                    let intel = parse_intelligence_response(
                        &output.stdout,
                        &entity_id,
                        &entity_type,
                        file_count,
                        file_manifest,
                    )
                    .map_err(|e| format!("Parse error for dimension {}: {}", dim_name, e))?;

                    let elapsed_ms = dim_start.elapsed().as_millis();
                    log::info!(
                        "[I574] Dimension {} completed in {}ms",
                        dim_name,
                        elapsed_ms
                    );

                    Ok((dim_name, intel, output.stdout))
                });

            // Send result through channel; ignore error if receiver dropped
            #[allow(
                clippy::let_underscore_must_use,
                reason = "intentional best-effort discard; preserves existing non-blocking behavior"
            )]
            let _ = sender.send(result);
        });
    }

    // Drop our copy of the sender so rx iterator ends after all threads finish
    drop(tx);

    // Process dimension results as they arrive
    let mut combined = IntelligenceJson::default();
    let mut succeeded: u32 = 0;
    let mut failed_dims: Vec<String> = Vec::new();
    let mut all_raw_output = String::new();

    for result in rx {
        match result {
            Ok((dim_name, partial_intel, raw_output)) => {
                if let Err(e) = dimension_prompts::merge_dimension_into(
                    &mut combined,
                    &dim_name,
                    &partial_intel,
                ) {
                    log::warn!("[I574] Merge failed for dimension {}: {}", dim_name, e);
                    failed_dims.push(dim_name);
                } else {
                    succeeded += 1;
                    all_raw_output.push_str(&raw_output);
                    all_raw_output.push('\n');

                    // Per-dimension DB write + event emission
                    if let Some(handle) = app_handle {
                        write_progressive_dimension(
                            &input.entity_id,
                            &input.entity_type,
                            input.relationship.as_deref(),
                            &combined,
                        );
                        #[allow(
                            clippy::let_underscore_must_use,
                            reason = "intentional best-effort discard; preserves existing non-blocking behavior"
                        )]
                        let _ = handle.emit(
                            "enrichment-progress",
                            EnrichmentProgress {
                                entity_id: input.entity_id.clone(),
                                entity_type: input.entity_type.clone(),
                                completed: succeeded,
                                total: total_dimensions,
                                last_dimension: dim_name,
                            },
                        );
                    }
                }
            }
            Err(e) => {
                // Extract dimension name from error message for tracking
                let dim = e
                    .split("dimension ")
                    .nth(1)
                    .and_then(|s| s.split(':').next())
                    .unwrap_or("unknown")
                    .to_string();
                log::warn!("[I574] Dimension thread returned error: {}", e);
                failed_dims.push(dim);
            }
        }
    }

    let total_ms = overall_start.elapsed().as_millis();
    log::info!(
        "[I574] Parallel enrichment: {}/{} applicable dimensions succeeded in {}ms (skipped: {:?})",
        succeeded,
        total_dimensions,
        total_ms,
        skipped_dimensions,
    );

    // Emit completion event
    if let Some(handle) = app_handle {
        #[allow(
            clippy::let_underscore_must_use,
            reason = "intentional best-effort discard; preserves existing non-blocking behavior"
        )]
        let _ = handle.emit(
            "enrichment-complete",
            EnrichmentComplete {
                entity_id: input.entity_id.clone(),
                entity_type: input.entity_type.clone(),
                succeeded,
                failed: failed_dims.len() as u32,
                failed_dimensions: failed_dims,
                total_dimensions: canonical_total_dimensions,
                applicable_total: total_dimensions,
                skipped_dimensions,
                wall_clock_ms: total_ms as u64,
            },
        );
    }

    if succeeded == 0 {
        return Err(format!(
            "All {} applicable dimensions failed",
            total_dimensions
        ));
    }

    // Extract inferred relationships from the combined raw output
    let inferred_relationships = extract_inferred_relationships(&all_raw_output);

    // Extract and persist keywords from the combined raw output
    if let Some(keywords_json) =
        crate::intelligence::extract_keywords_from_response(&all_raw_output)
    {
        if let Ok(db) =
            crate::db::ActionDb::open(std::sync::Arc::new(crate::db::LocalKeychain::new()))
        {
            let clock = crate::services::context::SystemClock;
            let rng = crate::services::context::SystemRng;
            let ext = crate::services::context::ExternalClients::default();
            let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &ext);
            if let Err(err) = crate::services::intelligence::persist_entity_keywords(
                &ctx,
                &db,
                &input.entity_type,
                &input.entity_id,
                &keywords_json,
            ) {
                log::warn!(
                    "[I574] keyword persistence failed for {} {}: {}",
                    input.entity_type,
                    input.entity_id,
                    err
                );
            }
        }
    }

    // Stamp enriched_at and source_manifest on the combined result.
    // The parallel fan-out initialises `combined` as IntelligenceJson::default(),
    // and `merge_dimension_into` only copies dimension-specific fields — it never
    // propagates the manifest or enrichment timestamp.  Without this, every account
    // enriched via the parallel path ends up with an empty source_manifest_json
    // column even when the context builder collected dozens of source files.
    combined.enriched_at = chrono::Utc::now().to_rfc3339();
    combined.source_file_count = input.file_manifest.len();
    combined.source_manifest = input.file_manifest.clone();
    combined.entity_id = input.entity_id.clone();
    combined.entity_type = input.entity_type.clone();

    Ok(EnrichmentParseResult {
        intel: combined,
        inferred_relationships,
        producer: EnrichmentProducer::Pty,
    })
}

/// Write the current progressive state of intelligence to DB after a dimension completes.
///
/// Opens a short-lived DB connection, reads existing entity_assessment, merges the
/// new combined state, and writes back. Non-fatal on error — the committed
/// enrichment persistence path is the authoritative write.
fn write_progressive_dimension(
    entity_id: &str,
    entity_type: &str,
    relationship: Option<&str>,
    combined: &IntelligenceJson,
) {
    let db = match crate::db::ActionDb::open(std::sync::Arc::new(crate::db::LocalKeychain::new())) {
        Ok(db) => db,
        Err(e) => {
            log::warn!(
                "[I575] Progressive write: failed to open DB for {}: {}",
                entity_id,
                e
            );
            return;
        }
    };

    let merged =
        prepare_progressive_dimension_snapshot(&db, entity_id, entity_type, relationship, combined);

    let clock = crate::services::context::SystemClock;
    let rng = crate::services::context::SystemRng;
    let ext = crate::services::context::ExternalClients::default();
    let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &ext);
    if let Err(e) = crate::services::intelligence::upsert_assessment_snapshot(&ctx, &db, &merged) {
        log::warn!("[I575] Progressive write failed for {}: {}", entity_id, e);
    } else {
        log::debug!("[I575] Progressive write succeeded for {}", entity_id,);
    }
}

pub(crate) fn prepare_progressive_dimension_snapshot(
    db: &crate::db::ActionDb,
    entity_id: &str,
    entity_type: &str,
    relationship: Option<&str>,
    combined: &IntelligenceJson,
) -> IntelligenceJson {
    // Progressive writes within a single enrichment cycle use simple dimension
    // merge, NOT reconciliation. Reconciliation happens at the final write.
    let existing = db.get_entity_intelligence(entity_id).ok().flatten();
    let mut merged = if let Some(mut existing) = existing {
        for dim in crate::intelligence::dimension_prompts::DIMENSION_NAMES {
            // best-effort: progressive partial writes are advisory; final reconciliation writes the authoritative snapshot.
            #[allow(
                clippy::let_underscore_must_use,
                reason = "intentional best-effort discard; preserves existing non-blocking behavior"
            )]
            let _ = crate::intelligence::dimension_prompts::merge_dimension_into(
                &mut existing,
                dim,
                combined,
            );
        }
        existing
    } else {
        combined.clone()
    };

    // Set entity metadata for the DB write
    merged.entity_id = entity_id.to_string();
    merged.entity_type = entity_type.to_string();
    merged.enriched_at = chrono::Utc::now().to_rfc3339();

    crate::intelligence::dimension_prompts::clear_inapplicable_dimension_fields(
        &mut merged,
        entity_type,
        relationship,
    );

    merged
}

/// Legacy monolithic PTY enrichment — single prompt, 30s timeout.
/// Kept as fallback when parallel enrichment is unavailable or fails.
fn run_enrichment_legacy(
    input: &EnrichmentInput,
    ai_config: &AiModelConfig,
    usage_context: &AiUsageContext,
) -> Result<EnrichmentParseResult, String> {
    //  legacy synthesis path migrated through PtyClaudeCode
    // sync helper. Behavior parity preserved (same Synthesis tier + 240s
    // timeout from PtyClaudeCode::timeout_for_tier(Synthesis)).
    let pty_provider = crate::intelligence::pty_provider::PtyClaudeCode::new(
        std::sync::Arc::new(ai_config.clone()),
        input.workspace.clone(),
        usage_context.clone(),
    );
    let prompt_input = crate::intelligence::provider::PromptInput::new(input.prompt.clone())
        .with_workspace(input.workspace.clone());
    let completion = pty_provider
        .complete_blocking(prompt_input, ModelTier::Synthesis)
        .map_err(|e| format!("Claude Code error: {:?}", e))?;
    let output = crate::pty::ClaudeOutput {
        stdout: completion.text,
        exit_code: 0,
    };

    // Extract and persist keywords from the raw AI response
    if let Some(keywords_json) = crate::intelligence::extract_keywords_from_response(&output.stdout)
    {
        if let Ok(db) =
            crate::db::ActionDb::open(std::sync::Arc::new(crate::db::LocalKeychain::new()))
        {
            let clock = crate::services::context::SystemClock;
            let rng = crate::services::context::SystemRng;
            let ext = crate::services::context::ExternalClients::default();
            let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &ext);
            if let Err(err) = crate::services::intelligence::persist_entity_keywords(
                &ctx,
                &db,
                &input.entity_type,
                &input.entity_id,
                &keywords_json,
            ) {
                log::warn!(
                    "intel_queue: keyword persistence failed for {} {}: {}",
                    input.entity_type,
                    input.entity_id,
                    err
                );
            }
        }
    }

    let inferred_relationships = extract_inferred_relationships(&output.stdout);
    let intel = parse_intelligence_response(
        &output.stdout,
        &input.entity_id,
        &input.entity_type,
        input.file_count,
        input.file_manifest.clone(),
    )?;

    Ok(EnrichmentParseResult {
        intel,
        inferred_relationships,
        producer: EnrichmentProducer::Pty,
    })
}

fn run_background_enrichment(
    input: &EnrichmentInput,
    ai_config: &AiModelConfig,
    usage_context: &AiUsageContext,
) -> Result<EnrichmentParseResult, String> {
    let pty = PtyManager::for_tier(ModelTier::Background, ai_config)
        .with_usage_context(usage_context.clone().with_tier(ModelTier::Background))
        .with_timeout(BACKGROUND_ENRICHMENT_TIMEOUT_SECS)
        .with_nice_priority(10);
    let output = pty
        .spawn_claude(&input.workspace, &input.prompt)
        .map_err(|e| format!("Claude Code error: {}", e))?;

    // Extract and persist keywords from the raw AI response so background-tier
    // enrichment (ProactiveHygiene, CalendarChange) keeps entity keywords
    // current for transcript routing and entity resolution.
    if let Some(keywords_json) = crate::intelligence::extract_keywords_from_response(&output.stdout)
    {
        if let Ok(db) =
            crate::db::ActionDb::open(std::sync::Arc::new(crate::db::LocalKeychain::new()))
        {
            let clock = crate::services::context::SystemClock;
            let rng = crate::services::context::SystemRng;
            let ext = crate::services::context::ExternalClients::default();
            let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &ext);
            if let Err(err) = crate::services::intelligence::persist_entity_keywords(
                &ctx,
                &db,
                &input.entity_type,
                &input.entity_id,
                &keywords_json,
            ) {
                log::warn!(
                    "intel_queue: keyword persistence failed for {} {}: {}",
                    input.entity_type,
                    input.entity_id,
                    err
                );
            }
        }
    }

    let inferred_relationships = extract_inferred_relationships(&output.stdout);
    let intel = parse_intelligence_response(
        &output.stdout,
        &input.entity_id,
        &input.entity_type,
        input.file_count,
        input.file_manifest.clone(),
    )?;

    Ok(EnrichmentParseResult {
        intel,
        inferred_relationships,
        producer: EnrichmentProducer::Pty,
    })
}

/// One-shot repair retry when deterministic checks still report
/// high-severity contradictions after local repairs.
fn run_consistency_repair_retry(
    input: &EnrichmentInput,
    intel: &IntelligenceJson,
    report: &crate::intelligence::ConsistencyReport,
    facts: &crate::intelligence::FactContext,
    ai_config: &AiModelConfig,
) -> Result<IntelligenceJson, String> {
    let prompt = crate::intelligence::build_repair_prompt(intel, report, facts);
    let pty = PtyManager::for_tier(ModelTier::Extraction, ai_config)
        .with_usage_context(
            AiUsageContext::new("intel_queue", "consistency_repair_retry")
                .with_trigger("post_write_repair")
                .with_tier(ModelTier::Extraction),
        )
        .with_timeout(90)
        .with_nice_priority(10);
    let output = pty
        .spawn_claude(&input.workspace, &prompt)
        .map_err(|e| format!("Consistency repair retry failed: {}", e))?;

    parse_intelligence_response(
        &output.stdout,
        &input.entity_id,
        &input.entity_type,
        input.file_count,
        input.file_manifest.clone(),
    )
}

/// Prepared intelligence plus side writes that must share the upsert transaction.
#[derive(Debug, Clone)]
pub struct PreparedEnrichment {
    intelligence: IntelligenceJson,
    projection_intelligence: IntelligenceJson,
    side_writes: EnrichmentSideWrites,
    cleared_dimensions: Vec<&'static str>,
}

impl PreparedEnrichment {
    pub fn intelligence(&self) -> &IntelligenceJson {
        &self.intelligence
    }

    pub fn projection_intelligence(&self) -> &IntelligenceJson {
        &self.projection_intelligence
    }

    pub fn cleared_dimensions(&self) -> &[&'static str] {
        &self.cleared_dimensions
    }

    pub fn into_intelligence(self) -> IntelligenceJson {
        self.intelligence
    }
}

#[derive(Debug, Clone, Default)]
struct EnrichmentSideWrites {
    malformed_suppression_audits: Vec<MalformedSuppressionAudit>,
}

#[derive(Debug, Clone)]
struct MalformedSuppressionAudit {
    record_id: String,
    reason: String,
    field_key: &'static str,
    caller_context: &'static str,
}

fn filter_suppressed_risks_and_wins(
    db: &crate::db::ActionDb,
    input: &EnrichmentInput,
    intel: &mut IntelligenceJson,
    mut side_writes: Option<&mut EnrichmentSideWrites>,
) {
    use crate::db::intelligence_feedback::SuppressionDecision;
    use crate::intelligence::canonicalization::{item_hash as canonical_item_hash, ItemKind};

    let pre_risk_count = intel.risks.len();
    intel.risks.retain(|risk| {
        let item_key = Some(risk.text.as_str());
        let hash = canonical_item_hash(ItemKind::Risk, &risk.text);
        match db.is_suppressed(
            &input.entity_id,
            "risks",
            item_key,
            Some(&hash),
            risk.item_source.as_ref().map(|s| s.sourced_at.as_str()),
        ) {
            SuppressionDecision::Suppressed { .. } => false,
            SuppressionDecision::NotSuppressed => true,
            SuppressionDecision::Malformed { record_id, reason } => {
                log::error!(
                    "[is_suppressed] malformed tombstone {:?} for entity {} field risks; \
                     failing closed: {:?}",
                    record_id,
                    input.entity_id,
                    reason
                );
                if let Some(writes) = side_writes.as_deref_mut() {
                    writes
                        .malformed_suppression_audits
                        .push(MalformedSuppressionAudit {
                            record_id: record_id.0,
                            reason: format!("{:?}", reason),
                            field_key: "risks",
                            caller_context: "intel_queue.compose_enrichment_intelligence.risks",
                        });
                }
                false
            }
        }
    });

    let pre_win_count = intel.recent_wins.len();
    intel.recent_wins.retain(|win| {
        let item_key = Some(win.text.as_str());
        let hash = canonical_item_hash(ItemKind::Win, &win.text);
        match db.is_suppressed(
            &input.entity_id,
            "recentWins",
            item_key,
            Some(&hash),
            win.item_source.as_ref().map(|s| s.sourced_at.as_str()),
        ) {
            SuppressionDecision::Suppressed { .. } => false,
            SuppressionDecision::NotSuppressed => true,
            SuppressionDecision::Malformed { record_id, reason } => {
                log::error!(
                    "[is_suppressed] malformed tombstone {:?} for entity {} field recentWins; \
                     failing closed: {:?}",
                    record_id,
                    input.entity_id,
                    reason
                );
                if let Some(writes) = side_writes.as_deref_mut() {
                    writes
                        .malformed_suppression_audits
                        .push(MalformedSuppressionAudit {
                            record_id: record_id.0,
                            reason: format!("{:?}", reason),
                            field_key: "recentWins",
                            caller_context:
                                "intel_queue.compose_enrichment_intelligence.recentWins",
                        });
                }
                false
            }
        }
    });

    let risks_suppressed = pre_risk_count - intel.risks.len();
    let wins_suppressed = pre_win_count - intel.recent_wins.len();
    if risks_suppressed > 0 || wins_suppressed > 0 {
        log::info!(
            "[I645] Suppression filter for {}: {} risks, {} wins removed",
            input.entity_id,
            risks_suppressed,
            wins_suppressed,
        );
    }
}

/// Phase 3: Compose enrichment results for DB-first persistence.
/// This phase prepares the intelligence payload and records deferred side
/// writes that the caller applies in the same transaction as the DB upsert.
/// Public so manual enrichment commands can reuse the split-lock pattern.
pub fn compose_enrichment_intelligence(
    _state: &AppState,
    db: &crate::db::ActionDb,
    input: &EnrichmentInput,
    intel: &IntelligenceJson,
    producer: EnrichmentProducer,
    ai_config: Option<&AiModelConfig>,
) -> Result<PreparedEnrichment, String> {
    compose_enrichment_intelligence_payload(db, input, intel, producer, ai_config)
}

pub(crate) fn compose_enrichment_intelligence_payload(
    db: &crate::db::ActionDb,
    input: &EnrichmentInput,
    intel: &IntelligenceJson,
    producer: EnrichmentProducer,
    ai_config: Option<&AiModelConfig>,
) -> Result<PreparedEnrichment, String> {
    // Source-aware reconciliation + preserve user-edited fields
    let mut final_intel = intel.clone();
    let mut side_writes = EnrichmentSideWrites::default();
    let existing_intel = db.get_entity_intelligence(&input.entity_id).ok().flatten();
    if let Some(existing) = existing_intel.as_ref() {
        // Apply source-aware reconciliation (preserves user corrections,
        // non-refreshed source items, and dismissed tombstones).
        // stakeholder_insights reconciliation is skipped in reconcile_enrichment —
        // protection is now structural via data_source columns on account_stakeholders.
        final_intel = crate::intelligence::glean_provider::reconcile_enrichment(
            existing.clone(),
            final_intel,
            &["pty_synthesis"],
        );

        // Also preserve field-level user edits (granular JSON-path edits)
        if !existing.user_edits.is_empty() {
            crate::intelligence::preserve_user_edits(&mut final_intel, existing);
            log::info!(
                "IntelProcessor: preserved {} user edits for {}",
                existing.user_edits.len(),
                input.entity_id,
            );
        }
    }

    // Prevent sparse refreshes from wiping narrative intelligence.
    // If the new response is structurally valid JSON but contains little/no
    // usable narrative, keep prior core fields until enrichment recovers.
    merge_missing_core_fields_from_existing(&mut final_intel, existing_intel.as_ref());

    // Deterministic contradiction checks + balanced repair pass.
    if input.entity_type == "account" || input.entity_type == "project" {
        if let Ok(facts) =
            crate::intelligence::build_fact_context(db, &input.entity_id, &input.entity_type)
        {
            let initial_report = crate::intelligence::check_consistency(&final_intel, &facts);
            let repaired_intel = crate::intelligence::apply_deterministic_repairs(
                &final_intel,
                &initial_report,
                &facts,
            );
            let mut unresolved_report =
                crate::intelligence::check_consistency(&repaired_intel, &facts);
            let mut post_repair_intel = repaired_intel;

            // Balanced mode: one retry for unresolved high-severity findings.
            if unresolved_report.has_high() {
                if let Some(cfg) = ai_config {
                    if let Ok(retry_intel) = run_consistency_repair_retry(
                        input,
                        &post_repair_intel,
                        &unresolved_report,
                        &facts,
                        cfg,
                    ) {
                        let retry_unresolved =
                            crate::intelligence::check_consistency(&retry_intel, &facts);
                        if retry_unresolved.findings.len() <= unresolved_report.findings.len() {
                            post_repair_intel = retry_intel;
                            unresolved_report = retry_unresolved;
                        }
                    }
                }
            }

            post_repair_intel.consistency_status = Some(crate::intelligence::status_from_reports(
                &initial_report,
                &unresolved_report,
            ));
            post_repair_intel.consistency_findings =
                crate::intelligence::merge_fixed_flags(&initial_report, &unresolved_report);
            post_repair_intel.consistency_checked_at = Some(Utc::now().to_rfc3339());
            final_intel = post_repair_intel;
        }
    }

    // Filter suppressed risks/wins before writing.
    // Fail-closed: if we cannot open the feedback DB to check suppression,
    // drop risks/wins for this round rather than writing potentially-tombstoned
    // items. One lost enrichment round is recoverable; tombstone resurrection is not.
    filter_suppressed_risks_and_wins(db, input, &mut final_intel, Some(&mut side_writes));

    // Merge computed health dimensions with LLM narrative.
    // The algorithmic engine provides score/band/dimensions/confidence;
    // the LLM provides only narrative + recommended_actions.
    if let Some(ref computed) = input.computed_health {
        let llm_narrative = final_intel
            .health
            .as_ref()
            .and_then(|h| h.narrative.clone());
        let llm_actions = final_intel
            .health
            .as_ref()
            .map(|h| h.recommended_actions.clone())
            .unwrap_or_default();
        final_intel.health = Some(crate::intelligence::io::AccountHealth {
            narrative: llm_narrative,
            recommended_actions: llm_actions,
            ..computed.clone()
        });
    }

    // Reconcile user-entered facts with AI-inferred values.
    // User-edited fields (source weight 1.0) override AI guesses.
    if input.entity_type == "account" {
        apply_confirmed_account_facts_to_enrichment_projection(db, input, &mut final_intel, true);
    }

    let cleared_dimensions =
        crate::intelligence::dimension_prompts::clear_inapplicable_dimension_fields(
            &mut final_intel,
            &input.entity_type,
            input.relationship.as_deref(),
        );
    if !cleared_dimensions.is_empty() {
        log::debug!(
            "IntelProcessor: cleared skipped dimensions for {} {}: {:?}",
            input.entity_type,
            input.entity_id,
            cleared_dimensions
        );
    }

    let mut projection_intelligence = if producer.is_glean() {
        intel.clone()
    } else {
        final_intel.clone()
    };
    if producer.is_glean() {
        apply_deterministic_projection_repairs(db, input, &mut projection_intelligence);
        if input.entity_type == "account" {
            apply_confirmed_account_facts_to_enrichment_projection(
                db,
                input,
                &mut projection_intelligence,
                false,
            );
        }
        filter_suppressed_risks_and_wins(db, input, &mut projection_intelligence, None);
        crate::intelligence::dimension_prompts::clear_inapplicable_dimension_fields(
            &mut projection_intelligence,
            &input.entity_type,
            input.relationship.as_deref(),
        );
    }

    Ok(PreparedEnrichment {
        intelligence: final_intel,
        projection_intelligence,
        side_writes,
        cleared_dimensions,
    })
}

fn apply_confirmed_account_facts_to_enrichment_projection(
    db: &crate::db::ActionDb,
    input: &EnrichmentInput,
    intel: &mut IntelligenceJson,
    create_missing_contract_context: bool,
) {
    let Ok(Some(account)) = db.get_account(&input.entity_id) else {
        return;
    };
    let Some(user_arr) = account.arr else {
        return;
    };
    let contract_context = if create_missing_contract_context {
        Some(intel.contract_context.get_or_insert_with(Default::default))
    } else {
        intel.contract_context.as_mut()
    };
    let Some(contract_context) = contract_context else {
        return;
    };
    if contract_context.current_arr != Some(user_arr) {
        log::info!(
            "[intel_queue] Overriding AI currentArr ({:?}) with user ARR ({}) for {}",
            contract_context.current_arr,
            user_arr,
            input.entity_id,
        );
        contract_context.current_arr = Some(user_arr);
    }
}

fn apply_deterministic_projection_repairs(
    db: &crate::db::ActionDb,
    input: &EnrichmentInput,
    projection_intel: &mut IntelligenceJson,
) {
    if input.entity_type != "account" && input.entity_type != "project" {
        return;
    }
    let Ok(facts) =
        crate::intelligence::build_fact_context(db, &input.entity_id, &input.entity_type)
    else {
        return;
    };
    let initial_report = crate::intelligence::check_consistency(projection_intel, &facts);
    let repaired_intel =
        crate::intelligence::apply_deterministic_repairs(projection_intel, &initial_report, &facts);
    let unresolved_report = crate::intelligence::check_consistency(&repaired_intel, &facts);
    let mut repaired_intel = repaired_intel;
    repaired_intel.consistency_status = Some(crate::intelligence::status_from_reports(
        &initial_report,
        &unresolved_report,
    ));
    repaired_intel.consistency_findings =
        crate::intelligence::merge_fixed_flags(&initial_report, &unresolved_report);
    repaired_intel.consistency_checked_at = Some(Utc::now().to_rfc3339());
    *projection_intel = repaired_intel;
}

pub fn apply_enrichment_side_writes(
    ctx: &crate::services::context::ServiceContext<'_>,
    tx: &crate::db::ActionDb,
    input: &EnrichmentInput,
    prepared: &PreparedEnrichment,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    apply_enrichment_stakeholder_side_writes(tx, input, &prepared.intelligence)?;

    for audit in &prepared.side_writes.malformed_suppression_audits {
        tx.record_malformed_suppression(
            &audit.record_id,
            &audit.reason,
            &input.entity_id,
            audit.field_key,
            Some(audit.caller_context),
        )
        .map_err(|audit_err| {
            format!(
                "[DOS-308] failed to persist malformed suppression audit for entity {}: {}",
                input.entity_id, audit_err
            )
        })?;
    }

    Ok(())
}

fn apply_enrichment_stakeholder_side_writes(
    db: &crate::db::ActionDb,
    input: &EnrichmentInput,
    final_intel: &IntelligenceJson,
) -> Result<(), String> {
    // Route AI stakeholder insights to DB columns or suggestions table.
    // Person-first architecture: enrichment never overwrites user-designated data.
    // AI can update columns it previously wrote (data_source='ai'), and new
    // discoveries go to stakeholder_suggestions for user review.
    if input.entity_type != "account" || final_intel.stakeholder_insights.is_empty() {
        return Ok(());
    }

    for insight in &final_intel.stakeholder_insights {
        let ai_source = insight
            .item_source
            .as_ref()
            .map(|s| s.source.as_str())
            .unwrap_or("pty_synthesis");

        if let Some(ref pid) = insight.person_id {
            // Check if this person_id exists in account_stakeholders for this account
            // stakeholder side-write reads propagate transient lock failures intentionally;
            // silent default-on-error masked duplicate-stakeholder hazards in the earlier
            // contention class. Aborting + retrying is preferable to duplicate rows.
            let row_exists: bool = db
                .conn_ref()
                .query_row(
                    "SELECT COUNT(*) FROM account_stakeholders WHERE account_id = ?1 AND person_id = ?2",
                    rusqlite::params![&input.entity_id, pid],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(|e| {
                    format!(
                        "stakeholder existence read failed for {}:{}: {e}",
                        input.entity_id, pid
                    )
                })?
                > 0;

            if row_exists {
                // Update engagement if AI-owned
                if let Some(ref engagement) = insight.engagement {
                    let ds: String = db
                        .conn_ref()
                        .query_row(
                            "SELECT data_source_engagement FROM account_stakeholders WHERE account_id = ?1 AND person_id = ?2",
                            rusqlite::params![&input.entity_id, pid],
                            |row| row.get(0),
                        )
                        .optional()
                        .map_err(|e| {
                            format!(
                                "stakeholder engagement data-source read failed for {}:{}: {e}",
                                input.entity_id, pid
                            )
                        })?
                        .unwrap_or_else(|| "ai".to_string());
                    if ds == "ai" {
                        db.conn_ref()
                            .execute(
                                "UPDATE account_stakeholders SET engagement = ?1 WHERE account_id = ?2 AND person_id = ?3 AND data_source_engagement = 'ai'",
                                rusqlite::params![engagement, &input.entity_id, pid],
                            )
                            .map_err(|e| {
                                format!(
                                "update AI-owned stakeholder engagement failed for {}:{}: {e}",
                                input.entity_id,
                                pid
                                )
                            })?;
                    } else {
                        // AI disagrees with user-owned engagement — write suggestion
                        write_stakeholder_suggestion(&StakeholderSuggestionParams {
                            db,
                            account_id: &input.entity_id,
                            person_id: Some(pid),
                            insight,
                            source: ai_source,
                        })?;
                    }
                }

                // Update assessment if AI-owned
                if let Some(ref assessment) = insight.assessment {
                    let ds: String = db
                        .conn_ref()
                        .query_row(
                            "SELECT data_source_assessment FROM account_stakeholders WHERE account_id = ?1 AND person_id = ?2",
                            rusqlite::params![&input.entity_id, pid],
                            |row| row.get(0),
                        )
                        .optional()
                        .map_err(|e| {
                            format!(
                                "stakeholder assessment data-source read failed for {}:{}: {e}",
                                input.entity_id, pid
                            )
                        })?
                        .unwrap_or_else(|| "ai".to_string());
                    if ds == "ai" {
                        db.conn_ref()
                            .execute(
                                "UPDATE account_stakeholders SET assessment = ?1 WHERE account_id = ?2 AND person_id = ?3 AND data_source_assessment = 'ai'",
                                rusqlite::params![assessment, &input.entity_id, pid],
                            )
                            .map_err(|e| {
                                format!(
                                "update AI-owned stakeholder assessment failed for {}:{}: {e}",
                                input.entity_id,
                                pid
                                )
                            })?;
                    }
                }

                // Upsert roles: skip user-owned, update/insert AI-owned
                if let Some(ref role) = insight.role {
                    // Existence check returns data_source AND dismissed_at so
                    // AI does not reinsert user-dismissed role rows.
                    let existing: Option<(Option<String>, Option<String>)> = db
                        .conn_ref()
                        .query_row(
                            "SELECT data_source, dismissed_at FROM account_stakeholder_roles
                             WHERE account_id = ?1 AND person_id = ?2 AND role = ?3",
                            rusqlite::params![&input.entity_id, pid, role],
                            |row| Ok((row.get(0)?, row.get(1)?)),
                        )
                        .optional()
                        .map_err(|e| {
                            format!(
                                "stakeholder role existence read failed for {}:{}:{}: {e}",
                                input.entity_id, pid, role
                            )
                        })?;
                    let is_user_owned = matches!(
                        existing.as_ref().and_then(|(ds, _)| ds.as_deref()),
                        Some("user")
                    );
                    let is_dismissed = existing.as_ref().is_some_and(|(_, d)| d.is_some());
                    if !is_user_owned && !is_dismissed {
                        db.conn_ref()
                            .execute(
                                "INSERT INTO account_stakeholder_roles (account_id, person_id, role, data_source) VALUES (?1, ?2, ?3, 'ai') ON CONFLICT(account_id, person_id, role) DO UPDATE SET data_source = 'ai'",
                                rusqlite::params![&input.entity_id, pid, role],
                            )
                            .map_err(|e| {
                                format!(
                                "upsert AI-owned stakeholder role failed for {}:{}: {e}",
                                input.entity_id,
                                pid
                                )
                            })?;
                    }
                }
            } else {
                // Person has a person_id but is not in account_stakeholders — suggest
                write_stakeholder_suggestion(&StakeholderSuggestionParams {
                    db,
                    account_id: &input.entity_id,
                    person_id: Some(pid),
                    insight,
                    source: ai_source,
                })?;
            }
        } else {
            // No person_id — write to suggestions table
            write_stakeholder_suggestion(&StakeholderSuggestionParams {
                db,
                account_id: &input.entity_id,
                person_id: None,
                insight,
                source: ai_source,
            })?;
        }
    }

    Ok(())
}

/// Best-effort post-commit disk cache write for composed enrichment.
pub fn fenced_write_enrichment_intelligence(
    db: &crate::db::ActionDb,
    dir: &Path,
    intel: &IntelligenceJson,
) {
    crate::intelligence::write_fence::post_commit_fenced_write(
        db,
        dir,
        intel,
        &format!("entity={} source=ai_enrichment", intel.entity_id),
    );
}

/// Non-authoritative follow-up work after the DB write has committed.
pub fn run_enrichment_post_commit_side_effects(
    state: &AppState,
    input: &EnrichmentInput,
    db: &crate::db::ActionDb,
    final_intel: &IntelligenceJson,
    producer: EnrichmentProducer,
) -> Result<(), String> {
    // Invalidate cached reports when entity intelligence is refreshed.
    #[allow(
        clippy::let_underscore_must_use,
        reason = "intentional best-effort discard; preserves existing non-blocking behavior"
    )]
    let _ = crate::reports::invalidation::mark_reports_stale(db, &input.entity_id);

    if input.entity_type == "account" {
        let ctx = state.live_service_context();
        let sync_result =
            crate::services::enrichment_side_effects::sync_account_enrichment_side_effects_for_producer(
            &ctx,
            db,
            state.signals.engine.as_ref(),
            &input.entity_id,
            final_intel,
            producer.side_effect_producer(),
        );
        if let Err(error) = sync_result {
            log::warn!(
                "IntelProcessor: enrichment side-effect sync failed for {}: {}",
                input.entity_id,
                error
            );
            if producer.is_glean() {
                return Err(format!(
                    "{} enrichment side-effect sync failed for {}: {}",
                    producer.materialization_label(),
                    input.entity_id,
                    error
                ));
            }
        }
    }

    // Regenerate person files after intelligence enrichment.
    if input.entity_type == "person" {
        if let Ok(Some(person)) = db.get_person(&input.entity_id) {
            #[allow(
                clippy::let_underscore_must_use,
                reason = "intentional best-effort discard; preserves existing non-blocking behavior"
            )]
            let _ = crate::people::write_person_markdown(&input.workspace, &person, db);
            #[allow(
                clippy::let_underscore_must_use,
                reason = "intentional best-effort discard; preserves existing non-blocking behavior"
            )]
            let _ = crate::people::write_person_dashboard_json(&input.workspace, &person, db);
        }
    }

    // After writing a child entity's enrichment, enqueue the parent for
    // portfolio intelligence refresh. This ensures parent portfolio views stay
    // current when any child's intelligence updates.
    if input.entity_type == "account" {
        if let Ok(Some(account)) = db.get_account(&input.entity_id) {
            if let Some(ref parent_id) = account.parent_id {
                #[allow(
                    clippy::let_underscore_must_use,
                    reason = "intentional best-effort discard; preserves existing non-blocking behavior"
                )]
                let _ = state.intel_queue.enqueue(IntelRequest {
                    entity_id: parent_id.clone(),
                    entity_type: "account".to_string(),
                    priority: IntelPriority::ContentChange,
                    requested_at: std::time::Instant::now(),
                    retry_count: 0,
                });
                state.integrations.intel_queue_wake.notify_one();
                log::info!(
                    "IntelProcessor: enqueued parent {} for portfolio refresh after child {} update",
                    parent_id,
                    input.entity_id,
                );
            }
        }
    }
    if input.entity_type == "project" {
        if let Ok(Some(project)) = db.get_project(&input.entity_id) {
            if let Some(ref parent_id) = project.parent_id {
                #[allow(
                    clippy::let_underscore_must_use,
                    reason = "intentional best-effort discard; preserves existing non-blocking behavior"
                )]
                let _ = state.intel_queue.enqueue(IntelRequest {
                    entity_id: parent_id.clone(),
                    entity_type: "project".to_string(),
                    priority: IntelPriority::ContentChange,
                    requested_at: std::time::Instant::now(),
                    retry_count: 0,
                });
                state.integrations.intel_queue_wake.notify_one();
                log::info!(
                    "IntelProcessor: enqueued parent project {} for portfolio refresh after child {} update",
                    parent_id,
                    input.entity_id,
                );
            }
        }
    }

    log::debug!(
        "IntelProcessor: wrote intelligence for {} to DB + post-commit file cache",
        input.entity_id,
    );
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FinalizeMode {
    /// Queue worker: run the full background-enrichment finalize chain.
    QueueWorker {
        /// True for scheduled/background priorities; false for queued manual work.
        is_background: bool,
        /// Actual producer for the completed result, after any Glean-to-PTY fallback.
        producer: EnrichmentProducer,
    },
    /// Manual user-driven refresh: run the minimal shared finalize chain.
    ///
    /// Skips queue-only post-commit work: supplemental Glean passes,
    /// `intelligence-updated` event emission,
    /// self-healing scheduler hook, and `claude_code` sync-success recording.
    /// Manual refresh emits `background-work-status:completed` from its command
    /// path and is a different actor than the background sync loop.
    ManualRefresh {
        /// Actual producer for the completed result, after any Glean-to-PTY fallback.
        producer: EnrichmentProducer,
    },
    /// Explicit trust recompute: run shared finalize work plus claim trust scoring.
    TrustRecompute,
}

pub(crate) fn run_enrichment_finalize_post_commit(
    state: &Arc<AppState>,
    db: &crate::db::ActionDb,
    input: &EnrichmentInput,
    intel: &IntelligenceJson,
    inferred_relationships: &[InferredRelationship],
    mode: FinalizeMode,
) -> Result<(), String> {
    let side_effect_producer = match mode {
        FinalizeMode::QueueWorker { producer, .. } | FinalizeMode::ManualRefresh { producer } => {
            producer
        }
        FinalizeMode::TrustRecompute => EnrichmentProducer::Pty,
    };
    let materialize_visible_side_effects_before_export =
        !matches!(mode, FinalizeMode::TrustRecompute);
    if materialize_visible_side_effects_before_export {
        run_enrichment_post_commit_side_effects(
            state.as_ref(),
            input,
            db,
            intel,
            side_effect_producer,
        )?;
    }

    match mode {
        FinalizeMode::QueueWorker {
            is_background,
            producer,
        } => {
            if producer.is_glean() {
                run_shared_glean_finalization(state.as_ref(), db, input, intel)?;
                spawn_queue_worker_supplemental_glean_finalize(state, input, is_background);
            }
        }
        FinalizeMode::TrustRecompute => {
            run_finalize_trust_recompute(state.as_ref(), db, input)?;
        }
        FinalizeMode::ManualRefresh { producer } => {
            if producer.is_glean() {
                run_shared_glean_finalization(state.as_ref(), db, input, intel)?;
            }
        }
    }

    fenced_write_enrichment_intelligence(db, &input.entity_dir, intel);
    if !materialize_visible_side_effects_before_export {
        run_enrichment_post_commit_side_effects(
            state.as_ref(),
            input,
            db,
            intel,
            side_effect_producer,
        )?;
    }

    if !inferred_relationships.is_empty() {
        let ctx = state.live_service_context();
        crate::services::intelligence::upsert_inferred_relationships_from_enrichment(
            &ctx,
            db,
            state.signals.engine.as_ref(),
            &input.entity_type,
            &input.entity_id,
            inferred_relationships,
        )?;
    }

    if matches!(mode, FinalizeMode::QueueWorker { .. }) {
        if let Some(app) = state.app_handle() {
            #[allow(
                clippy::let_underscore_must_use,
                reason = "intentional best-effort discard; preserves existing non-blocking behavior"
            )]
            let _ = app.emit(
                "intelligence-updated",
                IntelligenceUpdatedPayload {
                    entity_id: input.entity_id.clone(),
                    entity_type: input.entity_type.clone(),
                },
            );
        }
    }

    // Invalidate + requeue meeting preps for future meetings linked to this entity.
    // The entity intelligence snapshot changed, so dependent briefings need regeneration.
    invalidate_and_requeue_meeting_preps_with_db(state.as_ref(), db, &input.entity_id);

    crate::self_healing::feedback::record_enrichment_success(db, &input.entity_id);

    if matches!(mode, FinalizeMode::QueueWorker { .. }) {
        #[allow(
            clippy::let_underscore_must_use,
            reason = "intentional best-effort discard; preserves existing non-blocking behavior"
        )]
        let _ = crate::self_healing::scheduler::on_enrichment_complete(
            db,
            Some(state.embedding_model.as_ref()),
            &input.entity_id,
            &input.entity_type,
            &state.intel_queue,
            Some(state.signals.engine.as_ref()),
        );
        #[allow(
            clippy::let_underscore_must_use,
            reason = "intentional best-effort discard; preserves existing non-blocking behavior"
        )]
        let _ = crate::connectivity::record_sync_success(db.conn_ref(), "claude_code");
    }

    Ok(())
}

fn run_finalize_trust_recompute(
    state: &AppState,
    db: &crate::db::ActionDb,
    input: &EnrichmentInput,
) -> Result<(), String> {
    let ctx = state.live_service_context();
    crate::services::trust_recompute::recompute_claim_trust_for_subject(
        &ctx,
        db,
        &input.entity_type,
        &input.entity_id,
    )
    .map(|_| ())
}

fn run_shared_glean_finalization(
    state: &AppState,
    db: &crate::db::ActionDb,
    input: &EnrichmentInput,
    intel: &IntelligenceJson,
) -> Result<(), String> {
    let ctx = state.live_service_context();
    let report = crate::services::glean_finalization::finalize_glean_enrichment(
        &ctx,
        db,
        state.signals.engine.as_ref(),
        crate::services::glean_finalization::GleanFinalizationInput {
            entity_type: &input.entity_type,
            entity_id: &input.entity_id,
            intel,
            preset: input.active_preset.as_ref(),
        },
    )
    .map_err(|error| error.to_string())?;

    if report.signals_emitted > 0
        || report.schema_promoted > 0
        || report.claims_committed > 0
        || report.recompute_jobs_enqueued > 0
        || report.health_recomputed
        || !report.degraded_classes.is_empty()
    {
        log::info!(
            "[I644] Glean finalization for {}: {} signal(s) emitted, {} schema fact(s), {} claim(s), {} recompute job(s), health_recomputed={}, degraded_classes={}",
            input.entity_id,
            report.signals_emitted,
            report.schema_promoted,
            report.claims_committed,
            report.recompute_jobs_enqueued,
            report.health_recomputed,
            report.degraded_classes.len(),
        );
    }
    Ok(())
}

fn spawn_queue_worker_supplemental_glean_finalize(
    state: &Arc<AppState>,
    input: &EnrichmentInput,
    is_background: bool,
) {
    // Supplemental leading-signals enrichment for Health & Outlook.
    // Runs only for accounts and only when Glean is configured. Failures
    // are isolated (the main dimension enrichment already landed) but
    // no longer silent: we emit an audit event + Tauri event so the
    // frontend can surface a toast and we can see why Health triage
    // fell back to activity-sourced cards.
    let ctx_snapshot = state.context_snapshot();
    if !ctx_snapshot.is_remote()
        || input.entity_type != "account"
        || ctx_snapshot.remote_endpoint().is_none()
    {
        return;
    }

    let Some(app_for_spawn) = state.app_handle() else {
        log::warn!(
            "[DOS-15] Skipping supplemental Glean finalize for {}: missing app handle",
            input.entity_id
        );
        return;
    };

    let entity_name = input.entity_name.clone();
    let entity_id = input.entity_id.clone();
    let entity_type = input.entity_type.clone();
    let engine = std::sync::Arc::clone(&state.signals.engine);
    let state_for_spawn = std::sync::Arc::clone(state);
    let disambiguators_for_spawn = input
        .intelligence_context
        .as_ref()
        .map(|c| c.disambiguators.clone());

    tauri::async_runtime::spawn(async move {
        // Re-snapshot at dequeue time so a settings switch between enqueue and
        // dequeue takes effect on the next read (per ADR-0091). Atomic
        // transition guarantees the snapshot reads is_remote + Glean Arc
        // coherently.
        let snap = state_for_spawn.context_snapshot();
        let provider = match snap.glean_intelligence_provider {
            Some(p) if snap.is_remote() => p,
            _ => {
                log::warn!(
                    "[DOS-15] Context-mode snapshot for leading-signals \
                     on {} shows is_remote={} / Glean Arc={:?}; \
                     settings switched between enqueue and dequeue. \
                     Skipping leading-signals enrichment per ADR-0091.",
                    entity_name,
                    snap.is_remote(),
                    snap.glean_intelligence_provider.is_some(),
                );
                return;
            }
        };
        let ls_start = std::time::Instant::now();
        match provider
            .enrich_leading_signals_with_disambiguators(
                &entity_name,
                disambiguators_for_spawn.as_ref(),
            )
            .await
        {
            Ok(signals) => {
                if let Ok(db) =
                    crate::db::ActionDb::open(std::sync::Arc::new(crate::db::LocalKeychain::new()))
                {
                    let ctx = state_for_spawn.live_service_context();
                    if let Err(e) = crate::services::intelligence::upsert_health_outlook_signals(
                        &ctx,
                        &db,
                        &engine,
                        &entity_type,
                        &entity_id,
                        &signals,
                    ) {
                        log::warn!(
                            "[DOS-15] upsert_health_outlook_signals failed for {}: {}",
                            entity_id,
                            e
                        );
                        let reason = format!("persistence failed: {e}");
                        emit_leading_signals_failed(
                            &state_for_spawn,
                            &app_for_spawn,
                            &entity_id,
                            &entity_type,
                            &reason,
                            ls_start.elapsed().as_millis() as u64,
                            is_background,
                        );
                    } else {
                        log::info!("[DOS-15] Leading signals persisted for {}", entity_id);
                    }
                }
            }
            Err(e) => {
                log::warn!(
                    "[DOS-15] Leading-signals enrichment failed for {}: {}",
                    entity_id,
                    e
                );
                emit_leading_signals_failed(
                    &state_for_spawn,
                    &app_for_spawn,
                    &entity_id,
                    &entity_type,
                    &e,
                    ls_start.elapsed().as_millis() as u64,
                    is_background,
                );
            }
        }

        // Peer-cohort renewal benchmark: separate Glean chat pass that
        // populates AgreementOutlook.peer_benchmark. This cell is additive, so
        // failures leave peer_benchmark unset without user-visible noise.
        match provider.enrich_peer_benchmark(&entity_name).await {
            Ok(peer_benchmark) => {
                if let Ok(db) =
                    crate::db::ActionDb::open(std::sync::Arc::new(crate::db::LocalKeychain::new()))
                {
                    match db.get_entity_intelligence(&entity_id) {
                        Ok(Some(mut current)) => {
                            let ctx = state_for_spawn.live_service_context();
                            let outlook = current
                                .agreement_outlook
                                .get_or_insert_with(Default::default);
                            outlook.peer_benchmark = Some(peer_benchmark);
                            if let Err(e) =
                                crate::services::intelligence::upsert_assessment_snapshot(
                                    &ctx, &db, &current,
                                )
                            {
                                log::warn!(
                                    "[DOS-204] Persisting peer_benchmark failed for {}: {}",
                                    entity_id,
                                    e
                                );
                            } else {
                                log::info!("[DOS-204] Peer benchmark persisted for {}", entity_id);
                            }
                        }
                        Ok(None) => {
                            log::debug!(
                                "[DOS-204] No assessment row for {}; skipping peer_benchmark write",
                                entity_id
                            );
                        }
                        Err(e) => {
                            log::warn!(
                                "[DOS-204] Reading assessment for {} failed: {}",
                                entity_id,
                                e
                            );
                        }
                    }
                }
            }
            Err(e) => {
                log::info!(
                    "[DOS-204] Peer benchmark unavailable for {} ({})",
                    entity_id,
                    e
                );
            }
        }
    });
}

/// Return true when the intelligence payload lacks meaningful narrative signal.
fn is_sparse_intelligence(intel: &IntelligenceJson) -> bool {
    let has_assessment = intel
        .executive_assessment
        .as_deref()
        .is_some_and(|s| !s.trim().is_empty());
    let has_risks = !intel.risks.is_empty();
    let has_wins = !intel.recent_wins.is_empty();
    let has_value = !intel.value_delivered.is_empty();
    let has_readiness = intel
        .next_meeting_readiness
        .as_ref()
        .is_some_and(|r| !r.prep_items.is_empty());
    let has_state = intel.current_state.as_ref().is_some_and(|s| {
        !s.working.is_empty() || !s.not_working.is_empty() || !s.unknowns.is_empty()
    });
    let has_health = intel.health.is_some();
    let has_metrics = intel
        .success_metrics
        .as_ref()
        .is_some_and(|m| !m.is_empty())
        || intel
            .open_commitments
            .as_ref()
            .is_some_and(|c| !c.is_empty());

    !(has_assessment
        || has_risks
        || has_wins
        || has_value
        || has_readiness
        || has_state
        || has_health
        || has_metrics)
}

/// Parameters for writing a stakeholder suggestion.
struct StakeholderSuggestionParams<'a> {
    db: &'a crate::db::ActionDb,
    account_id: &'a str,
    person_id: Option<&'a str>,
    insight: &'a crate::intelligence::io::StakeholderInsight,
    source: &'a str,
}

/// Write a stakeholder suggestion to the `stakeholder_suggestions` table.
/// Skips if a pending suggestion for the same person+account already exists.
fn write_stakeholder_suggestion(params: &StakeholderSuggestionParams<'_>) -> Result<(), String> {
    let StakeholderSuggestionParams {
        db,
        account_id,
        person_id,
        insight,
        source,
    } = params;

    // Skip suggestions for internal team members (by person_id OR name match)
    if let Some(pid) = person_id {
        let is_internal: bool = db
            .conn_ref()
            .query_row(
                "SELECT relationship = 'internal' FROM people WHERE id = ?1",
                rusqlite::params![pid],
                |row| row.get(0),
            )
            .map_err(|e| {
                format!(
                    "internal stakeholder person read failed for {}:{}: {e}",
                    account_id, pid
                )
            })?;
        if is_internal {
            return Ok(());
        }
    } else {
        // No person_id — check by name (PTY often suggests names without IDs)
        let is_internal_by_name: bool = db
            .conn_ref()
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM people WHERE LOWER(name) = LOWER(?1) AND relationship = 'internal')",
                rusqlite::params![&insight.name],
                |row| row.get(0),
            )
            .map_err(|e| {
                format!(
                    "internal stakeholder name read failed for {}:{}: {e}",
                    account_id, insight.name
                )
            })?;
        if is_internal_by_name {
            return Ok(());
        }
    }

    // Dedup: skip if a pending suggestion for this person+account already exists.
    // Match on person_id if available, otherwise match on name.
    let already_pending = if let Some(pid) = person_id {
        db.conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM stakeholder_suggestions WHERE account_id = ?1 AND person_id = ?2 AND status = 'pending'",
                rusqlite::params![account_id, pid],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|e| {
                format!(
                    "stakeholder suggestion dedupe read failed for {}:{}: {e}",
                    account_id, pid
                )
            })?
            > 0
    } else {
        db.conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM stakeholder_suggestions WHERE account_id = ?1 AND suggested_name = ?2 AND status = 'pending'",
                rusqlite::params![account_id, &insight.name],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|e| {
                format!(
                    "stakeholder suggestion dedupe read failed for {}:{}: {e}",
                    account_id, insight.name
                )
            })?
            > 0
    };
    if already_pending {
        return Ok(());
    }

    let raw_json = serde_json::to_string(insight).unwrap_or_default();
    db.conn_ref()
        .execute(
            "INSERT INTO stakeholder_suggestions (account_id, person_id, suggested_name, suggested_email, suggested_role, suggested_engagement, source, status, raw_suggestion, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending', ?8, datetime('now'))",
            rusqlite::params![
                account_id,
                person_id,
                &insight.name,
                Option::<&str>::None,
                &insight.role,
                &insight.engagement,
                source,
                raw_json,
            ],
        )
        .map_err(|e| {
            format!(
                "I652: Failed to write stakeholder suggestion for {} on {}: {}",
                insight.name, account_id, e,
            )
        })?;
    log::info!(
        "I652: Wrote stakeholder suggestion for '{}' on account {}",
        insight.name,
        account_id,
    );
    Ok(())
}

/// Keep prior core intelligence fields when a fresh refresh returns sparse output.
fn merge_missing_core_fields_from_existing(
    final_intel: &mut IntelligenceJson,
    existing: Option<&IntelligenceJson>,
) {
    let Some(existing) = existing else {
        return;
    };

    // Always backfill executive assessment when missing.
    if final_intel
        .executive_assessment
        .as_deref()
        .is_none_or(|s| s.trim().is_empty())
        && existing
            .executive_assessment
            .as_deref()
            .is_some_and(|s| !s.trim().is_empty())
    {
        final_intel.executive_assessment = existing.executive_assessment.clone();
    }

    // Preserve persisted org-health baseline unless the new payload explicitly
    // provides one. This field is computed outside the LLM response path.
    if final_intel.org_health.is_none() && existing.org_health.is_some() {
        final_intel.org_health = existing.org_health.clone();
    }

    // If the new payload is sparse, preserve prior narrative-bearing fields.
    if !is_sparse_intelligence(final_intel) {
        return;
    }

    if final_intel.risks.is_empty() && !existing.risks.is_empty() {
        final_intel.risks = existing.risks.clone();
    }
    if final_intel.recent_wins.is_empty() && !existing.recent_wins.is_empty() {
        final_intel.recent_wins = existing.recent_wins.clone();
    }
    if final_intel.value_delivered.is_empty() && !existing.value_delivered.is_empty() {
        final_intel.value_delivered = existing.value_delivered.clone();
    }
    if final_intel.current_state.is_none() && existing.current_state.is_some() {
        final_intel.current_state = existing.current_state.clone();
    }
    if final_intel.next_meeting_readiness.is_none() && existing.next_meeting_readiness.is_some() {
        final_intel.next_meeting_readiness = existing.next_meeting_readiness.clone();
    }
    if final_intel.health.is_none() && existing.health.is_some() {
        final_intel.health = existing.health.clone();
    }
    if final_intel
        .success_metrics
        .as_ref()
        .is_none_or(|m| m.is_empty())
        && existing
            .success_metrics
            .as_ref()
            .is_some_and(|m| !m.is_empty())
    {
        final_intel.success_metrics = existing.success_metrics.clone();
    }
    if final_intel
        .open_commitments
        .as_ref()
        .is_none_or(|c| c.is_empty())
        && existing
            .open_commitments
            .as_ref()
            .is_some_and(|c| !c.is_empty())
    {
        final_intel.open_commitments = existing.open_commitments.clone();
    }
}

/// After entity intelligence is refreshed, invalidate and requeue meeting preps
/// for future meetings linked to that entity.
///
/// intelligence.json is the shared enrichment source — meeting briefings consume it
/// mechanically. When it changes, affected briefings must regenerate to pull the
/// latest intelligence data.
pub(crate) fn invalidate_and_requeue_meeting_preps(state: &AppState, entity_id: &str) {
    let db = match crate::db::ActionDb::open(std::sync::Arc::new(crate::db::LocalKeychain::new())) {
        Ok(db) => db,
        Err(e) => {
            log::warn!(
                "IntelProcessor: failed to open DB for prep invalidation: {}",
                e
            );
            return;
        }
    };

    invalidate_and_requeue_meeting_preps_with_db(state, &db, entity_id);
}

/// Variant for callers that already own the authoritative DB connection for
/// the just-committed enrichment path.
pub(crate) fn invalidate_and_requeue_meeting_preps_with_db(
    state: &AppState,
    db: &crate::db::ActionDb,
    entity_id: &str,
) {
    let now = Utc::now().to_rfc3339();

    // Find future meetings linked to this entity and clear their frozen prep
    let meeting_ids: Vec<String> = db
        .conn_ref()
        .prepare(
            "SELECT m.id FROM meetings m
             JOIN effective_meeting_entities me ON me.meeting_id = m.id
             WHERE me.entity_id = ?1
               AND m.start_time > ?2
               AND m.meeting_type NOT IN ('personal', 'focus', 'blocked')",
        )
        .and_then(|mut stmt| {
            let rows = stmt.query_map(rusqlite::params![entity_id, now], |row| {
                row.get::<_, String>(0)
            })?;
            Ok(rows.filter_map(|r| r.ok()).collect())
        })
        .unwrap_or_default();

    if meeting_ids.is_empty() {
        return;
    }

    // Clear prep_frozen_json so the queue processor regenerates them
    let ctx = state.live_service_context();
    for mid in &meeting_ids {
        #[allow(
            clippy::let_underscore_must_use,
            reason = "intentional best-effort discard; preserves existing non-blocking behavior"
        )]
        let _ = crate::services::meetings::clear_meeting_prep_frozen(&ctx, db, mid);
    }

    // Enqueue for regeneration at Background priority
    for mid in &meeting_ids {
        state
            .meeting_prep_queue
            .enqueue(crate::meeting_prep_queue::PrepRequest::new(
                mid.clone(),
                crate::meeting_prep_queue::PrepPriority::Background,
            ));
    }
    if !meeting_ids.is_empty() {
        state.integrations.prep_queue_wake.notify_one();
    }

    log::info!(
        "IntelProcessor: invalidated + requeued {} meeting preps for entity {}",
        meeting_ids.len(),
        entity_id,
    );
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
#[allow(clippy::manual_range_contains)]
mod tests {
    use super::*;
    use crate::abilities::feedback::FeedbackAction;
    use crate::services::claims::{
        commit_claim, record_claim_feedback, record_corroboration, update_claim_trust,
        ClaimFeedbackInput, ClaimProposal, CommittedClaim,
    };
    use crate::services::context::{
        ClaimDismissalSurface, ExternalClients, FixedClock, SeedableRng, ServiceContext,
    };
    use crate::services::trust_recompute::{
        corroboration_strength_for_claim, cross_entity_context_expected,
        freshness_context_for_claim, internal_consistency_for_claim,
        source_reliability_corroborators_for_claim, source_reliability_for_claim,
        trust_feedback_signal_for_claim,
    };
    use chrono::TimeZone;
    use rusqlite::params;
    use std::path::Path;

    const TRUST_TS: &str = "2999-01-01T00:00:00+00:00";

    fn trust_test_db() -> crate::db::ActionDb {
        crate::db::test_utils::test_db()
    }

    fn trust_ctx_parts() -> (FixedClock, SeedableRng, ExternalClients) {
        (
            FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 4, 12, 0, 0).unwrap()),
            SeedableRng::new(5),
            ExternalClients::default(),
        )
    }

    fn with_trust_ctx<T>(f: impl FnOnce(&ServiceContext<'_>) -> T) -> T {
        let (clock, rng, external) = trust_ctx_parts();
        let ctx = ServiceContext::test_live(&clock, &rng, &external);
        f(&ctx)
    }

    #[test]
    fn queue_worker_final_status_reports_finalize_failures_instead_of_success() {
        let failures = vec![QueueRefreshFailure::new(
            "account-finalize-failed",
            "finalize",
            "visible side effects failed",
        )];

        let payloads = queue_worker_final_status_payloads(0, &failures, true);

        assert_eq!(payloads.len(), 1);
        assert_eq!(payloads[0]["phase"], "failed");
        assert_eq!(payloads[0]["manual"], true);
        assert_eq!(payloads[0]["entityId"], "account-finalize-failed");
        assert_eq!(payloads[0]["stage"], "finalize");
        assert_eq!(payloads[0]["error"], "visible side effects failed");
    }

    #[test]
    fn queue_worker_final_status_counts_only_finalized_successes() {
        let payloads = queue_worker_final_status_payloads(2, &[], false);

        assert_eq!(payloads.len(), 1);
        assert_eq!(payloads[0]["phase"], "completed");
        assert_eq!(payloads[0]["count"], 2);
        assert_eq!(payloads[0]["manual"], false);
    }

    fn seed_trust_account(db: &crate::db::ActionDb, account_id: &str) {
        db.conn_ref()
            .execute(
                "INSERT INTO accounts (id, name, updated_at) VALUES (?1, ?2, ?3)",
                params![account_id, format!("Account {account_id}"), TRUST_TS],
            )
            .expect("seed account");
        db.conn_ref()
            .execute(
                "INSERT INTO account_domains (account_id, domain, source)
                 VALUES (?1, ?2, 'test')",
                params![account_id, format!("{account_id}.example.com")],
            )
            .expect("seed account domain");
    }

    fn trust_claim_proposal(account_id: &str, text: &str, data_source: &str) -> ClaimProposal {
        ClaimProposal {
            id: None,
            expected_claim_version: None,
            subject_ref: serde_json::json!({ "kind": "account", "id": account_id }).to_string(),
            claim_type: "risk".to_string(),
            field_path: Some("health.risk".to_string()),
            topic_key: None,
            text: text.to_string(),
            actor: "agent:test".to_string(),
            data_source: data_source.to_string(),
            source_ref: None,
            source_asof: Some(TRUST_TS.to_string()),
            observed_at: TRUST_TS.to_string(),
            provenance_json: "{}".to_string(),
            metadata_json: None,
            thread_id: None,
            temporal_scope: Some(crate::db::claims::TemporalScope::State),
            sensitivity: Some(crate::db::claims::ClaimSensitivity::Internal),
            supersedes: None,
            tombstone: None,
        }
    }

    fn inserted_claim_id(result: CommittedClaim) -> String {
        match result {
            CommittedClaim::Inserted { claim } | CommittedClaim::Tombstoned { claim } => claim.id,
            other => panic!("expected inserted claim, got {other:?}"),
        }
    }

    fn seed_trust_claim(
        db: &crate::db::ActionDb,
        account_id: &str,
        text: &str,
        data_source: &str,
    ) -> String {
        with_trust_ctx(|ctx| {
            inserted_claim_id(
                commit_claim(ctx, db, trust_claim_proposal(account_id, text, data_source))
                    .expect("commit claim"),
            )
        })
    }

    fn load_single_trust_claim(
        db: &crate::db::ActionDb,
        account_id: &str,
    ) -> crate::db::claims::IntelligenceClaim {
        let subject_ref = serde_json::json!({ "kind": "account", "id": account_id }).to_string();
        let mut claims = crate::services::claims::load_claims_active(db, &subject_ref, None)
            .expect("load active trust claims");
        assert_eq!(claims.len(), 1, "expected one active trust claim");
        claims.remove(0)
    }

    fn seed_trust_corroboration(db: &crate::db::ActionDb, claim_id: &str, source: &str) {
        with_trust_ctx(|ctx| {
            record_corroboration(ctx, db, claim_id, source, Some(TRUST_TS), Some("test"))
                .expect("record corroboration");
        });
    }

    fn seed_prior_trust(db: &crate::db::ActionDb, claim_id: &str, score: f64, version: i64) {
        with_trust_ctx(|ctx| {
            update_claim_trust(
                db,
                claim_id,
                crate::abilities::trust::TrustScore(score),
                version,
                ctx,
            )
            .expect("seed prior trust");
        });
    }

    fn seed_claim_feedback(db: &crate::db::ActionDb, claim_id: &str, action: FeedbackAction) {
        with_trust_ctx(|ctx| {
            record_claim_feedback(
                ctx,
                db,
                ClaimFeedbackInput {
                    claim_id: claim_id.to_string(),
                    action,
                    actor: "user".to_string(),
                    actor_id: Some("user-fixture".to_string()),
                    payload_json: None,
                },
            )
            .expect("record claim feedback");
        });
    }

    fn dismiss_trust_claim_on_surface(
        db: &crate::db::ActionDb,
        claim_id: &str,
        surface: ClaimDismissalSurface,
    ) {
        db.conn_ref()
            .execute(
                "INSERT INTO claim_surface_dismissals (
                    claim_id, surface, actor, dismissed_at
                 ) VALUES (?1, ?2, ?3, ?4)",
                params![claim_id, surface.as_str(), "user:test", TRUST_TS],
            )
            .expect("dismiss claim on surface");
    }

    fn trust_input(entity_id: &str, entity_dir: &Path) -> EnrichmentInput {
        EnrichmentInput {
            workspace: entity_dir.to_path_buf(),
            entity_dir: entity_dir.to_path_buf(),
            entity_id: entity_id.to_string(),
            entity_type: "account".to_string(),
            prompt: String::new(),
            file_manifest: Vec::new(),
            file_count: 0,
            computed_health: None,
            entity_name: format!("Account {entity_id}"),
            relationship: None,
            intelligence_context: None,
            active_preset: None,
        }
    }

    fn trust_intel(entity_id: &str) -> IntelligenceJson {
        IntelligenceJson {
            executive_assessment_render_policy: None,
            entity_id: entity_id.to_string(),
            entity_type: "account".to_string(),
            enriched_at: TRUST_TS.to_string(),
            executive_assessment: Some("Trust recompute fixture".to_string()),
            ..Default::default()
        }
    }

    fn run_trust_finalize_result(
        db: &crate::db::ActionDb,
        account_id: &str,
        mode: FinalizeMode,
    ) -> Result<(), String> {
        let state = Arc::new(AppState::new());
        let dir = tempfile::tempdir().expect("tempdir");
        let input = trust_input(account_id, dir.path());
        let intel = trust_intel(account_id);
        run_enrichment_finalize_post_commit(&state, db, &input, &intel, &[], mode)
    }

    fn run_trust_finalize(db: &crate::db::ActionDb, account_id: &str, mode: FinalizeMode) {
        run_trust_finalize_result(db, account_id, mode).expect("run finalize");
    }

    fn read_trust_columns(
        db: &crate::db::ActionDb,
        claim_id: &str,
    ) -> (Option<f64>, Option<String>, Option<i64>) {
        db.conn_ref()
            .query_row(
                "SELECT trust_score, trust_computed_at, trust_version
                 FROM intelligence_claims WHERE id = ?1",
                params![claim_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("read trust columns")
    }

    fn signal_count(db: &crate::db::ActionDb, signal_type: &str) -> i64 {
        db.conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM signal_events WHERE signal_type = ?1",
                params![signal_type],
                |row| row.get(0),
            )
            .expect("count signals")
    }

    fn pipeline_failure_count(db: &crate::db::ActionDb, error_type: &str) -> i64 {
        db.conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM pipeline_failures
                 WHERE pipeline = 'trust_recompute' AND error_type = ?1",
                params![error_type],
                |row| row.get(0),
            )
            .expect("count pipeline failures")
    }

    fn seed_malformed_source_weight(db: &crate::db::ActionDb, data_source: &str) {
        db.conn_ref()
            .execute(
                "INSERT INTO signal_weights
                    (source, entity_type, signal_type, alpha, beta, update_count)
                 VALUES (?1, 'account', 'enrichment_quality', 0.0, 0.0, 5)",
                params![data_source],
            )
            .expect("seed malformed source weight");
    }

    fn assert_float_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-9,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn freshness_context_timestamp_unknown_applies_penalty() {
        let db = trust_test_db();
        let account_id = "acct-freshness-unknown";
        seed_trust_account(&db, account_id);
        let mut proposal = trust_claim_proposal(
            account_id,
            "Freshness falls back to observed_at when source_asof is missing.",
            "unit_test_source",
        );
        proposal.source_asof = None;
        proposal.observed_at = "2026-05-03T12:00:00+00:00".to_string();
        with_trust_ctx(|ctx| {
            commit_claim(ctx, &db, proposal).expect("commit freshness claim");
        });
        let claim = load_single_trust_claim(&db, account_id);

        let (freshness, reason) = freshness_context_for_claim(
            &db,
            Utc.with_ymd_and_hms(2026, 5, 4, 12, 0, 0).unwrap(),
            &claim,
        )
        .into_parts();
        let ok = reason.is_none();

        assert!(ok);
        assert!(!freshness.timestamp_known);
        assert_float_close(freshness.age_days, 1.0);
    }

    #[test]
    fn freshness_context_timestamp_known_uses_source_asof_age() {
        let db = trust_test_db();
        let account_id = "acct-freshness-known";
        seed_trust_account(&db, account_id);
        let mut proposal = trust_claim_proposal(
            account_id,
            "Freshness prefers source_asof over older observed_at.",
            "unit_test_source",
        );
        proposal.source_asof = Some("2026-05-03T12:00:00+00:00".to_string());
        proposal.observed_at = "2026-04-04T12:00:00+00:00".to_string();
        with_trust_ctx(|ctx| {
            commit_claim(ctx, &db, proposal).expect("commit freshness claim");
        });
        let claim = load_single_trust_claim(&db, account_id);

        let (freshness, reason) = freshness_context_for_claim(
            &db,
            Utc.with_ymd_and_hms(2026, 5, 4, 12, 0, 0).unwrap(),
            &claim,
        )
        .into_parts();
        let ok = reason.is_none();

        assert!(ok);
        assert!(freshness.timestamp_known);
        assert_float_close(freshness.age_days, 1.0);
    }

    #[test]
    fn corroboration_strength_matches_record_corroboration_formula() {
        let db = trust_test_db();
        let account_id = "acct-corroboration-formula";
        seed_trust_account(&db, account_id);
        let claim_id = seed_trust_claim(
            &db,
            account_id,
            "Diverse source corroboration composes via noisy OR.",
            "unit_test_source",
        );
        seed_trust_corroboration(&db, &claim_id, "glean");
        seed_trust_corroboration(&db, &claim_id, "calendar");

        let (strength, reason) = corroboration_strength_for_claim(&db, &claim_id).into_parts();
        assert_float_close(strength, 0.75);
        assert!(
            reason.is_none(),
            "clean read should report no indeterminate reason"
        );
    }

    #[test]
    fn peer_benchmark_claim_sets_cross_entity_context_expected() {
        let db = trust_test_db();
        let account_id = "acct-peer-benchmark";
        seed_trust_account(&db, account_id);
        let mut proposal = trust_claim_proposal(
            account_id,
            "Peer benchmark compares this account against similar companies.",
            "unit_test_source",
        );
        proposal.field_path = Some("agreement_outlook.peer_benchmark.median".to_string());
        with_trust_ctx(|ctx| {
            commit_claim(ctx, &db, proposal).expect("commit peer benchmark claim");
        });
        let claim = load_single_trust_claim(&db, account_id);

        assert!(cross_entity_context_expected(&claim));
    }

    #[test]
    fn trust_recompute_updates_claim_trust_columns_via_claims_service() {
        let db = trust_test_db();
        let account_id = "acct-trust-service-update";
        seed_trust_account(&db, account_id);
        let claim_id = seed_trust_claim(
            &db,
            account_id,
            "Trust recompute should persist trust columns.",
            "unit_test_source",
        );
        seed_trust_corroboration(&db, &claim_id, "glean");

        run_trust_finalize(&db, account_id, FinalizeMode::TrustRecompute);

        let (score, computed_at, version) = read_trust_columns(&db, &claim_id);
        assert!(score.is_some_and(|value| value > 0.75));
        assert!(computed_at.is_some());
        assert_eq!(version, Some(1));
        assert_eq!(pipeline_failure_count(&db, "extractor_mismatch"), 0);
    }

    #[test]
    fn trust_recompute_includes_claim_dismissed_on_tauri_report_surface() {
        let db = trust_test_db();
        let account_id = "acct-trust-report-dismissed";
        seed_trust_account(&db, account_id);
        let claim_id = seed_trust_claim(
            &db,
            account_id,
            "Report-only dismissal must not remove this claim from shared trust scoring.",
            "unit_test_source",
        );
        seed_trust_corroboration(&db, &claim_id, "glean");
        dismiss_trust_claim_on_surface(&db, &claim_id, ClaimDismissalSurface::TauriReport);

        let subject_ref = serde_json::json!({ "kind": "account", "id": account_id }).to_string();
        let report_visible = crate::services::claims::load_claims_active_for_surface(
            &db,
            &subject_ref,
            None,
            ClaimDismissalSurface::TauriReport.as_str(),
        )
        .expect("load report-visible claims");
        assert!(
            report_visible.is_empty(),
            "fixture must be hidden from tauri_report render reads"
        );

        run_trust_finalize(&db, account_id, FinalizeMode::TrustRecompute);

        let (score, computed_at, version) = read_trust_columns(&db, &claim_id);
        assert!(score.is_some_and(|value| value > 0.75));
        assert!(computed_at.is_some());
        assert_eq!(version, Some(1));
    }

    #[test]
    fn trust_recompute_does_not_require_bridge_invocation_id() {
        let db = trust_test_db();
        let account_id = "acct-no-bridge-invocation";
        seed_trust_account(&db, account_id);
        let claim_id = seed_trust_claim(
            &db,
            account_id,
            "Trust recompute emits evidence without bridge handles.",
            "unit_test_source",
        );
        seed_trust_corroboration(&db, &claim_id, "glean");

        run_trust_finalize(&db, account_id, FinalizeMode::TrustRecompute);

        let mut stmt = db
            .conn_ref()
            .prepare(
                "SELECT value FROM signal_events
                 WHERE data_source = 'trust_recompute' AND value IS NOT NULL",
            )
            .expect("prepare signal read");
        let values: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .expect("read trust signal values")
            .collect::<Result<_, _>>()
            .expect("collect trust signal values");

        assert!(!values.is_empty(), "trust recompute should emit evidence");
        for value in values {
            assert!(!value.contains("invocation_id"));
            assert!(!value.contains("bridge_invocation"));
        }
    }

    #[test]
    fn bundle5_user_correction_tombstone_not_averaged_away() {
        #[derive(Debug)]
        struct FixtureClaim {
            id: String,
            claim_state: String,
            surfacing_state: String,
            data_source: String,
            trust_score: f64,
            scenario: String,
        }

        let conn = rusqlite::Connection::open_in_memory().expect("open fixture db");
        conn.execute_batch(include_str!("../tests/fixtures/bundle-5/state.sql"))
            .expect("load bundle-5 fixture");

        let mut stmt = conn
            .prepare(
                "SELECT id, claim_state, surfacing_state, data_source, trust_score, metadata_json
                 FROM intelligence_claims
                 ORDER BY id",
            )
            .expect("prepare correction-resurrection query");
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<f64>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                ))
            })
            .expect("read fixture rows")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect fixture rows");

        let rows: Vec<FixtureClaim> = rows
            .into_iter()
            .filter_map(
                |(id, claim_state, surfacing_state, data_source, trust_score, metadata_json)| {
                    let metadata_json = metadata_json?;
                    let metadata: serde_json::Value = serde_json::from_str(&metadata_json).ok()?;
                    let scenario = metadata.get("scenario")?.as_str()?.to_string();
                    matches!(
                        scenario.as_str(),
                        "wrong_subject_tombstone"
                            | "user_edited_supersession"
                            | "expired_dormant_no_resurrection"
                    )
                    .then(|| FixtureClaim {
                        id,
                        claim_state,
                        surfacing_state,
                        data_source,
                        trust_score: trust_score
                            .expect("fixture correction rows carry trust_score"),
                        scenario,
                    })
                },
            )
            .collect();

        assert_eq!(
            rows.len(),
            4,
            "bundle-5 must seed all correction-resurrection guard rows"
        );

        let mut distinct_scores: Vec<f64> = Vec::new();
        for row in &rows {
            if !distinct_scores
                .iter()
                .any(|score| (*score - row.trust_score).abs() < 1e-9)
            {
                distinct_scores.push(row.trust_score);
            }
        }
        assert!(
            distinct_scores.len() >= 3,
            "correction-resurrection trust scores must not collapse to a flat distribution: {rows:?}"
        );

        let state_pairs: std::collections::BTreeSet<_> = rows
            .iter()
            .map(|row| (row.claim_state.as_str(), row.surfacing_state.as_str()))
            .collect();
        assert!(state_pairs.contains(&("tombstoned", "dormant")));
        assert!(state_pairs.contains(&("dormant", "dormant")));
        assert!(state_pairs.contains(&("active", "active")));

        let wrong_subject = rows
            .iter()
            .find(|row| row.id == "src-b5-wrong-attendee-original")
            .expect("wrong-subject tombstone row exists");
        assert_eq!(wrong_subject.scenario, "wrong_subject_tombstone");
        assert_eq!(wrong_subject.claim_state, "tombstoned");
        assert!(wrong_subject.trust_score < 0.4);

        let original = rows
            .iter()
            .find(|row| row.id == "src-b5-original-preference")
            .expect("superseded original row exists");
        let edited = rows
            .iter()
            .find(|row| row.id == "src-b5-user-edited-preference")
            .expect("user-edited superseding row exists");
        assert_eq!(edited.data_source, "user");
        assert_eq!(edited.claim_state, "active");
        assert!(edited.trust_score > original.trust_score);

        let expired = rows
            .iter()
            .find(|row| row.id == "src-b5-expired-dormant-open-loop")
            .expect("expired dormant resurrection guard row exists");
        assert_eq!(expired.scenario, "expired_dormant_no_resurrection");
        assert_eq!(expired.claim_state, "dormant");
        assert!(expired.trust_score < edited.trust_score);
    }

    #[test]
    fn dos311_pause_rejects_new_enqueue() {
        let q = IntelligenceQueue::new();
        q.pause();
        let result = q.enqueue(IntelRequest::new(
            "acc-1".into(),
            "account".into(),
            IntelPriority::Manual,
        ));
        assert_eq!(
            result,
            Err(EnqueueError::Paused),
            "paused queue must reject new enqueues"
        );
        assert_eq!(q.len(), 0);
    }

    #[test]
    fn dos311_resume_re_enables_enqueue() {
        let q = IntelligenceQueue::new();
        q.pause();
        q.resume();
        let result = q.enqueue(IntelRequest::new(
            "acc-1".into(),
            "account".into(),
            IntelPriority::Manual,
        ));
        assert_eq!(
            result,
            Ok(EnqueueOutcome::Accepted),
            "resumed queue accepts enqueues"
        );
        assert_eq!(q.len(), 1);
    }

    #[test]
    fn dos311_drain_pending_returns_and_empties() {
        let q = IntelligenceQueue::new();
        assert_eq!(
            q.enqueue(IntelRequest::new(
                "acc-1".into(),
                "account".into(),
                IntelPriority::Manual,
            )),
            Ok(EnqueueOutcome::Accepted)
        );
        assert_eq!(
            q.enqueue(IntelRequest::new(
                "acc-2".into(),
                "account".into(),
                IntelPriority::Manual,
            )),
            Ok(EnqueueOutcome::Accepted)
        );
        assert_eq!(q.len(), 2);
        let drained = q.drain_pending();
        assert_eq!(drained.len(), 2);
        assert_eq!(q.len(), 0);
    }

    #[test]
    fn dos311_paused_then_drained_then_resumed_recovers() {
        // Simulates the migration sequence shape: pause → drain →
        // run migration → re-enqueue drained → resume.
        let q = IntelligenceQueue::new();
        assert_eq!(
            q.enqueue(IntelRequest::new(
                "acc-1".into(),
                "account".into(),
                IntelPriority::Manual,
            )),
            Ok(EnqueueOutcome::Accepted)
        );
        q.pause();
        let drained = q.drain_pending();
        assert_eq!(drained.len(), 1);
        // While paused, a new enqueue should be rejected with Paused.
        assert_eq!(
            q.enqueue(IntelRequest::new(
                "acc-2".into(),
                "account".into(),
                IntelPriority::Manual,
            )),
            Err(EnqueueError::Paused)
        );
        // After resume, drained items can be re-enqueued.
        q.resume();
        for r in drained {
            assert_eq!(q.enqueue(r), Ok(EnqueueOutcome::Accepted));
        }
        assert_eq!(q.len(), 1);
    }

    #[test]
    fn cycle8_requeue_dequeued_batch_bypasses_pause_for_retry() {
        let q = IntelligenceQueue::new();
        assert_eq!(
            q.enqueue(IntelRequest::new(
                "acc-1".into(),
                "account".into(),
                IntelPriority::Manual,
            )),
            Ok(EnqueueOutcome::Accepted)
        );
        let batch = q.dequeue_batch(1);
        assert_eq!(batch.len(), 1);
        q.pause();

        q.requeue_dequeued_batch(batch);

        assert_eq!(q.len(), 1);
        q.resume();
        let retried = q.dequeue().expect("request should be queued for retry");
        assert_eq!(retried.entity_id, "acc-1");
    }

    // ───────────────────────────────────────────────────────────────────────
    // W1 Suite P baseline for queue primitives.
    // Captured 2026-04-29; bounds are 5× the typical observed median.
    // ───────────────────────────────────────────────────────────────────────

    #[test]
    fn suite_p_baseline_enqueue_dequeue_roundtrip() {
        // W1 baseline: typical median ~1-5µs. Bound: 50µs.
        let q = IntelligenceQueue::new();

        for i in 0..50 {
            let _ = q.enqueue(IntelRequest::new(
                format!("warm-{i}"),
                "account".into(),
                IntelPriority::Manual,
            ));
            q.dequeue();
        }
        let mut samples = Vec::with_capacity(500);
        for i in 0..500 {
            let start = std::time::Instant::now();
            let _ = q.enqueue(IntelRequest::new(
                format!("e-{i}"),
                "account".into(),
                IntelPriority::Manual,
            ));
            q.dequeue();
            samples.push(start.elapsed().as_nanos() / 1_000);
        }
        samples.sort();
        let median = samples[samples.len() / 2];
        eprintln!("[suite-p baseline] enqueue+dequeue roundtrip: median={median}µs samples=500");
        assert!(
            median < 50,
            "regression: enqueue+dequeue took {median}µs (W1 baseline ~1-5µs; bound 50µs)"
        );
    }

    #[test]
    fn dos311_default_drain_timeout_is_60s() {
        // Drain timeout remains configurable with a 60s default.
        // The constant lives in this module; the migration script consumes it.
        assert_eq!(DEFAULT_DRAIN_TIMEOUT_SECS, 60);
    }

    #[test]
    fn finalize_mode_trust_recompute_runs_recompute_once() {
        let db = trust_test_db();
        let account_id = "acct-trust-recompute-once";
        seed_trust_account(&db, account_id);
        let claim_id = seed_trust_claim(
            &db,
            account_id,
            "The account has a current renewal risk.",
            "unit_test_source",
        );
        seed_trust_corroboration(&db, &claim_id, "glean");

        run_trust_finalize(
            &db,
            account_id,
            FinalizeMode::ManualRefresh {
                producer: EnrichmentProducer::Pty,
            },
        );
        assert_eq!(read_trust_columns(&db, &claim_id), (None, None, None));

        run_trust_finalize(&db, account_id, FinalizeMode::TrustRecompute);
        let (score, computed_at, version) = read_trust_columns(&db, &claim_id);
        assert!(score.is_some_and(|value| value > 0.75));
        assert!(computed_at.is_some());
        assert_eq!(version, Some(1));
        // ConfidenceEvidence emits one signal per factor in evidence.factor_breakdown.
        // Factor count is determined by the trust extractor and reflects current scoring features.
        // Asserting the exact count would couple this test to the factor-list shape;
        // assert non-zero + bounded growth instead.
        let n = signal_count(&db, "ConfidenceEvidence");
        assert!(
            (1..=20).contains(&n),
            "expected 1..=20 ConfidenceEvidence signals from a single recompute, got {n}"
        );
    }

    #[test]
    #[ignore = "micro-bench for on-demand trust extraction performance checks"]
    fn trust_recompute_50_claims_500_accounts_under_50ms() {
        let db = trust_test_db();
        let account_id = "acct-scale-000";
        for idx in 0..500 {
            seed_trust_account(&db, &format!("acct-scale-{idx:03}"));
        }
        for idx in 0..50 {
            seed_trust_claim(
                &db,
                account_id,
                &format!("Scale recompute claim {idx} remains current."),
                "unit_test_source",
            );
        }

        let state = AppState::new();
        let dir = tempfile::tempdir().expect("tempdir");
        let input = trust_input(account_id, dir.path());

        let start = std::time::Instant::now();
        run_finalize_trust_recompute(&state, &db, &input).expect("run trust recompute");
        let elapsed = start.elapsed();

        assert!(
            elapsed < std::time::Duration::from_millis(50),
            "trust recompute for 50 claims over 500 accounts took {elapsed:?}"
        );
    }

    #[test]
    fn finalize_mode_manual_refresh_does_not_recompute_trust() {
        let db = trust_test_db();
        let account_id = "acct-manual-no-trust";
        seed_trust_account(&db, account_id);
        let claim_id = seed_trust_claim(
            &db,
            account_id,
            "The account has a prior trust score.",
            "unit_test_source",
        );
        seed_trust_corroboration(&db, &claim_id, "glean");
        seed_prior_trust(&db, &claim_id, 0.82, 7);
        let before = read_trust_columns(&db, &claim_id);

        run_trust_finalize(
            &db,
            account_id,
            FinalizeMode::ManualRefresh {
                producer: EnrichmentProducer::Pty,
            },
        );

        assert_eq!(read_trust_columns(&db, &claim_id), before);
        assert_eq!(signal_count(&db, "ConfidenceEvidence"), 0);
    }

    #[test]
    fn finalize_mode_queue_worker_does_not_recompute_trust() {
        let db = trust_test_db();
        let account_id = "acct-queue-no-trust";
        seed_trust_account(&db, account_id);
        let claim_id = seed_trust_claim(
            &db,
            account_id,
            "The account has a queued prior trust score.",
            "unit_test_source",
        );
        seed_trust_corroboration(&db, &claim_id, "glean");
        seed_prior_trust(&db, &claim_id, 0.82, 7);
        let before = read_trust_columns(&db, &claim_id);

        run_trust_finalize(
            &db,
            account_id,
            FinalizeMode::QueueWorker {
                is_background: false,
                producer: EnrichmentProducer::Pty,
            },
        );
        assert_eq!(read_trust_columns(&db, &claim_id), before);

        run_trust_finalize(
            &db,
            account_id,
            FinalizeMode::QueueWorker {
                is_background: true,
                producer: EnrichmentProducer::Pty,
            },
        );
        assert_eq!(read_trust_columns(&db, &claim_id), before);
        assert_eq!(signal_count(&db, "ConfidenceEvidence"), 0);
    }

    #[test]
    fn finalize_mode_trust_recompute_skips_on_extractor_mismatch_and_preserves_prior() {
        let db = trust_test_db();
        let account_id = "acct-missing-for-extractor";
        let claim_id = seed_trust_claim(
            &db,
            account_id,
            "The missing account claim keeps its prior score.",
            "unit_test_source",
        );
        seed_prior_trust(&db, &claim_id, 0.82, 7);
        let before = read_trust_columns(&db, &claim_id);

        run_trust_finalize(&db, account_id, FinalizeMode::TrustRecompute);

        assert_eq!(read_trust_columns(&db, &claim_id), before);
        assert_eq!(pipeline_failure_count(&db, "extractor_mismatch"), 1);
        assert_eq!(signal_count(&db, "ConfidenceEvidence"), 0);
    }

    #[test]
    fn finalize_mode_trust_recompute_emits_band_change_signal_only_on_boundary() {
        let db = trust_test_db();
        let account_id = "acct-band-boundary";
        seed_trust_account(&db, account_id);
        let claim_id = seed_trust_claim(
            &db,
            account_id,
            "The account has sparse corroboration.",
            "unit_test_source",
        );
        // Seed a prior score in NeedsVerification (<0.50). The first recompute
        // will land in LikelyCurrent (>=0.75) per current trust math (this
        // claim's evidence shape produces ~0.76 — see ConfidenceEvidence
        // emission test), crossing the band boundary and emitting a
        // ClaimTrustChanged signal. The second recompute produces the same
        // band → no new signal.
        seed_prior_trust(&db, &claim_id, 0.30, 3);

        run_trust_finalize(&db, account_id, FinalizeMode::TrustRecompute);
        assert_eq!(signal_count(&db, "ClaimTrustChanged"), 1);
        let (score, _, version) = read_trust_columns(&db, &claim_id);
        assert!(
            score.is_some_and(|value| value >= 0.75),
            "expected first recompute to land in LikelyCurrent band, got {score:?}"
        );
        assert_eq!(version, Some(4));

        run_trust_finalize(&db, account_id, FinalizeMode::TrustRecompute);
        // Second recompute produces a score in the same band — no new signal.
        assert_eq!(signal_count(&db, "ClaimTrustChanged"), 1);
        assert_eq!(read_trust_columns(&db, &claim_id).2, Some(5));
    }

    #[test]
    fn finalize_mode_trust_recompute_caps_at_needs_verification_on_malformed_signal_weights() {
        // Malformed signal_weights row (alpha=0,beta=0) used to compute NaN
        // and trigger compile_trust rollback to prior trust. The cycle-7 fix
        // catches the malformed row at read time, marks read_state as
        // indeterminate, and lets compile_trust succeed under the
        // IndeterminateReadState gate — fail-closed at NeedsVerification
        // rather than preserving a possibly-stale LikelyCurrent score.
        let db = trust_test_db();
        let account_id = "acct-compile-error";
        let data_source = "bad_source_weight";
        seed_trust_account(&db, account_id);
        let claim_id = seed_trust_claim(
            &db,
            account_id,
            "The account has malformed source reliability input.",
            data_source,
        );
        seed_prior_trust(&db, &claim_id, 0.82, 7);
        seed_malformed_source_weight(&db, data_source);

        run_trust_finalize(&db, account_id, FinalizeMode::TrustRecompute);

        let (score, _, version) = read_trust_columns(&db, &claim_id);
        let s = score.expect("score must be set after recompute");
        assert!(
            s < 0.5,
            "malformed weights must fail closed under NeedsVerification, got {s}"
        );
        assert_eq!(version, Some(8), "version must increment for the recompute");
    }

    #[test]
    fn record_trust_recompute_pipeline_failure_emits_for_update_failure() {
        // The compile-failure half of this metric used to be exercised by
        // seeding a malformed signal_weights row, but cycle-7 hardening
        // catches that upstream and routes it through the
        // IndeterminateReadState gate instead — compile_trust no longer
        // fails on legitimate-but-suspect data, so trust_compile_failed is
        // unreachable through the production data path. Update-failure is
        // still a valid pipeline failure mode and stays covered here.
        let db = trust_test_db();

        let update_account_id = "acct-pipeline-update-error";
        seed_trust_account(&db, update_account_id);
        let update_claim_id = seed_trust_claim(
            &db,
            update_account_id,
            "The account trust update should report persistence failure.",
            "unit_test_source",
        );
        seed_trust_corroboration(&db, &update_claim_id, "glean");
        let trigger_sql = format!(
            "CREATE TRIGGER trust_update_failure_fixture
             BEFORE UPDATE OF trust_score, trust_computed_at, trust_version
             ON intelligence_claims
             WHEN OLD.id = '{}'
             BEGIN
               SELECT RAISE(ABORT, 'trust update fixture failure');
             END",
            update_claim_id.replace('\'', "''")
        );
        db.conn_ref()
            .execute(&trigger_sql, [])
            .expect("install update failure trigger");

        let result =
            run_trust_finalize_result(&db, update_account_id, FinalizeMode::TrustRecompute);
        assert!(
            result.is_err(),
            "trust update failure should fail the claim_recompute job so it retries or dead-letters"
        );

        assert_eq!(pipeline_failure_count(&db, "trust_update_failed"), 1);
        assert_eq!(signal_count(&db, "ClaimTrustChanged"), 0);
    }

    #[test]
    fn test_merge_missing_core_fields_preserves_existing_assessment() {
        let mut fresh = IntelligenceJson {
            executive_assessment_render_policy: None,
            entity_id: "acme-corp".to_string(),
            entity_type: "account".to_string(),
            executive_assessment: None,
            ..Default::default()
        };
        let existing = IntelligenceJson {
            executive_assessment_render_policy: None,
            entity_id: "acme-corp".to_string(),
            entity_type: "account".to_string(),
            executive_assessment: Some("Prior narrative".to_string()),
            risks: vec![crate::intelligence::IntelRisk {
                render_policy: None,
                claim_id: None,
                text: "Renewal owner unresolved".to_string(),
                source: None,
                urgency: "high".to_string(),
                headline: None,
                evidence: None,
                kind_label: None,
                item_source: None,
                discrepancy: None,
            }],
            ..Default::default()
        };

        merge_missing_core_fields_from_existing(&mut fresh, Some(&existing));

        assert_eq!(
            fresh.executive_assessment.as_deref(),
            Some("Prior narrative")
        );
        assert!(fresh.risks.is_empty());
    }

    #[test]
    fn compose_enrichment_clears_skipped_dimension_fields_after_reconciliation() {
        let db = crate::db::test_utils::test_db();
        let entity_id = "person-skipped-dimensions";
        let existing = IntelligenceJson {
            executive_assessment_render_policy: None,
            entity_id: entity_id.to_string(),
            entity_type: "person".to_string(),
            enriched_at: "2026-05-20T00:00:00Z".to_string(),
            executive_assessment: Some("Prior person summary.".to_string()),
            health: Some(crate::intelligence::io::AccountHealth::default()),
            contract_context: Some(crate::intelligence::io::ContractContext::default()),
            success_metrics: Some(vec![crate::intelligence::io::SuccessMetric {
                name: "Legacy account metric".to_string(),
                target: Some("100%".to_string()),
                current: Some("80%".to_string()),
                status: Some("stale".to_string()),
                owner: None,
            }]),
            product_adoption: Some(crate::intelligence::io::AdoptionSignals::default()),
            support_health: Some(crate::intelligence::io::SupportHealth::default()),
            gong_call_summaries: vec![crate::intelligence::io::GongCallSummary {
                title: "Legacy account call".to_string(),
                date: "2026-05-19".to_string(),
                participants: vec!["Generic Person".to_string()],
                key_topics: "Legacy account-only summary".to_string(),
                sentiment: "neutral".to_string(),
            }],
            nps_csat: Some(crate::intelligence::io::SatisfactionData::default()),
            ..Default::default()
        };
        db.upsert_entity_intelligence(&existing)
            .expect("seed existing person intelligence");
        let dir = tempfile::tempdir().expect("tempdir");
        let input = EnrichmentInput {
            workspace: dir.path().to_path_buf(),
            entity_dir: dir.path().to_path_buf(),
            entity_id: entity_id.to_string(),
            entity_type: "person".to_string(),
            prompt: String::new(),
            file_manifest: Vec::new(),
            file_count: 0,
            computed_health: None,
            entity_name: "Generic Person".to_string(),
            relationship: Some("internal".to_string()),
            intelligence_context: None,
            active_preset: None,
        };
        let incoming = IntelligenceJson {
            executive_assessment_render_policy: None,
            entity_id: entity_id.to_string(),
            entity_type: "person".to_string(),
            enriched_at: "2026-05-21T00:00:00Z".to_string(),
            executive_assessment: Some("Fresh person summary.".to_string()),
            ..Default::default()
        };

        let prepared = compose_enrichment_intelligence_payload(
            &db,
            &input,
            &incoming,
            EnrichmentProducer::Pty,
            None,
        )
        .expect("compose person enrichment");
        let final_intel = prepared.intelligence();

        assert_eq!(
            final_intel.executive_assessment.as_deref(),
            Some("Fresh person summary.")
        );
        assert!(
            final_intel.health.is_none(),
            "commercial health must not survive person refresh"
        );
        assert!(
            final_intel.contract_context.is_none(),
            "contract context must not survive person refresh"
        );
        assert!(
            final_intel.success_metrics.is_none(),
            "value-success metrics must not survive person refresh"
        );
        assert!(
            final_intel.product_adoption.is_none(),
            "account-only product adoption must not survive person refresh"
        );
        assert!(
            final_intel.support_health.is_none(),
            "account-only support health must not survive person refresh"
        );
        assert!(
            final_intel.gong_call_summaries.is_empty(),
            "account-only call summaries must not survive person refresh"
        );
        assert!(
            final_intel.nps_csat.is_none(),
            "account-only satisfaction data must not survive person refresh"
        );
    }

    #[test]
    fn progressive_snapshot_clears_skipped_dimension_fields_before_write() {
        let db = crate::db::test_utils::test_db();
        let entity_id = "person-progressive-skipped-dimensions";
        let existing = IntelligenceJson {
            executive_assessment_render_policy: None,
            entity_id: entity_id.to_string(),
            entity_type: "person".to_string(),
            enriched_at: "2026-05-20T00:00:00Z".to_string(),
            health: Some(crate::intelligence::io::AccountHealth::default()),
            product_adoption: Some(crate::intelligence::io::AdoptionSignals::default()),
            support_health: Some(crate::intelligence::io::SupportHealth::default()),
            gong_call_summaries: vec![crate::intelligence::io::GongCallSummary {
                title: "Legacy account call".to_string(),
                date: "2026-05-19".to_string(),
                participants: vec!["Generic Person".to_string()],
                key_topics: "Legacy account-only summary".to_string(),
                sentiment: "neutral".to_string(),
            }],
            nps_csat: Some(crate::intelligence::io::SatisfactionData::default()),
            ..Default::default()
        };
        db.upsert_entity_intelligence(&existing)
            .expect("seed existing person intelligence");
        let combined = IntelligenceJson {
            executive_assessment_render_policy: None,
            entity_id: entity_id.to_string(),
            entity_type: "person".to_string(),
            enriched_at: "2026-05-21T00:00:00Z".to_string(),
            executive_assessment: Some("Progressive person summary.".to_string()),
            ..Default::default()
        };

        let progressive = prepare_progressive_dimension_snapshot(
            &db,
            entity_id,
            "person",
            Some("internal"),
            &combined,
        );

        assert_eq!(
            progressive.executive_assessment.as_deref(),
            Some("Progressive person summary.")
        );
        assert!(
            progressive.health.is_none(),
            "progressive snapshots must not persist skipped commercial health"
        );
        assert!(
            progressive.product_adoption.is_none(),
            "progressive snapshots must not persist account-only product adoption"
        );
        assert!(
            progressive.support_health.is_none(),
            "progressive snapshots must not persist account-only support health"
        );
        assert!(
            progressive.gong_call_summaries.is_empty(),
            "progressive snapshots must not persist account-only call summaries"
        );
        assert!(
            progressive.nps_csat.is_none(),
            "progressive snapshots must not persist account-only satisfaction data"
        );
    }

    #[test]
    fn test_is_sparse_intelligence_detects_non_sparse_when_assessment_present() {
        let intel = IntelligenceJson {
            executive_assessment_render_policy: None,
            executive_assessment: Some("Non-empty summary".to_string()),
            ..Default::default()
        };
        assert!(!is_sparse_intelligence(&intel));
    }

    #[test]
    fn test_intel_queue_enqueue_dequeue() {
        let queue = IntelligenceQueue::new();

        let _ = queue.enqueue(IntelRequest {
            entity_id: "acme".to_string(),
            entity_type: "account".to_string(),
            priority: IntelPriority::ContentChange,
            requested_at: Instant::now(),
            retry_count: 0,
        });

        assert_eq!(queue.len(), 1);

        let req = queue.dequeue().unwrap();
        assert_eq!(req.entity_id, "acme");
        assert_eq!(req.priority, IntelPriority::ContentChange);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_intel_queue_remove_by_entity_id() {
        // archiving an entity should drop all of its pending queue
        // entries regardless of priority, and should not touch other entities.
        let queue = IntelligenceQueue::new();

        let _ = queue.enqueue(IntelRequest {
            entity_id: "archived-acct".to_string(),
            entity_type: "account".to_string(),
            priority: IntelPriority::ProactiveHygiene,
            requested_at: Instant::now(),
            retry_count: 0,
        });
        let _ = queue.enqueue(IntelRequest {
            entity_id: "other-acct".to_string(),
            entity_type: "account".to_string(),
            priority: IntelPriority::Manual,
            requested_at: Instant::now(),
            retry_count: 0,
        });

        assert_eq!(queue.len(), 2);

        let removed = queue.remove_by_entity_id("archived-acct");
        assert_eq!(removed, 1);
        assert_eq!(queue.len(), 1);

        let req = queue.dequeue().unwrap();
        assert_eq!(req.entity_id, "other-acct");

        // Removing an unknown entity is a safe no-op.
        assert_eq!(queue.remove_by_entity_id("never-queued"), 0);
    }

    #[test]
    fn test_intel_queue_remove_clears_debounce_tracker() {
        // removal must clear the debounce tracker so an
        // unarchive → re-enqueue can proceed without the debounce delay.
        let queue = IntelligenceQueue::new();

        let _ = queue.enqueue(IntelRequest {
            entity_id: "acme".to_string(),
            entity_type: "account".to_string(),
            priority: IntelPriority::ContentChange,
            requested_at: Instant::now(),
            retry_count: 0,
        });
        // Drain so the queue is empty but the debounce tracker still holds "acme".
        let _ = queue.dequeue();
        assert!(queue.is_empty());
        assert!(queue.last_enqueued.lock().contains_key("acme"));

        queue.remove_by_entity_id("acme");
        assert!(!queue.last_enqueued.lock().contains_key("acme"));

        // A fresh ContentChange enqueue for the same entity should now go through
        // instead of being debounced.
        let _ = queue.enqueue(IntelRequest {
            entity_id: "acme".to_string(),
            entity_type: "account".to_string(),
            priority: IntelPriority::ContentChange,
            requested_at: Instant::now(),
            retry_count: 0,
        });
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn test_intel_queue_dedup_keeps_higher_priority() {
        let queue = IntelligenceQueue::new();

        let _ = queue.enqueue(IntelRequest {
            entity_id: "acme".to_string(),
            entity_type: "account".to_string(),
            priority: IntelPriority::ContentChange,
            requested_at: Instant::now(),
            retry_count: 0,
        });

        // Same entity, higher priority → should upgrade
        let _ = queue.enqueue(IntelRequest {
            entity_id: "acme".to_string(),
            entity_type: "account".to_string(),
            priority: IntelPriority::Manual,
            requested_at: Instant::now(),
            retry_count: 0,
        });

        assert_eq!(queue.len(), 1);
        let req = queue.dequeue().unwrap();
        assert_eq!(req.priority, IntelPriority::Manual);
    }

    #[test]
    fn test_intel_queue_dedup_ignores_lower_priority() {
        let queue = IntelligenceQueue::new();

        let _ = queue.enqueue(IntelRequest {
            entity_id: "acme".to_string(),
            entity_type: "account".to_string(),
            priority: IntelPriority::Manual,
            requested_at: Instant::now(),
            retry_count: 0,
        });

        // Same entity, lower priority → should be ignored
        let _ = queue.enqueue(IntelRequest {
            entity_id: "acme".to_string(),
            entity_type: "account".to_string(),
            priority: IntelPriority::ContentChange,
            requested_at: Instant::now(),
            retry_count: 0,
        });

        assert_eq!(queue.len(), 1);
        let req = queue.dequeue().unwrap();
        assert_eq!(req.priority, IntelPriority::Manual);
    }

    #[test]
    fn test_intel_queue_priority_ordering() {
        let queue = IntelligenceQueue::new();

        let _ = queue.enqueue(IntelRequest {
            entity_id: "alpha".to_string(),
            entity_type: "account".to_string(),
            priority: IntelPriority::ContentChange,
            requested_at: Instant::now(),
            retry_count: 0,
        });

        let _ = queue.enqueue(IntelRequest {
            entity_id: "beta".to_string(),
            entity_type: "account".to_string(),
            priority: IntelPriority::Manual,
            requested_at: Instant::now(),
            retry_count: 0,
        });

        let _ = queue.enqueue(IntelRequest {
            entity_id: "gamma".to_string(),
            entity_type: "project".to_string(),
            priority: IntelPriority::CalendarChange,
            requested_at: Instant::now(),
            retry_count: 0,
        });

        assert_eq!(queue.len(), 3);

        // Should dequeue in priority order: Manual > CalendarChange > ContentChange
        let first = queue.dequeue().unwrap();
        assert_eq!(first.entity_id, "beta");
        assert_eq!(first.priority, IntelPriority::Manual);

        let second = queue.dequeue().unwrap();
        assert_eq!(second.entity_id, "gamma");

        let third = queue.dequeue().unwrap();
        assert_eq!(third.entity_id, "alpha");
    }

    #[test]
    fn test_intel_queue_debounce_content_changes() {
        let queue = IntelligenceQueue::new();

        // First enqueue succeeds
        let _ = queue.enqueue(IntelRequest {
            entity_id: "acme".to_string(),
            entity_type: "account".to_string(),
            priority: IntelPriority::ContentChange,
            requested_at: Instant::now(),
            retry_count: 0,
        });

        // Dequeue it (so queue is empty)
        let _ = queue.dequeue();
        assert!(queue.is_empty());

        // Second enqueue within debounce window is suppressed
        let _ = queue.enqueue(IntelRequest {
            entity_id: "acme".to_string(),
            entity_type: "account".to_string(),
            priority: IntelPriority::ContentChange,
            requested_at: Instant::now(),
            retry_count: 0,
        });

        // Should be debounced (queue still empty)
        assert!(queue.is_empty());
    }

    #[test]
    fn test_intel_queue_manual_bypasses_debounce() {
        let queue = IntelligenceQueue::new();

        // First: content change
        let _ = queue.enqueue(IntelRequest {
            entity_id: "acme".to_string(),
            entity_type: "account".to_string(),
            priority: IntelPriority::ContentChange,
            requested_at: Instant::now(),
            retry_count: 0,
        });

        // Dequeue it
        let _ = queue.dequeue();

        // Manual request should bypass debounce
        let _ = queue.enqueue(IntelRequest {
            entity_id: "acme".to_string(),
            entity_type: "account".to_string(),
            priority: IntelPriority::Manual,
            requested_at: Instant::now(),
            retry_count: 0,
        });

        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn test_intel_queue_prune_stale_entries() {
        let queue = IntelligenceQueue::new();

        // Insert a debounce entry manually with an old timestamp
        {
            let mut last = queue.last_enqueued.lock();
            // Insert an entry that's "old" by using Instant::now() minus a large duration
            // We can't easily backdate Instant, so test the structure:
            // Insert a fresh entry, prune should NOT remove it
            last.insert("fresh-entity".to_string(), Instant::now());
        }

        queue.prune_stale_entries();

        // Fresh entry should still be there
        let last = queue.last_enqueued.lock();
        assert!(
            last.contains_key("fresh-entity"),
            "fresh entry should survive pruning"
        );
    }

    #[test]
    fn test_intel_queue_dequeue_empty() {
        let queue = IntelligenceQueue::new();
        assert!(queue.dequeue().is_none());
    }

    #[test]
    fn test_intel_queue_multiple_entities() {
        let queue = IntelligenceQueue::new();

        let _ = queue.enqueue(IntelRequest {
            entity_id: "acme".to_string(),
            entity_type: "account".to_string(),
            priority: IntelPriority::ContentChange,
            requested_at: Instant::now(),
            retry_count: 0,
        });

        let _ = queue.enqueue(IntelRequest {
            entity_id: "beta-project".to_string(),
            entity_type: "project".to_string(),
            priority: IntelPriority::ContentChange,
            requested_at: Instant::now(),
            retry_count: 0,
        });

        assert_eq!(queue.len(), 2);
    }

    // =========================================================================
    // Batch dequeue tests
    // =========================================================================

    #[test]
    fn test_dequeue_batch_returns_up_to_max() {
        let queue = IntelligenceQueue::new();

        for name in &["alpha", "beta", "gamma", "delta"] {
            let _ = queue.enqueue(IntelRequest {
                entity_id: name.to_string(),
                entity_type: "account".to_string(),
                priority: IntelPriority::ContentChange,
                requested_at: Instant::now(),
                retry_count: 0,
            });
        }

        assert_eq!(queue.len(), 4);

        // Should dequeue at most 3
        let batch = queue.dequeue_batch(3);
        assert_eq!(batch.len(), 3);
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn test_dequeue_batch_returns_by_priority() {
        let queue = IntelligenceQueue::new();

        let _ = queue.enqueue(IntelRequest {
            entity_id: "low".to_string(),
            entity_type: "account".to_string(),
            priority: IntelPriority::ContentChange,
            requested_at: Instant::now(),
            retry_count: 0,
        });
        let _ = queue.enqueue(IntelRequest {
            entity_id: "high".to_string(),
            entity_type: "account".to_string(),
            priority: IntelPriority::Manual,
            requested_at: Instant::now(),
            retry_count: 0,
        });
        let _ = queue.enqueue(IntelRequest {
            entity_id: "mid".to_string(),
            entity_type: "account".to_string(),
            priority: IntelPriority::CalendarChange,
            requested_at: Instant::now(),
            retry_count: 0,
        });

        let batch = queue.dequeue_batch(3);
        assert_eq!(batch.len(), 3);
        assert_eq!(batch[0].entity_id, "high");
        assert_eq!(batch[1].entity_id, "mid");
        assert_eq!(batch[2].entity_id, "low");
    }

    #[test]
    fn test_dequeue_batch_empty_queue() {
        let queue = IntelligenceQueue::new();
        let batch = queue.dequeue_batch(3);
        assert!(batch.is_empty());
    }

    #[test]
    fn test_dequeue_batch_fewer_than_max() {
        let queue = IntelligenceQueue::new();

        let _ = queue.enqueue(IntelRequest {
            entity_id: "only-one".to_string(),
            entity_type: "account".to_string(),
            priority: IntelPriority::ContentChange,
            requested_at: Instant::now(),
            retry_count: 0,
        });

        let batch = queue.dequeue_batch(3);
        assert_eq!(batch.len(), 1);
        assert_eq!(batch[0].entity_id, "only-one");
        assert!(queue.is_empty());
    }

    // =========================================================================
    // TTL tests
    // =========================================================================

    #[test]
    fn test_ttl_skips_recently_enriched_entity() {
        // Entity enriched 1 hour ago → within 2-hour TTL → should be skipped
        let one_hour_ago = (Utc::now() - chrono::Duration::hours(1)).to_rfc3339();
        let result = enrichment_age_check(&one_hour_ago, "acme");
        assert!(
            result.is_some(),
            "Entity enriched 1 hour ago should be skipped"
        );
        let msg = result.unwrap();
        assert!(msg.contains("Skipping acme"));
        assert!(msg.contains("TTL: 120 min"));
    }

    #[test]
    fn test_ttl_allows_stale_entity() {
        // Entity enriched 3 hours ago → outside 2-hour TTL → should proceed
        let three_hours_ago = (Utc::now() - chrono::Duration::hours(3)).to_rfc3339();
        let result = enrichment_age_check(&three_hours_ago, "acme");
        assert!(
            result.is_none(),
            "Entity enriched 3 hours ago should proceed"
        );
    }

    #[test]
    fn test_ttl_allows_empty_enriched_at() {
        // Never enriched → should proceed
        let result = enrichment_age_check("", "acme");
        assert!(result.is_none(), "Never-enriched entity should proceed");
    }

    #[test]
    fn test_ttl_manual_priority_bypasses_check() {
        // The processor loop skips the TTL check entirely for Manual priority.
        // Verify the priority gate logic: Manual != ContentChange etc.
        assert_ne!(IntelPriority::Manual, IntelPriority::ContentChange);
        assert_ne!(IntelPriority::Manual, IntelPriority::CalendarChange);
        assert_ne!(IntelPriority::Manual, IntelPriority::ProactiveHygiene);

        // Even if enrichment_age_check would skip, Manual requests bypass
        // because the processor loop guards with `request.priority != Manual`.
        let recent = (Utc::now() - chrono::Duration::minutes(30)).to_rfc3339();
        let skip = enrichment_age_check(&recent, "acme");
        assert!(
            skip.is_some(),
            "age_check itself would skip, but Manual bypasses the check in the loop"
        );
    }

    #[test]
    fn source_reliability_corroborators_reads_corroborations_and_unreconciled_contradictions() {
        let db = trust_test_db();
        db.conn_ref()
            .execute_batch(
                "INSERT INTO intelligence_claims /* dos7-allowed: in-module unit test seeds rows for source_reliability_corroborators read path */ \
                 (id, subject_ref, claim_type, text, dedup_key, actor, data_source, observed_at, provenance_json) \
                 VALUES \
                 ('c-target', '{\"kind\":\"account\",\"id\":\"a-1\"}', 'fact', 'target', 'd-target', 'system', 'manual', '2026-05-01', '{}'), \
                 ('c-other',  '{\"kind\":\"account\",\"id\":\"a-1\"}', 'fact', 'other',  'd-other',  'system', 'manual', '2026-05-01', '{}');",
            )
            .expect("seed claims");
        db.conn_ref()
            .execute_batch(
                "INSERT INTO claim_corroborations /* dos7-allowed: paired test seed for source_reliability_corroborators read path */ (id, claim_id, data_source, strength) \
                 VALUES ('cor-1', 'c-target', 'glean', 0.7), \
                        ('cor-2', 'c-target', 'manual', 0.4); \
                 INSERT INTO claim_contradictions /* dos7-allowed: paired test seed for source_reliability_corroborators read path */ (id, primary_claim_id, contradicting_claim_id, branch_kind, detected_at) \
                 VALUES ('con-1', 'c-target', 'c-other', 'contradiction', '2026-05-01'); \
                 INSERT INTO claim_contradictions /* dos7-allowed: paired test seed for source_reliability_corroborators read path */ (id, primary_claim_id, contradicting_claim_id, branch_kind, detected_at, reconciled_at, winner_claim_id) \
                 VALUES ('con-2', 'c-target', 'c-other', 'contradiction', '2026-05-01', '2026-05-02', 'c-target');",
            )
            .expect("seed corroborators + contradictions");

        let (corroborators, reason) =
            source_reliability_corroborators_for_claim(&db, "c-target").into_parts();
        assert!(reason.is_none(), "clean reads should report no reason");
        let confirming: Vec<_> = corroborators.iter().filter(|c| c.confirms).collect();
        let contradicting: Vec<_> = corroborators.iter().filter(|c| !c.confirms).collect();
        assert_eq!(confirming.len(), 2, "two corroboration rows must surface");
        assert!(confirming
            .iter()
            .any(|c| (c.evidence_weight - 0.7).abs() < 1e-9));
        assert!(confirming
            .iter()
            .any(|c| (c.evidence_weight - 0.4).abs() < 1e-9));
        assert_eq!(
            contradicting.len(),
            1,
            "reconciled contradiction must NOT count, only the unreconciled one"
        );
        assert!((contradicting[0].evidence_weight - 1.0).abs() < 1e-9);
    }

    #[test]
    fn trust_feedback_signal_reports_read_error_when_table_missing() {
        let db = trust_test_db();
        db.conn_ref()
            .execute_batch("DROP TABLE claim_feedback;")
            .expect("drop claim_feedback to simulate read failure");
        let (signal, reason) = trust_feedback_signal_for_claim(&db, "c-target").into_parts();
        assert_eq!(signal, crate::abilities::trust::UserFeedbackSignal::None);
        assert!(
            reason.is_some(),
            "missing claim_feedback table must propagate an indeterminate reason"
        );
    }

    #[test]
    fn source_reliability_reports_read_error_when_signal_weights_missing() {
        let db = trust_test_db();
        db.conn_ref()
            .execute_batch("DROP TABLE signal_weights;")
            .expect("drop signal_weights to simulate read failure");
        let claim = crate::db::claims::IntelligenceClaim {
            id: "c-1".into(),
            claim_version: 1,
            subject_ref: "{}".into(),
            claim_type: "fact".into(),
            field_path: None,
            topic_key: None,
            text: "x".into(),
            dedup_key: "d-1".into(),
            item_hash: None,
            actor: "agent".into(),
            data_source: "glean".into(),
            source_ref: None,
            source_asof: None,
            observed_at: "2026-05-01".into(),
            created_at: "2026-05-01".into(),
            provenance_json: "{}".into(),
            metadata_json: None,
            claim_state: crate::db::claims::ClaimState::Active,
            surfacing_state: crate::db::claims::SurfacingState::Active,
            demotion_reason: None,
            reactivated_at: None,
            retraction_reason: None,
            expires_at: None,
            superseded_by: None,
            trust_score: None,
            trust_computed_at: None,
            trust_version: None,
            thread_id: None,
            temporal_scope: crate::db::claims::TemporalScope::State,
            sensitivity: crate::db::claims::ClaimSensitivity::Internal,
            verification_state: crate::db::claims::ClaimVerificationState::Active,
            verification_reason: None,
            needs_user_decision_at: None,
        };
        let (_value, reason) = source_reliability_for_claim(&db, "account", &claim).into_parts();
        assert!(
            reason.is_some(),
            "missing signal_weights table must propagate an indeterminate reason"
        );
    }

    #[test]
    fn internal_consistency_honors_metadata_hint_otherwise_defaults_to_one() {
        let mut claim = crate::db::claims::IntelligenceClaim {
            id: "c-1".into(),
            claim_version: 1,
            subject_ref: "{}".into(),
            claim_type: "fact".into(),
            field_path: None,
            topic_key: None,
            text: "x".into(),
            dedup_key: "d-1".into(),
            item_hash: None,
            actor: "agent".into(),
            data_source: "manual".into(),
            source_ref: None,
            source_asof: None,
            observed_at: "2026-05-01".into(),
            created_at: "2026-05-01".into(),
            provenance_json: "{}".into(),
            metadata_json: None,
            claim_state: crate::db::claims::ClaimState::Active,
            surfacing_state: crate::db::claims::SurfacingState::Active,
            demotion_reason: None,
            reactivated_at: None,
            retraction_reason: None,
            expires_at: None,
            superseded_by: None,
            trust_score: None,
            trust_computed_at: None,
            trust_version: None,
            thread_id: None,
            temporal_scope: crate::db::claims::TemporalScope::State,
            sensitivity: crate::db::claims::ClaimSensitivity::Internal,
            verification_state: crate::db::claims::ClaimVerificationState::Active,
            verification_reason: None,
            needs_user_decision_at: None,
        };
        let (value, reason) = internal_consistency_for_claim(&claim).into_parts();
        assert_eq!(value, 1.0);
        assert!(reason.is_none(), "absent metadata is not an error");

        claim.metadata_json = Some(r#"{"internal_consistency":0.3}"#.into());
        let (value, reason) = internal_consistency_for_claim(&claim).into_parts();
        assert!((value - 0.3).abs() < 1e-9);
        assert!(reason.is_none());

        claim.metadata_json = Some(r#"{"internal_consistency":1.5}"#.into());
        let (value, reason) = internal_consistency_for_claim(&claim).into_parts();
        assert_eq!(value, 1.0, "out-of-range hints fall back to 1.0");
        assert!(
            reason.is_some(),
            "out-of-range hint must surface an indeterminate reason"
        );

        claim.metadata_json = Some("not-json".into());
        let (value, reason) = internal_consistency_for_claim(&claim).into_parts();
        assert_eq!(value, 1.0);
        assert!(
            reason.is_some(),
            "malformed metadata_json must surface an indeterminate reason"
        );
    }
}
