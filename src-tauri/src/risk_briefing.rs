//! Risk Briefing generation for at-risk accounts.
//!
//! Produces a 6-slide executive risk briefing (Cover → Bottom Line →
//! What Happened → Stakes → The Plan → The Ask). Uses SCQA as an internal
//! thinking tool but outputs a presentation structure. Gathers context via
//! the `ContextProvider` trait (ADR-0095) and enriches via Claude Code PTY
//! with a specialized strategy consultant prompt.

use std::path::Path;

use chrono::Utc;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use std::path::PathBuf;

use crate::context_provider::ContextProvider;
use crate::db::ActionDb;
use crate::intelligence::IntelligenceContext;
use crate::pty::{ModelTier, PtyManager};
use crate::types::{
    AiModelConfig, RiskBottomLine, RiskBriefing, RiskCover, RiskStakes, RiskTheAsk,
    RiskThePlan, RiskWhatHappened,
};
use crate::util::{atomic_write_str, sanitize_external_field, wrap_user_data, INJECTION_PREAMBLE};

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
    pub context_prompt: String,
    pub tam_name: Option<String>,
    pub ai_models: AiModelConfig,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RiskBriefingProgress {
    account_id: String,
    section_name: String,
    completed: u32,
    total: u32,
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
    serde_json::from_str(&data).map_err(|e| format!("Failed to parse risk-briefing.json: {}", e))
}

/// Write risk briefing to `<account_dir>/risk-briefing.json`.
pub fn write_risk_briefing(account_dir: &Path, briefing: &RiskBriefing) -> Result<(), String> {
    let path = account_dir.join("risk-briefing.json");
    let json = serde_json::to_string_pretty(briefing)
        .map_err(|e| format!("Failed to serialize risk briefing: {}", e))?;
    atomic_write_str(&path, &json).map_err(|e| format!("Write error: {}", e))
}

// =============================================================================
// Prompt Construction
// =============================================================================

fn build_risk_briefing_context(
    account_name: &str,
    ctx: &IntelligenceContext,
    existing_intel: Option<&str>,
) -> String {
    let mut prompt = String::with_capacity(16_000);

    // I468: Injection resistance preamble
    prompt.push_str(INJECTION_PREAMBLE);

    prompt.push_str(
        "You are a senior strategy consultant preparing a 6-slide executive risk briefing. ",
    );
    prompt
        .push_str("Use SCQA thinking internally (Situation → Complication → Question → Answer). ");
    prompt.push_str("Output a presentation structure executives actually want.\n\n");

    prompt.push_str("# Task\n\n");
    prompt.push_str(&format!(
        "Generate a 6-slide risk briefing for **{}**.\n\n",
        sanitize_external_field(account_name)
    ));

    prompt.push_str("# Input Data\n\n");

    if !ctx.facts_block.is_empty() {
        prompt.push_str("## Account Facts\n");
        prompt.push_str(&wrap_user_data(&ctx.facts_block));
        prompt.push_str("\n\n");
    }

    if let Some(intel) = existing_intel {
        prompt.push_str("## Current Intelligence Assessment\n");
        prompt.push_str(&wrap_user_data(intel));
        prompt.push_str("\n\n");
    }

    if !ctx.meeting_history.is_empty() {
        prompt.push_str("## Recent Meeting History (last 90 days)\n");
        prompt.push_str(&wrap_user_data(&ctx.meeting_history));
        prompt.push_str("\n\n");
    }

    if !ctx.open_actions.is_empty() {
        prompt.push_str("## Open Actions\n");
        prompt.push_str(&wrap_user_data(&ctx.open_actions));
        prompt.push_str("\n\n");
    }

    if !ctx.recent_captures.is_empty() {
        prompt.push_str("## Recent Captures (wins/risks/decisions)\n");
        prompt.push_str(&wrap_user_data(&ctx.recent_captures));
        prompt.push_str("\n\n");
    }

    if !ctx.recent_email_signals.is_empty() {
        prompt.push_str("## Email Signals\n");
        prompt.push_str(&wrap_user_data(&ctx.recent_email_signals));
        prompt.push_str("\n\n");
    }

    if !ctx.stakeholders.is_empty() {
        prompt.push_str("## Stakeholders\n");
        prompt.push_str(&wrap_user_data(&ctx.stakeholders));
        prompt.push_str("\n\n");
    }

    if !ctx.file_contents.is_empty() {
        prompt.push_str("## Workspace Files Content\n");
        prompt.push_str(&wrap_user_data(&ctx.file_contents));
        prompt.push_str("\n\n");
    }

    if !ctx.recent_transcripts.is_empty() {
        prompt.push_str("## Recent Call Transcripts\n");
        prompt.push_str(&wrap_user_data(&ctx.recent_transcripts));
        prompt.push_str("\n\n");
    }

    prompt
}

