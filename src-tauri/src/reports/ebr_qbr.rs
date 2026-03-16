//! EBR/QBR report (I400) — Executive Business Review / Quarterly Business Review.
//!
//! Flagship customer-facing report. Full intelligence context + user entity
//! context. 8 structured sections. Value Delivered must cite real event IDs.

use crate::context_provider::ContextProvider;
use crate::db::ActionDb;
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
    pub story_bullets: Vec<String>, // 3 strategic bullets for The Story slide
    pub customer_quote: Option<String>, // Direct quote from transcript
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
    _workspace: &std::path::Path,
    entity_id: &str,
    _account: Option<&crate::db::DbAccount>,
    active_preset: &str,
    context_provider: &dyn ContextProvider,
) -> String {
    let entity_noun = match active_preset {
        "sales" => "deal",
        "agency" | "consulting" => "client",
        "partnerships" => "partner",
        "product" => "initiative",
        "the-desk" => "project",
        _ => "account",
    };
    let review_name = match active_preset {
        "agency" => "Client Business Review",
        "consulting" => "Executive Review",
        "partnerships" => "Partner Business Review",
        "sales" => "Business Review",
        "leadership" => "Executive Briefing",
        _ => "Executive Business Review",
    };
    let audience_framing = match active_preset {
        "agency" => "client-facing",
        "consulting" => "executive/steering committee-facing",
        "partnerships" => "partner-facing",
        "sales" => "customer-facing",
        "leadership" => "executive-facing",
        _ => "customer-facing",
    };
    let prior = db.get_entity_intelligence(entity_id).ok().flatten();

    let ctx = context_provider
        .gather_entity_context(db, entity_id, "account", prior.as_ref())
        .unwrap_or_default();

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
    prompt.push_str(&format!(
        "Role preset: {} ({} vocabulary). This is a {} {} document — adapt language accordingly.\n\n",
        active_preset, entity_noun, audience_framing, review_name
    ));
    prompt.push_str(&format!("# Quarter: {}\n\n", quarter_label));

    if !user_context.is_empty() {
        prompt.push_str("## Your Role Context\n");
        prompt.push_str(&crate::util::wrap_user_data(&user_context));
        prompt.push_str("\n\n");
    }

    // Gather verbatim customer quotes from captures (90 days)
    let customer_quotes: String = db
        .conn_ref()
        .prepare(
            "SELECT c.evidence_quote, c.content, c.capture_type, c.meeting_title, c.captured_at
             FROM captures c
             WHERE c.account_id = ?1
               AND c.evidence_quote IS NOT NULL
               AND c.evidence_quote != ''
               AND c.captured_at >= datetime('now', '-90 days')
             ORDER BY c.captured_at DESC
             LIMIT 10",
        )
        .and_then(|mut s| {
            let rows = s.query_map(rusqlite::params![entity_id], |row| {
                let quote: String = row.get(0)?;
                let content: String = row.get(1)?;
                let ctype: String = row.get(2)?;
                let mtitle: Option<String> = row.get(3)?;
                let captured: String = row.get(4)?;
                let date = captured.split('T').next().unwrap_or(&captured).to_string();
                let src = mtitle.unwrap_or_else(|| "unknown".to_string());
                Ok(format!("- \"{}\" — {} ({}) [context: {} — {}]", quote, src, date, ctype, content))
            })?;
            Ok(rows.filter_map(|r| r.ok()).collect::<Vec<_>>().join("\n"))
        })
        .unwrap_or_default();

    prompt.push_str("# Intelligence Data\n\n");
    append_intel_context(&mut prompt, &ctx);

    if !customer_quotes.is_empty() {
        prompt.push_str("\n## Customer Quotes (verbatim from meetings)\n");
        prompt.push_str(&crate::util::wrap_user_data(&customer_quotes));
        prompt.push_str("\n\n");
    }

    prompt.push_str("# Output Format\n\n");
    prompt.push_str(&format!(
        "This is a {} document ({}). Never use internal jargon ('enrichment', 'signals', 'entity', 'intelligence'). Use business language.\n\n",
        review_name, audience_framing
    ));
    prompt.push_str(
        "Respond with ONLY a valid JSON object (no markdown fences) matching this schema:\n\n",
    );
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
    prompt.push_str("- customer_quote: Select the most impactful verbatim customer quote from the Customer Quotes section above. Use their exact words. If no suitable quotes are available in the Customer Quotes section, return null. Do NOT fabricate or paraphrase — only use quotes marked as verbatim.\n");
    prompt.push_str(&format!(
        "- AUDIENCE: This is a {} document. Use appropriate language for the audience. No internal jargon, no app mechanics.\n",
        audience_framing
    ));
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
    active_preset: &str,
    context_provider: &dyn ContextProvider,
) -> Result<ReportGeneratorInput, String> {
    let account = db
        .get_account(entity_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Account not found: {}", entity_id))?;

    let entity_name = account.name.clone();
    let intel_hash = crate::reports::compute_intel_hash(entity_id, "account", db);
    let prompt = build_ebr_qbr_prompt(
        &entity_name,
        db,
        workspace,
        entity_id,
        Some(&account),
        active_preset,
        context_provider,
    );

    Ok(ReportGeneratorInput {
        entity_id: entity_id.to_string(),
        entity_type: "account".to_string(),
        report_type: "ebr_qbr".to_string(),
        entity_name,
        workspace: workspace.to_path_buf(),
        prompt,
        ai_models,
        intel_hash,
        extra_data: None,
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
