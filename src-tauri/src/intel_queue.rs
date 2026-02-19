//! Background intelligence enrichment queue (I132).
//!
//! Provides a priority queue for intelligence enrichment requests with
//! deduplication and debounce. A background processor drains the queue
//! and runs enrichment with split DB locking so the UI stays responsive
//! during the 30-120s PTY operation.

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use chrono::Utc;

use tauri::{AppHandle, Emitter};

use crate::entity_intel::{
    build_intelligence_context, build_intelligence_prompt, parse_intelligence_response,
    read_intelligence_json, write_intelligence_json, IntelligenceJson, SourceManifestEntry,
};
use crate::pty::{ModelTier, PtyManager};
use crate::state::AppState;
use crate::types::AiModelConfig;

/// Maximum number of entities to batch in a single PTY call (I289).
const MAX_BATCH_SIZE: usize = 3;

/// Debounce window for content-triggered enrichment requests.
const CONTENT_DEBOUNCE_SECS: u64 = 30;

/// How often the background processor checks for work.
const POLL_INTERVAL_SECS: u64 = 5;

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
    /// User clicked "Refresh Intelligence" manually.
    Manual = 3,
}

/// A request to enrich an entity's intelligence.
#[derive(Debug, Clone)]
pub struct IntelRequest {
    pub entity_id: String,
    pub entity_type: String,
    pub priority: IntelPriority,
    pub requested_at: Instant,
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
        // Debounce: skip if same entity was enqueued recently (low-priority triggers only)
        if request.priority == IntelPriority::ContentChange
            || request.priority == IntelPriority::ProactiveHygiene
        {
            if let Ok(last) = self.last_enqueued.lock() {
                if let Some(last_time) = last.get(&request.entity_id) {
                    if last_time.elapsed().as_secs() < CONTENT_DEBOUNCE_SECS {
                        log::debug!(
                            "IntelQueue: debounced {} ({}s since last)",
                            request.entity_id,
                            last_time.elapsed().as_secs()
                        );
                        return;
                    }
                }
            }
        }

        let mut queue = match self.queue.lock() {
            Ok(q) => q,
            Err(_) => return,
        };

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
        if let Ok(mut last) = self.last_enqueued.lock() {
            last.insert(request.entity_id, Instant::now());
        }
    }

    /// Dequeue the highest-priority request.
    pub fn dequeue(&self) -> Option<IntelRequest> {
        let mut queue = self.queue.lock().ok()?;
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
        let mut queue = match self.queue.lock() {
            Ok(q) => q,
            Err(_) => return Vec::new(),
        };
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
        self.queue.lock().map(|q| q.len()).unwrap_or(0)
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
        if let Ok(mut last) = self.last_enqueued.lock() {
            let before = last.len();
            last.retain(|_, instant| instant.elapsed().as_secs() < stale_threshold_secs);
            let pruned = before - last.len();
            if pruned > 0 {
                log::debug!("IntelQueue: pruned {} stale debounce entries", pruned);
            }
        }
    }
}

/// Payload emitted when intelligence is updated.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntelligenceUpdatedPayload {
    pub entity_id: String,
    pub entity_type: String,
}

/// Context gathered from the DB (held briefly, then released before PTY).
/// Public so manual enrichment commands can reuse the split-lock pattern (I173).
pub struct EnrichmentInput {
    pub workspace: PathBuf,
    pub entity_dir: PathBuf,
    pub entity_id: String,
    pub entity_type: String,
    pub prompt: String,
    pub file_manifest: Vec<SourceManifestEntry>,
    pub file_count: usize,
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
        tokio::time::sleep(std::time::Duration::from_secs(POLL_INTERVAL_SECS)).await;

        // Periodic pruning of stale debounce entries (I234)
        polls_since_prune += 1;
        if polls_since_prune >= prune_interval {
            state.intel_queue.prune_stale_entries();
            polls_since_prune = 0;
        }

        // Phase 0: Dequeue up to MAX_BATCH_SIZE requests (I289)
        let batch = state.intel_queue.dequeue_batch(MAX_BATCH_SIZE);
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

        // Phase 2: Run PTY enrichment (no DB lock held)
        let ai_config = state
            .config
            .read()
            .ok()
            .and_then(|g| g.as_ref().map(|c| c.ai_models.clone()))
            .unwrap_or_default();

