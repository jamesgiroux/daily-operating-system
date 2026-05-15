use serde::Serialize;
use serde_json::{json, Value};

use crate::abilities::provenance::PromptFingerprint;
use crate::intelligence::prompt_fingerprint::{
    canonical_template_hash, prompt_fingerprint_from_completion,
};
use crate::intelligence::provider::{Completion, PromptInput};

pub const TEMPLATE_ID: &str = "daily_readiness";
pub const TEMPLATE_VERSION: &str = "1.0.0";

const TEMPLATE: &str = include_str!("../../../../src/abilities/prompts/daily_readiness.v1.0.0.txt");

pub struct RenderedPrompt {
    pub text: String,
    prompt: PromptInput,
}

impl RenderedPrompt {
    pub fn prompt_input(&self) -> PromptInput {
        self.prompt.clone()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PromptSections {
    pub meeting_topics: Value,
    pub meeting_attendees: Value,
    pub meeting_open_loops: Value,
    pub meeting_outcomes: Value,
    pub entity_contexts: Value,
    pub risk_directions: Value,
    pub risk_summaries: Value,
    pub open_loop_texts: Value,
    pub overnight_summaries: Value,
    pub coverage_warnings: Value,
}

pub fn render_prompt<T: Serialize>(
    context: &T,
    sections: &PromptSections,
    schema_version: u32,
) -> Result<RenderedPrompt, serde_json::Error> {
    let context_value = serde_json::to_value(context)?;
    let text = TEMPLATE
        .replace("{{schema_version}}", &schema_version.to_string())
        .replace(
            "{{meeting.topics}}",
            &section_json(&sections.meeting_topics)?,
        )
        .replace(
            "{{meeting.attendees}}",
            &section_json(&sections.meeting_attendees)?,
        )
        .replace(
            "{{meeting.open_loops}}",
            &section_json(&sections.meeting_open_loops)?,
        )
        .replace(
            "{{meeting.outcomes}}",
            &section_json(&sections.meeting_outcomes)?,
        )
        .replace(
            "{{entity_contexts}}",
            &section_json(&sections.entity_contexts)?,
        )
        .replace(
            "{{risk_shifts[].direction}}",
            &section_json(&sections.risk_directions)?,
        )
        .replace(
            "{{risk_shifts[].summary}}",
            &section_json(&sections.risk_summaries)?,
        )
        .replace(
            "{{open_loops[].text}}",
            &section_json(&sections.open_loop_texts)?,
        )
        .replace(
            "{{overnight_changes[].summary}}",
            &section_json(&sections.overnight_summaries)?,
        )
        .replace(
            "{{coverage_warnings}}",
            &section_json(&sections.coverage_warnings)?,
        );
    let canonical_inputs = canonical_prompt_inputs(context_value, schema_version);
    let prompt = PromptInput::new(text.clone())
        .with_template(
            TEMPLATE_ID,
            TEMPLATE_VERSION,
            canonical_template_hash(TEMPLATE),
        )
        .with_canonical_json_inputs(canonical_inputs);
    Ok(RenderedPrompt { text, prompt })
}

pub fn fingerprint_from_completion(
    completion: &Completion,
    rendered: &RenderedPrompt,
) -> PromptFingerprint {
    let prompt = rendered.prompt_input();
    prompt_fingerprint_from_completion(completion, &prompt, TEMPLATE_ID, TEMPLATE_VERSION)
}

fn section_json(value: &Value) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(value)
}

fn canonical_prompt_inputs(context: Value, schema_version: u32) -> Value {
    json!({
        "schema_version": schema_version,
        "context": context,
    })
}
