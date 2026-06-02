//!  / ADR-0100: Glean-first intelligence provider.
//!
//! When Glean is connected, this provider uses the MCP `chat` tool as the
//! primary intelligence computation engine. It produces the same `IntelligenceJson`
//! output as the PTY path, but with data from Salesforce, Zendesk, Gong, Slack,
//! and org directories that local-only enrichment can't access.
//!
//! The provider is called from `intel_queue.rs` when `context_provider.is_remote()`.
//! On failure, the caller falls back to the PTY path transparently.

use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use parking_lot::Mutex;

use tauri::{AppHandle, Emitter, Manager};

use async_trait::async_trait;

use super::dimension_prompts::{self, DIMENSION_NAMES};
use super::io::{IntelligenceJson, SourceManifestEntry};
use super::prompts::{parse_intelligence_response, IntelligenceContext};
use super::provider::{
    Completion, FingerprintMetadata, IntelligenceProvider, ModelName, PromptInput, ProviderError,
    ProviderKind,
};
use crate::context_provider::glean::GleanMcpClient;
use crate::presets::schema::RolePreset;
use crate::pty::ModelTier;

use serde::{Deserialize, Serialize};

/// Glean-first intelligence provider.
///
/// Wraps the GleanMcpClient `chat` tool for structured intelligence queries.
/// Each call produces `IntelligenceJson`-compatible output parseable by
/// `parse_intelligence_response()` — the same parser used for PTY output.
pub struct GleanIntelligenceProvider {
    endpoint: String,
}

const DISCOVERY_CACHE_TTL: Duration = Duration::from_secs(4 * 60 * 60);

#[derive(Debug, Clone)]
struct DiscoveryCacheEntry {
    cached_at: Instant,
    accounts: Vec<DiscoveredAccount>,
}

static DISCOVERY_CACHE: OnceLock<Mutex<HashMap<String, DiscoveryCacheEntry>>> = OnceLock::new();

fn discovery_cache() -> &'static Mutex<HashMap<String, DiscoveryCacheEntry>> {
    DISCOVERY_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

///  documented temperature placeholder for Glean MCP chat.
///
/// Glean's MCP `chat` tool does not expose a temperature flag; the
/// underlying retrieval-augmented generation runs deterministically (the
/// agentic search is what introduces variation, not sampling temperature).
/// `0.0` is the documented placeholder for ADR-0106 §3 fingerprint
/// metadata completeness;  (W3) revisits when canonical
/// fingerprint hashing lands and may model Glean's effective entropy
/// differently.
const GLEAN_CHAT_DEFAULT_TEMPERATURE: f32 = 0.0;

/// Canonical `DiscoveredAccount` type (referenced by ).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredAccount {
    pub name: String,
    pub my_role: Option<String>,
    pub evidence: Option<String>,
    pub source: Option<String>,
    pub domain: Option<String>,
    pub industry: Option<String>,
    pub context_preview: Option<String>,
    #[serde(default)]
    pub already_in_dailyos: bool,
}

/// Wrapper for the discovery response.
#[derive(Debug, Deserialize)]
struct DiscoveryResponse {
    #[serde(default)]
    accounts: Vec<DiscoveredAccount>,
}

impl GleanIntelligenceProvider {
    pub fn new(endpoint: &str) -> Self {
        Self {
            endpoint: endpoint.to_string(),
        }
    }

    /// Full entity intelligence enrichment via Glean — tries parallel first, legacy fallback.
    ///
    /// Returns `IntelligenceJson` on success, or an error string for fallback.
    /// The caller (`intel_queue.rs`) falls back to PTY on any error.
    ///
    /// When `app_handle` is provided, emits progressive enrichment events.
    ///
    /// `is_background` suppresses user-visible degraded/fallback toasts for
    /// ProactiveHygiene / ContentChange / CalendarChange enrichments. These
    /// fire on a schedule with no user intent and the natural 1-in-6
    /// partial-failure rate would otherwise carpet the UI with warnings.
    /// Audit events still log regardless of priority so failures remain
    /// diagnosable from ~/.dailyos/audit.log.
    #[allow(clippy::too_many_arguments)]
    pub async fn enrich_entity(
        &self,
        entity_id: &str,
        entity_type: &str,
        entity_name: &str,
        ctx: &IntelligenceContext,
        relationship: Option<&str>,
        app_handle: Option<&AppHandle>,
        is_background: bool,
        preset: Option<&RolePreset>,
    ) -> Result<IntelligenceJson, String> {
        // Try parallel dimension fan-out first
        match self
            .enrich_entity_parallel(
                entity_id,
                entity_type,
                entity_name,
                ctx,
                relationship,
                app_handle,
                is_background,
                preset,
            )
            .await
        {
            Ok(intel) => Ok(intel),
            Err(e) => {
                log::warn!(
                    "[I574] Parallel Glean enrichment failed for {}, falling back to legacy: {}",
                    entity_name,
                    e
                );
                self.enrich_entity_legacy(
                    entity_id,
                    entity_type,
                    entity_name,
                    ctx,
                    relationship,
                    preset,
                )
                .await
            }
        }
    }