        let results: Vec<(IntelRequest, EnrichmentInput, IntelligenceJson)> = if inputs.len() == 1 {
            // Single entity — use existing direct path (no batching overhead)
            let (request, input) = inputs.pop().unwrap();
            match run_enrichment(&input, &ai_config) {
                Ok(intel) => vec![(request, input, intel)],
                Err(e) => {
                    log::warn!(
                        "IntelProcessor: enrichment failed for {}: {}",
                        request.entity_id,
                        e
                    );
                    Vec::new()
                }
            }
        } else {
            // Multi-entity batch — combined prompt with delimiters (I289)
            run_batch_enrichment(inputs, &ai_config)
        };

        // Phase 3 + 4: Write results and emit events for each entity
        for (request, input, intel) in &results {
            if let Err(e) = write_enrichment_results(&state, input, intel) {
                log::warn!(
                    "IntelProcessor: failed to write results for {}: {}",
                    request.entity_id,
                    e
                );
                continue;
            }

            let _ = app.emit(
                "intelligence-updated",
                IntelligenceUpdatedPayload {
                    entity_id: request.entity_id.clone(),
                    entity_type: request.entity_type.clone(),
                },
            );

            log::info!(
                "IntelProcessor: completed {} ({} risks, {} wins)",
                request.entity_id,
                intel.risks.len(),
                intel.recent_wins.len(),
            );
        }
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
    let age_secs = (Utc::now() - ts.with_timezone(&Utc))
        .num_seconds()
        .max(0) as u64;

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
fn check_enrichment_ttl(state: &AppState, request: &IntelRequest) -> Option<String> {
    let workspace = {
        let config_guard = state.config.read().ok()?;
        let config = config_guard.as_ref()?;
        PathBuf::from(&config.workspace_path)
    };

    let entity_dir = resolve_entity_dir(&workspace, request)?;
    let intel = read_intelligence_json(&entity_dir).ok()?;

    enrichment_age_check(&intel.enriched_at, &request.entity_id)
}

/// Resolve an entity's directory from its request metadata.
/// Lightweight helper that opens a short-lived DB connection.
fn resolve_entity_dir(workspace: &Path, request: &IntelRequest) -> Option<PathBuf> {
    let db = crate::db::ActionDb::open().ok()?;

    match request.entity_type.as_str() {
        "account" => {
            let acct = db.get_account(&request.entity_id).ok()??;
            Some(crate::accounts::resolve_account_dir(workspace, &acct))
        }
        "project" => {
            let proj = db.get_project(&request.entity_id).ok()??;
            Some(crate::projects::project_dir(workspace, &proj.name))
        }
        "person" => {
            let person = db.get_person(&request.entity_id).ok()??;
            Some(crate::people::person_dir(workspace, &person.name))
        }
        _ => None,
    }
}

/// Phase 1: Open own DB connection to gather all context needed for enrichment.
/// Uses `ActionDb::open()` instead of `state.db.lock()` to avoid blocking
/// foreground IPC commands while background enrichment runs.
/// Public so manual enrichment commands can reuse the split-lock pattern (I173).
pub fn gather_enrichment_input(
    state: &AppState,
    request: &IntelRequest,
) -> Result<EnrichmentInput, String> {
    let workspace = {
        let config_guard = state.config.read().map_err(|_| "Config lock poisoned")?;
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

    // Read prior intelligence
    let prior = read_intelligence_json(&entity_dir).ok();

    // Build context (reads from DB)
    let ctx = build_intelligence_context(
        &workspace,
        &db,
        &request.entity_id,
        &request.entity_type,
        account.as_ref(),
        project.as_ref(),
        prior.as_ref(),
        Some(state.embedding_model.as_ref()),
    );

    // Build prompt (pure function, but easier to do here while we have the data)
    // Extract relationship for person entities so the prompt adapts framing
    let relationship = person.as_ref().map(|p| p.relationship.as_str());
    // Read vocabulary from active preset for domain-specific prompt language (I313)
    let preset_guard = state.active_preset.read().map_err(|_| "Preset lock poisoned")?;
    let vocabulary = preset_guard.as_ref().map(|p| &p.vocabulary);
    let prompt = build_intelligence_prompt(&entity_name, &request.entity_type, &ctx, relationship, vocabulary);

    let file_manifest = ctx.file_manifest.clone();
    let file_count = file_manifest.len();

    // Own DB connection drops here when db goes out of scope
    Ok(EnrichmentInput {
        workspace,
        entity_dir,
        entity_id: request.entity_id.clone(),
        entity_type: request.entity_type.clone(),
        prompt,
        file_manifest,
        file_count,
    })
}

/// Phase 2: Run PTY enrichment (no DB lock held).
/// Public so manual enrichment commands can reuse the split-lock pattern (I173).
pub fn run_enrichment(
    input: &EnrichmentInput,
    ai_config: &AiModelConfig,
) -> Result<IntelligenceJson, String> {
    let pty = PtyManager::for_tier(ModelTier::Synthesis, ai_config)
        .with_timeout(180)
        .with_nice_priority(10);
    let output = pty
        .spawn_claude(&input.workspace, &input.prompt)
        .map_err(|e| format!("Claude Code error: {}", e))?;

    // I305: Extract and persist keywords from the raw AI response
    if let Some(keywords_json) =
        crate::entity_intel::extract_keywords_from_response(&output.stdout)
    {
        if let Ok(db) = crate::db::ActionDb::open() {
            match input.entity_type.as_str() {
                "account" => {
                    let _ = db.update_account_keywords(&input.entity_id, &keywords_json);
                }
                "project" => {
                    let _ = db.update_project_keywords(&input.entity_id, &keywords_json);
                }
                _ => {}
            }
        }
    }

    parse_intelligence_response(
        &output.stdout,
        &input.entity_id,
        &input.entity_type,
        input.file_count,
        input.file_manifest.clone(),
    )
}

/// Run batched enrichment for multiple entities in a single PTY call (I289).
///
/// Builds a combined prompt with per-entity delimiters, makes one PTY call,
/// and parses the response back into per-entity results. Falls back to
/// individual processing for any entity whose result cannot be parsed.
fn run_batch_enrichment(
    inputs: Vec<(IntelRequest, EnrichmentInput)>,
    ai_config: &AiModelConfig,
) -> Vec<(IntelRequest, EnrichmentInput, IntelligenceJson)> {
    let entity_names: Vec<&str> = inputs.iter().map(|(_, i)| i.entity_id.as_str()).collect();
    log::info!(
        "IntelProcessor: running batch enrichment for {} entities: {:?}",
        inputs.len(),
        entity_names,
    );

    // Use the first input's workspace for the PTY call
    let workspace = inputs[0].1.workspace.clone();

    // Build combined prompt
    let batch_prompt = build_batch_prompt(&inputs);

    // Scale timeout linearly with batch size (180s base per entity)
    let timeout_secs = 180 * inputs.len() as u64;

    let pty = PtyManager::for_tier(ModelTier::Synthesis, ai_config)
        .with_timeout(timeout_secs)
        .with_nice_priority(10);

    let output = match pty.spawn_claude(&workspace, &batch_prompt) {
        Ok(o) => o,
        Err(e) => {
            log::warn!(
                "IntelProcessor: batch PTY call failed, falling back to individual: {}",
                e
            );
            return run_individual_fallback(inputs, ai_config);
        }
    };

    // Parse combined response into per-entity results
    let parsed = parse_batch_response(&output.stdout, &inputs);

    // Collect successful parses and entities that need individual fallback
    let mut results: Vec<(IntelRequest, EnrichmentInput, IntelligenceJson)> = Vec::new();
    let mut fallback_inputs: Vec<(IntelRequest, EnrichmentInput)> = Vec::new();

    // Match parsed results back to inputs by entity_id
    let mut parsed_map: HashMap<String, IntelligenceJson> = parsed.into_iter().collect();

    for (request, input) in inputs {
        if let Some(intel) = parsed_map.remove(&input.entity_id) {
            results.push((request, input, intel));
        } else {
            log::warn!(
                "IntelProcessor: batch parse failed for {}, will retry individually",
                input.entity_id,
            );
            fallback_inputs.push((request, input));
        }
    }

    // Run individual fallback for unparsed entities
    if !fallback_inputs.is_empty() {
        log::info!(
            "IntelProcessor: running individual fallback for {} entities",
            fallback_inputs.len(),
        );
        results.extend(run_individual_fallback(fallback_inputs, ai_config));
    }

    results
}

/// Build a combined prompt for multiple entities (I289).
///
/// Structure:
/// - Shared preamble (instructions for batch mode)
/// - Per-entity sections delimited by `=== ENTITY: {entity_id} ===`
/// - Instructions to produce output delimited by `=== RESULT: {entity_id} ===`
fn build_batch_prompt(inputs: &[(IntelRequest, EnrichmentInput)]) -> String {
    let entity_ids: Vec<&str> = inputs.iter().map(|(_, i)| i.entity_id.as_str()).collect();

    let mut prompt = String::with_capacity(inputs.len() * 4096);

    // Shared preamble
    prompt.push_str(
        "You are running intelligence assessments for MULTIPLE entities in a single pass.\n\
         Process each entity independently. Do NOT cross-reference information between entities.\n\n\
         For each entity below, produce the EXACT same JSON output format as you would for a single entity.\n\
         Separate each result with the delimiter shown below.\n\n\
         OUTPUT FORMAT:\n\
         For each entity, output:\n",
    );

    for id in &entity_ids {
        prompt.push_str(&format!("=== RESULT: {} ===\n<JSON for this entity>\n\n", id));
    }

    prompt.push_str(
        "Each JSON block must be a complete, valid JSON object on its own.\n\
         Do NOT output anything before the first === RESULT delimiter or after the last JSON block.\n\n",
    );

    // Per-entity sections
    for (_, input) in inputs {
        prompt.push_str(&format!(
            "=== ENTITY: {} ===\n{}\n\n",
            input.entity_id, input.prompt,
        ));
    }

    prompt
}

/// Parse a batched PTY response back into per-entity results (I289).
///
/// Splits on `=== RESULT: {entity_id} ===` delimiters and parses each section
/// independently. Returns a vec of (entity_id, IntelligenceJson) pairs for
/// successfully parsed entities.
fn parse_batch_response(
    response: &str,
    inputs: &[(IntelRequest, EnrichmentInput)],
) -> Vec<(String, IntelligenceJson)> {
    let mut results = Vec::new();

    for (_, input) in inputs {
        let delimiter = format!("=== RESULT: {} ===", input.entity_id);

        // Find the section for this entity
        let section = if let Some(start) = response.find(&delimiter) {
            let after_delimiter = start + delimiter.len();
            let remaining = &response[after_delimiter..];

            // Find end: next === RESULT delimiter or end of string
            let end = remaining
                .find("=== RESULT:")
                .unwrap_or(remaining.len());

            remaining[..end].trim()
        } else {
            log::warn!(
                "IntelProcessor: no result delimiter found for {} in batch response",
                input.entity_id,
            );
            continue;
        };

        if section.is_empty() {
            log::warn!(
                "IntelProcessor: empty result section for {} in batch response",
                input.entity_id,
            );
            continue;
        }

        match parse_intelligence_response(
            section,
            &input.entity_id,
            &input.entity_type,
            input.file_count,
            input.file_manifest.clone(),
        ) {
            Ok(intel) => results.push((input.entity_id.clone(), intel)),
            Err(e) => {
                log::warn!(
                    "IntelProcessor: failed to parse batch result for {}: {}",
                    input.entity_id,
                    e,
                );
            }
        }
    }

    results
}

/// Fallback: process entities individually when batch parsing fails (I289).
fn run_individual_fallback(
    inputs: Vec<(IntelRequest, EnrichmentInput)>,
    ai_config: &AiModelConfig,
) -> Vec<(IntelRequest, EnrichmentInput, IntelligenceJson)> {
    let mut results = Vec::new();

    for (request, input) in inputs {
        match run_enrichment(&input, ai_config) {
            Ok(intel) => results.push((request, input, intel)),
            Err(e) => {
                log::warn!(
                    "IntelProcessor: individual fallback failed for {}: {}",
                    request.entity_id,
                    e,
                );
            }
        }
    }

    results
}

/// Phase 3: Write enrichment results to disk and DB.
/// Opens own DB connection to avoid blocking foreground IPC commands.
/// Public so manual enrichment commands can reuse the split-lock pattern (I173).
pub fn write_enrichment_results(
    _state: &AppState,
    input: &EnrichmentInput,
    intel: &IntelligenceJson,
) -> Result<(), String> {
    // Preserve user-edited fields from existing intelligence (I261)
    let mut final_intel = intel.clone();
    if let Ok(existing) = read_intelligence_json(&input.entity_dir) {
        if !existing.user_edits.is_empty() {
            crate::entity_intel::preserve_user_edits(&mut final_intel, &existing);
            log::info!(
                "IntelProcessor: preserved {} user edits for {}",
                existing.user_edits.len(),
                input.entity_id,
            );
        }
    }

    // Write intelligence.json to disk (no DB needed)
    write_intelligence_json(&input.entity_dir, &final_intel)?;

    // Own DB connection for cache update
    let db = crate::db::ActionDb::open().map_err(|e| format!("Failed to open DB: {}", e))?;
    let _ = db.upsert_entity_intelligence(&final_intel);

    // I338: Regenerate person files after intelligence enrichment
    if input.entity_type == "person" {
        if let Ok(Some(person)) = db.get_person(&input.entity_id) {
            let _ = crate::people::write_person_markdown(&input.workspace, &person, &db);
            let _ = crate::people::write_person_dashboard_json(&input.workspace, &person, &db);
        }
    }

    log::debug!(
        "IntelProcessor: wrote intelligence for {} to file + DB",
        input.entity_id,
    );

    Ok(())
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intel_queue_enqueue_dequeue() {
        let queue = IntelligenceQueue::new();

        queue.enqueue(IntelRequest {
            entity_id: "acme".to_string(),
            entity_type: "account".to_string(),
            priority: IntelPriority::ContentChange,
            requested_at: Instant::now(),
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
        });

        // Same entity, higher priority → should upgrade
        queue.enqueue(IntelRequest {
            entity_id: "acme".to_string(),
            entity_type: "account".to_string(),
            priority: IntelPriority::Manual,
            requested_at: Instant::now(),
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
        });

        // Same entity, lower priority → should be ignored
        queue.enqueue(IntelRequest {
            entity_id: "acme".to_string(),
            entity_type: "account".to_string(),
            priority: IntelPriority::ContentChange,
            requested_at: Instant::now(),
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
        });

        queue.enqueue(IntelRequest {
            entity_id: "beta".to_string(),
            entity_type: "account".to_string(),
            priority: IntelPriority::Manual,
            requested_at: Instant::now(),
        });

        queue.enqueue(IntelRequest {
            entity_id: "gamma".to_string(),
            entity_type: "project".to_string(),
            priority: IntelPriority::CalendarChange,
            requested_at: Instant::now(),
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
        });

        // Dequeue it
        let _ = queue.dequeue();

        // Manual request should bypass debounce
        queue.enqueue(IntelRequest {
            entity_id: "acme".to_string(),
            entity_type: "account".to_string(),
            priority: IntelPriority::Manual,
            requested_at: Instant::now(),
        });

        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn test_intel_queue_prune_stale_entries() {
        let queue = IntelligenceQueue::new();

        // Insert a debounce entry manually with an old timestamp
        {
            let mut last = queue.last_enqueued.lock().unwrap();
            // Insert an entry that's "old" by using Instant::now() minus a large duration
            // We can't easily backdate Instant, so test the structure:
            // Insert a fresh entry, prune should NOT remove it
            last.insert("fresh-entity".to_string(), Instant::now());
        }

        queue.prune_stale_entries();

        // Fresh entry should still be there
        let last = queue.last_enqueued.lock().unwrap();
        assert!(last.contains_key("fresh-entity"), "fresh entry should survive pruning");
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
        });

        queue.enqueue(IntelRequest {
            entity_id: "beta-project".to_string(),
            entity_type: "project".to_string(),
            priority: IntelPriority::ContentChange,
            requested_at: Instant::now(),
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
        });
        queue.enqueue(IntelRequest {
            entity_id: "high".to_string(),
            entity_type: "account".to_string(),
            priority: IntelPriority::Manual,
            requested_at: Instant::now(),
        });
        queue.enqueue(IntelRequest {
            entity_id: "mid".to_string(),
            entity_type: "account".to_string(),
            priority: IntelPriority::CalendarChange,
            requested_at: Instant::now(),
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
        });

        let batch = queue.dequeue_batch(3);
        assert_eq!(batch.len(), 1);
        assert_eq!(batch[0].entity_id, "only-one");
        assert!(queue.is_empty());
    }

    // =========================================================================
    // Batch prompt/response parsing tests (I289)
    // =========================================================================

    #[test]
    fn test_parse_batch_response_splits_by_delimiter() {
        use std::path::PathBuf;

        let inputs = vec![
            (
                IntelRequest {
                    entity_id: "acme".to_string(),
                    entity_type: "account".to_string(),
                    priority: IntelPriority::ContentChange,
                    requested_at: Instant::now(),
                },
                EnrichmentInput {
                    workspace: PathBuf::from("/tmp"),
                    entity_dir: PathBuf::from("/tmp/acme"),
                    entity_id: "acme".to_string(),
                    entity_type: "account".to_string(),
                    prompt: String::new(),
                    file_manifest: vec![],
                    file_count: 0,
                },
            ),
            (
                IntelRequest {
                    entity_id: "beta".to_string(),
                    entity_type: "project".to_string(),
                    priority: IntelPriority::ContentChange,
                    requested_at: Instant::now(),
                },
                EnrichmentInput {
                    workspace: PathBuf::from("/tmp"),
                    entity_dir: PathBuf::from("/tmp/beta"),
                    entity_id: "beta".to_string(),
                    entity_type: "project".to_string(),
                    prompt: String::new(),
                    file_manifest: vec![],
                    file_count: 0,
                },
            ),
        ];

        // Simulate a batch response with two valid JSON results
        let response = r#"=== RESULT: acme ===
{
  "executiveAssessment": "Acme is on track.",
  "risks": [],
  "recentWins": [],
  "currentState": {"working": [], "notWorking": [], "unknowns": []},
  "stakeholderInsights": [],
  "valueDelivered": [],
  "nextMeetingReadiness": {"prepItems": []}
}

=== RESULT: beta ===
{
  "executiveAssessment": "Beta project progressing.",
  "risks": [{"text": "Timeline risk", "urgency": "watch"}],
  "recentWins": [],
  "currentState": {"working": ["Good velocity"], "notWorking": [], "unknowns": []},
  "stakeholderInsights": [],
  "valueDelivered": [],
  "nextMeetingReadiness": {"prepItems": []}
}
"#;

        let results = parse_batch_response(response, &inputs);
        assert_eq!(results.len(), 2, "Should parse both entities");
        assert_eq!(results[0].0, "acme");
        assert_eq!(results[1].0, "beta");
        assert_eq!(results[1].1.risks.len(), 1);
    }

    #[test]
    fn test_parse_batch_response_handles_missing_delimiter() {
        use std::path::PathBuf;

        let inputs = vec![(
            IntelRequest {
                entity_id: "missing".to_string(),
                entity_type: "account".to_string(),
                priority: IntelPriority::ContentChange,
                requested_at: Instant::now(),
            },
            EnrichmentInput {
                workspace: PathBuf::from("/tmp"),
                entity_dir: PathBuf::from("/tmp/missing"),
                entity_id: "missing".to_string(),
                entity_type: "account".to_string(),
                prompt: String::new(),
                file_manifest: vec![],
                file_count: 0,
            },
        )];

        // Response with wrong delimiter
        let response = "=== RESULT: wrong-entity ===\n{}\n";

        let results = parse_batch_response(response, &inputs);
        assert!(results.is_empty(), "Should return empty for missing delimiter");
    }

    #[test]
    fn test_build_batch_prompt_contains_delimiters() {
        use std::path::PathBuf;

        let inputs = vec![
            (
                IntelRequest {
                    entity_id: "acme".to_string(),
                    entity_type: "account".to_string(),
                    priority: IntelPriority::ContentChange,
                    requested_at: Instant::now(),
                },
                EnrichmentInput {
                    workspace: PathBuf::from("/tmp"),
                    entity_dir: PathBuf::from("/tmp/acme"),
                    entity_id: "acme".to_string(),
                    entity_type: "account".to_string(),
                    prompt: "Prompt for acme".to_string(),
                    file_manifest: vec![],
                    file_count: 0,
                },
            ),
            (
                IntelRequest {
                    entity_id: "beta".to_string(),
                    entity_type: "project".to_string(),
                    priority: IntelPriority::ContentChange,
                    requested_at: Instant::now(),
                },
                EnrichmentInput {
                    workspace: PathBuf::from("/tmp"),
                    entity_dir: PathBuf::from("/tmp/beta"),
                    entity_id: "beta".to_string(),
                    entity_type: "project".to_string(),
                    prompt: "Prompt for beta".to_string(),
                    file_manifest: vec![],
                    file_count: 0,
                },
            ),
        ];

        let prompt = build_batch_prompt(&inputs);

        assert!(prompt.contains("=== ENTITY: acme ==="));
        assert!(prompt.contains("=== ENTITY: beta ==="));
        assert!(prompt.contains("=== RESULT: acme ==="));
        assert!(prompt.contains("=== RESULT: beta ==="));
        assert!(prompt.contains("Prompt for acme"));
        assert!(prompt.contains("Prompt for beta"));
        assert!(prompt.contains("MULTIPLE entities"));
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
