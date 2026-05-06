use serde_json::{json, Value};

use crate::abilities::provenance::{
    HashValue, ModelName as ProvenanceModelName, PromptFingerprint, PromptTemplateId, PromptVersion,
};
use crate::intelligence::provider::{
    canonical_prompt_hash, canonical_template_hash, CanonicalPromptRequest, Completion, PromptInput,
};

pub const TEMPLATE_ID: &str = "prepare_meeting_prep";
pub const TEMPLATE_VERSION: &str = "1.0.0";

const TEMPLATE: &str = include_str!("../../../prompts/templates/prepare_meeting_prep.v1.txt");

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
    let meta = &completion.fingerprint_metadata;
    let prompt = rendered.prompt_input();
    let canonical_hash = canonical_prompt_hash(CanonicalPromptRequest {
        prompt: &prompt,
        fingerprint_metadata: meta,
    });
    PromptFingerprint {
        provider: meta.provider.as_str().to_string(),
        model: ProvenanceModelName(meta.model.as_str().to_string()),
        prompt_template_id: PromptTemplateId(TEMPLATE_ID.to_string()),
        prompt_template_version: PromptVersion(TEMPLATE_VERSION.to_string()),
        canonical_prompt_hash: HashValue::new(canonical_hash),
        temperature: meta.temperature,
        top_p: meta.top_p,
        seed: meta.seed,
        tokens_input: meta.tokens_input,
        tokens_output: meta.tokens_output,
        provider_completion_id: meta.provider_completion_id.clone(),
    }
}

fn canonical_prompt_inputs(context_json: &str, schema_version: u32) -> Value {
    let context = serde_json::from_str(context_json)
        .unwrap_or_else(|_| Value::String(context_json.to_string()));
    json!({
        "schema_version": schema_version,
        "context": context,
    })
}
