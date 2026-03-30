//! I535 / ADR-0100: Glean-first intelligence provider.
//!
//! When Glean is connected, this provider uses the MCP `chat` tool as the
//! primary intelligence computation engine. It produces the same `IntelligenceJson`
//! output as the PTY path, but with data from Salesforce, Zendesk, Gong, Slack,
//! and org directories that local-only enrichment can't access.
//!
//! The provider is called from `intel_queue.rs` when `context_provider.is_remote()`.
//! On failure, the caller falls back to the PTY path transparently.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter};

use super::dimension_prompts::{self, DIMENSION_NAMES};
use super::io::{IntelligenceJson, SourceManifestEntry};
use super::prompts::{parse_intelligence_response, IntelligenceContext};
use crate::context_provider::glean::GleanMcpClient;

use serde::{Deserialize, Serialize};

/// I535: Glean-first intelligence provider.
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

/// I535: Canonical `DiscoveredAccount` type (referenced by I494, I561).
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
    /// I575: When `app_handle` is provided, emits progressive enrichment events.
    pub async fn enrich_entity(
        &self,
        entity_id: &str,
        entity_type: &str,
        entity_name: &str,
        ctx: &IntelligenceContext,
        relationship: Option<&str>,
        app_handle: Option<&AppHandle>,
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
                self.enrich_entity_legacy(entity_id, entity_type, entity_name, ctx, relationship)
                    .await
            }
        }
    }

    /// I574: Parallel per-dimension Glean enrichment.
    ///
    /// Spawns 6 concurrent Glean `chat` calls (one per dimension group),
    /// each with a 30s timeout. Merges successful dimensions into a single
    /// `IntelligenceJson`. Falls back to legacy if 0 dimensions succeed.
    ///
    /// I575: Uses `FuturesUnordered` to process dimensions as they complete,
    /// writing progressive updates to DB and emitting events when `app_handle`
    /// is provided.
    pub async fn enrich_entity_parallel(
        &self,
        entity_id: &str,
        entity_type: &str,
        entity_name: &str,
        ctx: &IntelligenceContext,
        relationship: Option<&str>,
        app_handle: Option<&AppHandle>,
    ) -> Result<IntelligenceJson, String> {
        use crate::intel_queue::{EnrichmentComplete, EnrichmentProgress};

        let overall_start = Instant::now();
        let is_incremental = ctx.prior_intelligence.is_some();
        let total_dimensions = DIMENSION_NAMES.len() as u32;

        // Build 6 dimension prompts
        let prompts: Vec<(String, String)> = DIMENSION_NAMES
            .iter()
            .map(|dim| {
                let prompt = dimension_prompts::build_glean_dimension_prompt(
                    dim,
                    entity_name,
                    entity_type,
                    relationship,
                    ctx,
                    is_incremental,
                );
                (dim.to_string(), prompt)
            })
            .collect();

        log::info!(
            "[I574] Glean parallel enrichment for {} ({}) — {} dimensions, incremental={}",
            entity_name,
            entity_type,
            prompts.len(),
            is_incremental,
        );

        // Clone values needed by spawned tasks (must be 'static)
        let endpoint = self.endpoint.clone();
        let entity_id_owned = entity_id.to_string();
        let entity_type_owned = entity_type.to_string();
        let entity_name_owned = entity_name.to_string();

        // I575: Use tokio::sync::mpsc channel to receive dimension results as they
        // complete, enabling progressive DB writes and event emission.
        let (tx, mut rx) =
            tokio::sync::mpsc::channel::<(String, Result<IntelligenceJson, String>)>(6);
        let mut wrote_debug_file = false;

        for (dim_name, prompt) in prompts {
            let ep = endpoint.clone();
            let eid = entity_id_owned.clone();
            let etype = entity_type_owned.clone();
            let ename = entity_name_owned.clone();
            let is_first = !wrote_debug_file;
            wrote_debug_file = true;
            let sender = tx.clone();

            tokio::spawn(async move {
                let start = Instant::now();
                let client = GleanMcpClient::new(&ep);

                let response_result =
                    tokio::time::timeout(Duration::from_secs(30), client.chat(&prompt, None)).await;

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
                        let _ = sender
                            .send((dim_name, Err("timed out after 30s".to_string())))
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

                // Write debug file for the first dimension only
                if is_first {
                    let debug_path = std::env::temp_dir().join("dailyos-glean-response.txt");
                    if let Err(e) = std::fs::write(&debug_path, &response_text) {
                        log::warn!("[I574] Failed to write debug response: {}", e);
                    } else {
                        log::info!(
                            "[I574] Glean dimension response written to {}",
                            debug_path.display()
                        );
                    }
                }

                // Build a minimal manifest for parsing
                let manifest = vec![SourceManifestEntry {
                    filename: format!("glean_chat_{}", dim_name),
                    content_type: Some("glean_synthesis".to_string()),
                    format: Some("json".to_string()),
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

                        // I651: After commercial_financial merge, extract and upsert products
                        if dim_name == "commercial_financial" && entity_type == "account" {
                            if let Ok(Some(products)) = extract_products_from_response(&partial) {
                                // Best-effort upsert — log but don't fail enrichment if products fail
                                match crate::db::ActionDb::open() {
                                    Ok(db) => {
                                        match upsert_products_to_db(&db, entity_id, products) {
                                            Ok(count) => {
                                                log::info!("[I651] Upserted {} products for {}", count, entity_id);
                                            }
                                            Err(e) => {
                                                log::warn!("[I651] Product upsert failed (best-effort): {}", e);
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        log::warn!("[I651] Could not open DB for product upsert: {}", e);
                                    }
                                }
                            }
                        }

                        // I575: Progressive DB write + event emission
                        if let Some(handle) = app_handle {
                            write_progressive_glean_dimension(entity_id, entity_type, &combined);
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
                Err(_) => {
                    failed_dims.push(dim_name);
                }
            }
        }

        let total_ms = overall_start.elapsed().as_millis();
        log::info!(
            "[I574] Glean parallel: {}/6 dimensions succeeded for {} in {}ms (failed: {:?})",
            succeeded,
            entity_name,
            total_ms,
            failed_dims,
        );

        // I575: Emit completion event
        if let Some(handle) = app_handle {
            let _ = handle.emit(
                "enrichment-complete",
                EnrichmentComplete {
                    entity_id: entity_id.to_string(),
                    entity_type: entity_type.to_string(),
                    succeeded,
                    failed: failed_dims.len() as u32,
                    failed_dimensions: failed_dims.clone(),
                    wall_clock_ms: total_ms as u64,
                },
            );
        }

        if succeeded == 0 {
            return Err(format!("All 6 Glean dimensions failed for {}", entity_name));
        }

        // Set metadata on combined result
        combined.enriched_at = chrono::Utc::now().to_rfc3339();
        // Build source manifest with a single glean_chat entry
        if combined.source_manifest.is_empty() {
            combined.source_manifest.push(SourceManifestEntry {
                filename: "glean_chat".to_string(),
                content_type: Some("glean_synthesis".to_string()),
                format: Some("json".to_string()),
                modified_at: chrono::Utc::now().to_rfc3339(),
                selected: true,
                skip_reason: None,
            });
        }

        // I576: Source-aware reconciliation with existing DB intelligence
        let intel = {
            let existing = crate::db::ActionDb::open()
                .ok()
                .and_then(|db| db.get_entity_intelligence(entity_id).ok().flatten());
            match existing {
                Some(mut existing) => {
                    // I644: Protect stakeholder_insights when user has designated
                    // team members via the Team panel (account_stakeholders with
                    // data_source='user'). Inject synthetic user_edits so
                    // reconcile_enrichment skips the stakeholder array.
                    if entity_type == "account" {
                        let has_user_stakeholders = crate::db::ActionDb::open()
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
                            existing
                                .user_edits
                                .push(crate::intelligence::io::UserEdit {
                                    field_path: "stakeholderInsights".to_string(),
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

        // I624: Stamp source="glean" on product adoption so dual_write_enrichment_products
        // writes products with correct Glean attribution.
        let mut intel = intel;
        stamp_glean_product_source(&mut intel);

        log::info!(
            "[I574] Glean parallel enrichment parsed for {} — assessment: {}, risks: {}, wins: {}, stakeholders: {}",
            entity_name,
            intel.executive_assessment.is_some(),
            intel.risks.len(),
            intel.recent_wins.len(),
            intel.stakeholder_insights.len(),
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
    ) -> Result<IntelligenceJson, String> {
        let is_incremental = ctx.prior_intelligence.is_some();

        // Build the structured prompt requesting I508+I554 JSON
        let prompt = super::glean_prompts::build_glean_enrichment_prompt(
            entity_name,
            entity_type,
            relationship,
            ctx,
            is_incremental,
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

        // I576: Source-aware reconciliation with existing intelligence.
        // Preserves user corrections, transcript items, and dismissed tombstones.
        let intel = {
            let existing = crate::db::ActionDb::open()
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

        // I624: Stamp source="glean" on product adoption so dual_write_enrichment_products
        // writes products with correct Glean attribution.
        let mut intel = intel;
        stamp_glean_product_source(&mut intel);

        log::info!(
            "[I535] Glean enrichment parsed for {} — assessment: {}, risks: {}, wins: {}, stakeholders: {}",
            entity_name,
            intel.executive_assessment.is_some(),
            intel.risks.len(),
            intel.recent_wins.len(),
            intel.stakeholder_insights.len(),
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
        if let Ok(cache) = discovery_cache().lock() {
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

        if let Ok(mut cache) = discovery_cache().lock() {
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
}

/// I575: Write progressive dimension state to DB during Glean parallel enrichment.
///
/// Similar to `write_progressive_dimension` in `intel_queue.rs` but for the Glean path.
/// Non-fatal on error — the final merge+write after all dimensions is authoritative.
fn write_progressive_glean_dimension(
    entity_id: &str,
    entity_type: &str,
    combined: &IntelligenceJson,
) {
    let db = match crate::db::ActionDb::open() {
        Ok(db) => db,
        Err(e) => {
            log::warn!(
                "[I575] Glean progressive write: failed to open DB for {}: {}",
                entity_id,
                e
            );
            return;
        }
    };

    // Progressive writes within a single enrichment cycle use simple dimension
    // merge, NOT reconciliation. Reconciliation is for cross-cycle merges
    // (e.g., Glean refresh on top of existing PTY data). Within one cycle,
    // the combined state is authoritative — just overlay it on existing.
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

    merged.entity_id = entity_id.to_string();
    merged.entity_type = entity_type.to_string();
    merged.enriched_at = chrono::Utc::now().to_rfc3339();

    if let Err(e) = crate::services::intelligence::upsert_assessment_snapshot(&db, &merged) {
        log::warn!(
            "[I575] Glean progressive write failed for {}: {}",
            entity_id,
            e
        );
    } else {
        log::debug!("[I575] Glean progressive write succeeded for {}", entity_id,);
    }
}

/// I535: Emit tiered signals from Glean enrichment output.
///
/// After Glean enrichment writes to entity_assessment, emit source-specific
/// signals at ADR-0100 confidence tiers so they flow through the Intelligence Loop
/// (propagation rules, health scoring, callouts, Bayesian feedback).
pub fn emit_glean_signals(
    db: &crate::db::ActionDb,
    engine: &crate::signals::propagation::PropagationEngine,
    entity_type: &str,
    entity_id: &str,
    intel: &IntelligenceJson,
) {
    use crate::signals::bus::{emit_signal, emit_signal_and_propagate};

    fn source_mentions_slack(source: Option<&str>) -> bool {
        source
            .map(|value| value.to_lowercase())
            .is_some_and(|value| value.contains("slack"))
    }

    let mut slack_context: Vec<String> = Vec::new();

    // CRM / Salesforce data at 0.9 — system of record
    if let Some(ref org) = intel.org_health {
        if let Ok(value) = serde_json::to_string(org) {
            if let Err(e) = emit_signal_and_propagate(
                db,
                engine,
                entity_type,
                entity_id,
                "renewal_data_updated",
                "glean_crm",
                Some(&value),
                0.9,
            ) {
                log::warn!("[I535] Failed to emit renewal_data_updated: {}", e);
            }
        }
    }

    // Support health from Zendesk at 0.85
    if let Some(ref support) = intel.support_health {
        if let Ok(value) = serde_json::to_string(support) {
            if let Err(e) = emit_signal_and_propagate(
                db,
                engine,
                entity_type,
                entity_id,
                "support_health_updated",
                "glean_zendesk",
                Some(&value),
                0.85,
            ) {
                log::warn!("[I535] Failed to emit support_health_updated: {}", e);
            }
        }
    }

    // I649: Write technical footprint from org_health + support_health
    if entity_type == "account" {
        let support_tier = intel
            .org_health
            .as_ref()
            .and_then(|oh| oh.support_tier.clone());
        let support_health_data = intel.support_health.as_ref();
        let has_footprint_data = support_tier.is_some() || support_health_data.is_some();
        if has_footprint_data {
            let csat = support_health_data.and_then(|sh| sh.csat);
            let open_tickets = support_health_data
                .and_then(|sh| sh.open_tickets)
                .unwrap_or(0) as i64;
            if let Err(e) = db.upsert_account_technical_footprint(
                entity_id,
                None,                        // integrations_json
                None,                        // usage_tier
                None,                        // adoption_score
                None,                        // active_users
                support_tier.as_deref(),
                csat,
                open_tickets,
                None,                        // services_stage
                "glean_zendesk",
            ) {
                log::warn!("[I649] Failed to upsert technical footprint for {}: {}", entity_id, e);
            } else if let Err(e) = emit_signal(
                db,
                entity_type,
                entity_id,
                "technical_footprint_updated",
                "glean_zendesk",
                None,
                0.85,
            ) {
                log::warn!("[I649] Failed to emit technical_footprint_updated: {}", e);
            }
        }
    }

    // Competitive mentions at 0.7
    if !intel.competitive_context.is_empty() {
        if let Ok(value) = serde_json::to_string(&intel.competitive_context) {
            let _ = emit_signal(
                db,
                entity_type,
                entity_id,
                "competitor_mentioned",
                "glean_chat",
                Some(&value),
                0.7,
            );
        }
        slack_context.extend(
            intel
                .competitive_context
                .iter()
                .filter(|item| {
                    source_mentions_slack(item.source.as_deref())
                        || item
                            .item_source
                            .as_ref()
                            .is_some_and(|source| source.source == "glean_slack")
                })
                .map(|item| format!("competitive: {}", item.competitor)),
        );
    }

    // Org changes at 0.8 — stakeholder movements
    if !intel.organizational_changes.is_empty() {
        if let Ok(value) = serde_json::to_string(&intel.organizational_changes) {
            let _ = emit_signal_and_propagate(
                db,
                engine,
                entity_type,
                entity_id,
                "glean_org_change",
                "glean_chat",
                Some(&value),
                0.8,
            );
        }
        slack_context.extend(
            intel
                .organizational_changes
                .iter()
                .filter(|item| {
                    source_mentions_slack(item.source.as_deref())
                        || item
                            .item_source
                            .as_ref()
                            .is_some_and(|source| source.source == "glean_slack")
                })
                .map(|item| format!("org_change: {}", item.person)),
        );
    }

    // Gong call summaries at 0.8 — engagement patterns from recorded calls
    if !intel.gong_call_summaries.is_empty() {
        if let Ok(value) = serde_json::to_string(&intel.gong_call_summaries) {
            let _ = emit_signal_and_propagate(
                db,
                engine,
                entity_type,
                entity_id,
                "gong_engagement_updated",
                "glean_gong",
                Some(&value),
                0.8,
            );
        }
    }

    slack_context.extend(
        intel
            .risks
            .iter()
            .filter(|item| {
                source_mentions_slack(item.source.as_deref())
                    || item
                        .item_source
                        .as_ref()
                        .is_some_and(|source| source.source == "glean_slack")
            })
            .map(|item| format!("risk: {}", item.text)),
    );
    slack_context.extend(
        intel
            .recent_wins
            .iter()
            .filter(|item| {
                source_mentions_slack(item.source.as_deref())
                    || item
                        .item_source
                        .as_ref()
                        .is_some_and(|source| source.source == "glean_slack")
            })
            .map(|item| format!("win: {}", item.text)),
    );
    slack_context.extend(
        intel
            .stakeholder_insights
            .iter()
            .filter(|item| {
                source_mentions_slack(item.source.as_deref())
                    || item
                        .item_source
                        .as_ref()
                        .is_some_and(|source| source.source == "glean_slack")
            })
            .map(|item| format!("stakeholder: {}", item.name)),
    );
    if let Some(open_commitments) = intel.open_commitments.as_ref() {
        slack_context.extend(
            open_commitments
                .iter()
                .filter(|item| {
                    source_mentions_slack(item.source.as_deref())
                        || item
                            .item_source
                            .as_ref()
                            .is_some_and(|source| source.source == "glean_slack")
                })
                .map(|item| format!("commitment: {}", item.description)),
        );
    }
    slack_context.extend(
        intel
            .expansion_signals
            .iter()
            .filter(|item| {
                source_mentions_slack(item.source.as_deref())
                    || item
                        .item_source
                        .as_ref()
                        .is_some_and(|source| source.source == "glean_slack")
            })
            .map(|item| format!("expansion: {}", item.opportunity)),
    );

    if !slack_context.is_empty() {
        let payload = serde_json::json!({
            "items": slack_context,
            "count": slack_context.len(),
        })
        .to_string();
        let _ = emit_signal_and_propagate(
            db,
            engine,
            entity_type,
            entity_id,
            "slack_context_updated",
            "glean_slack",
            Some(&payload),
            0.5,
        );
    }

    // Champion health at 0.8 — if champion is weak or lost, emit risk signal
    if let Some(ref health) = intel.health {
        let dims = &health.dimensions;
        {
            // Check champion dimension for concerning score
            if dims.champion_health.score < 40.0 && dims.champion_health.weight > 0.0 {
                let _ = emit_signal_and_propagate(
                    db,
                    engine,
                    entity_type,
                    entity_id,
                    "glean_champion_departed",
                    "glean_chat",
                    Some(
                        &serde_json::json!({
                            "score": dims.champion_health.score,
                            "evidence": dims.champion_health.evidence,
                        })
                        .to_string(),
                    ),
                    0.8,
                );
            }
        }
    }

    // Recompute health after Glean signals are emitted so that new CRM/Gong/Zendesk
    // data flows immediately into the 6 health dimensions.
    if entity_type == "account" {
        if let Err(e) =
            crate::services::intelligence::recompute_entity_health(db, entity_id, "account")
        {
            log::warn!(
                "Health recompute failed for {} after Glean signals: {}",
                entity_id,
                e
            );
        }
    }
}

/// I576: Source-aware reconciliation of enrichment output with existing intelligence.
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

    // --- Vec fields: source-aware item reconciliation ---
    // Skip reconciliation for fields with user edits — preserve_user_edits
    // handles those after this function returns.

    if !has_user_edits("risks") {
        result.risks = reconcile_vec_items(
            &existing.risks,
            &new_output.risks,
            refreshed_sources,
            dismissed,
            "risks",
            |r| &r.text,
        );
    }

    if !has_user_edits("recentWins") {
        result.recent_wins = reconcile_vec_items(
            &existing.recent_wins,
            &new_output.recent_wins,
            refreshed_sources,
            dismissed,
            "recentWins",
            |w| &w.text,
        );
    }

    if !has_user_edits("stakeholderInsights") {
        result.stakeholder_insights = reconcile_vec_items(
            &existing.stakeholder_insights,
            &new_output.stakeholder_insights,
            refreshed_sources,
            dismissed,
            "stakeholderInsights",
            |s| &s.name,
        );
    }

    if !has_user_edits("valueDelivered") {
        result.value_delivered = reconcile_vec_items(
            &existing.value_delivered,
            &new_output.value_delivered,
            refreshed_sources,
            dismissed,
            "valueDelivered",
            |v| &v.statement,
        );
    }

    if !has_user_edits("competitiveContext") {
        result.competitive_context = reconcile_vec_items(
            &existing.competitive_context,
            &new_output.competitive_context,
            refreshed_sources,
            dismissed,
            "competitiveContext",
            |c| &c.competitor,
        );
    }

    if !has_user_edits("organizationalChanges") {
        result.organizational_changes = reconcile_vec_items(
            &existing.organizational_changes,
            &new_output.organizational_changes,
            refreshed_sources,
            dismissed,
            "organizationalChanges",
            |o| &o.person,
        );
    }

    if !has_user_edits("expansionSignals") {
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
            let reconciled = reconcile_vec_items(
                existing_oc,
                new_oc,
                refreshed_sources,
                dismissed,
                "openCommitments",
                |c| &c.description,
            );
            result.open_commitments = Some(reconciled);
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
    reconcile_option!(renewal_outlook, "renewalOutlook");
    reconcile_option!(support_health, "supportHealth");
    reconcile_option!(product_adoption, "productAdoption");
    reconcile_option!(nps_csat, "npsCsat");
    reconcile_option!(source_attribution, "sourceAttribution");
    reconcile_option!(success_plan_signals, "successPlanSignals");

    // Non-source-attributed vecs: strategic_priorities, internal_team, blockers, gong_call_summaries
    if !new_output.strategic_priorities.is_empty() {
        result.strategic_priorities = new_output.strategic_priorities;
    }
    if !new_output.internal_team.is_empty() {
        result.internal_team =
            reconcile_internal_team(&existing.internal_team, &new_output.internal_team);
    }
    if !new_output.blockers.is_empty() {
        result.blockers = new_output.blockers;
    }
    if !new_output.gong_call_summaries.is_empty() {
        result.gong_call_summaries = new_output.gong_call_summaries;
    }

    // Carry forward user_edits and dismissed_items from existing
    result.user_edits = existing.user_edits;
    result.dismissed_items = existing.dismissed_items;

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

/// I624: Ensure product adoption from Glean enrichment carries source="glean".
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

/// I576: Reconcile a Vec of source-attributed items.
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
    // I645: Only enforce dismissals from the last 90 days
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
// I651: Product Classification Extraction & Upsert
// =============================================================================

/// I651: Extract product classification from a Financial dimension response.
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

/// I651: Upsert extracted products to the database.
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
    let mut count = 0;
    for (product_type, tier, arr, billing_terms) in products {
        match db.upsert_product_classification(
            account_id,
            &product_type,
            tier.as_deref(),
            arr,
            billing_terms.as_deref(),
            "salesforce",
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
}
