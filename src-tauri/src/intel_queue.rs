//! Background intelligence enrichment queue (I132).
//!
//! Provides a priority queue for intelligence enrichment requests with
//! deduplication and debounce. A background processor drains the queue
//! and runs enrichment with split DB locking so the UI stays responsive
//! during the 30-120s PTY operation.

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use tauri::{AppHandle, Emitter};

use crate::entity_intel::{
    build_intelligence_context, build_intelligence_prompt, parse_intelligence_response,
    read_intelligence_json, write_intelligence_json, IntelligenceJson, SourceManifestEntry,
};
use crate::pty::{ModelTier, PtyManager};
use crate::state::AppState;
use crate::types::AiModelConfig;

/// Debounce window for content-triggered enrichment requests.
const CONTENT_DEBOUNCE_SECS: u64 = 30;

/// How often the background processor checks for work.
const POLL_INTERVAL_SECS: u64 = 5;

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

    /// Current queue depth (for diagnostics).
    pub fn len(&self) -> usize {
        self.queue.lock().map(|q| q.len()).unwrap_or(0)
    }

    /// Whether the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
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
struct EnrichmentInput {
    workspace: PathBuf,
    entity_dir: PathBuf,
    entity_id: String,
    entity_type: String,
    prompt: String,
    file_manifest: Vec<SourceManifestEntry>,
    file_count: usize,
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

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(POLL_INTERVAL_SECS)).await;

        let request = match state.intel_queue.dequeue() {
            Some(r) => r,
            None => continue,
        };

        log::info!(
            "IntelProcessor: processing {} ({}) priority={:?}",
            request.entity_id,
            request.entity_type,
            request.priority
        );

        // Phase 1: Gather context (brief DB lock)
        let input = match gather_enrichment_input(&state, &request) {
            Ok(input) => input,
            Err(e) => {
                log::warn!(
                    "IntelProcessor: failed to gather context for {}: {}",
                    request.entity_id,
                    e
                );
                continue;
            }
        };

        // Phase 2: Run PTY enrichment (no DB lock held)
        let ai_config = state
            .config
            .read()
            .ok()
            .and_then(|g| g.as_ref().map(|c| c.ai_models.clone()))
            .unwrap_or_default();
        let intel = match run_enrichment(&input, &ai_config) {
            Ok(intel) => intel,
            Err(e) => {
                log::warn!(
                    "IntelProcessor: enrichment failed for {}: {}",
                    request.entity_id,
                    e
                );
                continue;
            }
        };

        // Phase 3: Write results (brief DB lock)
        if let Err(e) = write_enrichment_results(&state, &input, &intel) {
            log::warn!(
                "IntelProcessor: failed to write results for {}: {}",
                request.entity_id,
                e
            );
            continue;
        }

        // Phase 4: Emit event
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

/// Phase 1: Lock DB briefly to gather all context needed for enrichment.
fn gather_enrichment_input(
    state: &AppState,
    request: &IntelRequest,
) -> Result<EnrichmentInput, String> {
    let workspace = {
        let config_guard = state.config.read().map_err(|_| "Config lock poisoned")?;
        let config = config_guard.as_ref().ok_or("No config")?;
        PathBuf::from(&config.workspace_path)
    };

    let db_guard = state.db.lock().map_err(|_| "DB lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

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

    let entity_name = account
        .as_ref()
        .map(|a| a.name.clone())
        .or_else(|| project.as_ref().map(|p| p.name.clone()))
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
        _ => return Err(format!("Unsupported entity type: {}", request.entity_type)),
    };

    // Read prior intelligence
    let prior = read_intelligence_json(&entity_dir).ok();

    // Build context (reads from DB)
    let ctx = build_intelligence_context(
        &workspace,
        db,
        &request.entity_id,
        &request.entity_type,
        account.as_ref(),
        project.as_ref(),
        prior.as_ref(),
    );

    // Build prompt (pure function, but easier to do here while we have the data)
    let prompt = build_intelligence_prompt(&entity_name, &request.entity_type, &ctx);

    let file_manifest = ctx.file_manifest.clone();
    let file_count = file_manifest.len();

    // DB lock drops here when db_guard goes out of scope
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
fn run_enrichment(input: &EnrichmentInput, ai_config: &AiModelConfig) -> Result<IntelligenceJson, String> {
    let pty = PtyManager::for_tier(ModelTier::Synthesis, ai_config).with_timeout(180);
    let output = pty
        .spawn_claude(&input.workspace, &input.prompt)
        .map_err(|e| format!("Claude Code error: {}", e))?;

    parse_intelligence_response(
        &output.stdout,
        &input.entity_id,
        &input.entity_type,
        input.file_count,
        input.file_manifest.clone(),
    )
}

/// Phase 3: Lock DB briefly to write results.
fn write_enrichment_results(
    state: &AppState,
    input: &EnrichmentInput,
    intel: &IntelligenceJson,
) -> Result<(), String> {
    // Write intelligence.json to disk (no DB needed)
    write_intelligence_json(&input.entity_dir, intel)?;

    // Brief DB lock for cache update
    let db_guard = state.db.lock().map_err(|_| "DB lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let _ = db.upsert_entity_intelligence(intel);

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
}