fn build_risk_section_prompt(context_prompt: &str, section: &str) -> String {
    let mut prompt = String::with_capacity(context_prompt.len() + 4_000);
    prompt.push_str(context_prompt);
    prompt.push_str("# Output Format\n\n");
    prompt.push_str(
        "This is a SLIDE DECK for a 5-minute risk huddle. Generate ONLY the requested section as valid JSON.\n\n",
    );

    match section {
        "bottomLine" => {
            prompt.push_str("Return ONLY:\n");
            prompt.push_str(
                r#"{ "bottomLine": { "headline": "MAX 20 WORDS", "riskLevel": "high|medium|low", "renewalWindow": "string or null" } }"#,
            );
            prompt.push_str("\n\nRules:\n");
            prompt.push_str("- headline is the whole story in one breath.\n");
            prompt.push_str("- Use dates, names, and numbers when known.\n");
            prompt.push_str("- No commentary outside the JSON object.\n");
        }
        "whatHappened" => {
            prompt.push_str("Return ONLY:\n");
            prompt.push_str(
                r#"{ "whatHappened": { "narrative": "EXACTLY 3 SENTENCES, MAX 60 WORDS TOTAL", "healthArc": [{"period": "Q3 2025", "status": "green|yellow|red", "detail": "2-3 words"}], "keyLosses": ["Max 3 items, 10 words each"] } }"#,
            );
            prompt.push_str("\n\nRules:\n");
            prompt.push_str("- Sentence 1 baseline, sentence 2 disruption, sentence 3 current state.\n");
            prompt.push_str("- Cite dates and names.\n");
            prompt.push_str("- healthArc.detail must be 2-3 words, never a sentence.\n");
        }
        "stakes" => {
            prompt.push_str("Return ONLY:\n");
            prompt.push_str(
                r#"{ "stakes": { "financialHeadline": "ACTION HEADLINE, max 10 words", "stakeholders": [{ "name": "First Last", "role": "Title", "alignment": "champion|neutral|detractor|unknown", "engagement": "high|medium|low|disengaged", "decisionWeight": "decision_maker|influencer|user|blocker", "assessment": "optional" }], "decisionMaker": "Name, Title", "worstCase": "Single line" } }"#,
            );
            prompt.push_str("\n\nRules:\n");
            prompt.push_str("- Max 4 stakeholders.\n");
            prompt.push_str("- Focus on money, decision-maker posture, and downside.\n");
        }
        "thePlan" => {
            prompt.push_str("Return ONLY:\n");
            prompt.push_str(
                r#"{ "thePlan": { "strategy": "ACTION HEADLINE, max 10 words", "actions": [{ "step": "Verb phrase, max 6 words", "owner": "Role", "timeline": "This week" }], "timeline": "string or null", "assumptions": ["Max 2, 'If X, plan fails because Y'"] } }"#,
            );
            prompt.push_str("\n\nRules:\n");
            prompt.push_str("- Max 3 actions.\n");
            prompt.push_str("- Make the actions specific and near-term.\n");
        }
        "theAsk" => {
            prompt.push_str("Return ONLY:\n");
            prompt.push_str(
                r#"{ "theAsk": { "requests": [{ "request": "Verb phrase, max 8 words", "urgency": "immediate|this_week|this_month", "from": "Team or role" }], "decisions": ["Max 2, 8 words each"], "escalation": "Single line or null" } }"#,
            );
            prompt.push_str("\n\nRules:\n");
            prompt.push_str("- Max 3 requests.\n");
            prompt.push_str("- Every request should be concrete and action-oriented.\n");
        }
        _ => {}
    }

    prompt.push_str("\nShared rules:\n");
    prompt.push_str("- Brevity is absolute.\n");
    prompt.push_str("- McKinsey action titles, not prose.\n");
    prompt.push_str("- Name names, cite dates, state numbers.\n");
    prompt.push_str("- Do NOT wrap the JSON in markdown fences.\n");

    prompt
}

