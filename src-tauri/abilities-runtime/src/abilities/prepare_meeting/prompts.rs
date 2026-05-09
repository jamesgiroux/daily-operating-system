use serde_json::{json, Value};

use crate::abilities::provenance::PromptFingerprint;
use crate::intelligence::prompt_fingerprint::{
    canonical_template_hash, prompt_fingerprint_from_completion,
};
use crate::intelligence::provider::{Completion, PromptInput};

pub const TEMPLATE_ID: &str = "prepare_meeting_prep";
pub const TEMPLATE_VERSION: &str = "1.0.0";

const TEMPLATE: &str =
    include_str!("../../../../src/abilities/prompts/prepare_meeting_prep.v1.0.0.txt");

pub struct RenderedPrompt {
    pub text: String,
    prompt: PromptInput,
}

impl RenderedPrompt {
    pub fn prompt_input(&self) -> PromptInput {
        self.prompt.clone()
    }
}

pub fn render_prompt(context_json: &str, schema_version: u32) -> RenderedPrompt {
    let text = TEMPLATE
        .replace("{{schema_version}}", &schema_version.to_string())
        .replace("{{context_json}}", context_json);
    let canonical_inputs = canonical_prompt_inputs(context_json, schema_version);
    let prompt = PromptInput::new(text.clone())
        .with_template(
            TEMPLATE_ID,
            TEMPLATE_VERSION,
            canonical_template_hash(TEMPLATE),
        )
        .with_canonical_json_inputs(canonical_inputs);
    RenderedPrompt { text, prompt }
}

pub fn fingerprint_from_completion(
    completion: &Completion,
    rendered: &RenderedPrompt,
) -> PromptFingerprint {
    let prompt = rendered.prompt_input();
    prompt_fingerprint_from_completion(completion, &prompt, TEMPLATE_ID, TEMPLATE_VERSION)
}

fn canonical_prompt_inputs(context_json: &str, schema_version: u32) -> Value {
    let context = serde_json::from_str(context_json)
        .unwrap_or_else(|_| Value::String(context_json.to_string()));
    json!({
        "schema_version": schema_version,
        "context": context,
    })
}
