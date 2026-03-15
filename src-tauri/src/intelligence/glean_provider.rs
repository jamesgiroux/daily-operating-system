//! I535 / ADR-0100: Glean-first intelligence provider.
//!
//! When Glean is connected, this provider uses the MCP `chat` tool as the
//! primary intelligence computation engine. It produces the same `IntelligenceJson`
//! output as the PTY path, but with data from Salesforce, Zendesk, Gong, Slack,
//! and org directories that local-only enrichment can't access.
//!
//! The provider is called from `intel_queue.rs` when `context_provider.is_remote()`.
//! On failure, the caller falls back to the PTY path transparently.

use std::time::{Duration, Instant};

use crate::context_provider::glean::GleanMcpClient;
use super::dimension_prompts::{self, DIMENSION_NAMES};
use super::io::{IntelligenceJson, SourceManifestEntry};
use super::prompts::{parse_intelligence_response, IntelligenceContext};

use serde::{Deserialize, Serialize};

/// I535: Glean-first intelligence provider.
///
/// Wraps the GleanMcpClient `chat` tool for structured intelligence queries.
/// Each call produces `IntelligenceJson`-compatible output parseable by
/// `parse_intelligence_response()` — the same parser used for PTY output.
pub struct GleanIntelligenceProvider {
    endpoint: String,
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
    pub async fn enrich_entity(
        &self,
        entity_id: &str,
        entity_type: &str,
        entity_name: &str,
        ctx: &IntelligenceContext,
        relationship: Option<&str>,
    ) -> Result<IntelligenceJson, String> {
        // Try parallel dimension fan-out first
        match self
            .enrich_entity_parallel(entity_id, entity_type, entity_name, ctx, relationship)
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
    pub async fn enrich_entity_parallel(
        &self,
        entity_id: &str,
        entity_type: &str,
        entity_name: &str,
        ctx: &IntelligenceContext,
        relationship: Option<&str>,
    ) -> Result<IntelligenceJson, String> {
        let overall_start = Instant::now();
        let is_incremental = ctx.prior_intelligence.is_some();

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

        // Spawn 6 concurrent tasks
        let mut handles = Vec::with_capacity(6);
        let mut wrote_debug_file = false;

        for (dim_name, prompt) in prompts {
            let ep = endpoint.clone();
            let eid = entity_id_owned.clone();
            let etype = entity_type_owned.clone();
            let ename = entity_name_owned.clone();
            let is_first = !wrote_debug_file;
            wrote_debug_file = true;

            let handle = tokio::spawn(async move {
                let start = Instant::now();
                let client = GleanMcpClient::new(&ep);

                let response_result = tokio::time::timeout(
                    Duration::from_secs(30),
                    client.chat(&prompt, None),
                )
                .await;

                let elapsed_ms = start.elapsed().as_millis();

                let response_text = match response_result {
                    Ok(Ok(text)) => text,
                    Ok(Err(e)) => {
                        log::warn!(
                            "[I574] Glean dimension {} for {} — failed in {}ms: {}",
                            dim_name, ename, elapsed_ms, e
                        );
                        return (dim_name, Err(format!("chat failed: {}", e)));
                    }
                    Err(_) => {
                        log::warn!(
                            "[I574] Glean dimension {} for {} — timed out after {}ms",
                            dim_name, ename, elapsed_ms
                        );
                        return (dim_name, Err("timed out after 30s".to_string()));
                    }
                };

                log::info!(
                    "[I574] Glean dimension {} for {} — {}ms, {} chars",
                    dim_name, ename, elapsed_ms, response_text.len()
                );

                // Write debug file for the first dimension only
                if is_first {
                    let debug_path =
                        std::env::temp_dir().join("dailyos-glean-response.txt");
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

                let parse_result = parse_intelligence_response(
                    &response_text,
                    &eid,
                    &etype,
                    1,
                    manifest,
                );

                match parse_result {
                    Ok(intel) => (dim_name, Ok(intel)),
                    Err(e) => {
                        log::warn!(
                            "[I574] Glean dimension {} for {} — parse failed: {}",
                            dim_name, ename, e
                        );
                        (dim_name, Err(format!("parse failed: {}", e)))
                    }
                }
            });

            handles.push(handle);
        }

        // Collect results
        let mut combined = IntelligenceJson::default();
        let mut succeeded = 0u32;
        let mut failed_dims = Vec::new();

        for handle in handles {
            match handle.await {
                Ok((dim_name, Ok(partial))) => {
                    if let Err(e) =
                        dimension_prompts::merge_dimension_into(&mut combined, &dim_name, &partial)
                    {
                        log::warn!("[I574] Failed to merge dimension {}: {}", dim_name, e);
                        failed_dims.push(dim_name);
                    } else {
                        succeeded += 1;
                    }
                }
                Ok((dim_name, Err(_))) => {
                    failed_dims.push(dim_name);
                }
                Err(e) => {
                    log::warn!("[I574] Dimension task panicked: {}", e);
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

        if succeeded == 0 {
            return Err(format!(
                "All 6 Glean dimensions failed for {}",
                entity_name
            ));
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

        // Merge with existing DB intelligence (same pattern as legacy)
        let intel = {
            let existing = crate::db::ActionDb::open()
                .ok()
                .and_then(|db| db.get_entity_intelligence(entity_id).ok().flatten());
            match existing {
                Some(existing) => merge_glean_into_existing(existing, combined),
                None => combined,
            }
        };

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

        // Merge Glean results into existing intelligence — only overwrite fields
        // that Glean actually populated. Glean returns sparse JSON (omits fields
        // it has no data for), so a wholesale replace would wipe PTY-populated data.
        // Read existing intel from DB (not the truncated context field) for full fidelity.
        let intel = {
            let existing = crate::db::ActionDb::open()
                .ok()
                .and_then(|db| db.get_entity_intelligence(entity_id).ok().flatten());
            match existing {
                Some(existing) => merge_glean_into_existing(existing, glean_intel),
                None => glean_intel,
            }
        };

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
        let prompt =
            super::glean_prompts::build_account_discovery_prompt(user_email, user_name);

        let client = GleanMcpClient::new(&self.endpoint);
        let response_text = client
            .chat(&prompt, None)
            .await
            .map_err(|e| format!("Glean account discovery failed: {}", e))?;

        // Extract JSON from response
        let json_text = extract_json_object(&response_text)
            .ok_or_else(|| "Glean discovery response contained no JSON object".to_string())?;

        let discovery: DiscoveryResponse =
            serde_json::from_str(json_text).map_err(|e| {
                format!("Failed to parse Glean discovery response: {}", e)
            })?;

        log::info!(
            "[I535] Glean discovered {} accounts for {}",
            discovery.accounts.len(),
            user_email
        );

        Ok(discovery.accounts)
    }

    /// Get the endpoint this provider is configured for.
    pub fn endpoint(&self) -> &str {
        &self.endpoint
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
                    Some(&serde_json::json!({
                        "score": dims.champion_health.score,
                        "evidence": dims.champion_health.evidence,
                    }).to_string()),
                    0.8,
                );
            }
        }
    }
}

/// Merge Glean-produced intelligence into existing intelligence.
///
/// Rule: Glean wins for any field it actually populated. Existing data is
/// preserved for fields Glean omitted (returned empty/None/default).
/// This prevents sparse Glean responses from wiping PTY-populated data.
fn merge_glean_into_existing(
    mut existing: IntelligenceJson,
    glean: IntelligenceJson,
) -> IntelligenceJson {
    // Helper macros for the repetitive merge pattern
    macro_rules! merge_option {
        ($field:ident) => {
            if glean.$field.is_some() {
                existing.$field = glean.$field;
            }
        };
    }
    macro_rules! merge_vec {
        ($field:ident) => {
            if !glean.$field.is_empty() {
                existing.$field = glean.$field;
            }
        };
    }

    // Core fields
    merge_option!(executive_assessment);
    merge_vec!(risks);
    merge_vec!(recent_wins);
    merge_option!(current_state);
    merge_vec!(stakeholder_insights);
    merge_vec!(value_delivered);
    merge_option!(next_meeting_readiness);
    merge_option!(company_context);
    merge_option!(network);
    merge_option!(health);
    merge_option!(org_health);
    merge_option!(success_metrics);
    merge_option!(open_commitments);
    merge_option!(relationship_depth);

    // I508a dimension fields
    merge_vec!(competitive_context);
    merge_vec!(strategic_priorities);
    merge_option!(coverage_assessment);
    merge_vec!(organizational_changes);
    merge_vec!(internal_team);
    merge_option!(meeting_cadence);
    merge_option!(email_responsiveness);
    merge_vec!(blockers);
    merge_option!(contract_context);
    merge_vec!(expansion_signals);
    merge_option!(renewal_outlook);
    merge_option!(support_health);
    merge_option!(product_adoption);
    merge_option!(nps_csat);
    merge_option!(source_attribution);
    merge_option!(success_plan_signals);

    // Glean-specific fields
    merge_vec!(gong_call_summaries);

    // Update metadata
    existing.enriched_at = glean.enriched_at;
    if !glean.source_manifest.is_empty() {
        // Append Glean source to existing manifest rather than replacing
        for entry in glean.source_manifest {
            if !existing.source_manifest.iter().any(|e| e.filename == entry.filename) {
                existing.source_manifest.push(entry);
            }
        }
    }

    existing
}

/// Extract the first balanced JSON object from a text response.
///
/// Uses brace counting to find the correct closing `}` that matches
/// the first `{`, handling nested objects correctly.
fn extract_json_object(text: &str) -> Option<&str> {
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