fn empty_risk_briefing(input: &GatheredRiskInput) -> RiskBriefing {
    let now = Utc::now().to_rfc3339();
    RiskBriefing {
        account_id: input.account_id.clone(),
        generated_at: now.clone(),
        cover: RiskCover {
            account_name: input.account_name.clone(),
            risk_level: None,
            arr_at_risk: input.account_arr,
            date: now.split('T').next().unwrap_or(&now).to_string(),
            tam_name: input.tam_name.clone(),
        },
        bottom_line: RiskBottomLine {
            headline: String::new(),
            risk_level: None,
            renewal_window: None,
        },
        what_happened: RiskWhatHappened {
            narrative: String::new(),
            health_arc: Vec::new(),
            key_losses: Vec::new(),
        },
        stakes: RiskStakes {
            financial_headline: None,
            stakeholders: Vec::new(),
            decision_maker: None,
            worst_case: None,
        },
        the_plan: RiskThePlan {
            strategy: String::new(),
            actions: Vec::new(),
            timeline: None,
            assumptions: Vec::new(),
        },
        the_ask: RiskTheAsk {
            requests: Vec::new(),
            decisions: Vec::new(),
            escalation: None,
        },
    }
}

fn merge_risk_section(briefing: &mut RiskBriefing, section: &str, value: serde_json::Value) -> Result<(), String> {
    match section {
        "bottomLine" => {
            briefing.bottom_line = serde_json::from_value(
                value.get("bottomLine").cloned().unwrap_or_default(),
            )
            .map_err(|e| format!("Failed to parse bottomLine: {}", e))?;
            briefing.cover.risk_level = briefing.bottom_line.risk_level.clone();
        }
        "whatHappened" => {
            briefing.what_happened = serde_json::from_value(
                value.get("whatHappened").cloned().unwrap_or_default(),
            )
            .map_err(|e| format!("Failed to parse whatHappened: {}", e))?;
        }
        "stakes" => {
            briefing.stakes = serde_json::from_value(value.get("stakes").cloned().unwrap_or_default())
                .map_err(|e| format!("Failed to parse stakes: {}", e))?;
        }
        "thePlan" => {
            briefing.the_plan =
                serde_json::from_value(value.get("thePlan").cloned().unwrap_or_default())
                    .map_err(|e| format!("Failed to parse thePlan: {}", e))?;
        }
        "theAsk" => {
            briefing.the_ask = serde_json::from_value(value.get("theAsk").cloned().unwrap_or_default())
                .map_err(|e| format!("Failed to parse theAsk: {}", e))?;
        }
        _ => return Err(format!("Unknown risk briefing section: {}", section)),
    }
    briefing.generated_at = Utc::now().to_rfc3339();
    Ok(())
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
pub fn extract_json_object(text: &str) -> Option<String> {
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
    context_provider: &dyn ContextProvider,
) -> Result<GatheredRiskInput, String> {
    let account = db
        .get_account(account_id)
        .map_err(|e| format!("DB error: {}", e))?
        .ok_or_else(|| format!("Account not found: {}", account_id))?;

    let account_dir = crate::accounts::resolve_account_dir(workspace, &account);

    // Gather context via ContextProvider (Glean-aware), read from DB (I513)
    let prior_intel = db.get_entity_intelligence(account_id).ok().flatten();
    let ctx = context_provider
        .gather_entity_context(db, account_id, "account", prior_intel.as_ref())
        .unwrap_or_default();

    // Serialize existing intelligence for cross-reference
    let intel_json = prior_intel
        .as_ref()
        .and_then(|i| serde_json::to_string_pretty(i).ok());

    // Build prompt (all data is owned by the prompt string)
    let context_prompt = build_risk_briefing_context(&account.name, &ctx, intel_json.as_deref());

    Ok(GatheredRiskInput {
        account_id: account_id.to_string(),
        account_name: account.name.clone(),
        account_arr: account.arr,
        account_dir,
        workspace_path: workspace.to_path_buf(),
        context_prompt,
        tam_name,
        ai_models,
    })
}

/// Phase 2: Run PTY enrichment + parse + write (no DB lock needed).
///
/// This is the long-running operation. Keep the AI call bounded to 30s
/// so it cannot monopolize a worker indefinitely.
pub fn run_risk_enrichment(
    input: &GatheredRiskInput,
    app_handle: Option<&AppHandle>,
) -> Result<RiskBriefing, String> {
    let sections = ["bottomLine", "whatHappened", "stakes", "thePlan", "theAsk"];
    let total_sections = sections.len() as u32;
    let (tx, rx) = std::sync::mpsc::channel();

    for section in sections {
        let workspace = input.workspace_path.clone();
        let ai_models = input.ai_models.clone();
        let section_name = section.to_string();
        let prompt = build_risk_section_prompt(&input.context_prompt, section);
        let sender = tx.clone();

        std::thread::spawn(move || {
            let pty = PtyManager::for_tier(ModelTier::Extraction, &ai_models)
                .with_timeout(30)
                .with_nice_priority(10);
            let result = pty
                .spawn_claude(&workspace, &prompt)
                .map_err(|e| format!("Claude Code error for {}: {}", section_name, e))
                .and_then(|output| {
                    let json_str = extract_json_object(&output.stdout).ok_or_else(|| {
                        format!("No valid JSON object found in {} response", section_name)
                    })?;
                    let value = serde_json::from_str::<serde_json::Value>(&json_str)
                        .map_err(|e| format!("Failed to parse {} JSON: {}", section_name, e))?;
                    Ok((section_name, value, output.stdout))
                });
            let _ = sender.send(result);
        });
    }
    drop(tx);

    let mut briefing = empty_risk_briefing(input);
    let mut completed = 0u32;

    for result in rx {
        match result {
            Ok((section, value, raw_output)) => {
                let _ = crate::audit::write_audit_entry(
                    &input.workspace_path,
                    &format!("risk_briefing_{}", section),
                    &input.account_id,
                    &raw_output,
                );
                match merge_risk_section(&mut briefing, &section, value) {
                    Ok(()) => {
                        completed += 1;
                        if let Some(handle) = app_handle {
                            let _ = handle.emit(
                                "risk-briefing-progress",
                                RiskBriefingProgress {
                                    account_id: input.account_id.clone(),
                                    section_name: section,
                                    completed,
                                    total: total_sections,
                                },
                            );
                            let _ = handle.emit("risk-briefing-content", &briefing);
                        }
                    }
                    Err(e) => {
                        log::warn!("risk_briefing: section merge failed: {}", e);
                    }
                }
            }
            Err(e) => log::warn!("risk_briefing: section generation failed: {}", e),
        }
    }

    if completed == 0 {
        return Err("All risk briefing sections failed".to_string());
    }

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
