//! EBR/QBR report (I400) — Executive Business Review / Quarterly Business Review.
//!
//! Flagship customer-facing report. Full intelligence context + user entity
//! context. 8 structured sections. Value Delivered must cite real event IDs.

use crate::db::ActionDb;
use crate::intelligence::{build_intelligence_context, read_intelligence_json};
use crate::reports::generator::ReportGeneratorInput;
use crate::reports::prompts::{append_intel_context, build_report_preamble};
use crate::types::AiModelConfig;

// =============================================================================
// Output schema
// =============================================================================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EbrQbrMetric {
    pub metric: String,
    pub baseline: Option<String>,
    pub current: String,
    pub trend: Option<String>, // "up", "down", "stable"
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EbrQbrValueItem {
    pub outcome: String,
    pub source: String, // meeting ID or date — REQUIRED
    pub impact: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EbrQbrRisk {
    pub risk: String,
    pub resolution: Option<String>, // null if still open
    pub status: String,             // "resolved", "open", "mitigated"
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EbrQbrAction {
    pub action: String,
    pub owner: String,
    pub timeline: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EbrQbrContent {
    pub quarter_label: String,
    pub executive_summary: String,
    pub story_bullets: Vec<String>,              // 3 strategic bullets for The Story slide
    pub customer_quote: Option<String>,          // Direct quote from transcript
    pub value_delivered: Vec<EbrQbrValueItem>,
    pub success_metrics: Vec<EbrQbrMetric>,
    pub challenges_and_resolutions: Vec<EbrQbrRisk>,
    pub strategic_roadmap: String,
    pub next_steps: Vec<EbrQbrAction>,
}

// =============================================================================
// Prompt
// =============================================================================

fn build_ebr_qbr_prompt(
    entity_name: &str,
    db: &ActionDb,
    workspace: &std::path::Path,
    entity_id: &str,
    account: Option<&crate::db::DbAccount>,
) -> String {
    let prior = account.and_then(|a| {
        let dir = crate::accounts::resolve_account_dir(workspace, a);
        read_intelligence_json(&dir).ok()
    });

    let ctx = build_intelligence_context(
        workspace,
        db,
        entity_id,
        "account",
        account,
        None,
        prior.as_ref(),
        None,
    );

    // Gather user entity context (role, priorities) for framing
    let user_context: String = db
        .conn_ref()
        .query_row(
            "SELECT COALESCE(role_title, '') || ' | ' || COALESCE(annual_priorities, '[]') || ' | ' || COALESCE(quarterly_priorities, '[]') FROM user_entity LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap_or_default();

    // Quarter label
    use chrono::Datelike;
    let now = chrono::Utc::now();
    let quarter = (now.month() - 1) / 3 + 1;
    let quarter_label = format!("Q{} {}", quarter, now.year());

    let mut prompt = build_report_preamble(entity_name, "ebr_qbr", "account");
    prompt.push_str(&format!("# Quarter: {}\n\n", quarter_label));

    if !user_context.is_empty() {
        prompt.push_str("## Your Role Context\n");
        prompt.push_str(&crate::util::wrap_user_data(&user_context));
        prompt.push_str("\n\n");
    }

    prompt.push_str("# Intelligence Data\n\n");
    append_intel_context(&mut prompt, &ctx);

    prompt.push_str("# Output Format\n\n");
    prompt.push_str("This is a CUSTOMER-FACING document. Never use internal jargon ('enrichment', 'signals', 'entity', 'intelligence'). Use business language.\n\n");
    prompt.push_str("Respond with ONLY a valid JSON object (no markdown fences) matching this schema:\n\n");
    prompt.push_str(&format!(r#"{{
  "quarterLabel": "{quarter_label}",
  "executiveSummary": "1 strategic paragraph (max 60 words). State of the partnership.",
  "storyBullets": [
    "Strategic bullet 1 — what defined this quarter, max 20 words",
    "Strategic bullet 2",
    "Strategic bullet 3"
  ],
  "customerQuote": "A direct quote from a meeting transcript that captures a win or positive experience. Exact words. null if no real quote available.",
  "valueDelivered": [
    {{
      "outcome": "Specific business outcome, max 25 words",
      "source": "meeting-YYYY-MM-DD or event-id — REQUIRED",
      "impact": "Quantified impact or null"
    }}
  ],
  "successMetrics": [
    {{
      "metric": "Metric name",
      "baseline": "Starting value or null",
      "current": "Current value",
      "trend": "up|down|stable"
    }}
  ],
  "challengesAndResolutions": [
    {{
      "risk": "The challenge",
      "resolution": "How resolved or null",
      "status": "resolved|open|mitigated"
    }}
  ],
  "strategicRoadmap": "2-3 sentences on next quarter direction. Forward-looking, strategic level.",
  "nextSteps": [
    {{
      "action": "Verb phrase, max 10 words",
      "owner": "CSM|Customer|Joint",
      "timeline": "This month|This quarter|TBD"
    }}
  ]
}}"#,
        quarter_label = quarter_label
    ));

    prompt.push_str("\n\n# Rules\n");
    prompt.push_str("- value_delivered: Max 5 items. Each MUST have a real source (meeting date, signal date). No fabrication.\n");
    prompt.push_str("- success_metrics: Only include metrics where real data exists. Max 5.\n");
    prompt.push_str("- challenges: Include resolved risks too (status='resolved'). Temporal inference allowed.\n");
    prompt.push_str("- next_steps: 3–5 concrete actions. Mix CSM and customer owners.\n");
    prompt.push_str("- strategic_roadmap: Synthesis only — no promises not supported by data.\n");
    prompt.push_str("- customer_quote: Must be from actual transcript or email content. No paraphrasing — quote or null.\n");
    prompt.push_str("- CUSTOMER-FACING: No mentions of AI, enrichment, or internal tooling.\n");
    prompt.push_str("- INTERNAL CONTENT FILTER: Some meeting records and transcripts are internal team debriefs (Amy/CSM post-call discussions, internal strategy sessions). These are labeled as internal. Do NOT reference internal team assessments, pricing strategy, internal concerns, or CSM-only discussions in this document. Only use content from customer-facing meetings and customer statements.\n");
    prompt.push_str("- SPECIFICITY: Use the Entity Intelligence Assessment (if present) as your primary source. Extract the specific risks, wins, and strategic context from it. Generic statements like 'consistent meeting cadence' without specifics are not acceptable.\n");

    prompt
}

// =============================================================================
// Generation input (Phase 1)
// =============================================================================

pub fn gather_ebr_qbr_input(
    workspace: &std::path::Path,
    db: &ActionDb,
    entity_id: &str,
    ai_models: AiModelConfig,
) -> Result<ReportGeneratorInput, String> {
    let account = db
        .get_account(entity_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Account not found: {}", entity_id))?;

    let entity_name = account.name.clone();
    let intel_hash = crate::reports::compute_intel_hash(entity_id, "account", db);
    let prompt = build_ebr_qbr_prompt(&entity_name, db, workspace, entity_id, Some(&account));

    Ok(ReportGeneratorInput {
        entity_id: entity_id.to_string(),
        entity_type: "account".to_string(),
        report_type: "ebr_qbr".to_string(),
        entity_name,
        workspace: workspace.to_path_buf(),
        prompt,
        ai_models,
        intel_hash,
    })
}

// =============================================================================
// Response parsing
// =============================================================================

pub fn parse_ebr_qbr_response(stdout: &str) -> Result<EbrQbrContent, String> {
    let json_str = crate::risk_briefing::extract_json_object(stdout)
        .ok_or_else(|| "No valid JSON object found in EBR/QBR response".to_string())?;

    serde_json::from_str::<EbrQbrContent>(&json_str)
        .map_err(|e| format!("Failed to parse EBR/QBR JSON: {}", e))
}
