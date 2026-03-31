//! SWOT analysis report (Strengths, Weaknesses, Opportunities, Threats).
//!
//! One AI call: builds context from entity_assessment + meeting history,
//! produces structured JSON with 4 quadrants.

use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::context_provider::ContextProvider;
use crate::db::ActionDb;
use crate::pty::{AiUsageContext, ModelTier, PtyManager};
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

#[derive(Clone)]
pub struct SwotGatherInput {
    pub entity_id: String,
    pub entity_type: String,
    pub entity_name: String,
    pub workspace: PathBuf,
    pub ai_models: AiModelConfig,
    pub intel_hash: String,
    pub context_prompt: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwotProgressPayload {
    pub entity_id: String,
    pub completed: u32,
    pub total: u32,
    pub section_name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwotContentPayload {
    pub entity_id: String,
    pub content: SwotContent,
}

// =============================================================================
// Prompt
// =============================================================================

fn build_swot_context_prompt(
    entity_name: &str,
    entity_type: &str,
    db: &ActionDb,
    _workspace: &std::path::Path,
    entity_id: &str,
    _account: Option<&crate::db::DbAccount>,
    context_provider: &dyn ContextProvider,
) -> String {
    let prior = db.get_entity_intelligence(entity_id).ok().flatten();

    let ctx = context_provider
        .gather_entity_context(db, entity_id, entity_type, prior.as_ref())
        .unwrap_or_default();

    let mut prompt = build_report_preamble(entity_name, "swot", entity_type);
    prompt.push_str("# Intelligence Data\n\n");
    append_intel_context(&mut prompt, &ctx);
    prompt
}

fn build_swot_section_prompt(context_prompt: &str, section: &str) -> String {
    let mut prompt = String::with_capacity(context_prompt.len() + 2_500);
    prompt.push_str(context_prompt);
    prompt.push_str("\n\n# Output Format\n\n");
    prompt.push_str("Respond with ONLY a valid JSON object (no markdown fences).\n\n");

    match section {
        "strengths" | "weaknesses" | "opportunities" | "threats" => {
            prompt.push_str(&format!(
                r#"{{ "{section}": [{{ "text": "Specific item, max 20 words", "source": "meeting-id or signal-id or null" }}] }}"#,
            ));
            prompt.push_str("\n\nRules:\n");
            prompt.push_str("- Return 2-5 items. No padding.\n");
            prompt.push_str(
                "- Every item must cite a real meeting, signal, or event when possible.\n",
            );
            prompt.push_str("- Be specific. No generic consulting filler.\n");
            match section {
                "strengths" => prompt.push_str("- Focus on current internal advantages.\n"),
                "weaknesses" => {
                    prompt.push_str("- Focus on internal gaps, fragility, or coverage issues.\n")
                }
                "opportunities" => {
                    prompt.push_str("- Focus on future upside and growth openings.\n")
                }
                "threats" => {
                    prompt.push_str("- Focus on future downside, competition, churn, or risk.\n")
                }
                _ => {}
            }
        }
        "summary" => {
            prompt.push_str(
                r#"{ "summary": "One paragraph executive summary, max 50 words, or null" }"#,
            );
            prompt.push_str("\n\nRules:\n");
            prompt.push_str("- Summarize the strategic posture in one short paragraph.\n");
            prompt.push_str("- Mention the main leverage point and the main threat.\n");
        }
        _ => {}
    }

    prompt.push_str("- If the data is sparse, return fewer items rather than inventing detail.\n");
    prompt.push_str(
        "- If an Entity Intelligence Assessment is present, treat it as the primary source.\n",
    );
    prompt
}

pub fn gather_swot_data(
    workspace: &std::path::Path,
    db: &ActionDb,
    entity_id: &str,
    entity_type: &str,
    ai_models: AiModelConfig,
    context_provider: &dyn ContextProvider,
) -> Result<SwotGatherInput, String> {
    let account = if entity_type == "account" {
        db.get_account(entity_id).map_err(|e| e.to_string())?
    } else {
        None
    };

    let entity_name = account
        .as_ref()
        .map(|a| a.name.clone())
        .or_else(|| {
            if entity_type == "project" {
                db.get_project(entity_id).ok().flatten().map(|p| p.name)
            } else {
                None
            }
        })
        .or_else(|| {
            if entity_type == "person" {
                db.get_person(entity_id).ok().flatten().map(|p| p.name)
            } else {
                None
            }
        })
        .ok_or_else(|| format!("Entity not found: {} ({})", entity_id, entity_type))?;

    let intel_hash = crate::reports::compute_intel_hash(entity_id, entity_type, db);
    let context_prompt = build_swot_context_prompt(
        &entity_name,
        entity_type,
        db,
        workspace,
        entity_id,
        account.as_ref(),
        context_provider,
    );

    Ok(SwotGatherInput {
        entity_id: entity_id.to_string(),
        entity_type: entity_type.to_string(),
        entity_name,
        workspace: workspace.to_path_buf(),
        ai_models,
        intel_hash,
        context_prompt,
    })
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
    context_provider: &dyn ContextProvider,
) -> Result<ReportGeneratorInput, String> {
    let gathered = gather_swot_data(
        workspace,
        db,
        entity_id,
        entity_type,
        ai_models,
        context_provider,
    )?;

    Ok(ReportGeneratorInput {
        entity_id: gathered.entity_id,
        entity_type: gathered.entity_type,
        report_type: "swot".to_string(),
        entity_name: gathered.entity_name,
        workspace: gathered.workspace,
        prompt: gathered.context_prompt,
        ai_models: gathered.ai_models,
        intel_hash: gathered.intel_hash,
        extra_data: None,
    })
}

pub fn run_parallel_swot_generation(
    input: &SwotGatherInput,
    app_handle: Option<&AppHandle>,
) -> Result<SwotContent, String> {
    let sections = [
        "strengths",
        "weaknesses",
        "opportunities",
        "threats",
        "summary",
    ];
    let total = sections.len() as u32;
    let (tx, rx) = std::sync::mpsc::channel();

    for section in sections {
        let prompt = build_swot_section_prompt(&input.context_prompt, section);
        let workspace = input.workspace.clone();
        let ai_models = input.ai_models.clone();
        let section_name = section.to_string();
        let sender = tx.clone();
        std::thread::spawn(move || {
            let pty = PtyManager::for_tier(ModelTier::Extraction, &ai_models)
                .with_usage_context(
                    AiUsageContext::new("reports", "swot_section_generation")
                        .with_trigger(&section_name)
                        .with_tier(ModelTier::Extraction),
                )
                .with_timeout(30)
                .with_nice_priority(10);
            let result = pty
                .spawn_claude(&workspace, &prompt)
                .map_err(|e| format!("Claude Code error for {}: {}", section_name, e))
                .and_then(|output| {
                    let json_str = crate::risk_briefing::extract_json_object(&output.stdout)
                        .ok_or_else(|| {
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

    let mut content = SwotContent {
        strengths: Vec::new(),
        weaknesses: Vec::new(),
        opportunities: Vec::new(),
        threats: Vec::new(),
        summary: None,
    };
    let mut completed = 0u32;

    for result in rx {
        match result {
            Ok((section, value, raw_output)) => {
                let _ = crate::audit::write_audit_entry(
                    &input.workspace,
                    &format!("swot_{}", section),
                    &input.entity_id,
                    &raw_output,
                );
                match section.as_str() {
                    "strengths" => {
                        if let Some(v) = value.get("strengths") {
                            if let Ok(items) = serde_json::from_value::<Vec<SwotItem>>(v.clone()) {
                                content.strengths = items;
                            }
                        }
                    }
                    "weaknesses" => {
                        if let Some(v) = value.get("weaknesses") {
                            if let Ok(items) = serde_json::from_value::<Vec<SwotItem>>(v.clone()) {
                                content.weaknesses = items;
                            }
                        }
                    }
                    "opportunities" => {
                        if let Some(v) = value.get("opportunities") {
                            if let Ok(items) = serde_json::from_value::<Vec<SwotItem>>(v.clone()) {
                                content.opportunities = items;
                            }
                        }
                    }
                    "threats" => {
                        if let Some(v) = value.get("threats") {
                            if let Ok(items) = serde_json::from_value::<Vec<SwotItem>>(v.clone()) {
                                content.threats = items;
                            }
                        }
                    }
                    "summary" => {
                        content.summary = value
                            .get("summary")
                            .and_then(|v| v.as_str())
                            .map(|v| v.to_string());
                    }
                    _ => {}
                }

                completed += 1;
                if let Some(handle) = app_handle {
                    let _ = handle.emit(
                        "swot-progress",
                        SwotProgressPayload {
                            entity_id: input.entity_id.clone(),
                            completed,
                            total,
                            section_name: section.clone(),
                        },
                    );
                    let _ = handle.emit(
                        "swot-content",
                        SwotContentPayload {
                            entity_id: input.entity_id.clone(),
                            content: content.clone(),
                        },
                    );
                }
            }
            Err(e) => log::warn!("swot section generation failed: {}", e),
        }
    }

    if completed == 0 {
        return Err("All SWOT sections failed".to_string());
    }

    Ok(content)
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
