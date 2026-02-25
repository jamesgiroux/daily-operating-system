//! SWOT analysis report (Strengths, Weaknesses, Opportunities, Threats).
//!
//! One AI call: builds context from entity_intelligence + meeting history,
//! produces structured JSON with 4 quadrants.

use crate::db::ActionDb;
use crate::intelligence::{build_intelligence_context, read_intelligence_json};
use crate::reports::generator::ReportGeneratorInput;
use crate::reports::prompts::{append_intel_context, build_report_preamble};
use crate::types::AiModelConfig;

// =============================================================================
// Output schema
// =============================================================================

/// A single item in a SWOT quadrant.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SwotItem {
    pub text: String,
    pub source: Option<String>,
}

/// Full SWOT output.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SwotContent {
    pub strengths: Vec<SwotItem>,
    pub weaknesses: Vec<SwotItem>,
    pub opportunities: Vec<SwotItem>,
    pub threats: Vec<SwotItem>,
    pub summary: Option<String>,
}

// =============================================================================
// Prompt
// =============================================================================

fn build_swot_prompt(
    entity_name: &str,
    entity_type: &str,
    db: &ActionDb,
    workspace: &std::path::Path,
    entity_id: &str,
    account: Option<&crate::db::DbAccount>,
) -> String {
    let prior = if entity_type == "account" {
        account.and_then(|a| {
            let dir = crate::accounts::resolve_account_dir(workspace, a);
            read_intelligence_json(&dir).ok()
        })
    } else {
        None
    };

    let ctx = build_intelligence_context(
        workspace,
        db,
        entity_id,
        entity_type,
        account,
        None,
        prior.as_ref(),
        None,
    );

    let mut prompt = build_report_preamble(entity_name, "swot", entity_type);
    prompt.push_str("# Intelligence Data\n\n");
    append_intel_context(&mut prompt, &ctx);

    prompt.push_str("# Output Format\n\n");
    prompt.push_str("Respond with ONLY a valid JSON object (no markdown fences) matching this schema:\n\n");
    prompt.push_str(r#"{
  "strengths": [
    {"text": "Specific strength observed, max 20 words", "source": "meeting-id or null"}
  ],
  "weaknesses": [
    {"text": "Specific weakness or gap, max 20 words", "source": "meeting-id or null"}
  ],
  "opportunities": [
    {"text": "Specific opportunity with context, max 20 words", "source": "signal-id or null"}
  ],
  "threats": [
    {"text": "Specific threat or risk, max 20 words", "source": "signal-id or null"}
  ],
  "summary": "One paragraph executive summary, max 50 words. null if no clear narrative."
}"#);
    prompt.push_str("\n\n# Rules\n");
    prompt.push_str("- 2–5 items per quadrant. No padding.\n");
    prompt.push_str("- Every item must cite a real event, signal, or meeting from the data. Set source to null only if genuinely no citation applies.\n");
    prompt.push_str("- Strengths/Weaknesses = current internal state. Opportunities/Threats = future external forces.\n");
    prompt.push_str("- No generic consulting filler. If there's no data, say so in fewer items.\n");
    prompt.push_str("- SPECIFICITY: If an Entity Intelligence Assessment is present, it is your primary source. Use the specific risks, wins, and named stakeholders from it. Do not ignore it in favor of generic observations from DB metadata.\n");

    prompt
}

// =============================================================================
// Generation input (Phase 1 — called under brief DB lock)
// =============================================================================

pub fn gather_swot_input(
    workspace: &std::path::Path,
    db: &ActionDb,
    entity_id: &str,
    entity_type: &str,
    ai_models: AiModelConfig,
) -> Result<ReportGeneratorInput, String> {
    let account = if entity_type == "account" {
        db.get_account(entity_id)
            .map_err(|e| e.to_string())?
    } else {
        None
    };

    let entity_name = account
        .as_ref()
        .map(|a| a.name.clone())
        .or_else(|| {
            if entity_type == "project" {
                db.get_project(entity_id)
                    .ok()
                    .flatten()
                    .map(|p| p.name)
            } else {
                None
            }
        })
        .or_else(|| {
            if entity_type == "person" {
                db.get_person(entity_id)
                    .ok()
                    .flatten()
                    .map(|p| p.name)
            } else {
                None
            }
        })
        .ok_or_else(|| format!("Entity not found: {} ({})", entity_id, entity_type))?;

    let intel_hash = crate::reports::compute_intel_hash(entity_id, entity_type, db);
    let prompt = build_swot_prompt(
        &entity_name,
        entity_type,
        db,
        workspace,
        entity_id,
        account.as_ref(),
    );

    Ok(ReportGeneratorInput {
        entity_id: entity_id.to_string(),
        entity_type: entity_type.to_string(),
        report_type: "swot".to_string(),
        entity_name,
        workspace: workspace.to_path_buf(),
        prompt,
        ai_models,
        intel_hash,
    })
}

// =============================================================================
// Post-processing: validate and store
// =============================================================================

pub fn parse_swot_response(stdout: &str) -> Result<SwotContent, String> {
    let json_str = crate::risk_briefing::extract_json_object(stdout)
        .ok_or_else(|| "No valid JSON object found in SWOT response".to_string())?;

    serde_json::from_str::<SwotContent>(&json_str)
        .map_err(|e| format!("Failed to parse SWOT JSON: {}", e))
}
