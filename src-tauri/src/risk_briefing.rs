//! Risk Briefing generation for at-risk accounts.
//!
//! Produces a 6-slide executive risk briefing (Cover → Bottom Line →
//! What Happened → Stakes → The Plan → The Ask). Uses SCQA as an internal
//! thinking tool but outputs a presentation structure. Reuses
//! `build_intelligence_context()` from entity_intel.rs and enriches via
//! Claude Code PTY with a specialized strategy consultant prompt.

use std::path::Path;

use chrono::Utc;

use std::path::PathBuf;

use crate::db::ActionDb;
use crate::entity_intel::{build_intelligence_context, read_intelligence_json, IntelligenceContext};
use crate::pty::{ModelTier, PtyManager};
use crate::types::{AiModelConfig, RiskBriefing, RiskBottomLine, RiskCover};
use crate::util::atomic_write_str;

// =============================================================================
// Gathered Input (Phase 1 output — captured under brief DB lock)
// =============================================================================

/// Everything needed to run the PTY enrichment, gathered under a brief DB lock.
/// This struct owns all its data so it can be sent to a blocking task.
pub struct GatheredRiskInput {
    pub account_id: String,
    pub account_name: String,
    pub account_arr: Option<f64>,
    pub account_dir: PathBuf,
    pub workspace_path: PathBuf,
    pub prompt: String,
    pub tam_name: Option<String>,
    pub ai_models: AiModelConfig,
}

// =============================================================================
// File I/O
// =============================================================================

/// Read a cached risk briefing from `<account_dir>/risk-briefing.json`.
pub fn read_risk_briefing(account_dir: &Path) -> Result<RiskBriefing, String> {
    let path = account_dir.join("risk-briefing.json");
    if !path.exists() {
        return Err("No risk briefing found. Generate one first.".to_string());
    }
    let data = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read risk-briefing.json: {}", e))?;
    serde_json::from_str(&data)
        .map_err(|e| format!("Failed to parse risk-briefing.json: {}", e))
}

/// Write risk briefing to `<account_dir>/risk-briefing.json`.
fn write_risk_briefing(account_dir: &Path, briefing: &RiskBriefing) -> Result<(), String> {
    let path = account_dir.join("risk-briefing.json");
    let json = serde_json::to_string_pretty(briefing)
        .map_err(|e| format!("Failed to serialize risk briefing: {}", e))?;
    atomic_write_str(&path, &json).map_err(|e| format!("Write error: {}", e))
}

// =============================================================================
// Prompt Construction
// =============================================================================

