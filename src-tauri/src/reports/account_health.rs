//! Account Health Review report (I399).
//!
//! Produces a structured health assessment reading entity_assessment
//! fields plus meeting cadence and email signal data.

use crate::context_provider::ContextProvider;
use crate::db::ActionDb;
use crate::reports::generator::ReportGeneratorInput;
use crate::reports::prompts::{append_intel_context, build_report_preamble};
use crate::types::AiModelConfig;
use chrono::Utc;

// =============================================================================
// Output schema
// =============================================================================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AccountHealthSignal {
    pub text: String,
    pub source: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountHealthRisk {
    pub risk: String,
    pub status: String, // "open" | "mitigated" | "resolved"
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountHealthContent {
    pub overall_assessment: String,
    pub health_score_narrative: Option<String>,
    pub relationship_summary: String,    // 2 sentences
    pub engagement_cadence: String,      // "X meetings in 90 days" style
    pub customer_quote: Option<String>,  // Direct quote if available
    pub what_is_working: Vec<String>,    // 2-4 items
    pub what_is_struggling: Vec<String>, // 1-3 items
    pub expansion_signals: Vec<String>,  // Growth opportunities
    pub value_delivered: Vec<AccountHealthSignal>,
    pub risks: Vec<AccountHealthRisk>,
    pub renewal_context: Option<String>,
    pub recommended_actions: Vec<String>,
}

// =============================================================================
// Prompt
// =============================================================================

fn build_account_health_prompt(
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
    let close_concept = match active_preset {
        "sales" => "close date and deal stage",
        "agency" | "consulting" => "contract renewal or engagement end date",
        "partnerships" => "agreement renewal",
        "product" => "launch milestone or delivery date",
        "the-desk" => "project deadline",
        _ => "renewal date",
    };
    let relationship_framing = match active_preset {
        "sales" => "prospect/client relationship",
        "agency" | "consulting" => "client engagement",
        "partnerships" => "partner relationship",
        "leadership" => "strategic relationship",
        "product" => "stakeholder relationship",
        "the-desk" => "working relationship",
        _ => "customer partnership",
    };
    let prior = db.get_entity_intelligence(entity_id).ok().flatten();

    let ctx = context_provider
        .gather_entity_context(db, entity_id, "account", prior.as_ref())
        .unwrap_or_default();

    // Gather supplemental data: meeting count (90d), email signal count, renewal date
    let ninety_days_ago = (Utc::now() - chrono::Duration::days(90)).to_rfc3339();
    let meeting_count_90d: i64 = db
        .conn_ref()
        .query_row(
            "SELECT COUNT(*) FROM meetings m
             JOIN meeting_entities me ON me.meeting_id = m.id
             WHERE me.entity_id = ?1 AND m.start_time > ?2
               AND m.meeting_type NOT IN ('personal', 'focus', 'blocked')",
            rusqlite::params![entity_id, ninety_days_ago],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let email_signal_count: i64 = db
        .conn_ref()
        .query_row(
            "SELECT COUNT(*) FROM signal_events WHERE entity_id = ?1 AND signal_type LIKE 'email%'",
            rusqlite::params![entity_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let renewal_date: Option<String> = db
        .conn_ref()
        .query_row(
            "SELECT event_date FROM account_events WHERE account_id = ?1 AND event_type = 'renewal' ORDER BY event_date ASC LIMIT 1",
            rusqlite::params![entity_id],
            |row| row.get(0),
        )
        .ok();

    let mut prompt = build_report_preamble(entity_name, "account_health", "account");
    prompt.push_str(&format!(
        "Role preset: {} ({} vocabulary). Use '{}' not 'account' where applicable. The close/renewal concept for this role is '{}'.\n\n",
        active_preset, entity_noun, entity_noun, close_concept
    ));

    // Add supplemental engagement data
    prompt.push_str("## Engagement Metrics\n");
    prompt.push_str(&format!("- Meetings last 90 days: {}\n", meeting_count_90d));
    prompt.push_str(&format!(
        "- Email signals tracked: {}\n",
        email_signal_count
    ));
    if let Some(ref rd) = renewal_date {
        prompt.push_str(&format!("- Next renewal date: {}\n", rd));
    }
    prompt.push('\n');

    // Gather urgency-enriched captures (90 days) via existing DB function
    let enriched_captures = db
        .get_account_enriched_captures(entity_id, 90)
        .unwrap_or_default();

    let captures_section: String = if enriched_captures.is_empty() {
        String::new()
    } else {
        let mut lines = Vec::new();
        for cap in enriched_captures.iter().take(20) {
            let urg = cap.urgency.as_deref().unwrap_or("none");
            let sub = cap.sub_type.as_deref().unwrap_or("");
            let date = cap.captured_at.split('T').next().unwrap_or(&cap.captured_at);
            let quote = cap.evidence_quote.as_ref()
                .map(|q| format!(" #\"{}\"", q))
                .unwrap_or_default();
            lines.push(format!(
                "- [{}] {} | [{}] {} ({}){}", urg, cap.capture_type, sub, cap.content, date, quote
            ));
        }
        lines.join("\n")
    };

    prompt.push_str("# Intelligence Data\n\n");
    append_intel_context(&mut prompt, &ctx);

    if !captures_section.is_empty() {
        prompt.push_str("\n## Recent Captures (urgency-sorted, RED first)\n");
        prompt.push_str(&crate::util::wrap_user_data(&captures_section));
        prompt.push_str("\n\n");
    }

    prompt.push_str("# Output Format\n\n");
    prompt.push_str(
        "Respond with ONLY a valid JSON object (no markdown fences) matching this schema:\n\n",
    );
    prompt.push_str(&format!(
        "{{\n\
  \"overallAssessment\": \"One sentence: current state of this {entity_noun}. Direct.\",\n\
  \"healthScoreNarrative\": \"If trend data exists, describe it. null if not.\",\n\
  \"relationshipSummary\": \"2 sentences on {relationship_framing} quality and executive alignment.\",\n\
  \"engagementCadence\": \"Describe meeting rhythm and communication health for this {entity_noun}. e.g. '8 meetings in 90 days, {relationship_framing} led, with decision-maker present twice.'\",\n\
  \"customerQuote\": \"A direct quote from a meeting note or email signal that captures how the counterpart feels about the relationship. Use exact words if available. null if no clear quote.\",\n\
  \"whatIsWorking\": [\"2-4 specific things that are going well. Concrete, not generic.\"],\n\
  \"whatIsStruggling\": [\"1-3 honest gaps or friction points. If nothing is struggling, 1 item minimum.\"],\n\
  \"expansionSignals\": [\"Signals suggesting growth opportunity for this {entity_noun}: new use cases, team growth, positive feedback, scope expansion interest. Empty array if none.\"],\n\
  \"valueDelivered\": [\n\
    {{\"text\": \"Specific outcome, max 20 words\", \"source\": \"meeting-id or date or null\"}}\n\
  ],\n\
  \"risks\": [\n\
    {{\"risk\": \"Specific risk, max 15 words\", \"status\": \"open|mitigated|resolved\"}}\n\
  ],\n\
  \"renewalContext\": \"If a {close_concept} is known, describe context and confidence. null otherwise.\",\n\
  \"recommendedActions\": [\"Action 1 (verb phrase, max 10 words)\", \"Action 2\", \"Action 3\"]\n\
}}",
        entity_noun = entity_noun,
        relationship_framing = relationship_framing,
        close_concept = close_concept,
    ));

    prompt.push_str("\n\n# Rules\n");
    prompt.push_str("- customer_quote: Use real words from meeting notes or email signals. Quote format: \"They said X\" or just the quote itself. null if no real quote.\n");
    prompt.push_str(
        "- what_is_struggling: Be honest. Even healthy accounts have at least one challenge.\n",
    );
    prompt.push_str(
        "- expansion_signals: Only include if there's actual signal data — don't fabricate.\n",
    );
    prompt.push_str("- value_delivered: Must have real citations where possible.\n");
    prompt.push_str("- recommended_actions: Exactly 3. Concrete verb phrases.\n");
    prompt.push_str("- SPECIFICITY: The Entity Intelligence Assessment (if present above) is your primary source. Use the specific risks, named people, and strategic context it identifies. Generic observations from meeting metadata alone are not sufficient.\n");
    prompt.push_str("- CAPTURES: When Recent Captures are present, distinguish RED urgency items from GREEN_WATCH. RED items should inform risks and what_is_struggling. Use evidence_quote when available for customer_quote.\n");

    prompt
}

// =============================================================================
// Generation input (Phase 1 — called under brief DB lock)
// =============================================================================

pub fn gather_account_health_input(
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
    let prompt = build_account_health_prompt(
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
        report_type: "account_health".to_string(),
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

pub fn parse_account_health_response(stdout: &str) -> Result<AccountHealthContent, String> {
    let json_str = crate::risk_briefing::extract_json_object(stdout)
        .ok_or_else(|| "No valid JSON object found in Account Health response".to_string())?;

    serde_json::from_str::<AccountHealthContent>(&json_str)
        .map_err(|e| format!("Failed to parse Account Health JSON: {}", e))
}