    /// Parallel per-dimension Glean enrichment.
    ///
    /// Spawns 6 concurrent Glean `chat` calls (one per dimension group),
    /// each with a 30s timeout. Merges successful dimensions into a single
    /// `IntelligenceJson`. Falls back to legacy if 0 dimensions succeed.
    ///
    /// Uses `FuturesUnordered` to process dimensions as they complete,
    /// writing progressive updates to DB and emitting events when `app_handle`
    /// is provided.
    #[allow(clippy::too_many_arguments)]
    pub async fn enrich_entity_parallel(
        &self,
        entity_id: &str,
        entity_type: &str,
        entity_name: &str,
        ctx: &IntelligenceContext,
        relationship: Option<&str>,
        app_handle: Option<&AppHandle>,
        is_background: bool,
        preset: Option<&RolePreset>,
    ) -> Result<IntelligenceJson, String> {
        use crate::intel_queue::{EnrichmentComplete, EnrichmentProgress};

        let overall_start = Instant::now();
        let is_incremental = ctx.prior_intelligence.is_some();
        let applicability = dimension_prompts::dimension_applicability(entity_type, relationship);
        let total_dimensions = applicability.applicable.len() as u32;
        let canonical_total_dimensions = DIMENSION_NAMES.len() as u32;
        let skipped_dimensions: Vec<String> = applicability
            .skipped
            .iter()
            .map(|dimension| (*dimension).to_string())
            .collect();

        if total_dimensions == 0 {
            return Err(format!(
                "No applicable Glean dimensions for {} ({})",
                entity_name, entity_type
            ));
        }

        // Build dimension prompts for the dimensions that apply to this entity.
        let prompts: Vec<(String, String)> = DIMENSION_NAMES
            .iter()
            .filter(|dim| applicability.applicable.contains(dim))
            .map(|dim| {
                let prompt = dimension_prompts::build_glean_dimension_prompt(
                    dim,
                    entity_name,
                    entity_type,
                    relationship,
                    ctx,
                    is_incremental,
                    preset,
                );
                (dim.to_string(), prompt)
            })
            .collect();

        log::info!(
            "[I574] Glean parallel enrichment for {} ({}) — {}/{} applicable dimensions, incremental={}",
            entity_name,
            entity_type,
            prompts.len(),
            canonical_total_dimensions,
            is_incremental,
        );

        // Clone values needed by spawned tasks (must be 'static)
        let endpoint = self.endpoint.clone();
        let entity_id_owned = entity_id.to_string();
        let entity_type_owned = entity_type.to_string();
        let entity_name_owned = entity_name.to_string();

        // Use tokio::sync::mpsc channel to receive dimension results as they
        // complete, enabling progressive DB writes and event emission.
        let (tx, mut rx) = tokio::sync::mpsc::channel::<(String, Result<IntelligenceJson, String>)>(
            prompts.len().max(1),
        );

        for (dim_name, prompt) in prompts {
            let ep = endpoint.clone();
            let eid = entity_id_owned.clone();
            let etype = entity_type_owned.clone();
            let ename = entity_name_owned.clone();
            let sender = tx.clone();

            tokio::spawn(async move {
                let start = Instant::now();
                let client = GleanMcpClient::new(&ep);

                // 240s budget: Glean chat is agentic (runs internal search
                // tool-calls before generating the answer). For well-indexed
                // accounts with lots of docs/transcripts the response can
                // take minutes. The original 30s cap fired before the inner
                // reqwest timeout, killing every dimension and falling back
                // silently to PTY — resulting in items tagged with local
                // source enum (transcript|local_file|pty_synthesis) instead
                // of glean_*. 240s matches GLEAN_CHAT_TIMEOUT so slow-but-
                // valid responses complete before either timeout fires.
                let response_result =
                    tokio::time::timeout(Duration::from_secs(240), client.chat(&prompt, None))
                        .await;

                let elapsed_ms = start.elapsed().as_millis();

                let response_text = match response_result {
                    Ok(Ok(text)) => text,
                    Ok(Err(e)) => {
                        log::warn!(
                            "[I574] Glean dimension {} for {} — failed in {}ms: {}",
                            dim_name,
                            ename,
                            elapsed_ms,
                            e
                        );
                        #[allow(
                            clippy::let_underscore_must_use,
                            reason = "intentional best-effort discard; preserves existing non-blocking behavior"
                        )]
                        let _ = sender
                            .send((dim_name, Err(format!("chat failed: {}", e))))
                            .await;
                        return;
                    }
                    Err(_) => {
                        log::warn!(
                            "[I574] Glean dimension {} for {} — timed out after {}ms",
                            dim_name,
                            ename,
                            elapsed_ms
                        );
                        #[allow(
                            clippy::let_underscore_must_use,
                            reason = "intentional best-effort discard; preserves existing non-blocking behavior"
                        )]
                        let _ = sender
                            .send((dim_name, Err("timed out after 240s".to_string())))
                            .await;
                        return;
                    }
                };

                log::info!(
                    "[I574] Glean dimension {} for {} — {}ms, {} chars",
                    dim_name,
                    ename,
                    elapsed_ms,
                    response_text.len()
                );

                // Always capture every dimension's response. Single-file
                // capture lost dimensions 2-6 when the first one happened to
                // succeed; per-dim filenames preserve forensic state across
                // parallel failures.
                {
                    // dos259-exempt: temp debug filename disambiguator; not persisted intelligence state.
                    let ts = chrono::Utc::now().timestamp_millis();
                    let debug_path =
                        std::env::temp_dir().join(format!("dailyos-glean-{}-{}.txt", dim_name, ts));
                    if let Err(e) = std::fs::write(&debug_path, &response_text) {
                        log::warn!(
                            "[I574] Failed to write debug response for {}: {}",
                            dim_name,
                            e
                        );
                    } else {
                        log::info!(
                            "[I574] Glean dimension {} response written to {}",
                            dim_name,
                            debug_path.display()
                        );
                    }
                }

                // Build a minimal manifest for parsing
                let manifest = vec![SourceManifestEntry {
                    filename: format!("glean_chat_{}", dim_name),
                    content_type: Some("glean_synthesis".to_string()),
                    format: Some("json".to_string()),
                    // dos259-grandfathered: source manifest timestamp; migrates to ctx.clock.now() when W2-A lands ServiceContext.
                    modified_at: chrono::Utc::now().to_rfc3339(),
                    selected: true,
                    skip_reason: None,
                }];

                let parse_result =
                    parse_intelligence_response(&response_text, &eid, &etype, 1, manifest);

                let result = match parse_result {
                    Ok(intel) => (dim_name, Ok(intel)),
                    Err(e) => {
                        log::warn!(
                            "[I574] Glean dimension {} for {} — parse failed: {}",
                            dim_name,
                            ename,
                            e
                        );
                        (dim_name, Err(format!("parse failed: {}", e)))
                    }
                };

