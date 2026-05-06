use sha2::{Digest, Sha256};

use crate::abilities::provenance::{
    HashValue, ModelName as ProvenanceModelName, PromptFingerprint, PromptTemplateId, PromptVersion,
};
use crate::intelligence::provider::{Completion, PromptInput};

pub const TEMPLATE_ID: &str = "prepare_meeting_prep";
pub const TEMPLATE_VERSION: &str = "1.0.0";

const TEMPLATE: &str = include_str!("../../../prompts/templates/prepare_meeting_prep.v1.txt");

pub struct RenderedPrompt {
    pub text: String,
    pub canonical_hash: String,
}

impl RenderedPrompt {
    pub fn prompt_input(&self) -> PromptInput {
        let mut prompt = PromptInput::new(self.text.clone());
        prompt.template_id = Some(TEMPLATE_ID.to_string());
        prompt.template_hash = Some(self.canonical_hash.clone());
        prompt
    }
}

pub fn render_prompt(context_json: &str, schema_version: u32) -> RenderedPrompt {
    let text = TEMPLATE
        .replace("{{schema_version}}", &schema_version.to_string())
        .replace("{{context_json}}", context_json);
    let canonical_hash = canonical_prompt_hash(&text);
    RenderedPrompt {
        text,
        canonical_hash,
    }
}

pub fn fingerprint_from_completion(
    completion: &Completion,
    rendered: &RenderedPrompt,
) -> PromptFingerprint {
    let meta = &completion.fingerprint_metadata;
    PromptFingerprint {
        provider: meta.provider.as_str().to_string(),
        model: ProvenanceModelName(meta.model.as_str().to_string()),
        prompt_template_id: PromptTemplateId(TEMPLATE_ID.to_string()),
        prompt_template_version: PromptVersion(TEMPLATE_VERSION.to_string()),
        canonical_prompt_hash: HashValue::new(rendered.canonical_hash.clone()),
        temperature: meta.temperature,
        top_p: meta.top_p,
        seed: meta.seed,
        tokens_input: meta.tokens_input,
        tokens_output: meta.tokens_output,
        provider_completion_id: meta.provider_completion_id.clone(),
    }
}

fn canonical_prompt_hash(text: &str) -> String {
    let canonical = text
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n");
    let mut hasher = Sha256::new();
    hasher.update(TEMPLATE_ID.as_bytes());
    hasher.update(b"\0");
    hasher.update(TEMPLATE_VERSION.as_bytes());
    hasher.update(b"\0");
    hasher.update(canonical.as_bytes());
    hex::encode(hasher.finalize())
}
