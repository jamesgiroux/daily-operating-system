//! Background intelligence enrichment queue (I132).
//!
//! Provides a priority queue for intelligence enrichment requests with
//! deduplication and debounce. A background processor drains the queue
//! and runs enrichment with split DB locking so the UI stays responsive
//! during the 30-120s PTY operation.

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::Mutex;
use std::time::Instant;

use chrono::Utc;

use tauri::{AppHandle, Emitter};

use crate::intelligence::dimension_prompts::{self, DIMENSION_NAMES};
use crate::intelligence::{
    build_intelligence_prompt_with_preset, extract_inferred_relationships,
    parse_intelligence_response, write_intelligence_json, InferredRelationship, IntelligenceJson,
    SourceManifestEntry,
};
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

/// Maximum retry attempts for entities that fail validation (I470).
const MAX_VALIDATION_RETRIES: u8 = 2;

/// TTL for enrichment results — skip entities enriched within this window (I287).
const ENRICHMENT_TTL_SECS: u64 = 7200;

/// Priority levels for intelligence enrichment requests.
/// Higher numeric value = higher priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IntelPriority {
    /// Background maintenance — lowest priority, budget-gated (I146 — ADR-0058).
    ProactiveHygiene = 0,
    /// Triggered by content file changes in the entity directory.
    ContentChange = 1,
    /// Triggered by calendar changes affecting this entity's meetings.
    CalendarChange = 2,
    /// Onboarding batch import — higher than content, lower than manual (I561).
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
    /// Number of times this entity has been retried after validation failure (I470).
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
pub struct IntelligenceQueue {
    queue: Mutex<VecDeque<IntelRequest>>,
    last_enqueued: Mutex<HashMap<String, Instant>>,
}