                #[allow(
                    clippy::let_underscore_must_use,
                    reason = "intentional best-effort discard; preserves existing non-blocking behavior"
                )]
                let _ = sender.send(result).await;
            });
        }

        // Drop our sender so the channel closes after all spawned tasks finish
        drop(tx);

        // Process results progressively as each dimension completes
        let mut combined = IntelligenceJson::default();
        let mut succeeded = 0u32;
        let mut failed_dims = Vec::new();

        while let Some((dim_name, result)) = rx.recv().await {
            match result {
                Ok(partial) => {
                    if let Err(e) =
                        dimension_prompts::merge_dimension_into(&mut combined, &dim_name, &partial)
                    {
                        log::warn!("[I574] Failed to merge dimension {}: {}", dim_name, e);
                        failed_dims.push(dim_name);
                    } else {
                        succeeded += 1;

                        // After commercial_financial merge, extract and upsert products
                        if dim_name == "commercial_financial" && entity_type == "account" {
                            if let Ok(Some(products)) = extract_products_from_response(&partial) {
                                // Best-effort upsert — log but don't fail enrichment if products fail
                                match crate::db::ActionDb::open(std::sync::Arc::new(
                                    crate::db::LocalKeychain::new(),
                                )) {
                                    Ok(db) => {
                                        match upsert_products_to_db(&db, entity_id, products) {
                                            Ok(count) => {
                                                log::info!(
                                                    "[I651] Upserted {} products for {}",
                                                    count,
                                                    entity_id
                                                );
                                            }
                                            Err(e) => {
                                                log::warn!("[I651] Product upsert failed (best-effort): {}", e);
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        log::warn!(
                                            "[I651] Could not open DB for product upsert: {}",
                                            e
                                        );
                                    }
                                }
                            }
                        }

                        // Progressive UI cache write + event emission
                        if let Some(handle) = app_handle {
                            if write_progressive_glean_dimension(
                                entity_id,
                                entity_type,
                                relationship,
                                &combined,
                            ) {
                                #[allow(
                                    clippy::let_underscore_must_use,
                                    reason = "intentional best-effort discard; preserves existing non-blocking behavior"
                                )]
                                let _ = handle.emit(
                                    "enrichment-progress",
                                    EnrichmentProgress {
                                        entity_id: entity_id.to_string(),
                                        entity_type: entity_type.to_string(),
                                        completed: succeeded,
                                        total: total_dimensions,
                                        last_dimension: dim_name,
                                    },
                                );
                            }
                        }
                    }
                }
                Err(_) => {
                    failed_dims.push(dim_name);
                }
            }
        }

        let total_ms = overall_start.elapsed().as_millis();
        log::info!(
            "[I574] Glean parallel: {}/{} applicable dimensions succeeded for {} in {}ms (skipped: {:?}, failed: {:?})",
            succeeded,
            total_dimensions,
            entity_name,
            total_ms,
            skipped_dimensions,
            failed_dims,
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
                    entity_id: entity_id.to_string(),
                    entity_type: entity_type.to_string(),
                    succeeded,
                    failed: failed_dims.len() as u32,
                    failed_dimensions: failed_dims.clone(),
                    total_dimensions: canonical_total_dimensions,
                    applicable_total: total_dimensions,
                    skipped_dimensions: skipped_dimensions.clone(),
                    wall_clock_ms: total_ms as u64,
                },
            );
        }

        // Surface any Glean dimension failures loudly. Without this,
        // timeouts / errors on dimension fan-out fall through silently to
        // legacy enrichment → PTY fallback, leaving users staring at
        // local-sourced items with no signal that Glean couldn't finish.
        //
        // Emits:
        //   - Audit event "glean_enrichment_degraded" (partial) or
        //     "glean_enrichment_all_failed" (full miss) with failed
        //     dimensions + wall-clock ms. grep-able from ~/.dailyos/audit.log.
        //   - Tauri event "enrichment-glean-degraded" so the frontend can
        //     surface a toast/banner when Glean came back partial/empty.
        if !failed_dims.is_empty() {
            if let Some(handle) = app_handle {
                {
                    let state = handle.state::<std::sync::Arc<crate::state::AppState>>();
                    let mut audit = state.audit_log.lock();
                    #[allow(
                        clippy::let_underscore_must_use,
                        reason = "intentional best-effort discard; preserves existing non-blocking behavior"
                    )]
                    let _ = audit.append(
                        "data_access",
                        if succeeded == 0 {
                            "glean_enrichment_all_failed"
                        } else {
                            "glean_enrichment_degraded"
                        },
                        serde_json::json!({
                            "entity_id": entity_id,
                            "entity_type": entity_type,
                            "succeeded": succeeded,
                            "failed": failed_dims.len(),
                            "failed_dimensions": failed_dims.clone(),
                            "required_failed_dimensions": failed_dims.clone(),
                            "optional_failed_dimensions": Vec::<String>::new(),
                            "total_dimensions": canonical_total_dimensions,
                            "applicable_total": total_dimensions,
                            "skipped_dimensions": skipped_dimensions.clone(),
                            "wall_clock_ms": total_ms,
                        }),
                    );
                }
                // Toast only on user-initiated work. Background priorities
                // (ProactiveHygiene, ContentChange, CalendarChange) fire on a
                // schedule and the 1-in-6 partial-failure rate would carpet
                // the UI. Audit event above still logs for diagnostics.
                if !is_background {
                    #[allow(
                        clippy::let_underscore_must_use,
                        reason = "intentional best-effort discard; preserves existing non-blocking behavior"
                    )]
                    let _ = handle.emit(
                        "enrichment-glean-degraded",
                        serde_json::json!({
                            "entity_id": entity_id,
                            "entity_type": entity_type,
                            "succeeded": succeeded,
                            "failed": failed_dims.len(),
                            "failed_dimensions": failed_dims.clone(),
                            "required_failed_dimensions": failed_dims.clone(),
                            "optional_failed_dimensions": Vec::<String>::new(),
                            "total_dimensions": canonical_total_dimensions,
                            "applicable_total": total_dimensions,
                            "skipped_dimensions": skipped_dimensions.clone(),
                            "wall_clock_ms": total_ms,
                            "will_fall_back": succeeded == 0,
                        }),
                    );
                }
            }
        }

        if succeeded == 0 {
            return Err(format!(
                "All {} applicable Glean dimensions failed for {}",
                total_dimensions, entity_name
            ));
        }

        // Set metadata on combined result.
        // dos259-grandfathered: enrichment timestamp; migrates to ctx.clock.now() when W2-A lands ServiceContext.
        combined.enriched_at = chrono::Utc::now().to_rfc3339();
        // Surface the count of indexed source files we had to work with —
        // the manifest below records "glean_chat" as the synthesis channel,
        // but the UI's "About this intelligence" block expects
        // source_file_count to reflect how many real files backed the
        // enrichment (content_index rows passed through the 90-day cutoff).
        // Without this, the About panel shows "1 of 0 total files" because
        // IntelligenceJson::default() zeroes the field and merge_dimension_into
        // doesn't propagate per-dimension counts.
        combined.source_file_count = ctx.file_manifest.len();
        // Build source manifest with a single glean_chat entry
        if combined.source_manifest.is_empty() {
            combined.source_manifest.push(SourceManifestEntry {
                filename: "glean_chat".to_string(),
                content_type: Some("glean_synthesis".to_string()),
                format: Some("json".to_string()),
                // dos259-grandfathered: source manifest timestamp; migrates to ctx.clock.now() when W2-A lands ServiceContext.
                modified_at: chrono::Utc::now().to_rfc3339(),
                selected: true,
                skip_reason: None,
            });
        }

        // Source-aware reconciliation with existing DB intelligence
        let intel = {
            let existing =
                crate::db::ActionDb::open(std::sync::Arc::new(crate::db::LocalKeychain::new()))
                    .ok()
                    .and_then(|db| db.get_entity_intelligence(entity_id).ok().flatten());
            match existing {
                Some(mut existing) => {
                    // Protect stakeholder_insights when user has designated
                    // team members via the Team panel (account_stakeholders with
                    // data_source='user'). Inject synthetic user_edits so
                    // reconcile_enrichment skips the stakeholder array.
                    if entity_type == "account" {
                        let has_user_stakeholders = crate::db::ActionDb::open(std::sync::Arc::new(crate::db::LocalKeychain::new()))
                            .ok()
                            .and_then(|db| {
                                db.conn_ref()
                                    .query_row(
                                        "SELECT COUNT(*) FROM account_stakeholders WHERE account_id = ?1 AND data_source = 'user'",
                                        rusqlite::params![entity_id],
                                        |row| row.get::<_, i64>(0),
                                    )
                                    .ok()
                            })
                            .unwrap_or(0)
                            > 0;
                        if has_user_stakeholders
                            && !existing
                                .user_edits
                                .iter()
                                .any(|e| e.field_path.starts_with("stakeholderInsights"))
                        {
                            existing.user_edits.push(crate::intelligence::io::UserEdit {
                                field_path: "stakeholderInsights".to_string(),
                                // dos259-grandfathered: synthetic user-edit timestamp; migrates to ctx.clock.now() when W2-A lands ServiceContext.
                                edited_at: chrono::Utc::now().to_rfc3339(),
                            });
                        }
                    }
                    reconcile_enrichment(
                        existing,
                        combined,
                        &["glean_crm", "glean_zendesk", "glean_gong", "glean_chat"],
                    )
                }
                None => combined,
            }
        };

        // Stamp source="glean" on product adoption so dual_write_enrichment_products
        // writes products with correct Glean attribution.
        let mut intel = intel;
        stamp_glean_product_source(&mut intel);

        // Path 2c: Extract domains from Glean-enriched intelligence data
        // (company_context.website, stakeholder emails, org_health.website_url)
        extract_domains_for_glean_enrichment(&mut intel);

        log::info!(
            "[I574] Glean parallel enrichment parsed for {} — assessment: {}, risks: {}, wins: {}, stakeholders: {}, domains: {}",
            entity_name,
            intel.executive_assessment.is_some(),
            intel.risks.len(),
            intel.recent_wins.len(),
            intel.stakeholder_insights.len(),
            intel.domains.len(),
        );

        Ok(intel)
    }

    /// Legacy monolithic Glean enrichment (single chat call).
    ///
    /// Kept as fallback for when parallel dimension fan-out fails.
    /// Returns `IntelligenceJson` on success, or an error string for fallback.
    pub async fn enrich_entity_legacy(
        &self,
        entity_id: &str,
        entity_type: &str,
        entity_name: &str,
        ctx: &IntelligenceContext,
        relationship: Option<&str>,
        preset: Option<&RolePreset>,
    ) -> Result<IntelligenceJson, String> {
        let is_incremental = ctx.prior_intelligence.is_some();

        // Build the structured prompt requesting JSON
        let prompt = super::glean_prompts::build_glean_enrichment_prompt(
            entity_name,
            entity_type,
            relationship,
            ctx,
            is_incremental,
            preset,
        );

        log::info!(
            "[I535] Glean enrichment for {} ({}) — prompt length: {} chars",
            entity_name,
            entity_type,
            prompt.len()
        );

        // Call Glean chat
        let client = GleanMcpClient::new(&self.endpoint);
        let response_text = client
            .chat(&prompt, None)
            .await
            .map_err(|e| format!("Glean chat failed for {}: {}", entity_name, e))?;

        log::info!(
            "[I535] Glean response for {} — {} chars",
            entity_name,
            response_text.len()
        );
        // Debug: dump full Glean response to temp file for inspection
        let debug_path = std::env::temp_dir().join("dailyos-glean-response.txt");
        if let Err(e) = std::fs::write(&debug_path, &response_text) {
            log::warn!("[I535] Failed to write debug response: {}", e);
        } else {
            log::warn!("[I535] Glean response written to {}", debug_path.display());
        }

        // Parse using the same parser as PTY output
        let manifest = ctx
            .file_manifest
            .iter()
            .cloned()
            .chain(std::iter::once(SourceManifestEntry {
                filename: "glean_chat".to_string(),
                content_type: Some("glean_synthesis".to_string()),
                format: Some("json".to_string()),
                // dos259-grandfathered: source manifest timestamp; migrates to ctx.clock.now() when W2-A lands ServiceContext.
                modified_at: chrono::Utc::now().to_rfc3339(),
                selected: true,
                skip_reason: None,
            }))
            .collect::<Vec<_>>();

        let source_count = manifest.len();

        let glean_intel = parse_intelligence_response(
            &response_text,
            entity_id,
            entity_type,
            source_count,
            manifest,
        )?;

        // Source-aware reconciliation with existing intelligence.
        // Preserves user corrections, transcript items, and dismissed tombstones.
        let intel = {
            let existing =
                crate::db::ActionDb::open(std::sync::Arc::new(crate::db::LocalKeychain::new()))
                    .ok()
                    .and_then(|db| db.get_entity_intelligence(entity_id).ok().flatten());
            match existing {
                Some(existing) => reconcile_enrichment(
                    existing,
                    glean_intel,
                    &["glean_crm", "glean_zendesk", "glean_gong", "glean_chat"],
                ),
                None => glean_intel,
            }
        };

        // Stamp source="glean" on product adoption so dual_write_enrichment_products
        // writes products with correct Glean attribution.
        let mut intel = intel;
        stamp_glean_product_source(&mut intel);

        // Path 2c: Extract domains from Glean-enriched intelligence data
        // (company_context.website, stakeholder emails, org_health.website_url)
        extract_domains_for_glean_enrichment(&mut intel);

        log::info!(
            "[I535] Glean enrichment parsed for {} — assessment: {}, risks: {}, wins: {}, stakeholders: {}, domains: {}",
            entity_name,
            intel.executive_assessment.is_some(),
            intel.risks.len(),
            intel.recent_wins.len(),
            intel.stakeholder_insights.len(),
            intel.domains.len(),
        );

        Ok(intel)
    }

    /// Discover accounts associated with a user's email.
    ///
    /// Searches Salesforce, Gong, Zendesk for account associations.
    /// Returns a list of accounts with role attribution and evidence.
    pub async fn discover_accounts(
        &self,
        user_email: &str,
        user_name: &str,
    ) -> Result<Vec<DiscoveredAccount>, String> {
        let cache_key = format!("{}::{}", self.endpoint, user_email.to_lowercase());
        {
            let cache = discovery_cache().lock();
            if let Some(entry) = cache.get(&cache_key) {
                if entry.cached_at.elapsed() < DISCOVERY_CACHE_TTL {
                    log::info!(
                        "[I494] Using cached Glean discovery results for {} ({} accounts)",
                        user_email,
                        entry.accounts.len()
                    );
                    return Ok(entry.accounts.clone());
                }
            }
        }

        let prompt = super::glean_prompts::build_account_discovery_prompt(user_email, user_name);

        let client = GleanMcpClient::new(&self.endpoint);
        let response_text = client
            .chat(&prompt, None)
            .await
            .map_err(|e| format!("Glean account discovery failed: {}", e))?;

        // Extract JSON from response
        let json_text = extract_json_object(&response_text)
            .ok_or_else(|| "Glean discovery response contained no JSON object".to_string())?;

        let discovery: DiscoveryResponse = serde_json::from_str(json_text)
            .map_err(|e| format!("Failed to parse Glean discovery response: {}", e))?;

        log::info!(
            "[I535] Glean discovered {} accounts for {}",
            discovery.accounts.len(),
            user_email
        );

        {
            let mut cache = discovery_cache().lock();
            cache.insert(
                cache_key,
                DiscoveryCacheEntry {
                    cached_at: Instant::now(),
                    accounts: discovery.accounts.clone(),
                },
            );
        }

        Ok(discovery.accounts)
    }

    /// Get the endpoint this provider is configured for.
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Supplemental leading-signals enrichment for the Health & Outlook tab.
    ///
    /// Runs a second Glean `chat` call with the leading-signals prompt, parses the
    /// 7-bucket JSON response, and returns a normalized `HealthOutlookSignals`.
    /// Called by `intel_queue.rs` after the main per-dimension enrichment completes
    /// successfully. Returns `Err` on chat failure, timeout, or unparseable response
    /// — the caller swallows the error (silent fallback per ADR-0100).
    pub async fn enrich_leading_signals(
        &self,
        entity_name: &str,
    ) -> Result<super::glean_leading_signals::HealthOutlookSignals, String> {
        self.enrich_leading_signals_with_disambiguators(entity_name, None)
            .await
    }

    /// Leading-signals enrichment with optional structured
    /// disambiguators. Callers that have an `IntelligenceContext` handy
    /// should prefer this variant so Glean retrieval is biased correctly.
    pub async fn enrich_leading_signals_with_disambiguators(
        &self,
        entity_name: &str,
        disambiguators: Option<&super::prompts::EntityDisambiguators>,
    ) -> Result<super::glean_leading_signals::HealthOutlookSignals, String> {
        let prompt =
            super::glean_leading_signals::build_leading_signals_prompt(entity_name, disambiguators);
        let client = GleanMcpClient::new(&self.endpoint);

        let response_text =
            tokio::time::timeout(Duration::from_secs(240), client.chat(&prompt, None))
                .await
                .map_err(|_| format!("Glean leading-signals chat timed out for {}", entity_name))?
                .map_err(|e| {
                    format!(
                        "Glean leading-signals chat failed for {}: {}",
                        entity_name, e
                    )
                })?;

        log::info!(
            "[DOS-15] Glean leading-signals response for {} — {} chars",
            entity_name,
            response_text.len()
        );

        super::glean_leading_signals::parse_leading_signals(&response_text)
    }

    /// Peer-cohort renewal benchmark via a dedicated Glean chat pass.
    ///
    /// Builds the validated peer-benchmark prompt from `account_name`,
    /// `industry_descriptor`, and `size_descriptor`, calls Glean with citation
    /// metadata (so the cell can render the "Drawn from N source(s)" footer),
    /// and parses the response into a `PeerBenchmark`.
    ///
    /// The parser is a lowercase prefix match against the response text:
    /// - `above peers` → `PeerBenchmarkBand::Above`
    /// - `in line peers` / `in-line peers` → `PeerBenchmarkBand::At`
    /// - `below peers` → `PeerBenchmarkBand::Below`
    /// - `no comparable` (or anything else) → `PeerBenchmarkBand::Unknown`,
    ///   which the frontend cell renders as `null` (collapses the column).
    ///
    /// On unknown band, returns `Err` so the caller can skip the write
    /// (no point persisting a record the UI will hide). On chat failure
    /// or timeout, also returns `Err` — the caller swallows it as a
    /// silent fallback (parity with `enrich_leading_signals_*`).
    pub async fn enrich_peer_benchmark(
        &self,
        account_name: &str,
    ) -> Result<super::io::PeerBenchmark, String> {
        let prompt = format!(
            "For the customer **{account_name}**, what is a reasonable peer benchmark for \
their renewal trajectory and account health on WordPress VIP? Use whatever you know about \
their tier, ARR, package, industry, and size to identify comparable peer customers.\n\n\
Consider:\n\n\
1. Renewal rates and retention norms for VIP customers at similar tier and ACV\n\
2. Typical engagement indicators (meeting cadence, ticket volume, expansion patterns) \
for healthy renewals at this tier\n\
3. Whether there are recent or historical signals from comparable {account_name}-like \
accounts that suggest typical health patterns\n\n\
Return a short assessment in the form: \"**[Above / In line / Below] peers**\" followed \
by **one sentence** explaining the comparison with specific numbers where possible. \
Cite sources."
        );

        let client = GleanMcpClient::new(&self.endpoint);

        let response = tokio::time::timeout(
            Duration::from_secs(240),
            client.chat_with_citations(&prompt, None),
        )
        .await
        .map_err(|_| format!("Glean peer-benchmark chat timed out for {}", account_name))?
        .map_err(|e| {
            format!(
                "Glean peer-benchmark chat failed for {}: {}",
                account_name, e
            )
        })?;

        log::info!(
            "[DOS-204] Glean peer-benchmark response for {} — {} chars, {} sources",
            account_name,
            response.text.len(),
            response.source_count
        );

        let (band, narrative) = parse_peer_benchmark_response(&response.text)?;

        Ok(super::io::PeerBenchmark {
            band,
            narrative,
            source_count: response.source_count,
        })
    }
}