fn build_risk_briefing_prompt(
    account_name: &str,
    ctx: &IntelligenceContext,
    existing_intel: Option<&str>,
) -> String {
    let mut prompt = String::with_capacity(16_000);

    prompt.push_str("You are a senior strategy consultant preparing a 6-slide executive risk briefing. ");
    prompt.push_str("Use SCQA thinking internally (Situation → Complication → Question → Answer). ");
    prompt.push_str("Output a presentation structure executives actually want.\n\n");

    prompt.push_str("# Task\n\n");
    prompt.push_str(&format!(
        "Generate a 6-slide risk briefing for **{}**.\n\n",
        account_name
    ));

    prompt.push_str("# Input Data\n\n");

    if !ctx.facts_block.is_empty() {
        prompt.push_str("## Account Facts\n");
        prompt.push_str(&ctx.facts_block);
        prompt.push_str("\n\n");
    }

    if let Some(intel) = existing_intel {
        prompt.push_str("## Current Intelligence Assessment\n");
        prompt.push_str(intel);
        prompt.push_str("\n\n");
    }

    if !ctx.meeting_history.is_empty() {
        prompt.push_str("## Recent Meeting History (last 90 days)\n");
        prompt.push_str(&ctx.meeting_history);
        prompt.push_str("\n\n");
    }

    if !ctx.open_actions.is_empty() {
        prompt.push_str("## Open Actions\n");
        prompt.push_str(&ctx.open_actions);
        prompt.push_str("\n\n");
    }

    if !ctx.recent_captures.is_empty() {
        prompt.push_str("## Recent Captures (wins/risks/decisions)\n");
        prompt.push_str(&ctx.recent_captures);
        prompt.push_str("\n\n");
    }

    if !ctx.recent_email_signals.is_empty() {
        prompt.push_str("## Email Signals\n");
        prompt.push_str(&ctx.recent_email_signals);
        prompt.push_str("\n\n");
    }

    if !ctx.stakeholders.is_empty() {
        prompt.push_str("## Stakeholders\n");
        prompt.push_str(&ctx.stakeholders);
        prompt.push_str("\n\n");
    }

    if !ctx.file_contents.is_empty() {
        prompt.push_str("## Workspace Files Content\n");
        prompt.push_str(&ctx.file_contents);
        prompt.push_str("\n\n");
    }

    if !ctx.recent_transcripts.is_empty() {
        prompt.push_str("## Recent Call Transcripts\n");
        prompt.push_str(&ctx.recent_transcripts);
        prompt.push_str("\n\n");
    }

    prompt.push_str("# Output Format\n\n");
    prompt.push_str("This is a SLIDE DECK for a 5-minute risk huddle. Each slide fills one screen.\n");
    prompt.push_str("The story: severity → decline arc → stakes → recovery → ask.\n\n");
    prompt.push_str("Respond with ONLY a valid JSON object (no markdown fences, no commentary) matching this exact schema:\n\n");
    prompt.push_str(r#"{
  "bottomLine": {
    "headline": "The whole story in one breath. ~20 words max. e.g. 'Account at risk: MUV dispute blocks $308K renewal. Plan: scraping pilot plus analytics fix by April.'",
    "riskLevel": "high|medium|low — one word ONLY",
    "renewalWindow": "e.g. '9 weeks ending April 20' or null"
  },
  "whatHappened": {
    "narrative": "Exactly 3 sentences. Sentence 1: baseline state. Sentence 2: what disrupted it. Sentence 3: where we are now. Be specific — cite dates, names, numbers.",
    "healthArc": [{"period": "Q3 2025", "status": "green", "detail": "2-3 words only"}],
    "keyLosses": ["Max 3 one-liners. Merge stakeholder changes + perception shifts. e.g. 'VP Eng disengaged since Jan 15'"]
  },
  "stakes": {
    "financialHeadline": "ACTION HEADLINE, max 10 words. e.g. '$308K base plus $150K expansion at risk through April'",
    "stakeholders": [
      {
        "name": "First Last",
        "role": "Title",
        "alignment": "champion|neutral|detractor|unknown",
        "engagement": "high|medium|low|disengaged",
        "decisionWeight": "decision_maker|influencer|user|blocker",
        "assessment": "OMIT unless critical — badges say it"
      }
    ],
    "decisionMaker": "Name, Title — max 5 words",
    "worstCase": "One line. e.g. 'Full churn: $308K ARR loss, reference account gone'"
  },
  "thePlan": {
    "strategy": "ACTION HEADLINE, max 10 words. e.g. 'Scraping pilot plus analytics fix tied to Q4 expansion'",
    "actions": [
      {"step": "Verb phrase, max 6 words", "owner": "Role", "timeline": "This week"}
    ],
    "timeline": "e.g. 'Save window: 9 weeks ending April 20'",
    "assumptions": ["Max 2. 'If X, plan fails because Y.' Folded from red team."]
  },
  "theAsk": {
    "requests": [
      {"request": "Verb phrase, max 8 words", "urgency": "immediate|this_week|this_month", "from": "Team or role, 2 words"}
    ],
    "decisions": ["Max 2. Max 8 words each."],
    "escalation": "Single line. 'If X → escalate to Y.' or null"
  }
}"#);

    prompt.push_str("\n\n# Writing Rules\n\n");
    prompt.push_str("1. This is a SLIDE DECK for a 5-minute risk huddle. If the exec reads only bottomLine.headline, they get the whole story.\n");
    prompt.push_str("2. whatHappened.narrative must be EXACTLY 3 sentences — baseline → disruption → now.\n");
    prompt.push_str("3. McKinsey action titles, not prose. 'Champion intact, budget frozen' not 'The champion relationship remains stable.'\n");
    prompt.push_str("4. Name names, cite dates, state numbers. 'No VP Eng meeting since Jan 15' not 'engagement decreased.'\n");
    prompt.push_str("5. Max items: stakeholders=4, everything else=3 or fewer.\n");
    prompt.push_str("6. Assumptions max 2, each must state: 'If [assumption], plan fails because [consequence].'\n");
    prompt.push_str("7. Do NOT wrap the JSON in markdown code fences.\n");

    prompt
}

// =============================================================================
// Response Parsing
// =============================================================================

/// Extract and parse the JSON risk briefing from Claude's output.
fn parse_risk_briefing_response(
    stdout: &str,
    account_id: &str,
    account_name: &str,
    arr: Option<f64>,
    tam_name: Option<String>,
) -> Result<RiskBriefing, String> {
    // Try to find JSON object in the output
    let json_str = extract_json_object(stdout)
        .ok_or_else(|| "No valid JSON object found in Claude response".to_string())?;

    // Parse into a serde_json::Value first so we can merge with mechanical data
    let val: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| format!("Failed to parse risk briefing JSON: {}", e))?;

    // Build the full RiskBriefing by combining mechanical cover with AI content
    let now = Utc::now().to_rfc3339();

    // Parse the 5 AI-generated sections
    let bottom_line: RiskBottomLine =
        serde_json::from_value(val.get("bottomLine").cloned().unwrap_or_default())
            .map_err(|e| format!("Failed to parse bottomLine: {}", e))?;
    let what_happened =
        serde_json::from_value(val.get("whatHappened").cloned().unwrap_or_default())
            .map_err(|e| format!("Failed to parse whatHappened: {}", e))?;
    let stakes = serde_json::from_value(val.get("stakes").cloned().unwrap_or_default())
        .map_err(|e| format!("Failed to parse stakes: {}", e))?;
    let the_plan = serde_json::from_value(val.get("thePlan").cloned().unwrap_or_default())
        .map_err(|e| format!("Failed to parse thePlan: {}", e))?;
    let the_ask = serde_json::from_value(val.get("theAsk").cloned().unwrap_or_default())
        .map_err(|e| format!("Failed to parse theAsk: {}", e))?;

    // Construct mechanical cover, copying risk_level from bottom line
    let cover = RiskCover {
        account_name: account_name.to_string(),
        risk_level: bottom_line.risk_level.clone(),
        arr_at_risk: arr,
        date: now.split('T').next().unwrap_or(&now).to_string(),
        tam_name,
    };

    Ok(RiskBriefing {
        account_id: account_id.to_string(),
        generated_at: now,
        cover,
        bottom_line,
        what_happened,
        stakes,
        the_plan,
        the_ask,
    })
}