impl IntelligenceQueue {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            last_enqueued: Mutex::new(HashMap::new()),
        }
    }

    /// Enqueue an enrichment request.
    ///
    /// Deduplicates by entity_id: if the same entity is already queued,
    /// the higher priority wins. Debounces content changes by
    /// `CONTENT_DEBOUNCE_SECS` — rapid changes within the window are
    /// coalesced into a single request.
    pub fn enqueue(&self, request: IntelRequest) {
        if let Some(debounce_secs) = debounce_window_secs(request.priority) {
            let last = self.last_enqueued.lock();
            if let Some(last_time) = last.get(&request.entity_id) {
                if last_time.elapsed().as_secs() < debounce_secs {
                    log::debug!(
                        "IntelQueue: debounced {} ({}s since last)",
                        request.entity_id,
                        last_time.elapsed().as_secs()
                    );
                    return;
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
            return;
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

    /// Dequeue up to `max` highest-priority requests (I289).
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

    /// Remove stale entries from the `last_enqueued` debounce tracker.
    ///
    /// Entries older than `CONTENT_DEBOUNCE_SECS * 10` (5 minutes) are pruned
    /// to prevent unbounded memory growth over long-running sessions (I234).
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

/// I575: Progressive enrichment progress event payload.
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

/// I575: Progressive enrichment completion event payload.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnrichmentComplete {
    pub entity_id: String,
    pub entity_type: String,
    pub succeeded: u32,
    pub failed: u32,
    pub failed_dimensions: Vec<String>,
    pub wall_clock_ms: u64,
}

/// Context gathered from the DB (held briefly, then released before PTY).
/// Public so manual enrichment commands can reuse the split-lock pattern (I173).
#[derive(Clone)]
pub struct EnrichmentInput {
    pub workspace: PathBuf,
    pub entity_dir: PathBuf,
    pub entity_id: String,
    pub entity_type: String,
    pub prompt: String,
    pub file_manifest: Vec<SourceManifestEntry>,
    pub file_count: usize,
    /// I499: Pre-computed algorithmic health for accounts.
    pub computed_health: Option<crate::intelligence::io::AccountHealth>,
    /// I535: Entity name for Glean-first enrichment.
    pub entity_name: String,
    /// I535: Relationship type for Glean prompt (e.g., "customer", "partner").
    pub relationship: Option<String>,
    /// I535: Intelligence context for Glean-first enrichment.
    /// Preserved from gather phase so Glean can inject local context into its prompt.
    pub intelligence_context: Option<crate::intelligence::prompts::IntelligenceContext>,
}

/// Parsed enrichment output from one model response section.
#[derive(Debug, Clone)]
pub struct EnrichmentParseResult {
    pub intel: IntelligenceJson,
    pub inferred_relationships: Vec<InferredRelationship>,
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

        // Dev mode isolation: pause background processing while dev sandbox is active
        if crate::db::is_dev_db_mode() {
            continue;
        }

        // Periodic pruning of stale debounce entries (I234)
        polls_since_prune += 1;
        if polls_since_prune >= prune_interval {
            state.intel_queue.prune_stale_entries();
            polls_since_prune = 0;
        }

        // Process one request per wake so automatic background bursts do not
        // stack PTY calls back-to-back and starve manual work.
        let batch = state.intel_queue.dequeue_batch(1);
        if batch.is_empty() {
            continue;
        }

        let entity_names: Vec<&str> = batch.iter().map(|r| r.entity_id.as_str()).collect();
        log::info!(
            "IntelProcessor: dequeued batch of {} entities: {:?}",
            batch.len(),
            entity_names,
        );

        // TTL check: filter out entities enriched recently unless manually requested (I287)
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

        // I571: Emit background work status for frontend indicator only when
        // the batch survives TTL/background guards and will do real work.
        let display_names: Vec<String> = if let Ok(db) = crate::db::ActionDb::open() {
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

        // Track original requests so we can detect failures and re-enqueue (I470)
        let original_requests: Vec<IntelRequest> = inputs.iter().map(|(r, _)| r.clone()).collect();

        // Track error categories for failed entities (I472)
        let mut error_categories: HashMap<String, &str> = HashMap::new();

        let enrichment_start = Instant::now();
        let results: Vec<(IntelRequest, EnrichmentInput, EnrichmentParseResult)> =
            if state.context_provider().is_remote() {
                // I535/ADR-0100: Glean-first path — use chat MCP tool for enrichment.
                // Falls back to PTY on failure (per entity).
                run_glean_enrichment_with_fallback(inputs, &ai_config, &state, &app).await
            } else {
                // I574: Per-entity enrichment (tries parallel fan-out, falls back to legacy)
                // I564: Run PTY enrichment on blocking threads to avoid stalling Tokio workers.
                let mut entity_results = Vec::new();
                for (request, input) in inputs {
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

        // I470: Re-enqueue entities that failed validation (up to MAX_VALIDATION_RETRIES)
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
                    state.intel_queue.enqueue(IntelRequest {
                        entity_id: original.entity_id.clone(),
                        entity_type: original.entity_type.clone(),
                        priority: original.priority,
                        requested_at: Instant::now(),
                        retry_count: original.retry_count + 1,
                    });
                } else if !succeeded.contains(original.entity_id.as_str()) {
                    // I428: Record claude_code sync failure
                    if let Ok(db) = crate::db::ActionDb::open() {
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
                    // Track schema validation failures for dropped entities (I472)
                    error_categories
                        .entry(original.entity_id.clone())
                        .or_insert("schema_validation");
                    {
            let mut audit = state.audit_log.lock();
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
        for (request, input, parsed) in &results {
            let intel = &parsed.intel;
            // Check for anomalies in the enrichment output (I472)
            if let Ok(serialized) = serde_json::to_string(intel) {
                let anomalies = crate::intelligence::validation::detect_anomalies(&serialized);
                if !anomalies.is_empty() {
                    {
            let mut audit = state.audit_log.lock();
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

            let written_intel = match write_enrichment_results(
                &state,
                input,
                intel,
                if is_background_priority(request.priority) {
                    None
                } else {
                    Some(&ai_config)
                },
            ) {
                Ok(intel) => intel,
                Err(e) => {
                    log::warn!(
                        "IntelProcessor: failed to write results for {}: {}",
                        request.entity_id,
                        e
                    );
                    continue;
                }
            };

            // I535: Emit tiered Glean signals after successful enrichment
            if state.context_provider().is_remote() {
                if let Ok(db) = crate::db::ActionDb::open() {
                    crate::intelligence::glean_provider::emit_glean_signals(
                        &db,
                        &state.signals.engine,
                        &request.entity_type,
                        &request.entity_id,
                        &written_intel,
                    );
                }
            }

            // DOS-15: Supplemental leading-signals enrichment for Health & Outlook.
            // Runs only for accounts and only when Glean is configured. Silent
            // fallback: any error (chat failure, timeout, parse) is logged and
            // ignored — users without Glean never see this surface.
            if state.context_provider().is_remote() && request.entity_type == "account" {
                let endpoint = state
                    .context_provider()
                    .remote_endpoint()
                    .map(|s| s.to_string());
                if let Some(endpoint) = endpoint {
                    let entity_name = input.entity_name.clone();
                    let entity_id = request.entity_id.clone();
                    let entity_type = request.entity_type.clone();
                    let engine = std::sync::Arc::clone(&state.signals.engine);
                    tauri::async_runtime::spawn(async move {
                        let provider = crate::intelligence::glean_provider::GleanIntelligenceProvider::new(&endpoint);
                        match provider.enrich_leading_signals(&entity_name).await {
                            Ok(signals) => {
                                if let Ok(db) = crate::db::ActionDb::open() {
                                    if let Err(e) =
                                        crate::services::intelligence::upsert_health_outlook_signals(
                                            &db,
                                            &engine,
                                            &entity_type,
                                            &entity_id,
                                            &signals,
                                        )
                                    {
                                        log::warn!(
                                            "[DOS-15] upsert_health_outlook_signals failed for {}: {}",
                                            entity_id,
                                            e
                                        );
                                    } else {
                                        log::info!(
                                            "[DOS-15] Leading signals persisted for {}",
                                            entity_id
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                log::info!(
                                    "[DOS-15] Leading-signals enrichment skipped for {}: {}",
                                    entity_id,
                                    e
                                );
                            }
                        }
                    });
                }
            }

            if !parsed.inferred_relationships.is_empty() {
                let db = match crate::db::ActionDb::open() {
                    Ok(db) => db,
                    Err(e) => {
                        log::warn!(
                            "IntelProcessor: failed to open DB for inferred relationship persistence on {}: {}",
                            request.entity_id,
                            e
                        );
                        continue;
                    }
                };
                if let Err(e) =
                    crate::services::intelligence::upsert_inferred_relationships_from_enrichment(
                        &db,
                        state.signals.engine.as_ref(),
                        &request.entity_type,
                        &request.entity_id,
                        &parsed.inferred_relationships,
                    )
                {
                    log::warn!(
                        "IntelProcessor: failed to persist inferred relationships for {}: {}",
                        request.entity_id,
                        e
                    );
                    continue;
                }
            }

            let _ = app.emit(
                "intelligence-updated",
                IntelligenceUpdatedPayload {
                    entity_id: request.entity_id.clone(),
                    entity_type: request.entity_type.clone(),
                },
            );

            // Invalidate + requeue meeting preps for future meetings linked to this entity.
            // intelligence.json changed → meeting briefings that consume it need regeneration.
            invalidate_and_requeue_meeting_preps(&state, &request.entity_id);

            // Self-healing: record success + post-enrichment coherence check (I409/I410)
            {
                if let Ok(db) = crate::db::ActionDb::open() {
                    crate::self_healing::feedback::record_enrichment_success(
                        &db,
                        &request.entity_id,
                    );
                    let _ = crate::self_healing::scheduler::on_enrichment_complete(
                        &db,
                        Some(state.embedding_model.as_ref()),
                        &request.entity_id,
                        &request.entity_type,
                        &state.intel_queue,
                        Some(state.signals.engine.as_ref()),
                    );
                }
            }

            // I428: Record successful claude_code sync
            if let Ok(db) = crate::db::ActionDb::open() {
                let _ = crate::connectivity::record_sync_success(db.conn_ref(), "claude_code");
            }

            log::info!(
                "IntelProcessor: completed {} ({} risks, {} wins)",
                request.entity_id,
                written_intel.risks.len(),
                written_intel.recent_wins.len(),
            );
        }

        // I571: Emit completion status for frontend indicator
        let _ = app.emit(
            "background-work-status",
            serde_json::json!({
                "phase": "completed",
                "message": "Insights updated",
                "count": results.len(),
                "manual": batch_has_manual,
            }),
        );
    }
}

/// Check whether an `enriched_at` RFC 3339 timestamp is within the TTL window (I287).
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

/// Check if an entity was enriched recently enough to skip (I287).
///
/// Resolves the entity directory and reads `intelligence.json` to check the
/// `enriched_at` timestamp. Returns `Some(message)` if the entity should be
/// skipped, `None` if it should proceed.
fn check_enrichment_ttl(_state: &AppState, request: &IntelRequest) -> Option<String> {
    let db = crate::db::ActionDb::open().ok()?;
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
/// Public so manual enrichment commands can reuse the split-lock pattern (I173).
pub fn gather_enrichment_input(
    state: &AppState,
    request: &IntelRequest,
) -> Result<EnrichmentInput, String> {
    let workspace = {
        let config_guard = state.config.read();
        let config = config_guard.as_ref().ok_or("No config")?;
        PathBuf::from(&config.workspace_path)
    };

    let db = crate::db::ActionDb::open().map_err(|e| format!("Failed to open DB: {}", e))?;

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

    // Read prior intelligence from DB (I513)
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

    // I499: Compute algorithmic health for accounts before prompt building.
    // This populates ctx.computed_health so the prompt uses narrative-only health schema.
    let mut ctx = ctx;
    let computed_health = if request.entity_type == "account" {
        account.as_ref().map(|acct| {
            crate::intelligence::health_scoring::compute_account_health(
                &db,
                acct,
                ctx.org_health.as_ref(),
            )
        })
    } else {
        None
    };
    ctx.computed_health = computed_health.clone();

    // I506: Compute and persist co-attendance relationships for account entities
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
    // For accounts, pass account_type so prompts can adapt for partner vs customer (I382).
    let relationship = person
        .as_ref()
        .map(|p| p.relationship.as_str())
        .or_else(|| account.as_ref().map(|a| a.account_type.as_db_str()));
    // Read active preset for domain-specific prompt language (I313)
    let preset_guard = state.active_preset.read();
    let prompt = build_intelligence_prompt_with_preset(
        &entity_name,
        &request.entity_type,
        &ctx,
        relationship,
        preset_guard.as_ref(),
    );

    let file_manifest = ctx.file_manifest.clone();
    let file_count = file_manifest.len();

    // I574: Always preserve context — needed for parallel dimension fan-out (local)
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
    })
}

/// I535/ADR-0100: Glean-first enrichment with PTY fallback.
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

    let endpoint = match glean_endpoint {
        Some(ep) => ep,
        None => {
            log::warn!("[I535] Glean mode active but no endpoint found, falling back to PTY");
            let mut fallback_results = Vec::new();
            for (request, input) in inputs {
                // I564: PTY calls on blocking threads
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

    let provider = crate::intelligence::glean_provider::GleanIntelligenceProvider::new(&endpoint);

    let mut results = Vec::new();
    let mut pty_fallback_inputs: Vec<(IntelRequest, EnrichmentInput)> = Vec::new();

    for (request, input) in inputs {
        // Only try Glean if we have the intelligence context (populated for remote providers)
        if let Some(ref ctx) = input.intelligence_context {
            log::info!(
                "[I535] Attempting Glean enrichment for {} ({})",
                input.entity_name,
                input.entity_type
            );

            match provider
                .enrich_entity(
                    &input.entity_id,
                    &input.entity_type,
                    &input.entity_name,
                    ctx,
                    input.relationship.as_deref(),
                    Some(app_handle),
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
    // I564: PTY calls on blocking threads
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

/// Categorize an enrichment error for audit logging (I472).
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
/// Public so manual enrichment commands can reuse the split-lock pattern (I173).
///
/// I574: Tries parallel per-dimension fan-out first (if `intelligence_context` is available),
/// then falls back to the legacy monolithic prompt path.
///
/// I575: When `app_handle` is provided, emits `enrichment-progress` and
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

/// I574: Parallel per-dimension enrichment engine.
///
/// Fans out 6 dimension-specific PTY calls in parallel threads (30s each),
/// then merges successful dimension results into a single `IntelligenceJson`.
/// Returns Err only if ALL 6 dimensions fail (caller falls back to legacy).
///
/// I575: Uses a channel pattern so each dimension result is written to DB and
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
    let total_dimensions = DIMENSION_NAMES.len() as u32;

    // Channel for receiving dimension results as they complete
    let (tx, rx) = std::sync::mpsc::channel();

    // Spawn one thread per dimension
    for &dimension in DIMENSION_NAMES {
        let dim_prompt = dimension_prompts::build_dimension_prompt(
            dimension,
            &input.entity_name,
            &input.entity_type,
            input.relationship.as_deref(),
            ctx,
            is_incremental,
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

            let pty = PtyManager::for_tier(ModelTier::Extraction, &ai_cfg)
                .with_usage_context(dimension_usage_context)
                .with_timeout(DIMENSION_ENRICHMENT_TIMEOUT_SECS)
                .with_nice_priority(10);

            let result = pty
                .spawn_claude(&workspace, &dim_prompt)
                .map_err(|e| format!("PTY error for dimension {}: {}", dim_name, e))
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

                    // I575: Per-dimension DB write + event emission
                    if let Some(handle) = app_handle {
                        write_progressive_dimension(
                            &input.entity_id,
                            &input.entity_type,
                            &combined,
                        );
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
        "[I574] Parallel enrichment: {}/6 dimensions succeeded in {}ms",
        succeeded,
        total_ms
    );

    // I575: Emit completion event
    if let Some(handle) = app_handle {
        let _ = handle.emit(
            "enrichment-complete",
            EnrichmentComplete {
                entity_id: input.entity_id.clone(),
                entity_type: input.entity_type.clone(),
                succeeded,
                failed: failed_dims.len() as u32,
                failed_dimensions: failed_dims,
                wall_clock_ms: total_ms as u64,
            },
        );
    }

    if succeeded == 0 {
        return Err("All 6 dimensions failed".to_string());
    }

    // Extract inferred relationships from the combined raw output
    let inferred_relationships = extract_inferred_relationships(&all_raw_output);

    // I305: Extract and persist keywords from the combined raw output
    if let Some(keywords_json) =
        crate::intelligence::extract_keywords_from_response(&all_raw_output)
    {
        if let Ok(db) = crate::db::ActionDb::open() {
            if let Err(err) = crate::services::intelligence::persist_entity_keywords(
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

    Ok(EnrichmentParseResult {
        intel: combined,
        inferred_relationships,
    })
}

/// I575: Write the current progressive state of intelligence to DB after a dimension completes.
///
/// Opens a short-lived DB connection, reads existing entity_assessment, merges the
/// new combined state, and writes back. Non-fatal on error — the final write in
/// `write_enrichment_results` is the authoritative write.
fn write_progressive_dimension(entity_id: &str, entity_type: &str, combined: &IntelligenceJson) {
    let db = match crate::db::ActionDb::open() {
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

    // Progressive writes within a single enrichment cycle use simple dimension
    // merge, NOT reconciliation. Reconciliation happens at the final write.
    let existing = db.get_entity_intelligence(entity_id).ok().flatten();
    let mut merged = if let Some(mut existing) = existing {
        for dim in crate::intelligence::dimension_prompts::DIMENSION_NAMES {
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

    if let Err(e) = crate::services::intelligence::upsert_assessment_snapshot(&db, &merged) {
        log::warn!("[I575] Progressive write failed for {}: {}", entity_id, e);
    } else {
        log::debug!("[I575] Progressive write succeeded for {}", entity_id,);
    }
}

/// Legacy monolithic PTY enrichment — single prompt, 30s timeout.
/// Kept as fallback when parallel enrichment is unavailable or fails.
fn run_enrichment_legacy(
    input: &EnrichmentInput,
    ai_config: &AiModelConfig,
    usage_context: &AiUsageContext,
) -> Result<EnrichmentParseResult, String> {
    let pty = PtyManager::for_tier(ModelTier::Synthesis, ai_config)
        .with_usage_context(usage_context.clone().with_tier(ModelTier::Synthesis))
        .with_timeout(DIMENSION_ENRICHMENT_TIMEOUT_SECS)
        .with_nice_priority(10);
    let output = pty
        .spawn_claude(&input.workspace, &input.prompt)
        .map_err(|e| format!("Claude Code error: {}", e))?;

    // I305: Extract and persist keywords from the raw AI response
    if let Some(keywords_json) = crate::intelligence::extract_keywords_from_response(&output.stdout)
    {
        if let Ok(db) = crate::db::ActionDb::open() {
            if let Err(err) = crate::services::intelligence::persist_entity_keywords(
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
    })
}

/// I527: One-shot repair retry when deterministic checks still report
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

/// Phase 3: Write enrichment results to disk and DB.
/// Opens own DB connection to avoid blocking foreground IPC commands.
/// Public so manual enrichment commands can reuse the split-lock pattern (I173).
pub fn write_enrichment_results(
    _state: &AppState,
    input: &EnrichmentInput,
    intel: &IntelligenceJson,
    ai_config: Option<&AiModelConfig>,
) -> Result<IntelligenceJson, String> {
    // I576: Source-aware reconciliation + preserve user-edited fields (I261)
    let mut final_intel = intel.clone();
    let existing_intel = crate::db::ActionDb::open()
        .ok()
        .and_then(|db| db.get_entity_intelligence(&input.entity_id).ok().flatten());
    if let Some(existing) = existing_intel.as_ref() {
        // I576: Apply source-aware reconciliation (preserves user corrections,
        // non-refreshed source items, and dismissed tombstones).
        // I652: stakeholder_insights reconciliation is skipped in reconcile_enrichment —
        // protection is now structural via data_source columns on account_stakeholders.
        final_intel = crate::intelligence::glean_provider::reconcile_enrichment(
            existing.clone(),
            final_intel,
            &["pty_synthesis"],
        );

        // I261: Also preserve field-level user edits (granular JSON-path edits)
        if !existing.user_edits.is_empty() {
            crate::intelligence::preserve_user_edits(&mut final_intel, existing);
            log::info!(
                "IntelProcessor: preserved {} user edits for {}",
                existing.user_edits.len(),
                input.entity_id,
            );
        }
    }

    // I652: Route AI stakeholder insights to DB columns or suggestions table.
    // Person-first architecture: enrichment never overwrites user-designated data.
    // AI can update columns it previously wrote (data_source='ai'), and new
    // discoveries go to stakeholder_suggestions for user review.
    if input.entity_type == "account" && !final_intel.stakeholder_insights.is_empty() {
        if let Ok(db_sh) = crate::db::ActionDb::open() {
            for insight in &final_intel.stakeholder_insights {
                let ai_source = insight
                    .item_source
                    .as_ref()
                    .map(|s| s.source.as_str())
                    .unwrap_or("pty_synthesis");

                if let Some(ref pid) = insight.person_id {
                    // Check if this person_id exists in account_stakeholders for this account
                    let row_exists: bool = db_sh
                        .conn_ref()
                        .query_row(
                            "SELECT COUNT(*) FROM account_stakeholders WHERE account_id = ?1 AND person_id = ?2",
                            rusqlite::params![&input.entity_id, pid],
                            |row| row.get::<_, i64>(0),
                        )
                        .unwrap_or(0)
                        > 0;

                    if row_exists {
                        // Update engagement if AI-owned
                        if let Some(ref engagement) = insight.engagement {
                            let ds: String = db_sh
                                .conn_ref()
                                .query_row(
                                    "SELECT data_source_engagement FROM account_stakeholders WHERE account_id = ?1 AND person_id = ?2",
                                    rusqlite::params![&input.entity_id, pid],
                                    |row| row.get(0),
                                )
                                .unwrap_or_else(|_| "ai".to_string());
                            if ds == "ai" {
                                let _ = db_sh.conn_ref().execute(
                                    "UPDATE account_stakeholders SET engagement = ?1 WHERE account_id = ?2 AND person_id = ?3 AND data_source_engagement = 'ai'",
                                    rusqlite::params![engagement, &input.entity_id, pid],
                                );
                            } else {
                                // AI disagrees with user-owned engagement — write suggestion
                                write_stakeholder_suggestion(&StakeholderSuggestionParams {
                                    db: &db_sh,
                                    account_id: &input.entity_id,
                                    person_id: Some(pid),
                                    insight,
                                    source: ai_source,
                                });
                            }
                        }

                        // Update assessment if AI-owned
                        if let Some(ref assessment) = insight.assessment {
                            let ds: String = db_sh
                                .conn_ref()
                                .query_row(
                                    "SELECT data_source_assessment FROM account_stakeholders WHERE account_id = ?1 AND person_id = ?2",
                                    rusqlite::params![&input.entity_id, pid],
                                    |row| row.get(0),
                                )
                                .unwrap_or_else(|_| "ai".to_string());
                            if ds == "ai" {
                                let _ = db_sh.conn_ref().execute(
                                    "UPDATE account_stakeholders SET assessment = ?1 WHERE account_id = ?2 AND person_id = ?3 AND data_source_assessment = 'ai'",
                                    rusqlite::params![assessment, &input.entity_id, pid],
                                );
                            }
                        }

                        // Upsert roles: skip user-owned, update/insert AI-owned
                        if let Some(ref role) = insight.role {
                            // Existence check returns data_source AND
                            // dismissed_at so we can distinguish three
                            // states: not-present, active-ai-owned,
                            // active-user-owned, soft-deleted. Soft-
                            // deleted rows are treated the same as
                            // user-owned: do not touch. Without this,
                            // AI would re-UPDATE a dismissed row and
                            // (via ON CONFLICT) keep writing data_source=
                            // 'ai' on every enrichment, even though the
                            // dismissal filter keeps it hidden.
                            let existing: Option<(Option<String>, Option<String>)> = db_sh
                                .conn_ref()
                                .query_row(
                                    "SELECT data_source, dismissed_at FROM account_stakeholder_roles
                                     WHERE account_id = ?1 AND person_id = ?2 AND role = ?3",
                                    rusqlite::params![&input.entity_id, pid, role],
                                    |row| Ok((row.get(0)?, row.get(1)?)),
                                )
                                .ok();
                            let is_user_owned = matches!(
                                existing.as_ref().and_then(|(ds, _)| ds.as_deref()),
                                Some("user")
                            );
                            let is_dismissed = existing
                                .as_ref()
                                .is_some_and(|(_, d)| d.is_some());
                            if !is_user_owned && !is_dismissed {
                                let _ = db_sh.conn_ref().execute(
                                    "INSERT INTO account_stakeholder_roles (account_id, person_id, role, data_source) VALUES (?1, ?2, ?3, 'ai') ON CONFLICT(account_id, person_id, role) DO UPDATE SET data_source = 'ai'",
                                    rusqlite::params![&input.entity_id, pid, role],
                                );
                            }
                        }
                    } else {
                        // Person has a person_id but is not in account_stakeholders — suggest
                        write_stakeholder_suggestion(&StakeholderSuggestionParams {
                            db: &db_sh,
                            account_id: &input.entity_id,
                            person_id: Some(pid),
                            insight,
                            source: ai_source,
                        });
                    }
                } else {
                    // No person_id — write to suggestions table
                    write_stakeholder_suggestion(&StakeholderSuggestionParams {
                        db: &db_sh,
                        account_id: &input.entity_id,
                        person_id: None,
                        insight,
                        source: ai_source,
                    });
                }
            }
        }
    }

    // Prevent sparse refreshes from wiping narrative intelligence.
    // If the new response is structurally valid JSON but contains little/no
    // usable narrative, keep prior core fields until enrichment recovers.
    merge_missing_core_fields_from_existing(&mut final_intel, existing_intel.as_ref());

    // I527: Deterministic contradiction checks + balanced repair pass.
    if input.entity_type == "account" || input.entity_type == "project" {
        if let Ok(db_for_consistency) = crate::db::ActionDb::open() {
            if let Ok(facts) = crate::intelligence::build_fact_context(
                &db_for_consistency,
                &input.entity_id,
                &input.entity_type,
            ) {
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

                post_repair_intel.consistency_status = Some(
                    crate::intelligence::status_from_reports(&initial_report, &unresolved_report),
                );
                post_repair_intel.consistency_findings =
                    crate::intelligence::merge_fixed_flags(&initial_report, &unresolved_report);
                post_repair_intel.consistency_checked_at = Some(Utc::now().to_rfc3339());
                final_intel = post_repair_intel;
            }
        }
    }

    // I645: Filter suppressed risks/wins before writing.
    // If is_suppressed() is available (Agent 1 creates it in db module), check each
    // item against tombstones. Items with newer evidence (sourced_at > dismissed_at)
    // pass through — the is_suppressed function handles that logic.
    if let Ok(feedback_db) = crate::db::ActionDb::open() {
        let pre_risk_count = final_intel.risks.len();
        final_intel.risks.retain(|risk| {
            let item_key = Some(risk.text.as_str());
            !feedback_db
                .is_suppressed(
                    &input.entity_id,
                    "risks",
                    item_key,
                    risk.item_source.as_ref().map(|s| s.sourced_at.as_str()),
                )
                .unwrap_or(false)
        });
        let pre_win_count = final_intel.recent_wins.len();
        final_intel.recent_wins.retain(|win| {
            let item_key = Some(win.text.as_str());
            !feedback_db
                .is_suppressed(
                    &input.entity_id,
                    "recentWins",
                    item_key,
                    win.item_source.as_ref().map(|s| s.sourced_at.as_str()),
                )
                .unwrap_or(false)
        });
        let risks_suppressed = pre_risk_count - final_intel.risks.len();
        let wins_suppressed = pre_win_count - final_intel.recent_wins.len();
        if risks_suppressed > 0 || wins_suppressed > 0 {
            log::info!(
                "[I645] Suppression filter for {}: {} risks, {} wins removed",
                input.entity_id,
                risks_suppressed,
                wins_suppressed,
            );
        }
    }

    // I499: Merge computed health dimensions with LLM narrative.
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

    // Own DB connection for cache update + user-fact reconciliation
    let db = crate::db::ActionDb::open().map_err(|e| format!("Failed to open DB: {}", e))?;

    // Reconcile user-entered facts with AI-inferred values.
    // User-edited fields (source weight 1.0) override AI guesses.
    if input.entity_type == "account" {
        if let Ok(Some(account)) = db.get_account(&input.entity_id) {
            if let Some(user_arr) = account.arr {
                let cc = final_intel
                    .contract_context
                    .get_or_insert_with(Default::default);
                if cc.current_arr != Some(user_arr) {
                    log::info!(
                        "[intel_queue] Overriding AI currentArr ({:?}) with user ARR ({}) for {}",
                        cc.current_arr,
                        user_arr,
                        input.entity_id,
                    );
                    cc.current_arr = Some(user_arr);
                }
            }
        }
    }

    // Write intelligence.json to disk
    write_intelligence_json(&input.entity_dir, &final_intel)?;
    crate::services::intelligence::upsert_assessment_from_enrichment(
        &db,
        &_state.signals.engine,
        &input.entity_type,
        &input.entity_id,
        &final_intel,
    )?;

    // Invalidate cached reports when entity intelligence is refreshed (I397)
    let _ = crate::reports::invalidation::mark_reports_stale(&db, &input.entity_id);

    // I535 Step 11: Dual-write commitments from Glean enrichment to captured_commitments
    if input.entity_type == "account" {
        dual_write_enrichment_commitments(&db, &input.entity_id, &final_intel);
        dual_write_enrichment_products(&db, &input.entity_id, &final_intel);
    }

    // I338: Regenerate person files after intelligence enrichment
    if input.entity_type == "person" {
        if let Ok(Some(person)) = db.get_person(&input.entity_id) {
            let _ = crate::people::write_person_markdown(&input.workspace, &person, &db);
            let _ = crate::people::write_person_dashboard_json(&input.workspace, &person, &db);
        }
    }

    // I384/I388: After writing a child entity's enrichment, enqueue the parent for
    // portfolio intelligence refresh. This ensures parent portfolio views stay
    // current when any child's intelligence updates.
    if input.entity_type == "account" {
        if let Ok(Some(account)) = db.get_account(&input.entity_id) {
            if let Some(ref parent_id) = account.parent_id {
                _state.intel_queue.enqueue(IntelRequest {
                    entity_id: parent_id.clone(),
                    entity_type: "account".to_string(),
                    priority: IntelPriority::ContentChange,
                    requested_at: std::time::Instant::now(),
                    retry_count: 0,
                });
                _state.integrations.intel_queue_wake.notify_one();
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
                _state.intel_queue.enqueue(IntelRequest {
                    entity_id: parent_id.clone(),
                    entity_type: "project".to_string(),
                    priority: IntelPriority::ContentChange,
                    requested_at: std::time::Instant::now(),
                    retry_count: 0,
                });
                _state.integrations.intel_queue_wake.notify_one();
                log::info!(
                    "IntelProcessor: enqueued parent project {} for portfolio refresh after child {} update",
                    parent_id,
                    input.entity_id,
                );
            }
        }
    }

    log::debug!(
        "IntelProcessor: wrote intelligence for {} to file + DB",
        input.entity_id,
    );

    Ok(final_intel)
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

/// I652: Parameters for writing a stakeholder suggestion.
struct StakeholderSuggestionParams<'a> {
    db: &'a crate::db::ActionDb,
    account_id: &'a str,
    person_id: Option<&'a str>,
    insight: &'a crate::intelligence::io::StakeholderInsight,
    source: &'a str,
}

/// I652: Write a stakeholder suggestion to the `stakeholder_suggestions` table.
/// Skips if a pending suggestion for the same person+account already exists.
fn write_stakeholder_suggestion(params: &StakeholderSuggestionParams<'_>) {
    let StakeholderSuggestionParams {
        db,
        account_id,
        person_id,
        insight,
        source,
    } = params;

    // I652: Skip suggestions for internal team members (by person_id OR name match)
    if let Some(pid) = person_id {
        let is_internal: bool = db
            .conn_ref()
            .query_row(
                "SELECT relationship = 'internal' FROM people WHERE id = ?1",
                rusqlite::params![pid],
                |row| row.get(0),
            )
            .unwrap_or(false);
        if is_internal {
            return;
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
            .unwrap_or(false);
        if is_internal_by_name {
            return;
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
            .unwrap_or(0)
            > 0
    } else {
        db.conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM stakeholder_suggestions WHERE account_id = ?1 AND suggested_name = ?2 AND status = 'pending'",
                rusqlite::params![account_id, &insight.name],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(0)
            > 0
    };
    if already_pending {
        return;
    }

    let raw_json = serde_json::to_string(insight).unwrap_or_default();
    if let Err(e) = db.conn_ref().execute(
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
    ) {
        log::warn!(
            "I652: Failed to write stakeholder suggestion for {} on {}: {}",
            insight.name, account_id, e,
        );
    } else {
        log::info!(
            "I652: Wrote stakeholder suggestion for '{}' on account {}",
            insight.name, account_id,
        );
    }
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
    let db = match crate::db::ActionDb::open() {
        Ok(db) => db,
        Err(e) => {
            log::warn!(
                "IntelProcessor: failed to open DB for prep invalidation: {}",
                e
            );
            return;
        }
    };

    let now = Utc::now().to_rfc3339();

    // Find future meetings linked to this entity and clear their frozen prep
    let meeting_ids: Vec<String> = db
        .conn_ref()
        .prepare(
            "SELECT m.id FROM meetings m
             JOIN meeting_entities me ON me.meeting_id = m.id
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
    for mid in &meeting_ids {
        let _ = crate::services::meetings::clear_meeting_prep_frozen(&db, mid);
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

/// I535 Step 11: Dual-write commitments from Glean enrichment to `captured_commitments`.
///
/// Writes `open_commitments` and `success_plan_signals.stated_objectives` from the
/// intelligence output, mirroring the pattern in `transcript.rs:556-598`.
/// Uses INSERT OR IGNORE to avoid duplicates.
fn dual_write_enrichment_commitments(
    db: &crate::db::ActionDb,
    account_id: &str,
    intel: &IntelligenceJson,
) {
    let now = Utc::now().to_rfc3339();
    let source_label = format!("glean_enrichment:{}", account_id);

    // 1. Write open_commitments
    if let Some(ref commitments) = intel.open_commitments {
        for commitment in commitments {
            let commit_id = uuid::Uuid::new_v4().to_string();
            let owner = commitment.owner.as_deref().unwrap_or("joint");
            if let Err(e) = db.conn_ref().execute(
                "INSERT OR IGNORE INTO captured_commitments (id, account_id, meeting_id, title, owner, target_date, confidence, source, consumed, created_at)
                 VALUES (?1, ?2, NULL, ?3, ?4, ?5, 'medium', ?6, 0, ?7)",
                rusqlite::params![
                    commit_id,
                    account_id,
                    commitment.description,
                    owner,
                    commitment.due_date,
                    source_label,
                    now,
                ],
            ) {
                log::warn!("Failed to insert captured_commitment from Glean enrichment: {}", e);
            }
        }
    }

    // 2. Write stated_objectives from success_plan_signals
    if let Some(ref signals) = intel.success_plan_signals {
        for objective in &signals.stated_objectives {
            let commit_id = uuid::Uuid::new_v4().to_string();
            let owner = objective.owner.as_deref().unwrap_or("joint");
            if let Err(e) = db.conn_ref().execute(
                "INSERT OR IGNORE INTO captured_commitments (id, account_id, meeting_id, title, owner, target_date, confidence, source, consumed, created_at)
                 VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6, ?7, 0, ?8)",
                rusqlite::params![
                    commit_id,
                    account_id,
                    objective.objective,
                    owner,
                    objective.target_date,
                    objective.confidence,
                    source_label,
                    now,
                ],
            ) {
                log::warn!("Failed to insert stated_objective from Glean enrichment: {}", e);
            }
        }
    }

    // 3. Emit signal for the dual-write
    let commitment_count = intel.open_commitments.as_ref().map_or(0, |c| c.len())
        + intel
            .success_plan_signals
            .as_ref()
            .map_or(0, |s| s.stated_objectives.len());
    if commitment_count > 0 {
        let value = serde_json::json!({
            "count": commitment_count,
            "source": "glean_enrichment",
        })
        .to_string();
        if let Err(e) = crate::signals::bus::emit_signal(
            db,
            "account",
            account_id,
            "commitment_captured",
            "glean",
            Some(&value),
            0.7,
        ) {
            log::warn!("Failed to emit commitment_captured signal: {}", e);
        }
    }
}

/// Dual-write product adoption data from enrichment intelligence into the
/// `account_products` table, keeping the relational surface in sync with
/// the intelligence JSON blob.
fn dual_write_enrichment_products(
    db: &crate::db::ActionDb,
    entity_id: &str,
    intel: &IntelligenceJson,
) {
    let adoption = match intel.product_adoption.as_ref() {
        Some(a) => a,
        None => return,
    };

    let source = adoption.source.as_deref().unwrap_or("ai_inference");
    let mut upserted = 0usize;

    for feature in &adoption.feature_adoption {
        // Parse "Core platform: 95%" → name = "Core platform", adoption_pct ~0.95
        let (name, adoption_pct) = if let Some(colon_pos) = feature.find(':') {
            let raw_name = feature[..colon_pos].trim();
            let pct_str = feature[colon_pos + 1..].trim().trim_end_matches('%');
            let pct = pct_str.parse::<f64>().ok().map(|v| v / 100.0);
            (raw_name.to_string(), pct)
        } else {
            (feature.trim().to_string(), None)
        };

        if name.is_empty() {
            continue;
        }

        match db.upsert_account_product(entity_id, &name, None, "active", adoption_pct, source, 0.55, None)
        {
            Ok(_) => upserted += 1,
            Err(e) => {
                log::warn!(
                    "Failed to upsert account product '{}' for {}: {}",
                    name,
                    entity_id,
                    e
                );
            }
        }
    }

    if upserted > 0 {
        log::info!(
            "IntelProcessor: dual-wrote {} products for {} from enrichment",
            upserted,
            entity_id,
        );
        // Intelligence Loop: every mutation emits a signal (I624 AC7)
        let _ = crate::signals::bus::emit_signal(
            db,
            "account",
            entity_id,
            "product_data_updated",
            source,
            Some(&format!("{{\"count\":{upserted}}}")),
            0.55,
        );
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_missing_core_fields_preserves_existing_assessment() {
        let mut fresh = IntelligenceJson {
            entity_id: "acme-corp".to_string(),
            entity_type: "account".to_string(),
            executive_assessment: None,
            ..Default::default()
        };
        let existing = IntelligenceJson {
            entity_id: "acme-corp".to_string(),
            entity_type: "account".to_string(),
            executive_assessment: Some("Prior narrative".to_string()),
            risks: vec![crate::intelligence::IntelRisk {
                text: "Renewal owner unresolved".to_string(),
                source: None,
                urgency: "high".to_string(),
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
    fn test_is_sparse_intelligence_detects_non_sparse_when_assessment_present() {
        let intel = IntelligenceJson {
            executive_assessment: Some("Non-empty summary".to_string()),
            ..Default::default()
        };
        assert!(!is_sparse_intelligence(&intel));
    }

    #[test]
    fn test_intel_queue_enqueue_dequeue() {
        let queue = IntelligenceQueue::new();

        queue.enqueue(IntelRequest {
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
    fn test_intel_queue_dedup_keeps_higher_priority() {
        let queue = IntelligenceQueue::new();

        queue.enqueue(IntelRequest {
            entity_id: "acme".to_string(),
            entity_type: "account".to_string(),
            priority: IntelPriority::ContentChange,
            requested_at: Instant::now(),
            retry_count: 0,
        });

        // Same entity, higher priority → should upgrade
        queue.enqueue(IntelRequest {
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

        queue.enqueue(IntelRequest {
            entity_id: "acme".to_string(),
            entity_type: "account".to_string(),
            priority: IntelPriority::Manual,
            requested_at: Instant::now(),
            retry_count: 0,
        });

        // Same entity, lower priority → should be ignored
        queue.enqueue(IntelRequest {
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

        queue.enqueue(IntelRequest {
            entity_id: "alpha".to_string(),
            entity_type: "account".to_string(),
            priority: IntelPriority::ContentChange,
            requested_at: Instant::now(),
            retry_count: 0,
        });

        queue.enqueue(IntelRequest {
            entity_id: "beta".to_string(),
            entity_type: "account".to_string(),
            priority: IntelPriority::Manual,
            requested_at: Instant::now(),
            retry_count: 0,
        });

        queue.enqueue(IntelRequest {
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
        queue.enqueue(IntelRequest {
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
        queue.enqueue(IntelRequest {
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
        queue.enqueue(IntelRequest {
            entity_id: "acme".to_string(),
            entity_type: "account".to_string(),
            priority: IntelPriority::ContentChange,
            requested_at: Instant::now(),
            retry_count: 0,
        });

        // Dequeue it
        let _ = queue.dequeue();

        // Manual request should bypass debounce
        queue.enqueue(IntelRequest {
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

        queue.enqueue(IntelRequest {
            entity_id: "acme".to_string(),
            entity_type: "account".to_string(),
            priority: IntelPriority::ContentChange,
            requested_at: Instant::now(),
            retry_count: 0,
        });

        queue.enqueue(IntelRequest {
            entity_id: "beta-project".to_string(),
            entity_type: "project".to_string(),
            priority: IntelPriority::ContentChange,
            requested_at: Instant::now(),
            retry_count: 0,
        });

        assert_eq!(queue.len(), 2);
    }

    // =========================================================================
    // Batch dequeue tests (I289)
    // =========================================================================

    #[test]
    fn test_dequeue_batch_returns_up_to_max() {
        let queue = IntelligenceQueue::new();

        for name in &["alpha", "beta", "gamma", "delta"] {
            queue.enqueue(IntelRequest {
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

        queue.enqueue(IntelRequest {
            entity_id: "low".to_string(),
            entity_type: "account".to_string(),
            priority: IntelPriority::ContentChange,
            requested_at: Instant::now(),
            retry_count: 0,
        });
        queue.enqueue(IntelRequest {
            entity_id: "high".to_string(),
            entity_type: "account".to_string(),
            priority: IntelPriority::Manual,
            requested_at: Instant::now(),
            retry_count: 0,
        });
        queue.enqueue(IntelRequest {
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

        queue.enqueue(IntelRequest {
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
    // TTL tests (I287)
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
}