///  `IntelligenceProvider` trait impl over `GleanIntelligenceProvider`.
///
/// `complete()` invokes the Glean MCP `chat` tool and returns the raw response
/// text. The existing domain-specific helpers (`enrich_entity*`,
/// `enrich_leading_signals*`, `discover_accounts`, `enrich_peer_benchmark`) stay
/// as Glean-specific module-local methods, not provider-trait surface — they
/// embed Glean's MCP-only retrieval semantics that other providers cannot mirror.
///
/// The 240s timeout matches `GLEAN_CHAT_TIMEOUT` and the parallel-dimension
/// timeout in `enrich_entity_parallel` to keep behavior parity.
#[async_trait]
impl IntelligenceProvider for GleanIntelligenceProvider {
    async fn complete(
        &self,
        prompt: PromptInput,
        tier: ModelTier,
    ) -> Result<Completion, ProviderError> {
        let client = GleanMcpClient::new(&self.endpoint);
        let chat_result =
            tokio::time::timeout(Duration::from_secs(240), client.chat(&prompt.text, None)).await;
        let text = match chat_result {
            Ok(Ok(text)) => text,
            Ok(Err(e)) => {
                let msg = format!("{e}");
                if msg.to_lowercase().contains("auth")
                    || msg.to_lowercase().contains("unauthorized")
                {
                    return Err(ProviderError::Permanent(msg));
                }
                return Err(ProviderError::Transient(msg));
            }
            Err(_) => {
                return Err(ProviderError::Timeout { seconds: 240 });
            }
        };
        Ok(Completion {
            text,
            fingerprint_metadata: FingerprintMetadata {
                provider: ProviderKind::Other("glean"),
                model: self.current_model(tier),
                temperature: GLEAN_CHAT_DEFAULT_TEMPERATURE,
                top_p: None,
                seed: None,
                tokens_input: None,
                tokens_output: None,
                provider_completion_id: None,
            },
        })
    }