/// Find the first complete JSON object `{...}` in the text.
fn extract_json_object(text: &str) -> Option<String> {
    let start = text.find('{')?;
    let bytes = text.as_bytes();
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape = false;

    for (i, &b) in bytes.iter().enumerate().skip(start) {
        if escape {
            escape = false;
            continue;
        }
        if b == b'\\' && in_string {
            escape = true;
            continue;
        }
        if b == b'"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        match b {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(text[start..=i].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

// =============================================================================
// Generation Pipeline (two-phase: gather under lock, enrich without lock)
// =============================================================================

/// Phase 1: Gather context under a brief DB lock.
///
/// Returns a `GatheredRiskInput` that owns all data needed for PTY enrichment.
/// Call this with the DB lock held, then release the lock before calling
/// `run_risk_enrichment`.
pub fn gather_risk_input(
    workspace: &Path,
    db: &ActionDb,
    account_id: &str,
    tam_name: Option<String>,
    ai_models: AiModelConfig,
) -> Result<GatheredRiskInput, String> {
    let account = db
        .get_account(account_id)
        .map_err(|e| format!("DB error: {}", e))?
        .ok_or_else(|| format!("Account not found: {}", account_id))?;

    let account_dir = crate::accounts::resolve_account_dir(workspace, &account);

    // Gather context (reuses entity_intel)
    let prior_intel = read_intelligence_json(&account_dir).ok();
    let ctx = build_intelligence_context(
        workspace,
        db,
        account_id,
        "account",
        Some(&account),
        None,
        prior_intel.as_ref(),
    );

    // Serialize existing intelligence for cross-reference
    let intel_json = prior_intel
        .as_ref()
        .and_then(|i| serde_json::to_string_pretty(i).ok());

    // Build prompt (all data is owned by the prompt string)
    let prompt = build_risk_briefing_prompt(&account.name, &ctx, intel_json.as_deref());

    Ok(GatheredRiskInput {
        account_id: account_id.to_string(),
        account_name: account.name.clone(),
        account_arr: account.arr,
        account_dir,
        workspace_path: workspace.to_path_buf(),
        prompt,
        tam_name,
        ai_models,
    })
}

/// Phase 2: Run PTY enrichment + parse + write (no DB lock needed).
///
/// This is the long-running operation (~60-120s). Run via `spawn_blocking`.
pub fn run_risk_enrichment(input: &GatheredRiskInput) -> Result<RiskBriefing, String> {
    // Spawn Claude Code (Synthesis tier, 300s timeout for complex accounts)
    let pty = PtyManager::for_tier(ModelTier::Synthesis, &input.ai_models).with_timeout(300);
    let output = pty
        .spawn_claude(&input.workspace_path, &input.prompt)
        .map_err(|e| format!("Claude Code error: {}", e))?;

    // Parse response
    let briefing = parse_risk_briefing_response(
        &output.stdout,
        &input.account_id,
        &input.account_name,
        input.account_arr,
        input.tam_name.clone(),
    )?;

    // Write to file
    write_risk_briefing(&input.account_dir, &briefing)?;

    log::info!(
        "Generated risk briefing for '{}' (risk level: {:?})",
        input.account_name,
        briefing.bottom_line.risk_level,
    );

    Ok(briefing)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_object_simple() {
        let text = r#"Here is the result: {"foo": "bar"} and more text"#;
        let result = extract_json_object(text);
        assert_eq!(result, Some(r#"{"foo": "bar"}"#.to_string()));
    }

    #[test]
    fn test_extract_json_object_nested() {
        let text = r#"{"a": {"b": 1}, "c": 2}"#;
        let result = extract_json_object(text);
        assert_eq!(result, Some(r#"{"a": {"b": 1}, "c": 2}"#.to_string()));
    }

    #[test]
    fn test_extract_json_object_with_escaped_braces_in_strings() {
        let text = r#"{"text": "value with \"quotes\" inside"}"#;
        let result = extract_json_object(text);
        assert!(result.is_some());
    }

    #[test]
    fn test_extract_json_object_no_json() {
        let text = "No JSON here at all";
        assert_eq!(extract_json_object(text), None);
    }

    #[test]
    fn test_extract_json_object_with_markdown_fences() {
        let text = "```json\n{\"key\": \"value\"}\n```";
        let result = extract_json_object(text);
        assert_eq!(result, Some(r#"{"key": "value"}"#.to_string()));
    }
}