    fn provider_kind(&self) -> ProviderKind {
        ProviderKind::Other("glean")
    }

    fn current_model(&self, _tier: ModelTier) -> ModelName {
        // Glean does not expose its underlying model selection; report the
        // chat tool name as the canonical model identifier.
        ModelName::new("glean-chat")
    }
}

/// Parse a Glean peer-benchmark response into `(band, narrative)`.
///
/// Recognises the four explicit band prefixes (above / in line / in-line /
/// below) plus the explicit "no comparable" opt-out. Anything that doesn't
/// match returns `Err` so the caller can skip persisting the record — the
/// frontend cell hides on `Unknown`/missing, so writing a stub helps no one.
///
/// Strips the band prefix and any leading separator (em-dash, en-dash, hyphen,
/// colon) plus surrounding whitespace, then trims surrounding markdown
/// emphasis (`**`) the model often emits around the band label.
fn parse_peer_benchmark_response(
    raw: &str,
) -> Result<(super::io::PeerBenchmarkBand, String), String> {
    // Normalise: trim leading whitespace and any leading markdown emphasis
    // so "**In line peers** – ..." matches the same as "In line peers – ...".
    let trimmed = raw.trim_start().trim_start_matches('*');
    let lower = trimmed.to_lowercase();

    let (band, prefix_len) = if lower.starts_with("above peers") {
        (super::io::PeerBenchmarkBand::Above, "above peers".len())
    } else if lower.starts_with("in line peers") {
        (super::io::PeerBenchmarkBand::At, "in line peers".len())
    } else if lower.starts_with("in-line peers") {
        (super::io::PeerBenchmarkBand::At, "in-line peers".len())
    } else if lower.starts_with("below peers") {
        (super::io::PeerBenchmarkBand::Below, "below peers".len())
    } else if lower.starts_with("no comparable") {
        return Err("Glean reported no comparable peers".to_string());
    } else {
        return Err(format!(
            "Peer-benchmark response did not start with a recognised band: {}",
            trimmed.chars().take(80).collect::<String>()
        ));
    };

    // Strip the band prefix, trailing emphasis markers, and the
    // band/narrative separator (em-dash, en-dash, hyphen, or colon).
    let after_band = &trimmed[prefix_len..];
    let narrative = after_band
        .trim_start()
        .trim_start_matches('*')
        .trim_start()
        .trim_start_matches(|c: char| {
            c == '\u{2014}' // em-dash
                || c == '\u{2013}' // en-dash
                || c == '-'
                || c == ':'
        })
        .trim()
        .to_string();

    if narrative.is_empty() {
        return Err("Peer-benchmark response had a band but no narrative".to_string());
    }

    Ok((band, narrative))
}

/// Path 2c: Placeholder for domain extraction from Glean enrichment.
///
/// IntelligenceJson now has an optional `domains` field that can be populated by:
/// 1. Glean extracts domain from company_context (future: if Glean adds website field)
/// 2. Email classification pipeline (extract from meeting attendees)
/// 3. Enrichment future phases (if Glean company data includes domains)
///
/// For now, this function is a no-op. When Glean responses include domain data,
/// this is where extraction logic will be added.
fn extract_domains_for_glean_enrichment(_intel: &mut IntelligenceJson) {
    // TODO: When Glean responses include firmographic domain data,
    // extract and populate _intel.domains here.
    // This hook is in place for easy future enhancement.
}

/// Write progressive dimension state to the UI cache during Glean parallel enrichment.
///
/// Similar to `write_progressive_dimension` in `intel_queue.rs` but for the Glean path.
/// Non-fatal on error — the final merge+write after all dimensions is authoritative.
fn write_progressive_glean_dimension(
    entity_id: &str,
    entity_type: &str,
    relationship: Option<&str>,
    combined: &IntelligenceJson,
) -> bool {
    let db = match crate::db::ActionDb::open(std::sync::Arc::new(crate::db::LocalKeychain::new())) {
        Ok(db) => db,
        Err(e) => {
            log::warn!(
                "[I575] Glean progressive write: failed to open DB for {}: {}",
                entity_id,
                e
            );
            return false;
        }
    };

    let merged = crate::intel_queue::prepare_progressive_dimension_snapshot(
        &db,
        entity_id,
        entity_type,
        relationship,
        combined,
    );

    let clock = crate::services::context::SystemClock;
    let rng = crate::services::context::SystemRng;
    let ext = crate::services::context::ExternalClients::default();
    let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &ext);
    if let Err(e) =
        crate::services::intelligence::upsert_progressive_assessment_snapshot(&ctx, &db, &merged)
    {
        log::warn!(
            "[I575] Glean progressive write failed for {}: {}",
            entity_id,
            e
        );
        false
    } else {
        log::debug!("[I575] Glean progressive write succeeded for {}", entity_id,);
        true
    }
}

/// Compatibility wrapper for shared Glean finalization.
///
/// Glean retrieval and parsing stay in this provider; substrate writes belong
/// to `services::glean_finalization`.
pub fn emit_glean_signals(
    db: &crate::db::ActionDb,
    engine: &crate::signals::propagation::PropagationEngine,
    entity_type: &str,
    entity_id: &str,
    intel: &IntelligenceJson,
    preset: Option<&RolePreset>,
) {
    let clock = crate::services::context::SystemClock;
    let rng = crate::services::context::SystemRng;
    let ext = crate::services::context::ExternalClients::default();
    let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &ext);
    if let Err(error) = crate::services::glean_finalization::finalize_glean_enrichment(
        &ctx,
        db,
        engine,
        crate::services::glean_finalization::GleanFinalizationInput {
            entity_type,
            entity_id,
            intel,
            preset,
        },
    ) {
        log::warn!(
            "glean finalization wrapper failed for entity_type={} entity_id={}: {}",
            entity_type,
            entity_id,
            error
        );
    }
}

/// Source-aware reconciliation of enrichment output with existing intelligence.
///
/// Rules:
/// 1. User corrections (source "user_correction") — ALWAYS preserved
/// 2. Items from non-refreshed sources survive (transcript risk persists across Glean refresh)
/// 3. When new + existing have similar items from different sources, keep both
/// 4. Conflicts flagged with discrepancy: true
/// 5. Dismissed items (tombstones) prevent re-creation
pub fn reconcile_enrichment(
    existing: IntelligenceJson,
    new_output: IntelligenceJson,
    refreshed_sources: &[&str],
) -> IntelligenceJson {
    let mut result = existing.clone();
    let dismissed = &existing.dismissed_items;
    let refreshed_fields = new_output.refreshed_fields.clone();

    // Helper: check if a vec field (or any of its items' sub-fields) has user edits.
    // When a user edits an individual field like stakeholderInsights[0].engagement,
    // the item's item_source stays as pty_synthesis — reconcile_vec_items would
    // replace it. Skipping reconciliation for user-edited vec fields ensures
    // preserve_user_edits (called later) operates on the correct base data.
    let has_user_edits = |field_name: &str| -> bool {
        existing.user_edits.iter().any(|e| {
            e.field_path == field_name || e.field_path.starts_with(&format!("{}[", field_name))
        })
    };
    let field_refreshed =
        |field_name: &str| -> bool { refreshed_fields.iter().any(|field| field == field_name) };

    // --- Vec fields: source-aware item reconciliation ---
    // Skip reconciliation for fields with user edits — preserve_user_edits
    // handles those after this function returns.

    // Non-destructive-empty guard (DOS Globex regression): when a dimension
    // times out, its fields arrive empty. Reconciling an empty new_items array
    // against existing pty_synthesis items would wipe them all. Reconcile only
    // when the new output actually has data, or when existing is also empty
    // (nothing to preserve).
    if !has_user_edits("risks")
        && (field_refreshed("risks") || !new_output.risks.is_empty() || existing.risks.is_empty())
    {
        result.risks = reconcile_vec_items(
            &existing.risks,
            &new_output.risks,
            refreshed_sources,
            dismissed,
            "risks",
            |r| &r.text,
        );
    }

    if !has_user_edits("recentWins")
        && (field_refreshed("recentWins")
            || !new_output.recent_wins.is_empty()
            || existing.recent_wins.is_empty())
    {
        result.recent_wins = reconcile_vec_items(
            &existing.recent_wins,
            &new_output.recent_wins,
            refreshed_sources,
            dismissed,
            "recentWins",
            |w| &w.text,
        );
    }

    // stakeholder_insights is now write-only context in intelligence.json.
    // Real stakeholder protection is structural (data_source columns on account_stakeholders).
    // Always take the fresh AI output — intel_queue::compose_enrichment_intelligence routes
    // insights to DB columns or stakeholder_suggestions table.
    result.stakeholder_insights = new_output.stakeholder_insights;

    if !has_user_edits("valueDelivered")
        && (field_refreshed("valueDelivered")
            || !new_output.value_delivered.is_empty()
            || existing.value_delivered.is_empty())
    {
        result.value_delivered = reconcile_vec_items(
            &existing.value_delivered,
            &new_output.value_delivered,
            refreshed_sources,
            dismissed,
            "valueDelivered",
            |v| &v.statement,
        );
    }

    if !has_user_edits("competitiveContext")
        && (field_refreshed("competitiveContext")
            || !new_output.competitive_context.is_empty()
            || existing.competitive_context.is_empty())
    {
        result.competitive_context = reconcile_vec_items(
            &existing.competitive_context,
            &new_output.competitive_context,
            refreshed_sources,
            dismissed,
            "competitiveContext",
            |c| &c.competitor,
        );
    }

    // MarketContext reconciliation — same non-destructive-empty guard as
    // the other vec fields. Prevents a sparse enrichment from wiping
    // prior regulatory/market items that user corrections or earlier
    // enrichments accumulated.
    if !has_user_edits("marketContext")
        && (field_refreshed("marketContext")
            || !new_output.market_context.is_empty()
            || existing.market_context.is_empty())
    {
        result.market_context = reconcile_vec_items(
            &existing.market_context,
            &new_output.market_context,
            refreshed_sources,
            dismissed,
            "marketContext",
            |m| &m.title,
        );
    }

    if !has_user_edits("organizationalChanges")
        && (field_refreshed("organizationalChanges")
            || !new_output.organizational_changes.is_empty()
            || existing.organizational_changes.is_empty())
    {
        result.organizational_changes = reconcile_vec_items(
            &existing.organizational_changes,
            &new_output.organizational_changes,
            refreshed_sources,
            dismissed,
            "organizationalChanges",
            |o| &o.person,
        );
    }

    if !has_user_edits("expansionSignals")
        && (field_refreshed("expansionSignals")
            || !new_output.expansion_signals.is_empty()
            || existing.expansion_signals.is_empty())
    {
        result.expansion_signals = reconcile_vec_items(
            &existing.expansion_signals,
            &new_output.expansion_signals,
            refreshed_sources,
            dismissed,
            "expansionSignals",
            |e| &e.opportunity,
        );
    }

    // open_commitments is Option<Vec<...>>
    if !has_user_edits("openCommitments") {
        if let (Some(existing_oc), Some(new_oc)) =
            (&existing.open_commitments, &new_output.open_commitments)
        {
            // Non-destructive-empty: if new dimension returned empty, keep existing.
            if field_refreshed("openCommitments") || !new_oc.is_empty() || existing_oc.is_empty() {
                let reconciled = reconcile_vec_items(
                    existing_oc,
                    new_oc,
                    refreshed_sources,
                    dismissed,
                    "openCommitments",
                    |c| &c.description,
                );
                result.open_commitments = Some(reconciled);
            }
        } else if new_output.open_commitments.is_some() {
            result.open_commitments = new_output.open_commitments;
        }
    }

    // --- Option fields: fresh data wins, except user-edited fields ---
    macro_rules! reconcile_option {
        ($field:ident, $field_name:expr) => {
            if new_output.$field.is_some() {
                // Check if user edited this field — if so, keep existing
                if existing
                    .user_edits
                    .iter()
                    .any(|e| e.field_path == $field_name)
                {
                    // User-edited — keep existing
                } else {
                    result.$field = new_output.$field;
                }
            }
        };
    }

    reconcile_option!(executive_assessment, "executiveAssessment");
    reconcile_option!(current_state, "currentState");
    reconcile_option!(next_meeting_readiness, "nextMeetingReadiness");
    reconcile_option!(company_context, "companyContext");
    reconcile_option!(network, "network");
    reconcile_option!(health, "health");
    reconcile_option!(org_health, "orgHealth");
    reconcile_option!(success_metrics, "successMetrics");
    reconcile_option!(relationship_depth, "relationshipDepth");
    reconcile_option!(coverage_assessment, "coverageAssessment");
    reconcile_option!(meeting_cadence, "meetingCadence");
    reconcile_option!(email_responsiveness, "emailResponsiveness");
    reconcile_option!(contract_context, "contractContext");
    reconcile_option!(agreement_outlook, "agreementOutlook");
    reconcile_option!(support_health, "supportHealth");
    reconcile_option!(product_adoption, "productAdoption");
    reconcile_option!(nps_csat, "npsCsat");
    reconcile_option!(source_attribution, "sourceAttribution");
    reconcile_option!(success_plan_signals, "successPlanSignals");

    // Non-source-attributed vecs: strategic_priorities, internal_team, blockers, gong_call_summaries
    // These already use an "only-overwrite-if-non-empty" guard via the is_empty checks below,
    // which preserves existing values when the dimension returns nothing.
    if field_refreshed("strategicPriorities") || !new_output.strategic_priorities.is_empty() {
        result.strategic_priorities = new_output.strategic_priorities;
    }
    if field_refreshed("internalTeam") || !new_output.internal_team.is_empty() {
        result.internal_team =
            reconcile_internal_team(&existing.internal_team, &new_output.internal_team);
    }
    if field_refreshed("blockers") || !new_output.blockers.is_empty() {
        result.blockers = new_output.blockers;
    }
    if field_refreshed("gongCallSummaries") || !new_output.gong_call_summaries.is_empty() {
        result.gong_call_summaries = new_output.gong_call_summaries;
    }

    // Carry forward user_edits and dismissed_items from existing
    result.user_edits = existing.user_edits;
    result.dismissed_items = existing.dismissed_items;
    result.refreshed_fields = refreshed_fields;

    // Update metadata
    result.enriched_at = new_output.enriched_at;
    if !new_output.source_manifest.is_empty() {
        for entry in new_output.source_manifest {
            if !result
                .source_manifest
                .iter()
                .any(|e| e.filename == entry.filename)
            {
                result.source_manifest.push(entry);
            }
        }
    }

    result
}

/// Ensure product adoption from Glean enrichment carries source="glean".
///
/// The Glean response may or may not include a source field in productAdoption.
/// This stamps it explicitly so `dual_write_enrichment_products` writes products
/// with Glean attribution instead of the default "ai_inference".
fn stamp_glean_product_source(intel: &mut IntelligenceJson) {
    if let Some(ref mut adoption) = intel.product_adoption {
        if adoption.source.is_none()
            || !adoption
                .source
                .as_ref()
                .is_some_and(|s| s.contains("glean"))
        {
            adoption.source = Some("glean".to_string());
        }
    }
}

fn reconcile_internal_team(
    existing: &[super::io::InternalTeamMember],
    new_items: &[super::io::InternalTeamMember],
) -> Vec<super::io::InternalTeamMember> {
    let mut merged = existing
        .iter()
        .filter(|member| member.source.as_deref() == Some("user"))
        .cloned()
        .collect::<Vec<_>>();

    for item in new_items {
        let duplicate = merged.iter().any(|existing_item| {
            existing_item.name.eq_ignore_ascii_case(&item.name)
                && existing_item.role.eq_ignore_ascii_case(&item.role)
        });
        if !duplicate {
            merged.push(item.clone());
        }
    }

    merged
}

/// Reconcile a Vec of source-attributed items.
///
/// 1. Keep all existing items whose source is NOT in `refreshed_sources`
/// 2. Always keep items with source == "user_correction"
/// 3. Add all new items
/// 4. Drop new items that match dismissed tombstones
fn reconcile_vec_items<T: super::io::HasSource + Clone>(
    existing_items: &[T],
    new_items: &[T],
    refreshed_sources: &[&str],
    dismissed: &[super::io::DismissedItem],
    field_name: &str,
    get_text: fn(&T) -> &str,
) -> Vec<T> {
    let mut result: Vec<T> = Vec::new();

    // 1. Keep existing items from non-refreshed sources + all user corrections
    for item in existing_items {
        let source = item
            .item_source()
            .map(|s| s.source.as_str())
            .unwrap_or("pty_synthesis");

        if source == "user_correction" {
            // Sacred — always keep
            result.push(item.clone());
        } else if !refreshed_sources.contains(&source) {
            // Not refreshed — survive unconditionally
            result.push(item.clone());
        }
        // Items from refreshed sources are replaced by new_items
    }

    // 2. Add new items, filtering against dismissed tombstones and existing duplicates
    // Only enforce dismissals from the last 90 days
    // dos259-grandfathered: 90-day dismissal cutoff; migrates to ctx.clock.now() when W2-A lands ServiceContext.
    let cutoff_90d = (chrono::Utc::now() - chrono::Duration::days(90)).to_rfc3339();
    for item in new_items {
        let item_text = get_text(item).to_lowercase();

        let is_dismissed = dismissed.iter().any(|d| {
            d.field == field_name
                && d.dismissed_at > cutoff_90d
                && item_text.contains(&d.content.to_lowercase())
        });

        // Dedup: skip if an item with the same text already exists in result
        let is_duplicate = result
            .iter()
            .any(|existing| get_text(existing).to_lowercase() == item_text);

        if !is_dismissed && !is_duplicate {
            result.push(item.clone());
        }
    }

    result
}

/// Extract the first balanced JSON object from a text response.
///
/// Uses brace counting to find the correct closing `}` that matches
/// the first `{`, handling nested objects correctly.
pub fn extract_json_object(text: &str) -> Option<&str> {
    let start = text.find('{')?;
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape_next = false;

    for (i, ch) in text[start..].char_indices() {
        if escape_next {
            escape_next = false;
            continue;
        }
        match ch {
            '\\' if in_string => escape_next = true,
            '"' => in_string = !in_string,
            '{' if !in_string => depth += 1,
            '}' if !in_string => {
                depth -= 1;
                if depth == 0 {
                    return Some(&text[start..=start + i]);
                }
            }
            _ => {}
        }
    }
    None
}

// =============================================================================
// Product Classification Extraction & Upsert
// =============================================================================

/// Extract product classification from a Financial dimension response.
///
/// Parses the `productClassification.products` array from the Glean response
/// and returns structured product data ready for database upsert.
/// Returns None if the section is missing or empty (best-effort).
pub fn extract_products_from_response(
    response: &IntelligenceJson,
) -> Result<Option<Vec<(String, Option<String>, Option<f64>, Option<String>)>>, String> {
    match &response.product_classification {
        None => Ok(None),
        Some(classification) if classification.products.is_empty() => Ok(None),
        Some(classification) => {
            let mut products = Vec::new();
            for product in &classification.products {
                if let Some(ref product_type) = product.type_ {
                    // Parse type_ field which comes from Glean as "type"
                    products.push((
                        product_type.clone(),
                        product.tier.clone(),
                        product.arr,
                        product.billing_terms.clone(),
                    ));
                }
            }
            if products.is_empty() {
                Ok(None)
            } else {
                Ok(Some(products))
            }
        }
    }
}

#[allow(clippy::items_after_test_module)]
#[cfg(test)]
mod provider_trait_tests {
    use super::super::provider::{IntelligenceProvider, ProviderKind};
    use super::*;

    fn fixture() -> GleanIntelligenceProvider {
        GleanIntelligenceProvider::new("https://example.invalid/glean")
    }

    /// `glean_provider_fixture_returns_expected_fingerprint_metadata`
    /// (per plan §9): fingerprint metadata fields populated at
    /// `complete()` time are deterministic for a given (provider, tier).
    /// We assert the metadata shape via `current_model()` + `provider_kind()`
    /// rather than invoking Glean MCP (which requires a live endpoint);
    /// the byte-identical parity test in §9 covers chat-response shape.
    #[test]
    fn glean_provider_fixture_returns_expected_fingerprint_metadata() {
        let p = fixture();
        assert_eq!(p.provider_kind(), ProviderKind::Other("glean"));
        assert_eq!(p.current_model(ModelTier::Synthesis).as_str(), "glean-chat");
        assert_eq!(
            p.current_model(ModelTier::Extraction).as_str(),
            "glean-chat"
        );
    }
}

#[cfg(test)]
mod reconciliation_tests {
    use super::*;
    use crate::intelligence::io::{IntelRisk, IntelligenceJson, ItemSource};

    #[test]
    fn explicit_empty_refreshed_risks_remove_existing_glean_items() {
        let existing = IntelligenceJson {
            risks: vec![
                IntelRisk {
                    text: "Stale Glean risk".to_string(),
                    urgency: "watch".to_string(),
                    item_source: Some(ItemSource {
                        source: "glean_crm".to_string(),
                        confidence: 0.9,
                        sourced_at: "2026-05-20T00:00:00Z".to_string(),
                        reference: Some("CRM fixture".to_string()),
                    }),
                    ..Default::default()
                },
                IntelRisk {
                    text: "Local transcript risk".to_string(),
                    urgency: "watch".to_string(),
                    item_source: Some(ItemSource {
                        source: "pty_synthesis".to_string(),
                        confidence: 0.5,
                        sourced_at: "2026-05-19T00:00:00Z".to_string(),
                        reference: Some("local fixture".to_string()),
                    }),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        let new_output = IntelligenceJson {
            refreshed_fields: vec!["risks".to_string()],
            ..Default::default()
        };

        let result = reconcile_enrichment(existing, new_output, &["glean_crm"]);

        assert_eq!(result.risks.len(), 1);
        assert_eq!(result.risks[0].text, "Local transcript risk");
        assert!(result.refreshed_fields.contains(&"risks".to_string()));
    }
}

/// Upsert extracted products to the database.
///
/// For each (account_id, product_type, data_source) tuple, inserts or updates
/// the row with tier, arr, billing_terms, and last_verified_at.
/// Returns the count of products upserted on success.
/// Logs warnings on database errors but returns them for best-effort handling.
pub fn upsert_products_to_db(
    db: &crate::db::ActionDb,
    account_id: &str,
    products: Vec<(String, Option<String>, Option<f64>, Option<String>)>,
) -> Result<usize, String> {
    db.with_transaction(|tx| {
        let mut count = 0;
        for (product_type, tier, arr, billing_terms) in products {
            match tx.upsert_product_classification(
                account_id,
                &product_type,
                tier.as_deref(),
                arr,
                billing_terms.as_deref(),
                "Salesforce",
            ) {
                Ok(_) => {
                    count += 1;
                    log::info!(
                        "I651: Upserted product {} ({:?} tier, ${:?} ARR) for {}",
                        product_type,
                        tier,
                        arr,
                        account_id
                    );
                }
                Err(e) => {
                    log::warn!(
                        "I651: Failed to upsert product {} for {}: {}",
                        product_type,
                        account_id,
                        e
                    );
                    return Err(format!("Product upsert failed for {}: {}", product_type, e));
                }
            }
        }
        Ok(count)
    })
}
